//! Timer logic for the Pomodoro app
//! 
//! This module contains the core timer functionality, extracted for testability.

use std::time::Instant;

/// Represents the current mode of the timer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    Work,
    Break,
}

/// Core timer state, separated from UI concerns
#[derive(Debug)]
pub struct TimerState {
    pub mode: TimerMode,
    pub remaining_seconds: u32,
    pub is_running: bool,
    pub last_tick: Option<Instant>,
    pub sessions_completed: u32,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            mode: TimerMode::Work,
            remaining_seconds: 25 * 60, // Default 25 minutes
            is_running: false,
            last_tick: None,
            sessions_completed: 0,
        }
    }
}

impl TimerState {
    /// Create a new timer state with specified work duration (in minutes)
    pub fn new(work_duration: u32, _break_duration: u32) -> Self {
        Self {
            mode: TimerMode::Work,
            remaining_seconds: work_duration * 60,
            is_running: false,
            last_tick: None,
            sessions_completed: 0,
        }
    }

    /// Format the remaining time as MM:SS
    pub fn format_time(&self) -> String {
        format_time(self.remaining_seconds)
    }

    /// Toggle the timer running state
    pub fn toggle(&mut self) {
        self.is_running = !self.is_running;
        if self.is_running {
            self.last_tick = Some(Instant::now());
        }
    }

    /// Reset the timer to initial state
    pub fn reset(&mut self, work_duration: u32) {
        self.is_running = false;
        self.mode = TimerMode::Work;
        self.remaining_seconds = work_duration * 60;
        self.last_tick = None;
    }

    /// Skip to break mode
    pub fn skip_to_break(&mut self, break_duration: u32) {
        self.mode = TimerMode::Break;
        self.remaining_seconds = break_duration * 60;
        self.is_running = false;
        self.last_tick = None;
    }

    /// Decrement the timer by one second
    /// Returns true if a mode transition occurred
    pub fn tick(&mut self, work_duration: u32, break_duration: u32) -> bool {
        if !self.is_running {
            return false;
        }

        if self.remaining_seconds > 0 {
            self.remaining_seconds -= 1;
            false
        } else {
            // Timer complete - switch modes
            match self.mode {
                TimerMode::Work => {
                    self.sessions_completed += 1;
                    self.mode = TimerMode::Break;
                    self.remaining_seconds = break_duration * 60;
                }
                TimerMode::Break => {
                    self.mode = TimerMode::Work;
                    self.remaining_seconds = work_duration * 60;
                }
            }
            true
        }
    }

    /// Advance time by a specific duration (for testing)
    pub fn advance_by(&mut self, seconds: u32) {
        if seconds > self.remaining_seconds {
            self.remaining_seconds = 0;
        } else {
            self.remaining_seconds -= seconds;
        }
    }
}

/// Format seconds into MM:SS string
pub fn format_time(seconds: u32) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", minutes, secs)
}

/// Parse a time string (MM:SS) into seconds
pub fn parse_time(time_str: &str) -> Option<u32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let minutes: u32 = parts[0].parse().ok()?;
    let seconds: u32 = parts[1].parse().ok()?;
    Some(minutes * 60 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_zero() {
        assert_eq!(format_time(0), "00:00");
    }

    #[test]
    fn test_format_time_seconds_only() {
        assert_eq!(format_time(30), "00:30");
        assert_eq!(format_time(59), "00:59");
    }

    #[test]
    fn test_format_time_minutes_and_seconds() {
        assert_eq!(format_time(65), "01:05");
        assert_eq!(format_time(125), "02:05");
        assert_eq!(format_time(3661), "61:01");
    }

    #[test]
    fn test_format_time_typical_durations() {
        assert_eq!(format_time(25 * 60), "25:00"); // Work duration
        assert_eq!(format_time(5 * 60), "05:00");  // Break duration
        assert_eq!(format_time(15 * 60), "15:00"); // Long break
    }

    #[test]
    fn test_parse_time_valid() {
        assert_eq!(parse_time("00:00"), Some(0));
        assert_eq!(parse_time("00:30"), Some(30));
        assert_eq!(parse_time("01:00"), Some(60));
        assert_eq!(parse_time("25:00"), Some(1500));
        assert_eq!(parse_time("05:30"), Some(330));
    }

    #[test]
    fn test_parse_time_invalid() {
        assert_eq!(parse_time(""), None);
        assert_eq!(parse_time("1:2:3"), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time("1:abc"), None);
    }

    #[test]
    fn test_timer_state_default() {
        let state = TimerState::default();
        assert_eq!(state.mode, TimerMode::Work);
        assert_eq!(state.remaining_seconds, 25 * 60);
        assert!(!state.is_running);
        assert!(state.last_tick.is_none());
        assert_eq!(state.sessions_completed, 0);
    }

    #[test]
    fn test_timer_state_new_custom_duration() {
        let state = TimerState::new(30, 10);
        assert_eq!(state.remaining_seconds, 30 * 60);
    }

    #[test]
    fn test_timer_toggle() {
        let mut state = TimerState::default();
        assert!(!state.is_running);
        
        state.toggle();
        assert!(state.is_running);
        assert!(state.last_tick.is_some());
        
        state.toggle();
        assert!(!state.is_running);
    }

    #[test]
    fn test_timer_reset() {
        let mut state = TimerState::default();
        state.toggle();
        state.remaining_seconds = 100;
        state.mode = TimerMode::Break;
        state.sessions_completed = 5;
        
        state.reset(25);
        
        assert!(!state.is_running);
        assert_eq!(state.mode, TimerMode::Work);
        assert_eq!(state.remaining_seconds, 25 * 60);
        assert!(state.last_tick.is_none());
        // Note: reset doesn't reset sessions_completed by design
    }

    #[test]
    fn test_timer_skip_to_break() {
        let mut state = TimerState::default();
        state.toggle();
        
        state.skip_to_break(5);
        
        assert_eq!(state.mode, TimerMode::Break);
        assert_eq!(state.remaining_seconds, 5 * 60);
        assert!(!state.is_running);
        assert!(state.last_tick.is_none());
    }

    #[test]
    fn test_timer_tick_not_running() {
        let mut state = TimerState::default();
        let initial_seconds = state.remaining_seconds;
        
        let transitioned = state.tick(25, 5);
        
        assert!(!transitioned);
        assert_eq!(state.remaining_seconds, initial_seconds);
    }

    #[test]
    fn test_timer_tick_decrements() {
        let mut state = TimerState::default();
        state.is_running = true;
        let initial_seconds = state.remaining_seconds;
        
        let transitioned = state.tick(25, 5);
        
        assert!(!transitioned);
        assert_eq!(state.remaining_seconds, initial_seconds - 1);
    }

    #[test]
    fn test_timer_tick_transition_work_to_break() {
        let mut state = TimerState::default();
        state.mode = TimerMode::Work;
        state.remaining_seconds = 0;
        state.is_running = true;
        
        let transitioned = state.tick(25, 5);
        
        assert!(transitioned);
        assert_eq!(state.mode, TimerMode::Break);
        assert_eq!(state.remaining_seconds, 5 * 60);
        assert_eq!(state.sessions_completed, 1);
    }

    #[test]
    fn test_timer_tick_transition_break_to_work() {
        let mut state = TimerState::default();
        state.mode = TimerMode::Break;
        state.remaining_seconds = 0;
        state.is_running = true;
        state.sessions_completed = 3;
        
        let transitioned = state.tick(25, 5);
        
        assert!(transitioned);
        assert_eq!(state.mode, TimerMode::Work);
        assert_eq!(state.remaining_seconds, 25 * 60);
        assert_eq!(state.sessions_completed, 3); // Not incremented on break->work
    }

    #[test]
    fn test_timer_multiple_sessions() {
        let mut state = TimerState::default();
        state.is_running = true;
        
        // Simulate completing a work session
        state.remaining_seconds = 0;
        let _ = state.tick(25, 5);
        assert_eq!(state.sessions_completed, 1);
        assert_eq!(state.mode, TimerMode::Break);
        
        // Simulate completing a break
        state.remaining_seconds = 0;
        let _ = state.tick(25, 5);
        assert_eq!(state.sessions_completed, 1);
        assert_eq!(state.mode, TimerMode::Work);
        
        // Another work session
        state.remaining_seconds = 0;
        let _ = state.tick(25, 5);
        assert_eq!(state.sessions_completed, 2);
    }

    #[test]
    fn test_timer_advance_by() {
        let mut state = TimerState::default();
        state.remaining_seconds = 100;
        
        state.advance_by(30);
        assert_eq!(state.remaining_seconds, 70);
        
        state.advance_by(100);
        assert_eq!(state.remaining_seconds, 0);
    }

    #[test]
    fn test_timer_mode_equality() {
        assert_eq!(TimerMode::Work, TimerMode::Work);
        assert_eq!(TimerMode::Break, TimerMode::Break);
        assert_ne!(TimerMode::Work, TimerMode::Break);
    }
}
