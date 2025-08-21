use crate::app::App;
use crate::tab::Tab;
use crate::cursor::Cursor;
use crate::rope_buffer::RopeBuffer;
use crossterm::event::{MouseEvent, MouseButton, MouseEventKind};

impl App {
    pub fn handle_editor_scroll(&mut self, scroll_kind: crossterm::event::MouseEventKind) {
        use crossterm::event::MouseEventKind;

        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let now = std::time::Instant::now();

            // Check if we're continuing to scroll (within 100ms of last scroll)
            if let Some(last_time) = self.last_scroll_time {
                if now.duration_since(last_time).as_millis() < 100 {
                    self.scroll_acceleration = (self.scroll_acceleration + 1).min(5);
                } else {
                    self.scroll_acceleration = 1;
                }
            } else {
                self.scroll_acceleration = 1;
            }

            self.last_scroll_time = Some(now);

            let scroll_amount = self.scroll_acceleration;

            match tab {
                Tab::Editor { viewport_offset, buffer, .. } => {
                    let editor_height = (self.terminal_size.1 as usize).saturating_sub(2);
                    let max_scroll = buffer.len_lines().saturating_sub(editor_height);

                    match scroll_kind {
                        MouseEventKind::ScrollUp => {
                            viewport_offset.0 = viewport_offset.0.saturating_sub(scroll_amount);
                        }
                        MouseEventKind::ScrollDown => {
                            viewport_offset.0 = (viewport_offset.0 + scroll_amount).min(max_scroll);
                        }
                        _ => {}
                    }
                }
                Tab::Terminal { .. } => {
                    // Handle terminal scrolling if needed
                }
            }
        }
    }

    pub fn handle_scrollbar_click(&mut self, mouse: MouseEvent) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let is_markdown = tab.is_markdown();
            if let Tab::Editor { preview_mode, buffer, viewport_offset, .. } = tab {
                let editor_height = (self.terminal_size.1 as usize).saturating_sub(2); // Tab bar + status bar
                let click_y = (mouse.row as usize).saturating_sub(1); // Subtract tab bar
                let is_markdown_preview = *preview_mode && is_markdown;

                let content_lines = if is_markdown_preview {
                    // For markdown preview, count the rendered lines
                    let content = buffer.to_string();
                    let markdown_widget = crate::markdown_widget::MarkdownWidget::new(&content);
                    markdown_widget.parse_markdown().len()
                } else {
                    // For normal editor, use buffer lines
                    buffer.len_lines()
                };

                // Create scrollbar state to calculate click position
                let scrollbar_state =
                    crate::ui::ScrollbarState::new(content_lines, editor_height, viewport_offset.0);

                // Calculate new scroll position based on click
                let new_position = scrollbar_state.click_position(editor_height, click_y);

                // Update viewport offset
                viewport_offset.0 = new_position;
            }
        }
    }

    pub fn handle_mouse_on_input_dialog(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};

        if let crate::menu::MenuState::InputDialog(input_state) = &mut self.menu_system.state {
            // Calculate dialog position (same logic as in UI module)
            let dialog_width = 50u16.min(self.terminal_size.0.saturating_sub(4));
            let dialog_height = 8; // Updated to match UI spacing
            let dialog_x = (self.terminal_size.0.saturating_sub(dialog_width)) / 2;
            let dialog_y = (self.terminal_size.1.saturating_sub(dialog_height)) / 2;

            // Check if click is within dialog bounds
            if mouse.column >= dialog_x
                && mouse.column < dialog_x + dialog_width
                && mouse.row >= dialog_y
                && mouse.row < dialog_y + dialog_height
            {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Calculate input field position
                        let input_y = dialog_y + 3; // Title + border + spacing
                        if mouse.row == input_y {
                            // Click in input field - position cursor
                            let input_x_start = dialog_x + 2; // Border + padding
                            let input_width = dialog_width.saturating_sub(4); // Both borders + padding
                            
                            if mouse.column >= input_x_start && mouse.column < input_x_start + input_width {
                                let click_offset = (mouse.column - input_x_start) as usize;
                                input_state.cursor_position = click_offset.min(input_state.input.len());
                                input_state.selection_start = None;
                            }
                        }

                        // Check for button clicks
                        let button_y = dialog_y + dialog_height.saturating_sub(2);
                        if mouse.row == button_y {
                            let cancel_button_x = dialog_x + 5;
                            let ok_button_x = dialog_x + dialog_width.saturating_sub(8);
                            
                            if mouse.column >= cancel_button_x && mouse.column < cancel_button_x + 6 {
                                // Cancel button clicked
                                self.menu_system.close();
                            } else if mouse.column >= ok_button_x && mouse.column < ok_button_x + 4 {
                                // OK button clicked
                                let input = input_state.input.clone();
                                let operation = input_state.operation.clone();
                                let target_path = input_state.target_path.clone();
                                self.menu_system.close();
                                self.execute_file_operation(&operation, &target_path, &input);
                            }
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        // Handle text selection
                        let input_y = dialog_y + 3;
                        if mouse.row == input_y {
                            let input_x_start = dialog_x + 2;
                            let input_width = dialog_width.saturating_sub(4);
                            
                            if mouse.column >= input_x_start && mouse.column < input_x_start + input_width {
                                let drag_offset = (mouse.column - input_x_start) as usize;
                                let new_cursor_pos = drag_offset.min(input_state.input.len());
                                
                                if input_state.selection_start.is_none() {
                                    input_state.selection_start = Some(input_state.cursor_position);
                                }
                                input_state.cursor_position = new_cursor_pos;
                            }
                        }
                    }
                    _ => {}
                }
                return true; // Event consumed
            }
        }
        false
    }

    pub fn select_word_at_cursor(input_state: &mut crate::menu::InputDialogState) {
        let chars: Vec<char> = input_state.input.chars().collect();
        let pos = input_state.cursor_position.min(chars.len());
        
        if chars.is_empty() {
            return;
        }

        // Find start of word
        let mut start = pos;
        while start > 0 
            && !chars[start - 1].is_whitespace() 
            && !crate::app::is_word_separator(chars[start - 1])
        {
            start -= 1;
        }

        // Find end of word
        let mut end = pos;
        while end < chars.len() && !chars[end].is_whitespace() && !crate::app::is_word_separator(chars[end]) {
            end += 1;
        }

        // Set selection
        input_state.selection_start = Some(start);
        input_state.cursor_position = end;
    }

    pub fn delete_input_selection(input_state: &mut crate::menu::InputDialogState) {
        if let Some(sel_start) = input_state.selection_start {
            let (start, end) = if sel_start < input_state.cursor_position {
                (sel_start, input_state.cursor_position)
            } else {
                (input_state.cursor_position, sel_start)
            };

            // Remove selected characters
            for _ in start..end {
                if start < input_state.input.len() {
                    input_state.input.remove(start);
                }
            }

            input_state.cursor_position = start;
            input_state.selection_start = None;
        }
    }

    pub fn delete_selection(
        buffer: &mut RopeBuffer,
        cursor: &mut Cursor,
    ) {
        if let Some((start, end)) = cursor.get_selection() {
            let start_idx = buffer.line_to_char(start.line)
                + start.column.min(buffer.get_line_text(start.line).len());
            let end_idx = buffer.line_to_char(end.line)
                + end.column.min(buffer.get_line_text(end.line).len());

            buffer.delete_range(start_idx..end_idx);
            cursor.move_to(start.line, start.column);
            cursor.clear_selection();
        }
    }

    pub fn insert_tab(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
        let char_idx = buffer.line_to_char(cursor.position.line) + cursor.position.column;
        buffer.insert_char(char_idx, '\t');
        cursor.move_right(buffer);
    }

    pub fn handle_sidebar_resize(&mut self, mouse: MouseEvent) -> bool {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if we're clicking on the resize border (right edge of sidebar)
                if mouse.column == self.sidebar_width {
                    self.sidebar_resizing = true;
                    return true;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.sidebar_resizing {
                    // Update sidebar width, ensuring minimum and maximum bounds
                    let min_width = 15;
                    let max_width = self.terminal_size.0 / 2;
                    self.sidebar_width = mouse.column.max(min_width).min(max_width);
                    return true;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.sidebar_resizing {
                    self.sidebar_resizing = false;
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}