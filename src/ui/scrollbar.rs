use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct ScrollbarState {
    pub content_length: usize,
    pub viewport_size: usize,
    pub position: usize,
}

impl ScrollbarState {
    pub fn new(content_length: usize, viewport_size: usize, position: usize) -> Self {
        Self {
            content_length,
            viewport_size: viewport_size.max(1),
            position: position.min(content_length.saturating_sub(viewport_size)),
        }
    }

    pub fn click_position(&self, track_size: usize, click_y: usize) -> usize {
        if self.content_length <= self.viewport_size {
            return 0;
        }
        
        let ratio = click_y as f64 / track_size as f64;
        let target_line = (self.content_length as f64 * ratio) as usize;
        target_line.saturating_sub(self.viewport_size / 2)
            .min(self.content_length.saturating_sub(self.viewport_size))
    }

    #[allow(dead_code)]
    pub fn is_thumb_at(&self, track_size: usize, y: usize) -> bool {
        if !self.needs_scrollbar() {
            return false;
        }
        
        let thumb_size = self.thumb_size(track_size);
        let thumb_position = self.thumb_position(track_size);
        
        y >= thumb_position && y < thumb_position + thumb_size
    }

    pub fn thumb_size(&self, track_size: usize) -> usize {
        if self.content_length <= self.viewport_size {
            track_size
        } else {
            let ratio = self.viewport_size as f64 / self.content_length as f64;
            (track_size as f64 * ratio).max(1.0) as usize
        }
    }

    pub fn thumb_position(&self, track_size: usize) -> usize {
        if self.content_length <= self.viewport_size {
            0
        } else {
            let thumb_size = self.thumb_size(track_size);
            let available_space = track_size.saturating_sub(thumb_size);
            let ratio = self.position as f64 / (self.content_length - self.viewport_size) as f64;
            (available_space as f64 * ratio) as usize
        }
    }

    pub fn needs_scrollbar(&self) -> bool {
        self.content_length > self.viewport_size
    }
}

pub struct VerticalScrollbar {
    state: ScrollbarState,
    style: Style,
    thumb_style: Style,
    track_symbols: TrackSymbols,
}

#[derive(Debug, Clone)]
pub struct TrackSymbols {
    pub track: &'static str,
    pub thumb: &'static str,
}

impl Default for TrackSymbols {
    fn default() -> Self {
        Self {
            track: "│",
            thumb: "█",
        }
    }
}

impl VerticalScrollbar {
    pub fn new(state: ScrollbarState) -> Self {
        Self {
            state,
            style: Style::default().fg(Color::DarkGray),
            thumb_style: Style::default().fg(Color::Gray),
            track_symbols: TrackSymbols::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn thumb_style(mut self, style: Style) -> Self {
        self.thumb_style = style;
        self
    }

    pub fn track_symbols(mut self, symbols: TrackSymbols) -> Self {
        self.track_symbols = symbols;
        self
    }

    #[allow(dead_code)]
    pub fn compact() -> TrackSymbols {
        TrackSymbols {
            track: "│",
            thumb: "█",
        }
    }

    #[allow(dead_code)]
    pub fn thin() -> TrackSymbols {
        TrackSymbols {
            track: "┃",
            thumb: "▌",
        }
    }

    pub fn minimal() -> TrackSymbols {
        TrackSymbols {
            track: " ",
            thumb: "│",
        }
    }
}

impl Widget for VerticalScrollbar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || !self.state.needs_scrollbar() {
            return;
        }

        let track_height = area.height as usize;
        let thumb_size = self.state.thumb_size(track_height);
        let thumb_position = self.state.thumb_position(track_height);

        for y in 0..area.height {
            let row = y as usize;
            let is_thumb = row >= thumb_position && row < thumb_position + thumb_size;
            
            let symbol = if is_thumb {
                self.track_symbols.thumb
            } else {
                self.track_symbols.track
            };
            
            let style = if is_thumb {
                self.thumb_style
            } else {
                self.style
            };

            buf[(area.x, area.y + y)]
                .set_symbol(symbol)
                .set_style(style);
        }
    }
}

#[derive(Debug, Clone)]
pub struct HorizontalScrollbar {
    state: ScrollbarState,
    style: Style,
    thumb_style: Style,
    track_symbols: HorizontalTrackSymbols,
}

#[derive(Debug, Clone)]
pub struct HorizontalTrackSymbols {
    pub track: &'static str,
    pub thumb: &'static str,
}

impl Default for HorizontalTrackSymbols {
    fn default() -> Self {
        Self {
            track: "─",
            thumb: "█",
        }
    }
}

impl HorizontalScrollbar {
    #[allow(dead_code)]
    pub fn new(state: ScrollbarState) -> Self {
        Self {
            state,
            style: Style::default().fg(Color::DarkGray),
            thumb_style: Style::default().fg(Color::Gray),
            track_symbols: HorizontalTrackSymbols::default(),
        }
    }

    #[allow(dead_code)]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    #[allow(dead_code)]
    pub fn thumb_style(mut self, style: Style) -> Self {
        self.thumb_style = style;
        self
    }

    #[allow(dead_code)]
    pub fn track_symbols(mut self, symbols: HorizontalTrackSymbols) -> Self {
        self.track_symbols = symbols;
        self
    }

    #[allow(dead_code)]
    pub fn compact() -> HorizontalTrackSymbols {
        HorizontalTrackSymbols {
            track: "─",
            thumb: "▬",
        }
    }
}

impl Widget for HorizontalScrollbar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || !self.state.needs_scrollbar() {
            return;
        }

        let track_width = area.width as usize;
        let thumb_size = self.state.thumb_size(track_width);
        let thumb_position = self.state.thumb_position(track_width);

        for x in 0..area.width {
            let col = x as usize;
            let is_thumb = col >= thumb_position && col < thumb_position + thumb_size;
            
            let symbol = if is_thumb {
                self.track_symbols.thumb
            } else {
                self.track_symbols.track
            };
            
            let style = if is_thumb {
                self.thumb_style
            } else {
                self.style
            };

            buf[(area.x + x, area.y)]
                .set_symbol(symbol)
                .set_style(style);
        }
    }
}