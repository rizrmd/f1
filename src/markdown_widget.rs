use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::ui::{ScrollbarState, VerticalScrollbar};

pub struct MarkdownWidget<'a> {
    content: &'a str,
    viewport_offset: (usize, usize),
    show_scrollbar: bool,
}

impl<'a> MarkdownWidget<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            viewport_offset: (0, 0),
            show_scrollbar: true,
        }
    }

    pub fn viewport_offset(mut self, offset: (usize, usize)) -> Self {
        self.viewport_offset = offset;
        self
    }

    #[allow(dead_code)]
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
}

impl<'a> Widget for MarkdownWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Parse markdown using custom implementation
        let lines = self.parse_markdown();

        // Calculate scrollbar area
        let scrollbar_width = if self.show_scrollbar && lines.len() > area.height as usize {
            1
        } else {
            0
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(scrollbar_width)])
            .split(area);

        let content_area = chunks[0];
        let scrollbar_area = if scrollbar_width > 0 {
            Some(chunks[1])
        } else {
            None
        };

        // Apply viewport offset
        let start_line = self.viewport_offset.0.min(lines.len().saturating_sub(1));
        let visible_height = content_area.height as usize;
        let visible_lines: Vec<Line> = lines
            .iter()
            .skip(start_line)
            .take(visible_height)
            .cloned()
            .collect();

        // Render using Paragraph widget
        let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });

        paragraph.render(content_area, buf);

        // Render scrollbar if needed
        if let Some(scrollbar_area) = scrollbar_area {
            let scrollbar_state = ScrollbarState::new(lines.len(), visible_height, start_line);

            let scrollbar = VerticalScrollbar::new(scrollbar_state)
                .style(Style::default().fg(Color::Reset))
                .thumb_style(Style::default().fg(Color::White))
                .track_symbols(VerticalScrollbar::minimal());

            scrollbar.render(scrollbar_area, buf);
        }
    }
}

impl<'a> MarkdownWidget<'a> {
    pub fn parse_markdown(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let content_lines: Vec<&str> = self.content.lines().collect();
        let mut i = 0;

        while i < content_lines.len() {
            let line = content_lines[i];

            // Handle code block markers
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::DarkGray),
                )));
                i += 1;
                continue;
            }

            if in_code_block {
                // Inside code block - render as-is with monospace styling
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Green).bg(Color::Rgb(20, 20, 20)),
                )));
                i += 1;
                continue;
            }

            // Check if this line starts a table
            if line.trim().contains('|')
                && !line.trim().starts_with("```")
                && !line.trim().is_empty()
            {
                let (table_lines, consumed) = self.parse_table_block(&content_lines[i..]);
                if !table_lines.is_empty() {
                    lines.extend(table_lines);
                    i += consumed;
                } else {
                    // Fallback to regular line parsing if table parsing failed
                    let parsed_line = self.parse_line(line);
                    lines.push(parsed_line);
                    i += 1;
                }
            } else {
                let parsed_line = self.parse_line(line);
                lines.push(parsed_line);
                i += 1;
            }
        }

        lines
    }

    fn parse_line(&self, line: &str) -> Line<'static> {
        // Handle headers
        if line.starts_with("### ") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if line.starts_with("## ") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if line.starts_with("# ") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Handle lists
        if line.trim().starts_with("- ") || line.trim().starts_with("* ") {
            let indent = line.len() - line.trim_start().len();
            let bullet_indent = " ".repeat(indent);
            let text_start = line
                .find(|c: char| c != ' ' && c != '-' && c != '*')
                .unwrap_or(line.len());
            let list_text = if text_start < line.len() {
                &line[text_start..]
            } else {
                ""
            };

            return Line::from(vec![
                Span::styled(bullet_indent, Style::default()),
                Span::styled("• ", Style::default().fg(Color::Yellow)),
                Span::styled(list_text.to_string(), Style::default().fg(Color::White)),
            ]);
        }

        // Handle blockquotes
        if line.trim().starts_with("> ") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        // Handle inline formatting (bold, italic, code)
        if line.contains("**") || line.contains("*") || line.contains("`") {
            return self.parse_inline_formatting(line);
        }

        // Default text
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::White),
        ))
    }

    fn parse_inline_formatting(&self, line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    // Handle inline code
                    if !current.is_empty() {
                        spans.push(Span::styled(
                            current.clone(),
                            Style::default().fg(Color::White),
                        ));
                        current.clear();
                    }

                    let mut code_text = String::new();
                    let mut found_end = false;

                    for ch in chars.by_ref() {
                        if ch == '`' {
                            found_end = true;
                            break;
                        }
                        code_text.push(ch);
                    }

                    if found_end {
                        spans.push(Span::styled(
                            code_text,
                            Style::default().fg(Color::Green).bg(Color::Rgb(40, 40, 40)),
                        ));
                    } else {
                        current.push('`');
                        current.push_str(&code_text);
                    }
                }
                '*' => {
                    if chars.peek() == Some(&'*') {
                        // Handle bold **text**
                        chars.next(); // consume second *

                        if !current.is_empty() {
                            spans.push(Span::styled(
                                current.clone(),
                                Style::default().fg(Color::White),
                            ));
                            current.clear();
                        }

                        let mut bold_text = String::new();
                        let mut found_end = false;

                        while let Some(ch) = chars.next() {
                            if ch == '*' && chars.peek() == Some(&'*') {
                                chars.next(); // consume second *
                                found_end = true;
                                break;
                            }
                            bold_text.push(ch);
                        }

                        if found_end {
                            spans.push(Span::styled(
                                bold_text,
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            current.push_str("**");
                            current.push_str(&bold_text);
                        }
                    } else {
                        // Handle italic *text*
                        if !current.is_empty() {
                            spans.push(Span::styled(
                                current.clone(),
                                Style::default().fg(Color::White),
                            ));
                            current.clear();
                        }

                        let mut italic_text = String::new();
                        let mut found_end = false;

                        for ch in chars.by_ref() {
                            if ch == '*' {
                                found_end = true;
                                break;
                            }
                            italic_text.push(ch);
                        }

                        if found_end {
                            spans.push(Span::styled(
                                italic_text,
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::ITALIC),
                            ));
                        } else {
                            current.push('*');
                            current.push_str(&italic_text);
                        }
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            spans.push(Span::styled(current, Style::default().fg(Color::White)));
        }

        Line::from(spans)
    }

    fn parse_table_block(&self, lines: &[&str]) -> (Vec<Line<'static>>, usize) {
        let mut result = Vec::new();
        let mut table_rows = Vec::new();
        let mut consumed = 0;

        // Collect all consecutive table lines
        for line in lines {
            if line.trim().contains('|') && !line.trim().is_empty() {
                table_rows.push(*line);
                consumed += 1;
            } else {
                break;
            }
        }

        if table_rows.is_empty() {
            return (result, 0);
        }

        // Parse table structure
        let mut parsed_rows: Vec<Vec<String>> = Vec::new();
        let mut separator_index = None;

        for (i, row) in table_rows.iter().enumerate() {
            let trimmed = row.trim();

            // Check if it's a separator row
            if trimmed
                .chars()
                .all(|c| c == '|' || c == '-' || c == ':' || c.is_whitespace())
            {
                separator_index = Some(i);
                continue;
            }

            // Parse regular row
            let cells: Vec<String> = trimmed
                .split('|')
                .map(|cell| cell.trim().to_string())
                .filter(|cell| !cell.is_empty())
                .collect();

            if !cells.is_empty() {
                parsed_rows.push(cells);
            }
        }

        if parsed_rows.is_empty() {
            return (result, consumed);
        }

        // Calculate column widths
        let num_cols = parsed_rows.iter().map(|row| row.len()).max().unwrap_or(0);
        let mut col_widths = vec![0; num_cols];

        for row in &parsed_rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    let display_len = self.calculate_display_length(cell);
                    col_widths[i] = col_widths[i].max(display_len);
                }
            }
        }

        // Ensure minimum width and add padding
        for width in &mut col_widths {
            *width = (*width + 4).max(10); // Minimum 10 chars, +4 for padding (space + content + space)
        }

        // Render table
        let header_present = separator_index.is_some();

        for (row_idx, row) in parsed_rows.iter().enumerate() {
            // Add top border for first row
            if row_idx == 0 {
                result.push(self.create_table_border(&col_widths, "┌", "┬", "┐", "─"));
            }

            // Add header separator after first row if separator was found
            if header_present && row_idx == 1 {
                result.push(self.create_table_border(&col_widths, "├", "┼", "┤", "─"));
            }

            // Add row content
            result.push(self.create_table_row(row, &col_widths, header_present && row_idx == 0));
        }

        // Add bottom border
        result.push(self.create_table_border(&col_widths, "└", "┴", "┘", "─"));

        (result, consumed)
    }

    fn create_table_border(
        &self,
        col_widths: &[usize],
        left: &str,
        mid: &str,
        right: &str,
        fill: &str,
    ) -> Line<'static> {
        let mut border = String::new();
        border.push_str(left);

        for (i, &width) in col_widths.iter().enumerate() {
            border.push_str(&fill.repeat(width));
            if i < col_widths.len() - 1 {
                border.push_str(mid);
            }
        }
        border.push_str(right);

        Line::from(Span::styled(border, Style::default().fg(Color::Blue)))
    }

    fn create_table_row(
        &self,
        row: &[String],
        col_widths: &[usize],
        is_header: bool,
    ) -> Line<'static> {
        let mut spans = Vec::new();
        spans.push(Span::styled("│", Style::default().fg(Color::Blue)));

        for (i, cell) in row.iter().enumerate() {
            let width = col_widths.get(i).copied().unwrap_or(8);

            // Calculate actual display length (without markdown formatting characters)
            let display_len = self.calculate_display_length(cell);

            // Create padded cell content with fixed width
            let padded_content = if display_len >= width.saturating_sub(2) {
                // Truncate if too long, keeping some padding
                let max_len = width.saturating_sub(3);
                let truncated = if cell.len() > max_len {
                    &cell[..max_len]
                } else {
                    cell
                };
                format!(" {} ", truncated)
            } else {
                // Pad to exact width with left alignment
                let content_width = width.saturating_sub(2);
                format!(" {:<width$} ", cell, width = content_width)
            };

            // Apply formatting to the cell content
            if cell.contains("**") || cell.contains("*") || cell.contains("`") {
                // For formatted content, we need to handle it differently
                spans.push(Span::styled(" ", Style::default()));
                let formatted_line = self.parse_inline_formatting(cell);
                spans.extend(formatted_line.spans);

                // Calculate how much padding we need after the formatted content
                let remaining_width = width.saturating_sub(display_len + 2);
                if remaining_width > 0 {
                    spans.push(Span::styled(
                        " ".repeat(remaining_width + 1),
                        Style::default(),
                    ));
                } else {
                    spans.push(Span::styled(" ", Style::default()));
                }
            } else {
                // Regular cell content with proper padding
                let style = if is_header {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(padded_content, style));
            }

            spans.push(Span::styled("│", Style::default().fg(Color::Blue)));
        }

        // Fill remaining columns if row is shorter
        for &width in col_widths.iter().skip(row.len()) {
            let empty_cell = format!(" {:<width$} ", "", width = width.saturating_sub(2));
            spans.push(Span::styled(empty_cell, Style::default()));
            spans.push(Span::styled("│", Style::default().fg(Color::Blue)));
        }

        Line::from(spans)
    }

    fn calculate_display_length(&self, text: &str) -> usize {
        let mut display_len = 0;
        let mut chars = text.chars().peekable();
        let mut in_code = false;

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    in_code = !in_code;
                    // Don't count backticks
                }
                '*' => {
                    if chars.peek() == Some(&'*') {
                        // Bold **text** - skip both asterisks
                        chars.next();
                        // Don't count asterisks
                    } else {
                        // Italic *text* - don't count asterisk
                    }
                }
                _ => {
                    display_len += 1;
                }
            }
        }

        display_len
    }
}
