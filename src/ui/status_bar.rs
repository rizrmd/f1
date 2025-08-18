use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tab::TabManager;

pub struct StatusBar {}

impl StatusBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, tab_manager: &TabManager) {
        if let Some(tab) = tab_manager.active_tab() {
            let cursor_pos = format!(
                " L{}:C{} ",
                tab.cursor.position.line + 1,
                tab.cursor.position.column
            );

            let file_info = if let Some(path) = &tab.path {
                format!(" {} ", path.display())
            } else {
                format!(" {} ", tab.name)
            };

            let modified = if tab.modified { " [Modified] " } else { "" };

            let status_text = format!("{}{}", file_info, modified);

            let f1_menu = " ☰ F1 ";
            
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(6), // Exactly 6 characters for F1 button
                    Constraint::Min(0),
                    Constraint::Length(cursor_pos.len() as u16),
                ])
                .split(area);

            let f1_status = Paragraph::new(Line::from(vec![Span::raw(f1_menu)]))
                .style(
                    Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black),
                );

            let middle_status = Paragraph::new(Line::from(vec![Span::raw(status_text)]))
                .style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                );

            let right_status = Paragraph::new(Line::from(vec![Span::raw(cursor_pos)]))
                .style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White),
                );

            frame.render_widget(f1_status, chunks[0]);
            frame.render_widget(middle_status, chunks[1]);
            frame.render_widget(right_status, chunks[2]);
        }
    }
}