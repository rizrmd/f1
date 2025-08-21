use crate::app::App;
use crate::tab::Tab;
use crossterm::event::{KeyEvent, MouseEvent, MouseButton, MouseEventKind};

impl App {
    pub fn handle_file_picker_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    // Handle Ctrl+Q to quit even when file picker is open
                    self.menu_system.close();
                    self.handle_quit();
                }
                (KeyCode::Esc, KeyModifiers::NONE) => {
                    // Close file picker
                    self.menu_system.close();
                }
                (KeyCode::Enter, KeyModifiers::NONE) | (KeyCode::Right, KeyModifiers::NONE) => {
                    // Enter directory or open file
                    if let Some(selected_item) = picker_state.get_selected_item() {
                        if selected_item.is_dir {
                            // Enter directory
                            picker_state.enter_directory(selected_item.path.clone());
                        } else {
                            // Open file
                            match std::fs::read(&selected_item.path) {
                                Ok(bytes) => {
                                    // Try to convert to string, if it fails show warning
                                    match String::from_utf8(bytes) {
                                        Ok(text) => {
                                            // Valid text file - open it using from_file to set preview mode for markdown
                                            let mut new_tab = crate::tab::Tab::from_file(
                                                selected_item.path.clone(),
                                                &text,
                                            );
                                            if let Tab::Editor { word_wrap, .. } = &mut new_tab {
                                                *word_wrap = self.global_word_wrap;
                                            }
                                            self.tab_manager.add_tab(new_tab);
                                            self.menu_system.close();
                                        }
                                        Err(_) => {
                                            // Binary file - show warning, don't open
                                            let size = std::fs::metadata(&selected_item.path)
                                                .map(|m| m.len())
                                                .unwrap_or(0);
                                            self.warning_message = Some(format!(
                                                "Cannot open binary file '{}' ({} bytes)",
                                                selected_item.name, size
                                            ));
                                            self.warning_selected_button = 0;
                                            self.warning_is_info = true;
                                            // Close file picker but don't open the file
                                            self.menu_system.close();
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Error reading file
                                    self.warning_message = Some(format!(
                                        "Cannot read file '{}': {}",
                                        selected_item.name, e
                                    ));
                                    self.warning_selected_button = 0;
                                    self.warning_is_info = true;
                                    // Close file picker
                                    self.menu_system.close();
                                }
                            }
                        }
                    }
                }
                (KeyCode::Left, KeyModifiers::NONE) | (KeyCode::Backspace, KeyModifiers::NONE) => {
                    // Go back to parent directory
                    picker_state.go_up();
                }
                (KeyCode::Up, KeyModifiers::NONE) => {
                    picker_state.move_up();
                }
                (KeyCode::Down, KeyModifiers::NONE) => {
                    picker_state.move_down();
                }
                (KeyCode::PageUp, KeyModifiers::NONE) => {
                    picker_state.page_up();
                }
                (KeyCode::PageDown, KeyModifiers::NONE) => {
                    picker_state.page_down();
                }
                (KeyCode::Home, KeyModifiers::NONE) => {
                    picker_state.move_to_start();
                }
                (KeyCode::End, KeyModifiers::NONE) => {
                    picker_state.move_to_end();
                }
                _ => {}
            }
        }
    }

    pub fn handle_file_picker_scrollbar_click(&mut self, mouse: MouseEvent) {
        if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
            let _modal_width = 80u16.min(self.terminal_size.0.saturating_sub(4));
            let modal_height = 28u16.min(self.terminal_size.1.saturating_sub(4));

            // Calculate scrollbar properties
            let total_items = picker_state.filtered_items.len() as u16;
            let visible_items = modal_height.saturating_sub(4); // Header + border
            
            if total_items <= visible_items {
                return; // No scrolling needed
            }

            // Calculate scroll position based on mouse click
            let scrollbar_height = visible_items.saturating_sub(2);
            let click_y = mouse.row.saturating_sub(2); // Adjust for modal position
            
            if click_y <= scrollbar_height {
                let scroll_ratio = click_y as f32 / scrollbar_height as f32;
                let new_offset = (scroll_ratio * (total_items - visible_items) as f32) as usize;
                // FilePickerState doesn't have offset field - using selected_index instead
                picker_state.selected_index = new_offset.min((total_items as usize).saturating_sub(1));
            }
        }
    }

    pub fn handle_mouse_on_file_picker(&mut self, mouse: MouseEvent) -> bool {
        if let crate::menu::MenuState::FilePicker(_) = &self.menu_system.state {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Check if click is on scrollbar
                    if mouse.column >= self.terminal_size.0.saturating_sub(2) {
                        self.file_picker_scrollbar_dragging = true;
                        self.handle_file_picker_scrollbar_click(mouse);
                        return true;
                    }
                    
                    // Handle item selection
                    if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                        let modal_height = 28u16.min(self.terminal_size.1.saturating_sub(4));
                        let item_y = mouse.row.saturating_sub(2); // Adjust for modal header
                        
                        if item_y < modal_height.saturating_sub(4) {
                            let item_index = item_y as usize;
                            if item_index < picker_state.filtered_items.len() {
                                picker_state.selected_index = item_index;
                            }
                        }
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    if self.file_picker_scrollbar_dragging {
                        self.handle_file_picker_scrollbar_click(mouse);
                        return true;
                    }
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    if self.file_picker_scrollbar_dragging {
                        self.file_picker_scrollbar_dragging = false;
                        return true;
                    }
                }
                MouseEventKind::ScrollUp => {
                    if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                        picker_state.move_up();
                    }
                    return true;
                }
                MouseEventKind::ScrollDown => {
                    if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                        picker_state.move_down();
                    }
                    return true;
                }
                _ => {}
            }
            return true;
        }
        false
    }
}