use eframe::egui;
use crate::markdown::format_inline_markdown;
use crate::todo::TodoList;
use crate::localization::T;

/// Renders a markdown preview within an egui::Frame.
pub fn render_markdown_preview(ui: &mut egui::Ui, content: &str) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(0x2a, 0x2a, 0x40))
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
                            .color(egui::Color32::from_rgb(0xee, 0xee, 0xee)),
                    );
                    ui.add_space(4.0);
                } else if trimmed.starts_with("## ") {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("## ").unwrap_or(trimmed))
                            .size(20.0)
                            .strong()
                            .color(egui::Color32::from_rgb(0xee, 0xee, 0xee)),
                    );
                    ui.add_space(3.0);
                } else if trimmed.starts_with("### ") {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(trimmed.strip_prefix("### ").unwrap_or(trimmed))
                            .size(16.0)
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
                    // Regular paragraph text - handle inline bold via our markdown helper
                    let text = format_inline_markdown(trimmed);
                    ui.label(
                        egui::RichText::new(text).color(egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)),
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
                .color(egui::Color32::from_rgb(0x88, 0x88, 0x88)),
        );

        ui.add_space(5.0);

        // Add TODO input
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(todo_input)
                    .hint_text(t.todo_hint())
                    .desired_width(280.0),
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

    // TODO list scroll area
    ui.add_space(5.0);
    egui::ScrollArea::vertical()
        .id_salt("todo_scroll_area")
        .max_height(350.0)
        .show(ui, |ui| {
            let mut to_remove = None;
            let mut to_toggle = None;

            // Render Active items
            for item in todo_list.active.iter_mut() {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut item.completed, "").clicked() {
                        to_toggle = Some(item.id);
                    }

                    ui.label(egui::RichText::new(&item.text).color(text_color));

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

                    let cur_text_color = egui::Color32::from_rgb(0x88, 0x88, 0x88);
                    ui.label(
                        egui::RichText::new(&item.text)
                            .strikethrough()
                            .color(cur_text_color),
                    );

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
