use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Represents the current view for notes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesView {
    Edit,
    Preview,
    Project,
}

/// Represents the available languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    German,
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

/// A color representation for serialization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CustomColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Theme configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Theme {
    pub name: String,
    pub dark_mode: bool,
    pub text: CustomColor,
    pub text_dim: CustomColor,
    pub text_highlight: CustomColor,
    pub button_text: CustomColor,
    pub work: CustomColor,
    pub b_break: CustomColor,
    pub button: CustomColor,
    pub bg: CustomColor,
    pub tab_active: CustomColor,
    pub tab_inactive: CustomColor,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::robotic_lime()
    }
}

impl Theme {
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            dark_mode: false,
            text: CustomColor::new(0x2d, 0x2d, 0x2d),
            text_dim: CustomColor::new(0x88, 0x88, 0x88),
            text_highlight: CustomColor::new(0x2d, 0x2d, 0x2d),
            button_text: CustomColor::new(0xff, 0xff, 0xff),
            work: CustomColor::new(0xe7, 0x4c, 0x3c),
            b_break: CustomColor::new(0x27, 0xae, 0x60),
            button: CustomColor::new(0x3b, 0x82, 0xf6),
            bg: CustomColor::new(0xff, 0xff, 0xff),
            tab_active: CustomColor::new(0x3b, 0x82, 0xf6),
            tab_inactive: CustomColor::new(0xe5, 0xe7, 0xeb),
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            dark_mode: true,
            text: CustomColor::new(0xee, 0xee, 0xee),
            text_dim: CustomColor::new(0x88, 0x88, 0x88),
            text_highlight: CustomColor::new(0xff, 0xff, 0xff),
            button_text: CustomColor::new(0xee, 0xee, 0xee),
            work: CustomColor::new(0xe7, 0x4c, 0x3c),
            b_break: CustomColor::new(0x27, 0xae, 0x60),
            button: CustomColor::new(0x0f, 0x34, 0x60),
            bg: CustomColor::new(0x1a, 0x1a, 0x2e),
            tab_active: CustomColor::new(0x27, 0xae, 0x60),
            tab_inactive: CustomColor::new(0x0f, 0x34, 0x60),
        }
    }

    pub fn robotic_lime() -> Self {
        Self {
            name: "Robotic Lime".to_string(),
            dark_mode: true,
            text: CustomColor::new(0x00, 0xff, 0x00),
            text_dim: CustomColor::new(0x00, 0x88, 0x00),
            text_highlight: CustomColor::new(0x0a, 0x0a, 0x0a),
            button_text: CustomColor::new(0x0a, 0x0a, 0x0a), // Clean black for robotic green buttons
            work: CustomColor::new(0xcc, 0xff, 0x00),
            b_break: CustomColor::new(0x00, 0xcc, 0x00),
            button: CustomColor::new(0x00, 0xff, 0x00),
            bg: CustomColor::new(0x05, 0x05, 0x05),
            tab_active: CustomColor::new(0x00, 0xff, 0x00),
            tab_inactive: CustomColor::new(0x00, 0x22, 0x00),
        }
    }

    pub fn monokai_dark() -> Self {
        Self {
            name: "Monokai Dark".to_string(),
            dark_mode: true,
            text: CustomColor::new(0xF8, 0xF8, 0xF2),
            text_dim: CustomColor::new(0x75, 0x71, 0x5E),
            text_highlight: CustomColor::new(0x27, 0x28, 0x22),
            button_text: CustomColor::new(0xF8, 0xF8, 0xF2),
            work: CustomColor::new(0xF9, 0x26, 0x72),
            b_break: CustomColor::new(0xA6, 0xE2, 0x2E),
            button: CustomColor::new(0x49, 0x48, 0x3E),
            bg: CustomColor::new(0x27, 0x28, 0x22),
            tab_active: CustomColor::new(0xFD, 0x97, 0x1F),
            tab_inactive: CustomColor::new(0x3E, 0x3D, 0x32),
        }
    }

    pub fn monokai_light() -> Self {
        Self {
            name: "Monokai Light".to_string(),
            dark_mode: false,
            text: CustomColor::new(0x27, 0x28, 0x22),
            text_dim: CustomColor::new(0x75, 0x71, 0x5E),
            text_highlight: CustomColor::new(0x0a, 0x0a, 0x0a),
            button_text: CustomColor::new(0xFF, 0xFF, 0xFF), // Pure white text on colored buttons
            work: CustomColor::new(0xF9, 0x26, 0x72),
            b_break: CustomColor::new(0x74, 0xbc, 0x44),
            button: CustomColor::new(0x38, 0x97, 0xd8), // Blue button
            bg: CustomColor::new(0xFF, 0xFF, 0xFF),     // Pure white background
            tab_active: CustomColor::new(0xAE, 0x81, 0xFF),
            tab_inactive: CustomColor::new(0xE6, 0xE6, 0xE6),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_work_duration")]
    pub work_duration: u32,
    #[serde(default = "default_break_duration")]
    pub break_duration: u32,
    #[serde(default = "default_notes_directory")]
    pub notes_directory: String,
    #[serde(default = "default_notes_directory")]
    pub call_notes_directory: String,
    #[serde(default = "default_todo_file")]
    pub todo_file: String,
    #[serde(default)]
    pub notes_enabled: bool,
    #[serde(default = "default_survey_enabled")]
    pub survey_enabled: bool,
    #[serde(default)]
    pub language: Language,
    #[serde(default = "default_slash_commands")]
    pub slash_commands: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub todo_enabled: bool,
    #[serde(default)]
    pub kanban_enabled: bool,
    #[serde(default)]
    pub sidebar_collapsed: bool,
    #[serde(default)]
    pub active_listening_enabled: bool,
    #[serde(default)]
    pub bell_tune: BellTune,
    #[serde(default)]
    pub theme: Theme,
    // Audio AI notes feature — toggleable, off by default
    #[serde(default)]
    pub ai_notes_enabled: bool,
    #[serde(default = "default_ai_providers")]
    pub ai_providers: Vec<AiProviderConfig>,
    #[serde(default = "default_active_ai_provider")]
    pub active_ai_provider: String,
    #[serde(default = "default_whisper_model_path")]
    pub whisper_model_path: String,
    #[serde(default)]
    pub whisper_model_size: WhisperSize,
    #[serde(default = "default_synthesis_prompt")]
    pub synthesis_prompt: String,
    #[serde(default = "default_max_recording_minutes")]
    pub max_recording_minutes: u32,
    /// Stream partial transcripts to the UI while recording
    #[serde(default = "default_true")]
    pub live_transcription_enabled: bool,
    /// Append the raw transcript under the synthesized summary
    #[serde(default = "default_true")]
    pub include_transcript_in_notes: bool,
    /// Directory for thought-recorder outputs (one .md per recording)
    #[serde(default = "default_voice_notes_dir")]
    pub voice_notes_dir: String,
    /// Directory for call-recorder outputs (one .md per call)
    #[serde(default = "default_call_records_dir")]
    pub call_records_dir: String,
    /// Global hotkey to toggle voice recording (macOS: Cmd+Shift+<key>)
    #[serde(default = "default_voice_hotkey")]
    pub voice_hotkey: String,
    /// Synthesis prompt for the call recorder (different job, different ask)
    #[serde(default = "default_call_prompt")]
    pub call_prompt: String,
    /// Preferred input device for thought recordings (empty = system default)
    #[serde(default)]
    pub input_device_name: String,
    /// Preferred input device for call recordings — set this to a loopback
    /// device (e.g. BlackHole) to capture remote call audio; empty = default
    #[serde(default)]
    pub call_input_device_name: String,
}

fn default_voice_notes_dir() -> String {
    format!(
        "{}/.config/otamot/voice_notes",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}

fn default_call_records_dir() -> String {
    format!(
        "{}/.config/otamot/call_records",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}

fn default_voice_hotkey() -> String {
    "Cmd+Shift+R".to_string()
}

fn default_call_prompt() -> String {
    "You are a meeting-notes assistant. Transform the raw transcript of a \
call into polished Markdown meeting notes. Remove filler words and \
repetition. Organize under these headings: '## Summary' for a two-sentence \
overview, '## Decisions' for anything agreed, '## Action Items' as a \
checkbox list with owners when identifiable, and '## Open Questions' for \
unresolved topics. Be concise and factual. Reply with the Markdown snippet \
only, no preamble."
        .to_string()
}

/// Which recorder a synthesis belongs to — decides output dir and prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AiMode {
    #[default]
    Thoughts,
    Call,
}

impl AiMode {
    pub fn label(&self) -> &'static str {
        match self {
            AiMode::Thoughts => "thoughts",
            AiMode::Call => "call",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellTune {
    Default,
    LaCukaracha,
    IceCreamTruck,
}

impl Default for BellTune {
    fn default() -> Self {
        Self::Default
    }
}

/// The wire protocol an AI provider endpoint speaks.
/// New endpoint shapes are added as new variants here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointKind {
    /// OpenAI-compatible /chat/completions with Bearer auth
    /// (OpenAI, Ollama /v1, llama.cpp server, vLLM, ...)
    #[default]
    OpenAiCompatible,
    /// Anthropic /v1/messages with x-api-key auth
    Anthropic,
}

impl EndpointKind {
    pub fn label(&self) -> &'static str {
        match self {
            EndpointKind::OpenAiCompatible => "OpenAI-compatible",
            EndpointKind::Anthropic => "Anthropic",
        }
    }
}

/// Whisper model size for the download helper
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhisperSize {
    Tiny,
    #[default]
    Base,
    Small,
    Medium,
}

impl WhisperSize {
    pub fn file_name(&self) -> &'static str {
        match self {
            WhisperSize::Tiny => "ggml-tiny.bin",
            WhisperSize::Base => "ggml-base.bin",
            WhisperSize::Small => "ggml-small.bin",
            WhisperSize::Medium => "ggml-medium.bin",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            WhisperSize::Tiny => "Tiny (~75 MB)",
            WhisperSize::Base => "Base (~142 MB)",
            WhisperSize::Small => "Small (~466 MB)",
            WhisperSize::Medium => "Medium (~1.5 GB)",
        }
    }

    pub fn all() -> [WhisperSize; 4] {
        [
            WhisperSize::Tiny,
            WhisperSize::Base,
            WhisperSize::Small,
            WhisperSize::Medium,
        ]
    }
}

/// A single AI provider profile. Providers are data: adding a new
/// endpoint shape means a new EndpointKind variant + request/parse fns,
/// never a restructure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub name: String,
    #[serde(default)]
    pub kind: EndpointKind,
    pub endpoint: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            name: "Ollama local".to_string(),
            kind: EndpointKind::OpenAiCompatible,
            endpoint: "http://localhost:11434/v1".to_string(),
            api_key: String::new(),
            model: "llama3.2".to_string(),
        }
    }
}

fn default_ai_providers() -> Vec<AiProviderConfig> {
    vec![AiProviderConfig::default()]
}

fn default_active_ai_provider() -> String {
    "Ollama local".to_string()
}

fn default_whisper_model_path() -> String {
    format!(
        "{}/.config/otamot/models/ggml-base.bin",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}

fn default_synthesis_prompt() -> String {
    "You are a note-taking assistant. Transform the raw transcript of a work \
session into polished Markdown notes. Remove filler words and repetition. \
Organize the content under these headings: '## Accomplishments' for what was \
completed or progressed, '## Blockers' for obstacles encountered, and \
'## Notes' for anything else worth keeping. Be concise and factual. \
Reply with the Markdown snippet only, no preamble."
        .to_string()
}

fn default_max_recording_minutes() -> u32 {
    30
}

fn default_true() -> bool {
    true
}
fn default_survey_enabled() -> bool {
    true
}
fn default_work_duration() -> u32 {
    25
}
fn default_break_duration() -> u32 {
    5
}
fn default_notes_directory() -> String {
    format!(
        "{}/.config/otamot/notes",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}
fn default_todo_file() -> String {
    format!(
        "{}/.config/otamot/TODO.md",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}

fn default_slash_commands() -> HashMap<String, String> {
    let mut commands = HashMap::new();
    commands.insert("date".to_string(), "{{date}}".to_string());
    commands.insert("time".to_string(), "{{time}}".to_string());
    commands.insert("datetime".to_string(), "{{datetime}}".to_string());
    commands.insert("todo".to_string(), "- [ ] ".to_string());
    commands.insert("done".to_string(), "- [x] ".to_string());
    commands.insert("bullet".to_string(), "- ".to_string());
    commands.insert("hr".to_string(), "---\n".to_string());
    commands.insert("code".to_string(), "```\n\n```".to_string());
    commands
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_duration: default_work_duration(),
            break_duration: default_break_duration(),
            notes_directory: default_notes_directory(),
            call_notes_directory: default_notes_directory(),
            todo_file: default_todo_file(),
            notes_enabled: false,
            survey_enabled: default_survey_enabled(),
            language: Language::default(),
            slash_commands: default_slash_commands(),
            todo_enabled: true,
            kanban_enabled: false,
            sidebar_collapsed: false,
            active_listening_enabled: false,
            bell_tune: BellTune::Default,
            theme: Theme::robotic_lime(),
            ai_notes_enabled: false,
            ai_providers: default_ai_providers(),
            active_ai_provider: default_active_ai_provider(),
            whisper_model_path: default_whisper_model_path(),
            whisper_model_size: WhisperSize::default(),
            synthesis_prompt: default_synthesis_prompt(),
            max_recording_minutes: default_max_recording_minutes(),
            live_transcription_enabled: true,
            include_transcript_in_notes: true,
            voice_notes_dir: default_voice_notes_dir(),
            call_records_dir: default_call_records_dir(),
            voice_hotkey: default_voice_hotkey(),
            call_prompt: default_call_prompt(),
            input_device_name: String::new(),
            call_input_device_name: String::new(),
        }
    }
}

impl Config {
    /// Find the active AI provider profile by name, falling back to the
    /// first profile when the configured name is missing.
    pub fn active_ai_provider(&self) -> Option<&AiProviderConfig> {
        let active = &self.active_ai_provider;
        self.ai_providers
            .iter()
            .find(|p| &p.name == active)
            .or_else(|| self.ai_providers.first())
    }

    /// Which synthesis prompt and output directory a mode uses.
    pub fn ai_mode_parts(&self, mode: AiMode) -> (&str, &str) {
        match mode {
            AiMode::Thoughts => (
                self.synthesis_prompt.as_str(),
                self.voice_notes_dir.as_str(),
            ),
            AiMode::Call => (self.call_prompt.as_str(), self.call_records_dir.as_str()),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn save_to_path(&self, path: &PathBuf) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load_from_path(path: &PathBuf) -> Self {
        if path.exists() {
            fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/otamot/settings.json")
    }

    pub fn notes_path(&self) -> PathBuf {
        PathBuf::from(&self.notes_directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.work_duration, 25);
        assert_eq!(config.break_duration, 5);
        assert!(!config.notes_enabled);
    }

    #[test]
    fn test_ai_notes_disabled_by_default() {
        let config = Config::default();
        assert!(!config.ai_notes_enabled);
        assert_eq!(config.ai_providers.len(), 1);
        assert_eq!(config.active_ai_provider, "Ollama local");
        assert_eq!(config.max_recording_minutes, 30);
        assert!(!config.synthesis_prompt.is_empty());
        assert_eq!(config.whisper_model_size, WhisperSize::Base);
        assert!(config.whisper_model_path.ends_with("ggml-base.bin"));
    }

    #[test]
    fn test_config_without_ai_fields_loads_with_defaults() {
        // Backward compat: an old settings.json without ai_* keys
        let old_json = r#"{
            "work_duration": 30,
            "break_duration": 7,
            "notes_directory": "/tmp/notes",
            "call_notes_directory": "/tmp/notes",
            "todo_file": "/tmp/TODO.md"
        }"#;
        let config: Config = serde_json::from_str(old_json).unwrap();
        assert_eq!(config.work_duration, 30);
        assert!(!config.ai_notes_enabled);
        assert_eq!(config.ai_providers.len(), 1);
        assert_eq!(config.ai_providers[0].endpoint, "http://localhost:11434/v1");
        assert_eq!(config.active_ai_provider(), Some(&config.ai_providers[0]));
    }

    #[test]
    fn test_ai_providers_serde_roundtrip() {
        let mut config = Config::default();
        config.ai_notes_enabled = true;
        config.ai_providers.push(AiProviderConfig {
            name: "Claude".to_string(),
            kind: EndpointKind::Anthropic,
            endpoint: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "claude-sonnet-4".to_string(),
        });
        config.active_ai_provider = "Claude".to_string();
        config.whisper_model_size = WhisperSize::Small;

        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
        assert_eq!(parsed.active_ai_provider(), Some(&config.ai_providers[1]));
    }

    #[test]
    fn test_active_ai_provider_fallback() {
        let mut config = Config::default();
        config.active_ai_provider = "nonexistent".to_string();
        assert_eq!(config.active_ai_provider(), Some(&config.ai_providers[0]));
        config.ai_providers.clear();
        assert!(config.active_ai_provider().is_none());
    }

    #[test]
    fn test_endpoint_kind_labels() {
        assert_eq!(EndpointKind::default().label(), "OpenAI-compatible");
        assert_eq!(EndpointKind::Anthropic.label(), "Anthropic");
    }

    #[test]
    fn test_whisper_sizes() {
        assert_eq!(WhisperSize::Tiny.file_name(), "ggml-tiny.bin");
        assert_eq!(WhisperSize::Medium.file_name(), "ggml-medium.bin");
        assert_eq!(WhisperSize::all().len(), 4);
    }

    #[test]
    fn test_voice_notes_output_defaults() {
        let config = Config::default();
        assert!(config.voice_notes_dir.ends_with("voice_notes"));
        assert!(config.call_records_dir.ends_with("call_records"));
        assert_eq!(config.voice_hotkey, "Cmd+Shift+R");
        assert!(config.live_transcription_enabled);
        assert!(config.include_transcript_in_notes);
        assert!(!config.synthesis_prompt.is_empty());
        assert!(!config.call_prompt.is_empty());
        assert_ne!(config.synthesis_prompt, config.call_prompt);
    }

    #[test]
    fn test_voice_fields_backward_compat() {
        let old_json = r#"{"work_duration": 25}"#;
        let config: Config = serde_json::from_str(old_json).unwrap();
        assert_eq!(config.voice_hotkey, "Cmd+Shift+R");
        assert!(!config.voice_notes_dir.is_empty());
        assert!(!config.call_records_dir.is_empty());
    }

    #[test]
    fn test_old_output_file_field_ignored() {
        // Legacy settings.json with voice_notes_output_file still loads;
        // the new dir fields take their defaults.
        let old_json = r#"{"voice_notes_output_file": "/tmp/x.md"}"#;
        let config: Config = serde_json::from_str(old_json).unwrap();
        assert!(config.voice_notes_dir.ends_with("voice_notes"));
    }

    #[test]
    fn test_ai_mode_default_and_labels() {
        assert_eq!(AiMode::default(), AiMode::Thoughts);
        assert_eq!(AiMode::Thoughts.label(), "thoughts");
        assert_eq!(AiMode::Call.label(), "call");
    }

    #[test]
    fn test_ai_mode_parts() {
        let config = Config::default();
        let (thoughts_prompt, thoughts_dir) = config.ai_mode_parts(AiMode::Thoughts);
        let (call_prompt, call_dir) = config.ai_mode_parts(AiMode::Call);
        assert_eq!(thoughts_prompt, config.synthesis_prompt);
        assert_eq!(call_prompt, config.call_prompt);
        assert_ne!(thoughts_prompt, call_prompt);
        assert_eq!(thoughts_dir, config.voice_notes_dir);
        assert_eq!(call_dir, config.call_records_dir);
    }

    #[test]
    fn test_device_selection_defaults_empty() {
        let config = Config::default();
        // Empty means "system default device"
        assert!(config.input_device_name.is_empty());
        assert!(config.call_input_device_name.is_empty());
    }
}
