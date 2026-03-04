use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KanbanStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KanbanItem {
    pub id: usize,
    pub text: String,
    pub status: KanbanStatus,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KanbanBoard {
    pub items: Vec<KanbanItem>,
    pub history: Vec<KanbanItem>,
    pub next_id: usize,
    pub todo_sync: bool,

    #[serde(default)]
    pub kanban_file: String,
}

impl KanbanBoard {
    pub fn load_from_path(path_str: &str) -> Self {
        let path = PathBuf::from(path_str);
        let mut board = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        serde_json::from_str::<KanbanBoard>(&content).unwrap_or_default()
                    } else {
                        Self::from_markdown(&content)
                    }
                }
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        };
        board.kanban_file = path_str.to_string();
        board
    }

    pub fn save_to_path(&self, path_str: &str) -> std::io::Result<()> {
        let path = std::path::PathBuf::from(path_str);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let kanban_md = self.to_markdown();
        
        // If the file is the global TODO.md, we need to preserve the # TODO section if it exists
        // actually let's just make save_to_path much simpler and have it just write the md
        // and we will handle the merging and syncing in a dedicated "ProjectManager" or similar.
        // For now, let's just make it write the markdown.
        std::fs::write(path, kanban_md)
    }

    pub fn save(&self) -> std::io::Result<()> {
        if self.kanban_file.is_empty() {
             let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
             let path = format!("{}/.config/otamot/TODO.md", home);
             return self.save_to_path(&path);
        }
        self.save_to_path(&self.kanban_file)
    }


    pub fn add_item(&mut self, text: String) {
        let item = KanbanItem {
            id: self.next_id,
            text,
            status: KanbanStatus::Todo,
            created_at: Local::now(),
            updated_at: Local::now(),
        };
        self.items.push(item);
        self.next_id += 1;
    }

    pub fn move_item(&mut self, item_id: usize, new_status: KanbanStatus) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
            item.status = new_status;
            item.updated_at = Local::now();
        }
    }

    pub fn delete_item(&mut self, item_id: usize) {
        if let Some(pos) = self.items.iter().position(|i| i.id == item_id) {
            let mut item = self.items.remove(pos);
            item.updated_at = Local::now();
            self.history.push(item);
        }
    }

    pub fn clear_done(&mut self) {
        let done_items: Vec<_> = self.items.iter()
            .filter(|i| i.status == KanbanStatus::Done)
            .map(|i| i.id)
            .collect();
        
        for id in done_items {
            self.delete_item(id);
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str("# Kanban\n\n");
        
        let statuses = [
            (KanbanStatus::Todo, "TODO"),
            (KanbanStatus::InProgress, "IN PROGRESS"),
            (KanbanStatus::Done, "DONE"),
        ];

        for (status, label) in statuses {
            output.push_str(&format!("## {}\n", label));
            let items: Vec<_> = self.items.iter().filter(|i| i.status == status).collect();
            if items.is_empty() {
                output.push_str("(No items)\n");
            } else {
                for item in items {
                    let checkbox = if status == KanbanStatus::Done { "[x]" } else { "[ ]" };
                    output.push_str(&format!("- {} {}\n", checkbox, item.text));
                }
            }
            output.push('\n');
        }

        output
    }

    pub fn from_markdown(content: &str) -> Self {
        let mut board = Self::default();
        let mut in_kanban_section = false;
        let mut current_status = None;
        let mut next_id = 1;

        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed == "# Kanban" {
                in_kanban_section = true;
                continue;
            }
            
            if in_kanban_section && trimmed.starts_with("# ") && trimmed != "# Kanban" {
                break;
            }
            
            if in_kanban_section {
                if trimmed == "## TODO" {
                    current_status = Some(KanbanStatus::Todo);
                } else if trimmed == "## IN PROGRESS" {
                    current_status = Some(KanbanStatus::InProgress);
                } else if trimmed == "## DONE" {
                    current_status = Some(KanbanStatus::Done);
                } else if (trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ")) && current_status.is_some() {
                    let text = &trimmed[6..];
                    
                    if text != "(No items)" && !text.is_empty() {
                        board.items.push(KanbanItem {
                            id: next_id,
                            text: text.to_string(),
                            status: current_status.clone().unwrap(),
                            created_at: Local::now(),
                            updated_at: Local::now(),
                        });
                        next_id += 1;
                    }
                }
            }
        }
        board.next_id = next_id;
        board
    }

    pub fn sync_with_todo(&mut self, todo_list: &mut crate::todo::TodoList) {
        let mut changed_kanban = false;
        let mut changed_todo = false;

        // 1. Add new items from TodoList that aren't in Kanban
        for todo in &todo_list.active {
            if !self.items.iter().any(|k| k.text == todo.text) {
                self.add_item(todo.text.clone());
                changed_kanban = true;
            }
        }

        // 2. Add completed items from TodoList that aren't in Kanban (as Done)
        for todo in &todo_list.completed {
            if !self.items.iter().any(|k| k.text == todo.text) {
                let item = KanbanItem {
                    id: self.next_id,
                    text: todo.text.clone(),
                    status: KanbanStatus::Done,
                    created_at: Local::now(),
                    updated_at: Local::now(),
                };
                self.items.push(item);
                self.next_id += 1;
                changed_kanban = true;
            }
        }

        // 3. Sync Status changes
        for kanban_item in &self.items {
            match kanban_item.status {
                KanbanStatus::Done => {
                    // If Done in Kanban, it MUST be completed in Todo
                    if let Some(todo_pos) = todo_list.active.iter().position(|t| t.text == kanban_item.text) {
                        todo_list.toggle(todo_list.active[todo_pos].id);
                        changed_todo = true;
                    }
                }
                KanbanStatus::Todo | KanbanStatus::InProgress => {
                    // If NOT Done in Kanban, it MUST be active in Todo (if it exists)
                    if let Some(todo_pos) = todo_list.completed.iter().position(|t| t.text == kanban_item.text) {
                        todo_list.toggle(todo_list.completed[todo_pos].id);
                        changed_todo = true;
                    }
                }
            }
        }

        if changed_kanban {
            let _ = self.save();
        }
        if changed_todo {
            let _ = todo_list.save();
        }
    }
}
