use ropey::Rope;
use std::ops::Range;

#[derive(Clone)]
pub struct RopeBuffer {
    rope: Rope,
}

impl RopeBuffer {
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
    }

    pub fn insert_char(&mut self, char_idx: usize, ch: char) {
        self.rope.insert_char(char_idx, ch);
    }

    pub fn remove(&mut self, range: Range<usize>) {
        self.rope.remove(range);
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, line_idx: usize) -> ropey::RopeSlice<'_> {
        self.rope.line(line_idx)
    }

    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx)
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn get_line_text(&self, line_idx: usize) -> String {
        if line_idx < self.len_lines() {
            let line = self.line(line_idx);
            line.to_string().trim_end_matches('\n').to_string()
        } else {
            String::new()
        }
    }

    pub fn slice(&self, range: Range<usize>) -> ropey::RopeSlice<'_> {
        self.rope.slice(range)
    }

    pub fn replace_line(&mut self, line_idx: usize, new_text: &str) {
        if line_idx >= self.len_lines() {
            return;
        }

        let line_start = self.line_to_char(line_idx);
        let line_end = if line_idx + 1 < self.len_lines() {
            self.line_to_char(line_idx + 1) - 1 // Exclude the newline
        } else {
            self.len_chars()
        };

        // Remove the old line content
        if line_end > line_start {
            self.rope.remove(line_start..line_end);
        }

        // Insert the new line content
        self.rope.insert(line_start, new_text);
    }
}
