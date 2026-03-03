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
}

impl KanbanBoard {
    pub fn load() -> Self {
        let path = Self::path();
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self {
                items: Vec::new(),
                history: Vec::new(),
                next_id: 1,
            }
        }
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
}
