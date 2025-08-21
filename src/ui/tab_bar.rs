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

    pub fn get_tab_x_position(
        &self,
        tab_manager: &TabManager,
        target_tab_index: usize,
        available_width: usize,
    ) -> u16 {
        let hint_text = "  Ctrl+N";
        let hint_width = hint_text.len();
        let tabs_width = available_width.saturating_sub(hint_width);

        let tabs = tab_manager.tabs();
        let tab_count = tabs.len();

        if tab_count == 0 {
            return 0;
        }

        // Fixed width per tab
        const TAB_WIDTH: usize = 14;
        let max_tabs_that_fit = tabs_width / TAB_WIDTH;

        if tab_count <= max_tabs_that_fit {
            // All tabs are visible with fixed width
            // Simple calculation: tab_index * TAB_WIDTH
            (target_tab_index * TAB_WIDTH) as u16
        } else {
            // Too many tabs, showing subset with scrolling
            let active_index = tab_manager.active_index();
            let half_width = max_tabs_that_fit / 2;

            let start_index = if active_index >= half_width {
                (active_index - half_width).min(tab_count.saturating_sub(max_tabs_that_fit))
            } else {
                0
            };
            let end_index = (start_index + max_tabs_that_fit).min(tab_count);

            // Check if target tab is visible
            if target_tab_index < start_index || target_tab_index >= end_index {
                return 0; // Tab is not visible
            }

            // Calculate position
            let mut x_pos = 0u16;

            // Account for left truncation indicator
            if start_index > 0 {
                x_pos = 3; // Width of " « "
            }

            // Add offset for the target tab
            let tab_offset = target_tab_index - start_index;
            x_pos += (tab_offset * TAB_WIDTH) as u16;

            x_pos
        }
    }

    pub fn draw(
        &self,
        frame: &mut Frame,
        area: Rect,
        tab_manager: &TabManager,
        dragging_tab: Option<usize>,
    ) {
        let available_width = area.width as usize;
        let hint_text = "  Ctrl+N";
        let hint_width = hint_text.len();
        let tabs_width = available_width.saturating_sub(hint_width);

        // Render tabs and hint together in a single line
        let mut spans = Vec::new();

        // Calculate how to display tabs with truncation
        let tab_spans = self.calculate_tab_spans(tab_manager, tabs_width, dragging_tab);
        spans.extend(tab_spans);

        // Add the Ctrl+N hint directly after the tabs
        spans.push(Span::styled(
            hint_text,
            Style::default().fg(Color::Rgb(120, 120, 120)),
        ));

        // Create a single line with all spans
        let line = Line::from(spans);
        let paragraph = Paragraph::new(vec![line]);

        frame.render_widget(paragraph, area);
    }

    fn calculate_tab_spans(
        &self,
        tab_manager: &TabManager,
        available_width: usize,
        dragging_tab: Option<usize>,
    ) -> Vec<Span<'_>> {
        let mut spans = Vec::new();
        let tabs = tab_manager.tabs();
        let tab_count = tabs.len();

        if tab_count == 0 {
            return spans;
        }

        // Fixed width per tab
        const TAB_WIDTH: usize = 14;
        const TAB_CONTENT_WIDTH: usize = TAB_WIDTH - 2; // Minus padding
        let max_tabs_that_fit = available_width / TAB_WIDTH;

        if tab_count <= max_tabs_that_fit {
            // All tabs can fit with fixed width
            for (i, tab) in tabs.iter().enumerate() {
                let full_name = tab.display_name();
                let truncated_name = self.truncate_name(&full_name, TAB_CONTENT_WIDTH);

                // Pad to fixed width
                let tab_text = format!(" {:<width$} ", truncated_name, width = TAB_CONTENT_WIDTH);

                let style = if Some(i) == dragging_tab {
                    // Dragging tab: highlighted differently
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(100, 100, 100))
                        .add_modifier(Modifier::BOLD)
                } else if i == tab_manager.active_index() {
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
                spans.push(Span::styled(
                    " « ",
                    Style::default().fg(Color::Rgb(120, 120, 120)),
                ));
            }

            for (i, tab) in tabs
                .iter()
                .enumerate()
                .skip(start_index)
                .take(end_index - start_index)
            {
                let full_name = tab.display_name();
                let truncated_name = self.truncate_name(&full_name, TAB_CONTENT_WIDTH);

                // Pad to fixed width
                let tab_text = format!(" {:<width$} ", truncated_name, width = TAB_CONTENT_WIDTH);

                let style = if Some(i) == dragging_tab {
                    // Dragging tab: highlighted differently
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(100, 100, 100))
                        .add_modifier(Modifier::BOLD)
                } else if i == tab_manager.active_index() {
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
                spans.push(Span::styled(
                    " » ",
                    Style::default().fg(Color::Rgb(120, 120, 120)),
                ));
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
                    let available_for_name =
                        max_width.saturating_sub(extension.len()).saturating_sub(1);
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
