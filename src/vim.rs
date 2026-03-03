use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
}

impl Default for VimMode {
    fn default() -> Self {
        VimMode::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VimState {
    pub mode: VimMode,
    #[serde(skip)]
    pub last_col: usize,
    #[serde(skip)]
    pub yank_buffer: String,
    #[serde(skip)]
    pub pending_operator: Option<char>,
}

impl Default for VimState {
    fn default() -> Self {
        Self {
            mode: VimMode::Normal,
            last_col: 0,
            yank_buffer: String::new(),
            pending_operator: None,
        }
    }
}

impl VimState {
    pub fn handle_input(&mut self, ctx: &egui::Context, content: &mut String, cursor_pos: &mut usize) -> bool {
        let mut consumed = false;
        
        let events = ctx.input(|i| i.events.clone());
        
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if self.mode == VimMode::Normal {
                        for ch in text.chars() {
                            if self.process_normal_key(ch, content, cursor_pos) {
                                consumed = true;
                            }
                        }
                    }
                }
                egui::Event::Key { key, pressed: true, .. } => {
                    if self.mode == VimMode::Normal {
                         if self.process_normal_special_key(key, content, cursor_pos) {
                             consumed = true;
                         }
                    } else if self.mode == VimMode::Insert {
                        if key == egui::Key::Escape {
                            self.mode = VimMode::Normal;
                            self.pending_operator = None;
                            // Move cursor back one if possible (Vim behavior)
                            if *cursor_pos > 0 {
                                let line_start = self.get_line_start(content, *cursor_pos);
                                if *cursor_pos > line_start {
                                    *cursor_pos -= 1;
                                }
                            }
                            consumed = true;
                        }
                    }
                }
                _ => {}
            }
        }

        consumed
    }

    fn process_normal_key(&mut self, ch: char, content: &mut String, cursor_pos: &mut usize) -> bool {
        // Handle pending operators
        if let Some(op) = self.pending_operator.take() {
            match (op, ch) {
                ('d', 'd') => {
                    self.delete_current_line(content, cursor_pos);
                    return true;
                }
                ('y', 'y') => {
                    self.yank_current_line(content, cursor_pos);
                    return true;
                }
                _ => {
                    // Unknown combo, cancel
                    return false;
                }
            }
        }

        match ch {
            'i' => {
                self.mode = VimMode::Insert;
                true
            }
            'I' => {
                *cursor_pos = self.get_line_start(content, *cursor_pos);
                self.mode = VimMode::Insert;
                true
            }
            'a' => {
                self.mode = VimMode::Insert;
                let total_chars = content.chars().count();
                if *cursor_pos < total_chars {
                    // Only move forward if not at the absolute end of the file
                    // Actually 'a' always moves forward in vim unless it's an empty line/file
                     *cursor_pos += 1;
                }
                true
            }
            'A' => {
                let line_end = self.get_line_end(content, *cursor_pos);
                *cursor_pos = line_end;
                self.mode = VimMode::Insert;
                true
            }
            'o' => {
                 let line_end = self.get_line_end(content, *cursor_pos);
                 let byte_pos = self.char_to_byte(content, line_end);
                 content.insert_str(byte_pos, "\n");
                 *cursor_pos = line_end + 1;
                 self.mode = VimMode::Insert;
                 true
            }
            'O' => {
                let line_start = self.get_line_start(content, *cursor_pos);
                let byte_pos = self.char_to_byte(content, line_start);
                content.insert_str(byte_pos, "\n");
                *cursor_pos = line_start;
                self.mode = VimMode::Insert;
                true
            }
            'h' => {
                let line_start = self.get_line_start(content, *cursor_pos);
                if *cursor_pos > line_start {
                    *cursor_pos -= 1;
                }
                self.update_last_col(content, *cursor_pos);
                true
            }
            'l' => {
                let line_end = self.get_line_end(content, *cursor_pos);
                if *cursor_pos < line_end.saturating_sub(1) {
                    *cursor_pos += 1;
                }
                self.update_last_col(content, *cursor_pos);
                true
            }
            'j' => {
                self.move_vertical(content, cursor_pos, 1);
                true
            }
            'k' => {
                self.move_vertical(content, cursor_pos, -1);
                true
            }
            'x' => {
                if !content.is_empty() {
                     let total_chars = content.chars().count();
                     if *cursor_pos < total_chars {
                         let ch_at = content.chars().nth(*cursor_pos).unwrap();
                         if ch_at != '\n' {
                             let byte_pos = self.char_to_byte(content, *cursor_pos);
                             content.remove(byte_pos);
                             // If we deleted the last char of the line, move cursor back
                             let line_end = self.get_line_end(content, *cursor_pos);
                             if *cursor_pos >= line_end && *cursor_pos > 0 {
                                 *cursor_pos -= 1;
                             }
                         }
                     }
                }
                true
            }
            '0' => {
                *cursor_pos = self.get_line_start(content, *cursor_pos);
                self.last_col = 0;
                true
            }
            '$' => {
                let line_end = self.get_line_end(content, *cursor_pos);
                *cursor_pos = line_end.saturating_sub(1);
                self.last_col = usize::MAX;
                true
            }
            'd' | 'y' => {
                self.pending_operator = Some(ch);
                true
            }
            'p' => {
                if !self.yank_buffer.is_empty() {
                    let line_end = self.get_line_end(content, *cursor_pos);
                    let byte_pos = self.char_to_byte(content, line_end);
                    content.insert_str(byte_pos, &self.yank_buffer);
                    *cursor_pos = line_end + 1;
                }
                true
            }
            'P' => {
                if !self.yank_buffer.is_empty() {
                    let line_start = self.get_line_start(content, *cursor_pos);
                    let byte_pos = self.char_to_byte(content, line_start);
                    content.insert_str(byte_pos, &self.yank_buffer);
                    *cursor_pos = line_start;
                }
                true
            }
            'u' => {
                // TODO: Implement undo
                false
            }
            'G' => {
                *cursor_pos = content.chars().count().saturating_sub(1);
                true
            }
            'g' => {
                // gg
                self.pending_operator = Some('g');
                true
            }
            _ => {
                if self.pending_operator == Some('g') && ch == 'g' {
                    self.pending_operator = None;
                    *cursor_pos = 0;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn process_normal_special_key(&mut self, key: egui::Key, content: &mut String, cursor_pos: &mut usize) -> bool {
        match key {
            egui::Key::ArrowLeft => self.process_normal_key('h', content, cursor_pos),
            egui::Key::ArrowRight => self.process_normal_key('l', content, cursor_pos),
            egui::Key::ArrowUp => self.process_normal_key('k', content, cursor_pos),
            egui::Key::ArrowDown => self.process_normal_key('j', content, cursor_pos),
            egui::Key::Escape => {
                self.pending_operator = None;
                true
            }
            _ => false,
        }
    }

    fn char_to_byte(&self, content: &str, char_idx: usize) -> usize {
        content.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(content.len())
    }

    fn get_line_start(&self, content: &str, char_idx: usize) -> usize {
        let byte_pos = self.char_to_byte(content, char_idx);
        let start_byte = content[..byte_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        content[..start_byte].chars().count()
    }

    fn get_line_end(&self, content: &str, char_idx: usize) -> usize {
        let byte_pos = self.char_to_byte(content, char_idx);
        let next_newline = content[byte_pos..].find('\n');
        match next_newline {
            Some(offset) => content[..byte_pos + offset + 1].chars().count(),
            None => content.chars().count(),
        }
    }

    fn update_last_col(&mut self, content: &str, cursor_pos: usize) {
        let line_start = self.get_line_start(content, cursor_pos);
        self.last_col = cursor_pos - line_start;
    }

    fn delete_current_line(&mut self, content: &mut String, cursor_pos: &mut usize) {
        let start_char = self.get_line_start(content, *cursor_pos);
        let end_char = self.get_line_end(content, *cursor_pos);
        
        let start_byte = self.char_to_byte(content, start_char);
        let end_byte = self.char_to_byte(content, end_char);
        
        if start_byte < end_byte {
            self.yank_buffer = content[start_byte..end_byte].to_string();
            content.replace_range(start_byte..end_byte, "");
            
            let total_chars = content.chars().count();
            if *cursor_pos >= total_chars && total_chars > 0 {
                *cursor_pos = total_chars - 1;
            }
            *cursor_pos = self.get_line_start(content, *cursor_pos);
        }
    }

    fn yank_current_line(&mut self, content: &str, cursor_pos: &usize) {
        let start_char = self.get_line_start(content, *cursor_pos);
        let end_char = self.get_line_end(content, *cursor_pos);
        
        let start_byte = self.char_to_byte(content, start_char);
        let end_byte = self.char_to_byte(content, end_char);
        
        if start_byte < end_byte {
            self.yank_buffer = content[start_byte..end_byte].to_string();
        }
    }

    fn move_vertical(&mut self, content: &str, cursor_pos: &mut usize, dir: i32) {
        let current_char_pos = *cursor_pos;
        let line_start = self.get_line_start(content, current_char_pos);
        
        if dir > 0 {
            // Move down
            let current_line_end_char = self.get_line_end(content, current_char_pos);
            let next_line_start_char = current_line_end_char;
            if next_line_start_char < content.chars().count() {
                let next_line_end_char = self.get_line_end(content, next_line_start_char);
                let next_line_len = next_line_end_char - next_line_start_char;
                let next_line_len_no_nl = if next_line_end_char > next_line_start_char && content.chars().nth(next_line_end_char - 1) == Some('\n') {
                    next_line_len - 1
                } else {
                    next_line_len
                };

                let target_col = if self.last_col == usize::MAX {
                    next_line_len_no_nl.saturating_sub(1)
                } else {
                    self.last_col.min(next_line_len_no_nl.saturating_sub(1))
                };
                *cursor_pos = next_line_start_char + target_col;
            }
        } else {
            // Move up
            if line_start > 0 {
                let prev_line_end_char = line_start;
                let prev_line_start_char = self.get_line_start(content, prev_line_end_char - 1);
                let prev_line_len = prev_line_end_char - prev_line_start_char;
                let prev_line_len_no_nl = if prev_line_len > 0 && content.chars().nth(prev_line_end_char - 1) == Some('\n') {
                    prev_line_len - 1
                } else {
                    prev_line_len
                };

                let target_col = if self.last_col == usize::MAX {
                    prev_line_len_no_nl.saturating_sub(1)
                } else {
                    self.last_col.min(prev_line_len_no_nl.saturating_sub(1))
                };
                *cursor_pos = prev_line_start_char + target_col;
            }
        }
    }
}
