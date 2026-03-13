use eframe::egui::{self, text::{CCursor, CCursorRange}};
use crate::ui_components;
use crate::localization::T;
use crate::easy_mark::editor::EasyMarkEditor;
use crate::config::NotesView;

pub struct NotesEditor<'a> {
    pub view: &'a mut NotesView,
    pub content: &'a mut String,
    pub project_content: &'a mut String,
    pub focus_input: &'a mut bool,
    pub notes_cursor_pos: &'a mut usize,
    pub requested_cursor_pos: &'a mut Option<usize>,
    pub editor: &'a mut EasyMarkEditor,
    pub t: &'a T,
}

pub enum NotesAction {
    NotesChanged,
    SaveNotes,
}

pub struct NotesResponse {
    pub action: Option<NotesAction>,
    pub output: Option<egui::text_edit::TextEditOutput>,
}

impl<'a> NotesEditor<'a> {
    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, 
               text_color: egui::Color32, active_color: egui::Color32, 
               inactive_color: egui::Color32, text_highlight_color: egui::Color32, 
               bg_color: egui::Color32, todo_file: &str) -> NotesResponse {
        let mut action = None;
        let mut output_ret = None;

        egui::Frame::group(ui.style())
            .fill(bg_color)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(15.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Tab buttons
                    ui.horizontal(|ui| {
                        let edit_active = *self.view == NotesView::Edit;
                        let edit_color = if edit_active { active_color } else { inactive_color };
                        let edit_text_color = if edit_active { text_highlight_color } else { text_color };

                        let preview_active = *self.view == NotesView::Preview;
                        let preview_color = if preview_active { active_color } else { inactive_color };
                        let preview_text_color = if preview_active { text_highlight_color } else { text_color };

                        let project_active = *self.view == NotesView::Project;
                        let project_color = if project_active { active_color } else { inactive_color };
                        let project_text_color = if project_active { text_highlight_color } else { text_color };

                        if ui_components::small_rounded_button(ui, &self.t.edit_tab(), edit_text_color, edit_color).clicked() {
                            *self.view = NotesView::Edit;
                            *self.focus_input = true;
                        }
                        
                        ui.add_space(5.0);

                        if ui_components::small_rounded_button(ui, &self.t.preview_tab(), preview_text_color, preview_color).clicked() {
                            *self.view = NotesView::Preview;
                        }

                        ui.add_space(5.0);

                        if ui_components::small_rounded_button(ui, "Project", project_text_color, project_color).clicked() {
                            *self.view = NotesView::Project;
                            *self.project_content = std::fs::read_to_string(todo_file).unwrap_or_default();
                            *self.focus_input = true;
                        }
                    });

                    ui.add_space(5.0);
                    match *self.view {
                        NotesView::Edit => {
                            if *self.focus_input {
                                ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("notes_text_input")));
                                *self.focus_input = false;
                            }

                            // Handle CMD/CTRL+S hotkey for saving notes
                            let save_shortcut_pressed = ctx.input(|i| {
                                i.key_pressed(egui::Key::S) && i.modifiers.command
                            });
                            if save_shortcut_pressed {
                                action = Some(NotesAction::SaveNotes);
                            }

                            let old_notes = self.content.clone();
                            let output = self.editor.show(ui, self.content);
                            if *self.content != old_notes && action.is_none() {
                                action = Some(NotesAction::NotesChanged);
                            }
                            
                            let response = output.response.clone();

                            // Track cursor position for Tab handling
                            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                                if let Some(range) = state.cursor.char_range() {
                                    *self.notes_cursor_pos = range.primary.index;
                                }
                            }

                            if let Some(pos) = self.requested_cursor_pos.take() {
                                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                                    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(pos))));
                                    egui::TextEdit::store_state(ui.ctx(), response.id, state);
                                }
                            }
                            output_ret = Some(output);
                        }
                        NotesView::Preview => {
                            let text_color = egui::Color32::from_rgb(0xee, 0xee, 0xee); // Fallback or pass in
                            let bg_color = egui::Color32::from_rgb(0x1a, 0x1a, 0x2e);
                            ui_components::render_markdown_preview(ui, self.content, text_color, bg_color);
                        }
                        NotesView::Project => {
                            let old_project = self.project_content.clone();
                            ui.add(egui::TextEdit::multiline(self.project_content)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY));
                            if *self.project_content != old_project {
                                // Potentially signal to save project file
                            }
                        }
                    }
                });
            });
        NotesResponse {
            action,
            output: output_ret,
        }
    }
}
