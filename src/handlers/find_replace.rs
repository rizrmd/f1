use crate::app::App;
use crate::tab::{Tab, FindFocusedField};
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use std::time::Duration;

impl App {
    pub fn handle_find_replace_key(&mut self, key: KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        let tab = match self.tab_manager.active_tab_mut() {
            Some(tab) => tab,
            None => return false,
        };

        // Only handle find/replace for Editor tabs - destructure for easier access
        if let Tab::Editor { find_replace_state, .. } = tab {
            if !find_replace_state.active {
                return false;
            }
        } else {
            return false;
        }

        match (key.code, key.modifiers) {
            // ESC to close find/replace
            (KeyCode::Esc, KeyModifiers::NONE) => {
                tab.stop_find_replace();
                return true;
            }

            // Tab to switch between find and replace fields
            (KeyCode::Tab, KeyModifiers::NONE) => {
                if let Tab::Editor { find_replace_state, .. } = tab {
                    if find_replace_state.is_replace_mode {
                        find_replace_state.focused_field = match find_replace_state.focused_field {
                            FindFocusedField::Find => FindFocusedField::Replace,
                            FindFocusedField::Replace => FindFocusedField::Find,
                        };
                        return true;
                    }
                }
            }

            // Enter or F3 for next match
            (KeyCode::Enter, KeyModifiers::NONE) | (KeyCode::F(3), KeyModifiers::NONE) => {
                let has_matches = if let Tab::Editor { find_replace_state, .. } = tab {
                    !find_replace_state.matches.is_empty()
                } else {
                    false
                };
                
                if has_matches {
                    tab.find_next();
                    if let Tab::Editor { find_replace_state, .. } = tab {
                        let (idx, total) = (
                            find_replace_state.current_match_index,
                            find_replace_state.matches.len(),
                        );
                        if let Some(idx) = idx {
                            self.set_status_message(
                                format!("Match {} of {}", idx + 1, total),
                                Duration::from_secs(2),
                            );
                        }
                    }
                }
                return true;
            }

            // Shift+F3 or Shift+Enter for previous match
            (KeyCode::F(3), KeyModifiers::SHIFT) | (KeyCode::Enter, KeyModifiers::SHIFT) => {
                let has_matches = if let Tab::Editor { find_replace_state, .. } = tab {
                    !find_replace_state.matches.is_empty()
                } else {
                    false
                };
                
                if has_matches {
                    tab.find_prev();
                    if let Tab::Editor { find_replace_state, .. } = tab {
                        let (idx, total) = (
                            find_replace_state.current_match_index,
                            find_replace_state.matches.len(),
                        );
                        if let Some(idx) = idx {
                            self.set_status_message(
                                format!("Match {} of {}", idx + 1, total),
                                Duration::from_secs(2),
                            );
                        }
                    }
                }
                return true;
            }

            // Alt+C to toggle case sensitive
            (KeyCode::Char('c'), KeyModifiers::ALT) | (KeyCode::Char('C'), KeyModifiers::ALT) => {
                if let Tab::Editor { find_replace_state, .. } = tab {
                    find_replace_state.case_sensitive = !find_replace_state.case_sensitive;
                    tab.perform_find();
                }
                return true;
            }

            // Alt+W to toggle whole word
            (KeyCode::Char('w'), KeyModifiers::ALT) | (KeyCode::Char('W'), KeyModifiers::ALT) => {
                if let Tab::Editor { find_replace_state, .. } = tab {
                    find_replace_state.whole_word = !find_replace_state.whole_word;
                    tab.perform_find();
                }
                return true;
            }

            // Ctrl+H to toggle replace mode
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if let Tab::Editor { find_replace_state, .. } = tab {
                    find_replace_state.is_replace_mode = !find_replace_state.is_replace_mode;
                    // If toggling off replace mode, switch focus back to find field
                    if !find_replace_state.is_replace_mode
                        && find_replace_state.focused_field == FindFocusedField::Replace
                    {
                        find_replace_state.focused_field = FindFocusedField::Find;
                    }
                }
                return true;
            }

            // Ctrl+R to replace current
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                let is_replace_mode = if let Tab::Editor { find_replace_state, .. } = tab {
                    find_replace_state.is_replace_mode
                } else {
                    false
                };
                
                if is_replace_mode {
                    tab.replace_current();
                    if let Tab::Editor { find_replace_state, .. } = tab {
                        let remaining = find_replace_state.matches.len();
                        if remaining > 0 {
                            self.set_status_message(
                                format!("Replaced. {} matches remaining", remaining),
                                Duration::from_secs(2),
                            );
                        } else {
                            self.set_status_message(
                                "All matches replaced".to_string(),
                                Duration::from_secs(2),
                            );
                        }
                    }
                }
                return true;
            }

            // Character input for find/replace fields
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if let Tab::Editor { find_replace_state, .. } = tab {
                    match find_replace_state.focused_field {
                        FindFocusedField::Find => {
                            find_replace_state
                                .find_query
                                .insert(find_replace_state.find_cursor_position, c);
                            find_replace_state.find_cursor_position += 1;
                            tab.perform_find();
                        }
                        FindFocusedField::Replace => {
                            find_replace_state
                                .replace_query
                                .insert(find_replace_state.replace_cursor_position, c);
                            find_replace_state.replace_cursor_position += 1;
                        }
                    }
                }
                return true;
            }

            _ => {}
        }

        false
    }

    pub fn handle_mouse_on_find_replace(&mut self, mouse: MouseEvent) -> bool {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // Check if click is on find/replace bar
            let find_bar_row = self.terminal_size.1.saturating_sub(2);
            
            if mouse.row == find_bar_row {
                // Handle clicks on find/replace controls
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    if let Tab::Editor { find_replace_state, .. } = tab {
                        if find_replace_state.active {
                            // Simple field switching based on click position
                            let half_width = self.terminal_size.0 / 2;
                            
                            if find_replace_state.is_replace_mode && mouse.column > half_width {
                                find_replace_state.focused_field = FindFocusedField::Replace;
                            } else {
                                find_replace_state.focused_field = FindFocusedField::Find;
                            }
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}