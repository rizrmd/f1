use crate::{cursor::Cursor, rope_buffer::RopeBuffer};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Tab {
    pub name: String,
    pub path: Option<PathBuf>,
    pub buffer: RopeBuffer,
    pub cursor: Cursor,
    pub viewport_offset: (usize, usize),
    pub modified: bool,
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
        }
    }

    pub fn from_file(path: PathBuf, content: &str) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();
        
        Self {
            name,
            path: Some(path),
            buffer: RopeBuffer::from_str(content),
            cursor: Cursor::new(),
            viewport_offset: (0, 0),
            modified: false,
        }
    }

    pub fn display_name(&self) -> String {
        if self.modified {
            format!("{}*", self.name)
        } else {
            self.name.clone()
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