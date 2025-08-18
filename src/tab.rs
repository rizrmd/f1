use crate::{cursor::Cursor, rope_buffer::RopeBuffer};
use std::path::PathBuf;

#[derive(Clone)]
struct EditorState {
    buffer: RopeBuffer,
    cursor: Cursor,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 100,
        }
    }

    pub fn display_name(&self) -> String {
        let mut name = self.name.clone();
        if self.preview_mode {
            name = format!("[Preview] {}", name);
        }
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
}