use crate::{cursor::{Cursor, Position}, rope_buffer::RopeBuffer};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FindMatch {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone)]
struct EditorState {
    buffer: RopeBuffer,
    cursor: Cursor,
}

#[derive(Clone, PartialEq)]
pub enum FindFocusedField {
    Find,
    Replace,
}

#[derive(Clone)]
pub struct FindReplaceState {
    pub active: bool,
    pub find_query: String,
    pub replace_query: String,
    pub current_match_index: Option<usize>,
    pub matches: Vec<FindMatch>,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_replace_mode: bool,
    pub find_cursor_position: usize,
    pub replace_cursor_position: usize,
    pub focused_field: FindFocusedField,
}

impl Default for FindReplaceState {
    fn default() -> Self {
        Self {
            active: false,
            find_query: String::new(),
            replace_query: String::new(),
            current_match_index: None,
            matches: Vec::new(),
            case_sensitive: false,
            whole_word: false,
            is_replace_mode: false,
            find_cursor_position: 0,
            replace_cursor_position: 0,
            focused_field: FindFocusedField::Find,
        }
    }
}

#[derive(Clone)]
pub struct Tab {
    pub name: String,
    pub path: Option<PathBuf>,
    pub buffer: RopeBuffer,
    pub cursor: Cursor,
    pub viewport_offset: (usize, usize),
    pub modified: bool,
    pub preview_mode: bool,
    pub word_wrap: bool,
    pub find_replace_state: FindReplaceState,
    undo_stack: Vec<EditorState>,
    redo_stack: Vec<EditorState>,
    max_undo_history: usize,
}

impl Tab {
    pub fn new(name: String) -> Self {
        Self {
            name,
            path: None,
            buffer: RopeBuffer::new(),
            cursor: Cursor::new(),
            viewport_offset: (0, 0),
            modified: false,
            preview_mode: false,
            word_wrap: false,
            find_replace_state: FindReplaceState::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 100,
        }
    }

    pub fn from_file(path: PathBuf, content: &str) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();

        // Check if this is a markdown file and activate preview mode by default
        let is_markdown = if let Some(ext) = path.extension() {
            ext == "md" || ext == "markdown"
        } else {
            name.ends_with(".md") || name.ends_with(".markdown")
        };

        Self {
            name,
            path: Some(path),
            buffer: RopeBuffer::from_str(content),
            cursor: Cursor::new(),
            viewport_offset: (0, 0),
            modified: false,
            preview_mode: is_markdown, // Default to preview mode for markdown files
            word_wrap: false,
            find_replace_state: FindReplaceState::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 100,
        }
    }

    pub fn display_name(&self) -> String {
        let name = self.name.clone();
        if self.modified {
            format!("{}*", name)
        } else {
            name
        }
    }

    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    pub fn update_viewport(&mut self, height: usize) {
        // Only adjust viewport if cursor is outside the visible area
        // This allows manual scrolling to work without being overridden
        let cursor_line = self.cursor.position.line;
        let (viewport_line, viewport_col) = self.viewport_offset;

        // Only adjust if cursor is completely outside the visible area
        if cursor_line < viewport_line {
            self.viewport_offset.0 = cursor_line;
        } else if cursor_line >= viewport_line + height {
            self.viewport_offset.0 = cursor_line.saturating_sub(height - 1);
        }

        let cursor_col = self.cursor.position.column;
        if cursor_col < viewport_col {
            self.viewport_offset.1 = cursor_col;
        } else if cursor_col >= viewport_col + 80 {
            self.viewport_offset.1 = cursor_col.saturating_sub(79);
        }
    }

    pub fn ensure_cursor_visible(&mut self, height: usize) {
        // Force viewport to show cursor (used after cursor movement)
        self.update_viewport(height);
    }

    pub fn toggle_preview_mode(&mut self) {
        // Only allow preview mode for markdown files
        if self.is_markdown() {
            self.preview_mode = !self.preview_mode;
        }
    }

    #[allow(dead_code)]
    pub fn toggle_word_wrap(&mut self) {
        self.word_wrap = !self.word_wrap;
    }

    pub fn is_markdown(&self) -> bool {
        // Check if the file has a markdown extension
        if let Some(path) = &self.path {
            if let Some(ext) = path.extension() {
                return ext == "md" || ext == "markdown";
            }
        }
        // Also check if the tab name ends with .md
        self.name.ends_with(".md") || self.name.ends_with(".markdown")
    }

    pub fn save_state(&mut self) {
        // Save current state before making changes
        let state = EditorState {
            buffer: self.buffer.clone(),
            cursor: self.cursor.clone(),
        };

        // Add to undo stack
        self.undo_stack.push(state);

        // Limit undo history size
        if self.undo_stack.len() > self.max_undo_history {
            self.undo_stack.remove(0);
        }

        // Clear redo stack when new changes are made
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous_state) = self.undo_stack.pop() {
            // Save current state to redo stack
            let current_state = EditorState {
                buffer: self.buffer.clone(),
                cursor: self.cursor.clone(),
            };
            self.redo_stack.push(current_state);

            // Restore previous state
            self.buffer = previous_state.buffer;
            self.cursor = previous_state.cursor;

            // Clear modified flag if we're back to the original state (no more undo history)
            if self.undo_stack.is_empty() {
                self.modified = false;
            }

            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next_state) = self.redo_stack.pop() {
            // Save current state to undo stack
            let current_state = EditorState {
                buffer: self.buffer.clone(),
                cursor: self.cursor.clone(),
            };
            self.undo_stack.push(current_state);

            // Restore next state
            self.buffer = next_state.buffer;
            self.cursor = next_state.cursor;

            // Mark as modified
            self.modified = true;

            true
        } else {
            false
        }
    }

    pub fn start_find(&mut self) {
        self.find_replace_state.active = true;
        self.find_replace_state.is_replace_mode = true;  // Always show replace mode
        self.find_replace_state.find_query.clear();
        self.find_replace_state.replace_query.clear();
        self.find_replace_state.matches.clear();
        self.find_replace_state.current_match_index = None;
        self.find_replace_state.find_cursor_position = 0;
        self.find_replace_state.replace_cursor_position = 0;
        self.find_replace_state.focused_field = FindFocusedField::Find;
    }

    pub fn start_find_replace(&mut self) {
        self.find_replace_state.active = true;
        self.find_replace_state.is_replace_mode = true;
        self.find_replace_state.find_query.clear();
        self.find_replace_state.replace_query.clear();
        self.find_replace_state.matches.clear();
        self.find_replace_state.current_match_index = None;
        self.find_replace_state.find_cursor_position = 0;
        self.find_replace_state.replace_cursor_position = 0;
        self.find_replace_state.focused_field = FindFocusedField::Find;
    }

    pub fn stop_find_replace(&mut self) {
        self.find_replace_state.active = false;
        self.find_replace_state.matches.clear();
        self.find_replace_state.current_match_index = None;
    }


    pub fn perform_find(&mut self) {
        self.find_replace_state.matches.clear();
        self.find_replace_state.current_match_index = None;

        if self.find_replace_state.find_query.is_empty() {
            return;
        }

        let query = if self.find_replace_state.case_sensitive {
            self.find_replace_state.find_query.clone()
        } else {
            self.find_replace_state.find_query.to_lowercase()
        };

        // Search through all lines
        for line_idx in 0..self.buffer.len_lines() {
            let line_text = self.buffer.get_line_text(line_idx);
            let search_text = if self.find_replace_state.case_sensitive {
                line_text.clone()
            } else {
                line_text.to_lowercase()
            };

            // Find all matches in this line
            let mut start = 0;
            while let Some(match_start) = search_text[start..].find(&query) {
                let absolute_start = start + match_start;
                let match_end = absolute_start + query.len();

                // Check whole word constraint if enabled
                if self.find_replace_state.whole_word {
                    let is_word_start = absolute_start == 0 
                        || !search_text.chars().nth(absolute_start.saturating_sub(1))
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    let is_word_end = match_end >= search_text.len()
                        || !search_text.chars().nth(match_end)
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    
                    if is_word_start && is_word_end {
                        self.find_replace_state.matches.push(FindMatch {
                            start: Position::new(line_idx, absolute_start),
                            end: Position::new(line_idx, match_end),
                        });
                    }
                } else {
                    self.find_replace_state.matches.push(FindMatch {
                        start: Position::new(line_idx, absolute_start),
                        end: Position::new(line_idx, match_end),
                    });
                }

                start = match_end;
            }
        }

        // Set current match to the first one after cursor position
        if !self.find_replace_state.matches.is_empty() {
            let cursor_pos = (self.cursor.position.line, self.cursor.position.column);
            for (i, m) in self.find_replace_state.matches.iter().enumerate() {
                if m.start.line > cursor_pos.0 || (m.start.line == cursor_pos.0 && m.start.column >= cursor_pos.1) {
                    self.find_replace_state.current_match_index = Some(i);
                    break;
                }
            }
            // If no match after cursor, wrap to beginning
            if self.find_replace_state.current_match_index.is_none() {
                self.find_replace_state.current_match_index = Some(0);
            }

            // Jump to current match
            self.jump_to_current_match();
        }
    }

    pub fn find_next(&mut self) {
        if self.find_replace_state.matches.is_empty() {
            return;
        }

        let next_index = match self.find_replace_state.current_match_index {
            Some(idx) => (idx + 1) % self.find_replace_state.matches.len(),
            None => 0,
        };

        self.find_replace_state.current_match_index = Some(next_index);
        self.jump_to_current_match();
    }

    pub fn find_prev(&mut self) {
        if self.find_replace_state.matches.is_empty() {
            return;
        }

        let prev_index = match self.find_replace_state.current_match_index {
            Some(0) => self.find_replace_state.matches.len() - 1,
            Some(idx) => idx - 1,
            None => self.find_replace_state.matches.len() - 1,
        };

        self.find_replace_state.current_match_index = Some(prev_index);
        self.jump_to_current_match();
    }

    fn jump_to_current_match(&mut self) {
        if let Some(idx) = self.find_replace_state.current_match_index {
            if let Some(m) = self.find_replace_state.matches.get(idx) {
                self.cursor.position.line = m.start.line;
                self.cursor.position.column = m.start.column;
                self.ensure_cursor_visible(40); // Use reasonable default height
            }
        }
    }

    pub fn replace_current(&mut self) {
        if !self.find_replace_state.is_replace_mode {
            return;
        }

        if let Some(idx) = self.find_replace_state.current_match_index {
            if let Some(m) = self.find_replace_state.matches.get(idx).cloned() {
                // Save state for undo
                self.save_state();

                // Get the line text
                let line_text = self.buffer.get_line_text(m.start.line);
                
                // Perform replacement
                let mut new_line = String::new();
                new_line.push_str(&line_text[..m.start.column]);
                new_line.push_str(&self.find_replace_state.replace_query);
                new_line.push_str(&line_text[m.end.column..]);

                // Update the buffer
                self.buffer.replace_line(m.start.line, &new_line);
                self.mark_modified();

                // Re-perform find to update matches
                self.perform_find();
            }
        }
    }

    pub fn replace_all(&mut self) {
        if !self.find_replace_state.is_replace_mode {
            return;
        }

        if self.find_replace_state.matches.is_empty() {
            return;
        }

        // Save state for undo
        self.save_state();

        // Process replacements from bottom to top to maintain correct indices
        let mut matches = self.find_replace_state.matches.clone();
        matches.reverse();

        for m in matches {
            let line_text = self.buffer.get_line_text(m.start.line);
            
            let mut new_line = String::new();
            new_line.push_str(&line_text[..m.start.column]);
            new_line.push_str(&self.find_replace_state.replace_query);
            new_line.push_str(&line_text[m.end.column..]);

            self.buffer.replace_line(m.start.line, &new_line);
        }

        self.mark_modified();
        
        // Clear matches after replace all
        self.find_replace_state.matches.clear();
        self.find_replace_state.current_match_index = None;
    }

    #[allow(dead_code)]
    fn _toggle_case_sensitive(&mut self) {
        self.find_replace_state.case_sensitive = !self.find_replace_state.case_sensitive;
        self.perform_find();
    }

    #[allow(dead_code)]
    fn _toggle_whole_word(&mut self) {
        self.find_replace_state.whole_word = !self.find_replace_state.whole_word;
        self.perform_find();
    }
}

pub struct TabManager {
    pub tabs: Vec<Tab>,
    active_index: usize,
}

impl TabManager {
    pub fn new() -> Self {
        let mut manager = Self {
            tabs: Vec::new(),
            active_index: 0,
        };
        manager.add_tab(Tab::new("untitled".to_string()));
        manager
    }

    pub fn add_tab(&mut self, tab: Tab) {
        // Check if a tab with the same path already exists
        if let Some(ref path) = tab.path {
            for (index, existing_tab) in self.tabs.iter().enumerate() {
                if let Some(ref existing_path) = existing_tab.path {
                    if existing_path == path {
                        // Switch to existing tab instead of creating a new one
                        self.active_index = index;
                        return;
                    }
                }
            }
        }
        // No existing tab found, add the new one
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
    }
    
    #[allow(dead_code)]
    pub fn add_or_switch_to_tab(&mut self, tab: Tab) {
        // Check if a tab with the same path already exists
        if let Some(ref path) = tab.path {
            for (index, existing_tab) in self.tabs.iter().enumerate() {
                if let Some(ref existing_path) = existing_tab.path {
                    if existing_path == path {
                        // Switch to existing tab instead of creating a new one
                        self.active_index = index;
                        return;
                    }
                }
            }
        }
        // No existing tab found, add the new one
        self.add_tab(tab);
    }

    pub fn close_tab(&mut self, index: usize) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active_index >= self.tabs.len() {
                self.active_index = self.tabs.len() - 1;
            }
            true
        } else {
            false
        }
    }

    pub fn close_current_tab(&mut self) -> bool {
        self.close_tab(self.active_index)
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_index == 0 {
                self.active_index = self.tabs.len() - 1;
            } else {
                self.active_index -= 1;
            }
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_index)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_index)
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn set_active_index(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = index;
            // Ensure viewport shows cursor when switching tabs
            if let Some(tab) = self.active_tab_mut() {
                tab.ensure_cursor_visible(40); // Use a reasonable default height
            }
        }
    }

    pub fn close_other_tabs(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        // Keep only the active tab
        let active_tab = self.tabs.remove(self.active_index);
        self.tabs.clear();
        self.tabs.push(active_tab);
        self.active_index = 0;
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn reorder_tab(&mut self, from_index: usize, to_index: usize) {
        if from_index >= self.tabs.len() || to_index >= self.tabs.len() {
            return;
        }

        if from_index == to_index {
            return;
        }

        let tab = self.tabs.remove(from_index);
        self.tabs.insert(to_index, tab);

        // Update active_index if needed
        if self.active_index == from_index {
            self.active_index = to_index;
        } else if from_index < to_index {
            // Tab moved forward, adjust indices in between
            if self.active_index > from_index && self.active_index <= to_index {
                self.active_index -= 1;
            }
        } else {
            // Tab moved backward, adjust indices in between
            if self.active_index >= to_index && self.active_index < from_index {
                self.active_index += 1;
            }
        }
    }
}
