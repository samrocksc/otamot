use eframe::egui;
use crate::ui_components;
use crate::localization::T;
use crate::timer::TimerMode;

pub struct TimerView<'a> {
    pub is_running: bool,
    pub mode: TimerMode,
    pub time_formatted: String,
    pub sessions_completed: u32,
    pub notes_enabled: bool,
    pub has_notes_content: bool,
    pub t: &'a T,
}

pub enum TimerAction {
    Toggle,
    Reset,
    Skip,
    SaveNotes,
}

impl<'a> TimerView<'a> {
    pub fn show(&self, ui: &mut egui::Ui, text_color: egui::Color32, button_color: egui::Color32, work_color: egui::Color32, break_color: egui::Color32) -> Option<TimerAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            
            // Timer Display
            ui.label(
                egui::RichText::new(&self.time_formatted)
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
            if ui_components::rounded_button(ui, &label, text_color, button_color).clicked() {
                action = Some(TimerAction::Toggle);
            }
            ui.add_space(8.0);
            if ui_components::rounded_button(ui, &self.t.reset_button(), text_color, button_color).clicked()
            {
                action = Some(TimerAction::Reset);
            }
            ui.add_space(8.0);
            if ui_components::rounded_button(
                ui,
                &self.t.button_skip().to_uppercase(),
                text_color,
                button_color,
            )
            .clicked()
            {
                action = Some(TimerAction::Skip);
            }

            if self.notes_enabled && self.has_notes_content {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui_components::small_rounded_button(ui, &self.t.save_notes_btn(), text_color, egui::Color32::from_rgb(0x27, 0xae, 0x60)).clicked() {
                        action = Some(TimerAction::SaveNotes);
                    }
                });
            }
        });
        ui.add_space(10.0);
        ui.separator();
        action
    }
}
