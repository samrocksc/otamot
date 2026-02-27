// Otamot - A cross-platform Pomodoro timer
// This lib.rs exposes modules for testing

pub mod config;
pub mod timer;
pub mod markdown;
pub mod notes;
pub mod survey;

// Note: app module uses eframe which requires a GUI environment
// It's tested via integration tests rather than unit tests
