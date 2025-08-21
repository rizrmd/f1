/// Tab operations module - consolidates all tab management functionality
use crate::app::{App, FocusMode};
use crate::keyboard::EditorCommand;
use crate::tab::Tab;
use std::path::PathBuf;

#[allow(dead_code)]
impl App {
    /// Create a new untitled tab
    pub fn create_new_tab(&mut self) {
        let mut new_tab = Tab::new(format!("untitled-{}", self.tab_manager.len() + 1));
        if let Tab::Editor { word_wrap, .. } = &mut new_tab {
            *word_wrap = self.global_word_wrap;
        }
        self.tab_manager.add_tab(new_tab);
        self.expand_tree_to_current_file();
        // Focus the editor after creating new tab
        self.focus_mode = FocusMode::Editor;
        if let Some(tree_view) = &mut self.tree_view {
            tree_view.is_focused = false;
        }
    }

    /// Create a new terminal tab
    pub fn create_new_terminal(&mut self) {
        let terminal_tab = Tab::new_terminal();
        self.tab_manager.add_tab(terminal_tab);
        // Focus the terminal after creating it
        self.focus_mode = FocusMode::Editor;
        if let Some(tree_view) = &mut self.tree_view {
            tree_view.is_focused = false;
        }
    }

    /// Open a file in a new tab
    pub fn open_file_in_tab(&mut self, path: PathBuf, content: &str) {
        let mut new_tab = Tab::from_file(path, content);
        if let Tab::Editor { word_wrap, .. } = &mut new_tab {
            *word_wrap = self.global_word_wrap;
        }
        self.tab_manager.add_tab(new_tab);
        self.expand_tree_to_current_file();
        self.handle_command(EditorCommand::FocusEditor);
    }

    /// Switch to the next tab
    pub fn switch_next_tab(&mut self) {
        self.tab_manager.next_tab();
        self.expand_tree_to_current_file();
    }

    /// Switch to the previous tab
    pub fn switch_prev_tab(&mut self) {
        self.tab_manager.prev_tab();
        self.expand_tree_to_current_file();
    }

    /// Close the current tab with confirmation if modified
    pub fn close_current_tab_with_confirmation(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab() {
            let (is_modified, tab_name) = match tab {
                Tab::Editor { modified, name, .. } => (*modified, name.as_str()),
                Tab::Terminal { modified, name, .. } => (*modified, name.as_str()),
            };
            if is_modified {
                // Show warning for unsaved changes
                self.warning_message = Some(format!(
                    "Tab '{}' has unsaved changes. Close anyway?",
                    tab_name
                ));
                self.pending_close = true;
                self.warning_selected_button = 0; // Default to "No"
                return;
            }
        }

        // No unsaved changes, close directly
        if !self.tab_manager.close_current_tab() {
            self.running = false;
        }
    }

    /// Close all tabs except current one
    pub fn close_other_tabs(&mut self) {
        self.tab_manager.close_other_tabs();
    }

    /// Check if quitting should show unsaved changes warning
    pub fn check_unsaved_on_quit(&mut self) -> bool {
        let modified_tabs: Vec<String> = self
            .tab_manager
            .tabs()
            .iter()
            .filter(|tab| match tab {
                Tab::Editor { modified, .. } => *modified,
                Tab::Terminal { modified, .. } => *modified,
            })
            .map(|tab| match tab {
                Tab::Editor { name, .. } => name.clone(),
                Tab::Terminal { name, .. } => name.clone(),
            })
            .collect();

        if !modified_tabs.is_empty() {
            // Show warning for unsaved changes
            let message = if modified_tabs.len() == 1 {
                format!(
                    "Tab '{}' has unsaved changes. Quit anyway?",
                    modified_tabs[0]
                )
            } else {
                format!(
                    "{} tabs have unsaved changes. Quit anyway?",
                    modified_tabs.len()
                )
            };

            self.warning_message = Some(message);
            self.pending_quit = true;
            self.warning_selected_button = 0; // Default to "No"
            return true;
        }

        false
    }

    /// Toggle preview mode for markdown files
    pub fn toggle_preview_mode(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.toggle_preview_mode();
        }
    }

    /// Apply word wrap setting to all tabs
    pub fn apply_word_wrap_to_all_tabs(&mut self) {
        for tab in &mut self.tab_manager.tabs {
            if let Tab::Editor { word_wrap, .. } = tab {
                *word_wrap = self.global_word_wrap;
            }
        }
    }

    /// Update viewport for current tab
    pub fn update_current_tab_viewport(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let visible_height = self.terminal_size.1.saturating_sub(2) as usize;
            tab.update_viewport(visible_height);
        }
    }

    /// Ensure cursor is visible in current tab
    pub fn ensure_cursor_visible(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.ensure_cursor_visible(self.terminal_size.1.saturating_sub(2) as usize);
        }
    }

    /// Page up in current tab
    pub fn page_up(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            if let Tab::Editor { viewport_offset, .. } = tab {
                let page_size = self.terminal_size.1.saturating_sub(4) as usize;
                viewport_offset.0 = viewport_offset.0.saturating_sub(page_size);
            }
        }
    }

    /// Page down in current tab
    pub fn page_down(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            if let Tab::Editor { viewport_offset, .. } = tab {
                let page_size = self.terminal_size.1.saturating_sub(4) as usize;
                viewport_offset.0 += page_size;
            }
        }
    }

    /// Handle tab-related commands
    pub fn handle_tab_specific_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::NewTab => self.create_new_tab(),
            EditorCommand::NewTerminal => self.create_new_terminal(),
            EditorCommand::CloseTab => self.close_current_tab_with_confirmation(),
            EditorCommand::NextTab => self.switch_next_tab(),
            EditorCommand::PrevTab => self.switch_prev_tab(),
            EditorCommand::PageUp => self.page_up(),
            EditorCommand::PageDown => self.page_down(),
            EditorCommand::TogglePreview => self.toggle_preview_mode(),
            EditorCommand::ToggleWordWrap => {
                self.global_word_wrap = !self.global_word_wrap;
                self.apply_word_wrap_to_all_tabs();
            }
            _ => {}
        }
    }
}