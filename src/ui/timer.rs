use crate::localization::T;
use crate::timer::TimerMode;
use crate::ui_components;
use eframe::egui;

pub struct TimerView<'a> {
    pub is_running: bool,
    pub mode: TimerMode,
    pub time_formatted: String,
    pub sessions_completed: u32,
    pub notes_enabled: bool,
    pub has_notes_content: bool,
    pub call_mode_active: bool,
    pub call_time_formatted: Option<String>,
    pub t: &'a T,
}

pub enum TimerAction {
    Toggle,
    Reset,
    Skip,
    SaveNotes,
    StartCall,
    EndCall,
}

impl<'a> TimerView<'a> {
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        text_color: egui::Color32,
        button_color: egui::Color32,
        work_color: egui::Color32,
        break_color: egui::Color32,
        call_color: egui::Color32,
    ) -> Option<TimerAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.add_space(10.0);

            // Timer Display - show call timer if in call mode
            let display_time = if self.call_mode_active {
                self.call_time_formatted.as_deref().unwrap_or("00:00")
            } else {
                &self.time_formatted
            };

            ui.label(
                egui::RichText::new(display_time)
                    .size(48.0)
                    .color(text_color),
            );

            ui.add_space(20.0);

            // Mode label
            let (mode_label, mode_color) = if self.call_mode_active {
                (self.t.timer_call(), call_color)
            } else {
                match self.mode {
                    TimerMode::Work => (self.t.timer_work(), work_color),
                    TimerMode::Break => (self.t.timer_break(), break_color),
                }
            };
            ui.label(egui::RichText::new(mode_label).size(20.0).color(mode_color));

            ui.add_space(30.0);

            // Timer Controls (hidden during call mode)
            if !self.call_mode_active {
                let label = if self.is_running {
                    self.t.pause_button()
                } else {
                    self.t.start_button()
                };
                if ui_components::rounded_button(ui, &label, text_color, button_color).clicked() {
                    action = Some(TimerAction::Toggle);
                }
                ui.add_space(8.0);
                if ui_components::rounded_button(
                    ui,
                    &self.t.reset_button(),
                    text_color,
                    button_color,
                )
                .clicked()
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
            }

            // Call Mode Button
            ui.add_space(8.0);
            let call_button_color = if self.call_mode_active {
                egui::Color32::from_rgb(0xe7, 0x4c, 0x3c) // Red for end call
            } else {
                egui::Color32::from_rgb(0x27, 0xae, 0x60) // Green for start call
            };

            let call_label = if self.call_mode_active {
                self.t.end_call_button()
            } else {
                self.t.start_call_button()
            };

            if ui_components::rounded_button(ui, &call_label, text_color, call_button_color)
                .clicked()
            {
                action = if self.call_mode_active {
                    Some(TimerAction::EndCall)
                } else {
                    Some(TimerAction::StartCall)
                };
            }

            if self.notes_enabled && self.has_notes_content && !self.call_mode_active {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui_components::small_rounded_button(
                        ui,
                        &self.t.save_notes_btn(),
                        text_color,
                        egui::Color32::from_rgb(0x27, 0xae, 0x60),
                    )
                    .clicked()
                    {
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
