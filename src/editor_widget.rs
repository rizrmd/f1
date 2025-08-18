use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{cursor::{Cursor, Position}, rope_buffer::RopeBuffer, ui::{VerticalScrollbar, ScrollbarState}};

pub struct EditorWidget<'a> {
    buffer: &'a RopeBuffer,
    cursor: &'a Cursor,
    viewport_offset: (usize, usize),
    show_line_numbers: bool,
    focused: bool,
    show_scrollbar: bool,
}

impl<'a> EditorWidget<'a> {
    pub fn new(buffer: &'a RopeBuffer, cursor: &'a Cursor) -> Self {
        Self {
            buffer,
            cursor,
            viewport_offset: (0, 0),
            show_line_numbers: true,
            focused: true,
            show_scrollbar: true,
        }
    }

    pub fn viewport_offset(mut self, offset: (usize, usize)) -> Self {
        self.viewport_offset = offset;
        self
    }

    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    #[allow(dead_code)]
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    fn calculate_line_number_width(&self) -> u16 {
        let max_line = self.buffer.len_lines();
        let width = max_line.to_string().len();
        (width + 1).max(4) as u16
    }

    fn render_line(&self, line_idx: usize, cursor_col: Option<usize>) -> Vec<Span<'static>> {
        let line_text = self.buffer.get_line_text(line_idx);
        let mut spans = Vec::new();
        
        // Get selection range if any
        let selection = self.cursor.get_selection();
        
        for (col, ch) in line_text.chars().enumerate() {
            let mut style = Style::default();
            
            // Check if this character is within the selection
            let is_selected = if let Some((start, end)) = selection {
                self.is_position_selected(Position::new(line_idx, col), start, end)
            } else {
                false
            };
            
            if is_selected {
                // Selected text: white text on blue background
                style = style.bg(Color::Blue).fg(Color::White);
            } else if self.focused && cursor_col == Some(col) {
                // Cursor position: white text on gray background
                style = style.bg(Color::Rgb(100, 100, 100)).fg(Color::White);
            }
            
            spans.push(Span::styled(ch.to_string(), style));
        }
        
        // Handle cursor at end of line
        if self.focused && cursor_col == Some(line_text.len()) {
            let is_cursor_selected = if let Some((start, end)) = selection {
                self.is_position_selected(Position::new(line_idx, line_text.len()), start, end)
            } else {
                false
            };
            
            let style = if is_cursor_selected {
                Style::default().bg(Color::Blue)
            } else {
                Style::default().bg(Color::Rgb(100, 100, 100))
            };
            spans.push(Span::styled(" ", style));
        }
        
        // Handle empty lines with cursor
        if spans.is_empty() && self.focused && cursor_col == Some(0) {
            spans.push(Span::styled(" ", Style::default().bg(Color::Rgb(100, 100, 100))));
        }
        
        spans
    }
    
    fn is_position_selected(&self, pos: Position, start: Position, end: Position) -> bool {
        if pos.line > end.line || pos.line < start.line {
            return false;
        }
        
        if pos.line == start.line && pos.line == end.line {
            // Same line selection
            pos.column >= start.column && pos.column < end.column
        } else if pos.line == start.line {
            // First line of multi-line selection
            pos.column >= start.column
        } else if pos.line == end.line {
            // Last line of multi-line selection
            pos.column < end.column
        } else {
            // Middle lines of multi-line selection
            true
        }
    }
}

impl<'a> Widget for EditorWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE);
        
        let inner = block.inner(area);
        block.render(area, buf);
        
        let line_number_width = if self.show_line_numbers {
            self.calculate_line_number_width()
        } else {
            0
        };
        
        let scrollbar_width = if self.show_scrollbar && self.buffer.len_lines() > inner.height as usize {
            1
        } else {
            0
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(line_number_width),
                Constraint::Min(0),
                Constraint::Length(scrollbar_width),
            ])
            .split(inner);
        
        let line_numbers_area = chunks[0];
        let content_area = chunks[1];
        let scrollbar_area = if scrollbar_width > 0 { Some(chunks[2]) } else { None };
        
        let visible_lines = content_area.height as usize;
        let start_line = self.viewport_offset.0;
        let end_line = (start_line + visible_lines).min(self.buffer.len_lines());
        
        // Clear the entire inner area first to prevent artifacts
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                buf[(x, y)]
                    .set_symbol(" ")
                    .set_style(Style::default());
            }
        }
        
        if self.show_line_numbers && line_number_width > 0 {
            let mut line_numbers = Vec::new();
            for line_idx in start_line..end_line {
                let line_num = format!("{:>width$} ", line_idx + 1, width = (line_number_width - 1) as usize);
                line_numbers.push(Line::from(Span::styled(
                    line_num,
                    Style::default().fg(Color::DarkGray),
                )));
            }
            
            let line_numbers_widget = Paragraph::new(line_numbers);
            line_numbers_widget.render(line_numbers_area, buf);
        }
        
        let mut lines = Vec::new();
        for line_idx in start_line..end_line {
            let cursor_col = if line_idx == self.cursor.position.line {
                Some(self.cursor.position.column)
            } else {
                None
            };
            
            let spans = self.render_line(line_idx, cursor_col);
            lines.push(Line::from(spans));
        }
        
        if lines.is_empty() && self.buffer.len_lines() == 0 {
            let cursor_col = if self.cursor.position.line == 0 {
                Some(self.cursor.position.column)
            } else {
                None
            };
            
            let spans = if self.focused && cursor_col == Some(0) {
                vec![Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60)))]
            } else {
                vec![Span::raw("")]
            };
            lines.push(Line::from(spans));
        }
        
        let content = Paragraph::new(lines);
        content.render(content_area, buf);

        // Render scrollbar if needed
        if let Some(scrollbar_area) = scrollbar_area {
            let scrollbar_state = ScrollbarState::new(
                self.buffer.len_lines(),
                visible_lines,
                start_line,
            );
            
            let scrollbar = VerticalScrollbar::new(scrollbar_state)
                .style(Style::default().fg(Color::Reset))
                .thumb_style(Style::default().fg(Color::White))
                .track_symbols(VerticalScrollbar::minimal());
            
            scrollbar.render(scrollbar_area, buf);
        }
    }
}