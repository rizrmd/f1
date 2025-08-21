use crate::app::{App, FocusMode};
use crate::keyboard::EditorCommand;
use crate::tab::Tab;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[allow(dead_code)]
const TAB_WIDTH: usize = 14;

#[allow(dead_code)]
impl App {
    /// Handle mouse events on the tab bar
    pub fn handle_tab_bar_mouse(&mut self, mouse: MouseEvent, active_index: usize) -> bool {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row == 0 {
                    // Check if click is on Ctrl+N hint to create new tab
                    if self.is_ctrl_n_hint_clicked(mouse.column) {
                        self.create_new_tab_from_hint();
                        return true;
                    }

                    // Check if click is on a tab
                    if let Some(clicked_tab) = self.get_clicked_tab(mouse.column) {
                        self.handle_tab_click(clicked_tab, mouse.column, active_index);
                        return true;
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(dragging_idx) = self.dragging_tab {
                    if mouse.row == 0 {
                        self.handle_tab_drag(dragging_idx, mouse.column);
                    }
                    return true;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.dragging_tab.is_some() && mouse.row == 0 {
                    self.handle_tab_drop(mouse.column);
                }
                self.dragging_tab = None;
                self.tab_was_active_on_click = false;
            }
            _ => {}
        }
        false
    }

    /// Create a new tab from the Ctrl+N hint click
    fn create_new_tab_from_hint(&mut self) {
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

    /// Handle clicking on a tab
    fn handle_tab_click(&mut self, clicked_tab: usize, mouse_column: u16, active_index: usize) {
        // Store potential drag information
        self.dragging_tab = Some(clicked_tab);
        self.drag_start_x = mouse_column;

        // Remember if this tab was already active
        self.tab_was_active_on_click = clicked_tab == active_index;

        // Switch to the clicked tab if different
        if clicked_tab != active_index {
            self.tab_manager.set_active_index(clicked_tab);
            self.expand_tree_to_current_file();
        }
    }

    /// Handle dragging a tab
    fn handle_tab_drag(&mut self, dragging_idx: usize, mouse_column: u16) {
        // Close menu when dragging starts
        if matches!(
            self.menu_system.state,
            crate::menu::MenuState::CurrentTabMenu(_)
        ) {
            self.menu_system.close();
        }

        // Calculate which tab position we're hovering over
        if let Some(hover_tab) = self.get_clicked_tab(mouse_column) {
            if hover_tab != dragging_idx {
                // Reorder the tabs
                self.tab_manager.reorder_tab(dragging_idx, hover_tab);
                // Update the dragging index to the new position
                self.dragging_tab = Some(hover_tab);
            }
        }
    }

    /// Handle dropping a tab (mouse up after drag)
    fn handle_tab_drop(&mut self, mouse_column: u16) {
        // Check if this was a click on active tab (no drag occurred)
        if mouse_column.abs_diff(self.drag_start_x) <= 2 {
            // Only toggle menu if the tab was already active when we clicked it
            if self.tab_was_active_on_click {
                // Toggle current tab menu
                if matches!(
                    self.menu_system.state,
                    crate::menu::MenuState::CurrentTabMenu(_)
                ) {
                    self.menu_system.close();
                } else {
                    self.menu_system.open_current_tab_menu();
                }
            }
        }
    }

    /// Get which tab was clicked based on mouse X position
    pub fn get_clicked_tab(&self, mouse_x: u16) -> Option<usize> {
        let available_width = self.terminal_size.0 as usize;
        let hint_text = "  Ctrl+N";
        let hint_width = hint_text.len();
        let tabs_width = available_width.saturating_sub(hint_width);

        let tabs = self.tab_manager.tabs();
        let tab_count = tabs.len();

        if tab_count == 0 {
            return None;
        }

        let max_tabs_that_fit = tabs_width / TAB_WIDTH;

        if tab_count <= max_tabs_that_fit {
            // All tabs are visible with fixed width
            let tab_index = (mouse_x as usize) / TAB_WIDTH;
            if tab_index < tab_count {
                return Some(tab_index);
            }
        } else {
            // Too many tabs, showing subset with scrolling
            let active_index = self.tab_manager.active_index();
            let half_width = max_tabs_that_fit / 2;

            let start_index = if active_index >= half_width {
                (active_index - half_width).min(tab_count.saturating_sub(max_tabs_that_fit))
            } else {
                0
            };
            let end_index = (start_index + max_tabs_that_fit).min(tab_count);

            let mut current_x = 0u16;

            // Account for left truncation indicator
            if start_index > 0 {
                if mouse_x < 3 {
                    return None; // Clicked on « indicator
                }
                current_x = 3;
            }

            // Check visible tabs
            for i in start_index..end_index {
                if mouse_x >= current_x && mouse_x < current_x + TAB_WIDTH as u16 {
                    return Some(i);
                }
                current_x += TAB_WIDTH as u16;
            }
        }

        None
    }

    /// Get the X position of a tab for menu positioning
    pub fn get_tab_x_position_for_menu(&self, target_tab_index: usize) -> u16 {
        let available_width = self.terminal_size.0 as usize;
        self.ui
            .tab_bar
            .get_tab_x_position(&self.tab_manager, target_tab_index, available_width)
    }

    /// Check if the Ctrl+N hint was clicked
    pub fn is_ctrl_n_hint_clicked(&self, mouse_x: u16) -> bool {
        let available_width = self.terminal_size.0 as usize;
        let hint_text = "  Ctrl+N";
        let hint_width = hint_text.len();
        let tabs_width = available_width.saturating_sub(hint_width);

        let tab_count = self.tab_manager.tabs().len();
        if tab_count == 0 {
            // If no tabs, hint starts at x=0
            return mouse_x < hint_width as u16;
        }

        let max_tabs_that_fit = tabs_width / TAB_WIDTH;

        // Calculate where all tabs end
        let tabs_total_width = if tab_count <= max_tabs_that_fit {
            // All tabs visible with fixed width
            tab_count * TAB_WIDTH
        } else {
            // Showing subset with indicators
            let active_index = self.tab_manager.active_index();
            let half_width = max_tabs_that_fit / 2;

            let start_index = if active_index >= half_width {
                (active_index - half_width).min(tab_count.saturating_sub(max_tabs_that_fit))
            } else {
                0
            };
            let end_index = (start_index + max_tabs_that_fit).min(tab_count);

            let mut width = 0;
            if start_index > 0 {
                width += 3; // " « "
            }
            width += (end_index - start_index) * TAB_WIDTH;
            if end_index < tab_count {
                width += 3; // " » "
            }
            width
        };

        // The hint starts right after the tabs
        let hint_start_x = tabs_total_width as u16;
        let hint_end_x = hint_start_x + hint_width as u16;

        mouse_x >= hint_start_x && mouse_x < hint_end_x
    }

    /// Handle keyboard commands related to tabs
    pub fn handle_tab_bar_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::NewTab => {
                self.create_new_tab_from_hint();
            }
            EditorCommand::NewTerminal => {
                let terminal_tab = Tab::new_terminal();
                self.tab_manager.add_tab(terminal_tab);
                // Focus the terminal after creating it
                self.focus_mode = FocusMode::Editor;
                if let Some(tree_view) = &mut self.tree_view {
                    tree_view.is_focused = false;
                }
            }
            EditorCommand::CloseTab => {
                self.handle_close_tab();
            }
            EditorCommand::NextTab => {
                self.tab_manager.next_tab();
                self.expand_tree_to_current_file();
            }
            EditorCommand::PrevTab => {
                self.tab_manager.prev_tab();
                self.expand_tree_to_current_file();
            }
            _ => {}
        }
    }
}