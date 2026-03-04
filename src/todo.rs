//! TODO list management for the Pomodoro app
//!
//! Provides a simple TODO list that persists to ~/.config/otamot/todo.md
//! and ~/.config/otamot/todo.json.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single TODO item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoItem {
    pub id: usize,
    pub text: String,
    pub completed: bool,
    pub created_at: DateTime<Local>,
    pub completed_at: Option<DateTime<Local>>,
}

/// TODO list container
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoList {
    /// Legacy flat items list for backward compatibility
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<TodoItem>,

    /// Active (incomplete) items
    #[serde(default)]
    pub active: Vec<TodoItem>,

    /// Completed items
    #[serde(default)]
    pub completed: Vec<TodoItem>,

    pub history: Vec<TodoItem>,
    pub next_id: usize,
    pub enabled: bool,
    pub last_updated: Option<DateTime<Local>>,
    pub todo_file: String,
}

impl TodoList {
    /// Create a new empty TODO list
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            active: Vec::new(),
            completed: Vec::new(),
            history: Vec::new(),
            next_id: 1,
            enabled: false,
            last_updated: None,
            todo_file: String::new(),
        }
    }

    /// Load TODO list from config directory
    pub fn load_from_path(path_str: &str) -> Self {
        let path = PathBuf::from(path_str);

        let mut list = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    // Try to parse as JSON if it ends in .json
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                         serde_json::from_str::<TodoList>(&content).unwrap_or_default()
                    } else {
                        // Fall back to parsing markdown TODO format
                        Self::from_markdown(&content)
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read todo file: {}", e);
                    Self::default()
                }
            }
        } else {
            Self::default()
        };
        list.todo_file = path_str.to_string();
        list
    }

    /// Save TODO list to a specific path
    pub fn save_to_path(&self, path_str: &str) {
        let path = PathBuf::from(path_str);

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }
        }

        let mut full_content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let todo_md = self.to_markdown();
        if let Some(start) = full_content.find("# TODO") {
            // Find start of next section
            let end = full_content[start + 1..]
                .find("\n# ")
                .map(|i| i + start + 1)
                .unwrap_or(full_content.len());
            full_content.replace_range(start..end, &todo_md);
        } else {
            if !full_content.is_empty() && !full_content.ends_with('\n') {
                full_content.push('\n');
            }
            full_content.push_str(&todo_md);
        }

        if let Err(e) = std::fs::write(&path, full_content) {
            eprintln!("Failed to save todo markdown: {}", e);
        }
    }

    pub fn save(&self) {
        if self.todo_file.is_empty() {
             let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
             let default_path = format!("{}/.config/otamot/TODO.md", home);
             return self.save_to_path(&default_path);
        }
        self.save_to_path(&self.todo_file);
    }

    /// Add a new TODO item
    pub fn add(&mut self, text: String) {
        let item = TodoItem {
            id: self.next_id,
            text,
            completed: false,
            created_at: Local::now(),
            completed_at: None,
        };
        self.active.push(item);
        self.next_id += 1;
        self.last_updated = Some(Local::now());
    }

    /// Toggle a TODO item's completion status
    pub fn toggle(&mut self, id: usize) {
        // Check active first
        if let Some(pos) = self.active.iter().position(|i| i.id == id) {
            let mut item = self.active.remove(pos);
            item.completed = true;
            item.completed_at = Some(Local::now());
            self.completed.push(item);
            self.last_updated = Some(Local::now());
            return;
        }

        // Then check completed
        if let Some(pos) = self.completed.iter().position(|i| i.id == id) {
            let mut item = self.completed.remove(pos);
            item.completed = false;
            item.completed_at = None;
            self.active.push(item);
            self.last_updated = Some(Local::now());
        }
    }

    /// Remove a TODO item
    pub fn remove(&mut self, id: usize) {
        self.active.retain(|i| i.id != id);
        self.completed.retain(|i| i.id != id);
        self.last_updated = Some(Local::now());
    }

    /// Clear all completed items and move them to history
    pub fn clear_completed(&mut self) {
        self.history.extend(self.completed.drain(..));
        self.last_updated = Some(Local::now());
    }

    /// Get count of incomplete items
    pub fn incomplete_count(&self) -> usize {
        self.active.len()
    }

    /// Get count of completed items
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Convert to markdown format
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        // Section header
        output.push_str("# TODO\n\n");

        if self.active.is_empty() && self.completed.is_empty() {
            output.push_str("No TODO items yet!\n");
        } else {
            if !self.active.is_empty() {
                output.push_str(&format!("## Active ({} items)\n\n", self.active.len()));
                for item in &self.active {
                    output.push_str(&format!("- [ ] {}\n", item.text));
                }
            }

            if !self.completed.is_empty() {
                output.push_str(&format!(
                    "\n## Completed ({} items)\n\n",
                    self.completed.len()
                ));
                for item in &self.completed {
                    let ts = item
                        .completed_at
                        .map(|d| d.format(" [%Y-%m-%d %H:%M]").to_string())
                        .unwrap_or_default();
                    output.push_str(&format!("- [x] {}{}\n", item.text, ts));
                }
            }
        }

        output
    }

    /// Parse from markdown format
    pub fn from_markdown(content: &str) -> Self {
        let mut list = Self::default();
        let mut in_todo_section = false;
        let mut next_id = 1;

        for line in content.lines() {
            let trimmed = line.trim();
            
            // Look for the start of the TODO section
            if trimmed == "# TODO" {
                in_todo_section = true;
                continue;
            }
            
            // If we hit another top-level header, we have exited the TODO section
            if in_todo_section && trimmed.starts_with("# ") && trimmed != "# TODO" {
                break;
            }

            if in_todo_section {
                if trimmed.starts_with("- [ ] ") {
                    let text = trimmed[6..].to_string();
                    list.active.push(TodoItem {
                        id: next_id,
                        text,
                        completed: false,
                        created_at: Local::now(),
                        completed_at: None,
                    });
                    next_id += 1;
                } else if let Some(stripped) = trimmed.strip_prefix("- [x] ") {
                    let text_part = stripped.to_string();
                    let (text, completed_at) = if let Some(bracket_pos) = text_part.rfind(" [") {
                        (text_part[..bracket_pos].to_string(), Some(Local::now()))
                    } else {
                        (text_part, Some(Local::now()))
                    };

                    list.completed.push(TodoItem {
                        id: next_id,
                        text: text.trim().to_string(),
                        completed: true,
                        created_at: Local::now(),
                        completed_at,
                    });
                    next_id += 1;
                }
            }
        }
        list.next_id = next_id;
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_todo() {
        let mut list = TodoList::new();
        list.add("Test item".to_string());
        assert_eq!(list.active.len(), 1);
        assert_eq!(list.active[0].text, "Test item");
    }

    #[test]
    fn test_toggle_todo() {
        let mut list = TodoList::new();
        list.add("Test item".to_string());
        list.toggle(1);
        assert!(list.active.is_empty());
        assert_eq!(list.completed.len(), 1);
        assert!(list.completed[0].completed);

        list.toggle(1);
        assert!(list.completed.is_empty());
        assert_eq!(list.active.len(), 1);
    }

    #[test]
    fn test_to_markdown() {
        let mut list = TodoList::new();
        list.add("Task 1".to_string());
        list.add("Task 2".to_string());
        list.toggle(list.active[1].id);

        let md = list.to_markdown();
        assert!(md.contains("- [ ] Task 1"));
        assert!(md.contains("- [x] Task 2"));
    }
}
