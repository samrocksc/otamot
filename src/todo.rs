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

    /// Historical archive of all completed items ever cleared
    #[serde(default)]
    pub history: Vec<TodoItem>,

    pub next_id: usize,

    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub last_updated: Option<DateTime<Local>>,
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
        }
    }

    /// Load TODO list from config directory
    pub fn load() -> Self {
        let path = Self::todo_path();

        let mut list = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    // Try to parse as JSON first (new format usually in .json)
                    let json_path = path.with_extension("json");
                    let mut decoded = if json_path.exists() {
                        std::fs::read_to_string(&json_path)
                            .ok()
                            .and_then(|c| serde_json::from_str::<TodoList>(&c).ok())
                            .unwrap_or_default()
                    } else {
                        // Fall back to parsing markdown TODO format from .md
                        Self::parse_markdown(&content)
                    };

                    // Migration: if we have flat items, move them to active/completed
                    if !decoded.items.is_empty() {
                        for item in decoded.items.drain(..) {
                            if item.completed {
                                decoded.completed.push(item);
                            } else {
                                decoded.active.push(item);
                            }
                        }
                    }
                    decoded
                }
                Err(e) => {
                    eprintln!("Failed to read todo.md: {}", e);
                    Self::default()
                }
            }
        } else {
            Self::default()
        };

        // Ensure next_id is correct after loading
        let max_active = list.active.iter().map(|i| i.id).max().unwrap_or(0);
        let max_comp = list.completed.iter().map(|i| i.id).max().unwrap_or(0);
        list.next_id = max_active.max(max_comp) + 1;

        list
    }

    /// Save TODO list to config directory
    pub fn save(&self) {
        let path = Self::todo_path();

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }
        }

        // Save as JSON (primary format with separate sections)
        let json_path = path.with_extension("json");
        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&json_path, content) {
                    eprintln!("Failed to save todo.json: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to serialize todo list: {}", e),
        }

        // Also save as markdown for user accessibility (flat format with headers)
        let md_content = self.to_markdown();
        if let Err(e) = std::fs::write(&path, md_content) {
            eprintln!("Failed to save todo.md: {}", e);
        }
    }

    /// Get the path to the TODO file
    fn todo_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/otamot/todo.md")
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

        // Frontmatter
        output.push_str("---\n");
        output.push_str(&format!(
            "last_updated: {}\n",
            self.last_updated
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        ));
        output.push_str(&format!(
            "total_items: {}\n",
            self.active.len() + self.completed.len()
        ));
        output.push_str(&format!("completed: {}\n", self.completed.len()));
        output.push_str("---\n\n");

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

            if !self.history.is_empty() {
                output.push_str(&format!("\n## History ({} items)\n\n", self.history.len()));
                for item in &self.history {
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
    fn parse_markdown(content: &str) -> Self {
        let mut list = Self::default();
        let mut in_frontmatter = false;

        for line in content.lines() {
            if line.trim() == "---" {
                in_frontmatter = !in_frontmatter;
                continue;
            }
            if in_frontmatter {
                continue;
            }

            let trimmed = line.trim();
            if trimmed.starts_with("- [ ] ") {
                let text = trimmed[6..].to_string();
                list.active.push(TodoItem {
                    id: list.next_id,
                    text,
                    completed: false,
                    created_at: Local::now(),
                    completed_at: None,
                });
                list.next_id += 1;
            } else if trimmed.starts_with("- [x] ") {
                // Try to extract timestamp if present like "- [x] task [2026-03-03 ...]"
                let text_part = trimmed[6..].to_string();
                let (text, completed_at) = if let Some(bracket_pos) = text_part.rfind(" [") {
                    (text_part[..bracket_pos].to_string(), Some(Local::now()))
                } else {
                    (text_part, Some(Local::now()))
                };

                list.completed.push(TodoItem {
                    id: list.next_id,
                    text: text.trim().to_string(),
                    completed: true,
                    created_at: Local::now(),
                    completed_at,
                });
                list.next_id += 1;
            }
        }

        if !list.active.is_empty() || !list.completed.is_empty() {
            list.last_updated = Some(Local::now());
        }

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
