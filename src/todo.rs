//! TODO list management for the Pomodoro app
//!
//! Provides a simple TODO list that persists to ~/.config/otamot/todo.md

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
    pub items: Vec<TodoItem>,
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
            next_id: 1,
            enabled: false,
            last_updated: None,
        }
    }

    /// Load TODO list from config directory
    pub fn load() -> Self {
        let path = Self::todo_path();
        
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    // Try to parse as JSON first (new format)
                    if let Ok(list) = serde_json::from_str::<TodoList>(&content) {
                        return list;
                    }
                    
                    // Fall back to parsing markdown TODO format
                    return Self::parse_markdown(&content);
                }
                Err(e) => eprintln!("Failed to read todo.md: {}", e),
            }
        }
        
        Self::default()
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

        // Save as both JSON (for internal use) and markdown (for user readability)
        let json_path = path.with_extension("json");
        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&json_path, content) {
                    eprintln!("Failed to save todo.json: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to serialize todo list: {}", e),
        }

        // Also save as markdown for user accessibility
        let md_content = self.to_markdown();
        if let Err(e) = std::fs::write(&path, md_content) {
            eprintln!("Failed to save todo.md: {}", e);
        }
    }

    /// Get the path to the TODO file
    fn todo_path() -> PathBuf {
        std::env::var("HOME")
            .map(|home| PathBuf::from(format!("{}/.config/otamot/todo.md", home)))
            .unwrap_or_else(|_| PathBuf::from("todo.md"))
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
        self.items.push(item);
        self.next_id += 1;
        self.last_updated = Some(Local::now());
    }

    /// Toggle a TODO item's completion status
    pub fn toggle(&mut self, id: usize) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
            item.completed_at = if item.completed {
                Some(Local::now())
            } else {
                None
            };
            self.last_updated = Some(Local::now());
        }
    }

    /// Remove a TODO item
    pub fn remove(&mut self, id: usize) {
        self.items.retain(|i| i.id != id);
        self.last_updated = Some(Local::now());
    }

    /// Clear all completed items
    pub fn clear_completed(&mut self) {
        self.items.retain(|i| !i.completed);
        self.last_updated = Some(Local::now());
    }

    /// Get count of incomplete items
    pub fn incomplete_count(&self) -> usize {
        self.items.iter().filter(|i| !i.completed).count()
    }

    /// Get count of completed items
    pub fn completed_count(&self) -> usize {
        self.items.iter().filter(|i| i.completed).count()
    }

    /// Convert to markdown format
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();
        
        // Frontmatter
        output.push_str("---\n");
        output.push_str(&format!("last_updated: {}\n", 
            self.last_updated.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()));
        output.push_str(&format!("total_items: {}\n", self.items.len()));
        output.push_str(&format!("completed: {}\n", self.completed_count()));
        output.push_str("---\n\n");

        if self.items.is_empty() {
            output.push_str("No TODO items yet!\n");
            output.push_str("\nAdd items using the TODO panel in the app.\n");
        } else {
            // Incomplete items first
            let incomplete: Vec<_> = self.items.iter().filter(|i| !i.completed).collect();
            let completed: Vec<_> = self.items.iter().filter(|i| i.completed).collect();

            if !incomplete.is_empty() {
                output.push_str(&format!("## TODO ({} items)\n\n", incomplete.len()));
                for item in incomplete {
                    output.push_str(&format!("- [ ] {}\n", item.text));
                }
            }

            if !completed.is_empty() {
                output.push_str(&format!("\n## Completed ({} items)\n\n", completed.len()));
                for item in completed {
                    output.push_str(&format!("- [x] {}\n", item.text));
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
            // Handle frontmatter
            if line.trim() == "---" {
                in_frontmatter = !in_frontmatter;
                continue;
            }

            if in_frontmatter {
                continue; // Skip frontmatter content
            }

            // Parse TODO items
            let trimmed = line.trim();
            if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") {
                let completed = trimmed.starts_with("- [x] ");
                let text = trimmed[6..].to_string();
                
                let item = TodoItem {
                    id: list.next_id,
                    text,
                    completed,
                    created_at: Local::now(),
                    completed_at: if completed { Some(Local::now()) } else { None },
                };
                list.items.push(item);
                list.next_id += 1;
            }
        }

        if !list.items.is_empty() {
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
        
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].text, "Test item");
        assert!(!list.items[0].completed);
    }

    #[test]
    fn test_toggle_todo() {
        let mut list = TodoList::new();
        list.add("Test item".to_string());
        
        assert!(!list.items[0].completed);
        
        list.toggle(1);
        assert!(list.items[0].completed);
        
        list.toggle(1);
        assert!(!list.items[0].completed);
    }

    #[test]
    fn test_remove_todo() {
        let mut list = TodoList::new();
        list.add("Item 1".to_string());
        list.add("Item 2".to_string());
        
        list.remove(1);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].text, "Item 2");
    }

    #[test]
    fn test_counts() {
        let mut list = TodoList::new();
        list.add("Item 1".to_string());
        list.add("Item 2".to_string());
        list.toggle(1);
        
        assert_eq!(list.incomplete_count(), 1);
        assert_eq!(list.completed_count(), 1);
    }

    #[test]
    fn test_clear_completed() {
        let mut list = TodoList::new();
        list.add("Item 1".to_string());
        list.add("Item 2".to_string());
        list.toggle(1);
        
        list.clear_completed();
        assert_eq!(list.items.len(), 1);
        assert!(!list.items[0].completed);
    }

    #[test]
    fn test_to_markdown() {
        let mut list = TodoList::new();
        list.add("Task 1".to_string());
        list.add("Task 2".to_string());
        list.toggle(2);

        let md = list.to_markdown();
        assert!(md.contains("- [ ] Task 1"));
        assert!(md.contains("- [x] Task 2"));
    }

    #[test]
    fn test_parse_markdown() {
        let content = r#"---
last_updated: 2026-03-02
---

- [ ] Task 1
- [x] Task 2
"#;
        let list = TodoList::parse_markdown(content);
        
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].text, "Task 1");
        assert!(!list.items[0].completed);
        assert!(list.items[1].completed);
    }
}
