use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::Frame;
use std::time::{Duration, Instant};
use std::path::PathBuf;

fn is_word_separator(ch: char) -> bool {
    matches!(ch, '.' | '-' | '_' | '/' | '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | ',' | ';' | ':')
}

use crate::keyboard::{handle_key_event, EditorCommand};
use crate::tab::{Tab, TabManager};
use crate::ui::{UI, ScrollbarState};
use crate::cursor::Position;
use crate::menu::{MenuSystem, MenuState, FileItem};
use crate::tree_view::TreeView;

pub struct App {
    pub tab_manager: TabManager,
    pub running: bool,
    ui: UI,
    pub warning_message: Option<String>,
    pub pending_close: bool,
    pub pending_quit: bool,
    pub warning_selected_button: usize, // 0 = No, 1 = Yes
    pub warning_is_info: bool, // true = OK button only, false = Yes/No buttons
    pub mouse_selecting: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<(u16, u16)>,
    terminal_size: (u16, u16), // (width, height)
    pub menu_system: MenuSystem,
    scrollbar_dragging: bool,
    file_picker_scrollbar_dragging: bool,
    pub tree_view: Option<TreeView>,
    pub sidebar_width: u16,
    pub sidebar_resizing: bool,
    pub focus_mode: FocusMode,
    tree_scrollbar_dragging: bool,
    pub status_message: Option<String>,
    status_message_expires: Option<Instant>,
    pending_delete_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusMode {
    Editor,
    TreeView,
}

impl App {
    pub fn new() -> Self {
        // Initialize tree view with current working directory
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let tree_view = TreeView::new(current_dir, 30).ok();
        
        Self {
            tab_manager: TabManager::new(),
            running: true,
            ui: UI::new(),
            warning_message: None,
            pending_close: false,
            pending_quit: false,
            warning_selected_button: 0, // Default to "No" (safer)
            warning_is_info: false,
            mouse_selecting: false,
            last_click_time: None,
            last_click_pos: None,
            terminal_size: (80, 24), // Default size, will be updated during draw
            menu_system: MenuSystem::new(),
            scrollbar_dragging: false,
            file_picker_scrollbar_dragging: false,
            tree_view,
            sidebar_width: 30,
            sidebar_resizing: false,
            focus_mode: FocusMode::Editor,
            tree_scrollbar_dragging: false,
            status_message: None,
            status_message_expires: None,
            pending_delete_path: None,
        }
    }
    
    pub fn set_status_message(&mut self, message: String, duration: Duration) {
        self.status_message = Some(message);
        self.status_message_expires = Some(Instant::now() + duration);
    }
    
    pub fn update_status_message(&mut self) {
        if let Some(expires) = self.status_message_expires {
            if Instant::now() > expires {
                self.status_message = None;
                self.status_message_expires = None;
            }
        }
    }

    pub fn handle_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::Quit => self.handle_quit(),
            EditorCommand::Save => self.save_current_file(),
            EditorCommand::NewTab => {
                let new_tab = Tab::new(format!("untitled-{}", self.tab_manager.len() + 1));
                self.tab_manager.add_tab(new_tab);
                self.expand_tree_to_current_file();
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
                let (is_markdown, in_preview_mode) = if let Some(tab) = self.tab_manager.active_tab() {
                    (tab.is_markdown(), tab.preview_mode)
                } else {
                    (false, false)
                };
                self.menu_system.toggle_main_menu(is_markdown, in_preview_mode);
            }
            EditorCommand::OpenFile => {
                // Get the current tab's file path to open picker in that directory
                let current_path = self.tab_manager.active_tab()
                    .and_then(|tab| tab.path.clone());
                self.menu_system.open_file_picker_at_path(current_path);
            }
            EditorCommand::CurrentTab => {
                self.menu_system.open_current_tab_menu();
            }
            EditorCommand::Undo => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.undo();
                }
            }
            EditorCommand::Redo => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.redo();
                }
            }
            EditorCommand::TogglePreview => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.toggle_preview_mode();
                }
            }
            EditorCommand::FocusTreeView => {
                self.focus_mode = FocusMode::TreeView;
                if let Some(tree_view) = &mut self.tree_view {
                    tree_view.is_focused = true;
                }
            }
            EditorCommand::FocusEditor => {
                self.focus_mode = FocusMode::Editor;
                if let Some(tree_view) = &mut self.tree_view {
                    tree_view.is_focused = false;
                }
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

        // Handle input dialog
        if let crate::menu::MenuState::InputDialog(_) = &self.menu_system.state {
            self.handle_input_dialog_key(key);
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
                                // Get the current tab's file path to open picker in that directory
                                let current_path = self.tab_manager.active_tab()
                                    .and_then(|tab| tab.path.clone());
                                self.menu_system.open_file_picker_at_path(current_path);
                            }
                            "close_tab" => {
                                self.handle_close_tab();
                            }
                            "close_other_tab" => {
                                self.tab_manager.close_other_tabs();
                            }
                            "toggle_preview" => {
                                if let Some(tab) = self.tab_manager.active_tab_mut() {
                                    tab.toggle_preview_mode();
                                }
                            }
                            "quit" => {
                                self.handle_quit();
                            }
                            // Tree context menu actions
                            "new_file" => {
                                if let MenuState::TreeContextMenu(ref context_state) = &self.menu_system.state {
                                    let target_path = context_state.target_path.clone();
                                    self.menu_system.open_input_dialog(
                                        "Enter filename:".to_string(),
                                        "new_file".to_string(),
                                        target_path
                                    );
                                    // Cursor is already at position 0 by default
                                } else {
                                    self.menu_system.close();
                                }
                            }
                            "new_folder" => {
                                if let MenuState::TreeContextMenu(ref context_state) = &self.menu_system.state {
                                    let target_path = context_state.target_path.clone();
                                    self.menu_system.open_input_dialog(
                                        "Enter directory name:".to_string(),
                                        "new_folder".to_string(),
                                        target_path
                                    );
                                    // Cursor is already at position 0 by default
                                } else {
                                    self.menu_system.close();
                                }
                            }
                            "rename" => {
                                if let MenuState::TreeContextMenu(ref context_state) = &self.menu_system.state {
                                    let target_path = context_state.target_path.clone();
                                    let filename = target_path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("")
                                        .to_string();
                                    self.menu_system.open_input_dialog(
                                        "Enter new name:".to_string(),
                                        "rename".to_string(),
                                        target_path
                                    );
                                    // Pre-fill with current filename and select all
                                    if let MenuState::InputDialog(ref mut input_state) = &mut self.menu_system.state {
                                        input_state.input = filename.clone();
                                        input_state.cursor_position = filename.len();
                                        input_state.selection_start = Some(0); // Select all text
                                        input_state.hovered_button = None;
                                    }
                                } else {
                                    self.menu_system.close();
                                }
                            }
                            "delete" => {
                                if let MenuState::TreeContextMenu(ref context_state) = &self.menu_system.state {
                                    let target_path = context_state.target_path.clone();
                                    let filename = target_path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("item")
                                        .to_string();
                                    let file_type = if context_state.is_directory { "directory" } else { "file" };
                                    let message = format!("Delete {} '{}'?", file_type, filename);
                                    self.warning_message = Some(message);
                                    self.warning_selected_button = 0; // Default to "No"
                                    self.warning_is_info = false; // Yes/No dialog
                                    self.pending_delete_path = Some(target_path);
                                    self.menu_system.close();
                                } else {
                                    self.menu_system.close();
                                }
                            }
                            "open" => {
                                if let MenuState::TreeContextMenu(ref context_state) = &self.menu_system.state {
                                    let file_path = context_state.target_path.clone();
                                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                                        let new_tab = Tab::from_file(file_path, &content);
                                        self.tab_manager.add_tab(new_tab);
                                        self.expand_tree_to_current_file();
                                        self.handle_command(EditorCommand::FocusEditor);
                                    }
                                    self.menu_system.close();
                                } else {
                                    self.menu_system.close();
                                }
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

        // Handle tree view navigation when focused
        if self.focus_mode == FocusMode::TreeView {
            if let Some(tree_view) = &mut self.tree_view {
                match (key.code, key.modifiers) {
                    // Tab to switch focus back to editor
                    (KeyCode::Tab, KeyModifiers::NONE) => {
                        self.handle_command(EditorCommand::FocusEditor);
                        return;
                    }
                    // Escape to exit search mode or focus editor
                    (KeyCode::Esc, KeyModifiers::NONE) => {
                        if tree_view.is_searching {
                            tree_view.stop_search();
                        } else {
                            self.handle_command(EditorCommand::FocusEditor);
                        }
                        return;
                    }
                    // Arrow keys for navigation
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        tree_view.move_selection_up();
                        let visible_height = self.terminal_size.1.saturating_sub(2) as usize; // Account for tab and status bars
                        tree_view.update_scroll(visible_height);
                        return;
                    }
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        tree_view.move_selection_down();
                        let visible_height = self.terminal_size.1.saturating_sub(2) as usize;
                        tree_view.update_scroll(visible_height);
                        return;
                    }
                    // Enter or Right to expand/open
                    (KeyCode::Enter, KeyModifiers::NONE) | (KeyCode::Right, KeyModifiers::NONE) => {
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            if selected_item.is_dir {
                                let _ = tree_view.toggle_selected();
                            } else {
                                // Open file
                                if let Ok(content) = std::fs::read_to_string(&selected_item.path) {
                                    let new_tab = Tab::from_file(selected_item.path.clone(), &content);
                                    self.tab_manager.add_tab(new_tab);
                                    self.expand_tree_to_current_file();
                                    self.handle_command(EditorCommand::FocusEditor);
                                }
                            }
                        }
                        return;
                    }
                    // Left to collapse or go up
                    (KeyCode::Left, KeyModifiers::NONE) => {
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            if selected_item.is_dir {
                                let _ = tree_view.toggle_selected();
                            }
                        }
                        return;
                    }
                    // Backspace in search mode
                    (KeyCode::Backspace, KeyModifiers::NONE) => {
                        if tree_view.is_searching {
                            tree_view.remove_search_char();
                        }
                        return;
                    }
                    // File management shortcuts
                    (KeyCode::Char('n'), KeyModifiers::NONE) => {
                        // New file
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            let target_path = if selected_item.is_dir {
                                selected_item.path.clone()
                            } else {
                                selected_item.path.parent().unwrap_or(&selected_item.path).to_path_buf()
                            };
                            self.menu_system.open_input_dialog(
                                "Enter filename:".to_string(),
                                "new_file".to_string(),
                                target_path
                            );
                            // Cursor is already at position 0 by default, ready to type
                        }
                        return;
                    }
                    (KeyCode::Char('d'), KeyModifiers::NONE) => {
                        // New directory
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            let target_path = if selected_item.is_dir {
                                selected_item.path.clone()
                            } else {
                                selected_item.path.parent().unwrap_or(&selected_item.path).to_path_buf()
                            };
                            self.menu_system.open_input_dialog(
                                "Enter directory name:".to_string(),
                                "new_folder".to_string(),
                                target_path
                            );
                            // Cursor is already at position 0 by default, ready to type
                        }
                        return;
                    }
                    (KeyCode::F(5), KeyModifiers::NONE) | (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                        // Refresh tree view
                        if let Some(ref mut tree_view) = self.tree_view {
                            let current_selected = tree_view.get_selected_item().map(|item| item.path.clone());
                            tree_view.refresh();
                            
                            // Try to restore selection
                            if let Some(path) = current_selected {
                                tree_view.restore_selection(&path);
                            }
                            
                            // Show status message with more detail
                            let visible_count = tree_view.get_visible_items().len();
                            self.set_status_message(
                                format!("Tree view refreshed ({} items)", visible_count),
                                Duration::from_secs(2)
                            );
                        }
                        return;
                    }
                    (KeyCode::Char('r'), KeyModifiers::NONE) => {
                        // Rename
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            let filename = selected_item.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            // Pre-fill with current name
                            self.menu_system.open_input_dialog(
                                "Enter new name:".to_string(),
                                "rename".to_string(),
                                selected_item.path.clone()
                            );
                            // Set the input to current filename and select all
                            if let MenuState::InputDialog(ref mut input_state) = &mut self.menu_system.state {
                                input_state.input = filename.clone();
                                input_state.cursor_position = filename.len();
                                input_state.selection_start = Some(0); // Select all text
                                input_state.hovered_button = None;
                            }
                        }
                        return;
                    }
                    (KeyCode::Delete, KeyModifiers::NONE) => {
                        // Delete with confirmation
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            let filename = selected_item.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("item")
                                .to_string();
                            let file_type = if selected_item.is_dir { "directory" } else { "file" };
                            let message = format!("Delete {} '{}'?", file_type, filename);
                            self.warning_message = Some(message);
                            self.warning_selected_button = 0; // Default to "No"
                            self.warning_is_info = false; // Yes/No dialog
                            self.pending_delete_path = Some(selected_item.path.clone());
                        }
                        return;
                    }
                    // Context menu
                    (KeyCode::Char(' '), KeyModifiers::NONE) => {
                        // Space bar opens context menu
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            // Calculate position - roughly center of tree view
                            let tree_area_start_x = 0;
                            let tree_area_start_y = 1; // After tab bar
                            let menu_x = tree_area_start_x + 10;
                            let menu_y = tree_area_start_y + 5;
                            
                            self.menu_system.open_tree_context_menu(
                                selected_item.path.clone(),
                                selected_item.is_dir,
                                (menu_x, menu_y)
                            );
                        }
                        return;
                    }
                    // Any character starts search or adds to search
                    (KeyCode::Char(c), KeyModifiers::NONE) => {
                        if !tree_view.is_searching {
                            tree_view.start_search();
                        }
                        tree_view.add_search_char(c);
                        return;
                    }
                    _ => {}
                }
            }
        }

        // Tab handling for focus switching when not in tree view
        if self.focus_mode == FocusMode::Editor {
            match (key.code, key.modifiers) {
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    // Only switch to tree view if it exists
                    if self.tree_view.is_some() {
                        self.handle_command(EditorCommand::FocusTreeView);
                        return;
                    } else {
                        // No tree view, insert tab in editor
                        if let Some(tab) = self.tab_manager.active_tab_mut() {
                            if tab.cursor.has_selection() {
                                Self::delete_selection(&mut tab.buffer, &mut tab.cursor);
                            }
                            Self::insert_tab(&mut tab.buffer, &mut tab.cursor);
                            tab.mark_modified();
                        }
                        return;
                    }
                }
                _ => {}
            }
        }

        let needs_viewport_update = if let Some(tab) = self.tab_manager.active_tab_mut() {
            // In markdown preview mode, disable most editor commands except navigation
            if tab.preview_mode && tab.is_markdown() {
                // Only allow basic navigation and preview toggle commands
                match (key.code, key.modifiers) {
                    // Allow Ctrl+U to toggle preview mode
                    (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        tab.toggle_preview_mode();
                        return;
                    }
                    // Allow basic scrolling movements (without selection)
                    (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right, KeyModifiers::NONE) => {
                        // Allow cursor movement for potential future feature (like copy from preview)
                        // But no actual editing or selection
                        return;
                    }
                    // Allow non-editing control commands to pass through
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) |  // Quit
                    (KeyCode::Char('p'), KeyModifiers::CONTROL) |  // Open file
                    (KeyCode::Char('s'), KeyModifiers::CONTROL) |  // Save
                    (KeyCode::Char('n'), KeyModifiers::CONTROL) |  // New tab
                    (KeyCode::Char('w'), KeyModifiers::CONTROL) |  // Close tab
                    (KeyCode::Tab, KeyModifiers::CONTROL) |        // Next tab
                    (KeyCode::BackTab, KeyModifiers::NONE) |       // Previous tab
                    (KeyCode::Char('g'), KeyModifiers::CONTROL) |  // Current tab menu
                    (KeyCode::Esc, KeyModifiers::NONE) => {        // Escape
                        // Let these commands pass through to normal handling
                    }
                    // Check for selection attempts and show warning
                    (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right, KeyModifiers::SHIFT) |
                    (KeyCode::Char('a'), KeyModifiers::CONTROL) |
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) |
                    (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                        self.set_status_message(
                            "Selection disabled in preview mode. Press Ctrl+U to edit.".to_string(),
                            Duration::from_secs(3)
                        );
                        return;
                    }
                    // Block editing commands in preview mode
                    _ => {
                        self.set_status_message(
                            "Editing disabled in preview mode. Press Ctrl+U to edit.".to_string(),
                            Duration::from_secs(3)
                        );
                        return;
                    }
                }
            }
            
            let old_cursor_pos = tab.cursor.position.clone();
            
            // Check if this is a modification command that needs state saving
            let should_save_state = matches!(key.code, 
                crossterm::event::KeyCode::Backspace |
                crossterm::event::KeyCode::Delete |
                crossterm::event::KeyCode::Enter |
                crossterm::event::KeyCode::Tab
            ) || (matches!(key.code, crossterm::event::KeyCode::Char(_)) 
                && !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::SUPER)
                && !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::META))
            || (matches!(key.code, crossterm::event::KeyCode::Char('v')) 
                && key.modifiers.contains(KeyModifiers::CONTROL))
            || (matches!(key.code, crossterm::event::KeyCode::Char('x')) 
                && key.modifiers.contains(KeyModifiers::CONTROL));
            
            // Save state before modifications
            if should_save_state {
                tab.save_state();
            }
            
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
            
            // Handle scroll in file picker if it's open
            if matches!(self.menu_system.state, crate::menu::MenuState::FilePicker(_)) {
                self.handle_mouse_on_file_picker(mouse);
                return;
            }
            
            // Check if scroll is in tree view area first
            if self.handle_mouse_on_tree_view(mouse) {
                return;
            }
            
            // Handle scroll for editor (only if not handled by tree view)
            self.handle_editor_scroll(mouse.kind);
            return;
        }
        
        // Handle mouse events on warning dialog first
        if self.warning_message.is_some() {
            self.handle_mouse_on_dialog(mouse);
            return;
        }
        
        // Handle input dialog mouse events
        if let crate::menu::MenuState::InputDialog(_) = &self.menu_system.state {
            if self.handle_mouse_on_input_dialog(mouse) {
                return;
            }
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

        // Handle sidebar resize border first (highest priority)
        if self.handle_sidebar_resize(mouse) {
            return;
        }

        // Handle tree view mouse events
        if self.handle_mouse_on_tree_view(mouse) {
            return;
        }

        self.handle_mouse_on_editor(mouse);
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        // Update terminal size
        self.terminal_size = (frame.area().width, frame.area().height);
        
        // Update status message (remove if expired)
        self.update_status_message();
        
        self.ui.draw(frame, &mut self.tab_manager, &self.warning_message, self.warning_selected_button, self.warning_is_info, &self.menu_system, &self.tree_view, self.sidebar_width, &self.focus_mode, &self.status_message);
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
                if self.warning_is_info {
                    // Info dialog - just close it
                    self.warning_message = None;
                    self.warning_selected_button = 0;
                    self.warning_is_info = false;
                } else if self.warning_selected_button == 1 {
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
                    } else if let Some(delete_path) = &self.pending_delete_path {
                        // Delete the file/directory
                        let path_to_delete = delete_path.clone();
                        self.pending_delete_path = None;
                        
                        if let Some(tree_view) = &mut self.tree_view {
                            match tree_view.delete_file_or_directory(&path_to_delete) {
                                Ok(()) => {
                                    let filename = path_to_delete.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("item");
                                    self.set_status_message(
                                        format!("Deleted '{}'", filename),
                                        Duration::from_secs(3)
                                    );
                                }
                                Err(e) => {
                                    self.set_status_message(
                                        format!("Failed to delete: {}", e),
                                        Duration::from_secs(5)
                                    );
                                }
                            }
                        }
                    }
                } else {
                    // No button selected - cancel
                    self.warning_message = None;
                    self.pending_close = false;
                    self.pending_quit = false;
                    self.pending_delete_path = None;
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
                } else if let Some(delete_path) = &self.pending_delete_path {
                    // Delete the file/directory
                    let path_to_delete = delete_path.clone();
                    self.pending_delete_path = None;
                    
                    if let Some(tree_view) = &mut self.tree_view {
                        match tree_view.delete_file_or_directory(&path_to_delete) {
                            Ok(()) => {
                                let filename = path_to_delete.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("item");
                                self.set_status_message(
                                    format!("Deleted '{}'", filename),
                                    Duration::from_secs(3)
                                );
                            }
                            Err(e) => {
                                self.set_status_message(
                                    format!("Failed to delete: {}", e),
                                    Duration::from_secs(5)
                                );
                            }
                        }
                    }
                }
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) | (KeyCode::Char('N'), KeyModifiers::NONE) | (KeyCode::Esc, KeyModifiers::NONE) => {
                // User cancelled
                self.warning_message = None;
                self.pending_close = false;
                self.pending_quit = false;
                self.pending_delete_path = None;
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
        
        // Check if interaction is on scrollbar (rightmost column in editor area)
        if let Some(tab) = self.tab_manager.active_tab() {
            let content_lines = if tab.preview_mode && tab.is_markdown() {
                // For markdown preview, count the rendered lines
                let content = tab.buffer.to_string();
                let markdown_widget = crate::markdown_widget::MarkdownWidget::new(&content);
                markdown_widget.parse_markdown().len()
            } else {
                // For normal editor, use buffer lines
                tab.buffer.len_lines()
            };
            
            let has_scrollbar = content_lines > (self.terminal_size.1 as usize).saturating_sub(2);
            if has_scrollbar && mouse.column == self.terminal_size.0.saturating_sub(1) && mouse.row > 0 && (mouse.row as usize) < (self.terminal_size.1 as usize).saturating_sub(1) {
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

        // Stop scrollbar dragging if mouse is released anywhere
        if let MouseEventKind::Up(MouseButton::Left) = mouse.kind {
            self.scrollbar_dragging = false;
            self.file_picker_scrollbar_dragging = false;
        }
        
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
                            self.expand_tree_to_current_file();
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
                
                // Check if we're in markdown preview mode - disable selection if so
                if let Some(tab) = self.tab_manager.active_tab() {
                    if tab.preview_mode && tab.is_markdown() {
                        // Show warning message and return
                        self.set_status_message(
                            "Selection disabled in preview mode. Press Ctrl+U to edit.".to_string(),
                            Duration::from_secs(3)
                        );
                        return;
                    }
                }
                
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
                // Check if we're in markdown preview mode - disable selection if so
                if let Some(tab) = self.tab_manager.active_tab() {
                    if tab.preview_mode && tab.is_markdown() {
                        self.set_status_message(
                            "Selection disabled in preview mode. Press Ctrl+U to edit.".to_string(),
                            Duration::from_secs(3)
                        );
                        return;
                    }
                }
                
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
                // Check if we're in markdown preview mode - disable selection if so
                if let Some(tab) = self.tab_manager.active_tab() {
                    if tab.preview_mode && tab.is_markdown() {
                        // Don't show message on mouse up to avoid spam
                        return;
                    }
                }
                
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
                        } else if let Some(delete_path) = &self.pending_delete_path {
                            // Delete the file/directory
                            let path_to_delete = delete_path.clone();
                            self.pending_delete_path = None;
                            
                            if let Some(tree_view) = &mut self.tree_view {
                                match tree_view.delete_file_or_directory(&path_to_delete) {
                                    Ok(()) => {
                                        let filename = path_to_delete.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("item");
                                        self.set_status_message(
                                            format!("Deleted '{}'", filename),
                                            Duration::from_secs(3)
                                        );
                                    }
                                    Err(e) => {
                                        self.set_status_message(
                                            format!("Failed to delete: {}", e),
                                            Duration::from_secs(5)
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // No button clicked - cancel
                        self.warning_message = None;
                        self.pending_close = false;
                        self.pending_quit = false;
                        self.pending_delete_path = None;
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
                    crate::menu::MenuState::TreeContextMenu(context_state) => {
                        let menu_area = ratatui::layout::Rect {
                            x: context_state.position.0,
                            y: context_state.position.1,
                            width: context_state.menu.width,
                            height: context_state.menu.height,
                        };
                        
                        let hovered_item = context_state.menu.get_clicked_item(&menu_area, mouse.column, mouse.row);
                        
                        if let crate::menu::MenuState::TreeContextMenu(context_state) = &mut self.menu_system.state {
                            context_state.menu.hovered_index = hovered_item;
                        }
                        return true;
                    }
                    _ => {}
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is on F1 menu button in status bar
                if self.is_f1_button_clicked(mouse.column, mouse.row) {
                    let (is_markdown, in_preview_mode) = if let Some(tab) = self.tab_manager.active_tab() {
                        (tab.is_markdown(), tab.preview_mode)
                    } else {
                        (false, false)
                    };
                    self.menu_system.toggle_main_menu(is_markdown, in_preview_mode);
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
                                        "open_file" => {
                                            let current_path = self.tab_manager.active_tab()
                                                .and_then(|tab| tab.path.clone());
                                            self.menu_system.open_file_picker_at_path(current_path);
                                        }
                                        "toggle_preview" => {
                                            if let Some(tab) = self.tab_manager.active_tab_mut() {
                                                tab.toggle_preview_mode();
                                            }
                                        }
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
                    crate::menu::MenuState::TreeContextMenu(context_state) => {
                        let menu_area = ratatui::layout::Rect {
                            x: context_state.position.0,
                            y: context_state.position.1,
                            width: context_state.menu.width,
                            height: context_state.menu.height,
                        };
                        if let Some(item_index) = context_state.menu.get_clicked_item(&menu_area, mouse.column, mouse.row) {
                            // Extract needed information before modifying the menu state
                            let target_path = context_state.target_path.clone();
                            let is_directory = context_state.is_directory;
                            
                            // Update selection and handle click
                            if let crate::menu::MenuState::TreeContextMenu(context_state) = &mut self.menu_system.state {
                                context_state.menu.selected_index = item_index;
                                if let Some(action) = self.menu_system.handle_enter() {
                                    match action.as_str() {
                                        "new_file" => {
                                            self.menu_system.open_input_dialog(
                                                "Enter filename:".to_string(),
                                                "new_file".to_string(),
                                                target_path
                                            );
                                            // Cursor is already at position 0 by default, ready to type
                                        }
                                        "new_folder" => {
                                            self.menu_system.open_input_dialog(
                                                "Enter directory name:".to_string(),
                                                "new_folder".to_string(),
                                                target_path
                                            );
                                            // Cursor is already at position 0 by default, ready to type
                                        }
                                        "rename" => {
                                            let filename = target_path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("")
                                                .to_string();
                                            self.menu_system.open_input_dialog(
                                                "Enter new name:".to_string(),
                                                "rename".to_string(),
                                                target_path
                                            );
                                            // Pre-fill with current filename and select all
                                            if let crate::menu::MenuState::InputDialog(ref mut input_state) = &mut self.menu_system.state {
                                                input_state.input = filename.clone();
                                                input_state.cursor_position = filename.len();
                                                input_state.selection_start = Some(0); // Select all text
                                                input_state.hovered_button = None;
                                            }
                                        }
                                        "delete" => {
                                            let filename = target_path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("item")
                                                .to_string();
                                            let file_type = if is_directory { "directory" } else { "file" };
                                            let message = format!("Delete {} '{}'?", file_type, filename);
                                            self.warning_message = Some(message);
                                            self.warning_selected_button = 0; // Default to "No"
                                            self.warning_is_info = false; // Yes/No dialog
                                            self.pending_delete_path = Some(target_path);
                                            self.menu_system.close();
                                        }
                                        "open" => {
                                            if let Ok(content) = std::fs::read_to_string(&target_path) {
                                                let new_tab = Tab::from_file(target_path, &content);
                                                self.tab_manager.add_tab(new_tab);
                                                self.expand_tree_to_current_file();
                                                self.handle_command(EditorCommand::FocusEditor);
                                            }
                                            self.menu_system.close();
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
                
                // If we get here, the click was not on any menu, so close any open context menu
                if matches!(self.menu_system.state, crate::menu::MenuState::TreeContextMenu(_)) {
                    self.menu_system.close();
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
        // Use the terminal width for calculation
        let available_width = self.terminal_size.0 as usize;
        self.ui.tab_bar.get_tab_x_position(&self.tab_manager, target_tab_index, available_width)
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
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                // Handle scroll in file picker
                if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                    let scroll_amount = 3;
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            picker_state.selected_index = picker_state.selected_index.saturating_sub(scroll_amount);
                            picker_state.hovered_index = None; // Clear hover when scrolling
                        }
                        MouseEventKind::ScrollDown => {
                            let max_index = picker_state.filtered_items.len().saturating_sub(1);
                            picker_state.selected_index = (picker_state.selected_index + scroll_amount).min(max_index);
                            picker_state.hovered_index = None; // Clear hover when scrolling
                        }
                        _ => {}
                    }
                    return true;
                }
            }
            MouseEventKind::Moved => {
                // Handle hover on file picker items
                if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                    // Calculate file picker modal area (same as in UI draw method)
                    let modal_width = 70u16.min(self.terminal_size.0.saturating_sub(4));
                    let modal_height = 24u16.min(self.terminal_size.1.saturating_sub(4));
                    let modal_x = (self.terminal_size.0.saturating_sub(modal_width)) / 2;
                    let modal_y = (self.terminal_size.1.saturating_sub(modal_height)) / 2;
                    
                    // Check if mouse is within modal bounds
                    if mouse.column >= modal_x && mouse.column < modal_x + modal_width &&
                       mouse.row >= modal_y && mouse.row < modal_y + modal_height {
                        
                        // Calculate file list area (accounting for margin and search input)
                        let list_start_y = modal_y + 1 + 1 + 1 + 1; // border + margin + current dir + search + separator
                        let is_searching = !picker_state.search_query.is_empty();
                        let items_per_entry = if is_searching { 2 } else { 1 };
                        let list_height = modal_height.saturating_sub(5); // subtract borders and other elements
                        
                        // Check if click is on scrollbar (rightmost column of file list area)
                        let total_items = picker_state.filtered_items.len();
                        let has_scrollbar = total_items * items_per_entry > list_height as usize;
                        if has_scrollbar && mouse.column == modal_x + modal_width - 2 && // -2 for border and scrollbar position
                           mouse.row >= list_start_y && mouse.row < list_start_y + list_height {
                            // This is a scrollbar interaction, handle it based on event type
                            match mouse.kind {
                                MouseEventKind::Down(MouseButton::Left) => {
                                    self.file_picker_scrollbar_dragging = true;
                                    self.handle_file_picker_scrollbar_click(mouse);
                                    return true;
                                }
                                MouseEventKind::Drag(MouseButton::Left) => {
                                    if self.file_picker_scrollbar_dragging {
                                        self.handle_file_picker_scrollbar_click(mouse);
                                        return true; // Consume the drag event
                                    } else {
                                        return true; // Not our drag, ignore
                                    }
                                }
                                MouseEventKind::Up(MouseButton::Left) => {
                                    if self.file_picker_scrollbar_dragging {
                                        self.file_picker_scrollbar_dragging = false;
                                    }
                                    return true; // Consume the event
                                }
                                _ => return true, // Consume other scrollbar events
                            }
                        }
                        
                        if mouse.row >= list_start_y && mouse.row < list_start_y + list_height {
                            // Calculate which item is being hovered
                            let relative_y = (mouse.row - list_start_y) as usize;
                            let item_index = relative_y / items_per_entry;
                            let visible_items = (list_height as usize) / items_per_entry;
                            
                            let start_index = if picker_state.selected_index >= visible_items {
                                picker_state.selected_index.saturating_sub(visible_items - 1)
                            } else {
                                0
                            };
                            
                            let hovered_index = start_index + item_index;
                            
                            if hovered_index < picker_state.filtered_items.len() {
                                // Only update hovered index, not selected index
                                picker_state.hovered_index = Some(hovered_index);
                            }
                        } else {
                            // Mouse is inside modal but not on any item
                            picker_state.hovered_index = None;
                        }
                        
                        return true; // Mouse is inside modal
                    } else {
                        // Mouse is outside modal
                        picker_state.hovered_index = None;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // First, get the information we need from the picker state
                let click_info = if let crate::menu::MenuState::FilePicker(picker_state) = &self.menu_system.state {
                    // Calculate file picker modal area (same as in UI draw method)
                    let modal_width = 70u16.min(self.terminal_size.0.saturating_sub(4));
                    let modal_height = 24u16.min(self.terminal_size.1.saturating_sub(4));
                    let modal_x = (self.terminal_size.0.saturating_sub(modal_width)) / 2;
                    let modal_y = (self.terminal_size.1.saturating_sub(modal_height)) / 2;
                    
                    // Check if click is within modal bounds
                    if mouse.column >= modal_x && mouse.column < modal_x + modal_width &&
                       mouse.row >= modal_y && mouse.row < modal_y + modal_height {
                        
                        // Calculate file list area (accounting for margin and search input)
                        let list_start_y = modal_y + 1 + 1 + 1 + 1; // border + margin + current dir + search + separator
                        let is_searching = !picker_state.search_query.is_empty();
                        let items_per_entry = if is_searching { 2 } else { 1 };
                        let list_height = modal_height.saturating_sub(5); // subtract borders and other elements
                        
                        // Check if click is on scrollbar (rightmost column of file list area)
                        let total_items = picker_state.filtered_items.len();
                        let has_scrollbar = total_items * items_per_entry > list_height as usize;
                        if has_scrollbar && mouse.column == modal_x + modal_width - 2 && // -2 for border and scrollbar position
                           mouse.row >= list_start_y && mouse.row < list_start_y + list_height {
                            // Handle scrollbar click - calculate new position and return it
                            self.file_picker_scrollbar_dragging = true;
                            let click_y = (mouse.row - list_start_y) as usize;
                            let visible_items = (list_height as usize) / items_per_entry;
                            
                            let scrollbar_state = ScrollbarState::new(
                                total_items,
                                visible_items,
                                picker_state.selected_index.saturating_sub(visible_items.saturating_sub(1)),
                            );
                            
                            let new_position = scrollbar_state.click_position(list_height as usize, click_y);
                            let new_selected_index = new_position.min(total_items.saturating_sub(1));
                            
                            Some((FileItem { path: PathBuf::new(), name: String::new(), is_dir: false, relative_path: String::new() }, false, new_selected_index)) // false means scrollbar click, not file selection
                        } else if mouse.row >= list_start_y && mouse.row < list_start_y + list_height {
                            // Calculate which file was clicked
                            let relative_y = (mouse.row - list_start_y) as usize;
                            let item_index = relative_y / items_per_entry;
                            let visible_items = (list_height as usize) / items_per_entry;
                            
                            let start_index = if picker_state.selected_index >= visible_items {
                                picker_state.selected_index.saturating_sub(visible_items - 1)
                            } else {
                                0
                            };
                            
                            let clicked_file_index = start_index + item_index;
                            
                            if clicked_file_index < picker_state.filtered_items.len() {
                                // Update selected_index to match the click
                                let selected_item = picker_state.filtered_items[clicked_file_index].clone();
                                Some((selected_item, true, clicked_file_index)) // true means valid click
                            } else {
                                Some((FileItem { path: PathBuf::new(), name: String::new(), is_dir: false, relative_path: String::new() }, false, 0)) // false means invalid click but inside modal
                            }
                        } else {
                            Some((FileItem { path: PathBuf::new(), name: String::new(), is_dir: false, relative_path: String::new() }, false, 0)) // Click was inside modal but not on a file
                        }
                    } else {
                        None // Click was outside modal
                    }
                } else {
                    None
                };
                
                // Now handle the click based on the information we collected
                if let Some((selected_item, valid_click, clicked_index)) = click_info {
                    // Always update selected_index (for both file clicks and scrollbar clicks)
                    if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                        picker_state.selected_index = clicked_index;
                        picker_state.hovered_index = None; // Clear hover on click
                    }
                    
                    if valid_click {
                        // Only process file/directory selection for valid file clicks
                        
                        if selected_item.is_dir {
                            // Handle directory click - enter the directory
                            if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
                                picker_state.enter_directory(selected_item.path.clone());
                            }
                        } else {
                            // Handle file click - open the file
                            match std::fs::read(&selected_item.path) {
                                Ok(bytes) => {
                                    // Try to convert to string, if it fails show warning
                                    match String::from_utf8(bytes) {
                                        Ok(text) => {
                                            // Valid text file - open it using from_file to set preview mode for markdown
                                            let new_tab = crate::tab::Tab::from_file(selected_item.path.clone(), &text);
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
                                            self.warning_is_info = true; // This is an info dialog, not a confirmation
                                            // Close file picker but don't open the file
                                            self.menu_system.close();
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Could not read file - show error warning
                                    self.warning_message = Some(format!(
                                        "Cannot open '{}': {}",
                                        selected_item.name, e
                                    ));
                                    self.warning_selected_button = 0;
                                    self.warning_is_info = true; // This is an info dialog
                                    self.menu_system.close();
                                }
                            }
                        }
                    }
                    return true; // Click was inside modal
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
                                            let new_tab = crate::tab::Tab::from_file(selected_item.path.clone(), &text);
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
                                            self.warning_is_info = true; // This is an info dialog, not a confirmation
                                            // Close file picker but don't open the file
                                            self.menu_system.close();
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Could not read file - show error warning
                                    self.warning_message = Some(format!(
                                        "Cannot open '{}': {}",
                                        selected_item.name, e
                                    ));
                                    self.warning_selected_button = 0;
                                    self.warning_is_info = true; // This is an info dialog
                                    self.menu_system.close();
                                }
                            }
                        }
                    }
                }
                (KeyCode::Left, KeyModifiers::NONE) => {
                    // Go up one directory
                    picker_state.go_up();
                }
                (KeyCode::Up, KeyModifiers::NONE) => {
                    picker_state.move_selection_up();
                }
                (KeyCode::Down, KeyModifiers::NONE) => {
                    picker_state.move_selection_down();
                }
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    if picker_state.search_query.is_empty() {
                        // If search is empty, go up directory
                        picker_state.go_up();
                    } else {
                        // Remove last character from search
                        picker_state.search_query.pop();
                        picker_state.update_filter();
                    }
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

    fn handle_scrollbar_click(&mut self, mouse: MouseEvent) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            let editor_height = (self.terminal_size.1 as usize).saturating_sub(2); // Tab bar + status bar
            let click_y = (mouse.row as usize).saturating_sub(1); // Subtract tab bar
            
            let content_lines = if tab.preview_mode && tab.is_markdown() {
                // For markdown preview, count the rendered lines
                let content = tab.buffer.to_string();
                let markdown_widget = crate::markdown_widget::MarkdownWidget::new(&content);
                markdown_widget.parse_markdown().len()
            } else {
                // For normal editor, use buffer lines
                tab.buffer.len_lines()
            };
            
            // Create scrollbar state to calculate click position
            let scrollbar_state = ScrollbarState::new(
                content_lines,
                editor_height,
                tab.viewport_offset.0,
            );
            
            // Calculate new scroll position based on click
            let new_position = scrollbar_state.click_position(editor_height, click_y);
            
            // Update viewport offset
            tab.viewport_offset.0 = new_position;
        }
    }

    fn handle_mouse_on_input_dialog(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        if let crate::menu::MenuState::InputDialog(input_state) = &mut self.menu_system.state {
            // Calculate dialog position (same logic as in UI module)
            let dialog_width = 50u16.min(self.terminal_size.0.saturating_sub(4));
            let dialog_height = 8; // Updated to match UI spacing
            let dialog_x = (self.terminal_size.0.saturating_sub(dialog_width)) / 2;
            let dialog_y = (self.terminal_size.1.saturating_sub(dialog_height)) / 2;
            
            // Check if click is within dialog bounds
            if mouse.column < dialog_x || mouse.column >= dialog_x + dialog_width ||
               mouse.row < dialog_y || mouse.row >= dialog_y + dialog_height {
                // Click outside dialog - could close it or ignore
                return false;
            }
            
            // Calculate input field position (row 2 of inner dialog, with margin)
            let input_row = dialog_y + 3; // border + margin + title + prompt = 3
            let button_row = dialog_y + 5; // After input field and spacing
            
            // Handle button hover and clicks
            if mouse.row == button_row {
                match mouse.kind {
                    MouseEventKind::Moved => {
                        // Calculate button positions (centered)
                        let button_area_start = dialog_x + (dialog_width / 2) - 15;
                        let ok_start = button_area_start;
                        let ok_end = ok_start + 13; // " [Enter] OK  " = 13 chars
                        let cancel_start = ok_end + 2; // 2 spaces between buttons
                        let cancel_end = cancel_start + 14; // " [Esc] Cancel " = 14 chars
                        
                        if mouse.column >= ok_start && mouse.column < ok_end {
                            input_state.hovered_button = Some(0); // Hovering OK
                        } else if mouse.column >= cancel_start && mouse.column < cancel_end {
                            input_state.hovered_button = Some(1); // Hovering Cancel
                        } else {
                            input_state.hovered_button = None;
                        }
                        return true;
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Calculate button positions (centered)
                        let button_area_start = dialog_x + (dialog_width / 2) - 15;
                        let ok_start = button_area_start;
                        let ok_end = ok_start + 13; // " [Enter] OK  " = 13 chars
                        let cancel_start = ok_end + 2; // 2 spaces between buttons
                        let cancel_end = cancel_start + 14; // " [Esc] Cancel " = 14 chars
                        
                        if mouse.column >= ok_start && mouse.column < ok_end {
                            // OK button clicked - execute operation
                            let operation = input_state.operation.clone();
                            let target_path = input_state.target_path.clone();
                            let input_text = input_state.input.clone();
                            
                            // Close the dialog first
                            self.menu_system.close();
                            
                            // Perform the operation
                            if !input_text.trim().is_empty() {
                                self.execute_file_operation(&operation, &target_path, &input_text);
                            }
                            return true;
                        } else if mouse.column >= cancel_start && mouse.column < cancel_end {
                            // Cancel button clicked
                            self.menu_system.close();
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            
            // Clear button hover if mouse is elsewhere
            if mouse.row != button_row && matches!(mouse.kind, MouseEventKind::Moved) {
                input_state.hovered_button = None;
            }
            
            // Handle input field clicks
            if mouse.row == input_row {
                let input_start_x = dialog_x + 1; // margin
                let relative_x = mouse.column.saturating_sub(input_start_x) as usize;
                
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Set cursor position based on click
                        if relative_x <= input_state.input.len() {
                            input_state.cursor_position = relative_x;
                            input_state.selection_start = Some(relative_x); // Start selection
                        } else {
                            input_state.cursor_position = input_state.input.len();
                            input_state.selection_start = Some(input_state.input.len());
                        }
                        
                        // Check for double-click
                        static mut LAST_CLICK_TIME: Option<std::time::Instant> = None;
                        static mut LAST_CLICK_POS: usize = 0;
                        
                        let now = std::time::Instant::now();
                        let is_double_click = unsafe {
                            if let Some(last_time) = LAST_CLICK_TIME {
                                let same_pos = LAST_CLICK_POS == input_state.cursor_position;
                                now.duration_since(last_time) < std::time::Duration::from_millis(500) && same_pos
                            } else {
                                false
                            }
                        };
                        
                        if is_double_click {
                            // Double-click: select word at cursor
                            Self::select_word_at_cursor(input_state);
                            unsafe {
                                LAST_CLICK_TIME = None; // Reset to prevent triple-click
                            }
                        } else {
                            unsafe {
                                LAST_CLICK_TIME = Some(now);
                                LAST_CLICK_POS = input_state.cursor_position;
                            }
                        }
                        
                        return true;
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        // Update cursor position while maintaining selection
                        if relative_x <= input_state.input.len() {
                            input_state.cursor_position = relative_x;
                        } else {
                            input_state.cursor_position = input_state.input.len();
                        }
                        return true;
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        // End selection - if start == end, clear selection
                        if let Some(sel_start) = input_state.selection_start {
                            if sel_start == input_state.cursor_position {
                                input_state.selection_start = None;
                            }
                        }
                        return true;
                    }
                    _ => {}
                }
            }
            
            return true; // Mouse was in dialog area
        }
        
        false
    }
    
    fn select_word_at_cursor(input_state: &mut crate::menu::InputDialogState) {
        let text = &input_state.input;
        let pos = input_state.cursor_position;
        
        if text.is_empty() {
            return;
        }
        
        // Find word boundaries
        let chars: Vec<char> = text.chars().collect();
        
        // Find start of word
        let mut start = pos;
        while start > 0 && !chars[start - 1].is_whitespace() && !is_word_separator(chars[start - 1]) {
            start -= 1;
        }
        
        // Find end of word
        let mut end = pos;
        while end < chars.len() && !chars[end].is_whitespace() && !is_word_separator(chars[end]) {
            end += 1;
        }
        
        // Set selection
        input_state.selection_start = Some(start);
        input_state.cursor_position = end;
    }

    fn handle_input_dialog_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        if let crate::menu::MenuState::InputDialog(input_state) = &mut self.menu_system.state {
            match (key.code, key.modifiers) {
                // Character input
                (KeyCode::Char(c), KeyModifiers::NONE) => {
                    // Delete selection if any
                    if input_state.selection_start.is_some() {
                        Self::delete_input_selection(input_state);
                    }
                    // Insert character at cursor position
                    input_state.input.insert(input_state.cursor_position, c);
                    input_state.cursor_position += 1;
                }
                
                // Backspace
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    if input_state.selection_start.is_some() {
                        Self::delete_input_selection(input_state);
                    } else if input_state.cursor_position > 0 {
                        input_state.cursor_position -= 1;
                        input_state.input.remove(input_state.cursor_position);
                    }
                }
                
                // Ctrl+Backspace or Alt+Backspace - delete word
                (KeyCode::Backspace, KeyModifiers::CONTROL) | (KeyCode::Backspace, KeyModifiers::ALT) => {
                    if input_state.selection_start.is_some() {
                        Self::delete_input_selection(input_state);
                    } else if input_state.cursor_position > 0 {
                        let original_pos = input_state.cursor_position;
                        let chars: Vec<char> = input_state.input.chars().collect();
                        let mut pos = input_state.cursor_position;
                        
                        // Move back to find the start of deletion
                        if pos > 0 && pos <= chars.len() {
                            pos -= 1;
                            
                            // If we're on whitespace, delete all whitespace
                            if chars[pos].is_whitespace() {
                                while pos > 0 && chars[pos - 1].is_whitespace() {
                                    pos -= 1;
                                }
                            }
                            // Otherwise, delete the word
                            else {
                                // Skip to beginning of current word
                                while pos > 0 && !chars[pos - 1].is_whitespace() && !is_word_separator(chars[pos - 1]) {
                                    pos -= 1;
                                }
                            }
                        }
                        
                        // Delete from pos to original_pos
                        let delete_count = original_pos - pos;
                        for _ in 0..delete_count {
                            if pos < input_state.input.len() {
                                input_state.input.remove(pos);
                            }
                        }
                        input_state.cursor_position = pos;
                    }
                }
                
                // Delete
                (KeyCode::Delete, KeyModifiers::NONE) => {
                    if input_state.selection_start.is_some() {
                        Self::delete_input_selection(input_state);
                    } else if input_state.cursor_position < input_state.input.len() {
                        input_state.input.remove(input_state.cursor_position);
                    }
                }
                
                // Cursor movement
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
                
                // Word movement with Ctrl+Arrow or Alt+Arrow
                (KeyCode::Left, KeyModifiers::CONTROL) | (KeyCode::Left, KeyModifiers::ALT) => {
                    if input_state.cursor_position > 0 {
                        let chars: Vec<char> = input_state.input.chars().collect();
                        let mut pos = input_state.cursor_position - 1;
                        
                        // Skip current whitespace/separators
                        while pos > 0 && (chars[pos].is_whitespace() || is_word_separator(chars[pos])) {
                            pos -= 1;
                        }
                        
                        // Move to start of word
                        while pos > 0 && !chars[pos - 1].is_whitespace() && !is_word_separator(chars[pos - 1]) {
                            pos -= 1;
                        }
                        
                        input_state.cursor_position = pos;
                    }
                    input_state.selection_start = None;
                }
                (KeyCode::Right, KeyModifiers::CONTROL) | (KeyCode::Right, KeyModifiers::ALT) => {
                    if input_state.cursor_position < input_state.input.len() {
                        let chars: Vec<char> = input_state.input.chars().collect();
                        let mut pos = input_state.cursor_position;
                        
                        // Skip to end of current word
                        while pos < chars.len() && !chars[pos].is_whitespace() && !is_word_separator(chars[pos]) {
                            pos += 1;
                        }
                        
                        // Skip whitespace/separators
                        while pos < chars.len() && (chars[pos].is_whitespace() || is_word_separator(chars[pos])) {
                            pos += 1;
                        }
                        
                        input_state.cursor_position = pos;
                    }
                    input_state.selection_start = None;
                }
                
                // Selection movement
                (KeyCode::Left, KeyModifiers::SHIFT) => {
                    if input_state.selection_start.is_none() {
                        input_state.selection_start = Some(input_state.cursor_position);
                    }
                    if input_state.cursor_position > 0 {
                        input_state.cursor_position -= 1;
                    }
                }
                (KeyCode::Right, KeyModifiers::SHIFT) => {
                    if input_state.selection_start.is_none() {
                        input_state.selection_start = Some(input_state.cursor_position);
                    }
                    if input_state.cursor_position < input_state.input.len() {
                        input_state.cursor_position += 1;
                    }
                }
                (KeyCode::Home, KeyModifiers::SHIFT) => {
                    if input_state.selection_start.is_none() {
                        input_state.selection_start = Some(input_state.cursor_position);
                    }
                    input_state.cursor_position = 0;
                }
                (KeyCode::End, KeyModifiers::SHIFT) => {
                    if input_state.selection_start.is_none() {
                        input_state.selection_start = Some(input_state.cursor_position);
                    }
                    input_state.cursor_position = input_state.input.len();
                }
                
                // Word selection with Ctrl+Shift+Arrow or Alt+Shift+Arrow
                (KeyCode::Left, modifiers) if (modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::ALT)) && modifiers.contains(KeyModifiers::SHIFT) => {
                    if input_state.selection_start.is_none() {
                        input_state.selection_start = Some(input_state.cursor_position);
                    }
                    if input_state.cursor_position > 0 {
                        let chars: Vec<char> = input_state.input.chars().collect();
                        let mut pos = input_state.cursor_position - 1;
                        
                        // Skip current whitespace/separators
                        while pos > 0 && (chars[pos].is_whitespace() || is_word_separator(chars[pos])) {
                            pos -= 1;
                        }
                        
                        // Move to start of word
                        while pos > 0 && !chars[pos - 1].is_whitespace() && !is_word_separator(chars[pos - 1]) {
                            pos -= 1;
                        }
                        
                        input_state.cursor_position = pos;
                    }
                }
                (KeyCode::Right, modifiers) if (modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::ALT)) && modifiers.contains(KeyModifiers::SHIFT) => {
                    if input_state.selection_start.is_none() {
                        input_state.selection_start = Some(input_state.cursor_position);
                    }
                    if input_state.cursor_position < input_state.input.len() {
                        let chars: Vec<char> = input_state.input.chars().collect();
                        let mut pos = input_state.cursor_position;
                        
                        // Skip to end of current word
                        while pos < chars.len() && !chars[pos].is_whitespace() && !is_word_separator(chars[pos]) {
                            pos += 1;
                        }
                        
                        // Skip whitespace/separators
                        while pos < chars.len() && (chars[pos].is_whitespace() || is_word_separator(chars[pos])) {
                            pos += 1;
                        }
                        
                        input_state.cursor_position = pos;
                    }
                }
                
                // Select all
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    input_state.selection_start = Some(0);
                    input_state.cursor_position = input_state.input.len();
                }
                
                // Ctrl+W - Unix-style delete word backwards
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                    if input_state.selection_start.is_some() {
                        Self::delete_input_selection(input_state);
                    } else if input_state.cursor_position > 0 {
                        let original_pos = input_state.cursor_position;
                        let chars: Vec<char> = input_state.input.chars().collect();
                        let mut pos = input_state.cursor_position;
                        
                        // Move back to find the start of deletion
                        if pos > 0 && pos <= chars.len() {
                            pos -= 1;
                            
                            // If we're on whitespace, delete all whitespace
                            if chars[pos].is_whitespace() {
                                while pos > 0 && chars[pos - 1].is_whitespace() {
                                    pos -= 1;
                                }
                            }
                            // Otherwise, delete the word
                            else {
                                // Skip to beginning of current word
                                while pos > 0 && !chars[pos - 1].is_whitespace() && !is_word_separator(chars[pos - 1]) {
                                    pos -= 1;
                                }
                            }
                        }
                        
                        // Delete from pos to original_pos
                        let delete_count = original_pos - pos;
                        for _ in 0..delete_count {
                            if pos < input_state.input.len() {
                                input_state.input.remove(pos);
                            }
                        }
                        input_state.cursor_position = pos;
                    }
                }
                
                // Copy/Cut/Paste
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    if let Some(sel_start) = input_state.selection_start {
                        let (start, end) = if sel_start < input_state.cursor_position {
                            (sel_start, input_state.cursor_position)
                        } else {
                            (input_state.cursor_position, sel_start)
                        };
                        let selected_text: String = input_state.input.chars()
                            .skip(start)
                            .take(end - start)
                            .collect();
                        // TODO: Copy to clipboard
                        let _ = selected_text;
                    }
                }
                (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                    if input_state.selection_start.is_some() {
                        // TODO: Copy to clipboard before deleting
                        Self::delete_input_selection(input_state);
                    }
                }
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                    // TODO: Paste from clipboard
                }
                
                // Enter to submit
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    // Execute the operation
                    let operation = input_state.operation.clone();
                    let target_path = input_state.target_path.clone();
                    let input_text = input_state.input.clone();
                    
                    // Close the dialog first
                    self.menu_system.close();
                    
                    // Perform the operation
                    if !input_text.trim().is_empty() {
                        self.execute_file_operation(&operation, &target_path, &input_text);
                    }
                }
                
                // Escape to cancel
                (KeyCode::Esc, KeyModifiers::NONE) => {
                    // Cancel the operation
                    self.menu_system.close();
                }
                _ => {}
            }
        }
    }
    
    fn delete_input_selection(input_state: &mut crate::menu::InputDialogState) {
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

    fn execute_file_operation(&mut self, operation: &str, target_path: &PathBuf, input: &str) {
        if let Some(tree_view) = &mut self.tree_view {
            let result = match operation {
                "new_file" => {
                    tree_view.create_file(target_path, input.trim())
                        .map(|_| format!("Created file '{}'", input.trim()))
                        .map_err(|e| format!("Failed to create file: {}", e))
                }
                "new_folder" => {
                    tree_view.create_directory(target_path, input.trim())
                        .map(|_| format!("Created directory '{}'", input.trim()))
                        .map_err(|e| format!("Failed to create directory: {}", e))
                }
                "rename" => {
                    tree_view.rename_file_or_directory(target_path, input.trim())
                        .map(|_| format!("Renamed to '{}'", input.trim()))
                        .map_err(|e| format!("Failed to rename: {}", e))
                }
                _ => return,
            };
            
            match result {
                Ok(message) => {
                    self.set_status_message(message, Duration::from_secs(3));
                }
                Err(error) => {
                    self.set_status_message(error, Duration::from_secs(5));
                }
            }
        }
    }

    fn handle_file_picker_scrollbar_click(&mut self, mouse: MouseEvent) {
        if let crate::menu::MenuState::FilePicker(picker_state) = &mut self.menu_system.state {
            let _modal_width = 70u16.min(self.terminal_size.0.saturating_sub(4));
            let modal_height = 24u16.min(self.terminal_size.1.saturating_sub(4));
            let modal_y = (self.terminal_size.1.saturating_sub(modal_height)) / 2;
            let list_start_y = modal_y + 1 + 1 + 1 + 1; // border + margin + current dir + search + separator
            let is_searching = !picker_state.search_query.is_empty();
            let items_per_entry = if is_searching { 2 } else { 1 };
            let list_height = modal_height.saturating_sub(5); // subtract borders and other elements
            
            let click_y = (mouse.row - list_start_y) as usize;
            let visible_items = (list_height as usize) / items_per_entry;
            let total_items = picker_state.filtered_items.len();
            
            let scrollbar_state = ScrollbarState::new(
                total_items,
                visible_items,
                picker_state.selected_index.saturating_sub(visible_items.saturating_sub(1)),
            );
            
            let new_position = scrollbar_state.click_position(list_height as usize, click_y);
            let new_selected_index = new_position.min(total_items.saturating_sub(1));
            
            picker_state.selected_index = new_selected_index;
            picker_state.hovered_index = None;
        }
    }

    fn delete_selection(buffer: &mut crate::rope_buffer::RopeBuffer, cursor: &mut crate::cursor::Cursor) {
        if let Some((start, end)) = cursor.get_selection() {
            let start_idx = buffer.line_to_char(start.line) + start.column.min(buffer.get_line_text(start.line).len());
            let end_idx = buffer.line_to_char(end.line) + end.column.min(buffer.get_line_text(end.line).len());
            
            if end_idx > start_idx {
                buffer.remove(start_idx..end_idx);
                cursor.position = start;
            }
        }
        cursor.clear_selection();
    }

    fn insert_tab(buffer: &mut crate::rope_buffer::RopeBuffer, cursor: &mut crate::cursor::Cursor) {
        let char_idx = cursor.to_char_index(buffer);
        buffer.insert_char(char_idx, '\t');
        cursor.move_right(buffer);
    }

    fn expand_tree_to_current_file(&mut self) {
        if let (Some(tree_view), Some(current_tab)) = (&mut self.tree_view, self.tab_manager.active_tab()) {
            if let Some(file_path) = &current_tab.path {
                let _ = tree_view.expand_to_file(file_path);
            }
        }
    }

    fn handle_sidebar_resize(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        if self.tree_view.is_none() {
            return false;
        }
        
        let tree_area_start_y = 1; // After tab bar
        let tree_area_end_y = self.terminal_size.1.saturating_sub(1); // Before status bar
        let border_column = self.sidebar_width; // The border is at this column
        
        // Check if we're currently resizing - if so, handle all mouse events
        if self.sidebar_resizing {
            match mouse.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    // Resize sidebar - the border should be at the mouse position
                    let new_width = mouse.column.max(15).min(self.terminal_size.0 / 2);
                    self.sidebar_width = new_width;
                    if let Some(tree_view) = &mut self.tree_view {
                        tree_view.resize(self.sidebar_width);
                    }
                    return true;
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.sidebar_resizing = false;
                    return true;
                }
                _ => return true, // Consume all events during resize
            }
        }
        
        // Check if mouse is on the resize border
        if mouse.column == border_column && 
           mouse.row >= tree_area_start_y && 
           mouse.row < tree_area_end_y {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    self.sidebar_resizing = true;
                    return true;
                }
                _ => {}
            }
        }
        
        false
    }

    fn handle_mouse_on_tree_view(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        if self.tree_view.is_none() {
            return false;
        }
        
        // Check if mouse is within tree view area (excluding the border)
        let tree_area_width = self.sidebar_width;
        let tree_area_start_y = 1; // After tab bar
        let tree_area_end_y = self.terminal_size.1.saturating_sub(1); // Before status bar
        
        // For scroll events, be more lenient with position detection since scroll position might not be accurate
        let is_in_tree_area = if matches!(mouse.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
            // For scroll events, prioritize tree view if mouse is anywhere in the tree area
            mouse.column <= tree_area_width && mouse.row >= tree_area_start_y && mouse.row < tree_area_end_y
        } else {
            // For other events, use exact positioning (excluding border)
            mouse.column < tree_area_width && mouse.row >= tree_area_start_y && mouse.row < tree_area_end_y
        };
        
        if !is_in_tree_area {
            return false; // Mouse not in tree view area
        }
        
        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                // ALWAYS handle scroll events in tree view area, regardless of tree_view state
                if let Some(tree_view) = &mut self.tree_view {
                    // Handle scrolling by adjusting scroll offset directly
                    let scroll_amount = 3;
                    let visible_height = (tree_area_end_y - tree_area_start_y) as usize;
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            tree_view.scroll_up(scroll_amount);
                        }
                        MouseEventKind::ScrollDown => {
                            tree_view.scroll_down(scroll_amount, visible_height);
                        }
                        _ => {}
                    }
                }
                // Always return true for scroll events in tree area to prevent editor from handling them
                return true;
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(tree_view) = &mut self.tree_view {
                    // Check if click is on scrollbar
                    if mouse.column == tree_area_width.saturating_sub(1) {
                        self.tree_scrollbar_dragging = true;
                        let visible_height = (tree_area_end_y - tree_area_start_y) as usize;
                        let click_y = (mouse.row - tree_area_start_y) as usize;
                        tree_view.handle_scrollbar_click(visible_height, click_y);
                        return true;
                    }
                    
                    // Calculate which item was clicked first
                    let item_y = mouse.row - tree_area_start_y;
                    let visible_items = tree_view.get_visible_items();
                    let clicked_index = tree_view.scroll_offset + item_y as usize;
                    
                    if clicked_index < visible_items.len() {
                        tree_view.selected_index = clicked_index;
                        
                        // Double-click detection for opening files/expanding directories
                        let now = std::time::Instant::now();
                        let is_double_click = if let Some(last_time) = self.last_click_time {
                            now.duration_since(last_time) < std::time::Duration::from_millis(500)
                        } else {
                            false
                        };
                        
                        if is_double_click {
                            if let Some(selected_item) = tree_view.get_selected_item() {
                                if selected_item.is_dir {
                                    let _ = tree_view.toggle_selected();
                                } else {
                                    // Open file
                                    let file_path = selected_item.path.clone();
                                    let _ = tree_view; // Release borrow
                                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                                        let new_tab = Tab::from_file(file_path, &content);
                                        self.tab_manager.add_tab(new_tab);
                                        self.expand_tree_to_current_file();
                                        self.handle_command(EditorCommand::FocusEditor);
                                    }
                                }
                            }
                            self.last_click_time = None; // Reset to prevent triple-click
                        } else {
                            self.last_click_time = Some(now);
                        }
                    }
                }
                
                // Focus tree view on click (after handling the click)
                self.handle_command(EditorCommand::FocusTreeView);
                return true;
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if let Some(tree_view) = &mut self.tree_view {
                    // Calculate which item was right-clicked
                    let item_y = mouse.row - tree_area_start_y;
                    let visible_items = tree_view.get_visible_items();
                    let clicked_index = tree_view.scroll_offset + item_y as usize;
                    
                    if clicked_index < visible_items.len() {
                        tree_view.selected_index = clicked_index;
                        
                        if let Some(selected_item) = tree_view.get_selected_item() {
                            // Open context menu at mouse position
                            self.menu_system.open_tree_context_menu(
                                selected_item.path.clone(),
                                selected_item.is_dir,
                                (mouse.column, mouse.row)
                            );
                        }
                    }
                }
                
                // Focus tree view on right-click
                self.handle_command(EditorCommand::FocusTreeView);
                return true;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.tree_scrollbar_dragging {
                    if let Some(tree_view) = &mut self.tree_view {
                        let visible_height = (tree_area_end_y - tree_area_start_y) as usize;
                        let click_y = (mouse.row - tree_area_start_y) as usize;
                        tree_view.handle_scrollbar_click(visible_height, click_y);
                    }
                    return true;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.tree_scrollbar_dragging {
                    self.tree_scrollbar_dragging = false;
                    return true;
                }
            }
            _ => {}
        }
        
        true // Mouse was in tree view area
    }

}
