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

    pub fn get_tab_x_position(&self, tab_manager: &TabManager, target_tab_index: usize, available_width: usize) -> u16 {
        let hint_text = "  Ctrl+N";
        let hint_width = hint_text.len();
        let tabs_width = available_width.saturating_sub(hint_width);
        
        // Calculate the same way as in draw() to get consistent positioning
        let tab_widths = self.calculate_tab_widths(tab_manager, tabs_width);
        
        let mut x_pos = 0u16;
        for (i, &width) in tab_widths.iter().enumerate() {
            if i == target_tab_index {
                return x_pos;
            }
            x_pos += width as u16;
        }
        
        x_pos
    }
    
    fn calculate_tab_widths(&self, tab_manager: &TabManager, available_width: usize) -> Vec<usize> {
        let mut widths = Vec::new();
        let tabs = tab_manager.tabs();
        let tab_count = tabs.len();
        
        if tab_count == 0 {
            return widths;
        }
        
        // Minimum width per tab (including padding)
        let min_tab_width = 8;
        let max_tabs_that_fit = available_width / min_tab_width;
        
        if tab_count <= max_tabs_that_fit {
            // All tabs can fit, calculate actual widths
            let avg_width = available_width / tab_count;
            let max_name_width = avg_width.saturating_sub(2);
            
            for tab in tabs.iter() {
                let full_name = tab.display_name();
                let truncated_name = self.truncate_name(&full_name, max_name_width);
                let tab_width = truncated_name.len() + 2; // Add padding spaces
                widths.push(tab_width);
            }
        } else {
            // Too many tabs, show subset
            let active_index = tab_manager.active_index();
            let half_width = max_tabs_that_fit / 2;
            
            let start_index = if active_index >= half_width {
                (active_index - half_width).min(tab_count.saturating_sub(max_tabs_that_fit))
            } else {
                0
            };
            let end_index = (start_index + max_tabs_that_fit).min(tab_count);
            
            // Add width for left truncation indicator
            if start_index > 0 {
                widths.push(3); // " « "
            }
            
            let visible_tab_count = end_index - start_index;
            let remaining_width = available_width
                .saturating_sub(if start_index > 0 { 3 } else { 0 })
                .saturating_sub(if end_index < tab_count { 3 } else { 0 });
            let avg_width = remaining_width / visible_tab_count;
            let max_name_width = avg_width.saturating_sub(2);
            
            for i in start_index..end_index {
                let tab = &tabs[i];
                let full_name = tab.display_name();
                let truncated_name = self.truncate_name(&full_name, max_name_width);
                let tab_width = truncated_name.len() + 2;
                widths.push(tab_width);
            }
            
            // Add width for right truncation indicator
            if end_index < tab_count {
                widths.push(3); // " » "
            }
        }
        
        widths
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, tab_manager: &TabManager) {
        let available_width = area.width as usize;
        let hint_text = "  Ctrl+N";
        let hint_width = hint_text.len();
        let tabs_width = available_width.saturating_sub(hint_width);
        
        // Render tabs and hint together in a single line
        let mut spans = Vec::new();
        
        // Calculate how to display tabs with truncation
        let tab_spans = self.calculate_tab_spans(tab_manager, tabs_width);
        spans.extend(tab_spans);
        
        // Add the Ctrl+N hint directly after the tabs
        spans.push(Span::styled(hint_text, Style::default().fg(Color::Rgb(120, 120, 120))));
        
        // Create a single line with all spans
        let line = Line::from(spans);
        let paragraph = Paragraph::new(vec![line]);
        
        frame.render_widget(paragraph, area);
    }
    
    fn calculate_tab_spans(&self, tab_manager: &TabManager, available_width: usize) -> Vec<Span<'_>> {
        let mut spans = Vec::new();
        let tabs = tab_manager.tabs();
        let tab_count = tabs.len();
        
        if tab_count == 0 {
            return spans;
        }
        
        // Minimum width per tab (including padding)
        let min_tab_width = 8; // At least "...txt*" or similar
        let max_tabs_that_fit = available_width / min_tab_width;
        
        if tab_count <= max_tabs_that_fit {
            // All tabs can fit, but may need truncation
            let avg_width = available_width / tab_count;
            let max_name_width = avg_width.saturating_sub(2); // Account for padding spaces
            
            for (i, tab) in tabs.iter().enumerate() {
                let full_name = tab.display_name();
                let truncated_name = self.truncate_name(&full_name, max_name_width);
                let tab_text = format!(" {} ", truncated_name);
                
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
                
                spans.push(Span::styled(tab_text, style));
            }
        } else {
            // Too many tabs to show all, show as many as possible centered around active tab
            let active_index = tab_manager.active_index();
            let half_width = max_tabs_that_fit / 2;
            
            let start_index = if active_index >= half_width {
                (active_index - half_width).min(tab_count.saturating_sub(max_tabs_that_fit))
            } else {
                0
            };
            let end_index = (start_index + max_tabs_that_fit).min(tab_count);
            
            // Show truncation indicator if there are tabs before
            if start_index > 0 {
                spans.push(Span::styled(" « ", Style::default().fg(Color::Rgb(120, 120, 120))));
            }
            
            let visible_tab_count = end_index - start_index;
            let avg_width = available_width.saturating_sub(if start_index > 0 { 3 } else { 0 })
                .saturating_sub(if end_index < tab_count { 3 } else { 0 }) / visible_tab_count;
            let max_name_width = avg_width.saturating_sub(2);
            
            for i in start_index..end_index {
                let tab = &tabs[i];
                let full_name = tab.display_name();
                let truncated_name = self.truncate_name(&full_name, max_name_width);
                let tab_text = format!(" {} ", truncated_name);
                
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
                
                spans.push(Span::styled(tab_text, style));
            }
            
            // Show truncation indicator if there are tabs after
            if end_index < tab_count {
                spans.push(Span::styled(" » ", Style::default().fg(Color::Rgb(120, 120, 120))));
            }
        }
        
        spans
    }
    
    fn truncate_name(&self, name: &str, max_width: usize) -> String {
        if name.len() <= max_width {
            name.to_string()
        } else if max_width <= 3 {
            // Too small to show anything meaningful
            "…".to_string()
        } else {
            // Try to keep the file extension visible
            if let Some(dot_pos) = name.rfind('.') {
                let extension = &name[dot_pos..];
                if extension.len() < max_width.saturating_sub(1) {
                    // Can fit extension + some of the name
                    let available_for_name = max_width.saturating_sub(extension.len()).saturating_sub(1);
                    if available_for_name > 0 {
                        return format!("{}…{}", &name[..available_for_name], extension);
                    }
                }
            }
            
            // Fallback: just truncate from the end
            format!("{}…", &name[..max_width.saturating_sub(1)])
        }
    }
}