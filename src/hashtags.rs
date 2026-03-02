//! Hashtag library for the notes editor
//!
//! Manages a collection of hashtags that can be easily inserted into notes.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Hashtag library manager
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HashtagLibrary {
    /// Set of saved hashtags (without the # prefix)
    hashtags: HashSet<String>,
}

impl HashtagLibrary {
    /// Create a new empty hashtag library
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hashtag to the library
    pub fn add(&mut self, hashtag: &str) {
        // Remove # prefix if present
        let tag = hashtag.trim_start_matches('#').to_lowercase();
        if !tag.is_empty() {
            self.hashtags.insert(tag);
        }
    }

    /// Remove a hashtag from the library
    pub fn remove(&mut self, hashtag: &str) -> bool {
        let tag = hashtag.trim_start_matches('#').to_lowercase();
        self.hashtags.remove(&tag)
    }

    /// Check if a hashtag exists
    pub fn contains(&self, hashtag: &str) -> bool {
        let tag = hashtag.trim_start_matches('#').to_lowercase();
        self.hashtags.contains(&tag)
    }

    /// List all hashtags
    pub fn list(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.hashtags.iter().cloned().collect();
        tags.sort();
        tags
    }

    /// Search hashtags by prefix
    pub fn search(&self, query: &str) -> Vec<String> {
        let query = query.trim_start_matches('#').to_lowercase();
        let mut results: Vec<String> = self
            .hashtags
            .iter()
            .filter(|tag| tag.starts_with(&query))
            .cloned()
            .collect();
        results.sort();
        results
    }

    /// Extract hashtags from text and add them to the library
    pub fn extract_and_add(&mut self, text: &str) {
        for word in text.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                // Extract hashtag, removing any trailing punctuation
                let tag = word
                    .trim_start_matches('#')
                    .trim_end_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if !tag.is_empty() && tag.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    self.hashtags.insert(tag);
                }
            }
        }
    }

    /// Load hashtags from config directory
    pub fn load() -> Self {
        let config_dir = std::env::var("HOME")
            .map(|home| format!("{}/.config/otamot", home))
            .unwrap_or_else(|_| ".otamot".to_string());

        let hashtags_file = std::path::PathBuf::from(&config_dir).join("hashtags.json");

        if hashtags_file.exists() {
            match std::fs::read_to_string(&hashtags_file) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(library) => return library,
                    Err(e) => eprintln!("Failed to parse hashtags.json: {}", e),
                },
                Err(e) => eprintln!("Failed to read hashtags.json: {}", e),
            }
        }

        Self::default()
    }

    /// Save hashtags to config directory
    pub fn save(&self) {
        let config_dir = std::env::var("HOME")
            .map(|home| format!("{}/.config/otamot", home))
            .unwrap_or_else(|_| ".otamot".to_string());

        let config_path = std::path::PathBuf::from(&config_dir);

        if let Err(e) = std::fs::create_dir_all(&config_path) {
            eprintln!("Failed to create config directory: {}", e);
            return;
        }

        let hashtags_file = config_path.join("hashtags.json");

        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&hashtags_file, content) {
                    eprintln!("Failed to save hashtags.json: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to serialize hashtags: {}", e),
        }
    }

    /// Find hashtag at cursor position
    /// Returns (start_index, hashtag_text_without_hash) if a hashtag is being typed
    pub fn find_hashtag_at_cursor(text: &str, cursor_pos: usize) -> Option<(usize, String)> {
        // Find the start of the current line
        let line_start = text[..cursor_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_text = &text[line_start..cursor_pos];

        // Find the last '#' in the line before cursor
        if let Some(hash_pos) = line_text.rfind('#') {
            let after_hash = &line_text[hash_pos + 1..];

            // Check if there's a space between # and cursor (hashtag ended)
            if after_hash.contains(' ') {
                return None;
            }

            // Hashtag characters should be alphanumeric or underscore
            if !after_hash.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return None;
            }

            let tag_text = after_hash.to_string();
            let absolute_pos = line_start + hash_pos;

            // Only show dropdown if tag is not too long
            if tag_text.len() <= 30 {
                return Some((absolute_pos, tag_text));
            }
        }

        None
    }

    /// Insert hashtag into text, replacing the partial hashtag
    pub fn insert_hashtag(text: &str, cursor_pos: usize, hash_start: usize, tag: &str) -> String {
        let before = &text[..hash_start];
        let after = &text[cursor_pos..];
        format!("{}#{} {}", before, tag, after)
    }

    /// Get count of hashtags
    pub fn count(&self) -> usize {
        self.hashtags.len()
    }

    /// Clear all hashtags
    pub fn clear(&mut self) {
        self.hashtags.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_hashtag() {
        let mut library = HashtagLibrary::new();
        library.add("#work");
        assert!(library.contains("work"));
        assert!(library.contains("#work")); // Should work with or without #
    }

    #[test]
    fn test_remove_hashtag() {
        let mut library = HashtagLibrary::new();
        library.add("work");
        assert!(library.remove("work"));
        assert!(!library.contains("work"));
    }

    #[test]
    fn test_search_hashtags() {
        let mut library = HashtagLibrary::new();
        library.add("work");
        library.add("weekend");
        library.add("personal");

        let results = library.search("w");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"weekend".to_string()));
        assert!(results.contains(&"work".to_string()));
    }

    #[test]
    fn test_extract_and_add() {
        let mut library = HashtagLibrary::new();
        library.extract_and_add("Working on #project1 and #project_2 today!");

        assert!(library.contains("project1"));
        assert!(library.contains("project_2")); // Underscore preserved
    }

    #[test]
    fn test_find_hashtag_at_cursor() {
        let text = "Working on #pro today";
        // Cursor at position 15 is after "#pro" (positions: "Working on " = 11 chars, '#' at 11, 'pro' = 12-14)
        let result = HashtagLibrary::find_hashtag_at_cursor(text, 15);
        assert!(result.is_some());
        let (pos, tag) = result.unwrap();
        assert_eq!(pos, 11); // Position of '#'
        assert_eq!(tag, "pro");
    }

    #[test]
    fn test_no_hashtag_after_space() {
        let text = "Working on #pro today";
        // Cursor at position 16 is after the space following "#pro"
        let result = HashtagLibrary::find_hashtag_at_cursor(text, 16);
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_hashtag() {
        let text = "Working on #pro today";
        // Cursor at 15 (after "#pro"), hash_start at 11 (position of '#')
        let result = HashtagLibrary::insert_hashtag(text, 15, 11, "project");
        assert_eq!(result, "Working on #project  today");
    }

    #[test]
    fn test_case_insensitive() {
        let mut library = HashtagLibrary::new();
        library.add("#Work");
        assert!(library.contains("work"));
        assert!(library.contains("WORK"));
        assert!(library.contains("#WoRk"));
    }

    #[test]
    fn test_list_sorted() {
        let mut library = HashtagLibrary::new();
        library.add("zebra");
        library.add("apple");
        library.add("mango");

        let list = library.list();
        assert_eq!(list, vec!["apple", "mango", "zebra"]);
    }
}
