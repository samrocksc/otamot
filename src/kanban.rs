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
}

impl KanbanBoard {
    pub fn load() -> Self {
        let path = Self::path();
        let board = if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| {
                    serde_json::from_str::<KanbanBoard>(&content).ok()
                })
                .unwrap_or_default()
        } else {
            Self {
                items: Vec::new(),
                history: Vec::new(),
                next_id: 1,
                todo_sync: true,
            }
        };

        board
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/otamot/kanban.json")
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

    pub fn sync_with_todo(&mut self, todo_list: &mut crate::todo::TodoList) {
        let mut changed = false;

        // 1. Add new items from TodoList that aren't in Kanban
        for todo in &todo_list.active {
            if !self.items.iter().any(|k| k.text == todo.text) {
                self.add_item(todo.text.clone());
                changed = true;
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
                changed = true;
            }
        }

        // 3. Sync Status changes
        for kanban_item in &self.items {
            match kanban_item.status {
                KanbanStatus::Done => {
                    // If Done in Kanban, it MUST be completed in Todo
                    if let Some(todo_pos) = todo_list.active.iter().position(|t| t.text == kanban_item.text) {
                        todo_list.toggle(todo_list.active[todo_pos].id);
                        let _ = todo_list.save();
                    }
                }
                KanbanStatus::Todo | KanbanStatus::InProgress => {
                    // If NOT Done in Kanban, it MUST be active in Todo (if it exists)
                    if let Some(todo_pos) = todo_list.completed.iter().position(|t| t.text == kanban_item.text) {
                        todo_list.toggle(todo_list.completed[todo_pos].id);
                        let _ = todo_list.save();
                    }
                }
            }
        }

        if changed {
            let _ = self.save();
        }
    }
}
