use eframe::egui;
use crate::easy_mark::highlighter::MemoizedEasymarkHighlighter;

#[derive(Default)]
pub struct EasyMarkEditor {
    highlighter: MemoizedEasymarkHighlighter,
}

impl EasyMarkEditor {
    pub fn show(&mut self, ui: &mut egui::Ui, content: &mut String) -> egui::Response {
        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = self.highlighter.highlight(ui.style(), string);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        ui.add(egui::TextEdit::multiline(content)
            .id(egui::Id::new("notes_text_input"))
            .desired_width(f32::INFINITY)
            .desired_rows(12)
            .font(egui::TextStyle::Monospace)
            .layouter(&mut layouter))
    }
}
