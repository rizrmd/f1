use ratatui::prelude::*;
use ratatui::widgets::Widget;
use std::io::{self, Read};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize, Child};
use vte::{Parser, Params};
use crossterm::event::KeyEvent;

struct TerminalState {
    grid: Vec<Vec<(char, Style)>>,
    width: u16,
    height: u16,
    cursor_x: u16,
    cursor_y: u16,
}

impl TerminalState {
    fn new(width: u16, height: u16) -> Self {
        let mut grid = vec![];
        for _ in 0..height {
            let mut row = vec![];
            for _ in 0..width {
                row.push((' ', Style::default()));
            }
            grid.push(row);
        }
        Self { grid, width, height, cursor_x: 0, cursor_y: 0 }
    }

    fn resize(&mut self, new_width: u16, new_height: u16) {
        *self = Self::new(new_width, new_height);
    }

    fn perform(&mut self, byte: u8, parser: &mut Parser) {
        parser.advance(self, byte);
    }
}

impl vte::Perform for TerminalState {
    fn print(&mut self, ch: char) {
        if self.cursor_x < self.width && self.cursor_y < self.height {
            self.grid[self.cursor_y as usize][self.cursor_x as usize] = (ch, Style::default());
        }
        self.cursor_x += 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.cursor_y += 1;
                self.cursor_x = 0;
            }
            b'\r' => {
                self.cursor_x = 0;
            }
            b'\x08' => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        if action == 'H' {
            let y = params.iter().next().and_then(|p| p.first()).map(|&v| v).unwrap_or(1) as u16 - 1;
            let x = params.iter().nth(1).and_then(|p| p.first()).map(|&v| v).unwrap_or(1) as u16 - 1;
            self.cursor_x = x;
            self.cursor_y = y;
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

pub struct TerminalWidget {
    pty: PtyPair,
    #[allow(dead_code)]
    child: Box<dyn Child + Send + Sync>,
    parser: Parser,
    state: TerminalState,
    area: Rect,
}

impl TerminalWidget {
    pub fn new(area: Rect) -> io::Result<Self> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: area.height,
            cols: area.width,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pty = pty_system.openpty(size).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let cmd = CommandBuilder::new("sh");
        let child = pty.slave.spawn_command(cmd).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(Self {
            pty,
            child,
            parser: Parser::new(),
            state: TerminalState::new(area.width, area.height),
            area,
        })
    }

    pub fn resize(&mut self, new_area: Rect) {
        if new_area.width != self.area.width || new_area.height != self.area.height {
            let _ = self.pty.master.resize(PtySize {
                rows: new_area.height,
                cols: new_area.width,
                pixel_width: 0,
                pixel_height: 0,
            });
            self.state.resize(new_area.width, new_area.height);
            self.area = new_area;
        }
    }

    pub fn update(&mut self) {
        let mut reader = self.pty.master.try_clone_reader().unwrap();
        let mut buf = [0; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for &byte in &buf[0..n] {
                        self.state.perform(byte, &mut self.parser);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    pub fn handle_key(&mut self, _key: KeyEvent) {
        // Terminal key handling disabled for now - needs proper PTY writing implementation
        // This is a placeholder implementation
    }
}

impl Widget for &mut TerminalWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.resize(area);
        self.update();
        for (y, row) in self.state.grid.iter().enumerate() {
            for (x, (ch, style)) in row.iter().enumerate() {
                if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + y as u16)) {
                    cell.set_symbol(&ch.to_string())
                        .set_style(*style);
                }
            }
        }
        if self.state.cursor_x < area.width && self.state.cursor_y < area.height {
            if let Some(cell) = buf.cell_mut((area.x + self.state.cursor_x, area.y + self.state.cursor_y)) {
                cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
            }
        }
    }
}
