use chrono::Local;
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// otamot library imports
use otamot::audio_ai::{AudioAiWorker, WorkerCommand, WorkerEvent};
use otamot::bell::Bell;
use otamot::commands::CommandManager;
use otamot::config::{Config, Language, NotesView, Theme};
use otamot::easy_mark::editor::EasyMarkEditor;
use otamot::hashtags::HashtagLibrary;
use otamot::kanban::KanbanBoard;
use otamot::localization::T;
use otamot::markdown::{format_markdown, insert_date_bullet};
use otamot::notes;
use otamot::survey::SurveyData;
use otamot::timer::{CallState, TimerMode};
use otamot::todo::TodoList;
use otamot::ui::{
    notes::{NotesAction, NotesEditor},
    sidebar::Sidebar,
    timer::{TimerAction, TimerView},
};
use otamot::ui_components;

/// Dropdown state for autocomplete
#[derive(Debug, Clone, PartialEq)]
enum DropdownType {
    Command,
    Hashtag,
}

pub struct PomodoroApp {
    // Timer state
    mode: TimerMode,
    remaining_seconds: u32,
    is_running: bool,
    last_tick: Option<Instant>,
    session_start: Option<chrono::DateTime<Local>>,
    session_end: Option<chrono::DateTime<Local>>,

    // Call mode state
    call_state: CallState,

    // Configuration
    config: Config,

    // Bell sound
    bell: Bell,

    // UI state
    show_settings: bool,

    // Localization helper
    t: T,

    // Notes state
    notes_enabled: bool,
    notes_content: String,
    project_content: String,
    notes_view: NotesView,
    focus_notes_input: bool, // Flag to request focus on notes text input
    requested_cursor_pos: Option<usize>, // Requested cursor position for notes input
    notes_cursor_pos: usize, // Current cursor position in notes text input

    // Slash commands and hashtags
    command_manager: CommandManager,
    hashtag_library: HashtagLibrary,
    dropdown_visible: bool,
    dropdown_type: DropdownType,
    dropdown_items: Vec<String>,
    dropdown_selected: usize,
    dropdown_start_pos: usize, // Position of / or # in text

    // Help menu
    show_help: bool,
    show_about: bool,

    // TODO list
    todo_list: TodoList,
    todo_input: String,

    // Session metadata
    sessions_completed: u32,

    // Survey state
    show_survey: bool,
    show_survey_summary: bool,
    survey_data: SurveyData,
    survey_focus_rating: u32,
    survey_what_helped: String,
    survey_what_hurt: String,
    todo_enabled: bool,
    editor: EasyMarkEditor,

    // Kanban state
    kanban_enabled: bool,
    kanban_board: KanbanBoard,
    kanban_input: String,

    sidebar_collapsed: bool,

    // Tray Icon
    #[cfg(not(target_arch = "wasm32"))]
    tray_icon: Option<tray_icon::TrayIcon>,
    #[cfg(not(target_arch = "wasm32"))]
    tray_info_items: Option<TrayInfoItems>,
    #[cfg(not(target_arch = "wasm32"))]
    tray_menu_ids: std::collections::HashMap<String, tray_icon::menu::MenuId>,

    // Active listening notification state
    active_listening_next_notification: Option<Instant>,
    active_listening_message_index: usize,

    // Audio AI notes (toggleable feature)
    ai_worker: Option<AudioAiWorker>,
    ai_event_rx: Option<std::sync::mpsc::Receiver<WorkerEvent>>,
    ai_recording: bool,
    ai_busy: bool,
    ai_status: Option<AiStatus>,
    ai_model_download_progress: Option<f32>,
    // Cached model lists per endpoint (populated by the ↻ button)
    ai_models_cache: std::collections::HashMap<String, Vec<String>>,
    ai_fetching_models: Option<String>,
    // Endpoint test results per endpoint (None = untested/failed)
    ai_endpoint_status: std::collections::HashMap<String, bool>,
    ai_endpoint_status_detail: std::collections::HashMap<String, String>,
    // Rolling live transcript while recording (partial, from tail window)
    ai_live_transcript: Option<String>,
    // Authoritative transcript of the last recording (timestamped turns)
    ai_last_transcript: Option<String>,
    // Global hotkey receiver (Cmd+Shift+R by default, macOS only)
    voice_hotkey_rx: Option<std::sync::mpsc::Receiver<()>>,
    voice_hotkey_registered: bool,
    // Which recorder produced the pending transcript (thoughts or call)
    ai_active_mode: otamot::config::AiMode,
    // Cached list of input device names for the settings dropdowns
    ai_input_devices: Vec<String>,
    // Receiver for detached "Test system audio" results (polled in update)
    system_audio_test_rx: Option<std::sync::mpsc::Receiver<anyhow::Result<u32>>>,
}

/// Transient status shown under the timer while the AI pipeline runs.
#[derive(Debug, Clone, PartialEq)]
enum AiStatus {
    Recording,
    Transcribing,
    Synthesizing,
    Done(String),
    Failed(String),
}

/// Returns the tray label for the survey score.
/// Shows "—" when no survey responses have been recorded yet.
fn format_tray_score(average_focus: f64, focus_count: u32) -> String {
    if focus_count == 0 {
        "Survey Score: —".to_string()
    } else {
        format!("Survey Score: {:.1}", average_focus)
    }
}

/// Directory portion of a whisper model path (models live beside the file).
fn model_dir_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string())
}

/// Returns the tray label for total sessions completed.
fn format_tray_sessions(sessions_completed: u32) -> String {
    format!("Sessions Tracked: {}", sessions_completed)
}

/// Returns the tray label for a single top issue entry.
/// Falls back to "—" when fewer than 3 issues have been recorded.
fn format_tray_issue(issue: Option<&str>) -> String {
    match issue {
        Some(s) => format!("• {}", s),
        None => "—".to_string(),
    }
}

/// Holds the live `MenuItem` handles for the read-only tray overview row.
///
/// Items are disabled (non-clickable) and used only for display.
/// Call `refresh` after any `SurveyData` mutation to keep labels current.
#[cfg(not(target_arch = "wasm32"))]
struct TrayInfoItems {
    score: tray_icon::menu::MenuItem,
    sessions: tray_icon::menu::MenuItem,
    issue_0: tray_icon::menu::MenuItem,
    issue_1: tray_icon::menu::MenuItem,
    issue_2: tray_icon::menu::MenuItem,
}

#[cfg(not(target_arch = "wasm32"))]
impl TrayInfoItems {
    /// Creates all five disabled menu items populated from `survey`.
    fn new(survey: &SurveyData) -> Self {
        let [i0, i1, i2] = Self::top_issue_labels(survey);
        Self {
            score: tray_icon::menu::MenuItem::new(
                format_tray_score(survey.average_focus, survey.focus_count),
                false,
                None,
            ),
            sessions: tray_icon::menu::MenuItem::new(
                format_tray_sessions(survey.sessions_completed),
                false,
                None,
            ),
            issue_0: tray_icon::menu::MenuItem::new(i0, false, None),
            issue_1: tray_icon::menu::MenuItem::new(i1, false, None),
            issue_2: tray_icon::menu::MenuItem::new(i2, false, None),
        }
    }

    /// Updates all label text from the latest `survey` state.
    ///
    /// Pre-computes all strings before touching menu items to keep
    /// the borrow of `survey` and the borrow of `self` fully separate.
    fn refresh(&self, survey: &SurveyData) {
        let [i0, i1, i2] = Self::top_issue_labels(survey);
        self.score
            .set_text(format_tray_score(survey.average_focus, survey.focus_count));
        self.sessions
            .set_text(format_tray_sessions(survey.sessions_completed));
        self.issue_0.set_text(i0);
        self.issue_1.set_text(i1);
        self.issue_2.set_text(i2);
    }

    /// Returns formatted labels for the top 3 concentration issues.
    fn top_issue_labels(survey: &SurveyData) -> [String; 3] {
        let top = survey.top_issues(3);
        [
            format_tray_issue(top.get(0).map(|s| s.as_str())),
            format_tray_issue(top.get(1).map(|s| s.as_str())),
            format_tray_issue(top.get(2).map(|s| s.as_str())),
        ]
    }
}

impl PomodoroApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let remaining_seconds = config.work_duration * 60;
        let survey_data = SurveyData::load();
        let bell = Bell::default();

        let mut app = Self {
            mode: TimerMode::Work,
            remaining_seconds,
            is_running: false,
            last_tick: None,
            session_start: None,
            session_end: None,
            call_state: CallState::new(),
            config: config.clone(),
            bell,
            show_settings: false,
            t: T::new(config.language),
            notes_enabled: config.notes_enabled,
            notes_content: notes::load_draft(&config.notes_directory),
            project_content: std::fs::read_to_string(&config.todo_file).unwrap_or_default(),
            notes_view: NotesView::Edit,
            focus_notes_input: false,
            requested_cursor_pos: None,
            notes_cursor_pos: 0,
            command_manager: CommandManager::with_commands(config.slash_commands.clone()),
            hashtag_library: HashtagLibrary::load(),
            dropdown_visible: false,
            dropdown_type: DropdownType::Command,
            dropdown_items: Vec::new(),
            dropdown_selected: 0,
            dropdown_start_pos: 0,
            show_help: false,
            show_about: false,
            todo_list: TodoList::load_from_path(&config.todo_file),
            todo_input: String::new(),
            todo_enabled: config.todo_enabled,
            editor: EasyMarkEditor::default(),
            kanban_enabled: config.kanban_enabled,
            kanban_board: KanbanBoard::load_from_path(&config.todo_file),
            kanban_input: String::new(),
            sidebar_collapsed: config.sidebar_collapsed,

            #[cfg(not(target_arch = "wasm32"))]
            tray_icon: None,
            #[cfg(not(target_arch = "wasm32"))]
            tray_info_items: None,
            #[cfg(not(target_arch = "wasm32"))]
            tray_menu_ids: std::collections::HashMap::new(),

            active_listening_next_notification: None,
            active_listening_message_index: 0,

            ai_worker: None,
            ai_event_rx: None,
            ai_recording: false,
            ai_busy: false,
            ai_status: None,
            ai_model_download_progress: None,
            ai_models_cache: std::collections::HashMap::new(),
            ai_fetching_models: None,
            ai_endpoint_status: std::collections::HashMap::new(),
            ai_endpoint_status_detail: std::collections::HashMap::new(),
            ai_live_transcript: None,
            ai_last_transcript: None,
            voice_hotkey_rx: None,
            voice_hotkey_registered: false,
            ai_active_mode: otamot::config::AiMode::Thoughts,
            ai_input_devices: Vec::new(),
            system_audio_test_rx: None,

            sessions_completed: survey_data.sessions_completed,
            show_survey: false,
            show_survey_summary: false,
            survey_data,
            survey_focus_rating: 5,
            survey_what_helped: String::new(),
            survey_what_hurt: String::new(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        app.setup_tray_icon();

        app
    }

    fn get_notes_byte_pos(&self) -> usize {
        self.notes_content
            .char_indices()
            .nth(self.notes_cursor_pos)
            .map(|(idx, _)| idx)
            .unwrap_or(self.notes_content.len())
    }

    fn format_time(&self) -> String {
        let minutes = self.remaining_seconds / 60;
        let seconds = self.remaining_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    fn render_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        _text_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        text_dim_color: egui::Color32,
    ) {
        let mut sidebar = Sidebar {
            sidebar_collapsed: &mut self.sidebar_collapsed,
            notes_enabled: &mut self.notes_enabled,
            todo_enabled: &mut self.todo_enabled,
            kanban_enabled: &mut self.kanban_enabled,
            show_settings: &mut self.show_settings,
            show_survey_summary: &mut self.show_survey_summary,
            show_help: &mut self.show_help,
            sessions_completed: self.sessions_completed,
            t: &self.t,
            config: &mut self.config,
            ai_recording: self.ai_recording,
            ai_busy: self.ai_busy,
        };
        if let Some(otamot::ui::sidebar::SidebarAction::ToggleAiRecording) =
            sidebar.show(ui, button_color, button_text_color, text_dim_color)
        {
            self.toggle_ai_recording();
        }
    }

    fn render_timer(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
        call_color: egui::Color32,
    ) {
        let timer_view = TimerView {
            is_running: self.is_running,
            mode: self.mode,
            time_formatted: self.format_time(),
            sessions_completed: self.sessions_completed,
            notes_enabled: self.notes_enabled,
            has_notes_content: !self.notes_content.is_empty(),
            call_mode_active: self.call_state.is_active,
            call_time_formatted: Some(self.call_state.format_time()),
            t: &self.t,
        };

        if let Some(action) = timer_view.show(
            ui,
            text_color,
            button_color,
            work_color,
            break_color,
            call_color,
        ) {
            match action {
                TimerAction::Toggle => self.toggle_timer(),
                TimerAction::Reset => self.reset_timer(),
                TimerAction::Skip => self.skip_to_break(),
                TimerAction::SaveNotes => self.save_notes(),
                TimerAction::StartCall => self.start_call(),
                TimerAction::EndCall => self.end_call(),
            }
        }
    }

    fn toggle_timer(&mut self) {
        self.is_running = !self.is_running;
        if self.is_running {
            if self.session_start.is_none() {
                self.session_start = Some(Local::now());
            }
            self.last_tick = Some(Instant::now());
        }
    }

    fn reset_timer(&mut self) {
        self.is_running = false;
        self.mode = TimerMode::Work;
        self.remaining_seconds = self.config.work_duration * 60;
        self.last_tick = None;
        self.session_start = None;
        self.session_end = None;
    }

    pub fn send_notification(&self, title: &str, body: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = notify_rust::Notification::new()
                .summary(title)
                .body(body)
                .show();
        }
    }

    fn skip_to_break(&mut self) {
        self.mode = TimerMode::Break;
        self.remaining_seconds = self.config.break_duration * 60;
        self.is_running = false;
        self.last_tick = None;
    }

    fn start_call(&mut self) {
        self.call_state.start();
        self.last_tick = Some(Instant::now());
        // Auto-start call recording when the AI feature is enabled
        if self.config.ai_notes_enabled && !self.ai_recording {
            self.toggle_ai_recording_mode(otamot::config::AiMode::Call);
        }
        // Initialize active listening notifications if enabled
        if self.config.active_listening_enabled {
            self.active_listening_next_notification =
                Some(Instant::now() + Duration::from_secs(180));
            self.active_listening_message_index = 0;
        }
    }

    fn end_call(&mut self) {
        let duration = self.call_state.end();
        self.active_listening_next_notification = None;
        // Stop call recording first so its synthesis uses the call prompt
        if self.ai_recording {
            self.toggle_ai_recording_mode(otamot::config::AiMode::Call);
        }
        // Save call notes if there's content
        if self.notes_enabled && !self.notes_content.is_empty() && duration > 0 {
            self.save_call_notes(duration);
        }
    }

    /// Refresh the cached input-device list shown in the settings dropdowns.
    fn refresh_input_device_list(&mut self) {
        self.ai_input_devices = otamot::audio_ai::list_input_devices();
    }

    /// Poll the detached "Test system audio" result and surface it on the
    /// AI status line.
    fn poll_system_audio_test(&mut self) {
        let Some(rx) = &self.system_audio_test_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(rate)) => {
                self.ai_status = Some(AiStatus::Done(format!(
                    "system audio OK via system tap @ {} Hz",
                    rate
                )));
                self.system_audio_test_rx = None;
            }
            Ok(Err(e)) => {
                self.ai_status = Some(AiStatus::Failed(format!("{:#}", e)));
                self.system_audio_test_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.system_audio_test_rx = None;
            }
        }
    }

    /// Poll the global hotkey (registered lazily on first enabled frame).
    fn poll_voice_hotkey(&mut self) {
        if !self.voice_hotkey_registered {
            self.voice_hotkey_registered = true;
            self.voice_hotkey_rx = otamot::global_hotkey::register(&self.config.voice_hotkey);
            if self.voice_hotkey_rx.is_none() {
                eprintln!(
                    "global hotkey '{}' not registered (invalid spec or unsupported)",
                    self.config.voice_hotkey
                );
            }
        }
        if let Some(rx) = &self.voice_hotkey_rx {
            if rx.try_recv().is_ok() {
                self.toggle_ai_recording();
            }
        }
    }

    /// Spawn the AI worker lazily on first use so the feature toggle
    /// fully controls mic access and thread existence.
    fn ensure_ai_worker(&mut self) -> Result<(), String> {
        if self.ai_worker.is_some() {
            return Ok(());
        }
        match AudioAiWorker::spawn(
            self.config.max_recording_minutes,
            self.config.whisper_model_path.clone(),
        ) {
            Ok(worker) => {
                self.ai_event_rx = worker.take_event_receiver();
                self.ai_worker = Some(worker);
                Ok(())
            }
            Err(e) => Err(format!("{:#}", e)),
        }
    }

    fn toggle_ai_recording(&mut self) {
        self.toggle_ai_recording_mode(otamot::config::AiMode::Thoughts);
    }

    fn toggle_ai_recording_mode(&mut self, mode: otamot::config::AiMode) {
        self.ai_active_mode = mode;
        if self.ai_recording {
            // Stopping: hand the buffer over for transcription + synthesis
            if let Some(worker) = &self.ai_worker {
                if let Some(provider) = self.config.active_ai_provider().cloned() {
                    worker.command_tx().send(WorkerCommand::StopRecording).ok();
                    let (prompt, _) = self.config.ai_mode_parts(mode);
                    worker
                        .command_tx()
                        .send(WorkerCommand::TranscribeAndSynthesize {
                            mode,
                            endpoint: provider.endpoint.clone(),
                            kind: provider.kind,
                            api_key: provider.api_key.clone(),
                            model: provider.model.clone(),
                            prompt: prompt.to_string(),
                        })
                        .ok();
                    self.ai_recording = false;
                    self.ai_busy = true;
                    self.ai_status = Some(AiStatus::Transcribing);
                } else {
                    self.ai_status =
                        Some(AiStatus::Failed("no AI provider configured".to_string()));
                }
            }
        } else {
            match self.ensure_ai_worker() {
                Ok(()) => {
                    if let Some(worker) = &self.ai_worker {
                        // Call mode: also open the loopback device so remote
                        // participants are captured (requires e.g. BlackHole)
                        let loopback = match mode {
                            otamot::config::AiMode::Call => {
                                self.config.call_input_device_name.clone()
                            }
                            otamot::config::AiMode::Thoughts => String::new(),
                        };
                        worker
                            .command_tx()
                            .send(WorkerCommand::StartRecording {
                                mode,
                                loopback_device: loopback,
                            })
                            .ok();
                        worker
                            .command_tx()
                            .send(WorkerCommand::SetLiveTranscription {
                                enabled: self.config.live_transcription_enabled,
                                model_path: self.config.whisper_model_path.clone(),
                            })
                            .ok();
                        self.ai_recording = true;
                        self.ai_status = Some(AiStatus::Recording);
                    }
                }
                Err(message) => {
                    self.ai_status = Some(AiStatus::Failed(message));
                }
            }
        }
    }

    fn download_whisper_model(&mut self) {
        match self.ensure_ai_worker() {
            Ok(()) => {
                if let Some(worker) = &self.ai_worker {
                    worker
                        .command_tx()
                        .send(WorkerCommand::DownloadModel {
                            size: self.config.whisper_model_size,
                            target_dir: model_dir_from_path(&self.config.whisper_model_path),
                        })
                        .ok();
                    self.ai_model_download_progress = Some(0.0);
                }
            }
            Err(message) => {
                self.ai_status = Some(AiStatus::Failed(message));
            }
        }
    }

    /// Deterministically create + immediately tear down a system tap:
    /// triggers the macOS TCC prompt from Settings without recording.
    /// Result surfaces on the AI status line.
    fn test_system_audio(&mut self) {
        let endpoint = match self.config.active_ai_provider().cloned() {
            Some(p) => p.endpoint,
            None => String::new(),
        };
        let _ = endpoint; // tap doesn't need the provider; kept for symmetry
        let push: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let rate: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        match otamot::system_audio::SystemAudioCapture::start(push, rate) {
            Ok((capture, sample_rate, method)) => {
                drop(capture);
                self.ai_status = Some(AiStatus::Done(format!(
                    "system audio OK via {} @ {} Hz",
                    method.label(),
                    sample_rate
                )));
            }
            Err(e) => {
                self.ai_status = Some(AiStatus::Failed(format!("{:#}", e)));
            }
        }
    }

    fn fetch_models_for(
        &mut self,
        endpoint: String,
        kind: otamot::config::EndpointKind,
        api_key: String,
    ) {
        match self.ensure_ai_worker() {
            Ok(()) => {
                if let Some(worker) = &self.ai_worker {
                    worker
                        .command_tx()
                        .send(WorkerCommand::FetchModels {
                            endpoint: endpoint.clone(),
                            kind,
                            api_key,
                        })
                        .ok();
                    self.ai_fetching_models = Some(endpoint);
                }
            }
            Err(message) => {
                self.ai_status = Some(AiStatus::Failed(message));
            }
        }
    }

    fn test_endpoint(
        &mut self,
        endpoint: String,
        kind: otamot::config::EndpointKind,
        api_key: String,
    ) {
        match self.ensure_ai_worker() {
            Ok(()) => {
                if let Some(worker) = &self.ai_worker {
                    worker
                        .command_tx()
                        .send(WorkerCommand::TestEndpoint {
                            endpoint,
                            kind,
                            api_key,
                        })
                        .ok();
                }
            }
            Err(message) => {
                self.ai_status = Some(AiStatus::Failed(message));
            }
        }
    }

    /// Whether the configured whisper model file exists on disk.
    fn whisper_model_ready(&self) -> bool {
        std::path::Path::new(&self.config.whisper_model_path).exists()
    }

    /// Drain worker events without blocking. Called every frame.
    fn poll_ai_events(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.ai_event_rx.take() else {
            return;
        };
        let mut repaint = false;
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    repaint = true;
                    match event {
                        WorkerEvent::LiveTranscript { text } => {
                            self.ai_live_transcript = Some(text);
                        }
                        WorkerEvent::RecordingStarted => {
                            self.ai_live_transcript = None;
                            self.ai_status = Some(AiStatus::Recording);
                        }
                        WorkerEvent::RecordingStopped { .. } => {}
                        WorkerEvent::ModelDownloadProgress { percent } => {
                            self.ai_model_download_progress = Some(percent);
                        }
                        WorkerEvent::ModelDownloadDone { path } => {
                            self.config.whisper_model_path = path;
                            self.ai_model_download_progress = None;
                            let _ = self.config.save();
                        }
                        WorkerEvent::ModelsFetched { endpoint, models } => {
                            self.ai_models_cache.insert(endpoint, models);
                            self.ai_fetching_models = None;
                        }
                        WorkerEvent::EndpointTested {
                            endpoint,
                            ok,
                            detail,
                        } => {
                            self.ai_endpoint_status.insert(endpoint.clone(), ok);
                            if ok {
                                self.ai_endpoint_status_detail.remove(&endpoint);
                            } else {
                                self.ai_endpoint_status_detail.insert(endpoint, detail);
                            }
                        }
                        WorkerEvent::EndpointResolved {
                            requested,
                            resolved,
                        } => {
                            // Self-heal: a bare host auto-resolved to /v1 —
                            // persist it so synthesis skips the 404 round-trip
                            for provider in &mut self.config.ai_providers {
                                if provider.endpoint.trim_end_matches('/') == requested {
                                    provider.endpoint = resolved.clone();
                                }
                            }
                            let _ = self.config.save();
                        }
                        WorkerEvent::Transcribed { text } => {
                            self.ai_last_transcript = Some(text);
                            self.ai_status = Some(AiStatus::Synthesizing);
                        }
                        WorkerEvent::Synthesized { mode, markdown } => {
                            self.deliver_synthesis(mode, &markdown);
                            self.ai_busy = false;
                            self.ai_status = Some(AiStatus::Done(markdown));
                        }
                        WorkerEvent::Error { message } => {
                            self.ai_busy = false;
                            self.ai_status = Some(AiStatus::Failed(message));
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.ai_event_rx = Some(rx);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ai_event_rx = None;
                    break;
                }
            }
        }
        if repaint {
            ctx.request_repaint();
        }
    }

    /// Delivery target for synthesized Markdown: writes one timestamped
    /// .md file per recording into the mode's configured directory, and
    /// mirrors the block into the notes draft so it's visible in-app.
    /// Output = summary from the LLM plus (optionally) the timestamped
    /// transcript beneath it.
    fn deliver_synthesis(&mut self, mode: otamot::config::AiMode, markdown: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M");
        let mut block = format!(
            "\n## Voice Notes — {} ({})\n\n{}\n",
            timestamp,
            mode.label(),
            markdown.trim()
        );
        if self.config.include_transcript_in_notes {
            if let Some(transcript) = &self.ai_last_transcript {
                if !transcript.trim().is_empty() {
                    block.push_str(&format!("\n### Transcript\n\n{}\n", transcript.trim()));
                }
            }
        }
        block.push('\n');
        self.write_mode_file(mode, &block);
        // Transcript has been consumed into the note
        self.ai_last_transcript = None;
    }

    /// Write a voice-notes block as a timestamped file in the mode's
    /// directory, creating it (and parents) on first write. Mirrors into
    /// the notes draft so the result is visible in-app.
    fn write_mode_file(&mut self, mode: otamot::config::AiMode, block: &str) {
        let dir = match mode {
            otamot::config::AiMode::Thoughts => self.config.voice_notes_dir.clone(),
            otamot::config::AiMode::Call => self.config.call_records_dir.clone(),
        };
        let dir_path = std::path::PathBuf::from(&dir);
        if let Err(e) = std::fs::create_dir_all(&dir_path) {
            eprintln!("Failed to create {} directory: {}", mode.label(), e);
            self.ai_status = Some(AiStatus::Failed(format!(
                "could not create {} directory",
                mode.label()
            )));
            return;
        }

        let filename = format!(
            "{}-{}.md",
            Local::now().format("%m-%d-%Y-%H-%M-%S"),
            mode.label()
        );
        let path = dir_path.join(&filename);

        use std::io::Write;
        match std::fs::File::create(&path).and_then(|mut f| f.write_all(block.as_bytes())) {
            Ok(()) => {
                // Mirror the block into the notes draft so it's visible in-app
                self.notes_content.push_str(block);
                let _ = notes::save_draft(&self.config.notes_directory, &self.notes_content);
            }
            Err(e) => {
                eprintln!("Failed to write voice notes to {}: {}", path.display(), e);
                self.ai_status = Some(AiStatus::Failed(format!(
                    "could not write to {}",
                    path.display()
                )));
            }
        }
    }

    /// Status line text for the current AI pipeline state.
    fn ai_status_text(&self) -> Option<(String, egui::Color32)> {
        let theme = &self.config.theme;
        let text_color = egui::Color32::from_rgb(theme.text.r, theme.text.g, theme.text.b);
        let dim = egui::Color32::from_rgb(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b);
        let error = egui::Color32::from_rgb(0xe7, 0x4c, 0x3c);
        let recording_label = match self.ai_active_mode {
            otamot::config::AiMode::Thoughts => "● Recording thoughts…",
            otamot::config::AiMode::Call => "● Recording call…",
        };
        match &self.ai_status {
            Some(AiStatus::Recording) => Some((recording_label.to_string(), error)),
            Some(AiStatus::Transcribing) => Some(("Transcribing…".to_string(), dim)),
            Some(AiStatus::Synthesizing) => Some(("Synthesizing notes…".to_string(), dim)),
            Some(AiStatus::Done(_)) => Some(("Voice notes added ✓".to_string(), text_color)),
            Some(AiStatus::Failed(message)) => Some((format!("AI error: {}", message), error)),
            None => None,
        }
    }

    fn save_call_notes(&mut self, duration_seconds: u32) {
        self.hashtag_library.save();

        let notes_dir = std::path::PathBuf::from(&self.config.call_notes_directory);
        if let Err(e) = std::fs::create_dir_all(&notes_dir) {
            eprintln!("Failed to create call notes directory: {}", e);
            return;
        }

        let end_time = chrono::Local::now();
        let start_time = self.call_state.start_time.unwrap_or(end_time);

        let filename = notes::generate_filename(start_time, end_time);
        let filepath = notes_dir.join(&filename);

        // Generate frontmatter for call notes
        let hours = duration_seconds / 3600;
        let minutes = (duration_seconds % 3600) / 60;
        let secs = duration_seconds % 60;
        let duration_str = if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m {}s", minutes, secs)
        };

        let mut tags = vec!["pomodoro".to_string(), "call".to_string()];
        for tag in self.extract_hashtags(&self.notes_content) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        let tags_yaml = tags
            .iter()
            .map(|t| format!("  - {}", t))
            .collect::<Vec<_>>()
            .join("\n");

        let frontmatter = format!(
            "---\ntitle: \"Call Session\"\ndate: {}\nstart_time: {}\nend_time: {}\nduration_seconds: {}\nduration: {}\nmode: call\ntags:\n{}\n---\n\n",
            end_time.format("%Y-%m-%d %H:%M:%S"),
            start_time.format("%Y-%m-%d %H:%M:%S"),
            end_time.format("%Y-%m-%d %H:%M:%S"),
            duration_seconds,
            duration_str,
            tags_yaml
        );

        // Filter out auto-generated project sections
        let mut notes_to_save = self.notes_content.clone();
        if let Some(pos) = notes_to_save.find("# TODO") {
            notes_to_save.truncate(pos);
        }
        if let Some(pos) = notes_to_save.find("# Kanban") {
            notes_to_save.truncate(pos);
        }
        let notes_to_save = notes_to_save.trim().to_string();

        let content = format!("{}{}", frontmatter, notes_to_save);
        if let Err(e) = std::fs::write(&filepath, &content) {
            eprintln!("Failed to write call note file: {}", e);
        } else {
            self.notes_content.clear();
            let _ = notes::clear_draft(&self.config.notes_directory);
        }
    }

    fn submit_survey(&mut self) {
        self.survey_data.add_response(
            self.survey_focus_rating,
            self.survey_what_helped.clone(),
            self.survey_what_hurt.clone(),
        );
        if let Err(e) = self.survey_data.save() {
            eprintln!("Failed to save survey data: {}", e);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.update_tray_info();

        // Reset survey form
        self.survey_focus_rating = 5;
        self.survey_what_helped.clear();
        self.survey_what_hurt.clear();
        self.show_survey = false;
    }

    fn skip_survey(&mut self) {
        self.show_survey = false;
        // Reset survey form
        self.survey_focus_rating = 5;
        self.survey_what_helped.clear();
        self.survey_what_hurt.clear();
    }

    fn reset_survey_data(&mut self) {
        self.survey_data.reset();
        if let Err(e) = self.survey_data.save() {
            eprintln!("Failed to reset survey data: {}", e);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn setup_tray_icon(&mut self) {
        use tray_icon::{
            menu::{Menu, MenuItem, PredefinedMenuItem},
            TrayIconBuilder,
        };

        let info = TrayInfoItems::new(&self.survey_data);

        let tray_menu = Menu::new();
        let start_pause_item = MenuItem::new("Start/Pause", true, None);
        let reset_item = MenuItem::new("Reset", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        self.tray_menu_ids
            .insert("start_pause".to_string(), start_pause_item.id().clone());
        self.tray_menu_ids
            .insert("reset".to_string(), reset_item.id().clone());
        self.tray_menu_ids
            .insert("quit".to_string(), quit_item.id().clone());

        let _ = tray_menu.append_items(&[
            &info.score,
            &info.sessions,
            &PredefinedMenuItem::separator(),
            &info.issue_0,
            &info.issue_1,
            &info.issue_2,
            &PredefinedMenuItem::separator(),
            &start_pause_item,
            &reset_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ]);

        let icon = (|| {
            let icon_bytes = include_bytes!("../assets/icon.png");
            let img = image::load_from_memory(icon_bytes).ok()?;
            let img = img.resize(22, 22, image::imageops::FilterType::Lanczos3);
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).ok()
        })()
        .unwrap_or_else(|| {
            let size = 22u32;
            let mut pixels = vec![0u8; (size * size * 4) as usize];
            let center = (size / 2) as f32;
            let radius = (size / 2 - 2) as f32;
            for y in 0..size {
                for x in 0..size {
                    let dx = x as f32 - center;
                    let dy = y as f32 - center;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let idx = ((y * size + x) * 4) as usize;
                    if dist <= radius {
                        pixels[idx] = 220;
                        pixels[idx + 1] = 20;
                        pixels[idx + 2] = 60;
                        pixels[idx + 3] = 255;
                    } else if dist <= radius + 1.5 {
                        pixels[idx] = 139;
                        pixels[idx + 1] = 0;
                        pixels[idx + 2] = 0;
                        pixels[idx + 3] = 255;
                    }
                }
            }
            tray_icon::Icon::from_rgba(pixels, size, size).unwrap()
        });

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Otamot")
            .with_icon(icon)
            .with_icon_as_template(true)
            .build()
            .unwrap();

        self.tray_info_items = Some(info);
        self.tray_icon = Some(tray_icon);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle_tray_events(&mut self) {
        use tray_icon::menu::MenuEvent;
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.tray_menu_ids.get("start_pause") {
                self.toggle_timer();
            } else if Some(&event.id) == self.tray_menu_ids.get("reset") {
                self.reset_timer();
            } else if Some(&event.id) == self.tray_menu_ids.get("quit") {
                std::process::exit(0);
            }
        }
    }

    /// Refreshes tray overview labels from the current in-memory survey state.
    /// No-op when tray is unavailable (e.g. wasm, or tray setup failed).
    #[cfg(not(target_arch = "wasm32"))]
    fn update_tray_info(&self) {
        if let Some(ref info) = self.tray_info_items {
            info.refresh(&self.survey_data);
        }
    }

    fn tick(&mut self) {
        // Handle call mode ticking
        if self.call_state.is_active {
            if let Some(last) = self.last_tick {
                let elapsed = last.elapsed();
                if elapsed >= Duration::from_secs(1) {
                    self.call_state.tick();
                    self.last_tick = Some(Instant::now());
                    // Active listening notifications
                    if self.config.active_listening_enabled {
                        if let Some(next) = self.active_listening_next_notification {
                            if Instant::now() >= next {
                                const MESSAGES: [&str; 5] = [
                                    "Are you listening?",
                                    "Are you smiling?",
                                    "Nod your head",
                                    "Check your focus",
                                    "Are there any clarifying questions that need to be made?",
                                ];
                                let idx = self.active_listening_message_index % MESSAGES.len();
                                self.send_notification("Active Listening", MESSAGES[idx]);
                                self.active_listening_message_index += 1;
                                self.active_listening_next_notification =
                                    Some(Instant::now() + Duration::from_secs(180));
                            }
                        }
                    }
                }
            }
            return;
        }

        if !self.is_running {
            return;
        }

        if let Some(last) = self.last_tick {
            let elapsed = last.elapsed();
            if elapsed >= Duration::from_secs(1) {
                if self.remaining_seconds > 0 {
                    self.remaining_seconds -= 1;
                } else {
                    // Timer complete - switch modes
                    self.bell.play();

                    let (title, body) = match self.mode {
                        TimerMode::Work => ("Work Session Complete", "Time for a break!"),
                        TimerMode::Break => ("Break Over", "Back to work!"),
                    };
                    self.send_notification(title, body);

                    let previous_mode = self.mode;
                    self.mode = match self.mode {
                        TimerMode::Work => {
                            self.session_end = Some(Local::now());
                            if self.notes_enabled && !self.notes_content.is_empty() {
                                self.save_notes();
                            }
                            self.sessions_completed += 1;
                            self.survey_data.sessions_completed = self.sessions_completed;
                            let _ = self.survey_data.save();
                            #[cfg(not(target_arch = "wasm32"))]
                            self.update_tray_info();
                            self.remaining_seconds = self.config.break_duration * 60;
                            self.session_start = None;
                            self.session_end = None;
                            TimerMode::Break
                        }
                        TimerMode::Break => {
                            self.remaining_seconds = self.config.work_duration * 60;
                            TimerMode::Work
                        }
                    };

                    if previous_mode == TimerMode::Work && self.config.survey_enabled {
                        self.show_survey = true;
                    }
                }
                self.last_tick = Some(Instant::now());
            }
        }
    }

    /// Extract hashtags from text content
    fn extract_hashtags(&self, text: &str) -> Vec<String> {
        let mut tags = Vec::new();
        for word in text.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word
                    .trim_start_matches('#')
                    .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
                    .to_lowercase();
                if !tag.is_empty()
                    && tag.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !tags.contains(&tag)
                {
                    tags.push(tag);
                }
            }
        }
        tags
    }

    fn save_notes(&mut self) {
        self.hashtag_library.save();

        let notes_dir = std::path::PathBuf::from(&self.config.notes_directory);
        if let Err(e) = std::fs::create_dir_all(&notes_dir) {
            eprintln!("Failed to create notes directory: {}", e);
            return;
        }

        let end_time = self.session_end.unwrap_or_else(Local::now);
        let start_time = self.session_start.unwrap_or(end_time);

        let filename = notes::generate_filename(start_time, end_time);
        let filepath = notes_dir.join(&filename);

        // Generate frontmatter
        let mode_str = match self.mode {
            TimerMode::Work => "work",
            TimerMode::Break => "break",
        };
        let mut tags = vec!["pomodoro".to_string(), mode_str.to_string()];
        for tag in self.extract_hashtags(&self.notes_content) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        let tags_yaml = tags
            .iter()
            .map(|t| format!("  - {}", t))
            .collect::<Vec<_>>()
            .join("\n");

        // Filter out auto-generated project sections if they crept into the buffer
        let mut notes_to_save = self.notes_content.clone();
        if let Some(pos) = notes_to_save.find("# TODO") {
            notes_to_save.truncate(pos);
        }
        if let Some(pos) = notes_to_save.find("# Kanban") {
            notes_to_save.truncate(pos);
        }

        let notes_to_save = notes_to_save.trim().to_string();

        let frontmatter = format!(
            "---\ntitle: \"Pomodoro Session\"\ndate: {}\nstart_time: {}\nend_time: {}\nduration_minutes: {}\nmode: {}\nsessions_completed: {}\ntags:\n{}\n---\n\n",
            end_time.format("%Y-%m-%d %H:%M:%S"),
            start_time.format("%Y-%m-%d %H:%M:%S"),
            end_time.format("%Y-%m-%d %H:%M:%S"),
            self.config.work_duration,
            mode_str,
            self.sessions_completed,
            tags_yaml
        );

        let content = format!("{}{}", frontmatter, notes_to_save);
        if let Err(e) = std::fs::write(&filepath, &content) {
            eprintln!("Failed to write note file: {}", e);
        } else {
            // Clean up session notes but leave the project state if it was there
            // Note: we usually cleared everything, let's go back to that clean slate for next session
            self.notes_content.clear();
            let _ = notes::clear_draft(&self.config.notes_directory);
        }
    }

    fn save_project_file(&self) {
        let path = std::path::PathBuf::from(&self.config.todo_file);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let content = if self.notes_view == NotesView::Project {
            self.project_content.clone()
        } else {
            let mut c = String::new();
            c.push_str(&self.todo_list.to_markdown());
            c.push_str("\n\n");
            c.push_str(&self.kanban_board.to_markdown());
            c
        };

        let _ = std::fs::write(path, content);
    }
}

// --- App Trait Implementation ---

impl eframe::App for PomodoroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.handle_tray_events();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(self.t.menu_help(), |ui| {
                    if ui.button(self.t.help_button()).clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                    if ui.button(self.t.menu_about()).clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Essential state updates
        self.tick();

        // Drain AI worker events (non-blocking)
        if self.config.ai_notes_enabled {
            self.poll_ai_events(ctx);
            self.poll_system_audio_test();
            self.poll_voice_hotkey();
            // In-app shortcut (works everywhere; the global one is macOS)
            if self.config.ai_notes_enabled
                && ctx.input(|i| {
                    i.key_pressed(egui::Key::R) && i.modifiers.command && i.modifiers.shift
                })
            {
                self.toggle_ai_recording();
            }
        }

        // Auto-save notes draft if they've changed
        if self.notes_enabled && !self.notes_content.is_empty() {
            let _ = notes::save_draft(&self.config.notes_directory, &self.notes_content);
        }

        if self.is_running || self.call_state.is_active || self.ai_busy || self.ai_recording {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Handle early keyboard input for the notes editor (before UI renders)
        if self.notes_enabled && self.notes_view == NotesView::Edit {
            let is_focused = ctx.memory(|mem| mem.has_focus(egui::Id::new("notes_text_input")));

            if is_focused && self.dropdown_visible {
                if !self.dropdown_items.is_empty() {
                    // Dropdown keyboard navigation
                    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        self.dropdown_selected =
                            (self.dropdown_selected + 1) % self.dropdown_items.len();
                        ctx.input_mut(|i| {
                            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                        });
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        self.dropdown_selected = if self.dropdown_selected == 0 {
                            self.dropdown_items.len() - 1
                        } else {
                            self.dropdown_selected - 1
                        };
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let item = self.dropdown_items[self.dropdown_selected].clone();
                        self.apply_dropdown_selection(item);
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                } else if self.dropdown_type == DropdownType::Hashtag {
                    // Handle Enter for new hashtag (not in library yet)
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let byte_pos = self.get_notes_byte_pos();
                        if let Some((_pos, tag)) =
                            HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, byte_pos)
                        {
                            if !tag.is_empty() {
                                self.apply_dropdown_selection(tag);
                                ctx.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                                });
                            }
                        }
                    }
                }
            }

            let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
            let shift = ctx.input(|i| i.modifiers.shift);

            if tab_pressed && is_focused {
                // Dropdown pagination/navigation via Tab
                if self.dropdown_visible && !self.dropdown_items.is_empty() {
                    if shift {
                        self.dropdown_selected = if self.dropdown_selected == 0 {
                            self.dropdown_items.len() - 1
                        } else {
                            self.dropdown_selected - 1
                        };
                    } else {
                        self.dropdown_selected =
                            (self.dropdown_selected + 1) % self.dropdown_items.len();
                    }
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
                } else if !self.dropdown_visible {
                    // Consume Tab to prevent focus escape
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));

                    // Force focus back to the editor
                    ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("notes_text_input")));

                    // Get byte position from character position
                    let byte_pos = self.get_notes_byte_pos();

                    // Find the start of the current line
                    let line_start = self.notes_content[..byte_pos]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);

                    // Get the full line content
                    let line_end = self.notes_content[byte_pos..]
                        .find('\n')
                        .map(|i| byte_pos + i)
                        .unwrap_or(self.notes_content.len());
                    let full_line = &self.notes_content[line_start..line_end];

                    // Check if this line is a list item (with optional leading spaces)
                    let trimmed = full_line.trim_start();
                    let is_list_item = trimmed.starts_with("- ")
                        || trimmed.starts_with("* ")
                        || (trimmed
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                            && trimmed.contains(". "));

                    if shift {
                        // Handle Outdent (Shift+Tab)
                        let line_content = &self.notes_content[line_start..byte_pos];
                        if line_content.starts_with("  ") {
                            self.notes_content = format!(
                                "{}{}",
                                &self.notes_content[..line_start],
                                &self.notes_content[line_start + 2..]
                            );
                            self.requested_cursor_pos =
                                Some(self.notes_cursor_pos.saturating_sub(2));
                        } else if line_content.starts_with('\t') {
                            self.notes_content = format!(
                                "{}{}",
                                &self.notes_content[..line_start],
                                &self.notes_content[line_start + 1..]
                            );
                            self.requested_cursor_pos =
                                Some(self.notes_cursor_pos.saturating_sub(1));
                        }
                    } else if is_list_item {
                        // Handle Indent (Tab) on list item - insert spaces at line start
                        self.notes_content.insert_str(line_start, "  ");
                        self.requested_cursor_pos = Some(self.notes_cursor_pos + 2);
                    } else {
                        // Handle Indent (Tab) - insert 2 spaces at cursor position
                        self.notes_content.insert_str(byte_pos, "  ");
                        self.requested_cursor_pos = Some(self.notes_cursor_pos + 2);
                    }
                }
            }
        }

        // Check for early keyboard input for Enter in dropdown
        if self.notes_enabled
            && self.notes_view == NotesView::Edit
            && self.dropdown_visible
            && !self.dropdown_items.is_empty()
            && ctx.input(|i| i.key_pressed(egui::Key::Enter))
        {
            let item = self.dropdown_items[self.dropdown_selected].clone();
            self.apply_dropdown_selection(item);
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Comma) && i.modifiers.command) {
            self.show_settings = true;
            self.refresh_input_device_list();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Period) && i.modifiers.command) {
            self.toggle_timer();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::P) && i.modifiers.ctrl) && self.notes_enabled {
            if self.notes_view == NotesView::Edit {
                self.notes_content = format_markdown(&self.notes_content);
            }
            self.notes_view = match self.notes_view {
                NotesView::Edit => NotesView::Preview,
                NotesView::Preview => {
                    self.focus_notes_input = true;
                    NotesView::Edit
                }
                NotesView::Project => {
                    self.focus_notes_input = true;
                    NotesView::Edit
                }
            };
        }

        if ctx.input(|i| i.key_pressed(egui::Key::D) && i.modifiers.ctrl)
            && self.notes_enabled
            && self.notes_view == NotesView::Edit
        {
            self.notes_content = insert_date_bullet(&self.notes_content);
            self.focus_notes_input = true;
            self.requested_cursor_pos = Some(19);
        }

        // CMD/CTRL+K: Insert markdown link syntax []()
        if ctx.input(|i| i.key_pressed(egui::Key::K) && i.modifiers.command)
            && self.notes_enabled
            && self.notes_view == NotesView::Edit
        {
            let is_focused = ctx.memory(|mem| mem.has_focus(egui::Id::new("notes_text_input")));
            if is_focused {
                let byte_pos = self.get_notes_byte_pos();
                self.notes_content.insert_str(byte_pos, "[]()");
                // Position cursor inside the [] brackets (1 character after insertion point)
                self.requested_cursor_pos = Some(self.notes_cursor_pos + 1);
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K));
            }
        }

        // CMD/CTRL+SHIFT+/ (or CMD/CTRL+?) to toggle help
        let help_shortcut = ctx
            .input(|i| i.key_pressed(egui::Key::Slash) && i.modifiers.shift && i.modifiers.command);
        if help_shortcut {
            self.show_help = !self.show_help;
        }

        // Handle Enter key to continue list items
        if ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && self.notes_enabled
            && self.notes_view == NotesView::Edit
            && !self.dropdown_visible
        {
            let is_focused = ctx.memory(|mem| mem.has_focus(egui::Id::new("notes_text_input")));

            if is_focused {
                // Get byte position from character position
                let byte_pos = self.get_notes_byte_pos();

                // Find the start of the current line
                let line_start = self.notes_content[..byte_pos]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);

                let line_content = &self.notes_content[line_start..byte_pos];

                // Check for list markers at the start of the line (with optional leading spaces)
                let trimmed = line_content.trim_start();
                let leading_spaces = line_content.len() - trimmed.len();

                // Check for unordered list (- or *)
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    let marker = &trimmed[..2]; // "- " or "* "
                    let indent = &line_content[..leading_spaces];
                    let new_item = format!("\n{}{}", indent, marker);
                    self.notes_content.insert_str(byte_pos, &new_item);
                    self.requested_cursor_pos =
                        Some(self.notes_cursor_pos + new_item.chars().count());
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                }
                // Check for ordered list (1., 2., 3., etc.)
                else if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                    if rest.starts_with(". ") {
                        // Extract the number from the beginning
                        let num_str = trimmed
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>();
                        if let Ok(num) = num_str.parse::<u32>() {
                            let indent = &line_content[..leading_spaces];
                            let new_item = format!("\n{}{}. ", indent, num + 1);
                            self.notes_content.insert_str(byte_pos, &new_item);
                            self.requested_cursor_pos =
                                Some(self.notes_cursor_pos + new_item.chars().count());
                            ctx.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            });
                        }
                    }
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.dropdown_visible {
                self.dropdown_visible = false;
            } else {
                self.show_settings = false;
                self.show_help = false;
                self.show_survey = false;
                self.show_survey_summary = false;
            }
        }

        // Autocomplete drop-down logic
        if self.notes_enabled && self.notes_view == NotesView::Edit {
            let byte_pos = self.get_notes_byte_pos();
            if !self.dropdown_visible {
                if let Some((pos, cmd)) =
                    CommandManager::find_command_at_cursor(&self.notes_content, byte_pos)
                {
                    self.dropdown_visible = true;
                    self.dropdown_type = DropdownType::Command;
                    self.dropdown_start_pos = pos;
                    self.dropdown_items = self.command_manager.search_commands(&cmd);
                    self.dropdown_selected = 0;
                } else if let Some((pos, tag)) =
                    HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, byte_pos)
                {
                    self.dropdown_visible = true;
                    self.dropdown_type = DropdownType::Hashtag;
                    self.dropdown_start_pos = pos;
                    self.dropdown_items = self.hashtag_library.search(&tag);
                    self.dropdown_selected = 0;
                }
            } else {
                // Update dropdown state as user types
                match self.dropdown_type {
                    DropdownType::Command => {
                        if let Some((pos, cmd)) =
                            CommandManager::find_command_at_cursor(&self.notes_content, byte_pos)
                        {
                            self.dropdown_start_pos = pos;
                            self.dropdown_items = self.command_manager.search_commands(&cmd);
                        } else {
                            self.dropdown_visible = false;
                        }
                    }
                    DropdownType::Hashtag => {
                        if let Some((pos, tag)) =
                            HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, byte_pos)
                        {
                            self.dropdown_start_pos = pos;
                            self.dropdown_items = self.hashtag_library.search(&tag);
                        } else {
                            self.dropdown_visible = false;
                        }
                    }
                }
            }
        }

        // Theme Definitions
        let theme = &self.config.theme;
        let text_color = egui::Color32::from_rgb(theme.text.r, theme.text.g, theme.text.b);
        let text_dim_color =
            egui::Color32::from_rgb(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b);
        let text_highlight_color = egui::Color32::from_rgb(
            theme.text_highlight.r,
            theme.text_highlight.g,
            theme.text_highlight.b,
        );
        let work_color = egui::Color32::from_rgb(theme.work.r, theme.work.g, theme.work.b);
        let break_color =
            egui::Color32::from_rgb(theme.b_break.r, theme.b_break.g, theme.b_break.b);
        let call_color = egui::Color32::from_rgb(0xf5, 0x9e, 0x0b); // Orange/amber for call mode
        let button_color = egui::Color32::from_rgb(theme.button.r, theme.button.g, theme.button.b);
        let bg_color = egui::Color32::from_rgb(theme.bg.r, theme.bg.g, theme.bg.b);
        let tab_active_color =
            egui::Color32::from_rgb(theme.tab_active.r, theme.tab_active.g, theme.tab_active.b);
        let tab_inactive_color = egui::Color32::from_rgb(
            theme.tab_inactive.r,
            theme.tab_inactive.g,
            theme.tab_inactive.b,
        );

        let theme_visuals = if theme.dark_mode {
            egui::Visuals::dark()
        } else {
            let mut light = egui::Visuals::light();
            light.widgets.inactive.bg_fill = egui::Color32::WHITE;
            light.widgets.hovered.bg_fill = egui::Color32::from_gray(240);
            light.widgets.active.bg_fill = egui::Color32::from_gray(230);
            // Light grey border for text boxes in Monokai light
            light.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_gray(200));
            light
        };

        ctx.set_visuals(egui::Visuals {
            window_fill: bg_color,
            panel_fill: bg_color,
            override_text_color: Some(text_color),
            hyperlink_color: tab_active_color,
            ..theme_visuals
        });

        let button_text_color = egui::Color32::from_rgb(
            theme.button_text.r,
            theme.button_text.g,
            theme.button_text.b,
        );

        // Main UI Layout
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(15.0)))
            .show(ctx, |ui| {
                if self.notes_enabled || self.todo_enabled {
                    // Get the full available height before entering horizontal layout
                    let full_height = ui.available_height();
                    let total_width = ui.available_width();
                    let sidebar_width = if self.sidebar_collapsed { 50.0 } else { 200.0 };
                    let right_width = total_width - sidebar_width - 20.0;
                    ui.horizontal_top(|ui| {
                        // Side Pillar 1: Settings and toggles (Sidebar)
                        ui.allocate_ui(egui::vec2(sidebar_width, full_height), |ui| {
                            ui.vertical(|ui| {
                                // Sync Kanban with TODO if enabled
                                if self.kanban_enabled {
                                    self.kanban_board.sync_with_todo(&mut self.todo_list);
                                }

                                egui::ScrollArea::vertical()
                                    .id_salt("sidebar_scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        self.render_sidebar(
                                            ui,
                                            text_color,
                                            button_color,
                                            button_text_color,
                                            text_dim_color,
                                        );
                                    });
                            });
                        });

                        ui.separator();

                        // Side Pillar 2: Timer + Notes area and/or TODOs
                        ui.allocate_ui(egui::vec2(right_width, full_height), |ui| {
                            ui.style_mut().spacing.item_spacing.x = 10.0; // Internal spacing

                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("right_pillar_scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        // Wrap the entire right column in a frame to give
                                        // some consistent right-side padding/breathing room
                                        egui::Frame::none()
                                            .inner_margin(egui::Margin {
                                                left: 0.0,
                                                right: 20.0,
                                                top: 0.0,
                                                bottom: 10.0,
                                            })
                                            .show(ui, |ui| {
                                                self.render_right_column(
                                                    ctx,
                                                    ui,
                                                    text_color,
                                                    text_dim_color,
                                                    text_highlight_color,
                                                    tab_active_color,
                                                    tab_inactive_color,
                                                    button_color,
                                                    button_text_color,
                                                    work_color,
                                                    break_color,
                                                    call_color,
                                                    bg_color,
                                                );
                                            });
                                    });
                            });
                        });
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("pure_timer_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    self.render_pure_timer_layout(
                                        ui,
                                        text_color,
                                        button_color,
                                        button_text_color,
                                        work_color,
                                        break_color,
                                        call_color,
                                        text_dim_color,
                                    );
                                });
                            });
                    });
                }
            });

        // Full-screen / Modal windows
        self.show_settings_dialog(
            ctx,
            text_color,
            text_dim_color,
            button_color,
            button_text_color,
            tab_active_color,
        );
        self.show_help_dialog(ctx, text_color, text_dim_color, button_color);
        self.show_about_dialog(ctx, text_color, text_dim_color, button_color);
        self.show_survey_dialog(
            ctx,
            text_color,
            text_dim_color,
            button_color,
            button_text_color,
            tab_active_color,
        );
        self.show_survey_summary_dialog(ctx, text_color, text_dim_color, button_color);
    }

    /// Disable egui memory persistence to prevent segfaults when switching between
    /// monitors with different DPIs (e.g., Retina to non-Retina displays).
    /// This prevents window position/size corruption when the screen resolution changes.
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

// --- Extended Methods Implementation ---

impl PomodoroApp {
    pub fn apply_dropdown_selection(&mut self, selected_item: String) {
        match self.dropdown_type {
            DropdownType::Command => {
                if let Some(replacement) = self.command_manager.execute(&selected_item) {
                    let cursor_pos = self.get_notes_byte_pos();
                    self.notes_content = CommandManager::insert_command(
                        &self.notes_content,
                        cursor_pos,
                        self.dropdown_start_pos,
                        &replacement,
                    );
                    self.requested_cursor_pos =
                        Some(self.dropdown_start_pos + replacement.chars().count());
                }
            }
            DropdownType::Hashtag => {
                let cursor_pos = self.get_notes_byte_pos();
                self.notes_content = HashtagLibrary::insert_hashtag(
                    &self.notes_content,
                    cursor_pos,
                    self.dropdown_start_pos,
                    &selected_item,
                );
                self.requested_cursor_pos =
                    Some(self.dropdown_start_pos + selected_item.chars().count() + 2);
                self.hashtag_library.add(&selected_item);
            }
        }
        self.dropdown_visible = false;
        self.dropdown_items.clear();
        self.focus_notes_input = true;
    }

    fn render_right_column(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        _text_dim_color: egui::Color32,
        text_highlight_color: egui::Color32,
        active_color: egui::Color32,
        inactive_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
        call_color: egui::Color32,
        bg_color: egui::Color32,
    ) {
        let text_dim = {
            let theme = &self.config.theme;
            egui::Color32::from_rgb(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b)
        };
        // Render Timer at the top of the right column
        self.render_timer(
            ui,
            text_color,
            button_color,
            work_color,
            break_color,
            call_color,
        );

        // AI status line (only when the feature produced something to show)
        if self.config.ai_notes_enabled {
            if let Some((status_text, color)) = self.ai_status_text() {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(status_text).size(13.0).color(color));
                });
            }
            // Live partial transcript while recording
            if self.ai_recording {
                if let Some(live) = &self.ai_live_transcript {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("live:").size(11.0).color(text_dim));
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("ai_live_transcript")
                        .max_height(90.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(live.as_str())
                                        .size(12.0)
                                        .color(text_dim)
                                        .italics(),
                                );
                            });
                        });
                }
            }
            if let Some(progress) = self.ai_model_download_progress {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("Downloading model: {:.0}%", progress))
                            .size(13.0)
                            .color(text_dim),
                    );
                });
            }
        }

        // Render notes section if enabled
        if self.notes_enabled {
            let mut editor = NotesEditor {
                view: &mut self.notes_view,
                content: &mut self.notes_content,
                project_content: &mut self.project_content,
                focus_input: &mut self.focus_notes_input,
                notes_cursor_pos: &mut self.notes_cursor_pos,
                requested_cursor_pos: &mut self.requested_cursor_pos,
                editor: &mut self.editor,
                t: &self.t,
            };

            let todo_file = self.config.todo_file.clone();
            let response = editor.show(
                ctx,
                ui,
                text_color,
                active_color,
                inactive_color,
                text_highlight_color,
                bg_color,
                &todo_file,
            );

            if let Some(action) = response.action {
                match action {
                    NotesAction::NotesChanged => {
                        let _ =
                            notes::save_draft(&self.config.notes_directory, &self.notes_content);
                    }
                    NotesAction::SaveNotes => {
                        self.save_notes();
                    }
                }
            }

            if let Some(output) = response.output {
                self.render_dropdown(ui, &output);
            }
        }

        // Render TODO section if enabled
        if self.todo_enabled {
            if self.notes_enabled {
                ui.add_space(30.0);
            }
            egui::Frame::group(ui.style())
                .fill(bg_color)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(15.0))
                .show(ui, |ui| {
                    if ui_components::render_todo_panel(
                        ui,
                        &mut self.todo_list,
                        &mut self.todo_input,
                        &mut self.kanban_board,
                        &self.t,
                        text_color,
                        button_color,
                        button_text_color,
                    ) {
                        // Update global project file only
                        self.save_project_file();
                    }
                });
        }

        // Render Kanban section if enabled
        if self.kanban_enabled {
            if self.notes_enabled || self.todo_enabled {
                ui.add_space(30.0);
            }
            egui::Frame::group(ui.style())
                .fill(bg_color)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(15.0))
                .show(ui, |ui| {
                    if ui_components::render_kanban_board(
                        ui,
                        &mut self.kanban_board,
                        &mut self.kanban_input,
                        &mut self.todo_list,
                        &self.t,
                        text_color,
                        bg_color,
                        inactive_color,
                        button_text_color,
                    ) {
                        // Update global project file only
                        self.save_project_file();
                    }
                });
        }
    }

    fn render_pure_timer_layout(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
        call_color: egui::Color32,
        _text_dim_color: egui::Color32,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            // Show call timer or regular timer
            let display_time = if self.call_state.is_active {
                self.call_state.format_time()
            } else {
                self.format_time()
            };

            ui.label(
                egui::RichText::new(display_time)
                    .size(48.0)
                    .color(text_color),
            );
            ui.add_space(10.0);

            // Show call mode label or regular mode
            let (label, color) = if self.call_state.is_active {
                (self.t.timer_call(), call_color)
            } else {
                match self.mode {
                    TimerMode::Work => (self.t.timer_work(), work_color),
                    TimerMode::Break => (self.t.timer_break(), break_color),
                }
            };
            ui.label(egui::RichText::new(label).size(20.0).color(color));
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);

                // Timer controls (hidden during call mode)
                if !self.call_state.is_active {
                    let btn = if self.is_running {
                        self.t.pause_button()
                    } else {
                        self.t.start_button()
                    };
                    if ui_components::rounded_button(ui, &btn, button_text_color, button_color)
                        .clicked()
                    {
                        self.toggle_timer();
                    }
                    ui.add_space(10.0);
                    if ui_components::rounded_button(
                        ui,
                        &self.t.reset_button(),
                        button_text_color,
                        button_color,
                    )
                    .clicked()
                    {
                        self.reset_timer();
                    }
                    ui.add_space(10.0);
                    if ui_components::rounded_button(
                        ui,
                        &self.t.button_skip_upper(),
                        button_text_color,
                        button_color,
                    )
                    .clicked()
                    {
                        self.skip_to_break();
                    }
                }

                // Call Mode Button
                ui.add_space(10.0);
                let call_button_color = if self.call_state.is_active {
                    egui::Color32::from_rgb(0xe7, 0x4c, 0x3c) // Red for end call
                } else {
                    egui::Color32::from_rgb(0x27, 0xae, 0x60) // Green for start call
                };
                let call_label = if self.call_state.is_active {
                    self.t.end_call_button()
                } else {
                    self.t.start_call_button()
                };
                if ui_components::rounded_button(
                    ui,
                    &call_label,
                    button_text_color,
                    call_button_color,
                )
                .clicked()
                {
                    if self.call_state.is_active {
                        self.end_call();
                    } else {
                        self.start_call();
                    }
                }
            });
            ui.add_space(40.0);
            if ui_components::rounded_button(
                ui,
                &self.t.settings_btn(),
                button_text_color,
                button_color,
            )
            .clicked()
            {
                self.show_settings = true;
                self.refresh_input_device_list();
            }
            if ui_components::rounded_button(
                ui,
                &self.t.survey_summary_title(),
                button_text_color,
                button_color,
            )
            .clicked()
            {
                self.show_survey_summary = true;
            }
            ui.add_space(10.0);
            let notes_label = if self.notes_enabled {
                self.t.notes_on()
            } else {
                self.t.notes_off()
            };
            if ui_components::rounded_button(ui, &notes_label, button_text_color, button_color)
                .clicked()
            {
                self.notes_enabled = !self.notes_enabled;
                self.config.notes_enabled = self.notes_enabled;
                let _ = self.config.save();
            }
            ui.add_space(10.0);
            let todo_label = if self.todo_enabled {
                self.t.todo_on()
            } else {
                self.t.todo_off()
            };
            if ui_components::rounded_button(ui, &todo_label, button_text_color, button_color)
                .clicked()
            {
                self.todo_enabled = !self.todo_enabled;
                self.config.todo_enabled = self.todo_enabled;
                let _ = self.config.save();
            }
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(self.t.sessions_completed_label(self.sessions_completed))
                    .size(12.0)
                    .color(_text_dim_color),
            );
            if ui_components::rounded_button(
                ui,
                &self.t.help_button(),
                button_text_color,
                button_color,
            )
            .clicked()
            {
                self.show_help = true;
            }
        });
    }

    fn render_dropdown(&mut self, ui: &mut egui::Ui, output: &egui::text_edit::TextEditOutput) {
        if self.dropdown_visible && !self.dropdown_items.is_empty() {
            // Calculate cursor position in screen coordinates
            let mut dropdown_pos = output.response.rect.left_top(); // Default fallback

            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), output.response.id) {
                if let Some(range) = state.cursor.char_range() {
                    let cursor = output.galley.from_ccursor(range.primary);
                    // Get the position of the character, relative to the galley
                    let galley_cursor_rect = output.galley.pos_from_cursor(&cursor);

                    // Convert galley coordinates to screen coordinates
                    // output.galley_pos is the screen position of the top-left of the galley
                    dropdown_pos = output.galley_pos + galley_cursor_rect.left_bottom().to_vec2();

                    // Add a small vertical offset
                    dropdown_pos.y += 2.0;
                }
            }

            egui::Area::new(egui::Id::new("autocomplete_dropdown"))
                .fixed_pos(dropdown_pos)
                .pivot(egui::Align2::LEFT_TOP)
                .interactable(true)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(egui::Color32::from_rgb(0x2a, 0x2a, 0x3e))
                        .show(ui, |ui| {
                            ui.set_min_width(200.0);
                            ui.label(
                                egui::RichText::new(
                                    if self.dropdown_type == DropdownType::Command {
                                        self.t.autocomplete_commands()
                                    } else {
                                        self.t.autocomplete_hashtags()
                                    },
                                )
                                .weak()
                                .size(10.0),
                            );
                            egui::ScrollArea::vertical()
                                .id_salt("dropdown_scroll")
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    let mut selection = None;
                                    for (i, item) in self.dropdown_items.iter().enumerate() {
                                        let text = if self.dropdown_type == DropdownType::Hashtag {
                                            format!("#{}", item)
                                        } else {
                                            format!("/{}", item)
                                        };
                                        if ui
                                            .selectable_label(i == self.dropdown_selected, text)
                                            .clicked()
                                        {
                                            selection = Some(item.clone());
                                        }
                                    }
                                    if let Some(s) = selection {
                                        self.apply_dropdown_selection(s);
                                    }
                                });
                        });
                });
            // Ensure we handle mouse interaction correctly for standard egui Areas
            if ui.input(|i| i.pointer.any_pressed()) && self.dropdown_visible {
                // We let the clicked() check above handle selection,
                // but we need to make sure the Area doesn't block interaction
                // if clicked outside (though Area usually doesn't unless modal)
            }
        }
    }

    pub fn show_settings_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        text_dim_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        tab_active_color: egui::Color32,
    ) {
        if !self.show_settings {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("settings_scroll_area")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(format!("⚙ {}", self.t.settings_title()))
                                .size(28.0)
                                .color(text_color)
                                .strong(),
                        );
                        ui.add_space(30.0);
                        // Work duration slider - auto-save
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} min",
                                    self.t.work_duration(),
                                    self.config.work_duration
                                ))
                                .size(18.0)
                                .color(text_dim_color),
                            );
                            let old_work = self.config.work_duration;
                            ui.add(
                                egui::Slider::new(&mut self.config.work_duration, 1..=60)
                                    .show_value(false),
                            );
                            if self.config.work_duration != old_work {
                                let _ = self.config.save();
                                if !self.is_running {
                                    self.remaining_seconds = self.config.work_duration * 60;
                                }
                            }
                        });
                        ui.add_space(15.0);
                        // Break duration slider - auto-save
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} min",
                                    self.t.break_duration(),
                                    self.config.break_duration
                                ))
                                .size(18.0)
                                .color(text_dim_color),
                            );
                            let old_break = self.config.break_duration;
                            ui.add(
                                egui::Slider::new(&mut self.config.break_duration, 1..=30)
                                    .show_value(false),
                            );
                            if self.config.break_duration != old_break {
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(20.0);
                        // Notes directory - auto-save on change
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.notes_directory())
                                    .size(16.0)
                                    .color(text_dim_color),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let old_dir = self.config.notes_directory.clone();
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.notes_directory)
                                    .desired_width(350.0),
                            );
                            if self.config.notes_directory != old_dir {
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(20.0);
                        // Call notes directory - auto-save on change
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new("Call notes directory:")
                                    .size(16.0)
                                    .color(text_dim_color),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let old_call_dir = self.config.call_notes_directory.clone();
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.call_notes_directory)
                                    .desired_width(350.0),
                            );
                            if self.config.call_notes_directory != old_call_dir {
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(20.0);
                        // TODO file - auto-save on change
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new("TODO/Kanban File")
                                    .size(16.0)
                                    .color(text_dim_color),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let old_todo = self.config.todo_file.clone();
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.todo_file)
                                    .desired_width(350.0),
                            );
                            if self.config.todo_file != old_todo {
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(20.0);
                        // Survey toggle - already auto-saves
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let survey_label = if self.config.survey_enabled {
                                self.t.surveys_on()
                            } else {
                                self.t.surveys_off()
                            };
                            if ui_components::rounded_button(
                                ui,
                                &survey_label,
                                button_text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.config.survey_enabled = !self.config.survey_enabled;
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let active_label = if self.config.active_listening_enabled {
                                "Active Listening: ON"
                            } else {
                                "Active Listening: OFF"
                            };
                            if ui_components::rounded_button(
                                ui,
                                active_label,
                                button_text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.config.active_listening_enabled =
                                    !self.config.active_listening_enabled;
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(20.0);

                        // Bell tune radio - already auto-saves
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.bell_tune_label())
                                    .size(18.0)
                                    .color(text_dim_color),
                            );
                        });
                        let old_bell = self.config.bell_tune;
                        ui.horizontal(|ui| {
                            ui.add_space(60.0);
                            ui.radio_value(
                                &mut self.config.bell_tune,
                                otamot::config::BellTune::Default,
                                self.t.tune_default(),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(60.0);
                            ui.radio_value(
                                &mut self.config.bell_tune,
                                otamot::config::BellTune::LaCukaracha,
                                self.t.tune_cukaracha(),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(60.0);
                            ui.radio_value(
                                &mut self.config.bell_tune,
                                otamot::config::BellTune::IceCreamTruck,
                                self.t.tune_icecream(),
                            );
                        });
                        if self.config.bell_tune != old_bell {
                            let _ = self.config.save();
                            self.bell.set_config(otamot::bell::BellConfig {
                                enabled: self.bell.config().enabled,
                                volume: self.bell.config().volume,
                                duration_ms: self.bell.config().duration_ms,
                                frequency: self.bell.config().frequency,
                                tune: self.config.bell_tune,
                            });
                        }

                        ui.add_space(20.0);
                        // Theme selector - auto-save
                        ui.set_max_width(500.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new("Theme")
                                    .size(18.0)
                                    .color(text_dim_color),
                            );
                            let current_theme = self.config.theme.clone();
                            egui::ComboBox::from_id_salt("theme_selector")
                                .selected_text(&self.config.theme.name)
                                .show_ui(ui, |ui| {
                                    let themes: [Theme; 5] = [
                                        Theme::light(),
                                        Theme::dark(),
                                        Theme::robotic_lime(),
                                        Theme::monokai_dark(),
                                        Theme::monokai_light(),
                                    ];
                                    for theme in themes {
                                        ui.selectable_value(
                                            &mut self.config.theme,
                                            theme.clone(),
                                            &theme.name,
                                        );
                                    }
                                });
                            if self.config.theme != current_theme {
                                let _ = self.config.save();
                                ctx.request_repaint();
                            }
                        });
                        ui.add_space(20.0);
                        // Language selector - auto-save
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.language_setting())
                                    .size(18.0)
                                    .color(text_dim_color),
                            );
                            let current_lang = self.config.language;
                            egui::ComboBox::from_id_salt("language_selector")
                                .selected_text(match self.config.language {
                                    Language::English => self.t.lang_en(),
                                    Language::German => self.t.lang_de(),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.language,
                                        Language::English,
                                        self.t.lang_en(),
                                    );
                                    ui.selectable_value(
                                        &mut self.config.language,
                                        Language::German,
                                        self.t.lang_de(),
                                    );
                                });
                            if self.config.language != current_lang {
                                self.t = T::new(self.config.language);
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(20.0);

                        // ============ Audio AI Notes section ============
                        // (toggleable feature — section only when enabled)
                        if self.config.ai_notes_enabled {
                            ui.separator();
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("🎙 Voice Notes AI")
                                    .size(20.0)
                                    .color(text_color)
                                    .strong(),
                            );
                            ui.add_space(10.0);

                            // Whisper model size + download
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Whisper model")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                                let current_size = self.config.whisper_model_size;
                                egui::ComboBox::from_id_salt("whisper_size_selector")
                                    .selected_text(self.config.whisper_model_size.label())
                                    .show_ui(ui, |ui| {
                                        for size in otamot::config::WhisperSize::all() {
                                            ui.selectable_value(
                                                &mut self.config.whisper_model_size,
                                                size,
                                                size.label(),
                                            );
                                        }
                                    });
                                if self.config.whisper_model_size != current_size {
                                    // Point the model path at the newly chosen size
                                    let dir = model_dir_from_path(&self.config.whisper_model_path);
                                    self.config.whisper_model_path = std::path::Path::new(&dir)
                                        .join(self.config.whisper_model_size.file_name())
                                        .to_string_lossy()
                                        .into_owned();
                                    let _ = self.config.save();
                                }
                                let downloading = self.ai_model_download_progress.is_some();
                                let dl_label = if downloading {
                                    "Downloading…"
                                } else {
                                    "Download model"
                                };
                                if ui_components::small_rounded_button(
                                    ui,
                                    dl_label,
                                    button_text_color,
                                    button_color,
                                )
                                .clicked()
                                    && !downloading
                                {
                                    self.download_whisper_model();
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                let model_ready = self.whisper_model_ready();
                                let (checkbox, color) = if model_ready {
                                    ("☑", egui::Color32::from_rgb(0x27, 0xae, 0x60))
                                } else {
                                    ("☐", egui::Color32::from_rgb(0xe7, 0x4c, 0x3c))
                                };
                                ui.label(egui::RichText::new(checkbox).size(16.0).color(color));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Model file: {}",
                                        self.config.whisper_model_path
                                    ))
                                    .size(12.0)
                                    .color(text_dim_color),
                                );
                            });
                            if let Some(progress) = self.ai_model_download_progress {
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    ui.add(
                                        egui::ProgressBar::new(progress / 100.0)
                                            .show_percentage()
                                            .desired_width(350.0),
                                    );
                                });
                            }
                            ui.add_space(15.0);

                            // Provider profiles
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("AI Providers")
                                        .size(18.0)
                                        .color(text_dim_color),
                                );
                            });
                            let mut provider_changed = false;
                            let mut remove_index: Option<usize> = None;
                            let mut test_request: Option<(
                                String,
                                otamot::config::EndpointKind,
                                String,
                            )> = None;
                            let mut fetch_request: Option<(
                                String,
                                otamot::config::EndpointKind,
                                String,
                            )> = None;
                            // Snapshot endpoint status/fetching state before the loop
                            // to avoid borrowing self inside the iter_mut scope
                            let endpoint_status = self.ai_endpoint_status.clone();
                            let fetching_now = self.ai_fetching_models.clone();
                            for (idx, provider) in self.config.ai_providers.iter_mut().enumerate() {
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    // Active provider radio
                                    let is_active = self.config.active_ai_provider == provider.name;
                                    if ui.radio(is_active, "").clicked()
                                        && !provider.name.is_empty()
                                    {
                                        self.config.active_ai_provider = provider.name.clone();
                                        provider_changed = true;
                                    }
                                    ui.add(
                                        egui::TextEdit::singleline(&mut provider.name)
                                            .desired_width(120.0)
                                            .hint_text("name"),
                                    );
                                    let old_kind = provider.kind;
                                    egui::ComboBox::from_id_salt(format!("kind_{}", idx))
                                        .selected_text(provider.kind.label())
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut provider.kind,
                                                otamot::config::EndpointKind::OpenAiCompatible,
                                                "OpenAI-compatible",
                                            );
                                            ui.selectable_value(
                                                &mut provider.kind,
                                                otamot::config::EndpointKind::Anthropic,
                                                "Anthropic",
                                            );
                                        });
                                    if provider.kind != old_kind {
                                        provider_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    let old_endpoint = provider.endpoint.clone();
                                    ui.add(
                                        egui::TextEdit::singleline(&mut provider.endpoint)
                                            .desired_width(220.0)
                                            .hint_text("http://localhost:11434/v1"),
                                    );
                                    if provider.endpoint != old_endpoint {
                                        provider_changed = true;
                                    }
                                    let old_key = provider.api_key.clone();
                                    ui.add(
                                        egui::TextEdit::singleline(&mut provider.api_key)
                                            .desired_width(120.0)
                                            .password(true)
                                            .hint_text("api key"),
                                    );
                                    if provider.api_key != old_key {
                                        provider_changed = true;
                                    }
                                    let old_model = provider.model.clone();
                                    // Model field with dropdown of fetched models when available
                                    if let Some(models) =
                                        self.ai_models_cache.get(&provider.endpoint)
                                    {
                                        if !models.is_empty() {
                                            let mut selected = provider.model.clone();
                                            egui::ComboBox::from_id_salt(format!(
                                                "model_{}_{}",
                                                idx, provider.endpoint
                                            ))
                                            .selected_text(if selected.is_empty() {
                                                "pick model".to_string()
                                            } else {
                                                selected.clone()
                                            })
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    for m in models {
                                                        ui.selectable_value(
                                                            &mut selected,
                                                            m.clone(),
                                                            m,
                                                        );
                                                    }
                                                },
                                            );
                                            if selected != provider.model {
                                                provider.model = selected;
                                                provider_changed = true;
                                            }
                                        } else {
                                            ui.add(
                                                egui::TextEdit::singleline(&mut provider.model)
                                                    .desired_width(110.0)
                                                    .hint_text("model"),
                                            );
                                            if provider.model != old_model {
                                                provider_changed = true;
                                            }
                                        }
                                    } else {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut provider.model)
                                                .desired_width(110.0)
                                                .hint_text("model"),
                                        );
                                        if provider.model != old_model {
                                            provider_changed = true;
                                        }
                                    }
                                    // Test endpoint button with green check feedback
                                    let tested = endpoint_status.get(&provider.endpoint);
                                    let test_label = match tested {
                                        Some(true) => "✓",
                                        Some(false) => "✗",
                                        None => "Test",
                                    };
                                    let test_color = match tested {
                                        Some(true) => egui::Color32::from_rgb(0x27, 0xae, 0x60),
                                        Some(false) => egui::Color32::from_rgb(0xe7, 0x4c, 0x3c),
                                        None => button_color,
                                    };
                                    if ui_components::small_rounded_button(
                                        ui,
                                        test_label,
                                        button_text_color,
                                        test_color,
                                    )
                                    .clicked()
                                    {
                                        test_request = Some((
                                            provider.endpoint.clone(),
                                            provider.kind,
                                            provider.api_key.clone(),
                                        ));
                                    }
                                    // Fetch model list button
                                    let fetching =
                                        fetching_now.as_deref() == Some(provider.endpoint.as_str());
                                    let fetch_label = if fetching { "…" } else { "↻" };
                                    if ui_components::small_rounded_button(
                                        ui,
                                        fetch_label,
                                        button_text_color,
                                        button_color,
                                    )
                                    .clicked()
                                        && !fetching
                                    {
                                        fetch_request = Some((
                                            provider.endpoint.clone(),
                                            provider.kind,
                                            provider.api_key.clone(),
                                        ));
                                    }
                                    if ui.small_button("✕").clicked() {
                                        remove_index = Some(idx);
                                        provider_changed = true;
                                    }
                                });
                                // Inline failure detail under the row
                                let tested_now = endpoint_status.get(&provider.endpoint).copied();
                                if tested_now == Some(false) {
                                    if let Some(detail) =
                                        self.ai_endpoint_status_detail.get(&provider.endpoint)
                                    {
                                        ui.horizontal(|ui| {
                                            ui.add_space(100.0);
                                            ui.label(
                                                egui::RichText::new(detail).size(11.0).color(
                                                    egui::Color32::from_rgb(0xe7, 0x4c, 0x3c),
                                                ),
                                            );
                                        });
                                    }
                                }
                            }
                            if let Some((endpoint, kind, api_key)) = test_request {
                                self.ai_endpoint_status.remove(&endpoint);
                                self.ai_endpoint_status_detail.remove(&endpoint);
                                self.test_endpoint(endpoint, kind, api_key);
                            }
                            if let Some((endpoint, kind, api_key)) = fetch_request {
                                self.fetch_models_for(endpoint, kind, api_key);
                            }
                            if let Some(idx) = remove_index {
                                if self.config.ai_providers.len() > 1 {
                                    let removed = self.config.ai_providers.remove(idx);
                                    if self.config.active_ai_provider == removed.name {
                                        self.config.active_ai_provider = self
                                            .config
                                            .ai_providers
                                            .first()
                                            .map(|p| p.name.clone())
                                            .unwrap_or_default();
                                    }
                                }
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                if ui_components::small_rounded_button(
                                    ui,
                                    "+ Add provider",
                                    button_text_color,
                                    button_color,
                                )
                                .clicked()
                                {
                                    let n = self.config.ai_providers.len() + 1;
                                    self.config.ai_providers.push(
                                        otamot::config::AiProviderConfig {
                                            name: format!("Provider {}", n),
                                            ..Default::default()
                                        },
                                    );
                                    provider_changed = true;
                                }
                            });
                            if provider_changed {
                                let _ = self.config.save();
                            }
                            ui.add_space(15.0);

                            // Synthesis prompt editor
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Synthesis prompt")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                let old_prompt = self.config.synthesis_prompt.clone();
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.config.synthesis_prompt)
                                        .desired_width(430.0)
                                        .desired_rows(4),
                                );
                                if self.config.synthesis_prompt != old_prompt {
                                    let _ = self.config.save();
                                }
                            });
                            ui.add_space(15.0);

                            // Call recorder prompt editor
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Call prompt")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                let old_call_prompt = self.config.call_prompt.clone();
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.config.call_prompt)
                                        .desired_width(430.0)
                                        .desired_rows(4),
                                );
                                if self.config.call_prompt != old_call_prompt {
                                    let _ = self.config.save();
                                }
                            });
                            ui.add_space(15.0);

                            // Input device pickers (per mode). Call mode
                            // should point at a loopback device (e.g.
                            // BlackHole) to capture remote participants.
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Thoughts input device:")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                                let current = if self.config.input_device_name.is_empty() {
                                    "(system default)".to_string()
                                } else {
                                    self.config.input_device_name.clone()
                                };
                                egui::ComboBox::from_id_salt("thoughts_input_device")
                                    .selected_text(&current)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.config.input_device_name,
                                            String::new(),
                                            "(system default)",
                                        );
                                        for name in &self.ai_input_devices {
                                            if name != &"(system default)".to_string() {
                                                ui.selectable_value(
                                                    &mut self.config.input_device_name,
                                                    name.clone(),
                                                    name,
                                                );
                                            }
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Call input device (loopback):")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                                let current = if self.config.call_input_device_name.is_empty() {
                                    "(none — mic only)".to_string()
                                } else {
                                    self.config.call_input_device_name.clone()
                                };
                                egui::ComboBox::from_id_salt("call_input_device")
                                    .selected_text(&current)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.config.call_input_device_name,
                                            String::new(),
                                            "(none — mic only)",
                                        );
                                        for name in &self.ai_input_devices {
                                            if name != &"(none — mic only)".to_string() {
                                                ui.selectable_value(
                                                    &mut self.config.call_input_device_name,
                                                    name.clone(),
                                                    name,
                                                );
                                            }
                                        }
                                    });
                            });
                            ui.add_space(15.0);

                            // Output directories (per mode)
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Thoughts output dir:")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                let old_dir = self.config.voice_notes_dir.clone();
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.voice_notes_dir)
                                        .desired_width(350.0),
                                );
                                if self.config.voice_notes_dir != old_dir {
                                    let _ = self.config.save();
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Call records dir:")
                                        .size(16.0)
                                        .color(text_dim_color),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                let old_dir = self.config.call_records_dir.clone();
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.call_records_dir)
                                        .desired_width(350.0),
                                );
                                if self.config.call_records_dir != old_dir {
                                    let _ = self.config.save();
                                }
                            });
                            ui.add_space(15.0);

                            // Test system audio (triggers TCC prompt on
                            // first use without starting a recording)
                            if ui_components::small_rounded_button(
                                ui,
                                "Test system audio",
                                button_text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.test_system_audio();
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Max recording: {} min",
                                        self.config.max_recording_minutes
                                    ))
                                    .size(16.0)
                                    .color(text_dim_color),
                                );
                                let old_cap = self.config.max_recording_minutes;
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.config.max_recording_minutes,
                                        5..=120,
                                    )
                                    .show_value(false),
                                );
                                if self.config.max_recording_minutes != old_cap {
                                    let _ = self.config.save();
                                }
                            });
                            ui.add_space(15.0);

                            // Live transcription + transcript inclusion toggles
                            ui.horizontal(|ui| {
                                ui.add_space(40.0);
                                let live_label = if self.config.live_transcription_enabled {
                                    "Live transcription: ON"
                                } else {
                                    "Live transcription: OFF"
                                };
                                if ui_components::small_rounded_button(
                                    ui,
                                    live_label,
                                    button_text_color,
                                    button_color,
                                )
                                .clicked()
                                {
                                    self.config.live_transcription_enabled =
                                        !self.config.live_transcription_enabled;
                                    let _ = self.config.save();
                                }
                                let inc_label = if self.config.include_transcript_in_notes {
                                    "Include transcript: ON"
                                } else {
                                    "Include transcript: OFF"
                                };
                                if ui_components::small_rounded_button(
                                    ui,
                                    inc_label,
                                    button_text_color,
                                    button_color,
                                )
                                .clicked()
                                {
                                    self.config.include_transcript_in_notes =
                                        !self.config.include_transcript_in_notes;
                                    let _ = self.config.save();
                                }
                            });
                            ui.add_space(20.0);
                        }

                        // AI notes feature toggle (always visible so it can be turned on)
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let ai_label = if self.config.ai_notes_enabled {
                                "🎙 Voice Notes AI: ON"
                            } else {
                                "🎙 Voice Notes AI: OFF"
                            };
                            if ui_components::rounded_button(
                                ui,
                                ai_label,
                                button_text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.config.ai_notes_enabled = !self.config.ai_notes_enabled;
                                // Stopping the feature also stops any active recording
                                if !self.config.ai_notes_enabled && self.ai_recording {
                                    self.toggle_ai_recording();
                                }
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(40.0);
                        // Close button
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            if ui_components::rounded_button(
                                ui,
                                "Close",
                                button_text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.show_settings = false;
                            }
                        });
                        ui.add_space(20.0);
                    });
                });
        });
    }

    pub fn show_survey_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        text_dim_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        tab_active_color: egui::Color32,
    ) {
        if !self.show_survey {
            return;
        }
        egui::Window::new(format!("{} 🎉", self.t.survey_complete_title()))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("survey_scroll_area")
                    .show(ui, |ui| {
                        ui.set_min_width(400.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_focus())
                                .size(16.0)
                                .color(text_color),
                        );
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(
                                    self.t.survey_rating_label(self.survey_focus_rating),
                                )
                                .color(text_dim_color),
                            );
                            if ui_components::icon_button(ui, "-", text_color, button_color)
                                .clicked()
                            {
                                self.survey_focus_rating =
                                    self.survey_focus_rating.saturating_sub(1).max(1);
                            }
                            if ui_components::icon_button(ui, "+", text_color, button_color)
                                .clicked()
                            {
                                self.survey_focus_rating =
                                    self.survey_focus_rating.saturating_add(1).min(10);
                            }
                        });
                        ui.add_space(15.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_helped())
                                .color(text_dim_color),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.survey_what_helped)
                                .desired_width(350.0)
                                .hint_text(self.t.helped_hint()),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_hurt())
                                .color(text_dim_color),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.survey_what_hurt)
                                .desired_width(350.0)
                                .hint_text(self.t.hurt_hint()),
                        );
                        ui.add_space(20.0);
                        if self.survey_data.focus_count > 0 {
                            ui.separator();
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(
                                    self.t.avg_focus_today(self.survey_data.average_focus_today),
                                )
                                .size(12.0)
                                .color(text_dim_color),
                            );
                            ui.label(
                                egui::RichText::new(
                                    self.t.avg_focus_overall(self.survey_data.average_focus),
                                )
                                .size(12.0)
                                .color(text_dim_color),
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui_components::rounded_button(
                                ui,
                                &self.t.button_skip(),
                                button_text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.skip_survey();
                            }
                            if ui_components::rounded_button(
                                ui,
                                &self.t.button_submit(),
                                button_text_color,
                                tab_active_color,
                            )
                            .clicked()
                            {
                                self.submit_survey();
                            }
                        });
                    });
            });
    }

    pub fn show_survey_summary_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        text_dim_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_survey_summary {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(self.t.survey_summary_title())
                        .size(28.0)
                        .color(text_color)
                        .strong(),
                );
                ui.add_space(20.0);
            });
            egui::ScrollArea::vertical()
                .id_salt("survey_summary_scroll")
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(500.0);
                        if self.survey_data.focus_count == 0 {
                            ui.label(
                                egui::RichText::new(self.t.no_survey_data())
                                    .size(16.0)
                                    .color(text_dim_color),
                            );
                        } else {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(self.t.focus_ratings())
                                        .size(16.0)
                                        .strong()
                                        .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        self.t
                                            .avg_focus_today(self.survey_data.average_focus_today),
                                    )
                                    .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        self.t.avg_focus_overall(self.survey_data.average_focus),
                                    )
                                    .color(text_color),
                                );
                            });
                            ui.add_space(25.0);
                            if !self.survey_data.what_helped.is_empty() {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(self.t.how_helped())
                                            .size(16.0)
                                            .strong()
                                            .color(text_color),
                                    );
                                    for item in &self.survey_data.what_helped {
                                        ui.label(
                                            egui::RichText::new(format!("• {}", item))
                                                .color(text_dim_color),
                                        );
                                    }
                                });
                            }
                            ui.add_space(25.0);
                            if !self.survey_data.what_hurt.is_empty() {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(self.t.how_hurt())
                                            .size(16.0)
                                            .strong()
                                            .color(text_color),
                                    );
                                    for item in &self.survey_data.what_hurt {
                                        ui.label(
                                            egui::RichText::new(format!("• {}", item))
                                                .color(text_dim_color),
                                        );
                                    }
                                });
                            }
                        }
                        ui.add_space(30.0);
                        ui.separator();
                        ui.add_space(15.0);
                        if ui_components::rounded_button(
                            ui,
                            &self.t.button_close(),
                            text_color,
                            button_color,
                        )
                        .clicked()
                        {
                            self.show_survey_summary = false;
                        }
                    });
                });
        });
    }

    pub fn show_help_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        _text_dim_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_help {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(self.t.keyboard_shortcuts_title())
                        .size(28.0)
                        .color(text_color)
                        .strong(),
                );
                ui.add_space(20.0);
            });
            egui::ScrollArea::vertical()
                .id_salt("help_shortcuts_scroll")
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(500.0);
                        let shortcuts = [
                            (
                                self.t.help_timer_title(),
                                vec![
                                    ("Space", self.t.shortcut_start_pause()),
                                    ("R", self.t.shortcut_reset()),
                                    ("Cmd/Ctrl + .", self.t.shortcut_start_pause()),
                                ],
                            ),
                            (
                                self.t.help_notes_title(),
                                vec![
                                    ("Ctrl+P", self.t.shortcut_format()),
                                    ("Ctrl+D", self.t.shortcut_bullet()),
                                    ("Tab", self.t.shortcut_indent()),
                                    ("/", self.t.shortcut_slash()),
                                    ("#", self.t.shortcut_hashtag()),
                                ],
                            ),
                            (
                                self.t.help_general_title(),
                                vec![
                                    ("Ctrl+?", self.t.shortcut_toggle_help()),
                                    ("Cmd/Ctrl + Shift + /", self.t.shortcut_toggle_help()),
                                    ("Cmd/Ctrl + ,", self.t.shortcut_settings()),
                                ],
                            ),
                        ];
                        for (title, list) in shortcuts {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(title)
                                        .size(16.0)
                                        .strong()
                                        .color(text_color),
                                );
                                for (key, action) in list {
                                    ui.horizontal(|ui| {
                                        ui.add_space(10.0);
                                        ui.label(
                                            egui::RichText::new(format!("{:<15}", key))
                                                .monospace()
                                                .color(egui::Color32::from_rgb(0x88, 0xcc, 0xff)),
                                        );
                                        ui.label(egui::RichText::new(action).color(text_color));
                                    });
                                }
                            });
                            ui.add_space(20.0);
                        }
                        ui.separator();
                        if ui_components::rounded_button(
                            ui,
                            &self.t.button_close(),
                            text_color,
                            button_color,
                        )
                        .clicked()
                        {
                            self.show_help = false;
                        }
                    });
                });
        });
    }

    pub fn show_about_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        _text_dim_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_about {
            return;
        }

        let version = env!("CARGO_PKG_VERSION");
        let release_date = "2026-03-05"; // For now, we manually set this

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new(self.t.about_title())
                        .size(32.0)
                        .color(text_color)
                        .strong(),
                );
                ui.add_space(20.0);
                ui.label(egui::RichText::new(self.t.about_description()).color(text_color));
                ui.add_space(20.0);
                ui.label(egui::RichText::new(self.t.about_version(version)).color(text_color));
                ui.label(
                    egui::RichText::new(self.t.about_release_date(release_date)).color(text_color),
                );
                ui.add_space(40.0);

                if ui_components::rounded_button(
                    ui,
                    &self.t.button_close(),
                    text_color,
                    button_color,
                )
                .clicked()
                {
                    self.show_about = false;
                }
            });
        });
    }
}

#[cfg(test)]
mod tray_label_tests {
    use super::*;

    #[test]
    fn test_format_tray_score_with_data() {
        assert_eq!(format_tray_score(7.4, 3), "Survey Score: 7.4");
    }

    #[test]
    fn test_format_tray_score_no_data() {
        assert_eq!(format_tray_score(0.0, 0), "Survey Score: —");
    }

    #[test]
    fn test_format_tray_sessions() {
        assert_eq!(format_tray_sessions(12), "Sessions Tracked: 12");
    }

    #[test]
    fn test_format_tray_issue_present() {
        assert_eq!(
            format_tray_issue(Some("Slack notifications")),
            "• Slack notifications"
        );
    }

    #[test]
    fn test_format_tray_issue_absent() {
        assert_eq!(format_tray_issue(None), "—");
    }
}
