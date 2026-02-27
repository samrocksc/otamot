use chrono::Local;
use eframe::egui;
use std::time::{Duration, Instant};

// Since app.rs is included from main.rs, we use otamot:: for library imports
use otamot::bell::Bell;
use otamot::config::{Config, NotesView};
use otamot::survey::SurveyData;
use otamot::timer::TimerMode;

pub struct PomodoroApp {
    // Timer state
    mode: TimerMode,
    remaining_seconds: u32,
    is_running: bool,
    last_tick: Option<Instant>,
    session_start: Option<chrono::DateTime<Local>>,
    session_end: Option<chrono::DateTime<Local>>,

    // Configuration
    config: Config,

    // Bell sound
    bell: Bell,

    // UI state
    show_settings: bool,
    temp_work_duration: u32,
    temp_break_duration: u32,
    temp_notes_directory: String,

    // Notes state
    notes_enabled: bool,
    notes_content: String,
    notes_view: NotesView,
    focus_notes_input: bool, // Flag to request focus on notes text input

    // Session metadata
    sessions_completed: u32,

    // Survey state
    show_survey: bool,
    survey_data: SurveyData,
    survey_focus_rating: u32,
    survey_what_helped: String,
    survey_what_hurt: String,
}

impl PomodoroApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let remaining_seconds = config.work_duration * 60;
        let survey_data = SurveyData::load();
        let bell = Bell::default();

        Self {
            mode: TimerMode::Work,
            remaining_seconds,
            is_running: false,
            last_tick: None,
            session_start: None,
            session_end: None,
            config: config.clone(),
            bell,
            show_settings: false,
            temp_work_duration: config.work_duration,
            temp_break_duration: config.break_duration,
            temp_notes_directory: config.notes_directory.clone(),
            notes_enabled: config.notes_enabled,
            notes_content: String::new(),
            notes_view: NotesView::Edit,
            focus_notes_input: false,
            sessions_completed: 0,
            show_survey: false,
            survey_data,
            survey_focus_rating: 5,
            survey_what_helped: String::new(),
            survey_what_hurt: String::new(),
        }
    }

    fn format_time(&self) -> String {
        let minutes = self.remaining_seconds / 60;
        let seconds = self.remaining_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    fn toggle_timer(&mut self) {
        self.is_running = !self.is_running;
        if self.is_running {
            if self.session_start.is_none() {
                self.session_start = Some(Local::now());
            }
            self.last_tick = Some(Instant::now());
        }
    }

    fn reset_timer(&mut self) {
        self.is_running = false;
        self.mode = TimerMode::Work;
        self.remaining_seconds = self.config.work_duration * 60;
        self.last_tick = None;
        self.session_start = None;
        self.session_end = None;
    }

    fn skip_to_break(&mut self) {
        self.mode = TimerMode::Break;
        self.remaining_seconds = self.config.break_duration * 60;
        self.is_running = false;
        self.last_tick = None;
    }

    fn submit_survey(&mut self) {
        self.survey_data.add_response(
            self.survey_focus_rating,
            self.survey_what_helped.clone(),
            self.survey_what_hurt.clone(),
        );
        if let Err(e) = self.survey_data.save() {
            eprintln!("Failed to save survey data: {}", e);
        }

        // Reset survey form
        self.survey_focus_rating = 5;
        self.survey_what_helped.clear();
        self.survey_what_hurt.clear();
        self.show_survey = false;
    }

    fn skip_survey(&mut self) {
        self.show_survey = false;
        // Reset survey form
        self.survey_focus_rating = 5;
        self.survey_what_helped.clear();
        self.survey_what_hurt.clear();
    }

    fn reset_survey_data(&mut self) {
        self.survey_data.reset();
        if let Err(e) = self.survey_data.save() {
            eprintln!("Failed to reset survey data: {}", e);
        }
    }

    fn tick(&mut self) {
        if !self.is_running {
            return;
        }

        if let Some(last) = self.last_tick {
            let elapsed = last.elapsed();
            if elapsed >= Duration::from_secs(1) {
                if self.remaining_seconds > 0 {
                    self.remaining_seconds -= 1;
                } else {
                    // Timer complete - switch modes
                    // Play the bell sound to notify the user
                    self.bell.play();

                    let previous_mode = self.mode;
                    self.mode = match self.mode {
                        TimerMode::Work => {
                            // Record end time and save notes when work session completes
                            self.session_end = Some(Local::now());
                            if self.notes_enabled && !self.notes_content.is_empty() {
                                self.save_notes();
                            }
                            self.sessions_completed += 1;
                            self.remaining_seconds = self.config.break_duration * 60;
                            self.session_start = None;
                            self.session_end = None;
                            TimerMode::Break
                        }
                        TimerMode::Break => {
                            self.remaining_seconds = self.config.work_duration * 60;
                            TimerMode::Work
                        }
                    };

                    // Show survey after work session completes (if surveys are enabled)
                    if previous_mode == TimerMode::Work && self.config.survey_enabled {
                        self.show_survey = true;
                    }
                }
                self.last_tick = Some(Instant::now());
            }
        }
    }

    fn generate_frontmatter(
        &self,
        start: chrono::DateTime<Local>,
        end: chrono::DateTime<Local>,
    ) -> String {
        let mode = match self.mode {
            TimerMode::Work => "work",
            TimerMode::Break => "break",
        };

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
            end.format("%Y-%m-%d %H:%M:%S"),
            start.format("%Y-%m-%d %H:%M:%S"),
            end.format("%Y-%m-%d %H:%M:%S"),
            self.config.work_duration,
            mode,
            self.sessions_completed,
            mode
        )
    }

    fn save_notes(&mut self) {
        let notes_dir = std::path::PathBuf::from(&self.config.notes_directory);
        if let Err(e) = std::fs::create_dir_all(&notes_dir) {
            eprintln!("Failed to create notes directory: {}", e);
            return;
        }

        // Get start and end times
        let end_time = self.session_end.unwrap_or_else(Local::now);
        let start_time = self.session_start.unwrap_or(end_time);

        // Format filename: MM-DD-YYYY-HH-MM-HH-MM.md (start-end)
        let start_formatted = start_time.format("%m-%d-%Y-%H-%M");
        let end_formatted = end_time.format("%H-%M");
        let filename = format!("{}-{}.md", start_formatted, end_formatted);
        let filepath = notes_dir.join(&filename);

        let frontmatter = self.generate_frontmatter(start_time, end_time);
        let content = format!("{}{}", frontmatter, self.notes_content);

        if let Err(e) = std::fs::write(&filepath, &content) {
            eprintln!("Failed to save notes: {}", e);
        } else {
            println!("Notes saved to: {}", filepath.display());
            self.notes_content.clear();
        }
    }

    fn save_settings(&mut self) {
        self.config.work_duration = self.temp_work_duration;
        self.config.break_duration = self.temp_break_duration;
        self.config.notes_directory = self.temp_notes_directory.clone();

        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }

        // Reset timer if not running
        if !self.is_running {
            self.remaining_seconds = match self.mode {
                TimerMode::Work => self.config.work_duration * 60,
                TimerMode::Break => self.config.break_duration * 60,
            };
        }

        self.show_settings = false;
    }

    fn rounded_button(
        ui: &mut egui::Ui,
        label: &str,
        text_color: egui::Color32,
        bg_color: egui::Color32,
    ) -> egui::Response {
        ui.add(
            egui::Button::new(egui::RichText::new(label).color(text_color))
                .fill(bg_color)
                .rounding(8.0)
                .min_size(egui::vec2(70.0, 32.0)),
        )
    }

    fn render_markdown_preview(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut in_code_block = false;

            for line in self.notes_content.lines() {
                // Handle code blocks
                if line.trim().starts_with("```") {
                    in_code_block = !in_code_block;
                    ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                    );
                    continue;
                }

                if in_code_block {
                    ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .color(egui::Color32::from_rgb(0xaa, 0xaa, 0xaa)),
                    );
                    continue;
                }

                let trimmed = line.trim();

                if trimmed.starts_with("# ") {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("# ").unwrap_or(trimmed))
                            .size(20.0)
                            .strong()
                            .color(egui::Color32::from_rgb(0xee, 0xee, 0xee)),
                    );
                    ui.add_space(4.0);
                } else if trimmed.starts_with("## ") {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("## ").unwrap_or(trimmed))
                            .size(16.0)
                            .strong()
                            .color(egui::Color32::from_rgb(0xee, 0xee, 0xee)),
                    );
                    ui.add_space(3.0);
                } else if trimmed.starts_with("### ") {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("### ").unwrap_or(trimmed))
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::from_rgb(0xee, 0xee, 0xee)),
                    );
                    ui.add_space(2.0);
                } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    let bullet_text = format!(
                        "• {}",
                        trimmed
                            .strip_prefix("- ")
                            .or_else(|| trimmed.strip_prefix("* "))
                            .unwrap_or(trimmed)
                    );
                    ui.label(
                        egui::RichText::new(bullet_text)
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                    );
                } else if trimmed.starts_with("  - ") || trimmed.starts_with("  * ") {
                    // Indented list items
                    let bullet_text = format!(
                        "  ◦ {}",
                        trimmed[2..]
                            .trim()
                            .strip_prefix("- ")
                            .or_else(|| trimmed[2..].trim().strip_prefix("* "))
                            .unwrap_or(trimmed)
                    );
                    ui.label(
                        egui::RichText::new(bullet_text)
                            .color(egui::Color32::from_rgb(0xaa, 0xaa, 0xaa)),
                    );
                } else if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4
                {
                    let bold_text = trimmed[2..trimmed.len() - 2].to_string();
                    ui.label(
                        egui::RichText::new(bold_text)
                            .strong()
                            .color(egui::Color32::from_rgb(0xee, 0xee, 0xee)),
                    );
                } else if trimmed.is_empty() {
                    // Preserve empty lines as spacing
                    ui.add_space(6.0);
                } else {
                    // Regular paragraph text - handle inline bold
                    let text = self.format_inline_markdown(trimmed);
                    ui.label(
                        egui::RichText::new(text).color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                    );
                }
            }
        });
    }

    fn format_inline_markdown(&self, text: &str) -> String {
        // Simple inline formatting - remove markdown syntax for display
        let mut result = text.to_string();

        // Handle inline code `code`
        while let Some(start) = result.find('`') {
            if let Some(end) = result[start + 1..].find('`') {
                let code = &result[start + 1..start + 1 + end];
                result = format!(
                    "{}[{}]{}",
                    &result[..start],
                    code,
                    &result[start + 1 + end + 1..]
                );
            } else {
                break;
            }
        }

        // Remove bold markers but keep text
        result = result.replace("**", "");

        result
    }
}

impl eframe::App for PomodoroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Ctrl+P hotkey to toggle Edit/Preview
        if ctx.input(|i| i.key_pressed(egui::Key::P) && i.modifiers.ctrl) && self.notes_enabled {
            self.notes_view = match self.notes_view {
                NotesView::Edit => NotesView::Preview,
                NotesView::Preview => {
                    self.focus_notes_input = true; // Request focus when switching to Edit
                    NotesView::Edit
                }
            };
        }

        // Tick the timer
        self.tick();

        // Request repaint if running for smooth updates
        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Calculate window size based on notes visibility
        if self.notes_enabled {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(700.0, 420.0)));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(320.0, 420.0)));
        }

        // Dark theme colors
        let text_color = egui::Color32::from_rgb(0xee, 0xee, 0xee);
        let work_color = egui::Color32::from_rgb(0xe7, 0x4c, 0x3c);
        let break_color = egui::Color32::from_rgb(0x27, 0xae, 0x60);
        let button_color = egui::Color32::from_rgb(0x0f, 0x34, 0x60);
        let bg_color = egui::Color32::from_rgb(0x1a, 0x1a, 0x2e);
        let tab_active_color = egui::Color32::from_rgb(0x27, 0xae, 0x60);
        let tab_inactive_color = egui::Color32::from_rgb(0x0f, 0x34, 0x60);

        // Set dark background
        ctx.set_visuals(egui::Visuals {
            window_fill: bg_color,
            panel_fill: bg_color,
            ..Default::default()
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.notes_enabled {
                // Two-column layout when notes are enabled
                ui.horizontal(|ui| {
                    // Left column: Timer
                    ui.vertical(|ui| {
                        ui.set_min_width(200.0);
                        ui.add_space(30.0);

                        // Timer display
                        ui.label(
                            egui::RichText::new(self.format_time())
                                .size(48.0)
                                .color(text_color),
                        );

                        ui.add_space(10.0);

                        // Mode label
                        let (mode_label, mode_color) = match self.mode {
                            TimerMode::Work => ("WORK", work_color),
                            TimerMode::Break => ("BREAK", break_color),
                        };
                        ui.label(egui::RichText::new(mode_label).size(20.0).color(mode_color));

                        ui.add_space(20.0);

                        // Control buttons
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);

                            let button_label = if self.is_running { "PAUSE" } else { "START" };
                            if Self::rounded_button(ui, button_label, text_color, button_color)
                                .clicked()
                            {
                                self.toggle_timer();
                            }

                            ui.add_space(8.0);

                            if Self::rounded_button(ui, "RESET", text_color, button_color).clicked()
                            {
                                self.reset_timer();
                            }

                            ui.add_space(8.0);

                            if Self::rounded_button(ui, "SKIP", text_color, button_color).clicked()
                            {
                                self.skip_to_break();
                            }

                            ui.add_space(10.0);
                        });

                        ui.add_space(20.0);

                        // Settings button
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("⚙ Settings").color(text_color),
                                )
                                .fill(button_color)
                                .rounding(8.0),
                            )
                            .clicked()
                        {
                            self.temp_work_duration = self.config.work_duration;
                            self.temp_break_duration = self.config.break_duration;
                            self.temp_notes_directory = self.config.notes_directory.clone();
                            self.show_settings = true;
                        }

                        // Notes toggle (in timer column)
                        ui.add_space(15.0);
                        let toggle_label = if self.notes_enabled {
                            "📝 Notes: ON"
                        } else {
                            "📝 Notes: OFF"
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(toggle_label).color(text_color),
                                )
                                .fill(button_color)
                                .rounding(8.0),
                            )
                            .clicked()
                        {
                            self.notes_enabled = !self.notes_enabled;
                            self.config.notes_enabled = self.notes_enabled;
                            let _ = self.config.save();
                        }

                        // Save notes button
                        if self.notes_enabled && !self.notes_content.is_empty() {
                            ui.add_space(10.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("💾 Save Notes").color(text_color),
                                    )
                                    .fill(egui::Color32::from_rgb(0x27, 0xae, 0x60))
                                    .rounding(8.0),
                                )
                                .clicked()
                            {
                                self.session_end = Some(Local::now());
                                self.save_notes();
                            }
                        }

                        // Session counter
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("Sessions: {}", self.sessions_completed))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                        );

                        // Hotkey hint
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new("Ctrl+P: Toggle Preview")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(0x66, 0x66, 0x66)),
                        );
                    });

                    ui.add_space(20.0);

                    // Right column: Notes editor
                    ui.vertical(|ui| {
                        // Tab buttons
                        ui.horizontal(|ui| {
                            let edit_color = if self.notes_view == NotesView::Edit {
                                tab_active_color
                            } else {
                                tab_inactive_color
                            };
                            let preview_color = if self.notes_view == NotesView::Preview {
                                tab_active_color
                            } else {
                                tab_inactive_color
                            };

                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Edit").size(12.0).color(text_color),
                                    )
                                    .fill(edit_color)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(50.0, 20.0)),
                                )
                                .clicked()
                            {
                                self.notes_view = NotesView::Edit;
                                self.focus_notes_input = true; // Request focus when clicking Edit tab
                            }

                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Preview").size(12.0).color(text_color),
                                    )
                                    .fill(preview_color)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(60.0, 20.0)),
                                )
                                .clicked()
                            {
                                self.notes_view = NotesView::Preview;
                            }
                        });

                        ui.add_space(5.0);

                        match self.notes_view {
                            NotesView::Edit => {
                                // Request focus at the beginning of the frame if flag is set
                                if self.focus_notes_input {
                                    ctx.memory_mut(|mem| {
                                        mem.request_focus(egui::Id::new("notes_text_input"))
                                    });
                                    self.focus_notes_input = false;
                                }

                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.notes_content)
                                            .id(egui::Id::new("notes_text_input"))
                                            .desired_width(400.0)
                                            .desired_rows(15)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                });
                            }
                            NotesView::Preview => {
                                self.render_markdown_preview(ui);
                            }
                        }
                    });
                });
            } else {
                // Original layout when notes are disabled
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);

                    // Timer display
                    ui.label(
                        egui::RichText::new(self.format_time())
                            .size(48.0)
                            .color(text_color),
                    );

                    ui.add_space(10.0);

                    // Mode label
                    let (mode_label, mode_color) = match self.mode {
                        TimerMode::Work => ("WORK", work_color),
                        TimerMode::Break => ("BREAK", break_color),
                    };
                    ui.label(egui::RichText::new(mode_label).size(20.0).color(mode_color));

                    ui.add_space(30.0);

                    // Control buttons - centered with spacing
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);

                        let button_label = if self.is_running { "PAUSE" } else { "START" };
                        if Self::rounded_button(ui, button_label, text_color, button_color)
                            .clicked()
                        {
                            self.toggle_timer();
                        }

                        ui.add_space(10.0);

                        if Self::rounded_button(ui, "RESET", text_color, button_color).clicked() {
                            self.reset_timer();
                        }

                        ui.add_space(10.0);

                        if Self::rounded_button(ui, "SKIP", text_color, button_color).clicked() {
                            self.skip_to_break();
                        }

                        ui.add_space(20.0);
                    });

                    ui.add_space(40.0);

                    // Settings button
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("⚙ Settings").color(text_color))
                                .fill(button_color)
                                .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.temp_work_duration = self.config.work_duration;
                        self.temp_break_duration = self.config.break_duration;
                        self.temp_notes_directory = self.config.notes_directory.clone();
                        self.show_settings = true;
                    }

                    ui.add_space(10.0);

                    // Notes toggle
                    let toggle_label = if self.notes_enabled {
                        "📝 Notes: ON"
                    } else {
                        "📝 Notes: OFF"
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(toggle_label).color(text_color))
                                .fill(button_color)
                                .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.notes_enabled = !self.notes_enabled;
                        self.config.notes_enabled = self.notes_enabled;
                        let _ = self.config.save();
                    }

                    // Session counter
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("Sessions: {}", self.sessions_completed))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                    );
                });
            }
        });

        // Settings dialog
        if self.show_settings {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .constrain(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(350.0);

                    // Work duration
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Work Duration: {} min",
                                self.temp_work_duration
                            ))
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                        );

                        if ui
                            .add(egui::Button::new("-").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.temp_work_duration =
                                self.temp_work_duration.saturating_sub(1).max(1);
                        }
                        if ui
                            .add(egui::Button::new("+").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.temp_work_duration =
                                self.temp_work_duration.saturating_add(1).min(60);
                        }
                    });

                    ui.add_space(10.0);

                    // Break duration
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Break Duration: {} min",
                                self.temp_break_duration
                            ))
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                        );

                        if ui
                            .add(egui::Button::new("-").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.temp_break_duration =
                                self.temp_break_duration.saturating_sub(1).max(1);
                        }
                        if ui
                            .add(egui::Button::new("+").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.temp_break_duration =
                                self.temp_break_duration.saturating_add(1).min(30);
                        }
                    });

                    ui.add_space(15.0);

                    // Notes directory
                    ui.label(
                        egui::RichText::new("Notes Directory:")
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.temp_notes_directory)
                            .desired_width(300.0),
                    );

                    ui.add_space(15.0);

                    // Survey toggle
                    let survey_toggle_label = if self.config.survey_enabled {
                        "📊 Surveys: ON"
                    } else {
                        "📊 Surveys: OFF"
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(survey_toggle_label).color(text_color),
                            )
                            .fill(button_color)
                            .rounding(6.0),
                        )
                        .clicked()
                    {
                        self.config.survey_enabled = !self.config.survey_enabled;
                        let _ = self.config.save();
                    }

                    ui.add_space(10.0);

                    // Reset survey data button
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("🗑 Reset Survey Data")
                                    .color(egui::Color32::from_rgb(0xe7, 0x4c, 0x3c)),
                            )
                            .fill(button_color)
                            .rounding(6.0),
                        )
                        .clicked()
                    {
                        self.reset_survey_data();
                    }

                    ui.add_space(20.0);

                    // Dialog buttons
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("Cancel").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.show_settings = false;
                        }
                        if ui
                            .add(egui::Button::new("Save").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.save_settings();
                        }
                    });
                });
        }

        // Survey dialog
        if self.show_survey {
            egui::Window::new("Session Complete! 🎉")
                .collapsible(false)
                .resizable(false)
                .constrain(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(400.0);

                    ui.label(
                        egui::RichText::new("How was your focus this session?")
                            .size(16.0)
                            .color(text_color),
                    );

                    ui.add_space(15.0);

                    // Focus rating
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Focus Rating: {}/10",
                                self.survey_focus_rating
                            ))
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                        );

                        if ui
                            .add(egui::Button::new("-").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.survey_focus_rating =
                                self.survey_focus_rating.saturating_sub(1).max(1);
                        }
                        if ui
                            .add(egui::Button::new("+").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.survey_focus_rating =
                                self.survey_focus_rating.saturating_add(1).min(10);
                        }
                    });

                    ui.add_space(15.0);

                    // What helped
                    ui.label(
                        egui::RichText::new("What helped your focus? (optional)")
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.survey_what_helped)
                            .desired_width(350.0)
                            .hint_text("e.g., quiet room, coffee, music..."),
                    );

                    ui.add_space(10.0);

                    // What hurt
                    ui.label(
                        egui::RichText::new("What hurt your focus? (optional)")
                            .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.survey_what_hurt)
                            .desired_width(350.0)
                            .hint_text("e.g., notifications, noise, hunger..."),
                    );

                    ui.add_space(20.0);

                    // Show stats if available
                    if self.survey_data.focus_count > 0 {
                        ui.separator();
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Today's Avg Focus: {:.1}/10",
                                self.survey_data.average_focus_today
                            ))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Overall Avg Focus: {:.1}/10",
                                self.survey_data.average_focus
                            ))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                        );
                        ui.add_space(10.0);
                    }

                    // Dialog buttons
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("Skip").fill(button_color).rounding(6.0))
                            .clicked()
                        {
                            self.skip_survey();
                        }
                        if ui
                            .add(
                                egui::Button::new("Submit")
                                    .fill(egui::Color32::from_rgb(0x27, 0xae, 0x60))
                                    .rounding(6.0),
                            )
                            .clicked()
                        {
                            self.submit_survey();
                        }
                    });
                });
        }
    }
}
