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
    pub filtered_files: Vec<PathBuf>,
    pub selected_index: usize,
    pub all_files: Vec<PathBuf>,
}

impl FilePickerState {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            filtered_files: Vec::new(),
            selected_index: 0,
            all_files: Vec::new(),
        }
    }

    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_files = self.all_files.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_files = self.all_files
                .iter()
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_lowercase().contains(&query))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
        }
        self.selected_index = 0;
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.filtered_files.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn get_selected_file(&self) -> Option<&PathBuf> {
        self.filtered_files.get(self.selected_index)
    }
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

    pub fn toggle_main_menu(&mut self) {
        self.state = match self.state {
            MenuState::Closed => {
                let items = vec![
                    MenuItem::new("Current Tab", MenuAction::Custom("current_tab".to_string()))
                        .with_shortcut("Ctrl+G"),
                    MenuItem::new("Open File", MenuAction::Custom("open_file".to_string()))
                        .with_shortcut("Ctrl+P"),
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
    pub fn open_main_menu(&mut self) {
        let items = vec![
            MenuItem::new("Current Tab", MenuAction::Custom("current_tab".to_string()))
                .with_shortcut("Ctrl+G"),
            MenuItem::new("Open File", MenuAction::Custom("open_file".to_string()))
                .with_shortcut("Ctrl+P"),
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

    pub fn open_file_picker(&mut self) {
        let mut picker_state = FilePickerState::new();
        // Initialize with files from current directory
        if let Ok(entries) = std::fs::read_dir(".") {
            picker_state.all_files = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect();
            picker_state.update_filter();
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