use crate::app::App;
use crate::tab::Tab;
use crossterm::event::MouseEvent;

impl App {
    pub fn handle_mouse_on_editor(&mut self, mouse: MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};

        // Get the active tab index to avoid borrowing conflicts
        let active_index = self.tab_manager.active_index();

        // Check if interaction is on scrollbar (rightmost column in editor area)
        if let Some(tab) = self.tab_manager.active_tab() {
            let content_lines = match tab {
                Tab::Editor { preview_mode, buffer, .. } => {
                    if *preview_mode && tab.is_markdown() {
                        // For markdown preview, count the rendered lines
                        let content = buffer.to_string();
                        let markdown_widget = crate::markdown_widget::MarkdownWidget::new(&content);
                        markdown_widget.parse_markdown().len()
                    } else {
                        // For normal editor, use buffer lines
                        buffer.len_lines()
                    }
                }
                Tab::Terminal { .. } => 0, // Terminal doesn't have scrollable content in this context
            };

            let has_scrollbar = content_lines > (self.terminal_size.1 as usize).saturating_sub(2);
            if has_scrollbar
                && mouse.column == self.terminal_size.0.saturating_sub(1)
                && mouse.row > 0
                && (mouse.row as usize) < (self.terminal_size.1 as usize).saturating_sub(1)
            {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.scrollbar_dragging = true;
                        self.handle_scrollbar_click(mouse);
                        return;
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if self.scrollbar_dragging {
                            self.handle_scrollbar_click(mouse);
                            return;
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if self.scrollbar_dragging {
                            self.scrollbar_dragging = false;
                            return;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Handle editor scrolling
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.handle_editor_scroll(MouseEventKind::ScrollUp);
                return;
            }
            MouseEventKind::ScrollDown => {
                self.handle_editor_scroll(MouseEventKind::ScrollDown);
                return;
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // First get the text position without borrowing tab_manager mutably
                let text_position = if let Some(tab) = self.tab_manager.active_tab() {
                    if let Tab::Editor { buffer, .. } = tab {
                        self.mouse_to_text_position(mouse, buffer)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Now handle the click with the computed position
                if let (Some((line, col)), Some(tab)) = (text_position, self.tab_manager.active_tab_mut()) {
                    if let Tab::Editor { cursor, buffer, .. } = tab {
                        cursor.move_to(line, col);
                        cursor.clear_selection();
                        self.mouse_selecting = true;
                        
                        // Track click for double-click detection
                        let now = std::time::Instant::now();
                        let click_pos = (mouse.column, mouse.row);
                        
                        let is_double_click = if let (Some(last_time), Some(last_pos)) = 
                            (self.last_click_time, self.last_click_pos) {
                            now.duration_since(last_time).as_millis() < 500 &&
                            last_pos == click_pos
                        } else {
                            false
                        };
                        
                        if is_double_click {
                            // Double-click: select word
                            cursor.select_word(buffer);
                            self.last_click_time = None; // Prevent triple-click
                        } else {
                            self.last_click_time = Some(now);
                            self.last_click_pos = Some(click_pos);
                        }
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.mouse_selecting {
                    // First get the text position without borrowing tab_manager mutably
                    let text_position = if let Some(tab) = self.tab_manager.active_tab() {
                        if let Tab::Editor { buffer, .. } = tab {
                            self.mouse_to_text_position(mouse, buffer)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Now handle the drag with the computed position
                    if let (Some((line, col)), Some(tab)) = (text_position, self.tab_manager.active_tab_mut()) {
                        if let Tab::Editor { cursor, .. } = tab {
                            cursor.extend_selection_to(line, col);
                        }
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_selecting = false;
            }
            _ => {}
        }
    }

    pub fn mouse_to_text_position(
        &self,
        mouse: MouseEvent,
        buffer: &crate::rope_buffer::RopeBuffer,
    ) -> Option<(usize, usize)> {
        // Adjust for UI elements (skip status bar at bottom)
        if mouse.row == 0 || (mouse.row as usize) >= (self.terminal_size.1 as usize).saturating_sub(1) {
            return None;
        }

        let editor_row = mouse.row.saturating_sub(1) as usize; // Skip tab bar at top
        let editor_col = mouse.column as usize;

        // Get viewport offset from current tab
        let viewport_offset = if let Some(tab) = self.tab_manager.active_tab() {
            match tab {
                Tab::Editor { viewport_offset, .. } => *viewport_offset,
                Tab::Terminal { .. } => (0, 0),
            }
        } else {
            (0, 0)
        };

        let line_index = editor_row + viewport_offset.0;
        
        if line_index >= buffer.len_lines() {
            // Click below content - position at end of last line
            let last_line = buffer.len_lines().saturating_sub(1);
            let line_content = buffer.get_line(last_line);
            return Some((last_line, line_content.chars().count()));
        }

        let line_content = buffer.get_line(line_index);
        let line_chars: Vec<char> = line_content.chars().collect();
        
        // Handle clicks beyond line content
        let col_index = if editor_col >= line_chars.len() {
            line_chars.len()
        } else {
            editor_col
        };

        Some((line_index, col_index))
    }

    pub fn handle_mouse_on_dialog(&mut self, mouse: MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};
        
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            if let Some(button_index) = self.get_dialog_button_at_position(mouse.column, mouse.row) {
                match button_index {
                    0 => {
                        // "No" or "OK" button
                        if self.warning_is_info {
                            // Info dialog - just close
                            self.warning_message = None;
                        } else {
                            // Confirmation dialog - cancel action
                            self.warning_message = None;
                            self.pending_delete_path = None;
                        }
                    }
                    1 => {
                        // "Yes" button - proceed with action
                        if let Some(delete_path) = self.pending_delete_path.take() {
                            let result = if delete_path.is_dir() {
                                std::fs::remove_dir_all(&delete_path)
                                    .map(|_| format!("Deleted directory: {}", delete_path.display()))
                            } else {
                                std::fs::remove_file(&delete_path)
                                    .map(|_| format!("Deleted file: {}", delete_path.display()))
                            };

                            match result {
                                Ok(message) => {
                                    self.set_status_message(message, std::time::Duration::from_secs(3));
                                    // Refresh tree view
                                    if let Some(tree_view) = &mut self.tree_view {
                                        tree_view.refresh();
                                    }
                                }
                                Err(e) => {
                                    self.set_status_message(
                                        format!("Delete failed: {}", e),
                                        std::time::Duration::from_secs(5),
                                    );
                                }
                            }
                        }
                        self.warning_message = None;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn get_dialog_button_at_position(&self, mouse_x: u16, mouse_y: u16) -> Option<usize> {
        if self.warning_message.is_none() {
            return None;
        }

        let modal_width = 50u16.min(self.terminal_size.0.saturating_sub(4));
        let modal_height = if self.warning_is_info { 6 } else { 7 };

        let modal_x = (self.terminal_size.0.saturating_sub(modal_width)) / 2;
        let modal_y = (self.terminal_size.1.saturating_sub(modal_height)) / 2;

        // Check if click is within modal bounds
        if mouse_x < modal_x 
            || mouse_x >= modal_x + modal_width 
            || mouse_y < modal_y 
            || mouse_y >= modal_y + modal_height {
            return None;
        }

        let button_row = modal_y + modal_height.saturating_sub(2);
        
        if mouse_y != button_row {
            return None;
        }

        if self.warning_is_info {
            // Info dialog - only "OK" button
            let button_x = modal_x + (modal_width / 2).saturating_sub(2);
            if mouse_x >= button_x && mouse_x < button_x + 4 {
                Some(0) // OK button
            } else {
                None
            }
        } else {
            // Confirmation dialog - "No" and "Yes" buttons
            let no_button_x = modal_x + (modal_width / 3).saturating_sub(2);
            let yes_button_x = modal_x + (2 * modal_width / 3).saturating_sub(2);

            if mouse_x >= no_button_x && mouse_x < no_button_x + 4 {
                Some(0) // No button
            } else if mouse_x >= yes_button_x && mouse_x < yes_button_x + 5 {
                Some(1) // Yes button
            } else {
                None
            }
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        use crossterm::event::MouseEventKind;

        // Handle dialog first (highest priority)
        if self.warning_message.is_some() {
            self.handle_mouse_on_dialog(mouse);
            return;
        }

        // Handle input dialog
        if let crate::menu::MenuState::InputDialog(_) = &self.menu_system.state {
            if self.handle_mouse_on_input_dialog(mouse) {
                return;
            }
        }

        // Handle file picker
        if let crate::menu::MenuState::FilePicker(_) = &self.menu_system.state {
            if self.handle_mouse_on_file_picker(mouse) {
                return;
            }
        }

        // Handle menus
        if self.handle_mouse_on_menus(mouse) {
            return;
        }

        // Handle find/replace bar
        if self.handle_mouse_on_find_replace(mouse) {
            return;
        }

        // Handle sidebar resize
        if self.handle_sidebar_resize(mouse) {
            return;
        }

        // Handle tree view
        if mouse.column < self.sidebar_width && self.tree_view.is_some() {
            if self.handle_mouse_on_tree_view(mouse) {
                return;
            }
        }

        // Handle editor (remaining area)
        if mouse.column >= self.sidebar_width {
            // Adjust mouse coordinates for sidebar
            let adjusted_mouse = MouseEvent {
                column: mouse.column - self.sidebar_width,
                row: mouse.row,
                kind: mouse.kind,
                modifiers: mouse.modifiers,
            };
            self.handle_mouse_on_editor(adjusted_mouse);
        }

        // Handle mouse up events globally
        if let MouseEventKind::Up(_) = mouse.kind {
            self.scrollbar_dragging = false;
            self.file_picker_scrollbar_dragging = false;
            self.tree_scrollbar_dragging = false;
            self.sidebar_resizing = false;
        }
    }

    // Add missing mouse handler methods
    pub fn handle_mouse_on_menus(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};
        
        match &self.menu_system.state {
            crate::menu::MenuState::MainMenu(_) |
            crate::menu::MenuState::CurrentTabMenu(_) |
            crate::menu::MenuState::TreeContextMenu(_) => {
                // Handle menu interactions
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // For now, just close the menu on click
                        // In a full implementation, you'd check if click is on a menu item
                        self.menu_system.close();
                        true
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        // Handle menu selection
                        true
                    }
                    _ => false
                }
            }
            _ => false
        }
    }

    pub fn handle_mouse_on_tree_view(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};
        
        if let Some(tree_view) = &mut self.tree_view {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Set focus to tree view
                    self.focus_mode = crate::app::FocusMode::TreeView;
                    tree_view.is_focused = true;
                    
                    // Select item at mouse position
                    let visible_items = tree_view.get_visible_items();
                    let item_index = mouse.row as usize;
                    
                    if item_index < visible_items.len() {
                        tree_view.selected_index = item_index;
                    }
                    
                    true
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    true
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    // Handle right-click context menu
                    true
                }
                _ => false
            }
        } else {
            false
        }
    }
}