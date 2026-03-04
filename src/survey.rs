//! Survey module for tracking post-session focus data
//!
//! This module handles storing and retrieving survey data about focus quality
//! during work sessions.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Survey data stored in ~/.config/otamot/survey.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurveyData {
    /// Average focus rating for today (1-10 scale)
    #[serde(default)]
    pub average_focus_today: f64,

    /// Overall average focus rating across all sessions (1-10 scale)
    #[serde(default)]
    pub average_focus: f64,

    /// List of things that helped with focus
    #[serde(default)]
    pub what_helped: Vec<String>,

    /// List of things that hurt focus
    #[serde(default)]
    pub what_hurt: Vec<String>,

    /// Internal: total focus ratings (for calculating average)
    #[serde(default)]
    pub total_focus: u32,

    /// Internal: count of focus ratings (for calculating average)
    #[serde(default)]
    pub focus_count: u32,

    /// Internal: daily focus ratings by date
    #[serde(default)]
    pub daily_ratings: HashMap<String, Vec<u32>>,

    /// Total sessions completed across all time
    #[serde(default)]
    pub sessions_completed: u32,
}

impl SurveyData {
    /// Load survey data from file, or return defaults if file doesn't exist
    pub fn load() -> Self {
        let path = Self::survey_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save survey data to file
    pub fn save(&self) -> io::Result<()> {
        let path = Self::survey_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get the survey file path
    fn survey_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/otamot/survey.json")
    }

    /// Add a new survey response
    pub fn add_response(&mut self, focus_rating: u32, what_helped: String, what_hurt: String) {
        let today = Local::now().format("%Y-%m-%d").to_string();

        // Update total focus tracking
        self.total_focus += focus_rating;
        self.focus_count += 1;

        // Update overall average
        self.average_focus = self.total_focus as f64 / self.focus_count as f64;

        // Update daily ratings
        self.daily_ratings
            .entry(today.clone())
            .or_default()
            .push(focus_rating);

        // Calculate today's average
        if let Some(ratings) = self.daily_ratings.get(&today) {
            let sum: u32 = ratings.iter().sum();
            self.average_focus_today = sum as f64 / ratings.len() as f64;
        }

        // Add what helped/hurt (avoid duplicates)
        if !what_helped.is_empty() && !self.what_helped.contains(&what_helped) {
            self.what_helped.push(what_helped);
        }
        if !what_hurt.is_empty() && !self.what_hurt.contains(&what_hurt) {
            self.what_hurt.push(what_hurt);
        }
    }

    /// Reset all survey data
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get the survey file path (public for testing)
    pub fn get_survey_path() -> PathBuf {
        Self::survey_path()
    }

    /// Save to a specific path (for testing)
    pub fn save_to_path(&self, path: &PathBuf) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load from a specific path (for testing)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_survey_data_default() {
        let data = SurveyData::default();
        assert_eq!(data.average_focus_today, 0.0);
        assert_eq!(data.average_focus, 0.0);
        assert!(data.what_helped.is_empty());
        assert!(data.what_hurt.is_empty());
        assert_eq!(data.total_focus, 0);
        assert_eq!(data.focus_count, 0);
    }

    #[test]
    fn test_add_response_first() {
        let mut data = SurveyData::default();
        data.add_response(
            8,
            "Good coffee".to_string(),
            "Slack notifications".to_string(),
        );

        assert_eq!(data.total_focus, 8);
        assert_eq!(data.focus_count, 1);
        assert_eq!(data.average_focus, 8.0);
        assert_eq!(data.average_focus_today, 8.0);
        assert_eq!(data.what_helped.len(), 1);
        assert_eq!(data.what_helped[0], "Good coffee");
        assert_eq!(data.what_hurt.len(), 1);
        assert_eq!(data.what_hurt[0], "Slack notifications");
    }

    #[test]
    fn test_add_response_multiple() {
        let mut data = SurveyData::default();
        data.add_response(8, "Coffee".to_string(), "Slack".to_string());
        data.add_response(6, "Music".to_string(), "Email".to_string());

        assert_eq!(data.total_focus, 14);
        assert_eq!(data.focus_count, 2);
        assert_eq!(data.average_focus, 7.0);
        assert_eq!(data.what_helped.len(), 2);
        assert_eq!(data.what_hurt.len(), 2);
    }

    #[test]
    fn test_add_response_no_duplicates() {
        let mut data = SurveyData::default();
        data.add_response(8, "Coffee".to_string(), "Slack".to_string());
        data.add_response(7, "Coffee".to_string(), "Slack".to_string());

        // Should not add duplicates
        assert_eq!(data.what_helped.len(), 1);
        assert_eq!(data.what_hurt.len(), 1);
    }

    #[test]
    fn test_add_response_empty_strings() {
        let mut data = SurveyData::default();
        data.add_response(8, "".to_string(), "".to_string());

        assert!(data.what_helped.is_empty());
        assert!(data.what_hurt.is_empty());
    }

    #[test]
    fn test_reset_survey_data() {
        let mut data = SurveyData::default();
        data.add_response(8, "Coffee".to_string(), "Slack".to_string());

        data.reset();

        assert_eq!(data.average_focus_today, 0.0);
        assert_eq!(data.average_focus, 0.0);
        assert!(data.what_helped.is_empty());
        assert!(data.what_hurt.is_empty());
        assert_eq!(data.total_focus, 0);
        assert_eq!(data.focus_count, 0);
    }

    #[test]
    fn test_save_and_load_survey() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("survey.json");

        let mut data = SurveyData::default();
        data.add_response(9, "Music".to_string(), "Noisy neighbors".to_string());

        data.save_to_path(&path).unwrap();
        assert!(path.exists());

        let loaded = SurveyData::load_from_path(&path);
        assert_eq!(loaded.total_focus, 9);
        assert_eq!(loaded.focus_count, 1);
        assert_eq!(loaded.what_helped.len(), 1);
        assert_eq!(loaded.what_hurt.len(), 1);
    }

    #[test]
    fn test_survey_serialization() {
        let mut data = SurveyData::default();
        data.add_response(8, "Coffee".to_string(), "Phone".to_string());

        let json = serde_json::to_string(&data).unwrap();
        let parsed: SurveyData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total_focus, 8);
        assert_eq!(parsed.focus_count, 1);
        assert_eq!(parsed.what_helped.len(), 1);
        assert_eq!(parsed.what_hurt.len(), 1);
    }
}
