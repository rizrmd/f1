use crate::gitignore::GitIgnore;
use crate::ui::{MenuAction, MenuComponent, MenuItem};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuState {
    Closed,
    MainMenu(MenuComponent),
    CurrentTabMenu(MenuComponent),
    FilePicker(FilePickerState),
    TreeContextMenu(TreeContextMenuState),
    InputDialog(InputDialogState),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputDialogState {
    pub prompt: String,
    pub input: String,
    pub operation: String, // "new_file", "new_folder", "rename"
    pub target_path: PathBuf,
    pub cursor_position: usize,
    pub selection_start: Option<usize>,
    pub hovered_button: Option<usize>, // 0 = OK, 1 = Cancel
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeContextMenuState {
    pub menu: MenuComponent,
    pub target_path: PathBuf,
    pub is_directory: bool,
    pub position: (u16, u16), // (x, y) position for the menu
}

#[derive(Debug, Clone)]
pub struct FilePickerState {
    pub search_query: String,
    pub filtered_items: Vec<FileItem>,
    pub selected_index: usize,
    pub hovered_index: Option<usize>,
    pub current_dir: PathBuf,
    pub all_items: Vec<FileItem>,
    gitignore: GitIgnore,
    last_scroll_time: Option<Instant>,
    scroll_acceleration: usize,
}

impl PartialEq for FilePickerState {
    fn eq(&self, other: &Self) -> bool {
        self.search_query == other.search_query
            && self.filtered_items == other.filtered_items
            && self.selected_index == other.selected_index
            && self.hovered_index == other.hovered_index
            && self.current_dir == other.current_dir
            && self.all_items == other.all_items
            && self.scroll_acceleration == other.scroll_acceleration
        // Note: Skipping last_scroll_time comparison as Instant doesn't impl PartialEq
        // and gitignore comparison as it's internal state
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub relative_path: String,
}

impl FilePickerState {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Create a temporary state to use the find_repo_root method
        let temp_state = Self {
            search_query: String::new(),
            filtered_items: Vec::new(),
            selected_index: 0,
            hovered_index: None,
            current_dir: current_dir.clone(),
            all_items: Vec::new(),
            gitignore: GitIgnore::new(current_dir.clone()), // Temporary
            last_scroll_time: None,
            scroll_acceleration: 1,
        };

        let repo_root = temp_state.find_repo_root(&current_dir);
        let gitignore = GitIgnore::new(repo_root);

        let mut state = Self {
            search_query: String::new(),
            filtered_items: Vec::new(),
            selected_index: 0,
            hovered_index: None,
            current_dir: current_dir.clone(),
            all_items: Vec::new(),
            gitignore,
            last_scroll_time: None,
            scroll_acceleration: 1,
        };
        state.load_current_directory();
        state
    }

    pub fn load_current_directory(&mut self) {
        self.all_items.clear();

        // Add parent directory entry if not at root
        if self.current_dir.parent().is_some() {
            self.all_items.push(FileItem {
                path: self.current_dir.parent().unwrap().to_path_buf(),
                name: "..".to_string(),
                is_dir: true,
                relative_path: "..".to_string(),
            });
        }

        // Load directory contents
        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip hidden files (starting with .)
                if name.starts_with('.') && name != ".." {
                    continue;
                }

                // Skip gitignored files
                if self.gitignore.is_ignored(&path) {
                    continue;
                }

                let is_dir = path.is_dir();
                let item = FileItem {
                    path: path.clone(),
                    name,
                    is_dir,
                    relative_path: String::new(), // Will be set during search
                };

                if is_dir {
                    dirs.push(item);
                } else {
                    files.push(item);
                }
            }

            // Sort directories and files alphabetically
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            // Add directories first, then files
            self.all_items.extend(dirs);
            self.all_items.extend(files);
        }

        self.filtered_items = self.all_items.clone();
        self.selected_index = 0;
    }

    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_items = self.all_items.clone();
        } else {
            // Fuzzy search in current directory and subdirectories
            let query = self.search_query.to_lowercase();
            self.filtered_items.clear();

            // Search in current directory
            for item in &self.all_items {
                if item.name != ".." && fuzzy_match(&item.name.to_lowercase(), &query) {
                    self.filtered_items.push(item.clone());
                }
            }

            // Search in subdirectories (recursive) - start from depth 1 to avoid duplicating current dir
            if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        
                        // Skip hidden directories
                        if !name.starts_with('.') {
                            self.search_recursive(&path, &query, 1, 3); // Start at depth 1
                        }
                    }
                }
            }
        }
        self.selected_index = 0;
        self.hovered_index = None; // Clear hover when filtering
    }

    fn search_recursive(&mut self, dir: &PathBuf, query: &str, depth: usize, max_depth: usize) {
        if depth >= max_depth {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }

                let relative = path
                    .strip_prefix(&self.current_dir)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();

                if fuzzy_match(&name.to_lowercase(), query)
                    || fuzzy_match(&relative.to_lowercase(), query)
                {
                    self.filtered_items.push(FileItem {
                        path: path.clone(),
                        name,
                        is_dir: path.is_dir(),
                        relative_path: relative,
                    });
                }

                // Recursively search directories
                if path.is_dir() {
                    self.search_recursive(&path, query, depth + 1, max_depth);
                }
            }
        }
    }

    pub fn enter_directory(&mut self, dir: PathBuf) {
        self.current_dir = dir.clone();
        self.search_query.clear();
        self.hovered_index = None; // Clear hover when changing directory

        // Update gitignore for the new directory (find repo root)
        self.gitignore = GitIgnore::new(self.find_repo_root(&dir));

        self.load_current_directory();
    }

    fn find_repo_root(&self, path: &Path) -> PathBuf {
        let mut current = path.to_path_buf();
        loop {
            if current.join(".git").exists() {
                return current;
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                // If no .git found, use the current directory
                return path.to_path_buf();
            }
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.enter_directory(parent.to_path_buf());
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.hovered_index = None; // Clear hover when using keyboard
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.filtered_items.len().saturating_sub(1) {
            self.selected_index += 1;
            self.hovered_index = None; // Clear hover when using keyboard
        }
    }

    pub fn get_selected_item(&self) -> Option<&FileItem> {
        self.filtered_items.get(self.selected_index)
    }
    
    pub fn scroll_up(&mut self, base_amount: usize) {
        // Update scroll acceleration
        self.update_scroll_acceleration();
        
        // Calculate actual scroll amount with acceleration
        let scroll_amount = base_amount.saturating_mul(self.scroll_acceleration);
        self.selected_index = self.selected_index.saturating_sub(scroll_amount);
        self.hovered_index = None; // Clear hover when scrolling
    }
    
    pub fn scroll_down(&mut self, base_amount: usize) {
        // Update scroll acceleration
        self.update_scroll_acceleration();
        
        // Calculate actual scroll amount with acceleration
        let scroll_amount = base_amount.saturating_mul(self.scroll_acceleration);
        let max_index = self.filtered_items.len().saturating_sub(1);
        self.selected_index = (self.selected_index + scroll_amount).min(max_index);
        self.hovered_index = None; // Clear hover when scrolling
    }
    
    fn update_scroll_acceleration(&mut self) {
        let now = Instant::now();
        
        if let Some(last_time) = self.last_scroll_time {
            // If scrolling within 150ms, increase acceleration
            if now.duration_since(last_time).as_millis() < 150 {
                // More aggressive acceleration for file picker
                let increment = if self.scroll_acceleration < 3 {
                    1  // Start with +1 for initial acceleration
                } else if self.scroll_acceleration < 8 {
                    2  // Medium acceleration +2
                } else if self.scroll_acceleration < 15 {
                    3  // Fast acceleration +3
                } else {
                    4  // Very fast +4
                };
                
                self.scroll_acceleration = (self.scroll_acceleration + increment).min(20);
            } else {
                // Reset acceleration if too much time has passed
                self.scroll_acceleration = 1;
            }
        } else {
            // First scroll, start with base acceleration
            self.scroll_acceleration = 1;
        }
        
        self.last_scroll_time = Some(now);
    }
}

fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let mut pattern_chars = pattern.chars();
    let mut current_char = pattern_chars.next();

    for text_char in text.chars() {
        if let Some(pc) = current_char {
            if text_char == pc {
                current_char = pattern_chars.next();
            }
        } else {
            return true; // All pattern chars matched
        }
    }

    current_char.is_none() // True if all pattern chars were matched
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuSystem {
    pub state: MenuState,
}

impl MenuSystem {
    pub fn new() -> Self {
        Self {
            state: MenuState::Closed,
        }
    }

    pub fn toggle_main_menu(
        &mut self,
        _is_markdown: bool,
        _in_preview_mode: bool,
        word_wrap_enabled: bool,
        tree_view_enabled: bool,
        find_inline_enabled: bool,
    ) {
        self.state = match self.state {
            MenuState::Closed => {
                let items = vec![
                    MenuItem::new("Current Tab", MenuAction::Custom("current_tab".to_string()))
                        .with_shortcut("Ctrl+G"),
                    MenuItem::new("Open File", MenuAction::Custom("open_file".to_string()))
                        .with_shortcut("Ctrl+P"),
                    MenuItem::new("Tree View", MenuAction::Custom("toggle_tree_view".to_string()))
                        .with_checkbox(tree_view_enabled)
                        .with_shortcut("Ctrl+T"),
                    MenuItem::new("Find Inline", MenuAction::Custom("toggle_find_inline".to_string()))
                        .with_checkbox(find_inline_enabled)
                        .with_shortcut("Ctrl+F"),
                    MenuItem::new(
                        "Word Wrap",
                        MenuAction::Custom("toggle_word_wrap".to_string()),
                    )
                    .with_checkbox(word_wrap_enabled)
                    .with_shortcut("Alt+W"),
                    MenuItem::new("Quit", MenuAction::Custom("quit".to_string()))
                        .with_shortcut("Ctrl+Q"),
                    MenuItem::new("Cancel", MenuAction::Close),
                ];

                let menu = MenuComponent::new(items)
                    .with_width(30)
                    .with_colors(ratatui::style::Color::Yellow, ratatui::style::Color::Black);
                MenuState::MainMenu(menu)
            }
            _ => MenuState::Closed,
        };
    }

    #[allow(dead_code)]
    pub fn open_main_menu(
        &mut self,
        _is_markdown: bool,
        _in_preview_mode: bool,
        word_wrap_enabled: bool,
        tree_view_enabled: bool,
        find_inline_enabled: bool,
    ) {
        let items = vec![
            MenuItem::new("Current Tab", MenuAction::Custom("current_tab".to_string()))
                .with_shortcut("Ctrl+G"),
            MenuItem::new("Open File", MenuAction::Custom("open_file".to_string()))
                .with_shortcut("Ctrl+P"),
            MenuItem::new("Tree View", MenuAction::Custom("toggle_tree_view".to_string()))
                .with_checkbox(tree_view_enabled)
                .with_shortcut("Ctrl+T"),
            MenuItem::new("Find Inline", MenuAction::Custom("toggle_find_inline".to_string()))
                .with_checkbox(find_inline_enabled)
                .with_shortcut("Ctrl+F"),
            MenuItem::new(
                "Word Wrap",
                MenuAction::Custom("toggle_word_wrap".to_string()),
            )
            .with_checkbox(word_wrap_enabled)
            .with_shortcut("Alt+W"),
            MenuItem::new("Quit", MenuAction::Custom("quit".to_string())).with_shortcut("Ctrl+Q"),
            MenuItem::new("Cancel", MenuAction::Close),
        ];

        let menu = MenuComponent::new(items)
            .with_width(30)
            .with_colors(ratatui::style::Color::Yellow, ratatui::style::Color::Black);
        self.state = MenuState::MainMenu(menu);
    }

    pub fn open_current_tab_menu(&mut self) {
        let items = vec![
            MenuItem::new("Next Tab", MenuAction::Custom("next_tab".to_string()))
                .with_shortcut("Ctrl+]"),
            MenuItem::new("Previous Tab", MenuAction::Custom("prev_tab".to_string()))
                .with_shortcut("Ctrl+["),
            MenuItem::new("Close Tab", MenuAction::Custom("close_tab".to_string()))
                .with_shortcut("Ctrl+W"),
            MenuItem::new(
                "Close Other Tab",
                MenuAction::Custom("close_other_tab".to_string()),
            )
            .with_shortcut("Ctrl+Shift+W"),
            MenuItem::new("Cancel", MenuAction::Close),
        ];
        let menu = MenuComponent::new(items)
            .with_width(30)
            .with_colors(ratatui::style::Color::Cyan, ratatui::style::Color::Black);
        self.state = MenuState::CurrentTabMenu(menu);
    }

    #[allow(dead_code)]
    pub fn open_file_picker(&mut self) {
        let picker_state = FilePickerState::new();
        self.state = MenuState::FilePicker(picker_state);
    }

    pub fn open_file_picker_at_path(&mut self, path: Option<PathBuf>) {
        let mut picker_state = FilePickerState::new();

        // If a path is provided, navigate to its directory
        if let Some(file_path) = path {
            let dir = if file_path.is_dir() {
                file_path
            } else {
                file_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| {
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    })
            };
            picker_state.enter_directory(dir);
        }

        self.state = MenuState::FilePicker(picker_state);
    }

    pub fn close(&mut self) {
        self.state = MenuState::Closed;
    }

    pub fn open_tree_context_menu(
        &mut self,
        path: PathBuf,
        is_directory: bool,
        position: (u16, u16),
        has_clipboard: bool,
    ) {
        let mut items = Vec::new();

        if is_directory {
            items.push(MenuItem::new(
                "New File",
                MenuAction::Custom("new_file".to_string()),
            ));
            items.push(MenuItem::new(
                "New Folder",
                MenuAction::Custom("new_folder".to_string()),
            ));
        }

        if !is_directory {
            items.push(MenuItem::new(
                "Open",
                MenuAction::Custom("open".to_string()),
            ));
        }

        // File management operations
        items.push(MenuItem::new(
            "Copy",
            MenuAction::Custom("copy".to_string()),
        ));
        items.push(MenuItem::new("Cut", MenuAction::Custom("cut".to_string())));
        
        // Only show Paste if there's something in clipboard
        if has_clipboard {
            items.push(MenuItem::new(
                "Paste",
                MenuAction::Custom("paste".to_string()),
            ));
        }
        
        items.push(MenuItem::new(
            "Rename",
            MenuAction::Custom("rename".to_string()),
        ));
        items.push(MenuItem::new(
            "Delete",
            MenuAction::Custom("delete".to_string()),
        ));

        let menu = MenuComponent::new(items);

        let context_state = TreeContextMenuState {
            menu,
            target_path: path,
            is_directory,
            position,
        };

        self.state = MenuState::TreeContextMenu(context_state);
    }
    
    pub fn open_tree_empty_area_menu(
        &mut self,
        path: PathBuf,
        position: (u16, u16),
        has_clipboard: bool,
    ) {
        let mut items = Vec::new();

        // Only show New File and New Folder for empty area
        items.push(MenuItem::new(
            "New File",
            MenuAction::Custom("new_file".to_string()),
        ));
        items.push(MenuItem::new(
            "New Folder",
            MenuAction::Custom("new_folder".to_string()),
        ));
        
        // Only show Paste if there's something in clipboard
        if has_clipboard {
            items.push(MenuItem::new(
                "Paste",
                MenuAction::Custom("paste".to_string()),
            ));
        }

        let menu = MenuComponent::new(items);

        let context_state = TreeContextMenuState {
            menu,
            target_path: path,
            is_directory: true, // Empty area is treated as directory for operations
            position,
        };

        self.state = MenuState::TreeContextMenu(context_state);
    }

    pub fn open_input_dialog(&mut self, prompt: String, operation: String, target_path: PathBuf) {
        let input_state = InputDialogState {
            prompt,
            input: String::new(),
            operation,
            target_path,
            cursor_position: 0,
            selection_start: None,
            hovered_button: None,
        };

        self.state = MenuState::InputDialog(input_state);
    }

    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        !matches!(self.state, MenuState::Closed)
    }

    pub fn handle_up(&mut self) {
        match &mut self.state {
            MenuState::MainMenu(menu) => menu.move_up(),
            MenuState::CurrentTabMenu(menu) => menu.move_up(),
            MenuState::TreeContextMenu(context_state) => context_state.menu.move_up(),
            _ => {}
        }
    }

    pub fn handle_down(&mut self) {
        match &mut self.state {
            MenuState::MainMenu(menu) => menu.move_down(),
            MenuState::CurrentTabMenu(menu) => menu.move_down(),
            MenuState::TreeContextMenu(context_state) => context_state.menu.move_down(),
            _ => {}
        }
    }

    pub fn handle_enter(&mut self) -> Option<String> {
        match &self.state {
            MenuState::MainMenu(menu) => {
                if let Some(action) = menu.get_selected_action() {
                    match action {
                        MenuAction::Close => {
                            self.close();
                            None
                        }
                        MenuAction::Custom(action_name) => {
                            let result = action_name.clone();
                            if action_name != "current_tab" {
                                self.close();
                            }
                            Some(result)
                        }
                    }
                } else {
                    None
                }
            }
            MenuState::CurrentTabMenu(menu) => {
                if let Some(action) = menu.get_selected_action() {
                    match action {
                        MenuAction::Close => {
                            self.close();
                            None
                        }
                        MenuAction::Custom(action_name) => {
                            let result = action_name.clone();
                            self.close();
                            Some(result)
                        }
                    }
                } else {
                    None
                }
            }
            MenuState::TreeContextMenu(context_state) => {
                if let Some(action) = context_state.menu.get_selected_action() {
                    match action {
                        MenuAction::Close => {
                            self.close();
                            None
                        }
                        MenuAction::Custom(action_name) => {
                            let result = action_name.clone();
                            self.close();
                            Some(result)
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
