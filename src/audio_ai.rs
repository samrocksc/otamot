//! Audio capture, transcription and AI synthesis worker
//!
//! All heavy lifting (mic capture, Whisper inference, HTTP calls, model
//! downloads) happens on a single background worker thread. The UI thread
//! talks to it through an mpsc command channel and polls an event channel
//! with `try_recv` every frame — the egui thread never blocks.
//!
//! Pipeline stages are separate functions so future modes (transcribe-only,
//! file import, different delivery targets) are new command variants,
//! not rewrites.

use crate::ai_client;
use crate::config::WhisperSize;
use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Whisper expects 16 kHz mono 32-bit float
const TARGET_SAMPLE_RATE: u32 = 16_000;
/// Cap the buffer at max_recording_minutes + a little headroom
const BUFFER_HEADROOM_MINUTES: u32 = 2;

/// Commands the UI thread can send to the worker.
/// New capabilities = new variant + one match arm.
pub enum WorkerCommand {
    StartRecording {
        mode: crate::config::AiMode,
        /// Loopback device name for call mode (captures remote audio);
        /// empty = skip loopback capture
        loopback_device: String,
    },
    StopRecording,
    /// Run the full pipeline: transcribe the buffer, then synthesize
    /// Markdown via the active provider profile.
    TranscribeAndSynthesize {
        mode: crate::config::AiMode,
        endpoint: String,
        kind: crate::config::EndpointKind,
        api_key: String,
        model: String,
        prompt: String,
    },
    DownloadModel {
        size: WhisperSize,
        target_dir: String,
    },
    /// Query the provider's /models endpoint and report the list.
    FetchModels {
        endpoint: String,
        kind: crate::config::EndpointKind,
        api_key: String,
    },
    /// Lightweight endpoint health check (GET /models).
    TestEndpoint {
        endpoint: String,
        kind: crate::config::EndpointKind,
        api_key: String,
    },
    /// Enable/disable live partial transcription while recording.
    SetLiveTranscription {
        enabled: bool,
        model_path: String,
    },
    Shutdown,
}

/// Events the worker reports back to the UI thread.
pub enum WorkerEvent {
    RecordingStarted,
    RecordingStopped {
        seconds: f32,
    },
    /// Partial transcript from the rolling tail window during recording
    LiveTranscript {
        text: String,
    },
    ModelDownloadProgress {
        percent: f32,
    },
    ModelDownloadDone {
        path: String,
    },
    ModelsFetched {
        endpoint: String,
        models: Vec<String>,
    },
    EndpointTested {
        endpoint: String,
        ok: bool,
        detail: String,
    },
    /// The endpoint that actually worked differs from what was configured
    /// (e.g. bare host auto-resolved to host/v1). UI should persist it.
    EndpointResolved {
        requested: String,
        resolved: String,
    },
    Transcribed {
        text: String,
    },
    Synthesized {
        mode: crate::config::AiMode,
        markdown: String,
    },
    Error {
        message: String,
    },
}

/// Accumulated audio in capture units (f32 samples at the device rate).
struct CaptureBuffer {
    samples: Vec<f32>,
    sample_rate: u32,
    max_samples: usize,
}

impl CaptureBuffer {
    fn new(sample_rate: u32, max_minutes: u32) -> Self {
        let capped_minutes = max_minutes + BUFFER_HEADROOM_MINUTES;
        Self {
            samples: Vec::new(),
            sample_rate,
            max_samples: capped_minutes as usize * 60 * sample_rate as usize,
        }
    }

    fn push(&mut self, chunk: &[f32]) {
        let room = self.max_samples.saturating_sub(self.samples.len());
        let take = room.min(chunk.len());
        if take > 0 {
            self.samples.extend_from_slice(&chunk[..take]);
        }
    }

    fn seconds(&self) -> f32 {
        self.samples.len() as f32 / self.sample_rate as f32
    }
}

/// Downmix any channel layout to mono and resample to 16 kHz.
/// Linear interpolation resampling is fine for speech.
pub fn convert_to_whisper_format(samples: &[f32], channels: usize, in_rate: u32) -> Vec<f32> {
    if samples.is_empty() || channels == 0 {
        return Vec::new();
    }

    // 1. Downmix to mono
    let mono: Vec<f32> = if channels == 1 {
        samples.to_vec()
    } else {
        samples
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    // 2. Resample to 16 kHz
    if in_rate == TARGET_SAMPLE_RATE {
        return mono;
    }

    let in_len = mono.len();
    let out_len = ((in_len as f64 * TARGET_SAMPLE_RATE as f64 / in_rate as f64) as usize)
        .saturating_sub(1)
        .max(1);
    let mut out = Vec::with_capacity(out_len);
    let step = (in_len - 1) as f64 / out_len as f64;
    for i in 0..out_len {
        let pos = i as f64 * step;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = mono[idx];
        let b = mono[(idx + 1).min(in_len - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}

/// Segment text joined into a single transcript.
fn segments_to_text(state: &whisper_rs::WhisperState) -> String {
    let mut text = String::new();
    for segment in state.as_iter() {
        if let Ok(s) = segment.to_str_lossy() {
            text.push_str(&s);
            text.push(' ');
        }
    }
    text.trim().to_string()
}

/// Silence gap (in centiseconds, whisper's unit) that starts a new turn.
const TURN_GAP_CENTISECONDS: i64 = 90;

/// Format whisper segments into timestamped, pause-separated turns.
///
/// whisper.cpp has no speaker embeddings, so true diarization is out of
/// scope here; instead, silence gaps longer than `TURN_GAP_CENTISECONDS`
/// break the text into labeled turns with `[mm:ss]` start timestamps.
/// Turns are numbered so readers can follow the conversational flow.
pub fn format_timestamped_turns(state: &whisper_rs::WhisperState) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut turn: Vec<String> = Vec::new();
    let mut turn_start_cs: Option<i64> = None;
    let mut prev_end_cs: Option<i64> = None;
    let mut turn_number = 0usize;

    for segment in state.as_iter() {
        let Ok(text) = segment.to_str_lossy() else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let start = segment.start_timestamp();
        let end = segment.end_timestamp();

        let new_turn = is_new_turn(prev_end_cs, start);
        if new_turn {
            if !turn.is_empty() {
                if let Some(ts) = turn_start_cs {
                    turn_number += 1;
                    lines.push(format!(
                        "[{}] turn {}: {}",
                        fmt_ts(ts),
                        turn_number,
                        turn.join(" ")
                    ));
                }
                turn.clear();
            }
            turn_start_cs = Some(start);
        }
        turn.push(text.to_string());
        prev_end_cs = Some(end);
    }
    if !turn.is_empty() {
        if let Some(ts) = turn_start_cs {
            turn_number += 1;
            lines.push(format!(
                "[{}] turn {}: {}",
                fmt_ts(ts),
                turn_number,
                turn.join(" ")
            ));
        }
    }
    lines.join("\n")
}

/// Format centiseconds as `[mm:ss]`.
fn fmt_ts(centiseconds: i64) -> String {
    let total_seconds = centiseconds / 100;
    format!("[{:02}:{:02}]", total_seconds / 60, total_seconds % 60)
}

/// Pure helper mirroring the turn-break rule in `format_timestamped_turns`
/// so the threshold logic is testable without a loaded whisper model.
fn is_new_turn(prev_end_cs: Option<i64>, start_cs: i64) -> bool {
    match prev_end_cs {
        Some(prev) => start_cs.saturating_sub(prev) > TURN_GAP_CENTISECONDS,
        None => true,
    }
}

/// Lazily-loaded, cached Whisper context — the model is never reloaded
/// between transcriptions unless the path changes.
struct WhisperEngine {
    model_path: String,
    context: Option<whisper_rs::WhisperContext>,
}

impl WhisperEngine {
    fn new(model_path: String) -> Self {
        Self {
            model_path,
            context: None,
        }
    }

    fn ensure_loaded(&mut self, model_path: &str) -> Result<&whisper_rs::WhisperContext> {
        if self.context.is_none() || self.model_path != model_path {
            if !Path::new(model_path).exists() {
                return Err(anyhow!(
                    "Whisper model not found at {}. Use the download button in settings.",
                    model_path
                ));
            }
            let ctx = whisper_rs::WhisperContext::new_with_params(
                model_path,
                whisper_rs::WhisperContextParameters::default(),
            )
            .map_err(|e| anyhow!("failed to load Whisper model: {}", e))
            .context("loading whisper model")?;
            self.model_path = model_path.to_string();
            self.context = Some(ctx);
        }
        Ok(self.context.as_ref().unwrap())
    }

    /// Transcribe 16 kHz mono f32 audio. Blocking, CPU heavy —
    /// worker thread only.
    fn transcribe(&mut self, audio: &[f32], model_path: &str) -> Result<String> {
        let (text, _) = self.transcribe_with_turns(audio, model_path)?;
        Ok(text)
    }

    /// Transcribe and return both the plain text (for synthesis) and the
    /// timestamped turn-formatted transcript (for display/notes).
    fn transcribe_with_turns(
        &mut self,
        audio: &[f32],
        model_path: &str,
    ) -> Result<(String, String)> {
        let ctx = self.ensure_loaded(model_path)?;
        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow!("whisper state creation failed: {}", e))?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, audio)
            .map_err(|e| anyhow!("whisper transcription failed: {}", e))?;

        let text = segments_to_text(&state);
        let turns = format_timestamped_turns(&state);
        Ok((text, turns))
    }
}

/// Streaming model downloader with progress events.
/// HuggingFace hosts the ggml models for whisper.cpp.
fn download_model(size: WhisperSize, target_dir: &str, report: &dyn Fn(f32)) -> Result<String> {
    let file_name = size.file_name();
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
        file_name
    );
    std::fs::create_dir_all(target_dir)
        .context(format!("creating model directory {}", target_dir))?;
    let target_path = Path::new(target_dir).join(file_name);

    if target_path.exists() {
        return Ok(target_path.to_string_lossy().into_owned());
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(1800))
        .build()?;
    let mut response = client
        .get(&url)
        .send()
        .context(format!("reaching {}", url))?;
    if !response.status().is_success() {
        return Err(anyhow!("download failed: HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0) as f32;
    let mut file = std::fs::File::create(&target_path)
        .context(format!("creating {}", target_path.display()))?;
    let mut downloaded: usize = 0;
    let mut buffer = [0u8; 64 * 1024];
    use std::io::{Read, Write};
    loop {
        let chunk = response
            .read(&mut buffer)
            .context("reading download stream")?;
        if chunk == 0 {
            break;
        }
        file.write_all(&buffer[..chunk])?;
        downloaded += chunk;
        if total > 0.0 {
            report((downloaded as f32 / total) * 100.0);
        }
    }
    Ok(target_path.to_string_lossy().into_owned())
}

/// Handle to the background worker. Cloned into the UI side.
pub struct AudioAiWorker {
    command_tx: Sender<WorkerCommand>,
    event_rx: Arc<Mutex<Option<std::sync::mpsc::Receiver<WorkerEvent>>>>,
}

impl AudioAiWorker {
    /// Spawn the worker thread. Fails fast if no usable microphone exists —
    /// the feature is useless without one, and the caller can surface the
    /// reason instead of failing later mid-recording.
    pub fn spawn(max_recording_minutes: u32, model_path: String) -> Result<Self> {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WorkerCommand>();
        let (event_tx, event_rx) = std::sync::mpsc::channel::<WorkerEvent>();

        // Fail fast: probe the default input device before spawning
        let host = cpal::default_host();
        let default_device = host.default_input_device().ok_or_else(|| {
            anyhow!(
                "no microphone input device found — if this is a fresh install, \
                 grant Otamot microphone access in System Settings > Privacy & \
                 Security > Microphone, then restart the app"
            )
        })?;
        let supported = default_device.default_input_config().context(
            "querying microphone config — if this is a fresh install, grant \
                 Otamot microphone access in System Settings > Privacy & Security \
                 > Microphone, then restart the app",
        )?;

        let probe = std::thread::spawn(move || {
            worker_loop(
                cmd_rx,
                event_tx,
                max_recording_minutes,
                model_path,
                supported,
            )
        });
        std::mem::forget(probe);

        Ok(Self {
            command_tx: cmd_tx,
            event_rx: Arc::new(Mutex::new(Some(event_rx))),
        })
    }

    pub fn command_tx(&self) -> &Sender<WorkerCommand> {
        &self.command_tx
    }

    /// Take ownership of the event receiver. Called once from the UI thread.
    pub fn take_event_receiver(&self) -> Option<std::sync::mpsc::Receiver<WorkerEvent>> {
        self.event_rx.lock().ok().and_then(|mut guard| guard.take())
    }
}

/// Names of all available input devices (for the settings dropdowns).
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    match host.input_devices() {
        Ok(devices) => devices.filter_map(|d| d.name().ok()).collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    }
}

/// Pick an input device by name; falls back to the system default when the
/// name is empty or not found. Reports which name was actually used.
fn select_input_device(
    host: &cpal::Host,
    preferred_name: &str,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig, String)> {
    let default = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no microphone input device found"))?;
    if preferred_name.trim().is_empty() {
        let config = default
            .default_input_config()
            .context("querying default microphone config")?;
        let name = default.name().unwrap_or_else(|_| "default".to_string());
        return Ok((default, config, name));
    }
    let wanted = preferred_name.trim();
    for device in host.input_devices().context("listing input devices")? {
        if let Ok(name) = device.name() {
            if name == wanted {
                let config = device
                    .default_input_config()
                    .with_context(|| format!("querying config for device {}", name))?;
                return Ok((device, config, name));
            }
        }
    }
    // Preferred device missing — fall back to default rather than failing
    let config = default
        .default_input_config()
        .context("querying default microphone config")?;
    let name = default.name().unwrap_or_else(|_| "default".to_string());
    Ok((default, config, name))
}

fn worker_loop(
    cmd_rx: Receiver<WorkerCommand>,
    event_tx: Sender<WorkerEvent>,
    max_recording_minutes: u32,
    model_path: String,
    input_config: cpal::SupportedStreamConfig,
) {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("device checked at spawn");

    let mut engine = WhisperEngine::new(model_path);
    let buffer: Arc<Mutex<CaptureBuffer>> = Arc::new(Mutex::new(CaptureBuffer::new(
        input_config.sample_rate().0,
        max_recording_minutes,
    )));
    // Streams stay on this thread (not Send on all platforms); only the
    // sample buffer is shared with capture callbacks. All streams (mic +
    // optional loopback) live in one vec; clearing it stops all capture.
    let mut active_streams: Vec<cpal::Stream> = Vec::new();
    // Native system-audio capture (macOS tap / Linux monitor) for call mode.
    // Held alive until recording stops; dropping it tears down the tap.
    #[allow(unused_assignments)]
    let mut system_capture: Option<crate::system_audio::SystemAudioCapture> = None;
    // The f32 tail of the capture buffer that system audio pushes into —
    // system capture writes raw f32 at its own rate; worker converts at stop.
    let buffer_push_f32: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    // Live transcription state
    let mut live_transcription = false;
    let mut last_live_end_samples: usize = 0;
    const LIVE_WINDOW_SECONDS: f32 = 12.0;
    const LIVE_INTERVAL_SECONDS: f32 = 4.0;

    loop {
        // While recording with live mode on, poll for commands but wake up
        // periodically to transcribe the newest audio tail.
        let timeout = if live_transcription && !active_streams.is_empty() {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(5)
        };
        let command = match cmd_rx.recv_timeout(timeout) {
            Ok(cmd) => cmd,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No command — run a live transcription pass if eligible
                if live_transcription && !active_streams.is_empty() {
                    let pass = {
                        let buf = buffer.lock().unwrap();
                        buf.samples.len().saturating_sub(last_live_end_samples)
                            >= (LIVE_INTERVAL_SECONDS * buf.sample_rate as f32) as usize
                    };
                    if pass {
                        let audio_16k = {
                            let buf = buffer.lock().unwrap();
                            let tail_len = (LIVE_WINDOW_SECONDS * buf.sample_rate as f32) as usize;
                            let start = buf.samples.len().saturating_sub(tail_len);
                            convert_to_whisper_format(&buf.samples[start..], 1, buf.sample_rate)
                        };
                        let model_path = engine.model_path.clone();
                        match engine.transcribe(&audio_16k, &model_path) {
                            Ok(text) if !text.is_empty() => {
                                let _ = event_tx.send(WorkerEvent::LiveTranscript { text });
                            }
                            Ok(_) => {}
                            Err(e) => {
                                // Live pass failures are non-fatal; report once
                                let _ = event_tx.send(WorkerEvent::Error {
                                    message: format!("live transcription: {:#}", e),
                                });
                            }
                        }
                        last_live_end_samples = buffer.lock().unwrap().samples.len();
                    }
                }
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match command {
            WorkerCommand::StartRecording {
                mode,
                loopback_device,
            } => {
                if !active_streams.is_empty() {
                    let _ = event_tx.send(WorkerEvent::Error {
                        message: "already recording".to_string(),
                    });
                    continue;
                }
                let config: cpal::StreamConfig = input_config.clone().into();
                let err_fn = |err| eprintln!("audio stream error: {}", err);
                let buf = Arc::clone(&buffer);
                let result = device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        if let Ok(mut b) = buf.lock() {
                            b.push(data);
                        }
                    },
                    err_fn,
                    None,
                );
                match result {
                    Ok(new_stream) => {
                        if let Err(e) = new_stream.play() {
                            let _ = event_tx.send(WorkerEvent::Error {
                                message: format!("failed to start capture: {}", e),
                            });
                        } else {
                            active_streams.push(new_stream);
                            last_live_end_samples = buffer.lock().unwrap().samples.len();

                            // Call mode with a loopback device: open a second
                            // stream so remote call audio is captured too.
                            // Both streams push into the same buffer; the
                            // mixer interleaves at convert time.
                            // Call mode: try native system-audio capture first
                            // (macOS tap / Linux monitor). Fall back to the
                            // configured loopback device, then mic-only.
                            let mut system_capture_handle: Option<
                                crate::system_audio::SystemAudioCapture,
                            > = None;
                            if mode == crate::config::AiMode::Call {
                                let push = Arc::clone(&buffer_push_f32);
                                let rate_cell = Arc::new(Mutex::new(0u32));
                                match crate::system_audio::SystemAudioCapture::start(
                                    push,
                                    Arc::clone(&rate_cell),
                                ) {
                                    Ok((capture, rate, method)) => {
                                        system_capture_handle = Some(capture);
                                        let _ = event_tx.send(WorkerEvent::Error {
                                            message: format!(
                                                "system audio capture active via {} ({} Hz)",
                                                method.label(),
                                                rate
                                            ),
                                        });
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(WorkerEvent::Error {
                                            message: format!(
                                                "system audio unavailable, falling back: {:#}",
                                                e
                                            ),
                                        });
                                    }
                                }
                            }
                            system_capture = system_capture_handle;

                            if mode == crate::config::AiMode::Call
                                && system_capture.is_none()
                                && !loopback_device.trim().is_empty()
                            {
                                match select_input_device(&host, &loopback_device) {
                                    Ok((l_dev, l_cfg, _l_name)) => {
                                        let l_config: cpal::StreamConfig = l_cfg.into();
                                        let l_buf = Arc::clone(&buffer);
                                        match l_dev.build_input_stream(
                                            &l_config,
                                            move |data: &[f32], _| {
                                                if let Ok(mut b) = l_buf.lock() {
                                                    b.push(data);
                                                }
                                            },
                                            |err| eprintln!("loopback stream error: {}", err),
                                            None,
                                        ) {
                                            Ok(l_stream) => {
                                                if let Err(e) = l_stream.play() {
                                                    let _ = event_tx.send(WorkerEvent::Error {
                                                        message: format!(
                                                            "loopback capture failed: {}",
                                                            e
                                                        ),
                                                    });
                                                } else {
                                                    active_streams.push(l_stream);
                                                }
                                            }
                                            Err(e) => {
                                                let _ = event_tx.send(WorkerEvent::Error {
                                                    message: format!(
                                                        "loopback stream build failed: {}",
                                                        e
                                                    ),
                                                });
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(WorkerEvent::Error {
                                            message: format!("loopback device not found: {:#}", e),
                                        });
                                    }
                                }
                            }

                            let _ = event_tx.send(WorkerEvent::RecordingStarted);
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(WorkerEvent::Error {
                            message: format!(
                                "failed to build capture stream: {} — if this is a \
                                 fresh install, grant Otamot microphone access in \
                                 System Settings > Privacy & Security > Microphone, \
                                 then restart the app",
                                e
                            ),
                        });
                    }
                }
            }
            WorkerCommand::StopRecording => {
                if active_streams.is_empty() {
                    let _ = event_tx.send(WorkerEvent::Error {
                        message: "not recording".to_string(),
                    });
                    continue;
                }
                // Drop all streams (mic + loopback) so no more samples
                // arrive mid-convert, then stop native system capture
                active_streams.clear();
                if let Some(cap) = system_capture.take() {
                    drop(cap);
                }
                let seconds = buffer.lock().unwrap().seconds();
                let _ = event_tx.send(WorkerEvent::RecordingStopped { seconds });
            }
            WorkerCommand::TranscribeAndSynthesize {
                mode,
                endpoint,
                kind,
                api_key,
                model,
                prompt,
            } => {
                let (audio, sample_rate) = {
                    let mut buf = buffer.lock().unwrap();
                    let audio = convert_to_whisper_format(
                        &buf.samples,
                        1, // buffer already stores raw device frames; downmix uses channels param
                        buf.sample_rate,
                    );
                    let rate = buf.sample_rate;
                    buf.samples.clear();
                    (audio, rate)
                };
                if audio.is_empty() {
                    let _ = event_tx.send(WorkerEvent::Error {
                        message: "no audio recorded".to_string(),
                    });
                    continue;
                }

                let model_path = engine.model_path.clone();
                match engine.transcribe_with_turns(&audio, &model_path) {
                    Ok((transcript, turns)) => {
                        let _ = event_tx.send(WorkerEvent::Transcribed {
                            text: turns.clone(),
                        });
                        if transcript.is_empty() {
                            let _ = event_tx.send(WorkerEvent::Error {
                                message: "transcription was empty".to_string(),
                            });
                            continue;
                        }
                        let provider = crate::config::AiProviderConfig {
                            name: "worker".to_string(),
                            kind,
                            endpoint,
                            api_key,
                            model,
                        };
                        match ai_client::synthesize(&provider, &prompt, &transcript) {
                            Ok(markdown) => {
                                let _ = event_tx.send(WorkerEvent::Synthesized { mode, markdown });
                            }
                            Err(e) => {
                                let _ = event_tx.send(WorkerEvent::Error {
                                    message: format!("synthesis failed: {:#}", e),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(WorkerEvent::Error {
                            message: format!("transcription failed: {:#}", e),
                        });
                    }
                }
                let _ = sample_rate;
            }
            WorkerCommand::DownloadModel { size, target_dir } => {
                let report = |percent: f32| {
                    let _ = event_tx.send(WorkerEvent::ModelDownloadProgress { percent });
                };
                match download_model(size, &target_dir, &report) {
                    Ok(path) => {
                        let _ = event_tx.send(WorkerEvent::ModelDownloadDone { path });
                    }
                    Err(e) => {
                        let _ = event_tx.send(WorkerEvent::Error {
                            message: format!("model download failed: {:#}", e),
                        });
                    }
                }
            }
            WorkerCommand::FetchModels {
                endpoint,
                kind,
                api_key,
            } => {
                let provider = crate::config::AiProviderConfig {
                    name: String::new(),
                    kind,
                    endpoint,
                    api_key,
                    model: String::new(),
                };
                match ai_client::fetch_models_resolved(&provider) {
                    Ok((models, resolved)) => {
                        if resolved != provider.endpoint {
                            let _ = event_tx.send(WorkerEvent::EndpointResolved {
                                requested: provider.endpoint.clone(),
                                resolved: resolved.clone(),
                            });
                        }
                        let _ = event_tx.send(WorkerEvent::ModelsFetched {
                            endpoint: resolved,
                            models,
                        });
                    }
                    Err(e) => {
                        let _ = event_tx.send(WorkerEvent::Error {
                            message: format!("model list failed: {:#}", e),
                        });
                    }
                }
            }
            WorkerCommand::TestEndpoint {
                endpoint,
                kind,
                api_key,
            } => {
                let provider = crate::config::AiProviderConfig {
                    name: String::new(),
                    kind,
                    endpoint,
                    api_key,
                    model: String::new(),
                };
                match ai_client::fetch_models_resolved(&provider) {
                    Ok((models, resolved)) => {
                        let _ = event_tx.send(WorkerEvent::EndpointTested {
                            endpoint: resolved.clone(),
                            ok: true,
                            detail: format!("{} models available", models.len()),
                        });
                        if resolved != provider.endpoint {
                            let _ = event_tx.send(WorkerEvent::EndpointResolved {
                                requested: provider.endpoint.clone(),
                                resolved: resolved.clone(),
                            });
                        }
                        let _ = event_tx.send(WorkerEvent::ModelsFetched {
                            endpoint: resolved,
                            models,
                        });
                    }
                    Err(e) => {
                        let _ = event_tx.send(WorkerEvent::EndpointTested {
                            endpoint: provider.endpoint,
                            ok: false,
                            detail: format!("{:#}", e),
                        });
                    }
                }
            }
            WorkerCommand::SetLiveTranscription {
                enabled,
                model_path,
            } => {
                live_transcription = enabled;
                if enabled {
                    // Warm the model so the first live pass isn't slow
                    let path = engine.model_path.clone();
                    if !path.is_empty() {
                        let _ = engine.ensure_loaded(&model_path);
                    }
                }
            }
            WorkerCommand::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EndpointKind;

    #[test]
    fn test_convert_passthrough_16k_mono() {
        let samples = vec![0.1f32, 0.2, 0.3, 0.4];
        let out = convert_to_whisper_format(&samples, 1, 16_000);
        assert_eq!(out, samples);
    }

    #[test]
    fn test_convert_downmixes_stereo() {
        // stereo frames: (0.5, 0.5), (1.0, 0.0) -> mono: 0.5, 0.5
        let stereo = vec![0.5f32, 0.5, 1.0, 0.0];
        let out = convert_to_whisper_format(&stereo, 2, 16_000);
        assert_eq!(out, vec![0.5, 0.5]);
    }

    #[test]
    fn test_convert_resamples_48k_to_16k() {
        // 3 seconds at 48k = 144000 samples -> expect ~48000 out
        let samples: Vec<f32> = (0..144_000).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = convert_to_whisper_format(&samples, 1, 48_000);
        let expected = 144_000f64 * 16_000f64 / 48_000f64;
        assert!((out.len() as f64 - expected).abs() <= 2.0);
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_convert_empty() {
        assert!(convert_to_whisper_format(&[], 1, 44_100).is_empty());
    }

    #[test]
    fn test_capture_buffer_caps() {
        // 1 sample per second rate trick: rate=1, max=1 minute => 60 samples + headroom 2 min => 180
        let mut buf = CaptureBuffer::new(1, 1);
        let chunk = vec![0.0f32; 1000];
        buf.push(&chunk);
        assert_eq!(buf.samples.len(), 180); // (1 + 2 headroom) * 60 * 1
        assert_eq!(buf.seconds(), 180.0);
    }

    #[test]
    fn test_capture_buffer_seconds() {
        let mut buf = CaptureBuffer::new(100, 5);
        buf.push(&vec![0.0f32; 250]);
        assert!((buf.seconds() - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_worker_event_and_command_are_enumerable() {
        // Protocol smoke: constructing every variant stays possible
        // (catch-all UI arms must keep compiling as variants grow).
        let commands = vec![
            WorkerCommand::StartRecording {
                mode: crate::config::AiMode::Thoughts,
                loopback_device: String::new(),
            },
            WorkerCommand::StopRecording,
            WorkerCommand::TranscribeAndSynthesize {
                mode: crate::config::AiMode::Thoughts,
                endpoint: "http://localhost:11434/v1".to_string(),
                kind: EndpointKind::OpenAiCompatible,
                api_key: String::new(),
                model: "llama3.2".to_string(),
                prompt: "p".to_string(),
            },
            WorkerCommand::DownloadModel {
                size: WhisperSize::Tiny,
                target_dir: "/tmp".to_string(),
            },
            WorkerCommand::Shutdown,
        ];
        assert_eq!(commands.len(), 5);

        let events = vec![
            WorkerEvent::RecordingStarted,
            WorkerEvent::RecordingStopped { seconds: 1.0 },
            WorkerEvent::LiveTranscript {
                text: "l".to_string(),
            },
            WorkerEvent::ModelDownloadProgress { percent: 50.0 },
            WorkerEvent::ModelDownloadDone {
                path: "/tmp/m.bin".to_string(),
            },
            WorkerEvent::ModelsFetched {
                endpoint: "http://x".to_string(),
                models: vec![],
            },
            WorkerEvent::EndpointTested {
                endpoint: "http://x".to_string(),
                ok: true,
                detail: String::new(),
            },
            WorkerEvent::EndpointResolved {
                requested: "http://x".to_string(),
                resolved: "http://x/v1".to_string(),
            },
            WorkerEvent::Transcribed {
                text: "t".to_string(),
            },
            WorkerEvent::Synthesized {
                mode: crate::config::AiMode::Call,
                markdown: "m".to_string(),
            },
            WorkerEvent::Error {
                message: "e".to_string(),
            },
        ];
        assert_eq!(events.len(), 11);
    }

    #[test]
    fn test_event_channel_roundtrip() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(WorkerEvent::RecordingStarted).unwrap();
        match rx.try_recv() {
            Ok(WorkerEvent::RecordingStarted) => {}
            _ => panic!("expected RecordingStarted"),
        }
    }

    #[test]
    fn test_convert_channel_out_of_range() {
        // one sample with 8 declared channels: partial frame is averaged
        // over the declared channel count (0.5 / 8)
        let out = convert_to_whisper_format(&[0.5f32], 8, 16_000);
        assert_eq!(out, vec![0.0625]);
    }

    #[test]
    fn test_whisper_engine_missing_model_errors() {
        let mut engine = WhisperEngine::new("/nonexistent/model.bin".to_string());
        let err = engine.transcribe(&[0.0f32; 100], "/nonexistent/model.bin");
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("Whisper model not found"));
    }

    #[test]
    fn test_fmt_ts() {
        assert_eq!(fmt_ts(0), "[00:00]");
        assert_eq!(fmt_ts(6_500), "[01:05]");
        assert_eq!(fmt_ts(60_000), "[10:00]");
        assert_eq!(fmt_ts(3_723_00), "[62:03]");
    }

    #[test]
    fn test_is_new_turn() {
        // First segment always starts a turn
        assert!(is_new_turn(None, 0));
        // Small gap (< 0.9s = 90cs): same turn
        assert!(!is_new_turn(Some(1_000), 1_050));
        // Exactly the threshold: same turn (strictly greater breaks)
        assert!(!is_new_turn(Some(1_000), 1_090));
        // Gap of 1s: new turn
        assert!(is_new_turn(Some(1_000), 1_100));
        // Clock skew (negative gap): saturates to 0, same turn
        assert!(!is_new_turn(Some(2_000), 1_900));
    }

    #[test]
    fn test_turn_gap_threshold_value() {
        assert_eq!(TURN_GAP_CENTISECONDS, 90); // 0.9 seconds
    }
}
