use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: MenuAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Custom(String),
    Close,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuComponent {
    pub items: Vec<MenuItem>,
    pub selected_index: usize,
    pub hovered_index: Option<usize>,
    pub width: u16,
    pub height: u16,
    pub background_color: Color,
    pub foreground_color: Color,
    pub selected_bg_color: Option<Color>,
    pub selected_fg_color: Option<Color>,
    pub show_border: bool,
}

impl MenuComponent {
    pub fn new(items: Vec<MenuItem>) -> Self {
        let height = items.len() as u16;
        Self {
            items,
            selected_index: 0,
            hovered_index: None,
            width: 30,
            height,
            background_color: Color::Yellow,
            foreground_color: Color::Black,
            selected_bg_color: Some(Color::Yellow),
            selected_fg_color: Some(Color::Black),
            show_border: false,
        }
    }

    pub fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn with_colors(mut self, bg: Color, fg: Color) -> Self {
        self.background_color = bg;
        self.foreground_color = fg;
        self
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn get_selected_action(&self) -> Option<&MenuAction> {
        self.items.get(self.selected_index).map(|item| &item.action)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Clear the area
        frame.render_widget(Clear, area);

        let mut lines = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let is_selected = i == self.selected_index;
            let is_hovered = self.hovered_index == Some(i);

            let style = if is_selected {
                // Selected item - white background
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else if is_hovered {
                // Hovered item - slightly lighter background with bold text
                Style::default()
                    .bg(Color::LightYellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Normal item
                Style::default()
                    .bg(self.background_color)
                    .fg(self.foreground_color)
            };

            let line_text = if let Some(shortcut) = &item.shortcut {
                // Right-align shortcut: " item_name                shortcut"
                let available_space = self.width as usize - 2; // -2 for left and right padding
                let shortcut_len = shortcut.len();
                let item_len = item.label.len();

                if item_len + shortcut_len < available_space {
                    // Enough space to separate item and shortcut
                    let spaces_needed = available_space - item_len - shortcut_len;
                    format!(" {}{}{} ", item.label, " ".repeat(spaces_needed), shortcut)
                } else {
                    // Not enough space, truncate item name
                    let max_item_len = available_space.saturating_sub(shortcut_len + 1);
                    let truncated_item = if item.label.len() > max_item_len {
                        format!("{}…", &item.label[..max_item_len.saturating_sub(1)])
                    } else {
                        item.label.clone()
                    };
                    let spaces_needed = available_space - truncated_item.len() - shortcut_len;
                    format!(
                        " {}{}{} ",
                        truncated_item,
                        " ".repeat(spaces_needed),
                        shortcut
                    )
                }
            } else {
                let mut text = format!(" {}", item.label);
                while text.len() < self.width as usize {
                    text.push(' ');
                }
                text.truncate(self.width as usize);
                text
            };

            lines.push(Line::from(Span::styled(line_text, style)));
        }

        let menu_paragraph = Paragraph::new(lines);
        frame.render_widget(menu_paragraph, area);
    }

    pub fn is_position_inside(&self, area: &Rect, x: u16, y: u16) -> bool {
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    }

    pub fn get_clicked_item(&self, area: &Rect, x: u16, y: u16) -> Option<usize> {
        if !self.is_position_inside(area, x, y) {
            return None;
        }

        let relative_y = y.saturating_sub(area.y);
        if relative_y < self.items.len() as u16 {
            Some(relative_y as usize)
        } else {
            None
        }
    }
}

impl MenuItem {
    pub fn new(label: &str, action: MenuAction) -> Self {
        Self {
            label: label.to_string(),
            shortcut: None,
            action,
        }
    }

    pub fn with_shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_string());
        self
    }
}
