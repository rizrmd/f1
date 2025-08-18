mod status_bar;
mod tab_bar;
mod menu_component;

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

    pub fn draw(&mut self, frame: &mut Frame, tab_manager: &mut TabManager, warning_message: &Option<String>, selected_button: usize, menu_system: &MenuSystem) {
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

        // Render editor content
        if let Some(tab) = tab_manager.active_tab_mut() {
            // Don't update viewport here - it should only be updated when cursor moves
            // tab.update_viewport(chunks[1].height as usize);

            let editor = EditorWidget::new(&tab.buffer, &tab.cursor)
                .viewport_offset(tab.viewport_offset)
                .show_line_numbers(true)
                .focused(true);

            frame.render_widget(editor, chunks[1]);
        }

        // Render status bar
        self.status_bar.draw(frame, chunks[2], tab_manager);

        // Render warning dialog if present
        if let Some(message) = warning_message {
            self.draw_warning_dialog(frame, message, selected_button);
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

    fn draw_warning_dialog(&self, frame: &mut Frame, message: &str, selected_button: usize) {
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
        
        // Create buttons with background colors and borders
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
        
        let buttons = Line::from(vec![
            Span::styled("  ", space_style),      // Left padding
            no_left_border,                       // Left border or space
            Span::styled(" No ", no_style),       // No button with padding
            no_right_border,                      // Right border or space
            Span::styled("  ", space_style),      // Space between buttons
            yes_left_border,                      // Left border or space
            Span::styled(" Yes ", yes_style),     // Yes button with padding
            yes_right_border,                     // Right border or space
            Span::styled("  ", space_style),      // Right padding
        ]);
        
        let buttons_paragraph = Paragraph::new(buttons)
            .alignment(Alignment::Center);
        frame.render_widget(buttons_paragraph, dialog_chunks[3]);
    }


    fn draw_file_picker(&self, frame: &mut Frame, picker_state: &crate::menu::FilePickerState) {
        let size = frame.area();
        
        // Center the file picker modal
        let modal_width = 60u16.min(size.width.saturating_sub(4));
        let modal_height = 20u16.min(size.height.saturating_sub(4));
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
                Constraint::Length(1), // Search input
                Constraint::Length(1), // Separator
                Constraint::Min(0),    // File list
            ])
            .split(modal_area);

        // Render modal border
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .title(" Open File ")
            .style(Style::default().bg(Color::Black).fg(Color::White));
        frame.render_widget(modal_block, modal_area);

        // Search input
        let search_text = format!("Search: {}", picker_state.search_query);
        let search_input = Paragraph::new(Line::from(search_text))
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(search_input, modal_chunks[0]);

        // File list
        let visible_files = modal_chunks[2].height as usize;
        let start_index = if picker_state.selected_index >= visible_files {
            picker_state.selected_index.saturating_sub(visible_files - 1)
        } else {
            0
        };

        let mut file_lines = Vec::new();
        for (i, file_path) in picker_state.filtered_files
            .iter()
            .skip(start_index)
            .take(visible_files)
            .enumerate() 
        {
            let global_index = start_index + i;
            let style = if global_index == picker_state.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            let file_name = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<invalid>");
            
            file_lines.push(Line::from(Span::styled(format!(" {}", file_name), style)));
        }

        let file_list = Paragraph::new(file_lines);
        frame.render_widget(file_list, modal_chunks[2]);
    }
}