use crate::localization::T;
use crate::ui_components;
use eframe::egui;

pub struct Sidebar<'a> {
    pub sidebar_collapsed: &'a mut bool,
    pub notes_enabled: &'a mut bool,
    pub todo_enabled: &'a mut bool,
    pub kanban_enabled: &'a mut bool,
    pub show_settings: &'a mut bool,
    pub show_survey_summary: &'a mut bool,
    pub show_help: &'a mut bool,
    pub sessions_completed: u32,
    pub t: &'a T,
    pub config: &'a mut crate::config::Config,
}

impl<'a> Sidebar<'a> {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        button_color: egui::Color32,
        button_text_color: egui::Color32,
        text_dim_color: egui::Color32,
    ) {
        ui.vertical(|ui| {
            // Consistent spacing at top
            ui.add_space(8.0);

            // Sidebar Toggle Arrow (icon button)
            let arrow = if *self.sidebar_collapsed {
                "▶"
            } else {
                "◀"
            };
            if ui_components::sidebar_icon_button(ui, arrow, button_text_color, button_color)
                .clicked()
            {
                *self.sidebar_collapsed = !*self.sidebar_collapsed;
                self.config.sidebar_collapsed = *self.sidebar_collapsed;
                let _ = self.config.save();
            }

            if !*self.sidebar_collapsed {
                // Section: Main Navigation
                ui.add_space(12.0);

                // Settings button
                if ui_components::sidebar_button(
                    ui,
                    &self.t.settings_btn(),
                    button_text_color,
                    button_color,
                )
                .clicked()
                {
                    *self.show_settings = true;
                }

                // Survey summary button
                ui.add_space(8.0);
                if ui_components::sidebar_button(
                    ui,
                    &self.t.survey_summary_title(),
                    button_text_color,
                    button_color,
                )
                .clicked()
                {
                    *self.show_survey_summary = true;
                }

                // Divider - horizontal line matching button width
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.set_min_width(100.0);
                    ui.set_max_width(140.0);
                    ui.add(egui::Separator::default().horizontal().spacing(4.0));
                });
                ui.add_space(12.0);

                // Section: Feature Toggles
                // Notes Toggle
                let notes_label = if *self.notes_enabled {
                    self.t.notes_on()
                } else {
                    self.t.notes_off()
                };
                if ui_components::sidebar_button(ui, &notes_label, button_text_color, button_color)
                    .clicked()
                {
                    *self.notes_enabled = !*self.notes_enabled;
                    self.config.notes_enabled = *self.notes_enabled;
                    let _ = self.config.save();
                }

                // TODO Toggle
                ui.add_space(8.0);
                let todo_label = if *self.todo_enabled {
                    self.t.todo_on()
                } else {
                    self.t.todo_off()
                };
                if ui_components::sidebar_button(ui, &todo_label, button_text_color, button_color)
                    .clicked()
                {
                    *self.todo_enabled = !*self.todo_enabled;
                    self.config.todo_enabled = *self.todo_enabled;
                    let _ = self.config.save();
                }

                // Kanban Toggle
                ui.add_space(8.0);
                let kanban_label = if *self.kanban_enabled {
                    "KANBAN: ON"
                } else {
                    "KANBAN: OFF"
                };
                if ui_components::sidebar_button(ui, kanban_label, button_text_color, button_color)
                    .clicked()
                {
                    *self.kanban_enabled = !*self.kanban_enabled;
                    self.config.kanban_enabled = *self.kanban_enabled;
                    let _ = self.config.save();
                }

                // Divider - horizontal line matching button width
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.set_min_width(100.0);
                    ui.set_max_width(140.0);
                    ui.add(egui::Separator::default().horizontal().spacing(4.0));
                });
                ui.add_space(12.0);

                // Section: Help and Info
                if ui_components::sidebar_button(
                    ui,
                    &self.t.help_button(),
                    button_text_color,
                    button_color,
                )
                .clicked()
                {
                    *self.show_help = !*self.show_help;
                }

                // Sessions completed label
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(self.t.sessions_completed_label(self.sessions_completed))
                        .size(12.0)
                        .color(text_dim_color),
                );
            }
        });
    }
}
