use chrono::Local;
use eframe::egui;
use std::time::{Duration, Instant};

// otamot library imports
use otamot::bell::Bell;
use otamot::commands::CommandManager;
use otamot::config::{Config, Language, NotesView};
use otamot::easy_mark::editor::EasyMarkEditor;
use otamot::hashtags::HashtagLibrary;
use otamot::kanban::KanbanBoard;
use otamot::localization::T;
use otamot::markdown::{format_markdown, insert_date_bullet};
use otamot::survey::SurveyData;
use otamot::timer::TimerMode;
use otamot::todo::TodoList;
use otamot::ui_components;

/// Dropdown state for autocomplete
#[derive(Debug, Clone, PartialEq)]
enum DropdownType {
    Command,
    Hashtag,
}

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
    temp_language: Language,

    // Localization helper
    t: T,

    // Notes state
    notes_enabled: bool,
    notes_content: String,
    notes_view: NotesView,
    focus_notes_input: bool, // Flag to request focus on notes text input
    requested_cursor_pos: Option<usize>, // Requested cursor position for notes input
    notes_cursor_pos: usize, // Current cursor position in notes text input

    // Slash commands and hashtags
    command_manager: CommandManager,
    hashtag_library: HashtagLibrary,
    dropdown_visible: bool,
    dropdown_type: DropdownType,
    dropdown_items: Vec<String>,
    dropdown_selected: usize,
    dropdown_start_pos: usize, // Position of / or # in text

    // Help menu
    show_help: bool,

    // TODO list
    todo_list: TodoList,
    todo_input: String,

    // Session metadata
    sessions_completed: u32,

    // Survey state
    show_survey: bool,
    show_survey_summary: bool,
    survey_data: SurveyData,
    survey_focus_rating: u32,
    survey_what_helped: String,
    survey_what_hurt: String,
    todo_enabled: bool,
    editor: EasyMarkEditor,

    // Kanban state
    kanban_enabled: bool,
    kanban_board: KanbanBoard,
    kanban_input: String,

    sidebar_collapsed: bool,
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
            temp_language: config.language,
            t: T::new(config.language),
            notes_enabled: config.notes_enabled,
            notes_content: String::new(),
            notes_view: NotesView::Edit,
            focus_notes_input: false,
            requested_cursor_pos: None,
            notes_cursor_pos: 0,
            command_manager: CommandManager::with_commands(config.slash_commands.clone()),
            hashtag_library: HashtagLibrary::load(),
            dropdown_visible: false,
            dropdown_type: DropdownType::Command,
            dropdown_items: Vec::new(),
            dropdown_selected: 0,
            dropdown_start_pos: 0,
            show_help: false,
            todo_list: TodoList::load(),
            todo_input: String::new(),
            todo_enabled: config.todo_enabled,
            editor: EasyMarkEditor::default(),
            kanban_enabled: config.kanban_enabled,
            kanban_board: KanbanBoard::load(),
            kanban_input: String::new(),
            sidebar_collapsed: config.sidebar_collapsed,

            sessions_completed: 0,
            show_survey: false,
            show_survey_summary: false,
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

    fn render_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        ui.vertical(|ui| {
            ui.add_space(10.0);
            
            // Hamburger Menu Button
            let menu_btn = egui::Button::new(egui::RichText::new("☰").size(18.0).color(text_color))
                .fill(button_color)
                .rounding(egui::Rounding::same(8.0))
                .min_size(egui::vec2(32.0, 32.0));
            
            if ui.add(menu_btn).clicked() {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                self.config.sidebar_collapsed = self.sidebar_collapsed;
                let _ = self.config.save();
            }

            if !self.sidebar_collapsed {
                ui.add_space(20.0);
                
                // Settings button
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(self.t.settings_btn()).color(text_color))
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
                
                // Survey summary button
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(self.t.survey_summary_title()).color(text_color),
                        )
                        .fill(button_color)
                        .rounding(8.0),
                    )
                    .clicked()
                {
                    self.show_survey_summary = true;
                }

                ui.add_space(30.0);
                ui.separator();
                ui.add_space(10.0);

                // Toggles
                // Notes Toggle
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(if self.notes_enabled {
                                self.t.notes_on()
                            } else {
                                self.t.notes_off()
                            })
                            .color(text_color),
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

                ui.add_space(10.0);
                
                // TODO Toggle
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(if self.todo_enabled {
                                self.t.todo_on()
                            } else {
                                self.t.todo_off()
                            })
                            .color(text_color),
                        )
                        .fill(button_color)
                        .rounding(8.0),
                    )
                    .clicked()
                {
                    self.todo_enabled = !self.todo_enabled;
                    self.config.todo_enabled = self.todo_enabled;
                    let _ = self.config.save();
                }

                ui.add_space(10.0);
                
                // Kanban Toggle
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(if self.kanban_enabled {
                                "Kanban: ON"
                            } else {
                                "Kanban: OFF"
                            })
                            .color(text_color),
                        )
                        .fill(button_color)
                        .rounding(8.0),
                    )
                    .clicked()
                {
                    self.kanban_enabled = !self.kanban_enabled;
                    self.config.kanban_enabled = self.kanban_enabled;
                    let _ = self.config.save();
                }

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(self.t.help_button()).color(text_color))
                            .fill(button_color)
                            .rounding(8.0),
                    )
                    .clicked()
                {
                    self.show_help = !self.show_help;
                }

                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(self.t.sessions_completed_label(self.sessions_completed))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                );
            }
        });
    }

    fn render_timer(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            
            // Timer Display
            ui.label(
                egui::RichText::new(self.format_time())
                    .size(48.0)
                    .color(text_color),
            );
            
            ui.add_space(20.0);
            
            let (mode_label, mode_color) = match self.mode {
                TimerMode::Work => (self.t.timer_work(), work_color),
                TimerMode::Break => (self.t.timer_break(), break_color),
            };
            ui.label(egui::RichText::new(mode_label).size(20.0).color(mode_color));
            
            ui.add_space(30.0);
            
            // Timer Controls
            let label = if self.is_running {
                self.t.pause_button()
            } else {
                self.t.start_button()
            };
            if Self::rounded_button(ui, &label, text_color, button_color).clicked() {
                self.toggle_timer();
            }
            ui.add_space(8.0);
            if Self::rounded_button(ui, &self.t.reset_button(), text_color, button_color).clicked()
            {
                self.reset_timer();
            }
            ui.add_space(8.0);
            if Self::rounded_button(
                ui,
                &self.t.button_skip().to_uppercase(),
                text_color,
                button_color,
            )
            .clicked()
            {
                self.skip_to_break();
            }

            if self.notes_enabled && !self.notes_content.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.t.save_notes_btn()).color(text_color),
                            )
                            .fill(egui::Color32::from_rgb(0x27, 0xae, 0x60))
                            .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.save_notes();
                    }
                });
            }
        });
        ui.add_space(10.0);
        ui.separator();
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
                    self.bell.play();

                    let previous_mode = self.mode;
                    self.mode = match self.mode {
                        TimerMode::Work => {
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

                    if previous_mode == TimerMode::Work && self.config.survey_enabled {
                        self.show_survey = true;
                    }
                }
                self.last_tick = Some(Instant::now());
            }
        }
    }

    /// Extract hashtags from text content
    fn extract_hashtags(&self, text: &str) -> Vec<String> {
        let mut tags = Vec::new();
        for word in text.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word
                    .trim_start_matches('#')
                    .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
                    .to_lowercase();
                    if !tag.is_empty() && tag.chars().all(|c| c.is_alphanumeric() || c == '_') && !tags.contains(&tag) {
                        tags.push(tag);
                    }
            }
        }
        tags
    }

    fn save_notes(&mut self) {
        self.hashtag_library.save();

        let notes_dir = std::path::PathBuf::from(&self.config.notes_directory);
        if let Err(e) = std::fs::create_dir_all(&notes_dir) {
            eprintln!("Failed to create notes directory: {}", e);
            return;
        }

        let end_time = self.session_end.unwrap_or_else(Local::now);
        let start_time = self.session_start.unwrap_or(end_time);

        let start_formatted = start_time.format("%m-%d-%Y-%H-%M");
        let end_formatted = end_time.format("%H-%M");
        let filename = format!("{}-{}.md", start_formatted, end_formatted);
        let filepath = notes_dir.join(&filename);

        // Generate frontmatter (using local values)
        let mode_str = match self.mode {
            TimerMode::Work => "work",
            TimerMode::Break => "break",
        };
        let mut tags = vec!["pomodoro".to_string(), mode_str.to_string()];
        for tag in self.extract_hashtags(&self.notes_content) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        let tags_yaml = tags
            .iter()
            .map(|t| format!("  - {}", t))
            .collect::<Vec<_>>()
            .join("\n");

        let frontmatter = format!(
            "---\ntitle: \"Pomodoro Session\"\ndate: {}\nstart_time: {}\nend_time: {}\nduration_minutes: {}\nmode: {}\nsessions_completed: {}\ntags:\n{}\n---\n\n",
            end_time.format("%Y-%m-%d %H:%M:%S"),
            start_time.format("%Y-%m-%d %H:%M:%S"),
            end_time.format("%Y-%m-%d %H:%M:%S"),
            self.config.work_duration,
            mode_str,
            self.sessions_completed,
            tags_yaml
        );

        let content = format!("{}{}", frontmatter, self.notes_content);
        if let Err(e) = std::fs::write(&filepath, &content) {
            eprintln!("Failed to save notes: {}", e);
        } else {
            self.notes_content.clear();
        }
    }

    fn save_settings(&mut self) {
        self.config.work_duration = self.temp_work_duration;
        self.config.break_duration = self.temp_break_duration;
        self.config.notes_directory = self.temp_notes_directory.clone();
        self.config.language = self.temp_language;
        self.config.todo_enabled = self.todo_enabled;
        self.t = T::new(self.config.language);
        self.config.slash_commands = self.command_manager.get_commands();

        let _ = self.config.save();
        self.hashtag_library.save();

        if !self.is_running {
            self.remaining_seconds = match self.mode {
                TimerMode::Work => self.config.work_duration * 60,
                TimerMode::Break => self.config.break_duration * 60,
            };
        }
        self.show_settings = false;
    }

    /// Wrapper for the rounded button component
    fn rounded_button(
        ui: &mut egui::Ui,
        label: &str,
        text_color: egui::Color32,
        bg_color: egui::Color32,
    ) -> egui::Response {
        ui_components::rounded_button(ui, label, text_color, bg_color)
    }

    /// Helper for markdown rendering
    fn render_markdown_preview(&self, ui: &mut egui::Ui) {
        ui_components::render_markdown_preview(ui, &self.notes_content);
    }
}

// --- App Trait Implementation ---

impl eframe::App for PomodoroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Essential state updates
        self.tick();
        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Handle early keyboard input for the notes editor (before UI renders)
        if self.notes_enabled && self.notes_view == NotesView::Edit {
            let is_focused = ctx.memory(|mem| mem.has_focus(egui::Id::new("notes_text_input")));

            if is_focused && self.dropdown_visible && !self.dropdown_items.is_empty() {
                // Dropdown keyboard navigation
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.dropdown_selected =
                        (self.dropdown_selected + 1) % self.dropdown_items.len();
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.dropdown_selected = if self.dropdown_selected == 0 {
                        self.dropdown_items.len() - 1
                    } else {
                        self.dropdown_selected - 1
                    };
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let item = self.dropdown_items[self.dropdown_selected].clone();
                    self.apply_dropdown_selection(item);
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                }
            }

            let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
            let shift = ctx.input(|i| i.modifiers.shift);

            if tab_pressed && is_focused {
                // Dropdown pagination/navigation via Tab
                if self.dropdown_visible && !self.dropdown_items.is_empty() {
                    if shift {
                        self.dropdown_selected = if self.dropdown_selected == 0 {
                            self.dropdown_items.len() - 1
                        } else {
                            self.dropdown_selected - 1
                        };
                    } else {
                        self.dropdown_selected =
                            (self.dropdown_selected + 1) % self.dropdown_items.len();
                    }
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
                } else if !self.dropdown_visible {
                    // Consume Tab to prevent focus escape
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));

                    // Force focus back to the editor
                    ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("notes_text_input")));

                    // Get byte position from character position
                    let byte_pos = self
                        .notes_content
                        .char_indices()
                        .nth(self.notes_cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(self.notes_content.len());

                    // Find the start of the current line
                    let line_start = self.notes_content[..byte_pos]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);

                    // Get the full line content
                    let line_end = self.notes_content[byte_pos..]
                        .find('\n')
                        .map(|i| byte_pos + i)
                        .unwrap_or(self.notes_content.len());
                    let full_line = &self.notes_content[line_start..line_end];

                    // Check if this line is a list item (with optional leading spaces)
                    let trimmed = full_line.trim_start();
                    let is_list_item = trimmed.starts_with("- ")
                        || trimmed.starts_with("* ")
                        || (trimmed
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                            && trimmed.contains(". "));

                    if shift {
                        // Handle Outdent (Shift+Tab)
                        let line_content = &self.notes_content[line_start..];
                        if line_content.starts_with("  ") {
                            self.notes_content = format!(
                                "{}{}",
                                &self.notes_content[..line_start],
                                &self.notes_content[line_start + 2..]
                            );
                            self.requested_cursor_pos =
                                Some(self.notes_cursor_pos.saturating_sub(2));
                        } else if line_content.starts_with('\t') {
                            self.notes_content = format!(
                                "{}{}",
                                &self.notes_content[..line_start],
                                &self.notes_content[line_start + 1..]
                            );
                            self.requested_cursor_pos =
                                Some(self.notes_cursor_pos.saturating_sub(1));
                        }
                    } else if is_list_item {
                        // Handle Indent (Tab) on list item - insert spaces at line start
                        self.notes_content.insert_str(line_start, "  ");
                        self.requested_cursor_pos = Some(self.notes_cursor_pos + 2);
                    } else {
                        // Handle Indent (Tab) - insert 2 spaces at cursor position
                        self.notes_content.insert_str(byte_pos, "  ");
                        self.requested_cursor_pos = Some(self.notes_cursor_pos + 2);
                    }
                }
            }
        }

        // Global hotkey handling
        if ctx.input(|i| i.key_pressed(egui::Key::P) && i.modifiers.ctrl) && self.notes_enabled {
            if self.notes_view == NotesView::Edit {
                self.notes_content = format_markdown(&self.notes_content);
            }
            self.notes_view = match self.notes_view {
                NotesView::Edit => NotesView::Preview,
                NotesView::Preview => {
                    self.focus_notes_input = true;
                    NotesView::Edit
                }
            };
        }

        if ctx.input(|i| i.key_pressed(egui::Key::D) && i.modifiers.ctrl)
            && self.notes_enabled
            && self.notes_view == NotesView::Edit
        {
            self.notes_content = insert_date_bullet(&self.notes_content);
            self.focus_notes_input = true;
            self.requested_cursor_pos = Some(19);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Slash) && i.modifiers.ctrl && i.modifiers.shift) {
            self.show_help = !self.show_help;
        }

        // Handle Enter key to continue list items
        if ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && self.notes_enabled
            && self.notes_view == NotesView::Edit
            && !self.dropdown_visible
        {
            let is_focused = ctx.memory(|mem| mem.has_focus(egui::Id::new("notes_text_input")));

            if is_focused {
                // Get byte position from character position
                let byte_pos = self
                    .notes_content
                    .char_indices()
                    .nth(self.notes_cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(self.notes_content.len());

                // Find the start of the current line
                let line_start = self.notes_content[..byte_pos]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);

                let line_content = &self.notes_content[line_start..byte_pos];

                // Check for list markers at the start of the line (with optional leading spaces)
                let trimmed = line_content.trim_start();
                let leading_spaces = line_content.len() - trimmed.len();

                // Check for unordered list (- or *)
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    let marker = &trimmed[..2]; // "- " or "* "
                    let indent = &line_content[..leading_spaces];
                    let new_item = format!("\n{}{}", indent, marker);
                    self.notes_content.insert_str(byte_pos, &new_item);
                    self.requested_cursor_pos =
                        Some(self.notes_cursor_pos + new_item.chars().count());
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                }
                // Check for ordered list (1., 2., 3., etc.)
                else if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                    if rest.starts_with(". ") {
                        // Extract the number from the beginning
                        let num_str = trimmed
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>();
                        if let Ok(num) = num_str.parse::<u32>() {
                            let indent = &line_content[..leading_spaces];
                            let new_item = format!("\n{}{}. ", indent, num + 1);
                            self.notes_content.insert_str(byte_pos, &new_item);
                            self.requested_cursor_pos =
                                Some(self.notes_cursor_pos + new_item.chars().count());
                            ctx.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            });
                        }
                    }
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.dropdown_visible {
                self.dropdown_visible = false;
            } else {
                self.show_settings = false;
                self.show_help = false;
                self.show_survey = false;
                self.show_survey_summary = false;
            }
        }

        // Autocomplete drop-down logic
        if self.notes_enabled && self.notes_view == NotesView::Edit {
            let cursor_pos = self.notes_cursor_pos;
            if !self.dropdown_visible {
                if let Some((pos, cmd)) =
                    CommandManager::find_command_at_cursor(&self.notes_content, cursor_pos)
                {
                    self.dropdown_visible = true;
                    self.dropdown_type = DropdownType::Command;
                    self.dropdown_start_pos = pos;
                    self.dropdown_items = self.command_manager.search_commands(&cmd);
                    self.dropdown_selected = 0;
                } else if let Some((pos, tag)) =
                    HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, cursor_pos)
                {
                    self.dropdown_visible = true;
                    self.dropdown_type = DropdownType::Hashtag;
                    self.dropdown_start_pos = pos;
                    self.dropdown_items = self.hashtag_library.search(&tag);
                    self.dropdown_selected = 0;
                }
            } else {
                // Update dropdown state as user types
                match self.dropdown_type {
                    DropdownType::Command => {
                        if let Some((pos, cmd)) =
                            CommandManager::find_command_at_cursor(&self.notes_content, cursor_pos)
                        {
                            self.dropdown_start_pos = pos;
                            self.dropdown_items = self.command_manager.search_commands(&cmd);
                        } else {
                            self.dropdown_visible = false;
                        }
                    }
                    DropdownType::Hashtag => {
                        if let Some((pos, tag)) =
                            HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, cursor_pos)
                        {
                            self.dropdown_start_pos = pos;
                            self.dropdown_items = self.hashtag_library.search(&tag);
                        } else {
                            self.dropdown_visible = false;
                        }
                    }
                }
            }
        }

        // Theme Definitions
        let text_color = egui::Color32::from_rgb(0xee, 0xee, 0xee);
        let work_color = egui::Color32::from_rgb(0xe7, 0x4c, 0x3c);
        let break_color = egui::Color32::from_rgb(0x27, 0xae, 0x60);
        let button_color = egui::Color32::from_rgb(0x0f, 0x34, 0x60);
        let bg_color = egui::Color32::from_rgb(0x1a, 0x1a, 0x2e);
        let tab_active_color = egui::Color32::from_rgb(0x27, 0xae, 0x60);
        let tab_inactive_color = egui::Color32::from_rgb(0x0f, 0x34, 0x60);

        ctx.set_visuals(egui::Visuals {
            window_fill: bg_color,
            panel_fill: bg_color,
            ..Default::default()
        });

        // Main UI Layout
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(15.0)))
            .show(ctx, |ui| {
            if self.notes_enabled || self.todo_enabled {
                // Get the full available height before entering horizontal layout
                let full_height = ui.available_height();
                let total_width = ui.available_width();
                let sidebar_width = if self.sidebar_collapsed { 50.0 } else { 200.0 };
                let right_width = total_width - sidebar_width - 20.0;
                ui.horizontal_top(|ui| {
                    // Side Pillar 1: Settings and toggles (Sidebar)
                    ui.allocate_ui(egui::vec2(sidebar_width, full_height), |ui| {
                        ui.vertical(|ui| {
                            // Sync Kanban with TODO if enabled
                            if self.kanban_enabled {
                                self.kanban_board.sync_with_todo(&mut self.todo_list);
                            }

                            egui::ScrollArea::vertical()
                                .id_salt("sidebar_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    self.render_sidebar(
                                        ui,
                                        text_color,
                                        button_color,
                                    );
                                });
                        });
                    });

                    ui.separator();

                    // Side Pillar 2: Timer + Notes area and/or TODOs
                    ui.allocate_ui(egui::vec2(right_width, full_height), |ui| {
                        ui.vertical(|ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("right_pillar_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    self.render_right_column(
                                        ctx,
                                        ui,
                                        text_color,
                                        tab_active_color,
                                        tab_inactive_color,
                                        button_color,
                                        work_color,
                                        break_color,
                                    );
                                });
                        });
                    });
                });
            } else {
                ui.centered_and_justified(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("pure_timer_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                self.render_pure_timer_layout(
                                    ui,
                                    text_color,
                                    button_color,
                                    work_color,
                                    break_color,
                                );
                            });
                        });
                });
            }
        });

        // Full-screen / Modal windows
        self.show_settings_dialog(ctx, text_color, button_color);
        self.show_help_dialog(ctx, text_color, button_color);
        self.show_survey_dialog(ctx, text_color, button_color);
        self.show_survey_summary_dialog(ctx, text_color, button_color);
    }
}

// --- Extended Methods Implementation ---

impl PomodoroApp {
    fn apply_dropdown_selection(&mut self, selected_item: String) {
        match self.dropdown_type {
            DropdownType::Command => {
                if let Some(replacement) = self.command_manager.execute(&selected_item) {
                    let cursor_pos = self.notes_content.len();
                    self.notes_content = CommandManager::insert_command(
                        &self.notes_content,
                        cursor_pos,
                        self.dropdown_start_pos,
                        &replacement,
                    );
                    self.requested_cursor_pos =
                        Some(self.dropdown_start_pos + replacement.chars().count());
                }
            }
            DropdownType::Hashtag => {
                let cursor_pos = self.notes_content.len();
                self.notes_content = HashtagLibrary::insert_hashtag(
                    &self.notes_content,
                    cursor_pos,
                    self.dropdown_start_pos,
                    &selected_item,
                );
                self.requested_cursor_pos =
                    Some(self.dropdown_start_pos + selected_item.chars().count() + 1);
                self.hashtag_library.add(&selected_item);
            }
        }
        self.dropdown_visible = false;
        self.dropdown_items.clear();
        self.focus_notes_input = true;
    }

    fn render_right_column(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        active_color: egui::Color32,
        inactive_color: egui::Color32,
        button_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
    ) {
        // Render Timer at the top of the right column
        self.render_timer(ui, text_color, button_color, work_color, break_color);

        // Render notes section if enabled
        if self.notes_enabled {
            // Tab buttons
            ui.horizontal(|ui| {
                let edit_color = if self.notes_view == NotesView::Edit {
                    active_color
                } else {
                    inactive_color
                };
                let preview_color = if self.notes_view == NotesView::Preview {
                    active_color
                } else {
                    inactive_color
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(self.t.edit_tab())
                                .size(12.0)
                                .color(text_color),
                        )
                        .fill(edit_color)
                        .rounding(4.0),
                    )
                    .clicked()
                {
                    self.notes_view = NotesView::Edit;
                    self.focus_notes_input = true;
                }
                if self.notes_enabled {
                    ui.add_space(5.0);
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(self.t.preview_tab())
                                .size(12.0)
                                .color(text_color),
                        )
                        .fill(preview_color)
                        .rounding(4.0),
                    )
                    .clicked()
                {
                    self.notes_view = NotesView::Preview;
                }
            });

            ui.add_space(5.0);
            match self.notes_view {
                NotesView::Edit => {
                    if self.focus_notes_input {
                        ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("notes_text_input")));
                        self.focus_notes_input = false;
                    }

                    let output = self.editor.show(ui, &mut self.notes_content);
                    let response = output.response.clone();

                    // Track cursor position for Tab handling
                    if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                        if let Some(range) = state.cursor.char_range() {
                            self.notes_cursor_pos = range.primary.index;
                        }
                    }

                    if let Some(pos) = self.requested_cursor_pos.take() {
                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::one(
                                    egui::text::CCursor::new(pos),
                                )));
                            state.store(ui.ctx(), response.id);
                            self.notes_cursor_pos = pos;
                        }
                    }

                    self.render_dropdown(ui, &output);
                }
                NotesView::Preview => {
                    // We render preview without an internal scroll here, let the pillar scroll handle it
                    self.render_markdown_preview(ui);
                }
            }
        }

        // Render TODO section if enabled
        if self.todo_enabled {
            if self.notes_enabled {
                ui.add_space(30.0);
            }
            ui_components::render_todo_panel(
                ui,
                &mut self.todo_list,
                &mut self.todo_input,
                &self.kanban_board,
                &self.t,
                text_color,
                button_color,
            );
        }

        // Render Kanban section if enabled
        if self.kanban_enabled {
            if self.notes_enabled || self.todo_enabled {
                ui.add_space(30.0);
            }
            ui_components::render_kanban_board(
                ui,
                &mut self.kanban_board,
                &mut self.kanban_input,
                &self.t,
            );
        }
    }

    fn render_pure_timer_layout(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(
                egui::RichText::new(self.format_time())
                    .size(48.0)
                    .color(text_color),
            );
            ui.add_space(10.0);
            let (label, color) = match self.mode {
                TimerMode::Work => (self.t.timer_work(), work_color),
                TimerMode::Break => (self.t.timer_break(), break_color),
            };
            ui.label(egui::RichText::new(label).size(20.0).color(color));
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                let btn = if self.is_running {
                    self.t.pause_button()
                } else {
                    self.t.start_button()
                };
                if Self::rounded_button(ui, &btn, text_color, button_color).clicked() {
                    self.toggle_timer();
                }
                ui.add_space(10.0);
                if Self::rounded_button(ui, &self.t.reset_button(), text_color, button_color)
                    .clicked()
                {
                    self.reset_timer();
                }
                ui.add_space(10.0);
                if Self::rounded_button(
                    ui,
                    &self.t.button_skip().to_uppercase(),
                    text_color,
                    button_color,
                )
                .clicked()
                {
                    self.skip_to_break();
                }
            });
            ui.add_space(40.0);
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(self.t.settings_btn()).color(text_color))
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
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(self.t.survey_summary_title()).color(text_color),
                    )
                    .fill(button_color)
                    .rounding(8.0),
                )
                .clicked()
            {
                self.show_survey_summary = true;
            }
            ui.add_space(10.0);
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(if self.notes_enabled {
                            self.t.notes_on()
                        } else {
                            self.t.notes_off()
                        })
                        .color(text_color),
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
            ui.add_space(10.0);
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(if self.todo_enabled {
                            self.t.todo_on()
                        } else {
                            self.t.todo_off()
                        })
                        .color(text_color),
                    )
                    .fill(button_color)
                    .rounding(8.0),
                )
                .clicked()
            {
                self.todo_enabled = !self.todo_enabled;
                self.config.todo_enabled = self.todo_enabled;
                let _ = self.config.save();
            }
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(self.t.sessions_completed_label(self.sessions_completed))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
            );
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(self.t.help_button()).color(text_color))
                        .fill(button_color)
                        .rounding(8.0),
                )
                .clicked()
            {
                self.show_help = true;
            }
        });
    }

    fn render_dropdown(&mut self, ui: &mut egui::Ui, output: &egui::text_edit::TextEditOutput) {
        if self.dropdown_visible && !self.dropdown_items.is_empty() {
            // Calculate cursor position in screen coordinates
            let mut dropdown_pos = output.response.rect.left_top(); // Default fallback

            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), output.response.id) {
                if let Some(range) = state.cursor.char_range() {
                    let cursor = output.galley.from_ccursor(range.primary);
                    // Get the position of the character, relative to the galley
                    let galley_cursor_rect = output.galley.pos_from_cursor(&cursor);

                    // Convert galley coordinates to screen coordinates
                    // output.galley_pos is the screen position of the top-left of the galley
                    dropdown_pos = output.galley_pos + galley_cursor_rect.left_bottom().to_vec2();

                    // Add a small vertical offset
                    dropdown_pos.y += 2.0;
                }
            }

            egui::Area::new(egui::Id::new("autocomplete_dropdown"))
                .fixed_pos(dropdown_pos)
                .pivot(egui::Align2::LEFT_TOP)
                .interactable(true)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(egui::Color32::from_rgb(0x2a, 0x2a, 0x3e))
                        .show(ui, |ui| {
                            ui.set_min_width(200.0);
                            ui.label(
                                egui::RichText::new(
                                    if self.dropdown_type == DropdownType::Command {
                                        self.t.autocomplete_commands()
                                    } else {
                                        self.t.autocomplete_hashtags()
                                    },
                                )
                                .weak()
                                .size(10.0),
                            );
                            egui::ScrollArea::vertical()
                                .id_salt("dropdown_scroll")
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    let mut selection = None;
                                    for (i, item) in self.dropdown_items.iter().enumerate() {
                                        let text = if self.dropdown_type == DropdownType::Hashtag {
                                            format!("#{}", item)
                                        } else {
                                            format!("/{}", item)
                                        };
                                        if ui
                                            .selectable_label(i == self.dropdown_selected, text)
                                            .clicked()
                                        {
                                            selection = Some(item.clone());
                                        }
                                    }
                                    if let Some(s) = selection {
                                        self.apply_dropdown_selection(s);
                                    }
                                });
                        });
                });
        }
    }

    pub fn show_settings_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_settings {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("settings_scroll_area")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(format!("⚙ {}", self.t.settings_title()))
                                .size(28.0)
                                .color(text_color)
                                .strong(),
                        );
                        ui.add_space(30.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} min",
                                    self.t.work_duration(),
                                    self.temp_work_duration
                                ))
                                .size(18.0)
                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                            );
                            if ui
                                .add(
                                    egui::Button::new("-")
                                        .fill(button_color)
                                        .min_size(egui::vec2(40.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.temp_work_duration =
                                    self.temp_work_duration.saturating_sub(1).max(1);
                            }
                            if ui
                                .add(
                                    egui::Button::new("+")
                                        .fill(button_color)
                                        .min_size(egui::vec2(40.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.temp_work_duration =
                                    self.temp_work_duration.saturating_add(1).min(60);
                            }
                        });
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} min",
                                    self.t.break_duration(),
                                    self.temp_break_duration
                                ))
                                .size(18.0)
                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                            );
                            if ui
                                .add(
                                    egui::Button::new("-")
                                        .fill(button_color)
                                        .min_size(egui::vec2(40.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.temp_break_duration =
                                    self.temp_break_duration.saturating_sub(1).max(1);
                            }
                            if ui
                                .add(
                                    egui::Button::new("+")
                                        .fill(button_color)
                                        .min_size(egui::vec2(40.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.temp_break_duration =
                                    self.temp_break_duration.saturating_add(1).min(30);
                            }
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.notes_directory())
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.temp_notes_directory)
                                    .desired_width(350.0),
                            );
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(if self.config.survey_enabled {
                                            self.t.surveys_on()
                                        } else {
                                            self.t.surveys_off()
                                        })
                                        .color(text_color),
                                    )
                                    .fill(button_color),
                                )
                                .clicked()
                            {
                                self.config.survey_enabled = !self.config.survey_enabled;
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(self.t.reset_survey_data_btn())
                                            .color(egui::Color32::from_rgb(0xe7, 0x4c, 0x3c)),
                                    )
                                    .fill(button_color),
                                )
                                .clicked()
                            {
                                self.reset_survey_data();
                            }
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.language_setting())
                                    .size(18.0)
                                    .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                            );
                            egui::ComboBox::from_label("")
                                .selected_text(match self.temp_language {
                                    Language::English => self.t.lang_en(),
                                    Language::German => self.t.lang_de(),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.temp_language,
                                        Language::English,
                                        self.t.lang_en(),
                                    );
                                    ui.selectable_value(
                                        &mut self.temp_language,
                                        Language::German,
                                        self.t.lang_de(),
                                    );
                                });
                        });
                        ui.add_space(40.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            if ui_components::rounded_button(
                                ui,
                                &self.t.button_cancel(),
                                text_color,
                                button_color,
                            )
                            .clicked()
                            {
                                self.show_settings = false;
                                self.temp_work_duration = self.config.work_duration;
                                self.temp_break_duration = self.config.break_duration;
                                self.temp_notes_directory = self.config.notes_directory.clone();
                                self.temp_language = self.config.language;
                            }
                            ui.add_space(15.0);
                            if ui
                                .add(
                                    egui::Button::new(self.t.button_save())
                                        .fill(egui::Color32::from_rgb(0x27, 0xae, 0x60))
                                        .rounding(8.0)
                                        .min_size(egui::vec2(100.0, 35.0)),
                                )
                                .clicked()
                            {
                                self.save_settings();
                                self.show_settings = false;
                            }
                        });
                        ui.add_space(20.0);
                    });
                });
        });
    }

    pub fn show_survey_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_survey {
            return;
        }
        egui::Window::new(format!("{} 🎉", self.t.survey_complete_title()))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("survey_scroll_area")
                    .show(ui, |ui| {
                        ui.set_min_width(400.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_focus())
                                .size(16.0)
                                .color(text_color),
                        );
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(
                                    self.t.survey_rating_label(self.survey_focus_rating),
                                )
                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                            );
                            if ui.add(egui::Button::new("-").fill(button_color)).clicked() {
                                self.survey_focus_rating =
                                    self.survey_focus_rating.saturating_sub(1).max(1);
                            }
                            if ui.add(egui::Button::new("+").fill(button_color)).clicked() {
                                self.survey_focus_rating =
                                    self.survey_focus_rating.saturating_add(1).min(10);
                            }
                        });
                        ui.add_space(15.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_helped())
                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.survey_what_helped)
                                .desired_width(350.0)
                                .hint_text(self.t.helped_hint()),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_hurt())
                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.survey_what_hurt)
                                .desired_width(350.0)
                                .hint_text(self.t.hurt_hint()),
                        );
                        ui.add_space(20.0);
                        if self.survey_data.focus_count > 0 {
                            ui.separator();
                            ui.add_space(5.0);
                            ui.label(
                                egui::RichText::new(
                                    self.t.avg_focus_today(self.survey_data.average_focus_today),
                                )
                                .size(12.0)
                                .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                            );
                            ui.label(
                                egui::RichText::new(
                                    self.t.avg_focus_overall(self.survey_data.average_focus),
                                )
                                .size(12.0)
                                .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new(self.t.button_skip()).fill(button_color))
                                .clicked()
                            {
                                self.skip_survey();
                            }
                            if ui
                                .add(
                                    egui::Button::new(self.t.button_submit())
                                        .fill(egui::Color32::from_rgb(0x27, 0xae, 0x60)),
                                )
                                .clicked()
                            {
                                self.submit_survey();
                            }
                        });
                    });
            });
    }

    pub fn show_survey_summary_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_survey_summary {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(self.t.survey_summary_title())
                        .size(28.0)
                        .color(text_color)
                        .strong(),
                );
                ui.add_space(20.0);
            });
            egui::ScrollArea::vertical()
                .id_salt("survey_summary_scroll")
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(500.0);
                        if self.survey_data.focus_count == 0 {
                            ui.label(
                                egui::RichText::new(self.t.no_survey_data())
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(0xaa, 0xaa, 0xaa)),
                            );
                        } else {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(self.t.focus_ratings())
                                        .size(16.0)
                                        .strong()
                                        .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        self.t
                                            .avg_focus_today(self.survey_data.average_focus_today),
                                    )
                                    .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        self.t.avg_focus_overall(self.survey_data.average_focus),
                                    )
                                    .color(text_color),
                                );
                            });
                            ui.add_space(25.0);
                            if !self.survey_data.what_helped.is_empty() {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(self.t.how_helped())
                                            .size(16.0)
                                            .strong()
                                            .color(text_color),
                                    );
                                    for item in &self.survey_data.what_helped {
                                        ui.label(
                                            egui::RichText::new(format!("• {}", item))
                                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                                        );
                                    }
                                });
                            }
                            ui.add_space(25.0);
                            if !self.survey_data.what_hurt.is_empty() {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(self.t.how_hurt())
                                            .size(16.0)
                                            .strong()
                                            .color(text_color),
                                    );
                                    for item in &self.survey_data.what_hurt {
                                        ui.label(
                                            egui::RichText::new(format!("• {}", item))
                                                .color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
                                        );
                                    }
                                });
                            }
                        }
                        ui.add_space(30.0);
                        ui.separator();
                        ui.add_space(15.0);
                        if ui_components::rounded_button(
                            ui,
                            &self.t.button_close(),
                            text_color,
                            button_color,
                        )
                        .clicked()
                        {
                            self.show_survey_summary = false;
                        }
                    });
                });
        });
    }

    pub fn show_help_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_help {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(self.t.keyboard_shortcuts_title())
                        .size(28.0)
                        .color(text_color)
                        .strong(),
                );
                ui.add_space(20.0);
            });
            egui::ScrollArea::vertical()
                .id_salt("help_shortcuts_scroll")
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(500.0);
                        let shortcuts = [
                            (
                                self.t.help_timer_title(),
                                vec![
                                    ("Space", self.t.shortcut_start_pause()),
                                    ("R", self.t.shortcut_reset()),
                                ],
                            ),
                            (
                                self.t.help_notes_title(),
                                vec![
                                    ("Ctrl+P", self.t.shortcut_format()),
                                    ("Ctrl+D", self.t.shortcut_bullet()),
                                    ("Tab", self.t.shortcut_indent()),
                                    ("/", self.t.shortcut_slash()),
                                    ("#", self.t.shortcut_hashtag()),
                                ],
                            ),
                            (
                                self.t.help_general_title(),
                                vec![("Ctrl+?", self.t.shortcut_toggle_help())],
                            ),
                        ];
                        for (title, list) in shortcuts {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(title)
                                        .size(16.0)
                                        .strong()
                                        .color(text_color),
                                );
                                for (key, action) in list {
                                    ui.horizontal(|ui| {
                                        ui.add_space(10.0);
                                        ui.label(
                                            egui::RichText::new(format!("{:<15}", key))
                                                .monospace()
                                                .color(egui::Color32::from_rgb(0x88, 0xcc, 0xff)),
                                        );
                                        ui.label(egui::RichText::new(action).color(text_color));
                                    });
                                }
                            });
                            ui.add_space(20.0);
                        }
                        ui.separator();
                        if ui_components::rounded_button(
                            ui,
                            &self.t.button_close(),
                            text_color,
                            button_color,
                        )
                        .clicked()
                        {
                            self.show_help = false;
                        }
                    });
                });
        });
    }
}
