use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::Frame;
use std::time::{Duration, Instant};

use crate::keyboard::{handle_key_event, EditorCommand};
use crate::tab::{Tab, TabManager};
use crate::ui::UI;
use crate::cursor::Position;
use crate::menu::MenuSystem;

pub struct App {
    pub tab_manager: TabManager,
    pub running: bool,
    ui: UI,
    pub warning_message: Option<String>,
    pub pending_close: bool,
    pub pending_quit: bool,
    pub warning_selected_button: usize, // 0 = No, 1 = Yes
    pub mouse_selecting: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<(u16, u16)>,
    terminal_size: (u16, u16), // (width, height)
    pub menu_system: MenuSystem,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab_manager: TabManager::new(),
            running: true,
            ui: UI::new(),
            warning_message: None,
            pending_close: false,
            pending_quit: false,
            warning_selected_button: 0, // Default to "No" (safer)
            mouse_selecting: false,
            last_click_time: None,
            last_click_pos: None,
            terminal_size: (80, 24), // Default size, will be updated during draw
            menu_system: MenuSystem::new(),
        }
    }

    pub fn handle_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::Quit => self.handle_quit(),
            EditorCommand::Save => self.save_current_file(),
            EditorCommand::NewTab => {
                let new_tab = Tab::new(format!("untitled-{}", self.tab_manager.len() + 1));
                self.tab_manager.add_tab(new_tab);
            }
            EditorCommand::CloseTab => {
                self.handle_close_tab();
            }
            EditorCommand::NextTab => self.tab_manager.next_tab(),
            EditorCommand::PrevTab => self.tab_manager.prev_tab(),
            EditorCommand::PageUp => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.viewport_offset.0 = tab.viewport_offset.0.saturating_sub(10);
                }
            }
            EditorCommand::PageDown => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.viewport_offset.0 += 10;
                }
            }
            EditorCommand::Modified => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.mark_modified();
                }
            }
            EditorCommand::ToggleMenu => {
                self.menu_system.toggle_main_menu();
            }
            EditorCommand::OpenFile => {
                self.menu_system.open_file_picker();
            }
            EditorCommand::CurrentTab => {
                self.menu_system.open_current_tab_menu();
            }
        }
    }

    fn save_current_file(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            if let Some(path) = &tab.path {
                if let Ok(_) = std::fs::write(path, tab.buffer.to_string()) {
                    tab.mark_saved();
                }
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        // Handle warning dialog first
        if self.warning_message.is_some() {
            self.handle_warning_key(key);
            return;
        }

        // Handle file picker dialog first (blocks all other input)
        if let crate::menu::MenuState::FilePicker(_) = &self.menu_system.state {
            self.handle_file_picker_key(key);
            return;
        }

        // Handle menu navigation
        if self.menu_system.is_open() {
            match (key.code, key.modifiers) {
                (KeyCode::Up, KeyModifiers::NONE) => {
                    self.menu_system.handle_up();
                    return;
                }
                (KeyCode::Down, KeyModifiers::NONE) => {
                    self.menu_system.handle_down();
                    return;
                }
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    if let Some(action) = self.menu_system.handle_enter() {
                        match action.as_str() {
                            "current_tab" => {
                                // Switch to current tab menu
                                self.menu_system.open_current_tab_menu();
                            }
                            "open_file" => {
                                self.menu_system.open_file_picker();
                            }
                            "close_tab" => {
                                self.handle_close_tab();
                            }
                            "close_other_tab" => {
                                self.tab_manager.close_other_tabs();
                            }
                            "quit" => {
                                self.handle_quit();
                            }
                            _ => {}
                        }
                    }
                    return;
                }
                (KeyCode::Esc, KeyModifiers::NONE) => {
                    self.menu_system.close();
                    return;
                }
                _ => {}
            }
        }

        let needs_viewport_update = if let Some(tab) = self.tab_manager.active_tab_mut() {
            let old_cursor_pos = tab.cursor.position.clone();
            let command = handle_key_event(key, &mut tab.buffer, &mut tab.cursor);
            let cursor_moved = tab.cursor.position != old_cursor_pos;
            
            if let Some(cmd) = command {
                self.handle_command(cmd);
            }
            
            cursor_moved
        } else {
            false
        };
        
        // Update viewport if cursor moved
        if needs_viewport_update {
            if let Some(tab) = self.tab_manager.active_tab_mut() {
                let visible_height = self.terminal_size.1.saturating_sub(2) as usize;
                tab.update_viewport(visible_height);
            }
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        use crossterm::event::MouseEventKind;
        
        // Handle scroll events globally first (they should work everywhere except dialogs)
        if matches!(mouse.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
            // Don't scroll if warning dialog is open
            if self.warning_message.is_some() {
                return;
            }
            
            // Don't scroll if file picker is open
            if matches!(self.menu_system.state, crate::menu::MenuState::FilePicker(_)) {
                return;
            }
            
            // Handle scroll for editor
            self.handle_editor_scroll(mouse.kind);
            return;
        }
        
        // Handle mouse events on warning dialog first
        if self.warning_message.is_some() {
            self.handle_mouse_on_dialog(mouse);
            return;
        }

        // Handle file picker mouse events
        if let crate::menu::MenuState::FilePicker(_) = &self.menu_system.state {
            if self.handle_mouse_on_file_picker(mouse) {
                return;
            }
        }

        // Handle menu mouse events
        if self.handle_mouse_on_menus(mouse) {
            return;
        }

        self.handle_mouse_on_editor(mouse);
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        // Update terminal size
        self.terminal_size = (frame.area().width, frame.area().height);
        
        self.ui.draw(frame, &mut self.tab_manager, &self.warning_message, self.warning_selected_button, &self.menu_system);
    }

    fn handle_quit(&mut self) {
        // Check if any tabs have unsaved changes
        let modified_tabs: Vec<String> = self.tab_manager
            .tabs()
            .iter()
            .filter(|tab| tab.modified)
            .map(|tab| tab.name.clone())
            .collect();

        if !modified_tabs.is_empty() {
            // Show warning for unsaved changes
            let message = if modified_tabs.len() == 1 {
                format!("Tab '{}' has unsaved changes. Quit anyway?", modified_tabs[0])
            } else {
                format!("{} tabs have unsaved changes. Quit anyway?", modified_tabs.len())
            };
            
            self.warning_message = Some(message);
            self.pending_quit = true;
            self.warning_selected_button = 0; // Default to "No"
            return;
        }
        
        // No unsaved changes, quit directly
        self.running = false;
    }

    fn handle_close_tab(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab() {
            if tab.modified {
                // Show warning for unsaved changes
                let tab_name = &tab.name;
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

    fn handle_warning_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        match (key.code, key.modifiers) {
            // Arrow keys to navigate between buttons
            (KeyCode::Left, KeyModifiers::NONE) | (KeyCode::Right, KeyModifiers::NONE) | (KeyCode::Tab, KeyModifiers::NONE) => {
                self.warning_selected_button = 1 - self.warning_selected_button; // Toggle between 0 and 1
            }
            
            // Enter to activate selected button
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if self.warning_selected_button == 1 {
                    // Yes button selected
                    self.warning_message = None;
                    self.warning_selected_button = 0;
                    
                    if self.pending_quit {
                        // Quit the application
                        self.pending_quit = false;
                        self.running = false;
                    } else if self.pending_close {
                        // Close the tab
                        self.pending_close = false;
                        if !self.tab_manager.close_current_tab() {
                            self.running = false;
                        }
                    }
                } else {
                    // No button selected - cancel
                    self.warning_message = None;
                    self.pending_close = false;
                    self.pending_quit = false;
                    self.warning_selected_button = 0;
                }
            }
            
            // Keyboard shortcuts still work
            (KeyCode::Char('y'), KeyModifiers::NONE) | (KeyCode::Char('Y'), KeyModifiers::NONE) => {
                // User confirmed
                self.warning_message = None;
                self.warning_selected_button = 0;
                
                if self.pending_quit {
                    // Quit the application
                    self.pending_quit = false;
                    self.running = false;
                } else if self.pending_close {
                    // Close the tab
                    self.pending_close = false;
                    if !self.tab_manager.close_current_tab() {
                        self.running = false;
                    }
                }
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) | (KeyCode::Char('N'), KeyModifiers::NONE) | (KeyCode::Esc, KeyModifiers::NONE) => {
                // User cancelled
                self.warning_message = None;
                self.pending_close = false;
                self.pending_quit = false;
                self.warning_selected_button = 0;
            }
            _ => {
                // Ignore other keys while warning is shown
            }
        }
    }

    fn handle_editor_scroll(&mut self, scroll_kind: crossterm::event::MouseEventKind) {
        use crossterm::event::MouseEventKind;
        
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let scroll_amount = 3;
            let total_lines = tab.buffer.len_lines();
            
            match scroll_kind {
                MouseEventKind::ScrollUp => {
                    // Scroll up
                    tab.viewport_offset.0 = tab.viewport_offset.0.saturating_sub(scroll_amount);
                }
                MouseEventKind::ScrollDown => {
                    // Scroll down
                    // The maximum offset should allow us to see the last line
                    // We need to ensure that start_line + visible_height can reach total_lines
                    let max_offset = total_lines.saturating_sub(1); // Allow scrolling to the very last line
                    
                    let new_offset = tab.viewport_offset.0 + scroll_amount;
                    tab.viewport_offset.0 = new_offset.min(max_offset);
                }
                _ => {}
            }
        }
    }
    
    fn handle_mouse_on_editor(&mut self, mouse: MouseEvent) {
        use crossterm::event::{MouseEventKind, MouseButton};

        // Get the active tab index to avoid borrowing conflicts
        let active_index = self.tab_manager.active_index();
        
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is on tab bar (row 0)
                if mouse.row == 0 {
                    // Check if click is on Ctrl+N hint
                    if self.is_ctrl_n_hint_clicked(mouse.column) {
                        // Create new tab
                        let new_tab = crate::tab::Tab::new(format!("untitled-{}", self.tab_manager.len() + 1));
                        self.tab_manager.add_tab(new_tab);
                        return;
                    }
                    
                    if let Some(clicked_tab) = self.get_clicked_tab(mouse.column) {
                        if clicked_tab == active_index {
                            // Clicked on active tab - show current tab menu
                            self.menu_system.open_current_tab_menu();
                            return;
                        } else {
                            // Clicked on different tab - switch to it
                            self.tab_manager.set_active_index(clicked_tab);
                            return;
                        }
                    }
                }
                let now = Instant::now();
                let current_pos = (mouse.column, mouse.row);
                
                // Check for double-click
                let is_double_click = if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    now.duration_since(last_time) < Duration::from_millis(500) && 
                    (current_pos.0.abs_diff(last_pos.0) <= 1 && current_pos.1.abs_diff(last_pos.1) <= 1)
                } else {
                    false
                };
                
                // Convert mouse coordinates to text position
                if let Some(pos) = self.mouse_to_text_position(mouse.column, mouse.row, active_index) {
                    if let Some(tab) = self.tab_manager.active_tab_mut() {
                        tab.cursor.position = pos;
                        // Update viewport after cursor move
                        let visible_height = self.terminal_size.1.saturating_sub(2) as usize;
                        tab.update_viewport(visible_height);
                        
                        if is_double_click {
                            // Double-click: select word at cursor position
                            tab.cursor.select_word_at_position(&tab.buffer);
                            self.mouse_selecting = false;
                            
                            // Reset click tracking to prevent triple-click detection
                            self.last_click_time = None;
                            self.last_click_pos = None;
                        } else {
                            // Single click: clear selection and prepare for potential drag
                            tab.cursor.clear_selection();
                            tab.cursor.start_selection();
                            self.mouse_selecting = true;
                            
                            // Update click tracking
                            self.last_click_time = Some(now);
                            self.last_click_pos = Some(current_pos);
                        }
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.mouse_selecting {
                    if let Some(pos) = self.mouse_to_text_position(mouse.column, mouse.row, active_index) {
                        if let Some(tab) = self.tab_manager.active_tab_mut() {
                            // Update cursor position while maintaining selection start
                            tab.cursor.position = pos;
                        }
                    }
                }
                // Note: If not mouse_selecting, we ignore drag (e.g., after double-click word selection)
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.mouse_selecting {
                    if let Some(tab) = self.tab_manager.active_tab_mut() {
                        // If we didn't actually drag (selection start == current position), clear selection
                        if let Some((start, end)) = tab.cursor.get_selection() {
                            if start == end {
                                tab.cursor.clear_selection();
                            }
                        }
                    }
                    self.mouse_selecting = false;
                }
            }
            _ => {}
        }
    }

    fn mouse_to_text_position(&self, mouse_x: u16, mouse_y: u16, tab_index: usize) -> Option<Position> {
        // Get the tab safely
        let tab = self.tab_manager.tabs().get(tab_index)?;
        
        // We need to account for the UI layout:
        // - Tab bar: 1 line
        // - Editor content: rest minus status bar
        // - Status bar: 1 line
        
        // Calculate editor content area (accounting for tab bar)
        let editor_start_y = 1; // Tab bar takes 1 line
        
        if mouse_y < editor_start_y {
            return None; // Click is in tab bar
        }
        
        let editor_y = mouse_y - editor_start_y;
        let editor_line = editor_y as usize + tab.viewport_offset.0;
        
        // Calculate line number width
        let max_line = tab.buffer.len_lines();
        let line_number_width = if max_line > 0 {
            (max_line.to_string().len() + 1).max(4)
        } else {
            4
        };
        
        // Account for line numbers
        if mouse_x < line_number_width as u16 {
            return None; // Click is in line number area
        }
        
        let editor_x = mouse_x - line_number_width as u16;
        
        // Ensure we don't go beyond the buffer
        if editor_line >= tab.buffer.len_lines() {
            // Click beyond last line - position at end of last line
            if tab.buffer.len_lines() > 0 {
                let last_line = tab.buffer.len_lines() - 1;
                let last_line_len = tab.buffer.get_line_text(last_line).len();
                return Some(Position::new(last_line, last_line_len));
            } else {
                return Some(Position::new(0, 0));
            }
        }
        
        // Get the line text and ensure column doesn't exceed line length
        let line_text = tab.buffer.get_line_text(editor_line);
        let column = editor_x.min(line_text.len() as u16) as usize;
        
        Some(Position::new(editor_line, column))
    }

    fn handle_mouse_on_dialog(&mut self, mouse: MouseEvent) {
        use crossterm::event::{MouseEventKind, MouseButton};

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(button_clicked) = self.get_dialog_button_at_position(mouse.column, mouse.row) {
                    // Execute the button action immediately on click
                    if button_clicked == 1 {
                        // Yes button clicked
                        self.warning_message = None;
                        self.warning_selected_button = 0;
                        
                        if self.pending_quit {
                            self.pending_quit = false;
                            self.running = false;
                        } else if self.pending_close {
                            self.pending_close = false;
                            if !self.tab_manager.close_current_tab() {
                                self.running = false;
                            }
                        }
                    } else {
                        // No button clicked - cancel
                        self.warning_message = None;
                        self.pending_close = false;
                        self.pending_quit = false;
                        self.warning_selected_button = 0;
                    }
                }
            }
            MouseEventKind::Moved => {
                // Update button highlighting on hover
                if let Some(button_hovered) = self.get_dialog_button_at_position(mouse.column, mouse.row) {
                    self.warning_selected_button = button_hovered;
                } else {
                    // Optional: Reset to No button when not hovering any button
                    // This provides better visual feedback
                    // self.warning_selected_button = 0;
                }
            }
            _ => {}
        }
    }

    fn get_dialog_button_at_position(&self, mouse_x: u16, mouse_y: u16) -> Option<usize> {
        if let Some(message) = &self.warning_message {
            // Calculate dialog position and size (same logic as in UI module)
            let terminal_width = self.terminal_size.0;
            let terminal_height = self.terminal_size.1;
            
            let popup_width = (message.len() + 4).max(30).min(80) as u16;
            let popup_height = 7;
            let popup_x = (terminal_width.saturating_sub(popup_width)) / 2;
            let popup_y = (terminal_height.saturating_sub(popup_height)) / 2;
            
            // Check if click is within dialog bounds
            if mouse_x < popup_x || mouse_x >= popup_x + popup_width ||
               mouse_y < popup_y || mouse_y >= popup_y + popup_height {
                return None;
            }
            
            // Dialog layout with margin(1):
            // Row 0: border
            // Row 1: title spacer
            // Row 2: message  
            // Row 3: spacer
            // Row 4: buttons
            // Row 5: border
            
            // Buttons are at popup_y + 1 (margin) + 3 (button row index in chunks)
            let button_row = popup_y + 4;
            
            // Allow clicking on the button row or slightly around it for better UX
            if mouse_y >= button_row.saturating_sub(1) && mouse_y <= button_row + 1 {
                // Calculate button positions within the dialog
                // New button layout with borders: "  [ No ]    [ Yes ]  "
                // Layout: "  " + "[" + " No " + "]" + "  " + "[" + " Yes " + "]" + "  "
                //         2sp   1ch    4ch    1ch   2sp   1ch    5ch    1ch   2sp = 21 total
                let buttons_width = 21u16;
                let buttons_start_x = popup_x + (popup_width.saturating_sub(buttons_width)) / 2;
                
                // "No" button area: [ No ] at positions 2-7
                let no_button_start = buttons_start_x + 2;
                let no_button_end = buttons_start_x + 7;
                
                // "Yes" button area: [ Yes ] at positions 10-16  
                let yes_button_start = buttons_start_x + 10;
                let yes_button_end = buttons_start_x + 16;
                
                if mouse_x >= no_button_start && mouse_x <= no_button_end {
                    return Some(0); // No button
                } else if mouse_x >= yes_button_start && mouse_x <= yes_button_end {
                    return Some(1); // Yes button
                }
            }
        }
        
        None
    }

    fn handle_mouse_on_menus(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseEventKind, MouseButton};

        match mouse.kind {
            MouseEventKind::Moved => {
                // Handle hover on menus
                match &self.menu_system.state {
                    crate::menu::MenuState::MainMenu(menu) => {
                        let menu_area = ratatui::layout::Rect {
                            x: 0,
                            y: self.terminal_size.1.saturating_sub(menu.height + 1),
                            width: menu.width,
                            height: menu.height,
                        };
                        
                        let hovered_item = menu.get_clicked_item(&menu_area, mouse.column, mouse.row);
                        
                        if let crate::menu::MenuState::MainMenu(menu) = &mut self.menu_system.state {
                            menu.hovered_index = hovered_item;
                        }
                        return true;
                    }
                    crate::menu::MenuState::CurrentTabMenu(menu) => {
                        let tab_index = self.tab_manager.active_index();
                        let tab_x = self.get_tab_x_position_for_menu(tab_index);
                        let menu_width = menu.width;
                        let menu_height = menu.height;
                        
                        let menu_area = ratatui::layout::Rect {
                            x: tab_x,
                            y: 1,
                            width: menu_width,
                            height: menu_height,
                        };
                        
                        let hovered_item = menu.get_clicked_item(&menu_area, mouse.column, mouse.row);
                        
                        if let crate::menu::MenuState::CurrentTabMenu(menu) = &mut self.menu_system.state {
                            menu.hovered_index = hovered_item;
                        }
                        return true;
                    }
                    _ => {}
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is on F1 menu button in status bar
                if self.is_f1_button_clicked(mouse.column, mouse.row) {
                    self.menu_system.toggle_main_menu();
                    return true;
                }

                // Check if click is on any open menu
                match &self.menu_system.state {
                    crate::menu::MenuState::MainMenu(menu) => {
                        let menu_area = ratatui::layout::Rect {
                            x: 0,
                            y: self.terminal_size.1.saturating_sub(menu.height + 1),
                            width: menu.width,
                            height: menu.height,
                        };
                        if let Some(item_index) = menu.get_clicked_item(&menu_area, mouse.column, mouse.row) {
                            // Update selection and handle click
                            if let crate::menu::MenuState::MainMenu(menu) = &mut self.menu_system.state {
                                menu.selected_index = item_index;
                                if let Some(action) = self.menu_system.handle_enter() {
                                    match action.as_str() {
                                        "current_tab" => self.menu_system.open_current_tab_menu(),
                                        "open_file" => self.menu_system.open_file_picker(),
                                        "quit" => self.handle_quit(),
                                        _ => {}
                                    }
                                }
                            }
                            return true;
                        }
                    }
                    crate::menu::MenuState::CurrentTabMenu(menu) => {
                        let tab_index = self.tab_manager.active_index();
                        let tab_x = self.get_tab_x_position_for_menu(tab_index);
                        let menu_area = ratatui::layout::Rect {
                            x: tab_x,
                            y: 1, // Directly below tab bar
                            width: menu.width,
                            height: menu.height,
                        };
                        if let Some(item_index) = menu.get_clicked_item(&menu_area, mouse.column, mouse.row) {
                            // Update selection and handle click
                            if let crate::menu::MenuState::CurrentTabMenu(menu) = &mut self.menu_system.state {
                                menu.selected_index = item_index;
                                if let Some(action) = self.menu_system.handle_enter() {
                                    match action.as_str() {
                                        "close_tab" => self.handle_close_tab(),
                                        "close_other_tab" => {
                                            self.tab_manager.close_other_tabs();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        false
    }

    fn is_f1_button_clicked(&self, mouse_x: u16, mouse_y: u16) -> bool {
        let status_row = self.terminal_size.1.saturating_sub(1);
        if mouse_y != status_row {
            return false;
        }

        // F1 button is at the leftmost position of status bar, 6 characters wide
        mouse_x < 6
    }

    fn get_clicked_tab(&self, mouse_x: u16) -> Option<usize> {
        let mut current_x = 0u16;
        
        for (i, tab) in self.tab_manager.tabs().iter().enumerate() {
            let tab_width = tab.display_name().len() + 2; // " " + name + " "
            
            if mouse_x >= current_x && mouse_x < current_x + tab_width as u16 {
                return Some(i);
            }
            
            current_x += tab_width as u16;
        }
        
        None
    }

    fn get_tab_x_position_for_menu(&self, target_tab_index: usize) -> u16 {
        let mut x_pos = 0u16;
        
        for (i, tab) in self.tab_manager.tabs().iter().enumerate() {
            if i == target_tab_index {
                return x_pos;
            }
            // Calculate tab width: " " + tab_name + " " = tab_name.len() + 2
            let tab_width = tab.display_name().len() + 2;
            x_pos += tab_width as u16;
        }
        
        x_pos
    }

    fn is_ctrl_n_hint_clicked(&self, mouse_x: u16) -> bool {
        // Calculate where the Ctrl+N hint appears
        let mut hint_start_x = 0u16;
        
        // Sum up all tab widths
        for tab in self.tab_manager.tabs().iter() {
            let tab_width = tab.display_name().len() + 2;
            hint_start_x += tab_width as u16;
        }
        
        // Add the spacing: "  " (2 spaces before Ctrl+N)
        hint_start_x += 2;
        
        // Ctrl+N is 6 characters
        let hint_end_x = hint_start_x + 6;
        
        mouse_x >= hint_start_x && mouse_x < hint_end_x
    }

    fn handle_mouse_on_file_picker(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let crate::menu::MenuState::FilePicker(picker_state) = &self.menu_system.state {
                    // Calculate file picker modal area (same as in UI draw method)
                    let modal_width = 60u16.min(self.terminal_size.0.saturating_sub(4));
                    let modal_height = 20u16.min(self.terminal_size.1.saturating_sub(4));
                    let modal_x = (self.terminal_size.0.saturating_sub(modal_width)) / 2;
                    let modal_y = (self.terminal_size.1.saturating_sub(modal_height)) / 2;
                    
                    // Check if click is within modal bounds
                    if mouse.column >= modal_x && mouse.column < modal_x + modal_width &&
                       mouse.row >= modal_y && mouse.row < modal_y + modal_height {
                        
                        // Calculate file list area (accounting for margin and search input)
                        let list_start_y = modal_y + 1 + 1 + 1 + 1; // border + margin + search + separator
                        let list_height = modal_height.saturating_sub(4); // subtract borders and other elements
                        
                        if mouse.row >= list_start_y && mouse.row < list_start_y + list_height {
                            // Calculate which file was clicked
                            let relative_y = mouse.row - list_start_y;
                            let visible_files = list_height as usize;
                            let start_index = if picker_state.selected_index >= visible_files {
                                picker_state.selected_index.saturating_sub(visible_files - 1)
                            } else {
                                0
                            };
                            
                            let clicked_file_index = start_index + relative_y as usize;
                            
                            if clicked_file_index < picker_state.filtered_files.len() {
                                // Open the clicked file
                                let selected_file = &picker_state.filtered_files[clicked_file_index];
                                if let Ok(content) = std::fs::read_to_string(selected_file) {
                                    let mut new_tab = crate::tab::Tab::new(
                                        selected_file.file_name()
                                            .and_then(|name| name.to_str())
                                            .unwrap_or("untitled")
                                            .to_string()
                                    );
                                    new_tab.path = Some(selected_file.clone());
                                    new_tab.buffer = crate::rope_buffer::RopeBuffer::from_str(&content);
                                    self.tab_manager.add_tab(new_tab);
                                }
                                self.menu_system.close();
                                return true;
                            }
                        }
                        
                        return true; // Click was inside modal, consume it
                    }
                }
            }
            _ => {}
        }
        
        false
    }

    fn handle_file_picker_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, KeyModifiers::NONE) => {
                    // Close file picker
                    self.menu_system.close();
                }
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    // Open selected file
                    if let Some(selected_file) = picker_state.get_selected_file() {
                        if let Ok(content) = std::fs::read_to_string(selected_file) {
                            let mut new_tab = crate::tab::Tab::new(
                                selected_file.file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("untitled")
                                    .to_string()
                            );
                            new_tab.path = Some(selected_file.clone());
                            new_tab.buffer = crate::rope_buffer::RopeBuffer::from_str(&content);
                            self.tab_manager.add_tab(new_tab);
                        }
                    }
                    self.menu_system.close();
                }
                (KeyCode::Up, KeyModifiers::NONE) => {
                    picker_state.move_selection_up();
                }
                (KeyCode::Down, KeyModifiers::NONE) => {
                    picker_state.move_selection_down();
                }
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    // Remove last character from search
                    picker_state.search_query.pop();
                    picker_state.update_filter();
                }
                (KeyCode::Char(c), KeyModifiers::NONE) => {
                    // Add character to search
                    picker_state.search_query.push(c);
                    picker_state.update_filter();
                }
                _ => {
                    // Ignore other keys
                }
            }
        }
    }


}