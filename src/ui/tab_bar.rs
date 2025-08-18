use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tab::TabManager;

pub struct TabBar {}

impl TabBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_tab_x_position(&self, tab_manager: &TabManager, target_tab_index: usize) -> u16 {
        let mut x_pos = 0u16;
        
        for (i, tab) in tab_manager.tabs().iter().enumerate() {
            if i == target_tab_index {
                return x_pos;
            }
            // Calculate tab width: " " + tab_name + " " = tab_name.len() + 2
            let tab_width = tab.display_name().len() + 2;
            x_pos += tab_width as u16;
        }
        
        x_pos
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, tab_manager: &TabManager) {
        // Render tabs and hint together in a single line
        let mut spans = Vec::new();
        
        // Add all tabs as spans
        for (i, tab) in tab_manager.tabs().iter().enumerate() {
            let tab_name = format!(" {} ", tab.display_name());
            
            let style = if i == tab_manager.active_index() {
                // Active tab: black text on cyan background
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Inactive tab: dimmed text
                Style::default().fg(Color::Rgb(180, 180, 180))
            };
            
            spans.push(Span::styled(tab_name, style));
        }
        
        // Add the Ctrl+N hint directly after the tabs with some spacing
        spans.push(Span::styled("  ", Style::default())); // Add spacing
        spans.push(Span::styled("Ctrl+N", Style::default().fg(Color::Rgb(120, 120, 120))));
        
        // Create a single line with all spans
        let line = Line::from(spans);
        let paragraph = Paragraph::new(vec![line]);
        
        frame.render_widget(paragraph, area);
    }
}