//! Slash command support for the notes editor
//!
//! Provides a system for custom slash commands that insert content into notes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default slash commands included with the app
pub fn default_commands() -> HashMap<String, String> {
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

/// Slash command manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandManager {
    /// Custom commands defined by the user
    commands: HashMap<String, String>,
}

impl Default for CommandManager {
    fn default() -> Self {
        Self {
            commands: default_commands(),
        }
    }
}

impl CommandManager {
    /// Create a new command manager with default commands
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all available command names
    pub fn list_commands(&self) -> Vec<String> {
        let mut cmds: Vec<String> = self.commands.keys().cloned().collect();
        cmds.sort();
        cmds
    }

    /// Search commands by prefix
    pub fn search_commands(&self, query: &str) -> Vec<String> {
        let query = query.to_lowercase();
        let mut results: Vec<String> = self
            .commands
            .keys()
            .filter(|cmd| cmd.to_lowercase().starts_with(&query))
            .cloned()
            .collect();
        results.sort();
        results
    }

    /// Execute a command, returning the text to insert
    pub fn execute(&self, name: &str) -> Option<String> {
        self.commands.get(name).map(|template| {
            // Process template placeholders
            let now = chrono::Local::now();
            let result = template
                .replace("{{date}}", &now.format("%Y-%m-%d").to_string())
                .replace("{{time}}", &now.format("%H:%M").to_string())
                .replace("{{datetime}}", &now.format("%Y-%m-%d %H:%M").to_string());
            result
        })
    }

    /// Add a custom command
    pub fn add_command(&mut self, name: String, template: String) {
        self.commands.insert(name, template);
    }

    /// Remove a command
    pub fn remove_command(&mut self, name: &str) -> bool {
        self.commands.remove(name).is_some()
    }

    /// Load commands from config directory
    pub fn load() -> Self {
        let config_dir = std::env::var("HOME")
            .map(|home| format!("{}/.config/otamot", home))
            .unwrap_or_else(|_| ".otamot".to_string());

        let commands_file = std::path::PathBuf::from(&config_dir).join("commands.json");

        if commands_file.exists() {
            match std::fs::read_to_string(&commands_file) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(manager) => return manager,
                    Err(e) => eprintln!("Failed to parse commands.json: {}", e),
                },
                Err(e) => eprintln!("Failed to read commands.json: {}", e),
            }
        }

        Self::default()
    }

    /// Save commands to config directory
    pub fn save(&self) {
        let config_dir = std::env::var("HOME")
            .map(|home| format!("{}/.config/otamot", home))
            .unwrap_or_else(|_| ".otamot".to_string());

        let config_path = std::path::PathBuf::from(&config_dir);

        if let Err(e) = std::fs::create_dir_all(&config_path) {
            eprintln!("Failed to create config directory: {}", e);
            return;
        }

        let commands_file = config_path.join("commands.json");

        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&commands_file, content) {
                    eprintln!("Failed to save commands.json: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to serialize commands: {}", e),
        }
    }

    /// Process text to find slash command at cursor position
    /// Returns (command_start_index, command_text) if a command is being typed
    pub fn find_command_at_cursor(text: &str, cursor_pos: usize) -> Option<(usize, String)> {
        // Find the start of the current line
        let line_start = text[..cursor_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_text = &text[line_start..cursor_pos];

        // Find the last '/' in the line before cursor
        if let Some(slash_pos) = line_text.rfind('/') {
            // Check if there's a space between slash and cursor (command ended)
            let after_slash = &line_text[slash_pos + 1..];
            if after_slash.contains(' ') {
                return None;
            }

            // Return the command text (without the slash)
            let command_text = after_slash.to_string();
            let absolute_pos = line_start + slash_pos;

            // Only show dropdown if command is not too long
            if command_text.len() <= 20 {
                return Some((absolute_pos, command_text));
            }
        }

        None
    }

    /// Insert command result into text, replacing the command syntax
    pub fn insert_command(
        text: &str,
        cursor_pos: usize,
        command_start: usize,
        replacement: &str,
    ) -> String {
        let before = &text[..command_start];
        let after = &text[cursor_pos..];
        format!("{}{}{}", before, replacement, after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_commands_exist() {
        let manager = CommandManager::new();
        assert!(manager.execute("date").is_some());
        assert!(manager.execute("time").is_some());
        assert!(manager.execute("todo").is_some());
    }

    #[test]
    fn test_search_commands() {
        let manager = CommandManager::new();
        let results = manager.search_commands("t");
        assert!(results.contains(&"time".to_string()));
        assert!(results.contains(&"todo".to_string()));
    }

    #[test]
    fn test_find_command_at_cursor() {
        let text = "Hello /dat world";
        // Cursor at position 10 is after "/dat" (positions: "Hello " = 6 chars, '/' at 6, 'dat' = 7-9)
        let result = CommandManager::find_command_at_cursor(text, 10);
        assert!(result.is_some());
        let (pos, cmd) = result.unwrap();
        assert_eq!(pos, 6); // Position of '/'
        assert_eq!(cmd, "dat");
    }

    #[test]
    fn test_no_command_after_space() {
        let text = "Hello /dat world";
        // Cursor at position 11 is after the space following "/dat"
        let result = CommandManager::find_command_at_cursor(text, 11);
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_command() {
        let text = "Hello /dat world";
        // Cursor at 10 (after "/dat"), command_start at 6 (position of '/')
        let result = CommandManager::insert_command(text, 10, 6, "2026-03-02");
        assert_eq!(result, "Hello 2026-03-02 world");
    }

    #[test]
    fn test_template_replacement() {
        let manager = CommandManager::new();
        let result = manager.execute("date").unwrap();
        // Should be in YYYY-MM-DD format
        assert!(result.len() == 10);
        assert!(result.contains('-'));
    }

    #[test]
    fn test_add_remove_command() {
        let mut manager = CommandManager::new();
        manager.add_command("custom".to_string(), "CUSTOM TEXT".to_string());
        assert!(manager.execute("custom").is_some());
        assert!(manager.remove_command("custom"));
        assert!(manager.execute("custom").is_none());
    }
}
