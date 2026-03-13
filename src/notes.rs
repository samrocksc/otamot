//! Notes management for the Pomodoro app
//!
//! This module contains notes-related functionality including
//! frontmatter generation and filename formatting.

use chrono::{DateTime, Local, TimeZone};

/// Represents a note's metadata for frontmatter generation
#[derive(Debug, Clone)]
pub struct NoteMetadata {
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub work_duration: u32,
    pub mode: NoteMode,
    pub sessions_completed: u32,
}

/// Mode for note metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteMode {
    Work,
    Break,
    Call,
}

impl std::fmt::Display for NoteMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteMode::Work => write!(f, "work"),
            NoteMode::Break => write!(f, "break"),
            NoteMode::Call => write!(f, "call"),
        }
    }
}

/// Generate YAML frontmatter for a note
pub fn generate_frontmatter(meta: &NoteMetadata) -> String {
    format!(
        r#"---
title: "Pomodoro Session"
date: {}
start_time: {}
end_time: {}
duration_minutes: {}
mode: {}
sessions_completed: {}
tags:
  - pomodoro
  - {}
---

"#,
        meta.end_time.format("%Y-%m-%d %H:%M:%S"),
        meta.start_time.format("%Y-%m-%d %H:%M:%S"),
        meta.end_time.format("%Y-%m-%d %H:%M:%S"),
        meta.work_duration,
        meta.mode,
        meta.sessions_completed,
        meta.mode
    )
}

/// Generate a filename for a note based on start and end times
/// Format: MM-DD-YYYY-HH-MM-HH-MM.md (start date, then start-end times)
pub fn generate_filename(start: DateTime<Local>, end: DateTime<Local>) -> String {
    let start_formatted = start.format("%m-%d-%Y-%H-%M");
    let end_formatted = end.format("%H-%M");
    format!("{}-{}.md", start_formatted, end_formatted)
}

/// Parse a note filename back into start and end times
/// Returns (start_time, end_time) if successful
pub fn parse_filename(filename: &str) -> Option<(DateTime<Local>, DateTime<Local>)> {
    // Expected format: MM-DD-YYYY-HH-MM-HH-MM.md
    let basename = filename.strip_suffix(".md")?;

    let parts: Vec<&str> = basename.split('-').collect();
    if parts.len() != 7 {
        return None;
    }

    // Parse: MM-DD-YYYY-HH-MM-HH-MM
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: i32 = parts[2].parse().ok()?;
    let start_hour: u32 = parts[3].parse().ok()?;
    let start_minute: u32 = parts[4].parse().ok()?;
    let end_hour: u32 = parts[5].parse().ok()?;
    let end_minute: u32 = parts[6].parse().ok()?;

    let start = Local
        .with_ymd_and_hms(year, month, day, start_hour, start_minute, 0)
        .single()?;
    let end = Local
        .with_ymd_and_hms(year, month, day, end_hour, end_minute, 0)
        .single()?;

    Some((start, end))
}

/// Save a note to a file
pub fn save_note(directory: &str, filename: &str, content: &str) -> std::io::Result<()> {
    use std::fs;
    use std::path::Path;

    let dir_path = Path::new(directory);
    fs::create_dir_all(dir_path)?;

    let filepath = dir_path.join(filename);
    fs::write(filepath, content)?;

    Ok(())
}

/// Get path to draft file
pub fn draft_path(directory: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(directory).join("draft.md")
}

/// Save draft content
pub fn save_draft(directory: &str, content: &str) -> std::io::Result<()> {
    use std::fs;
    fs::create_dir_all(directory)?;
    fs::write(draft_path(directory), content)
}

/// Load draft content
pub fn load_draft(directory: &str) -> String {
    std::fs::read_to_string(draft_path(directory)).unwrap_or_default()
}

/// Clear draft file
pub fn clear_draft(directory: &str) -> std::io::Result<()> {
    let path = draft_path(directory);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    fn create_test_datetime(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
    ) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn test_generate_filename_basic() {
        let start = create_test_datetime(2024, 2, 27, 10, 30);
        let end = create_test_datetime(2024, 2, 27, 10, 55);

        let filename = generate_filename(start, end);
        assert_eq!(filename, "02-27-2024-10-30-10-55.md");
    }

    #[test]
    fn test_generate_filename_with_padding() {
        let start = create_test_datetime(2024, 1, 5, 9, 5);
        let end = create_test_datetime(2024, 1, 5, 9, 30);

        let filename = generate_filename(start, end);
        assert_eq!(filename, "01-05-2024-09-05-09-30.md");
    }

    #[test]
    fn test_generate_filename_midnight() {
        let start = create_test_datetime(2024, 12, 31, 23, 45);
        let end = create_test_datetime(2024, 12, 31, 0, 10);

        let filename = generate_filename(start, end);
        assert_eq!(filename, "12-31-2024-23-45-00-10.md");
    }

    #[test]
    fn test_parse_filename_valid() {
        let result = parse_filename("02-27-2024-10-30-10-55.md");
        assert!(result.is_some());

        let (start, end) = result.unwrap();
        assert_eq!(start.year(), 2024);
        assert_eq!(start.month(), 2);
        assert_eq!(start.day(), 27);
        assert_eq!(start.hour(), 10);
        assert_eq!(start.minute(), 30);
        assert_eq!(end.hour(), 10);
        assert_eq!(end.minute(), 55);
    }

    #[test]
    fn test_parse_filename_padded() {
        let result = parse_filename("01-05-2024-09-05-09-30.md");
        assert!(result.is_some());

        let (start, end) = result.unwrap();
        assert_eq!(start.month(), 1);
        assert_eq!(start.day(), 5);
        assert_eq!(start.hour(), 9);
        assert_eq!(start.minute(), 5);
        assert_eq!(end.hour(), 9);
        assert_eq!(end.minute(), 30);
    }

    #[test]
    fn test_parse_filename_no_extension() {
        let result = parse_filename("02-27-2024-10-30-10-55");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_filename_wrong_format() {
        assert!(parse_filename("2024-02-27.md").is_none());
        assert!(parse_filename("invalid.md").is_none());
        assert!(parse_filename("").is_none());
        assert!(parse_filename("02-27-2024-10-30.md").is_none()); // Too few parts
    }

    #[test]
    fn test_parse_filename_invalid_values() {
        assert!(parse_filename("13-27-2024-10-30-10-55.md").is_none()); // Invalid month
        assert!(parse_filename("02-32-2024-10-30-10-55.md").is_none()); // Invalid day
        assert!(parse_filename("02-27-2024-25-30-10-55.md").is_none()); // Invalid hour
    }

    #[test]
    fn test_generate_frontmatter_work() {
        let start = create_test_datetime(2024, 2, 27, 10, 0);
        let end = create_test_datetime(2024, 2, 27, 10, 25);

        let meta = NoteMetadata {
            start_time: start,
            end_time: end,
            work_duration: 25,
            mode: NoteMode::Work,
            sessions_completed: 3,
        };

        let frontmatter = generate_frontmatter(&meta);

        assert!(frontmatter.starts_with("---\n"));
        assert!(frontmatter.contains("title: \"Pomodoro Session\""));
        assert!(frontmatter.contains("date: 2024-02-27 10:25:00"));
        assert!(frontmatter.contains("start_time: 2024-02-27 10:00:00"));
        assert!(frontmatter.contains("end_time: 2024-02-27 10:25:00"));
        assert!(frontmatter.contains("duration_minutes: 25"));
        assert!(frontmatter.contains("mode: work"));
        assert!(frontmatter.contains("sessions_completed: 3"));
        assert!(frontmatter.contains("- pomodoro"));
        assert!(frontmatter.contains("- work"));
        assert!(frontmatter.ends_with("---\n\n"));
    }

    #[test]
    fn test_generate_frontmatter_break() {
        let start = create_test_datetime(2024, 2, 27, 10, 25);
        let end = create_test_datetime(2024, 2, 27, 10, 30);

        let meta = NoteMetadata {
            start_time: start,
            end_time: end,
            work_duration: 25,
            mode: NoteMode::Break,
            sessions_completed: 1,
        };

        let frontmatter = generate_frontmatter(&meta);

        assert!(frontmatter.contains("mode: break"));
        assert!(frontmatter.contains("- break"));
        assert!(frontmatter.contains("sessions_completed: 1"));
    }

    #[test]
    fn test_frontmatter_content_combination() {
        let start = create_test_datetime(2024, 2, 27, 9, 0);
        let end = create_test_datetime(2024, 2, 27, 9, 25);

        let meta = NoteMetadata {
            start_time: start,
            end_time: end,
            work_duration: 25,
            mode: NoteMode::Work,
            sessions_completed: 0,
        };

        let frontmatter = generate_frontmatter(&meta);
        let content = format!(
            "{}{}",
            frontmatter, "# My Notes\n\nThis is my work session."
        );

        assert!(content.starts_with("---\n"));
        assert!(content.contains("# My Notes"));
        assert!(content.contains("This is my work session."));
    }

    #[test]
    fn test_note_mode_display() {
        assert_eq!(format!("{}", NoteMode::Work), "work");
        assert_eq!(format!("{}", NoteMode::Break), "break");
        assert_eq!(format!("{}", NoteMode::Call), "call");
    }

    #[test]
    fn test_note_mode_equality() {
        assert_eq!(NoteMode::Work, NoteMode::Work);
        assert_eq!(NoteMode::Break, NoteMode::Break);
        assert_eq!(NoteMode::Call, NoteMode::Call);
        assert_ne!(NoteMode::Work, NoteMode::Break);
        assert_ne!(NoteMode::Work, NoteMode::Call);
        assert_ne!(NoteMode::Break, NoteMode::Call);
    }

    #[test]
    fn test_save_note() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();

        let result = save_note(
            dir_path,
            "test-note.md",
            "# Test Note\n\nThis is test content.",
        );

        assert!(result.is_ok());

        let filepath = temp_dir.path().join("test-note.md");
        assert!(filepath.exists());

        let content = std::fs::read_to_string(filepath).unwrap();
        assert!(content.contains("# Test Note"));
        assert!(content.contains("This is test content."));
    }

    #[test]
    fn test_save_note_creates_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("nested/notes/dir");
        let dir_path = nested_dir.to_str().unwrap();

        let result = save_note(dir_path, "test-note.md", "Test content");

        assert!(result.is_ok());
        assert!(nested_dir.exists());
    }
}
