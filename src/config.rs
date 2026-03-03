use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

// TimerMode is now defined in timer module
// NotesView is only used in the UI (app.rs)

/// Represents the current view for notes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesView {
    Edit,
    Preview,
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

/// Configuration for the Pomodoro app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_work_duration")]
    pub work_duration: u32,

    #[serde(default = "default_break_duration")]
    pub break_duration: u32,

    #[serde(default = "default_notes_directory")]
    pub notes_directory: String,

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
    pub vim_enabled: bool,
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
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.config/otamot/notes", home)
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
            notes_enabled: false,
            survey_enabled: default_survey_enabled(),
            language: Language::default(),
            slash_commands: default_slash_commands(),
            todo_enabled: true,
            vim_enabled: false,
        }
    }
}

impl Config {
    /// Load configuration from file, or return defaults if file doesn't exist
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

    /// Save configuration to file
    pub fn save(&self) -> io::Result<()> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Save configuration to a specific path (for testing)
    pub fn save_to_path(&self, path: &PathBuf) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from a specific path (for testing)
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

    /// Get the default config file path
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/otamot/settings.json")
    }

    /// Get the notes directory path
    #[allow(dead_code)]
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

    #[test]
    fn test_config_default_notes_directory() {
        let config = Config::default();
        // Should contain .config/otamot/notes
        assert!(config.notes_directory.contains(".config/otamot/notes"));
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            work_duration: 30,
            break_duration: 10,
            notes_directory: "/custom/path".to_string(),
            notes_enabled: true,
            survey_enabled: true,
            language: Language::German,
            slash_commands: HashMap::new(),
            todo_enabled: true,
            vim_enabled: false,
        };

        let json = serde_json::to_string(&config).unwrap();
        // Parse it back to verify the values
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.work_duration, 30);
        assert_eq!(parsed.break_duration, 10);
        assert_eq!(parsed.notes_directory, "/custom/path");
        assert!(parsed.notes_enabled);
        assert!(parsed.survey_enabled);
        assert_eq!(parsed.language, Language::German);
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "work_duration": 45,
            "break_duration": 15,
            "notes_directory": "/test/notes",
            "notes_enabled": true,
            "survey_enabled": false
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.work_duration, 45);
        assert_eq!(config.break_duration, 15);
        assert_eq!(config.notes_directory, "/test/notes");
        assert!(config.notes_enabled);
        assert!(!config.survey_enabled);
    }

    #[test]
    fn test_custom_slash_command_persistence() {
        let json = r#"{
            "slash_commands": {
                "ted": "Ted Williams"
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.slash_commands.get("ted"), Some(&"Ted Williams".to_string()));
        // Ensure defaults are also there if using the default() constructor then merging, 
        // but here we just test deserialization of the specific field.
    }


    #[test]
    fn test_config_deserialization_partial() {
        // Test that missing fields get defaults
        let json = r#"{
            "work_duration": 20
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.work_duration, 20);
        assert_eq!(config.break_duration, 5); // default
        assert!(!config.notes_enabled); // default
        assert!(config.survey_enabled); // default
    }

    #[test]
    fn test_config_deserialization_empty() {
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.work_duration, 25); // default
        assert_eq!(config.break_duration, 5); // default
        assert!(config.survey_enabled); // default
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");

        let config = Config {
            work_duration: 40,
            break_duration: 10,
            notes_directory: "/custom/notes".to_string(),
            notes_enabled: true,
            survey_enabled: true,
            language: Language::English,
            slash_commands: HashMap::new(),
            todo_enabled: true,
            vim_enabled: false,
        };

        config.save_to_path(&config_path).unwrap();
        assert!(config_path.exists());

        let loaded = Config::load_from_path(&config_path);
        assert_eq!(loaded.work_duration, 40);
        assert_eq!(loaded.break_duration, 10);
        assert_eq!(loaded.notes_directory, "/custom/notes");
        assert!(loaded.notes_enabled);
        assert!(loaded.survey_enabled);
    }

    #[test]
    fn test_config_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.json");

        let loaded = Config::load_from_path(&config_path);
        assert_eq!(loaded.work_duration, 25); // default
        assert_eq!(loaded.break_duration, 5); // default
    }

    #[test]
    fn test_config_load_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.json");

        fs::write(&config_path, "not valid json {{{").unwrap();

        let loaded = Config::load_from_path(&config_path);
        // Should return default on parse error
        assert_eq!(loaded.work_duration, 25);
        assert_eq!(loaded.break_duration, 5);
    }

    #[test]
    fn test_config_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nested/dir/settings.json");

        let config = Config::default();
        config.save_to_path(&config_path).unwrap();

        assert!(config_path.exists());
        assert!(config_path.parent().unwrap().exists());
    }

    #[test]
    fn test_notes_view_equality() {
        assert_eq!(NotesView::Edit, NotesView::Edit);
        assert_eq!(NotesView::Preview, NotesView::Preview);
        assert_ne!(NotesView::Edit, NotesView::Preview);
    }
}
