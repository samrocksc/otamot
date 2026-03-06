use chrono::Local;
use eframe::egui;
use std::time::{Duration, Instant};

// otamot library imports
use otamot::bell::Bell;
use otamot::commands::CommandManager;
use otamot::config::{Config, Language, NotesView, Theme};
use otamot::easy_mark::editor::EasyMarkEditor;
use otamot::hashtags::HashtagLibrary;
use otamot::kanban::KanbanBoard;
use otamot::localization::T;
use otamot::markdown::{format_markdown, insert_date_bullet};
use otamot::notes;
use otamot::survey::SurveyData;
use otamot::timer::TimerMode;
use otamot::todo::TodoList;
use otamot::ui_components;
use otamot::ui::{sidebar::Sidebar, timer::{TimerView, TimerAction}, notes::{NotesEditor, NotesAction}};

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
    temp_todo_file: String,
    temp_language: Language,
    temp_theme: Theme,

    // Localization helper
    t: T,

    // Notes state
    notes_enabled: bool,
    notes_content: String,
    project_content: String,
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
    show_about: bool,

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

    // Tray Icon
    #[cfg(not(target_arch = "wasm32"))]
    tray_icon: Option<tray_icon::TrayIcon>,
    #[cfg(not(target_arch = "wasm32"))]
    tray_menu_ids: std::collections::HashMap<String, tray_icon::menu::MenuId>,
}

impl PomodoroApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let remaining_seconds = config.work_duration * 60;
        let survey_data = SurveyData::load();
        let bell = Bell::default();

        let mut app = Self {
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
            temp_todo_file: config.todo_file.clone(),
            temp_language: config.language,
            temp_theme: config.theme.clone(),
            t: T::new(config.language),
            notes_enabled: config.notes_enabled,
            notes_content: notes::load_draft(&config.notes_directory),
            project_content: std::fs::read_to_string(&config.todo_file).unwrap_or_default(),
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
            show_about: false,
            todo_list: TodoList::load_from_path(&config.todo_file),
            todo_input: String::new(),
            todo_enabled: config.todo_enabled,
            editor: EasyMarkEditor::default(),
            kanban_enabled: config.kanban_enabled,
            kanban_board: KanbanBoard::load_from_path(&config.todo_file),
            kanban_input: String::new(),
            sidebar_collapsed: config.sidebar_collapsed,

            #[cfg(not(target_arch = "wasm32"))]
            tray_icon: None,
            #[cfg(not(target_arch = "wasm32"))]
            tray_menu_ids: std::collections::HashMap::new(),

            sessions_completed: survey_data.sessions_completed,
            show_survey: false,
            show_survey_summary: false,
            survey_data,
            survey_focus_rating: 5,
            survey_what_helped: String::new(),
            survey_what_hurt: String::new(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        app.setup_tray_icon();

        app
    }

    fn get_notes_byte_pos(&self) -> usize {
        self.notes_content
            .char_indices()
            .nth(self.notes_cursor_pos)
            .map(|(idx, _)| idx)
            .unwrap_or(self.notes_content.len())
    }

    fn format_time(&self) -> String {
        let minutes = self.remaining_seconds / 60;
        let seconds = self.remaining_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    fn render_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        _text_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        text_dim_color: egui::Color32,
    ) {
        let mut sidebar = Sidebar {
            sidebar_collapsed: &mut self.sidebar_collapsed,
            notes_enabled: &mut self.notes_enabled,
            todo_enabled: &mut self.todo_enabled,
            kanban_enabled: &mut self.kanban_enabled,
            show_settings: &mut self.show_settings,
            show_survey_summary: &mut self.show_survey_summary,
            show_help: &mut self.show_help,
            sessions_completed: self.sessions_completed,
            t: &self.t,
            config: &mut self.config,
        };
        sidebar.show(ui, button_color, button_text_color, text_dim_color);
        
        // Handle side effects that couldn't be moved easily or need local state
        if self.show_settings {
            self.temp_work_duration = self.config.work_duration;
            self.temp_break_duration = self.config.break_duration;
            self.temp_notes_directory = self.config.notes_directory.clone();
            self.temp_theme = self.config.theme.clone();
        }
    }

    fn render_timer(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
    ) {
        let timer_view = TimerView {
            is_running: self.is_running,
            mode: self.mode,
            time_formatted: self.format_time(),
            sessions_completed: self.sessions_completed,
            notes_enabled: self.notes_enabled,
            has_notes_content: !self.notes_content.is_empty(),
            t: &self.t,
        };

        if let Some(action) = timer_view.show(ui, text_color, button_color, work_color, break_color) {
            match action {
                TimerAction::Toggle => self.toggle_timer(),
                TimerAction::Reset => self.reset_timer(),
                TimerAction::Skip => self.skip_to_break(),
                TimerAction::SaveNotes => self.save_notes(),
            }
        }
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

    pub fn send_notification(&self, title: &str, body: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = notify_rust::Notification::new()
                .summary(title)
                .body(body)
                .show();
        }
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

    #[cfg(not(target_arch = "wasm32"))]
    fn setup_tray_icon(&mut self) {
        use tray_icon::{
            menu::{Menu, MenuItem, PredefinedMenuItem},
            TrayIconBuilder,
        };

        let tray_menu = Menu::new();
        let start_pause_item = MenuItem::new("Start/Pause", true, None);
        let reset_item = MenuItem::new("Reset", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        self.tray_menu_ids
            .insert("start_pause".to_string(), start_pause_item.id().clone());
        self.tray_menu_ids
            .insert("reset".to_string(), reset_item.id().clone());
        self.tray_menu_ids
            .insert("quit".to_string(), quit_item.id().clone());

        let _ = tray_menu.append_items(&[
            &start_pause_item,
            &reset_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ]);

        // Load real icon from assets
        let icon = (|| {
            let icon_bytes = include_bytes!("../assets/icon.png");
            let img = image::load_from_memory(icon_bytes).ok()?;
            let img = img.resize(32, 32, image::imageops::FilterType::Lanczos3);
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).ok()
        })()
        .unwrap_or_else(|| {
            // Fallback to a solid red pixel if loading fails, so it's not transparent
            tray_icon::Icon::from_rgba(vec![220, 20, 60, 255], 1, 1).unwrap()
        });

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Otamot")
            .with_icon(icon)
            .build()
            .unwrap();

        self.tray_icon = Some(tray_icon);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle_tray_events(&mut self) {
        use tray_icon::menu::MenuEvent;
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.tray_menu_ids.get("start_pause") {
                self.toggle_timer();
            } else if Some(&event.id) == self.tray_menu_ids.get("reset") {
                self.reset_timer();
            } else if Some(&event.id) == self.tray_menu_ids.get("quit") {
                std::process::exit(0);
            }
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

                    let (title, body) = match self.mode {
                        TimerMode::Work => ("Work Session Complete", "Time for a break!"),
                        TimerMode::Break => ("Break Over", "Back to work!"),
                    };
                    self.send_notification(title, body);

                    let previous_mode = self.mode;
                    self.mode = match self.mode {
                        TimerMode::Work => {
                            self.session_end = Some(Local::now());
                            if self.notes_enabled && !self.notes_content.is_empty() {
                                self.save_notes();
                            }
                            self.sessions_completed += 1;
                            self.survey_data.sessions_completed = self.sessions_completed;
                            let _ = self.survey_data.save();
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

        let filename = notes::generate_filename(start_time, end_time);
        let filepath = notes_dir.join(&filename);

        // Generate frontmatter
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

        // Filter out auto-generated project sections if they crept into the buffer
        let mut notes_to_save = self.notes_content.clone();
        if let Some(pos) = notes_to_save.find("# TODO") {
            notes_to_save.truncate(pos);
        }
        if let Some(pos) = notes_to_save.find("# Kanban") {
            notes_to_save.truncate(pos);
        }
        
        let notes_to_save = notes_to_save.trim().to_string();

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

        let content = format!("{}{}", frontmatter, notes_to_save);
        if let Err(e) = std::fs::write(&filepath, &content) {
            eprintln!("Failed to write note file: {}", e);
        } else {
            // Clean up session notes but leave the project state if it was there
            // Note: we usually cleared everything, let's go back to that clean slate for next session
            self.notes_content.clear();
            let _ = notes::clear_draft(&self.config.notes_directory);
        }
    }

    fn save_project_file(&self) {
        let path = std::path::PathBuf::from(&self.config.todo_file);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        let content = if self.notes_view == NotesView::Project {
            self.project_content.clone()
        } else {
            let mut c = String::new();
            c.push_str(&self.todo_list.to_markdown());
            c.push_str("\n\n");
            c.push_str(&self.kanban_board.to_markdown());
            c
        };
        
        let _ = std::fs::write(path, content);
    }

    fn save_settings(&mut self) {
        self.config.work_duration = self.temp_work_duration;
        self.config.break_duration = self.temp_break_duration;
        self.config.notes_directory = self.temp_notes_directory.clone();
        self.config.language = self.temp_language;
        self.config.theme = self.temp_theme.clone();
        self.config.todo_enabled = self.todo_enabled;
        self.config.todo_file = self.temp_todo_file.clone();
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
}

// --- App Trait Implementation ---

impl eframe::App for PomodoroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.handle_tray_events();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(self.t.menu_help(), |ui| {
                    if ui.button(self.t.help_button()).clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                    if ui.button(self.t.menu_about()).clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Essential state updates
        self.tick();
        
        // Auto-save notes draft if they've changed
        if self.notes_enabled && !self.notes_content.is_empty() {
             let _ = notes::save_draft(&self.config.notes_directory, &self.notes_content);
        }

        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Handle early keyboard input for the notes editor (before UI renders)
        if self.notes_enabled && self.notes_view == NotesView::Edit {
            let is_focused = ctx.memory(|mem| mem.has_focus(egui::Id::new("notes_text_input")));

            if is_focused && self.dropdown_visible {
                if !self.dropdown_items.is_empty() {
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
                } else if self.dropdown_type == DropdownType::Hashtag {
                    // Handle Enter for new hashtag (not in library yet)
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let byte_pos = self.get_notes_byte_pos();
                        if let Some((_pos, tag)) =
                            HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, byte_pos)
                        {
                            if !tag.is_empty() {
                                self.apply_dropdown_selection(tag);
                                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                            }
                        }
                    }
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
                    let byte_pos = self.get_notes_byte_pos();

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
                        let line_content = &self.notes_content[line_start..byte_pos];
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

        // Check for early keyboard input for Enter in dropdown
        if self.notes_enabled && self.notes_view == NotesView::Edit && self.dropdown_visible && !self.dropdown_items.is_empty() {
             if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                 let item = self.dropdown_items[self.dropdown_selected].clone();
                 self.apply_dropdown_selection(item);
                 ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
             }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Comma) && i.modifiers.command) {
            self.reset_timer();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Period) && i.modifiers.command) {
            self.toggle_timer();
        }

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
                NotesView::Project => {
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
                let byte_pos = self.get_notes_byte_pos();

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
            let byte_pos = self.get_notes_byte_pos();
            if !self.dropdown_visible {
                if let Some((pos, cmd)) =
                    CommandManager::find_command_at_cursor(&self.notes_content, byte_pos)
                {
                    self.dropdown_visible = true;
                    self.dropdown_type = DropdownType::Command;
                    self.dropdown_start_pos = pos;
                    self.dropdown_items = self.command_manager.search_commands(&cmd);
                    self.dropdown_selected = 0;
                } else if let Some((pos, tag)) =
                    HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, byte_pos)
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
                            CommandManager::find_command_at_cursor(&self.notes_content, byte_pos)
                        {
                            self.dropdown_start_pos = pos;
                            self.dropdown_items = self.command_manager.search_commands(&cmd);
                        } else {
                            self.dropdown_visible = false;
                        }
                    }
                    DropdownType::Hashtag => {
                        if let Some((pos, tag)) =
                            HashtagLibrary::find_hashtag_at_cursor(&self.notes_content, byte_pos)
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
        let theme = &self.config.theme;
        let text_color = egui::Color32::from_rgb(theme.text.r, theme.text.g, theme.text.b);
        let text_dim_color = egui::Color32::from_rgb(theme.text_dim.r, theme.text_dim.g, theme.text_dim.b);
        let text_highlight_color = egui::Color32::from_rgb(theme.text_highlight.r, theme.text_highlight.g, theme.text_highlight.b);
        let work_color = egui::Color32::from_rgb(theme.work.r, theme.work.g, theme.work.b);
        let break_color = egui::Color32::from_rgb(theme.b_break.r, theme.b_break.g, theme.b_break.b);
        let button_color = egui::Color32::from_rgb(theme.button.r, theme.button.g, theme.button.b);
        let bg_color = egui::Color32::from_rgb(theme.bg.r, theme.bg.g, theme.bg.b);
        let tab_active_color = egui::Color32::from_rgb(theme.tab_active.r, theme.tab_active.g, theme.tab_active.b);
        let tab_inactive_color = egui::Color32::from_rgb(theme.tab_inactive.r, theme.tab_inactive.g, theme.tab_inactive.b);

        let theme_visuals = if theme.dark_mode {
            egui::Visuals::dark()
        } else {
            let mut light = egui::Visuals::light();
            light.widgets.inactive.bg_fill = egui::Color32::WHITE;
            light.widgets.hovered.bg_fill = egui::Color32::from_gray(240);
            light.widgets.active.bg_fill = egui::Color32::from_gray(230);
            // Light grey border for text boxes in Monokai light
            light.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(200));
            light
        };

        ctx.set_visuals(egui::Visuals {
            window_fill: bg_color,
            panel_fill: bg_color,
            override_text_color: Some(text_color),
            hyperlink_color: tab_active_color,
            ..theme_visuals
        });

        let button_text_color = egui::Color32::from_rgb(theme.button_text.r, theme.button_text.g, theme.button_text.b);

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
                                        button_text_color,
                                        text_dim_color,
                                    );
                                });
                        });
                    });

                    ui.separator();

                    // Side Pillar 2: Timer + Notes area and/or TODOs
                    ui.allocate_ui(egui::vec2(right_width, full_height), |ui| {
                        ui.style_mut().spacing.item_spacing.x = 10.0; // Internal spacing
                        
                        ui.vertical(|ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("right_pillar_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    // Wrap the entire right column in a frame to give 
                                    // some consistent right-side padding/breathing room
                                    egui::Frame::none()
                                        .inner_margin(egui::Margin {
                                            left: 0.0,
                                            right: 20.0,
                                            top: 0.0,
                                            bottom: 10.0,
                                        })
                                        .show(ui, |ui| {
                        self.render_right_column(
                                                ctx,
                                                ui,
                                                text_color,
                                                text_dim_color,
                                                text_highlight_color,
                                                tab_active_color,
                                                tab_inactive_color,
                                                button_color,
                                                button_text_color,
                                                work_color,
                                                break_color,
                                                bg_color,
                                            );
                                        });
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
                                    button_text_color,
                                    work_color,
                                    break_color,
                                    text_dim_color,
                                );
                            });
                        });
                });
            }
        });

        // Full-screen / Modal windows
        self.show_settings_dialog(ctx, text_color, text_dim_color, button_color, button_text_color, tab_active_color);
        self.show_help_dialog(ctx, text_color, text_dim_color, button_color);
        self.show_about_dialog(ctx, text_color, text_dim_color, button_color);
        self.show_survey_dialog(ctx, text_color, text_dim_color, button_color, button_text_color, tab_active_color);
        self.show_survey_summary_dialog(ctx, text_color, text_dim_color, button_color);
    }
}

// --- Extended Methods Implementation ---

impl PomodoroApp {
    pub fn apply_dropdown_selection(&mut self, selected_item: String) {
        match self.dropdown_type {
            DropdownType::Command => {
                if let Some(replacement) = self.command_manager.execute(&selected_item) {
                    let cursor_pos = self.get_notes_byte_pos();
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
                let cursor_pos = self.get_notes_byte_pos();
                self.notes_content = HashtagLibrary::insert_hashtag(
                    &self.notes_content,
                    cursor_pos,
                    self.dropdown_start_pos,
                    &selected_item,
                );
                self.requested_cursor_pos =
                    Some(self.dropdown_start_pos + selected_item.chars().count() + 2);
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
        _text_dim_color: egui::Color32,
        text_highlight_color: egui::Color32,
        active_color: egui::Color32,
        inactive_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
        bg_color: egui::Color32,
    ) {
        // Render Timer at the top of the right column
        self.render_timer(ui, text_color, button_color, work_color, break_color);

        // Render notes section if enabled
        if self.notes_enabled {
            let mut editor = NotesEditor {
                view: &mut self.notes_view,
                content: &mut self.notes_content,
                project_content: &mut self.project_content,
                focus_input: &mut self.focus_notes_input,
                notes_cursor_pos: &mut self.notes_cursor_pos,
                requested_cursor_pos: &mut self.requested_cursor_pos,
                editor: &mut self.editor,
                t: &self.t,
            };

            let todo_file = self.config.todo_file.clone();
            let response = editor.show(
                ctx,
                ui,
                text_color,
                active_color,
                inactive_color,
                text_highlight_color,
                bg_color,
                &todo_file,
            );
            
            if let Some(action) = response.action {
                match action {
                    NotesAction::NotesChanged => {
                        let _ = notes::save_draft(&self.config.notes_directory, &self.notes_content);
                    }
                    NotesAction::SaveNotes => {
                        self.save_notes();
                    }
                }
            }

            if let Some(output) = response.output {
                self.render_dropdown(ui, &output);
            }
        }

        // Render TODO section if enabled
        if self.todo_enabled {
            if self.notes_enabled {
                ui.add_space(30.0);
            }
            egui::Frame::group(ui.style())
                .fill(bg_color)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(15.0))
                .show(ui, |ui| {
                    if ui_components::render_todo_panel(
                        ui,
                        &mut self.todo_list,
                        &mut self.todo_input,
                        &mut self.kanban_board,
                        &self.t,
                        text_color,
                        button_color,
                        button_text_color,
                    ) {
                        // Update global project file only
                        self.save_project_file();
                    }
                });
        }

        // Render Kanban section if enabled
        if self.kanban_enabled {
            if self.notes_enabled || self.todo_enabled {
                ui.add_space(30.0);
            }
            egui::Frame::group(ui.style())
                .fill(bg_color)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(15.0))
                .show(ui, |ui| {
                    if ui_components::render_kanban_board(
                        ui,
                        &mut self.kanban_board,
                        &mut self.kanban_input,
                        &mut self.todo_list,
                        &self.t,
                        text_color,
                        bg_color,
                        inactive_color,
                        button_text_color,
                    ) {
                        // Update global project file only
                        self.save_project_file();
                    }
                });
        }
    }

    fn render_pure_timer_layout(
        &mut self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
        _text_dim_color: egui::Color32,
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
                if ui_components::rounded_button(ui, &btn, button_text_color, button_color).clicked() {
                    self.toggle_timer();
                }
                ui.add_space(10.0);
                if ui_components::rounded_button(ui, &self.t.reset_button(), button_text_color, button_color)
                    .clicked()
                {
                    self.reset_timer();
                }
                ui.add_space(10.0);
                if ui_components::rounded_button(
                    ui,
                    &self.t.button_skip_upper(),
                    button_text_color,
                    button_color,
                )
                .clicked()
                {
                    self.skip_to_break();
                }
            });
            ui.add_space(40.0);
            if ui_components::rounded_button(ui, &self.t.settings_btn(), button_text_color, button_color).clicked() {
                self.temp_work_duration = self.config.work_duration;
                self.temp_break_duration = self.config.break_duration;
                self.temp_notes_directory = self.config.notes_directory.clone();
                self.temp_theme = self.config.theme.clone();
                self.show_settings = true;
            }
            if ui_components::rounded_button(ui, &self.t.survey_summary_title(), button_text_color, button_color).clicked() {
                self.show_survey_summary = true;
            }
            ui.add_space(10.0);
            let notes_label = if self.notes_enabled { self.t.notes_on() } else { self.t.notes_off() };
            if ui_components::rounded_button(ui, &notes_label, button_text_color, button_color).clicked() {
                self.notes_enabled = !self.notes_enabled;
                self.config.notes_enabled = self.notes_enabled;
                let _ = self.config.save();
            }
            ui.add_space(10.0);
            let todo_label = if self.todo_enabled { self.t.todo_on() } else { self.t.todo_off() };
            if ui_components::rounded_button(ui, &todo_label, button_text_color, button_color).clicked() {
                self.todo_enabled = !self.todo_enabled;
                self.config.todo_enabled = self.todo_enabled;
                let _ = self.config.save();
            }
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(self.t.sessions_completed_label(self.sessions_completed))
                    .size(12.0)
                    .color(_text_dim_color),
            );
            if ui_components::rounded_button(ui, &self.t.help_button(), button_text_color, button_color).clicked() {
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
            // Ensure we handle mouse interaction correctly for standard egui Areas
            if ui.input(|i| i.pointer.any_pressed()) && self.dropdown_visible {
                 // We let the clicked() check above handle selection, 
                 // but we need to make sure the Area doesn't block interaction 
                 // if clicked outside (though Area usually doesn't unless modal)
            }
        }
    }

    pub fn show_settings_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        text_dim_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        tab_active_color: egui::Color32,
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
                                .color(text_dim_color),
                            );
                            ui.add(egui::Slider::new(&mut self.temp_work_duration, 1..=60).show_value(false));
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
                                .color(text_dim_color),
                            );
                            ui.add(egui::Slider::new(&mut self.temp_break_duration, 1..=30).show_value(false));
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.notes_directory())
                                    .size(16.0)
                                    .color(text_dim_color),
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
                            ui.label(
                                egui::RichText::new("TODO/Kanban File")
                                    .size(16.0)
                                    .color(text_dim_color),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.temp_todo_file)
                                    .desired_width(350.0),
                            );
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            let survey_label = if self.config.survey_enabled { self.t.surveys_on() } else { self.t.surveys_off() };
                            if ui_components::rounded_button(ui, &survey_label, button_text_color, button_color).clicked() {
                                self.config.survey_enabled = !self.config.survey_enabled;
                                let _ = self.config.save();
                            }
                        });
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            if ui_components::rounded_button(ui, &self.t.reset_survey_data_btn(), button_text_color, button_color).clicked() {
                                self.reset_survey_data();
                            }
                        });
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.bell_tune_label())
                                    .size(18.0)
                                    .color(text_dim_color),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(60.0);
                            ui.radio_value(&mut self.config.bell_tune, otamot::config::BellTune::Default, self.t.tune_default());
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(60.0);
                            ui.radio_value(&mut self.config.bell_tune, otamot::config::BellTune::LaCukaracha, self.t.tune_cukaracha());
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(60.0);
                            ui.radio_value(&mut self.config.bell_tune, otamot::config::BellTune::IceCreamTruck, self.t.tune_icecream());
                        });

                        // Update bell config immediately when changed in settings
                        self.bell.set_config(otamot::bell::BellConfig {
                            enabled: self.bell.config().enabled,
                            volume: self.bell.config().volume,
                            duration_ms: self.bell.config().duration_ms,
                            frequency: self.bell.config().frequency,
                            tune: self.config.bell_tune,
                        });

                        ui.add_space(20.0);
                        ui.set_max_width(500.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new("Theme")
                                    .size(18.0)
                                    .color(text_dim_color),
                            );
                            egui::ComboBox::from_id_salt("theme_selector")
                                .selected_text(&self.temp_theme.name)
                                .show_ui(ui, |ui| {
                                    let themes: [Theme; 4] = [
                                        Theme::robotic_lime(),
                                        Theme::monokai_dark(),
                                        Theme::monokai_light(),
                                        Theme::dark(),
                                    ];
                                    for theme in themes {
                                        ui.selectable_value(
                                            &mut self.temp_theme,
                                            theme.clone(),
                                            &theme.name,
                                        );
                                    }
                                });
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(self.t.language_setting())
                                    .size(18.0)
                                    .color(text_dim_color),
                            );
                            egui::ComboBox::from_id_salt("language_selector")
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
                            if ui_components::rounded_button(ui, &self.t.button_cancel(), button_text_color, button_color).clicked() {
                                self.show_settings = false;
                                self.temp_work_duration = self.config.work_duration;
                                self.temp_break_duration = self.config.break_duration;
                                self.temp_notes_directory = self.config.notes_directory.clone();
                                self.temp_language = self.config.language;
                            }
                            ui.add_space(15.0);
                            if ui_components::rounded_button(ui, &self.t.button_save(), button_text_color, tab_active_color).clicked() {
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
        text_dim_color: egui::Color32,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        tab_active_color: egui::Color32,
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
                                .color(text_dim_color),
                            );
                            if ui_components::icon_button(ui, "-", text_color, button_color).clicked() {
                                self.survey_focus_rating =
                                    self.survey_focus_rating.saturating_sub(1).max(1);
                            }
                            if ui_components::icon_button(ui, "+", text_color, button_color).clicked() {
                                self.survey_focus_rating =
                                    self.survey_focus_rating.saturating_add(1).min(10);
                            }
                        });
                        ui.add_space(15.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_helped())
                                .color(text_dim_color),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.survey_what_helped)
                                .desired_width(350.0)
                                .hint_text(self.t.helped_hint()),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(self.t.survey_question_hurt())
                                .color(text_dim_color),
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
                                .color(text_dim_color),
                            );
                            ui.label(
                                egui::RichText::new(
                                    self.t.avg_focus_overall(self.survey_data.average_focus),
                                )
                                .size(12.0)
                                .color(text_dim_color),
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui_components::rounded_button(ui, &self.t.button_skip(), button_text_color, button_color).clicked() {
                                self.skip_survey();
                            }
                            if ui_components::rounded_button(ui, &self.t.button_submit(), button_text_color, tab_active_color).clicked() {
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
        text_dim_color: egui::Color32,
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
                                    .color(text_dim_color),
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
                                                .color(text_dim_color),
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
                                                .color(text_dim_color),
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
        _text_dim_color: egui::Color32,
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
                                    ("Cmd/Ctrl + .", self.t.shortcut_start_pause()),
                                    ("Cmd/Ctrl + ,", self.t.shortcut_reset()),
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
                                vec![
                                    ("Ctrl+?", self.t.shortcut_toggle_help()),
                                    ("Cmd/Ctrl + Shift + /", self.t.shortcut_toggle_help()),
                                ],
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

    pub fn show_about_dialog(
        &mut self,
        ctx: &egui::Context,
        text_color: egui::Color32,
        _text_dim_color: egui::Color32,
        button_color: egui::Color32,
    ) {
        if !self.show_about {
            return;
        }

        let version = env!("CARGO_PKG_VERSION");
        let release_date = "2026-03-05"; // For now, we manually set this

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new(self.t.about_title())
                        .size(32.0)
                        .color(text_color)
                        .strong(),
                );
                ui.add_space(20.0);
                ui.label(egui::RichText::new(self.t.about_description()).color(text_color));
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(self.t.about_version(version))
                        .color(text_color),
                );
                ui.label(
                    egui::RichText::new(self.t.about_release_date(release_date))
                        .color(text_color),
                );
                ui.add_space(40.0);

                if ui_components::rounded_button(
                    ui,
                    &self.t.button_close(),
                    text_color,
                    button_color,
                )
                .clicked()
                {
                    self.show_about = false;
                }
            });
        });
    }
}
