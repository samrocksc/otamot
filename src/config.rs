use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerMode {
    Work,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesView {
    Edit,
    Preview,
}

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
}

fn default_work_duration() -> u32 { 25 }
fn default_break_duration() -> u32 { 5 }
fn default_notes_directory() -> String { 
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.config/otamot/notes", home)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_duration: default_work_duration(),
            break_duration: default_break_duration(),
            notes_directory: default_notes_directory(),
            notes_enabled: false,
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

    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/otamot/settings.json")
    }
    
    #[allow(dead_code)]
    pub fn notes_path(&self) -> PathBuf {
        PathBuf::from(&self.notes_directory)
    }
}
