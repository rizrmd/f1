mod status_bar;
mod tab_bar;
mod menu_component;
mod scrollbar;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::editor_widget::EditorWidget;
use crate::tab::TabManager;
use crate::menu::{MenuSystem, MenuState};

use self::status_bar::StatusBar;
use self::tab_bar::TabBar;
pub use self::menu_component::{MenuComponent, MenuItem, MenuAction};
pub use self::scrollbar::{VerticalScrollbar, ScrollbarState};

pub struct UI {
    tab_bar: TabBar,
    status_bar: StatusBar,
}

impl UI {
    pub fn new() -> Self {
        Self {
            tab_bar: TabBar::new(),
            status_bar: StatusBar::new(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, tab_manager: &mut TabManager, warning_message: &Option<String>, selected_button: usize, is_info: bool, menu_system: &MenuSystem) {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tab bar (reduced from 2 to 1)
                Constraint::Min(0),    // Editor content
                Constraint::Length(1), // Status bar
            ])
            .split(size);

        // Render tab bar
        self.tab_bar.draw(frame, chunks[0], tab_manager);

        // Render editor content or markdown preview
        if let Some(tab) = tab_manager.active_tab_mut() {
            if tab.preview_mode && tab.is_markdown() {
                // Render markdown preview
                let content = tab.buffer.to_string();
                let preview = crate::markdown_widget::MarkdownWidget::new(&content)
                    .viewport_offset(tab.viewport_offset);
                frame.render_widget(preview, chunks[1]);
            } else {
                // Render normal editor
                let editor = EditorWidget::new(&tab.buffer, &tab.cursor)
                    .viewport_offset(tab.viewport_offset)
                    .show_line_numbers(true)
                    .focused(true);
                frame.render_widget(editor, chunks[1]);
            }
        }

        // Render status bar
        self.status_bar.draw(frame, chunks[2], tab_manager);

        // Render warning dialog if present
        if let Some(message) = warning_message {
            self.draw_warning_dialog(frame, message, selected_button, is_info);
        }

        // Render menus if present
        match &menu_system.state {
            MenuState::MainMenu(menu) => {
                let menu_area = Rect {
                    x: 0,
                    y: size.height.saturating_sub(menu.height + 1),
                    width: menu.width,
                    height: menu.height,
                };
                menu.render(frame, menu_area);
            }
            MenuState::CurrentTabMenu(menu) => {
                let tab_index = tab_manager.active_index();
                let tab_x = self.tab_bar.get_tab_x_position(tab_manager, tab_index);
                let menu_area = Rect {
                    x: tab_x,
                    y: 1, // Directly below tab bar
                    width: menu.width,
                    height: menu.height,
                };
                menu.render(frame, menu_area);
            }
            MenuState::FilePicker(picker_state) => {
                self.draw_file_picker(frame, picker_state);
            }
            MenuState::Closed => {}
        }
    }

    fn draw_warning_dialog(&self, frame: &mut Frame, message: &str, selected_button: usize, is_info: bool) {
        let size = frame.area();
        
        // Calculate popup size and position  
        let popup_width = (message.len() + 4).max(30).min(80) as u16;
        let popup_height = 7; // Increased height for buttons
        let popup_x = (size.width.saturating_sub(popup_width)) / 2;
        let popup_y = (size.height.saturating_sub(popup_height)) / 2;
        
        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);
        
        // Create layout for dialog content
        let dialog_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title spacer
                Constraint::Length(1), // Message
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Buttons
            ])
            .split(popup_area);

        // Render the border and title
        let warning_block = Block::default()
            .borders(Borders::ALL)
            .title(" Warning ")
            .style(Style::default().bg(Color::Red).fg(Color::White));
        frame.render_widget(warning_block, popup_area);
        
        // Render the message
        let warning_text = Paragraph::new(Line::from(message))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        frame.render_widget(warning_text, dialog_chunks[1]);
        
        // Create buttons based on dialog type
        let buttons = if is_info {
            // Info dialog - only OK button
            let ok_style = Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD);
            
            Line::from(vec![
                Span::styled("       [ OK ]       ", ok_style),
            ])
        } else {
            // Confirmation dialog - Yes/No buttons
            let border_style = Style::default().fg(Color::White);
            let space_style = Style::default();
            
            let (no_style, no_left_border, no_right_border) = if selected_button == 0 {
                // Selected No: bright red background with white border
                (
                    Style::default()
                        .bg(Color::Rgb(200, 50, 50))  // Bright red
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                    Span::styled("[", border_style.bg(Color::Rgb(200, 50, 50))),
                    Span::styled("]", border_style.bg(Color::Rgb(200, 50, 50))),
                )
            } else {
                // Not selected: dark gray background
                (
                    Style::default()
                        .bg(Color::Rgb(60, 60, 60))   // Dark gray
                        .fg(Color::Rgb(200, 200, 200)),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                )
            };
            
            let (yes_style, yes_left_border, yes_right_border) = if selected_button == 1 {
                // Selected Yes: bright green background with white border
                (
                    Style::default()
                        .bg(Color::Rgb(50, 200, 50))  // Bright green
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                    Span::styled("[", border_style.bg(Color::Rgb(50, 200, 50))),
                    Span::styled("]", border_style.bg(Color::Rgb(50, 200, 50))),
                )
            } else {
                // Not selected: dark gray background
                (
                    Style::default()
                        .bg(Color::Rgb(60, 60, 60))   // Dark gray
                        .fg(Color::Rgb(200, 200, 200)),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                )
            };
            
            Line::from(vec![
                Span::styled("  ", space_style),      // Left padding
                no_left_border,                       // Left border or space
                Span::styled(" No ", no_style),       // No button with padding
                no_right_border,                      // Right border or space
                Span::styled("  ", space_style),      // Space between buttons
                yes_left_border,                      // Left border or space
                Span::styled(" Yes ", yes_style),     // Yes button with padding
                yes_right_border,                     // Right border or space
                Span::styled("  ", space_style),      // Right padding
            ])
        };
        
        let buttons_paragraph = Paragraph::new(buttons)
            .alignment(Alignment::Center);
        frame.render_widget(buttons_paragraph, dialog_chunks[3]);
    }


    fn draw_file_picker(&self, frame: &mut Frame, picker_state: &crate::menu::FilePickerState) {
        let size = frame.area();
        
        // Center the file picker modal
        let modal_width = 70u16.min(size.width.saturating_sub(4));
        let modal_height = 24u16.min(size.height.saturating_sub(4));
        let modal_x = (size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (size.height.saturating_sub(modal_height)) / 2;
        
        let modal_area = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        // Clear the area
        frame.render_widget(Clear, modal_area);

        // Create layout
        let modal_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Current directory
                Constraint::Length(1), // Search input
                Constraint::Length(1), // Separator
                Constraint::Min(0),    // File list
            ])
            .split(modal_area);

        // Render modal border with current directory in title
        let title = format!(" 📁 {} ", picker_state.current_dir.display());
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(Color::Black).fg(Color::White));
        frame.render_widget(modal_block, modal_area);

        // Current directory info
        let dir_info = format!("📂 {}", picker_state.current_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("/"));
        let dir_paragraph = Paragraph::new(Line::from(dir_info))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(dir_paragraph, modal_chunks[0]);

        // Search input
        let search_text = if picker_state.search_query.is_empty() {
            "🔍 Type to search...".to_string()
        } else {
            format!("🔍 {}", picker_state.search_query)
        };
        let search_input = Paragraph::new(Line::from(search_text))
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(search_input, modal_chunks[1]);

        // File list with two lines per item when searching
        let is_searching = !picker_state.search_query.is_empty();
        let items_per_entry = if is_searching { 2 } else { 1 };
        
        let total_items = picker_state.filtered_items.len();
        
        // Calculate scrollbar area
        let scrollbar_width = if total_items * items_per_entry > modal_chunks[3].height as usize {
            1
        } else {
            0
        };

        let file_list_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(scrollbar_width),
            ])
            .split(modal_chunks[3]);
        
        let file_content_area = file_list_chunks[0];
        let file_scrollbar_area = if scrollbar_width > 0 { Some(file_list_chunks[1]) } else { None };
        
        // Calculate visible items and start index
        let available_height = file_content_area.height as usize;
        let visible_items = available_height / items_per_entry;
        let start_index = if picker_state.selected_index >= visible_items {
            picker_state.selected_index.saturating_sub(visible_items - 1)
        } else {
            0
        };

        let mut file_lines = Vec::new();
        
        for (i, item) in picker_state.filtered_items
            .iter()
            .skip(start_index)
            .take(visible_items)
            .enumerate() 
        {
            let global_index = start_index + i;
            // Use hovered_index if available, otherwise use selected_index
            let is_selected = if let Some(hovered) = picker_state.hovered_index {
                global_index == hovered
            } else {
                global_index == picker_state.selected_index
            };
            
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            
            let dim_style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::Gray)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Icon based on type
            let icon = if item.name == ".." {
                "⬆️"
            } else if item.is_dir {
                "📁"
            } else {
                // File icon based on extension
                match item.path.extension().and_then(|e| e.to_str()) {
                    Some("rs") => "🦀",
                    Some("toml") => "⚙️",
                    Some("md") => "📝",
                    Some("txt") => "📄",
                    Some("json") => "📋",
                    Some("yml") | Some("yaml") => "📋",
                    Some("sh") => "🔧",
                    Some("js") | Some("ts") => "📜",
                    Some("py") => "🐍",
                    Some("go") => "🐹",
                    Some("cpp") | Some("c") | Some("h") => "⚡",
                    _ => "📄",
                }
            };
            
            // First line: icon and name (padded to content area width)
            let name_line = format!(" {} {}", icon, item.name);
            let content_width = (file_content_area.width as usize).saturating_sub(2);
            let padded_name_line = format!("{:<width$}", name_line, width = content_width);
            file_lines.push(Line::from(Span::styled(padded_name_line, style)));
            
            // Second line: relative path (only when searching, also padded)
            if is_searching {
                let path_to_show = if item.relative_path.is_empty() || item.relative_path == item.name {
                    ".".to_string()
                } else {
                    item.relative_path.clone()
                };
                let path_line = format!("    {}", path_to_show);
                let padded_path_line = format!("{:<width$}", path_line, width = content_width);
                file_lines.push(Line::from(Span::styled(padded_path_line, dim_style)));
            }
        }

        let file_list = Paragraph::new(file_lines);
        frame.render_widget(file_list, file_content_area);

        // Render scrollbar if needed
        if let Some(scrollbar_area) = file_scrollbar_area {
            let scrollbar_state = ScrollbarState::new(
                total_items,
                visible_items,
                start_index,
            );
            
            let scrollbar = VerticalScrollbar::new(scrollbar_state)
                .style(Style::default().fg(Color::Reset))
                .thumb_style(Style::default().fg(Color::White))
                .track_symbols(VerticalScrollbar::minimal());
            
            frame.render_widget(scrollbar, scrollbar_area);
        }
    }
}