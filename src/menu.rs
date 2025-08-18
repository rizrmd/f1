use std::path::PathBuf;
use crate::ui::{MenuComponent, MenuItem, MenuAction};

#[derive(Debug, Clone, PartialEq)]
pub enum MenuState {
    Closed,
    MainMenu(MenuComponent),
    CurrentTabMenu(MenuComponent),
    FilePicker(FilePickerState),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilePickerState {
    pub search_query: String,
    pub filtered_items: Vec<FileItem>,
    pub selected_index: usize,
    pub hovered_index: Option<usize>,
    pub current_dir: PathBuf,
    pub all_items: Vec<FileItem>,
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
        let mut state = Self {
            search_query: String::new(),
            filtered_items: Vec::new(),
            selected_index: 0,
            hovered_index: None,
            current_dir: current_dir.clone(),
            all_items: Vec::new(),
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
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                // Skip hidden files (starting with .)
                if name.starts_with('.') && name != ".." {
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
            
            // Search in subdirectories (recursive)
            let current_dir = self.current_dir.clone();
            self.search_recursive(&current_dir, &query, 0, 3); // Max depth 3
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
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }
                
                let relative = path.strip_prefix(&self.current_dir)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                
                if fuzzy_match(&name.to_lowercase(), query) || fuzzy_match(&relative.to_lowercase(), query) {
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
        self.current_dir = dir;
        self.search_query.clear();
        self.hovered_index = None; // Clear hover when changing directory
        self.load_current_directory();
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

    pub fn toggle_main_menu(&mut self, is_markdown: bool, in_preview_mode: bool) {
        self.state = match self.state {
            MenuState::Closed => {
                let preview_text = if is_markdown {
                    if in_preview_mode {
                        "Edit Mode"
                    } else {
                        "Preview Mode"
                    }
                } else {
                    "Toggle Preview"
                };
                
                let items = vec![
                    MenuItem::new("Current Tab", MenuAction::Custom("current_tab".to_string()))
                        .with_shortcut("Ctrl+G"),
                    MenuItem::new("Open File", MenuAction::Custom("open_file".to_string()))
                        .with_shortcut("Ctrl+P"),
                    MenuItem::new(preview_text, MenuAction::Custom("toggle_preview".to_string()))
                        .with_shortcut("Ctrl+U"),
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
    pub fn open_main_menu(&mut self, is_markdown: bool, in_preview_mode: bool) {
        let preview_text = if is_markdown {
            if in_preview_mode {
                "Edit Mode"
            } else {
                "Preview Mode"
            }
        } else {
            "Toggle Preview"
        };
        
        let items = vec![
            MenuItem::new("Current Tab", MenuAction::Custom("current_tab".to_string()))
                .with_shortcut("Ctrl+G"),
            MenuItem::new("Open File", MenuAction::Custom("open_file".to_string()))
                .with_shortcut("Ctrl+P"),
            MenuItem::new(preview_text, MenuAction::Custom("toggle_preview".to_string()))
                .with_shortcut("Ctrl+U"),
            MenuItem::new("Quit", MenuAction::Custom("quit".to_string()))
                .with_shortcut("Ctrl+Q"),
            MenuItem::new("Cancel", MenuAction::Close),
        ];
        let menu = MenuComponent::new(items)
            .with_width(30)
            .with_colors(ratatui::style::Color::Yellow, ratatui::style::Color::Black);
        self.state = MenuState::MainMenu(menu);
    }

    pub fn open_current_tab_menu(&mut self) {
        let items = vec![
            MenuItem::new("Close Tab", MenuAction::Custom("close_tab".to_string()))
                .with_shortcut("Ctrl+W"),
            MenuItem::new("Close Other Tab", MenuAction::Custom("close_other_tab".to_string()))
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
                file_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                })
            };
            picker_state.current_dir = dir;
            picker_state.load_current_directory();
        }
        
        self.state = MenuState::FilePicker(picker_state);
    }

    pub fn close(&mut self) {
        self.state = MenuState::Closed;
    }

    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        !matches!(self.state, MenuState::Closed)
    }

    pub fn handle_up(&mut self) {
        match &mut self.state {
            MenuState::MainMenu(menu) => menu.move_up(),
            MenuState::CurrentTabMenu(menu) => menu.move_up(),
            _ => {}
        }
    }

    pub fn handle_down(&mut self) {
        match &mut self.state {
            MenuState::MainMenu(menu) => menu.move_down(),
            MenuState::CurrentTabMenu(menu) => menu.move_down(),
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
            _ => None
        }
    }

}