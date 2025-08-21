// Removed unused imports KeyEvent, MouseEvent, and Frame
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub fn is_word_separator(ch: char) -> bool {
    matches!(
        ch,
        '.' | '-'
            | '_'
            | '/'
            | '\\'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '"'
            | '\''
            | ','
            | ';'
            | ':'
    )
}

use crate::keyboard::EditorCommand;
use crate::menu::MenuSystem;
use crate::tab::{Tab, TabManager};
use crate::tree_view::TreeView;
use crate::ui::UI;

pub struct App {
    pub tab_manager: TabManager,
    pub running: bool,
    pub ui: UI,
    pub warning_message: Option<String>,
    pub pending_close: bool,
    pub pending_quit: bool,
    pub warning_selected_button: usize, // 0 = No, 1 = Yes
    pub warning_is_info: bool,          // true = OK button only, false = Yes/No buttons
    pub mouse_selecting: bool,
    pub last_click_time: Option<Instant>,
    pub last_click_pos: Option<(u16, u16)>,
    pub terminal_size: (u16, u16), // (width, height)
    pub menu_system: MenuSystem,
    pub scrollbar_dragging: bool,
    pub file_picker_scrollbar_dragging: bool,
    pub tree_view: Option<TreeView>,
    pub sidebar_width: u16,
    pub sidebar_resizing: bool,
    pub focus_mode: FocusMode,
    pub tree_scrollbar_dragging: bool,
    pub status_message: Option<String>,
    status_message_expires: Option<Instant>,
    pub pending_delete_path: Option<PathBuf>,
    pub global_word_wrap: bool,
    pub last_scroll_time: Option<Instant>,
    pub scroll_acceleration: usize,
    pub dragging_tab: Option<usize>,   // Index of tab being dragged
    pub drag_start_x: u16,             // Starting X position of drag
    pub tab_was_active_on_click: bool, // Whether the tab was already active when clicked
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

        let mut app = Self {
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
            global_word_wrap: false,
            last_scroll_time: None,
            scroll_acceleration: 1,
            dragging_tab: None,
            drag_start_x: 0,
            tab_was_active_on_click: false,
        };

        // Apply global word wrap to initial tab
        if let Some(tab) = app.tab_manager.active_tab_mut() {
            if let Tab::Editor { word_wrap, .. } = tab {
                *word_wrap = app.global_word_wrap;
            }
        }

        app
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
                    if let Tab::Editor { viewport_offset, .. } = tab {
                        // Move by most of the visible area for faster navigation
                        let page_size = self.terminal_size.1.saturating_sub(4) as usize;
                        viewport_offset.0 = viewport_offset.0.saturating_sub(page_size);
                    }
                }
            }
            EditorCommand::PageDown => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    if let Tab::Editor { viewport_offset, .. } = tab {
                        // Move by most of the visible area for faster navigation
                        let page_size = self.terminal_size.1.saturating_sub(4) as usize;
                        viewport_offset.0 += page_size;
                    }
                }
            }
            EditorCommand::Modified => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.mark_modified();
                    // Ensure cursor is visible after modifications (like paste)
                    tab.ensure_cursor_visible(self.terminal_size.1.saturating_sub(2) as usize);
                }
            }
            EditorCommand::ToggleMenu => {
                let (is_markdown, in_preview_mode) =
                    if let Some(tab) = self.tab_manager.active_tab() {
                        match tab {
                            Tab::Editor { preview_mode, .. } => (tab.is_markdown(), *preview_mode),
                            Tab::Terminal { .. } => (false, false),
                        }
                    } else {
                        (false, false)
                    };
                let word_wrap_enabled = self.global_word_wrap;
                let tree_view_enabled = self.tree_view.is_some();
                let find_inline_enabled = self
                    .tab_manager
                    .active_tab()
                    .and_then(|t| match t {
                        Tab::Editor { find_replace_state, .. } => Some(find_replace_state.active),
                        Tab::Terminal { .. } => Some(false),
                    })
                    .unwrap_or(false);
                self.menu_system.toggle_main_menu(
                    is_markdown,
                    in_preview_mode,
                    word_wrap_enabled,
                    tree_view_enabled,
                    find_inline_enabled,
                );
            }
            EditorCommand::OpenFile => {
                // Get the current tab's file path to open picker in that directory
                let current_path = self
                    .tab_manager
                    .active_tab()
                    .and_then(|tab| tab.path())
                    .cloned();
                self.menu_system.open_file_picker_at_path(current_path);
            }
            EditorCommand::CurrentTab => {
                self.menu_system.open_current_tab_menu();
            }
            EditorCommand::Undo => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    if tab.undo() {
                        // Ensure cursor is visible with actual terminal height
                        tab.ensure_cursor_visible(self.terminal_size.1.saturating_sub(2) as usize);
                    }
                }
            }
            EditorCommand::Redo => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    if tab.redo() {
                        // Ensure cursor is visible with actual terminal height
                        tab.ensure_cursor_visible(self.terminal_size.1.saturating_sub(2) as usize);
                    }
                }
            }
            EditorCommand::TogglePreview => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.toggle_preview_mode();
                }
            }
            EditorCommand::ToggleWordWrap => {
                // Toggle global word wrap setting
                self.global_word_wrap = !self.global_word_wrap;

                // Apply to all tabs
                for tab in &mut self.tab_manager.tabs {
                    if let Tab::Editor { word_wrap, .. } = tab {
                        *word_wrap = self.global_word_wrap;
                    }
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
            EditorCommand::Find => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.start_find();
                }
            }
            EditorCommand::FindReplace => {
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    tab.start_find_replace();
                }
            }
            EditorCommand::NewTerminal => {
                let new_tab = Tab::new_terminal();
                self.tab_manager.add_tab(new_tab);
                self.expand_tree_to_current_file();
                // Focus the editor after creating new terminal tab
                self.focus_mode = FocusMode::Editor;
                if let Some(tree_view) = &mut self.tree_view {
                    tree_view.is_focused = false;
                }
            }
        }
    }


    pub fn handle_close_tab(&mut self) {
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

    pub fn handle_quit(&mut self) {
        // Check for unsaved changes before quitting
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
            return;
        }

        // No unsaved changes, quit directly
        self.running = false;
    }

    pub fn expand_tree_to_current_file(&mut self) {
        if let Some(tree_view) = &mut self.tree_view {
            if let Some(tab) = self.tab_manager.active_tab() {
                if let Some(path) = tab.path() {
                    tree_view.expand_to_file(path);
                }
            }
        }
    }

    pub fn create_new_terminal_tab(&mut self) {
        let terminal_tab = Tab::new_terminal();
        self.tab_manager.add_tab(terminal_tab);
        self.expand_tree_to_current_file();
        // Focus the editor after creating new terminal tab
        self.focus_mode = FocusMode::Editor;
        if let Some(tree_view) = &mut self.tree_view {
            tree_view.is_focused = false;
        }
    }

    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.ui.draw(
            frame,
            &mut self.tab_manager,
            &self.warning_message,
            self.warning_selected_button,
            self.warning_is_info,
            &self.menu_system,
            &self.tree_view,
            self.sidebar_width,
            &self.focus_mode,
            &self.status_message,
            self.dragging_tab,
        );
    }
}

