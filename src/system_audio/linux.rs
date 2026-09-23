//! Linux system-audio capture via PulseAudio/PipeWire sink monitors.
//!
//! PipeWire ships a PulseAudio compatibility server (`pipewire-pulse`), so
//! recording from the default sink's `.monitor` source captures everything
//! the computer plays — no extra permissions, no extra installs on modern
//! distros.

use anyhow::{anyhow, Result};
use libpulse_binding as pulse;
use libpulse_binding::sample::{Format, Spec};
use libpulse_simple_binding::Simple;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct PulseCapture {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl PulseCapture {
    /// Start recording from the default sink's monitor. f32 at the system
    /// rate (usually 48 kHz) is pushed into `push`.
    pub fn start(push: Arc<Mutex<Vec<f32>>>, rate_out: Arc<Mutex<u32>>) -> Result<(Self, u32)> {
        let spec = Spec {
            format: Format::FLOAT32NE,
            channels: 2,
            rate: 48_000,
        };
        if !spec.is_valid() {
            return Err(anyhow!("invalid pulse sample spec"));
        }

        let default_sink = default_sink_name().unwrap_or_else(|| "@DEFAULT_SINK@".to_string());
        let source = format!("{}.monitor", default_sink);

        let s = Simple::new(
            None,     // default server
            "otamot", // app name
            pulse::stream::Direction::Record,
            Some(&source), // monitor of default sink
            None,          // default stream name
            &spec,
            None, // channel map
            None, // sink input attributes
        )
        .map_err(|e| anyhow!("failed to open pulse monitor {}: {}", source, e))?;

        let rate = spec.rate;
        *rate_out.lock().unwrap() = spec.rate;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        let thread = std::thread::spawn(move || {
            // 20 ms of stereo f32 at 48 kHz
            let frames = spec.rate as usize / 50;
            let mut buf = vec![0f32; frames * 2];
            loop {
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }
                match s.read(&mut buf) {
                    Ok(()) => {
                        // Downmix stereo to mono before pushing
                        let mono: Vec<f32> = buf
                            .chunks(2)
                            .map(|frame| frame.iter().sum::<f32>() / 2.0)
                            .collect();
                        if let Ok(mut p) = push.lock() {
                            p.extend_from_slice(&mono);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((
            Self {
                stop,
                thread: Some(thread),
            },
            rate,
        ))
    }
}

impl Drop for PulseCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Query the default sink name via the introspection API (separate
/// mainloop/context since `Simple` holds its own connection).
fn default_sink_name() -> Option<String> {
    use pulse::context::{Context, FlagSet, State};
    use pulse::mainloop::standard::{IterateResult, Mainloop};

    let mut mainloop = Mainloop::new()?;
    let mut context = Context::new(&mainloop, "otamot-probe")?;
    context.connect(None, FlagSet::NOFLAGS, None).ok()?;
    loop {
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => return None,
            IterateResult::Success(_) => {}
        }
        if context.get_state() == State::Ready {
            break;
        }
    }

    let name: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let name_clone = Arc::clone(&name);
    {
        let introspector = context.introspector();
        let op = introspector.get_server_info(move |info| {
            if let Some(sink) = info.default_sink_name.as_ref() {
                *name_clone.lock().unwrap() = Some(sink.to_string());
            }
        });
        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => return None,
                IterateResult::Success(_) => {}
            }
            if op.get_state() == pulse::operation::State::Done {
                break;
            }
        }
    }
    name.lock().unwrap().clone()
}
