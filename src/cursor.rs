use crate::rope_buffer::RopeBuffer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub position: Position,
    pub desired_column: Option<usize>,
    pub selection_start: Option<Position>,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            position: Position::new(0, 0),
            desired_column: None,
            selection_start: None,
        }
    }


    pub fn move_left(&mut self, buffer: &RopeBuffer) {
        if self.position.column > 0 {
            self.position.column -= 1;
        } else if self.position.line > 0 {
            self.position.line -= 1;
            let line_len = buffer.get_line_text(self.position.line).len();
            self.position.column = line_len;
        }
        self.desired_column = None;
    }

    pub fn move_right(&mut self, buffer: &RopeBuffer) {
        let line_len = buffer.get_line_text(self.position.line).len();
        if self.position.column < line_len {
            self.position.column += 1;
        } else if self.position.line < buffer.len_lines().saturating_sub(1) {
            self.position.line += 1;
            self.position.column = 0;
        }
        self.desired_column = None;
    }

    pub fn move_up(&mut self, buffer: &RopeBuffer) {
        if self.position.line > 0 {
            self.position.line -= 1;
            let line_len = buffer.get_line_text(self.position.line).len();
            
            if let Some(desired) = self.desired_column {
                self.position.column = desired.min(line_len);
            } else {
                self.desired_column = Some(self.position.column);
                self.position.column = self.position.column.min(line_len);
            }
        }
    }

    pub fn move_down(&mut self, buffer: &RopeBuffer) {
        if self.position.line < buffer.len_lines().saturating_sub(1) {
            self.position.line += 1;
            let line_len = buffer.get_line_text(self.position.line).len();
            
            if let Some(desired) = self.desired_column {
                self.position.column = desired.min(line_len);
            } else {
                self.desired_column = Some(self.position.column);
                self.position.column = self.position.column.min(line_len);
            }
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.position.column = 0;
        self.desired_column = None;
    }

    pub fn move_to_line_end(&mut self, buffer: &RopeBuffer) {
        let line_len = buffer.get_line_text(self.position.line).len();
        self.position.column = line_len;
        self.desired_column = None;
    }

    pub fn move_word_left(&mut self, buffer: &RopeBuffer) {
        let line_text = buffer.get_line_text(self.position.line);
        let chars: Vec<char> = line_text.chars().collect();
        
        if self.position.column > 0 && !chars.is_empty() {
            let mut pos = self.position.column.min(chars.len());
            
            // If we're past the end of line, move to end
            if pos > chars.len() {
                pos = chars.len();
            }
            
            // Move left by one to start
            if pos > 0 {
                pos -= 1;
            }
            
            // Skip whitespace backwards
            while pos > 0 && chars.get(pos).map_or(false, |c| !c.is_alphanumeric() && *c != '_') {
                pos -= 1;
            }
            
            // Skip word characters backwards
            while pos > 0 && chars.get(pos - 1).map_or(false, |c| c.is_alphanumeric() || *c == '_') {
                pos -= 1;
            }
            
            self.position.column = pos;
        } else if self.position.line > 0 {
            self.position.line -= 1;
            self.move_to_line_end(buffer);
        }
        self.desired_column = None;
    }

    pub fn move_word_right(&mut self, buffer: &RopeBuffer) {
        let line_text = buffer.get_line_text(self.position.line);
        let chars: Vec<char> = line_text.chars().collect();
        let line_len = chars.len();
        
        if self.position.column < line_len {
            let mut pos = self.position.column;
            
            // Skip current word characters
            while pos < line_len && chars.get(pos).map_or(false, |c| c.is_alphanumeric() || *c == '_') {
                pos += 1;
            }
            
            // Skip whitespace and punctuation
            while pos < line_len && chars.get(pos).map_or(false, |c| !c.is_alphanumeric() && *c != '_') {
                pos += 1;
            }
            
            self.position.column = pos;
        } else if self.position.line < buffer.len_lines().saturating_sub(1) {
            self.position.line += 1;
            self.position.column = 0;
        }
        self.desired_column = None;
    }

    pub fn to_char_index(&self, buffer: &RopeBuffer) -> usize {
        let line_start = buffer.line_to_char(self.position.line);
        let line_text = buffer.get_line_text(self.position.line);
        let column = self.position.column.min(line_text.len());
        line_start + column
    }

    pub fn start_selection(&mut self) {
        self.selection_start = Some(self.position);
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
    }

    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
    }

    pub fn get_selection(&self) -> Option<(Position, Position)> {
        if let Some(start) = self.selection_start {
            let end = self.position;
            
            // Ensure start comes before end
            if start.line < end.line || (start.line == end.line && start.column <= end.column) {
                Some((start, end))
            } else {
                Some((end, start))
            }
        } else {
            None
        }
    }

    pub fn select_all(&mut self, buffer: &RopeBuffer) {
        self.selection_start = Some(Position::new(0, 0));
        if buffer.len_lines() > 0 {
            let last_line = buffer.len_lines() - 1;
            let last_line_len = buffer.get_line_text(last_line).len();
            self.position = Position::new(last_line, last_line_len);
        }
    }

    // Movement with selection
    pub fn move_left_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_left(buffer);
    }

    pub fn move_right_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_right(buffer);
    }

    pub fn move_up_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_up(buffer);
    }

    pub fn move_down_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_down(buffer);
    }

    pub fn move_word_left_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_word_left(buffer);
    }

    pub fn move_word_right_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_word_right(buffer);
    }

    pub fn move_to_line_start_with_selection(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_to_line_start();
    }

    pub fn move_to_line_end_with_selection(&mut self, buffer: &RopeBuffer, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.start_selection();
        } else if !extend_selection {
            self.clear_selection();
        }
        self.move_to_line_end(buffer);
    }

    pub fn select_word_at_position(&mut self, buffer: &RopeBuffer) {
        let line_text = buffer.get_line_text(self.position.line);
        let chars: Vec<char> = line_text.chars().collect();
        
        if chars.is_empty() {
            return;
        }
        
        // Handle position at end of line
        let actual_column = if self.position.column >= chars.len() {
            if chars.len() > 0 { chars.len() - 1 } else { return; }
        } else {
            self.position.column
        };
        
        let current_char = chars[actual_column];
        
        // If not on a word character, don't select anything
        if !is_word_char(current_char) {
            return;
        }
        
        // Find word boundaries
        let mut start_col = actual_column;
        let mut end_col = actual_column;
        
        // Move start backwards to beginning of word
        while start_col > 0 && is_word_char(chars[start_col - 1]) {
            start_col -= 1;
        }
        
        // Move end forwards to end of word
        while end_col < chars.len() && is_word_char(chars[end_col]) {
            end_col += 1;
        }
        
        // Set selection
        self.selection_start = Some(Position::new(self.position.line, start_col));
        self.position = Position::new(self.position.line, end_col);
    }

}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}