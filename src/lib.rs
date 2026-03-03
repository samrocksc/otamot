// Otamot - A cross-platform Pomodoro timer
// This lib.rs exposes modules for testing

pub mod bell;
pub mod commands;
pub mod config;
pub mod easy_mark;
pub mod hashtags;
pub mod localization;
pub mod markdown;
pub mod notes;
pub mod survey;
pub mod timer;
pub mod todo;
pub mod ui_components;


// Note: app module uses eframe which requires a GUI environment
// It's tested via integration tests rather than unit tests
