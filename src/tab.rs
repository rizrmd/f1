use crate::{
    cursor::{Cursor, Position},
    rope_buffer::RopeBuffer,
    terminal_widget::TerminalWidget
};
use ratatui::layout::Rect;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FindMatch {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone)]
pub struct EditorState {
    pub buffer: RopeBuffer,
    pub cursor: Cursor,
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

pub enum Tab {
    Editor {
        name: String,
        path: Option<PathBuf>,
        buffer: RopeBuffer,
        cursor: Cursor,
        viewport_offset: (usize, usize),
        modified: bool,
        preview_mode: bool,
        word_wrap: bool,
        find_replace_state: FindReplaceState,
        undo_stack: Vec<EditorState>,
        redo_stack: Vec<EditorState>,
        max_undo_history: usize,
    },
    Terminal {
        name: String,
        terminal: TerminalWidget,
        #[allow(dead_code)]
        viewport_offset: (usize, usize),
        modified: bool,
    },
}

impl Tab {
    pub fn new(name: String) -> Self {
        Tab::Editor {
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

        let is_markdown = if let Some(ext) = path.extension() {
            ext == "md" || ext == "markdown"
        } else {
            name.ends_with(".md") || name.ends_with(".markdown")
        };

        Tab::Editor {
            name,
            path: Some(path),
            buffer: RopeBuffer::from_str(content),
            cursor: Cursor::new(),
            viewport_offset: (0, 0),
            modified: false,
            preview_mode: is_markdown,
            word_wrap: false,
            find_replace_state: FindReplaceState::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 100,
        }
    }

    pub fn new_terminal() -> Self {
        Tab::Terminal {
            name: "Terminal".to_string(),
            terminal: TerminalWidget::new(Rect::new(0, 0, 80, 24)).unwrap(),
            viewport_offset: (0, 0),
            modified: false,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Tab::Editor { name, modified, .. } => if *modified { format!("{}*", name) } else { name.clone() },
            Tab::Terminal { name, modified, .. } => if *modified { format!("{}*", name) } else { name.clone() },
        }
    }

    pub fn mark_modified(&mut self) {
        match self {
            Tab::Editor { modified, .. } => *modified = true,
            Tab::Terminal { modified, .. } => *modified = true,
        }
    }

    pub fn mark_saved(&mut self) {
        match self {
            Tab::Editor { modified, .. } => *modified = false,
            Tab::Terminal { modified, .. } => *modified = false,
        }
    }

    pub fn update_viewport(&mut self, height: usize) {
        match self {
            Tab::Editor { cursor, viewport_offset, .. } => {
                let cursor_line = cursor.position.line;
                let (viewport_line, viewport_col) = *viewport_offset;

                if cursor_line < viewport_line {
                    viewport_offset.0 = cursor_line;
                } else if cursor_line >= viewport_line + height {
                    viewport_offset.0 = cursor_line.saturating_sub(height - 1);
                }

                let cursor_col = cursor.position.column;
                if cursor_col < viewport_col {
                    viewport_offset.1 = cursor_col;
                } else if cursor_col >= viewport_col + 80 {
                    viewport_offset.1 = cursor_col.saturating_sub(79);
                }
            }
            Tab::Terminal { .. } => {
                // Similar logic for terminal
                // For now, stub
            }
        }
    }

    pub fn ensure_cursor_visible(&mut self, height: usize) {
        self.update_viewport(height);
    }

    pub fn toggle_preview_mode(&mut self) {
        let is_markdown = self.is_markdown();
        if let Tab::Editor { preview_mode, .. } = self {
            if is_markdown {
                *preview_mode = !*preview_mode;
            }
        }
    }

    #[allow(dead_code)]
    pub fn toggle_word_wrap(&mut self) {
        if let Tab::Editor { word_wrap, .. } = self {
            *word_wrap = !*word_wrap;
        }
    }

    pub fn is_markdown(&self) -> bool {
        match self {
            Tab::Editor { path, name, .. } => {
                if let Some(p) = path {
                    if let Some(ext) = p.extension() {
                        return ext == "md" || ext == "markdown";
                    }
                }
                name.ends_with(".md") || name.ends_with(".markdown")
            }
            Tab::Terminal { .. } => false,
        }
    }

    pub fn save_state(&mut self) {
        if let Tab::Editor { buffer, cursor, undo_stack, max_undo_history, redo_stack, .. } = self {
            let state = EditorState {
                buffer: buffer.clone(),
                cursor: cursor.clone(),
            };
            undo_stack.push(state);
            if undo_stack.len() > *max_undo_history {
                undo_stack.remove(0);
            }
            redo_stack.clear();
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Tab::Editor { buffer, cursor, undo_stack, redo_stack, modified, .. } = self {
            if let Some(previous_state) = undo_stack.pop() {
                let current_state = EditorState {
                    buffer: buffer.clone(),
                    cursor: cursor.clone(),
                };
                redo_stack.push(current_state);
                *buffer = previous_state.buffer;
                *cursor = previous_state.cursor;
                if undo_stack.is_empty() {
                    *modified = false;
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Tab::Editor { buffer, cursor, undo_stack, redo_stack, modified, .. } = self {
            if let Some(next_state) = redo_stack.pop() {
                let current_state = EditorState {
                    buffer: buffer.clone(),
                    cursor: cursor.clone(),
                };
                undo_stack.push(current_state);
                *buffer = next_state.buffer;
                *cursor = next_state.cursor;
                *modified = true;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn start_find(&mut self) {
        if let Tab::Editor { find_replace_state, .. } = self {
            find_replace_state.active = true;
            find_replace_state.is_replace_mode = true;
            find_replace_state.find_query.clear();
            find_replace_state.replace_query.clear();
            find_replace_state.matches.clear();
            find_replace_state.current_match_index = None;
            find_replace_state.find_cursor_position = 0;
            find_replace_state.replace_cursor_position = 0;
            find_replace_state.focused_field = FindFocusedField::Find;
        }
    }

    pub fn start_find_replace(&mut self) {
        self.start_find();
    }

    pub fn stop_find_replace(&mut self) {
        if let Tab::Editor { find_replace_state, .. } = self {
            find_replace_state.active = false;
            find_replace_state.matches.clear();
            find_replace_state.current_match_index = None;
        }
    }

    pub fn perform_find(&mut self) {
        if let Tab::Editor { find_replace_state, buffer, cursor, .. } = self {
            find_replace_state.matches.clear();
            find_replace_state.current_match_index = None;

            if find_replace_state.find_query.is_empty() {
                return;
            }

            let query = if find_replace_state.case_sensitive {
                find_replace_state.find_query.clone()
            } else {
                find_replace_state.find_query.to_lowercase()
            };

            for line_idx in 0..buffer.len_lines() {
                let line_text = buffer.get_line_text(line_idx);
                let search_text = if find_replace_state.case_sensitive {
                    line_text.clone()
                } else {
                    line_text.to_lowercase()
                };

                let mut start = 0;
                while let Some(match_start) = search_text[start..].find(&query) {
                    let absolute_start = start + match_start;
                    let match_end = absolute_start + query.len();

                    if find_replace_state.whole_word {
                        let is_word_start = absolute_start == 0
                            || !search_text
                                .chars()
                                .nth(absolute_start.saturating_sub(1))
                                .is_some_and(|c| c.is_alphanumeric() || c == '_');
                        let is_word_end = match_end >= search_text.len()
                            || !search_text
                                .chars()
                                .nth(match_end)
                                .is_some_and(|c| c.is_alphanumeric() || c == '_');

                        if is_word_start && is_word_end {
                            find_replace_state.matches.push(FindMatch {
                                start: Position::new(line_idx, absolute_start),
                                end: Position::new(line_idx, match_end),
                            });
                        }
                    } else {
                        find_replace_state.matches.push(FindMatch {
                            start: Position::new(line_idx, absolute_start),
                            end: Position::new(line_idx, match_end),
                        });
                    }

                    start = match_end;
                }
            }

            if !find_replace_state.matches.is_empty() {
                let cursor_pos = (cursor.position.line, cursor.position.column);
                for (i, m) in find_replace_state.matches.iter().enumerate() {
                    if m.start.line > cursor_pos.0
                        || (m.start.line == cursor_pos.0 && m.start.column >= cursor_pos.1)
                    {
                        find_replace_state.current_match_index = Some(i);
                        break;
                    }
                }
                if find_replace_state.current_match_index.is_none() {
                    find_replace_state.current_match_index = Some(0);
                }

                self.jump_to_current_match();
            }
        }
    }

    pub fn find_next(&mut self) {
        if let Tab::Editor { find_replace_state, .. } = self {
            if find_replace_state.matches.is_empty() {
                return;
            }

            let next_index = match find_replace_state.current_match_index {
                Some(idx) => (idx + 1) % find_replace_state.matches.len(),
                None => 0,
            };

            find_replace_state.current_match_index = Some(next_index);
            self.jump_to_current_match();
        }
    }

    pub fn find_prev(&mut self) {
        if let Tab::Editor { find_replace_state, .. } = self {
            if find_replace_state.matches.is_empty() {
                return;
            }

            let prev_index = match find_replace_state.current_match_index {
                Some(0) => find_replace_state.matches.len() - 1,
                Some(idx) => idx - 1,
                None => find_replace_state.matches.len() - 1,
            };

            find_replace_state.current_match_index = Some(prev_index);
            self.jump_to_current_match();
        }
    }

    fn jump_to_current_match(&mut self) {
        if let Tab::Editor { find_replace_state, cursor, .. } = self {
            if let Some(idx) = find_replace_state.current_match_index {
                if let Some(m) = find_replace_state.matches.get(idx) {
                    cursor.position.line = m.start.line;
                    cursor.position.column = m.start.column;
                    self.ensure_cursor_visible(40);
                }
            }
        }
    }

    pub fn replace_current(&mut self) {
        // First check if this is a valid operation
        let (should_replace, match_info, replace_query) = match self {
            Tab::Editor { find_replace_state, .. } => {
                if !find_replace_state.is_replace_mode {
                    return;
                }
                
                if let Some(idx) = find_replace_state.current_match_index {
                    if let Some(m) = find_replace_state.matches.get(idx) {
                        (true, m.clone(), find_replace_state.replace_query.clone())
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
            Tab::Terminal { .. } => return
        };

        if should_replace {
            self.save_state();
            
            if let Tab::Editor { buffer, .. } = self {
                let line_text = buffer.get_line_text(match_info.start.line);

                let mut new_line = String::new();
                new_line.push_str(&line_text[..match_info.start.column]);
                new_line.push_str(&replace_query);
                new_line.push_str(&line_text[match_info.end.column..]);

                buffer.replace_line(match_info.start.line, &new_line);
            }
            
            self.mark_modified();
            self.perform_find();
        }
    }

    pub fn replace_all(&mut self) {
        // First extract the data we need
        let (should_replace, matches, replace_query) = match self {
            Tab::Editor { find_replace_state, .. } => {
                if !find_replace_state.is_replace_mode || find_replace_state.matches.is_empty() {
                    return;
                }
                
                let mut matches = find_replace_state.matches.clone();
                matches.reverse();
                (true, matches, find_replace_state.replace_query.clone())
            }
            Tab::Terminal { .. } => return
        };

        if should_replace {
            self.save_state();

            if let Tab::Editor { buffer, .. } = self {
                for m in matches {
                    let line_text = buffer.get_line_text(m.start.line);

                    let mut new_line = String::new();
                    new_line.push_str(&line_text[..m.start.column]);
                    new_line.push_str(&replace_query);
                    new_line.push_str(&line_text[m.end.column..]);

                    buffer.replace_line(m.start.line, &new_line);
                }
            }

            self.mark_modified();

            if let Tab::Editor { find_replace_state, .. } = self {
                find_replace_state.matches.clear();
                find_replace_state.current_match_index = None;
            }
        }
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
        if let Some(ref path) = tab.path() {
            for (index, existing_tab) in self.tabs.iter().enumerate() {
                if let Some(ref existing_path) = existing_tab.path() {
                    if existing_path == path {
                        self.active_index = index;
                        return;
                    }
                }
            }
        }
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
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
            if let Some(tab) = self.active_tab_mut() {
                tab.ensure_cursor_visible(40);
            }
        }
    }

    pub fn close_other_tabs(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

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

        if self.active_index == from_index {
            self.active_index = to_index;
        } else if from_index < to_index {
            if self.active_index > from_index && self.active_index <= to_index {
                self.active_index -= 1;
            }
        } else {
            if self.active_index >= to_index && self.active_index < from_index {
                self.active_index += 1;
            }
        }
    }
}

// Add path method to Tab
impl Tab {
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Tab::Editor { path, .. } => path.as_ref(),
            Tab::Terminal { .. } => None,
        }
    }
}
