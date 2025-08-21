use crate::app::App;
use crate::tab::Tab;
use crossterm::event::KeyEvent;

impl App {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Handle warning dialog first
        if self.warning_message.is_some() {
            self.handle_warning_key(key);
            return false;
        }

        // Handle file picker dialog first (blocks all other input)
        if let crate::menu::MenuState::FilePicker(_) = &self.menu_system.state {
            self.handle_file_picker_key(key);
            return false;
        }

        // Handle input dialog
        if let crate::menu::MenuState::InputDialog(_) = &self.menu_system.state {
            self.handle_input_dialog_key(key);
            return false;
        }

        // Check if find/replace is active
        let is_find_active = if let Some(tab) = self.tab_manager.active_tab() {
            match tab {
                Tab::Editor { find_replace_state, .. } => find_replace_state.active,
                Tab::Terminal { .. } => false,
            }
        } else {
            false
        };

        // Handle find/replace keys if active
        if is_find_active && self.handle_find_replace_key(key) {
            return true;
        }

        // Handle global commands
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.handle_quit();
                return true;
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                self.save_current_file();
                return true;
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                self.handle_close_tab();
                return true;
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.create_new_tab();
                return true;
            }
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.create_new_terminal_tab();
                return true;
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.start_find();
                }
                return true;
            }
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.start_find_replace();
                }
                return true;
            }
            (KeyCode::F(1), KeyModifiers::NONE) => {
                self.menu_system.toggle_help();
                return true;
            }
            (KeyCode::Tab, KeyModifiers::CONTROL) => {
                self.switch_next_tab();
                return true;
            }
            (KeyCode::BackTab, KeyModifiers::SHIFT) => {
                self.switch_prev_tab();
                return true;
            }
            _ => {}
        }

        // Handle tree view commands when focused
        if self.focus_mode == crate::app::FocusMode::TreeView {
            if let Some(tree_view) = &mut self.tree_view {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, KeyModifiers::NONE) => {
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            if !selected_item.is_dir {
                                // Open file in new tab
                                match std::fs::read_to_string(&selected_item.path) {
                                    Ok(content) => {
                                        let mut new_tab = Tab::from_file(selected_item.path.clone(), &content);
                                        if let Tab::Editor { word_wrap, .. } = &mut new_tab {
                                            *word_wrap = self.global_word_wrap;
                                        }
                                        self.tab_manager.add_tab(new_tab);
                                        self.focus_mode = crate::app::FocusMode::Editor;
                                        tree_view.is_focused = false;
                                    }
                                    Err(e) => {
                                        self.set_status_message(
                                            format!("Failed to open file: {}", e),
                                            std::time::Duration::from_secs(3),
                                        );
                                    }
                                }
                            } else {
                                tree_view.toggle_directory();
                            }
                        }
                        return true;
                    }
                    (KeyCode::Char(' '), KeyModifiers::NONE) => {
                        tree_view.toggle_directory();
                        return true;
                    }
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        tree_view.move_up();
                        return true;
                    }
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        tree_view.move_down();
                        return true;
                    }
                    (KeyCode::Esc, KeyModifiers::NONE) => {
                        self.focus_mode = crate::app::FocusMode::Editor;
                        tree_view.is_focused = false;
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // Handle editor commands
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            match tab {
                Tab::Editor { cursor, buffer, .. } => {
                    match (key.code, key.modifiers) {
                        // Navigation
                        (KeyCode::Left, KeyModifiers::NONE) => {
                            cursor.move_left(buffer);
                        }
                        (KeyCode::Right, KeyModifiers::NONE) => {
                            cursor.move_right(buffer);
                        }
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            cursor.move_up(buffer);
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            cursor.move_down(buffer);
                        }
                        (KeyCode::Home, KeyModifiers::NONE) => {
                            cursor.move_to_line_start();
                        }
                        (KeyCode::End, KeyModifiers::NONE) => {
                            cursor.move_to_line_end(buffer);
                        }
                        (KeyCode::PageUp, KeyModifiers::NONE) => {
                            let visible_height = (self.terminal_size.1 as usize).saturating_sub(2);
                            cursor.page_up(buffer, visible_height);
                        }
                        (KeyCode::PageDown, KeyModifiers::NONE) => {
                            let visible_height = (self.terminal_size.1 as usize).saturating_sub(2);
                            cursor.page_down(buffer, visible_height);
                        }
                        // Text editing
                        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                            if cursor.has_selection() {
                                Self::delete_selection(buffer, cursor);
                            }
                            let char_idx = buffer.line_to_char(cursor.position.line) + cursor.position.column;
                            buffer.insert_char(char_idx, c);
                            cursor.move_right(buffer);
                            tab.mark_modified();
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            if cursor.has_selection() {
                                Self::delete_selection(buffer, cursor);
                            }
                            let char_idx = buffer.line_to_char(cursor.position.line) + cursor.position.column;
                            buffer.insert_char(char_idx, '\n');
                            cursor.move_down(buffer);
                            cursor.move_to_line_start();
                            tab.mark_modified();
                        }
                        (KeyCode::Tab, KeyModifiers::NONE) => {
                            if cursor.has_selection() {
                                Self::delete_selection(buffer, cursor);
                            }
                            Self::insert_tab(buffer, cursor);
                            tab.mark_modified();
                        }
                        (KeyCode::Backspace, KeyModifiers::NONE) => {
                            if cursor.has_selection() {
                                Self::delete_selection(buffer, cursor);
                            } else if cursor.position.column > 0 {
                                cursor.move_left(buffer);
                                let char_idx = buffer.line_to_char(cursor.position.line) + cursor.position.column;
                                buffer.delete_char(char_idx);
                            } else if cursor.position.line > 0 {
                                let prev_line_len = buffer.get_line_text(cursor.position.line - 1).len();
                                cursor.move_up(buffer);
                                cursor.position.column = prev_line_len;
                                let char_idx = buffer.line_to_char(cursor.position.line) + cursor.position.column;
                                buffer.delete_char(char_idx);
                            }
                            tab.mark_modified();
                        }
                        (KeyCode::Delete, KeyModifiers::NONE) => {
                            if cursor.has_selection() {
                                Self::delete_selection(buffer, cursor);
                            } else {
                                let char_idx = buffer.line_to_char(cursor.position.line) + cursor.position.column;
                                if char_idx < buffer.len_chars() {
                                    buffer.delete_char(char_idx);
                                }
                            }
                            tab.mark_modified();
                        }
                        _ => {}
                    }
                    tab.update_viewport((self.terminal_size.1 as usize).saturating_sub(2));
                }
                Tab::Terminal { .. } => {
                    // Terminal handles its own key events
                }
            }
        }

        true
    }
}