use crate::app::App;
use crossterm::event::KeyEvent;
use std::time::Duration;

impl App {
    pub fn handle_input_dialog_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if let crate::menu::MenuState::InputDialog(input_state) = &mut self.menu_system.state {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, KeyModifiers::NONE) => {
                    self.menu_system.close();
                }
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    let input = input_state.input.clone();
                    let operation = input_state.operation.clone();
                    let target_path = input_state.target_path.clone();
                    self.menu_system.close();
                    self.execute_file_operation(&operation, &target_path, &input);
                }
                (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                    // Handle character input
                    if let Some(selection_start) = input_state.selection_start {
                        // Replace selected text
                        let start = selection_start.min(input_state.cursor_position);
                        let end = selection_start.max(input_state.cursor_position);
                        input_state.input.replace_range(start..end, &c.to_string());
                        input_state.cursor_position = start + 1;
                        input_state.selection_start = None;
                    } else {
                        // Insert character at cursor
                        input_state.input.insert(input_state.cursor_position, c);
                        input_state.cursor_position += 1;
                    }
                }
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    if let Some(selection_start) = input_state.selection_start {
                        // Delete selected text
                        let start = selection_start.min(input_state.cursor_position);
                        let end = selection_start.max(input_state.cursor_position);
                        input_state.input.replace_range(start..end, "");
                        input_state.cursor_position = start;
                        input_state.selection_start = None;
                    } else if input_state.cursor_position > 0 {
                        input_state.cursor_position -= 1;
                        input_state.input.remove(input_state.cursor_position);
                    }
                }
                (KeyCode::Delete, KeyModifiers::NONE) => {
                    if let Some(selection_start) = input_state.selection_start {
                        // Delete selected text
                        let start = selection_start.min(input_state.cursor_position);
                        let end = selection_start.max(input_state.cursor_position);
                        input_state.input.replace_range(start..end, "");
                        input_state.cursor_position = start;
                        input_state.selection_start = None;
                    } else if input_state.cursor_position < input_state.input.len() {
                        input_state.input.remove(input_state.cursor_position);
                    }
                }
                (KeyCode::Left, KeyModifiers::NONE) => {
                    if input_state.cursor_position > 0 {
                        input_state.cursor_position -= 1;
                    }
                    input_state.selection_start = None;
                }
                (KeyCode::Right, KeyModifiers::NONE) => {
                    if input_state.cursor_position < input_state.input.len() {
                        input_state.cursor_position += 1;
                    }
                    input_state.selection_start = None;
                }
                (KeyCode::Home, KeyModifiers::NONE) => {
                    input_state.cursor_position = 0;
                    input_state.selection_start = None;
                }
                (KeyCode::End, KeyModifiers::NONE) => {
                    input_state.cursor_position = input_state.input.len();
                    input_state.selection_start = None;
                }
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    // Select all
                    input_state.selection_start = Some(0);
                    input_state.cursor_position = input_state.input.len();
                }
                _ => {}
            }
        }
    }

    pub fn handle_warning_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        if self.warning_message.is_none() {
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                // ESC or Ctrl+Q cancels
                self.warning_message = None;
                self.pending_delete_path = None;
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if self.warning_is_info {
                    // Info dialog - just dismiss
                    self.warning_message = None;
                } else {
                    // Confirmation dialog - execute based on selected button
                    if self.warning_selected_button == 1 {
                        // "Yes" button - proceed with deletion
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
                                    self.set_status_message(message, Duration::from_secs(3));
                                    // Refresh tree view
                                    if let Some(tree_view) = &mut self.tree_view {
                                        tree_view.refresh();
                                    }
                                }
                                Err(e) => {
                                    self.set_status_message(
                                        format!("Delete failed: {}", e),
                                        Duration::from_secs(5),
                                    );
                                }
                            }
                        }
                    }
                    self.warning_message = None;
                    self.warning_selected_button = 0;
                }
            }
            (KeyCode::Left, KeyModifiers::NONE) | (KeyCode::Right, KeyModifiers::NONE) => {
                if !self.warning_is_info {
                    // Toggle between Yes/No buttons
                    self.warning_selected_button = 1 - self.warning_selected_button;
                }
            }
            (KeyCode::Char('y'), KeyModifiers::NONE) | (KeyCode::Char('Y'), KeyModifiers::NONE) => {
                if !self.warning_is_info {
                    self.warning_selected_button = 1; // Select "Yes"
                }
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) | (KeyCode::Char('N'), KeyModifiers::NONE) => {
                if !self.warning_is_info {
                    self.warning_selected_button = 0; // Select "No"
                }
            }
            _ => {}
        }
    }
}