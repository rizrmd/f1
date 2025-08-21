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

    pub fn draw(
        &self,
        frame: &mut Frame,
        area: Rect,
        tab_manager: &TabManager,
        status_message: Option<&String>,
    ) {
        if let Some(tab) = tab_manager.active_tab() {
            match tab {
                crate::tab::Tab::Editor { cursor, path, name, modified, preview_mode, .. } => {
                    let cursor_pos = format!(
                        " L{}:C{} ",
                        cursor.position.line + 1,
                        cursor.position.column
                    );

                    let status_text = if let Some(message) = status_message {
                        // Show temporary status message with warning styling
                        format!(" {} ", message)
                    } else {
                        // Show normal file info
                        let file_info = if let Some(path) = path {
                            format!(" {} ", path.display())
                        } else {
                            format!(" {} ", name)
                        };

                        let modified_text = if *modified { " [Modified] " } else { "" };
                        format!("{}{}", file_info, modified_text)
                    };

                    let f1_menu = " ☰ F1 ";

                    // Add preview/edit toggle indicator for markdown files (shows current state)
                    let preview_indicator = if tab.is_markdown() {
                        if *preview_mode {
                            " PREVIEW (Ctrl+U) "
                        } else {
                            " EDIT (Ctrl+U) "
                        }
                    } else {
                        ""
                    };

                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(6), // Exactly 6 characters for F1 button
                            Constraint::Length(preview_indicator.len() as u16), // Preview indicator
                            Constraint::Min(0),
                            Constraint::Length(cursor_pos.len() as u16),
                        ])
                        .split(area);

                    let f1_status = Paragraph::new(Line::from(vec![Span::raw(f1_menu)]))
                        .style(Style::default().bg(Color::Yellow).fg(Color::Black));

                    let middle_status = if status_message.is_some() {
                        // Use warning text color but same background for status messages
                        Paragraph::new(Line::from(vec![Span::raw(status_text)])).style(
                            Style::default()
                                .bg(Color::Rgb(40, 40, 40))
                                .fg(Color::Yellow),
                        )
                    } else {
                        // Use normal colors for file info
                        Paragraph::new(Line::from(vec![Span::raw(status_text)]))
                            .style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White))
                    };

                    let right_status = Paragraph::new(Line::from(vec![Span::raw(cursor_pos)]))
                        .style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White));

                    let preview_status = if !preview_indicator.is_empty() {
                        Some(
                            Paragraph::new(Line::from(vec![Span::raw(preview_indicator)])).style(
                                Style::default()
                                    .bg(Color::Rgb(100, 50, 200)) // Purple background for preview
                                    .fg(Color::White),
                            ),
                        )
                    } else {
                        None
                    };

                    frame.render_widget(f1_status, chunks[0]);
                    if let Some(preview_widget) = preview_status {
                        frame.render_widget(preview_widget, chunks[1]);
                    }
                    frame.render_widget(middle_status, chunks[2]);
                    frame.render_widget(right_status, chunks[3]);
                }
                crate::tab::Tab::Terminal { name, modified, .. } => {
                    let status_text = if let Some(message) = status_message {
                        format!(" {} ", message)
                    } else {
                        let modified_text = if *modified { " [Modified] " } else { "" };
                        format!(" {}{}", name, modified_text)
                    };

                    let f1_menu = " ☰ F1 ";
                    let terminal_indicator = " TERMINAL ";

                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(6), // F1 button
                            Constraint::Length(terminal_indicator.len() as u16), // Terminal indicator
                            Constraint::Min(0), // Status text
                        ])
                        .split(area);

                    let f1_status = Paragraph::new(Line::from(vec![Span::raw(f1_menu)]))
                        .style(Style::default().bg(Color::Yellow).fg(Color::Black));

                    let terminal_status = Paragraph::new(Line::from(vec![Span::raw(terminal_indicator)]))
                        .style(Style::default().bg(Color::Green).fg(Color::Black));

                    let middle_status = if status_message.is_some() {
                        Paragraph::new(Line::from(vec![Span::raw(status_text)])).style(
                            Style::default()
                                .bg(Color::Rgb(40, 40, 40))
                                .fg(Color::Yellow),
                        )
                    } else {
                        Paragraph::new(Line::from(vec![Span::raw(status_text)]))
                            .style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White))
                    };

                    frame.render_widget(f1_status, chunks[0]);
                    frame.render_widget(terminal_status, chunks[1]);
                    frame.render_widget(middle_status, chunks[2]);
                }
            }
        }
    }
}
