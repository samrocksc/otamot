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
            button: CustomColor::new(0x38, 0x97, 0xd8),    // Blue button
            bg: CustomColor::new(0xFF, 0xFF, 0xFF),       // Pure white background
            tab_active: CustomColor::new(0xAE, 0x81, 0xFF),
            tab_inactive: CustomColor::new(0xE6, 0xE6, 0xE6),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_work_duration")]
    pub work_duration: u32,
    #[serde(default = "default_break_duration")]
    pub break_duration: u32,
    #[serde(default = "default_notes_directory")]
    pub notes_directory: String,
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
    pub bell_tune: BellTune,
    #[serde(default)]
    pub theme: Theme,
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

fn default_true() -> bool { true }
fn default_survey_enabled() -> bool { true }
fn default_work_duration() -> u32 { 25 }
fn default_break_duration() -> u32 { 5 }
fn default_notes_directory() -> String {
    format!("{}/.config/otamot/notes", std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
}
fn default_todo_file() -> String {
    format!("{}/.config/otamot/TODO.md", std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
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
            todo_file: default_todo_file(),
            notes_enabled: false,
            survey_enabled: default_survey_enabled(),
            language: Language::default(),
            slash_commands: default_slash_commands(),
            todo_enabled: true,
            kanban_enabled: false,
            sidebar_collapsed: false,
            bell_tune: BellTune::Default,
            theme: Theme::robotic_lime(),
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
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.work_duration, 25);
        assert_eq!(config.break_duration, 5);
        assert!(!config.notes_enabled);
    }
}
