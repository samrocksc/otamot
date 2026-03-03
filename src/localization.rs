use crate::config::Language;
use serde_json::Value;

pub struct T {
    data: Value,
}

impl T {
    pub fn new(language: Language) -> Self {
        let json = match language {
            Language::English => include_str!("../locales/en.json"),
            Language::German => include_str!("../locales/de.json"),
        };
        let data = serde_json::from_str(json).unwrap_or_else(|_| serde_json::json!({}));
        Self { data }
    }

    fn get(&self, key: &str) -> String {
        self.data
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| key.to_string())
    }

    pub fn timer_work(&self) -> String {
        self.get("timer_work")
    }
    pub fn timer_break(&self) -> String {
        self.get("timer_break")
    }
    pub fn settings_title(&self) -> String {
        self.get("settings_title")
    }
    pub fn work_duration(&self) -> String {
        self.get("work_duration")
    }
    pub fn break_duration(&self) -> String {
        self.get("break_duration")
    }
    pub fn notes_directory(&self) -> String {
        self.get("notes_directory")
    }
    pub fn enable_todo(&self) -> String {
        self.get("enable_todo")
    }
    pub fn todo_on(&self) -> String {
        self.get("todo_on")
    }
    pub fn todo_off(&self) -> String {
        self.get("todo_off")
    }

    pub fn enable_notes(&self) -> String {
        self.get("enable_notes")
    }
    pub fn enable_survey(&self) -> String {
        self.get("enable_survey")
    }
    pub fn language_setting(&self) -> String {
        self.get("language_setting")
    }
    pub fn button_save(&self) -> String {
        self.get("button_save")
    }
    pub fn button_cancel(&self) -> String {
        self.get("button_cancel")
    }
    pub fn survey_summary_title(&self) -> String {
        self.get("survey_summary_title")
    }
    pub fn keyboard_shortcuts_title(&self) -> String {
        self.get("keyboard_shortcuts_title")
    }
    pub fn button_close(&self) -> String {
        self.get("button_close")
    }
    pub fn start_button(&self) -> String {
        self.get("start_button")
    }
    pub fn pause_button(&self) -> String {
        self.get("pause_button")
    }
    pub fn reset_button(&self) -> String {
        self.get("reset_button")
    }
    pub fn survey_complete_title(&self) -> String {
        self.get("survey_complete_title")
    }
    pub fn survey_question_focus(&self) -> String {
        self.get("survey_question_focus")
    }

    pub fn survey_rating_label(&self, rating: u32) -> String {
        self.get("survey_rating_label")
            .replace("{}", &rating.to_string())
    }

    pub fn survey_question_helped(&self) -> String {
        self.get("survey_question_helped")
    }
    pub fn survey_question_hurt(&self) -> String {
        self.get("survey_question_hurt")
    }
    pub fn button_skip(&self) -> String {
        self.get("button_skip")
    }
    pub fn button_submit(&self) -> String {
        self.get("button_submit")
    }

    pub fn avg_focus_today(&self, avg: f64) -> String {
        self.get("avg_focus_today")
            .replace("{:.1}", &format!("{:.1}", avg))
    }

    pub fn avg_focus_overall(&self, avg: f64) -> String {
        self.get("avg_focus_overall")
            .replace("{:.1}", &format!("{:.1}", avg))
    }

    pub fn notes_on(&self) -> String {
        self.get("notes_on")
    }
    pub fn notes_off(&self) -> String {
        self.get("notes_off")
    }
    pub fn help_button(&self) -> String {
        self.get("help_button")
    }
    pub fn todo_title(&self) -> String {
        self.get("todo_title")
    }
    pub fn todo_hint(&self) -> String {
        self.get("todo_hint")
    }
    pub fn autocomplete_commands(&self) -> String {
        self.get("autocomplete_commands")
    }
    pub fn autocomplete_hashtags(&self) -> String {
        self.get("autocomplete_hashtags")
    }
    pub fn toggle_preview_hover(&self) -> String {
        self.get("toggle_preview_hover")
    }
    pub fn surveys_on(&self) -> String {
        self.get("surveys_on")
    }
    pub fn surveys_off(&self) -> String {
        self.get("surveys_off")
    }
    pub fn reset_survey_data_btn(&self) -> String {
        self.get("reset_survey_data")
    }
    pub fn edit_tab(&self) -> String {
        self.get("edit_tab")
    }
    pub fn preview_tab(&self) -> String {
        self.get("preview_tab")
    }
    pub fn save_notes_btn(&self) -> String {
        self.get("save_notes")
    }
    pub fn sessions_completed_label(&self, count: u32) -> String {
        self.get("sessions_completed")
            .replace("{}", &count.to_string())
    }
    pub fn how_helped(&self) -> String {
        self.get("how_helped")
    }
    pub fn how_hurt(&self) -> String {
        self.get("how_hurt")
    }
    pub fn no_survey_data(&self) -> String {
        self.get("no_survey_data")
    }
    pub fn complete_session_prompt(&self) -> String {
        self.get("complete_session_prompt")
    }

    pub fn focus_ratings(&self) -> String {
        self.get("focus_ratings")
    }
    pub fn todays_average(&self) -> String {
        self.get("todays_average")
    }
    pub fn overall_average(&self) -> String {
        self.get("overall_average")
    }
    pub fn total_sessions(&self) -> String {
        self.get("total_sessions")
    }
    pub fn survey_hint(&self) -> String {
        self.get("survey_hint")
    }
    pub fn helped_hint(&self) -> String {
        self.get("helped_hint")
    }
    pub fn hurt_hint(&self) -> String {
        self.get("hurt_hint")
    }
    pub fn lang_en(&self) -> String {
        self.get("lang_en")
    }
    pub fn lang_de(&self) -> String {
        self.get("lang_de")
    }

    pub fn help_timer_title(&self) -> String {
        self.get("help_timer_title")
    }
    pub fn help_notes_title(&self) -> String {
        self.get("help_notes_title")
    }
    pub fn help_general_title(&self) -> String {
        self.get("help_general_title")
    }
    pub fn shortcut_start_pause(&self) -> String {
        self.get("shortcut_start_pause")
    }
    pub fn shortcut_reset(&self) -> String {
        self.get("shortcut_reset")
    }
    pub fn shortcut_format(&self) -> String {
        self.get("shortcut_format")
    }
    pub fn shortcut_bullet(&self) -> String {
        self.get("shortcut_bullet")
    }
    pub fn shortcut_indent(&self) -> String {
        self.get("shortcut_indent")
    }
    pub fn shortcut_slash(&self) -> String {
        self.get("shortcut_slash")
    }
    pub fn shortcut_hashtag(&self) -> String {
        self.get("shortcut_hashtag")
    }
    pub fn shortcut_navigate(&self) -> String {
        self.get("shortcut_navigate")
    }
    pub fn shortcut_select(&self) -> String {
        self.get("shortcut_select")
    }
    pub fn shortcut_close_dropdown(&self) -> String {
        self.get("shortcut_close_dropdown")
    }
    pub fn shortcut_toggle_help(&self) -> String {
        self.get("shortcut_toggle_help")
    }
    pub fn shortcut_settings(&self) -> String {
        self.get("shortcut_settings")
    }

    pub fn duration_label(&self, duration: u32) -> String {
        self.get("duration_min")
            .replace("{}", &duration.to_string())
    }
    pub fn settings_btn(&self) -> String {
        self.get("settings_btn")
    }

    pub fn todo_status(&self, pending: usize, done: usize) -> String {
        self.get("todo_status")
            .replacen("{}", &pending.to_string(), 1)
            .replacen("{}", &done.to_string(), 1)
    }
    pub fn todo_clear_completed_btn(&self) -> String {
        self.get("todo_clear_completed_btn")
    }
}
