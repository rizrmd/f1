mod app;
mod cursor;
mod editor_widget;
mod file_icons;
mod gitignore;
mod keyboard;
mod markdown_widget;
mod menu;
mod rope_buffer;
mod tab;
mod tree_view;
mod ui;

use std::io::{self, stdout};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::App;
use crate::tab::Tab;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    if let Some(args) = std::env::args().nth(1) {
        if let Ok(content) = std::fs::read_to_string(&args) {
            let tab = Tab::from_file(args.into(), &content);
            app.tab_manager.tabs.clear();
            app.tab_manager.add_tab(tab);
        }
    }

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if !app.running {
            break;
        }

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    app.handle_key_event(key);
                }
                crossterm::event::Event::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
