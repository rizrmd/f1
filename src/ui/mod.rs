mod menu_component;
pub mod scrollbar;
mod status_bar;
mod tab_bar;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::FocusMode;
use crate::editor_widget::EditorWidget;
use crate::file_icons;
use crate::menu::{MenuState, MenuSystem};
use crate::tab::TabManager;
use crate::tree_view::TreeView;

pub use self::menu_component::{MenuAction, MenuComponent, MenuItem};
pub use self::scrollbar::{ScrollbarState, VerticalScrollbar};
use self::status_bar::StatusBar;
use self::tab_bar::TabBar;

pub struct UI {
    pub tab_bar: TabBar,
    status_bar: StatusBar,
}

impl UI {
    pub fn new() -> Self {
        Self {
            tab_bar: TabBar::new(),
            status_bar: StatusBar::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        tab_manager: &mut TabManager,
        warning_message: &Option<String>,
        selected_button: usize,
        is_info: bool,
        menu_system: &MenuSystem,
        tree_view: &Option<TreeView>,
        sidebar_width: u16,
        focus_mode: &FocusMode,
        status_message: &Option<String>,
    ) {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tab bar (reduced from 2 to 1)
                Constraint::Min(0),    // Main content (tree view + editor)
                Constraint::Length(1), // Status bar
            ])
            .split(size);

        // Render tab bar
        self.tab_bar.draw(frame, chunks[0], tab_manager);

        // Split main content area into sidebar and editor if tree view exists
        let main_area = chunks[1];
        if let Some(tree_view) = tree_view {
            // Create horizontal layout with tree view, border, and editor
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(sidebar_width), // Tree view sidebar
                    Constraint::Length(1),             // Vertical border
                    Constraint::Min(0),                // Editor content
                ])
                .split(main_area);

            // Render tree view
            frame.render_widget(tree_view, horizontal_chunks[0]);

            // Draw dimmed vertical border in the dedicated border area
            let border_area = horizontal_chunks[1];
            for y in border_area.y..border_area.y + border_area.height {
                if border_area.x < size.width {
                    frame.buffer_mut()[(border_area.x, y)]
                        .set_symbol("│")
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }

            // Render editor content in the remaining space
            let editor_area = horizontal_chunks[2];
            if let Some(tab) = tab_manager.active_tab_mut() {
                let is_editor_focused = matches!(focus_mode, FocusMode::Editor);
                if tab.preview_mode && tab.is_markdown() {
                    // Render markdown preview
                    let content = tab.buffer.to_string();
                    let preview = crate::markdown_widget::MarkdownWidget::new(&content)
                        .viewport_offset(tab.viewport_offset);
                    frame.render_widget(preview, editor_area);
                } else {
                    // Render normal editor
                    let editor = EditorWidget::new(&tab.buffer, &tab.cursor)
                        .viewport_offset(tab.viewport_offset)
                        .show_line_numbers(true)
                        .focused(is_editor_focused)
                        .word_wrap(tab.word_wrap);
                    frame.render_widget(editor, editor_area);
                }
            }
        } else {
            // No tree view, render editor in full main area
            if let Some(tab) = tab_manager.active_tab_mut() {
                if tab.preview_mode && tab.is_markdown() {
                    // Render markdown preview
                    let content = tab.buffer.to_string();
                    let preview = crate::markdown_widget::MarkdownWidget::new(&content)
                        .viewport_offset(tab.viewport_offset);
                    frame.render_widget(preview, main_area);
                } else {
                    // Render normal editor
                    let editor = EditorWidget::new(&tab.buffer, &tab.cursor)
                        .viewport_offset(tab.viewport_offset)
                        .show_line_numbers(true)
                        .focused(true)
                        .word_wrap(tab.word_wrap);
                    frame.render_widget(editor, main_area);
                }
            }
        }

        // Render status bar
        self.status_bar
            .draw(frame, chunks[2], tab_manager, status_message.as_ref());

        // Render warning dialog if present
        if let Some(message) = warning_message {
            self.draw_warning_dialog(frame, message, selected_button, is_info);
        }

        // Render menus if present
        match &menu_system.state {
            MenuState::MainMenu(menu) => {
                let menu_area = Rect {
                    x: 0,
                    y: size.height.saturating_sub(menu.height + 1),
                    width: menu.width,
                    height: menu.height,
                };
                menu.render(frame, menu_area);
            }
            MenuState::CurrentTabMenu(menu) => {
                let tab_index = tab_manager.active_index();
                let available_width = frame.area().width as usize;
                let tab_x =
                    self.tab_bar
                        .get_tab_x_position(tab_manager, tab_index, available_width);
                let menu_area = Rect {
                    x: tab_x,
                    y: 1, // Directly below tab bar
                    width: menu.width,
                    height: menu.height,
                };
                menu.render(frame, menu_area);
            }
            MenuState::FilePicker(picker_state) => {
                self.draw_file_picker(frame, picker_state);
            }
            MenuState::TreeContextMenu(context_state) => {
                let menu_area = Rect {
                    x: context_state.position.0,
                    y: context_state.position.1,
                    width: context_state.menu.width,
                    height: context_state.menu.height,
                };
                context_state.menu.render(frame, menu_area);
            }
            MenuState::InputDialog(input_state) => {
                self.draw_input_dialog(frame, input_state);
            }
            MenuState::Closed => {}
        }
    }

    fn draw_warning_dialog(
        &self,
        frame: &mut Frame,
        message: &str,
        selected_button: usize,
        is_info: bool,
    ) {
        let size = frame.area();

        // Calculate popup size and position
        let popup_width = (message.len() + 4).clamp(30, 80) as u16;
        let popup_height = 7; // Increased height for buttons
        let popup_x = (size.width.saturating_sub(popup_width)) / 2;
        let popup_y = (size.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Create layout for dialog content
        let dialog_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title spacer
                Constraint::Length(1), // Message
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Buttons
            ])
            .split(popup_area);

        // Render the border and title
        let warning_block = Block::default()
            .borders(Borders::ALL)
            .title(" Warning ")
            .style(Style::default().bg(Color::Red).fg(Color::White));
        frame.render_widget(warning_block, popup_area);

        // Render the message
        let warning_text = Paragraph::new(Line::from(message))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        frame.render_widget(warning_text, dialog_chunks[1]);

        // Create buttons based on dialog type
        let buttons = if is_info {
            // Info dialog - only OK button
            let ok_style = Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD);

            Line::from(vec![Span::styled("       [ OK ]       ", ok_style)])
        } else {
            // Confirmation dialog - Yes/No buttons
            let border_style = Style::default().fg(Color::White);
            let space_style = Style::default();

            let (no_style, no_left_border, no_right_border) = if selected_button == 0 {
                // Selected No: bright red background with white border
                (
                    Style::default()
                        .bg(Color::Rgb(200, 50, 50)) // Bright red
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                    Span::styled("[", border_style.bg(Color::Rgb(200, 50, 50))),
                    Span::styled("]", border_style.bg(Color::Rgb(200, 50, 50))),
                )
            } else {
                // Not selected: dark gray background
                (
                    Style::default()
                        .bg(Color::Rgb(60, 60, 60)) // Dark gray
                        .fg(Color::Rgb(200, 200, 200)),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                )
            };

            let (yes_style, yes_left_border, yes_right_border) = if selected_button == 1 {
                // Selected Yes: bright green background with white border
                (
                    Style::default()
                        .bg(Color::Rgb(50, 200, 50)) // Bright green
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                    Span::styled("[", border_style.bg(Color::Rgb(50, 200, 50))),
                    Span::styled("]", border_style.bg(Color::Rgb(50, 200, 50))),
                )
            } else {
                // Not selected: dark gray background
                (
                    Style::default()
                        .bg(Color::Rgb(60, 60, 60)) // Dark gray
                        .fg(Color::Rgb(200, 200, 200)),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                    Span::styled(" ", Style::default().bg(Color::Rgb(60, 60, 60))),
                )
            };

            Line::from(vec![
                Span::styled("  ", space_style),  // Left padding
                no_left_border,                   // Left border or space
                Span::styled(" No ", no_style),   // No button with padding
                no_right_border,                  // Right border or space
                Span::styled("  ", space_style),  // Space between buttons
                yes_left_border,                  // Left border or space
                Span::styled(" Yes ", yes_style), // Yes button with padding
                yes_right_border,                 // Right border or space
                Span::styled("  ", space_style),  // Right padding
            ])
        };

        let buttons_paragraph = Paragraph::new(buttons).alignment(Alignment::Center);
        frame.render_widget(buttons_paragraph, dialog_chunks[3]);
    }

    fn draw_input_dialog(&self, frame: &mut Frame, input_state: &crate::menu::InputDialogState) {
        let size = frame.area();

        // Calculate dialog size
        let dialog_width = 50u16.min(size.width.saturating_sub(4));
        let dialog_height = 8; // Increased to accommodate spacing
        let dialog_x = (size.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (size.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear the background
        let background_style = Style::default().bg(Color::Rgb(30, 30, 30));
        frame.render_widget(Clear, dialog_area);
        frame.render_widget(
            Block::default()
                .style(background_style)
                .borders(Borders::ALL),
            dialog_area,
        );

        // Split into sections: title, prompt, input, spacing, buttons
        let inner = dialog_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        let dialog_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Prompt
                Constraint::Length(1), // Input
                Constraint::Length(1), // Spacing between input and buttons
                Constraint::Length(1), // Buttons
                Constraint::Min(0),    // Extra space
            ])
            .split(inner);

        // Title
        let title = Line::from(vec![Span::styled(
            "File Operation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]);
        let title_paragraph = Paragraph::new(title).alignment(Alignment::Center);
        frame.render_widget(title_paragraph, dialog_chunks[0]);

        // Prompt
        let prompt = Line::from(vec![Span::raw(&input_state.prompt)]);
        let prompt_paragraph = Paragraph::new(prompt);
        frame.render_widget(prompt_paragraph, dialog_chunks[1]);

        // Input field with cursor and selection
        let mut input_spans = Vec::new();
        let input_bg = Color::Rgb(50, 50, 50);
        let selection_bg = Color::Rgb(100, 100, 200);

        for (i, ch) in input_state.input.chars().enumerate() {
            let is_selected = if let Some(sel_start) = input_state.selection_start {
                let (start, end) = if sel_start < input_state.cursor_position {
                    (sel_start, input_state.cursor_position)
                } else {
                    (input_state.cursor_position, sel_start)
                };
                i >= start && i < end
            } else {
                false
            };

            let style = if is_selected {
                Style::default().bg(selection_bg).fg(Color::White)
            } else {
                Style::default().bg(input_bg).fg(Color::White)
            };

            input_spans.push(Span::styled(ch.to_string(), style));
        }

        // Add cursor
        if input_state.cursor_position == input_state.input.len() {
            input_spans.push(Span::styled(
                "_",
                Style::default()
                    .bg(input_bg)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        } else {
            // Insert cursor indicator at position
            let cursor_style = Style::default().bg(Color::Yellow).fg(Color::Black);
            if input_state.cursor_position < input_spans.len() {
                let ch = input_state
                    .input
                    .chars()
                    .nth(input_state.cursor_position)
                    .unwrap_or(' ');
                input_spans[input_state.cursor_position] =
                    Span::styled(ch.to_string(), cursor_style);
            }
        }

        let input = Line::from(input_spans);
        let input_paragraph = Paragraph::new(input);
        frame.render_widget(input_paragraph, dialog_chunks[2]);

        // Buttons (now at index 4 after adding spacing) with hover effects
        let ok_style = if input_state.hovered_button == Some(0) {
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let cancel_style = if input_state.hovered_button == Some(1) {
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let buttons = Line::from(vec![
            Span::styled(" [Enter] OK  ", ok_style),
            Span::raw("  "),
            Span::styled(" [Esc] Cancel ", cancel_style),
        ]);
        let buttons_paragraph = Paragraph::new(buttons).alignment(Alignment::Center);
        frame.render_widget(buttons_paragraph, dialog_chunks[4]);
    }

    fn draw_file_picker(&self, frame: &mut Frame, picker_state: &crate::menu::FilePickerState) {
        let size = frame.area();

        // Center the file picker modal
        let modal_width = 70u16.min(size.width.saturating_sub(4));
        let modal_height = 24u16.min(size.height.saturating_sub(4));
        let modal_x = (size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (size.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        // Clear the area
        frame.render_widget(Clear, modal_area);

        // Create layout
        let modal_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Current directory
                Constraint::Length(1), // Search input
                Constraint::Length(1), // Separator
                Constraint::Min(0),    // File list
            ])
            .split(modal_area);

        // Render modal border with current directory in title
        let title = format!(" 📁 {} ", picker_state.current_dir.display());
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(Color::Black).fg(Color::White));
        frame.render_widget(modal_block, modal_area);

        // Current directory info
        let dir_info = format!(
            "📂 {}",
            picker_state
                .current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("/")
        );
        let dir_paragraph =
            Paragraph::new(Line::from(dir_info)).style(Style::default().fg(Color::Cyan));
        frame.render_widget(dir_paragraph, modal_chunks[0]);

        // Search input
        let search_text = if picker_state.search_query.is_empty() {
            "🔍 Type to search...".to_string()
        } else {
            format!("🔍 {}", picker_state.search_query)
        };
        let search_input = Paragraph::new(Line::from(search_text))
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(search_input, modal_chunks[1]);

        // File list with two lines per item when searching
        let is_searching = !picker_state.search_query.is_empty();
        let items_per_entry = if is_searching { 2 } else { 1 };

        let total_items = picker_state.filtered_items.len();

        // Calculate scrollbar area
        let scrollbar_width = if total_items * items_per_entry > modal_chunks[3].height as usize {
            1
        } else {
            0
        };

        let file_list_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(scrollbar_width)])
            .split(modal_chunks[3]);

        let file_content_area = file_list_chunks[0];
        let file_scrollbar_area = if scrollbar_width > 0 {
            Some(file_list_chunks[1])
        } else {
            None
        };

        // Calculate visible items and start index
        let available_height = file_content_area.height as usize;
        let visible_items = available_height / items_per_entry;
        let start_index = if picker_state.selected_index >= visible_items {
            picker_state
                .selected_index
                .saturating_sub(visible_items - 1)
        } else {
            0
        };

        let mut file_lines = Vec::new();

        for (i, item) in picker_state
            .filtered_items
            .iter()
            .skip(start_index)
            .take(visible_items)
            .enumerate()
        {
            let global_index = start_index + i;
            // Use hovered_index if available, otherwise use selected_index
            let is_selected = if let Some(hovered) = picker_state.hovered_index {
                global_index == hovered
            } else {
                global_index == picker_state.selected_index
            };

            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            let dim_style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::Gray)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Icon based on type using the modular icon system
            let icon = if item.name == ".." {
                "⬆️"
            } else if item.is_dir {
                file_icons::get_directory_icon(false) // Always show closed folder in file picker
            } else {
                file_icons::get_file_icon(&item.path)
            };

            // First line: icon and name (padded to content area width)
            let name_line = format!(" {}  {}", icon, item.name);
            let content_width = (file_content_area.width as usize).saturating_sub(2);
            let padded_name_line = format!("{:<width$}", name_line, width = content_width);
            file_lines.push(Line::from(Span::styled(padded_name_line, style)));

            // Second line: relative path (only when searching, also padded)
            if is_searching {
                let path_to_show =
                    if item.relative_path.is_empty() || item.relative_path == item.name {
                        ".".to_string()
                    } else {
                        item.relative_path.clone()
                    };
                let path_line = format!("    {}", path_to_show);
                let padded_path_line = format!("{:<width$}", path_line, width = content_width);
                file_lines.push(Line::from(Span::styled(padded_path_line, dim_style)));
            }
        }

        let file_list = Paragraph::new(file_lines);
        frame.render_widget(file_list, file_content_area);

        // Render scrollbar if needed
        if let Some(scrollbar_area) = file_scrollbar_area {
            let scrollbar_state = ScrollbarState::new(total_items, visible_items, start_index);

            let scrollbar = VerticalScrollbar::new(scrollbar_state)
                .style(Style::default().fg(Color::Reset))
                .thumb_style(Style::default().fg(Color::White))
                .track_symbols(VerticalScrollbar::minimal());

            frame.render_widget(scrollbar, scrollbar_area);
        }
    }
}
