use crate::kanban::{KanbanBoard, KanbanStatus};
use crate::localization::T;
use crate::markdown::format_inline_markdown;
use crate::todo::TodoList;
use eframe::egui;
use egui::{Color32, Frame, Id};

/// Manual word wrap utility for character-based wrapping.
fn wrap_text(text: &str, max_chars: usize) -> String {
    let mut result = String::new();
    let mut current_line_len = 0;

    for word in text.split_whitespace() {
        if current_line_len + word.len() > max_chars {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(word);
            current_line_len = word.len();
        } else {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push(' ');
                current_line_len += 1;
            }
            result.push_str(word);
            current_line_len += word.len();
        }
    }

    if result.is_empty() {
        text.to_string()
    } else {
        result
    }
}

/// Renders a markdown preview within an egui::Frame.
pub fn render_markdown_preview(ui: &mut egui::Ui, content: &str, text_color: Color32, bg_color: Color32) {
    egui::Frame::group(ui.style())
        .fill(bg_color)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut in_code_block = false;

            for line in content.lines() {
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
                            .size(24.0)
                            .strong()
                            .color(text_color),
                    );
                    ui.add_space(4.0);
                } else if trimmed.starts_with("## ") {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("## ").unwrap_or(trimmed))
                            .size(20.0)
                            .strong()
                            .color(text_color),
                    );
                    ui.add_space(3.0);
                } else if trimmed.starts_with("### ") {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("### ").unwrap_or(trimmed))
                            .size(16.0)
                            .strong()
                            .color(text_color),
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
                            .color(text_color),
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
                            .color(text_color),
                    );
                } else if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4
                {
                    let bold_text = trimmed[2..trimmed.len() - 2].to_string();
                    ui.label(
                        egui::RichText::new(bold_text)
                            .strong()
                            .color(text_color),
                    );
                } else if trimmed.is_empty() {
                    // Preserve empty lines as spacing
                    ui.add_space(6.0);
                } else {
                    // Regular paragraph text - handle inline bold via our markdown helper
                    let text = format_inline_markdown(trimmed);
                    ui.label(
                        egui::RichText::new(text).color(text_color),
                    );
                }
            }
            ui.add_space(20.0);
        });
}

/// Renders the TODO list panel.
pub fn render_todo_panel(
    ui: &mut egui::Ui,
    todo_list: &mut TodoList,
    todo_input: &mut String,
    kanban_board: &crate::kanban::KanbanBoard,
    t: &T,
    text_color: egui::Color32,
    button_color: egui::Color32,
) {
    ui.add_space(15.0);
    ui.vertical(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(t.todo_title())
                .size(24.0)
                .color(text_color)
                .strong(),
        );
        ui.add_space(10.0);

        // TODO status
        let incomplete = todo_list.incomplete_count();
        let completed = todo_list.completed_count();
        ui.label(
            egui::RichText::new(t.todo_status(incomplete, completed))
                .size(11.0)
                .color(text_color.linear_multiply(0.6)),
        );

        ui.add_space(5.0);

        // Add TODO input
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(todo_input)
                    .hint_text(t.todo_hint())
                    .desired_width(ui.available_width() - 20.0),
            );
            if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                && !todo_input.trim().is_empty()
            {
                todo_list.add(todo_input.trim().to_string());
                todo_input.clear();
                let _ = todo_list.save();
            }
        });
    });

    // TODO list area
    ui.add_space(5.0);
    // Use the available space in the column; the parent sidebar provides scrolling
    ui.vertical(|ui| {
        let mut to_remove = None;
        let mut to_toggle = None;

        // Render Active items
        for item in todo_list.active.iter_mut() {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut item.completed, "").clicked() {
                    to_toggle = Some(item.id);
                }

                ui.label(egui::RichText::new(&item.text).color(text_color));

                // Add status indicator emoji for Kanban interplay
                let status_emoji = if let Some(k_item) = kanban_board.items.iter().find(|k| k.text == item.text) {
                    match k_item.status {
                        KanbanStatus::Todo => "🔵",
                        KanbanStatus::InProgress => "🟡",
                        KanbanStatus::Done => "🟢",
                    }
                } else {
                    "⚪"
                };
                ui.label(status_emoji);

                if ui
                    .add(egui::Button::new("❌").fill(button_color).small())
                    .clicked()
                {
                    to_remove = Some(item.id);
                }
            });
        }

        // Render Completed items
        for item in todo_list.completed.iter_mut() {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut item.completed, "").clicked() {
                    to_toggle = Some(item.id);
                }

                let cur_text_color = text_color.linear_multiply(0.5);
                ui.label(
                    egui::RichText::new(&item.text)
                        .strikethrough()
                        .color(cur_text_color),
                );
                ui.label("🟢"); // Always green if completed in TODO list

                if ui
                    .add(egui::Button::new("❌").fill(button_color).small())
                    .clicked()
                {
                    to_remove = Some(item.id);
                }
            });
        }

        if let Some(id) = to_toggle {
            todo_list.toggle(id);
            let _ = todo_list.save();
        }

        if let Some(id) = to_remove {
            todo_list.remove(id);
            let _ = todo_list.save();
        }
    });

    // Clear completed button
    let completed = todo_list.completed_count();
    if completed > 0 {
        ui.add_space(10.0);
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new(t.todo_clear_completed_btn()).color(text_color),
                )
                .fill(button_color)
                .rounding(8.0),
            )
            .clicked()
        {
            todo_list.clear_completed();
            let _ = todo_list.save();
        }
    }
}

/// A standard rounded button for the UI.
pub fn rounded_button(
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

/// Renders the Kanban board with drag-and-drop support.
pub fn render_kanban_board(
    ui: &mut egui::Ui,
    board: &mut KanbanBoard,
    kanban_input: &mut String,
    _t: &T,
    text_color: Color32,
    bg_color: Color32,
    item_bg_color: Color32,
) {
    ui.add_space(10.0);
    
    egui::Frame::none()
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Kanban Board")
                    .size(24.0)
                    .color(text_color)
                    .strong(),
            );
            ui.add_space(10.0);

            // Add Kanban item input
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(kanban_input)
                        .hint_text("Add new task...")
                        .desired_width(ui.available_width() - 220.0),
                );
                if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    && !kanban_input.trim().is_empty()
                {
                    board.add_item(kanban_input.trim().to_string());
                    kanban_input.clear();
                    let _ = board.save();
                }
                if ui.button("Add").clicked() && !kanban_input.trim().is_empty() {
                    board.add_item(kanban_input.trim().to_string());
                    kanban_input.clear();
                    let _ = board.save();
                }

                ui.add_space(5.0);
                if ui.button("Clear").clicked() {
                    kanban_input.clear();
                }

                ui.add_space(5.0);
                if ui.button("Clear Done").clicked() {
                    board.clear_done();
                    let _ = board.save();
                }
            });

            ui.add_space(10.0);

            let mut from_item_id = None;
            let mut to_status = None;

            let statuses = [
                (KanbanStatus::Todo, "TODO"),
                (KanbanStatus::InProgress, "IN PROGRESS"),
                (KanbanStatus::Done, "DONE"),
            ];

            let column_width = (ui.available_width() - 20.0) / 3.0;

            ui.horizontal_top(|ui| {
                for (status, label) in statuses.iter() {
                    ui.vertical(|ui| {
                        ui.set_width(column_width);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(*label)
                                    .strong()
                                    .color(text_color),
                            );
                        });
                        ui.add_space(5.0);

                        let frame = Frame::group(ui.style())
                            .fill(bg_color)
                            .inner_margin(4.0);

                        let (_, dropped_payload) = ui.dnd_drop_zone::<usize, ()>(frame, |ui| {
                            ui.set_min_height(250.0);
                            ui.set_width(ui.available_width());

                            let items_in_col: Vec<_> = board
                                .items
                                .iter()
                                .filter(|i| i.status == *status)
                                .cloned()
                                .collect();

                            for item in items_in_col {
                                let item_id = Id::new(("kanban_item", item.id));
                                let response = ui
                                    .dnd_drag_source(item_id, item.id, |ui| {
                                        Frame::none()
                                            .fill(item_bg_color)
                                            .rounding(4.0)
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.horizontal(|ui| {
                                                    let wrapped = wrap_text(&item.text, 45);
                                                    ui.label(
                                                        egui::RichText::new(wrapped)
                                                            .color(text_color),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if ui.small_button("❌").clicked() {
                                                                board.delete_item(item.id);
                                                                let _ = board.save();
                                                            }
                                                        },
                                                    );
                                                });
                                            });
                                    })
                                    .response;

                                if let Some(dragged_id) = response.dnd_release_payload() {
                                    from_item_id = Some(*dragged_id);
                                    to_status = Some(status.clone());
                                }
                            }
                        });

                        if let Some(dragged_id) = dropped_payload {
                            from_item_id = Some(*dragged_id);
                            to_status = Some(status.clone());
                        }
                    });

                    if *status != KanbanStatus::Done {
                        ui.separator();
                    }
                }
            });

            if let (Some(item_id), Some(status)) = (from_item_id, to_status) {
                board.move_item(item_id, status);
                let _ = board.save();
            }
        });
}
