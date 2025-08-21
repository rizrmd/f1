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
        dragging_tab: Option<usize>,
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
        self.tab_bar
            .draw(frame, chunks[0], tab_manager, dragging_tab);

        let main_area = chunks[1];

        // Split main content area into sidebar and editor if tree view exists
        if let Some(tree_view) = tree_view {
            // Create horizontal layout with tree view and editor
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(sidebar_width), // Tree view sidebar
                    Constraint::Min(0),                // Editor content
                ])
                .split(main_area);

            // Render tree view
            frame.render_widget(tree_view, horizontal_chunks[0]);

            // Render editor content in the remaining space
            let editor_area = horizontal_chunks[1];
            if let Some(tab) = tab_manager.active_tab_mut() {
                // Check if we need to show find/replace bar in editor area
                let final_editor_area = if tab.find_replace_state.active {
                    let bar_height = if tab.find_replace_state.is_replace_mode {
                        2
                    } else {
                        1
                    };
                    let split = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(bar_height), Constraint::Min(0)])
                        .split(editor_area);

                    // Draw find/replace bar at top of editor
                    self.draw_find_replace_bar(frame, split[0], &tab.find_replace_state);
                    split[1]
                } else {
                    editor_area
                };

                let is_editor_focused = matches!(focus_mode, FocusMode::Editor);
                if tab.preview_mode && tab.is_markdown() {
                    // Render markdown preview
                    let content = tab.buffer.to_string();
                    let preview = crate::markdown_widget::MarkdownWidget::new(&content)
                        .viewport_offset(tab.viewport_offset);
                    frame.render_widget(preview, final_editor_area);
                } else {
                    // Render normal editor
                    let mut editor = EditorWidget::new(&tab.buffer, &tab.cursor)
                        .viewport_offset(tab.viewport_offset)
                        .show_line_numbers(true)
                        .focused(is_editor_focused)
                        .word_wrap(tab.word_wrap);

                    // Add find matches if search is active
                    if tab.find_replace_state.active && !tab.find_replace_state.matches.is_empty() {
                        editor = editor.find_matches(
                            &tab.find_replace_state.matches,
                            tab.find_replace_state.current_match_index,
                        );
                    }

                    frame.render_widget(editor, final_editor_area);
                }
            }
        } else {
            // No tree view, render editor in full main area
            if let Some(tab) = tab_manager.active_tab_mut() {
                // Check if we need to show find/replace bar
                let final_editor_area = if tab.find_replace_state.active {
                    let bar_height = if tab.find_replace_state.is_replace_mode {
                        2
                    } else {
                        1
                    };
                    let split = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(bar_height), Constraint::Min(0)])
                        .split(main_area);

                    // Draw find/replace bar at top of editor
                    self.draw_find_replace_bar(frame, split[0], &tab.find_replace_state);
                    split[1]
                } else {
                    main_area
                };

                if tab.preview_mode && tab.is_markdown() {
                    // Render markdown preview
                    let content = tab.buffer.to_string();
                    let preview = crate::markdown_widget::MarkdownWidget::new(&content)
                        .viewport_offset(tab.viewport_offset);
                    frame.render_widget(preview, final_editor_area);
                } else {
                    // Render normal editor
                    let mut editor = EditorWidget::new(&tab.buffer, &tab.cursor)
                        .viewport_offset(tab.viewport_offset)
                        .show_line_numbers(true)
                        .focused(true)
                        .word_wrap(tab.word_wrap);

                    // Add find matches if search is active
                    if tab.find_replace_state.active && !tab.find_replace_state.matches.is_empty() {
                        editor = editor.find_matches(
                            &tab.find_replace_state.matches,
                            tab.find_replace_state.current_match_index,
                        );
                    }

                    frame.render_widget(editor, final_editor_area);
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

    fn draw_find_replace_bar(
        &self,
        frame: &mut Frame,
        area: Rect,
        find_state: &crate::tab::FindReplaceState,
    ) {
        use crate::tab::FindFocusedField;

        // Clear background
        let bg_style = Style::default().bg(Color::Rgb(40, 40, 40));
        frame.render_widget(Block::default().style(bg_style), area);

        // Split into rows for find and optionally replace
        let rows = if find_state.is_replace_mode {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1)])
                .split(area)
        };

        // Draw find row
        let find_row = rows[0];
        let find_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10), // "Find:" label (aligned with Replace)
                Constraint::Min(20),    // Input field (flexible)
                Constraint::Length(12), // Match counter
                Constraint::Length(12), // Find Next button (with padding)
                Constraint::Length(5),  // Case button
                Constraint::Length(5),  // Whole word button
                Constraint::Length(2),  // Right padding
            ])
            .split(find_row);

        // Find label
        let find_label = Span::styled("  Find:", Style::default().fg(Color::Gray));
        frame.render_widget(Paragraph::new(find_label), find_chunks[0]);

        // Find input field
        let find_input_style = if find_state.focused_field == FindFocusedField::Find {
            Style::default().bg(Color::Rgb(60, 60, 60)).fg(Color::White)
        } else {
            Style::default().bg(Color::Rgb(50, 50, 50)).fg(Color::Gray)
        };

        let mut find_text = find_state.find_query.clone();
        if find_state.focused_field == FindFocusedField::Find
            && find_state.find_cursor_position <= find_text.len()
        {
            find_text.insert(find_state.find_cursor_position, '│');
        }

        let find_input = Paragraph::new(find_text).style(find_input_style);
        frame.render_widget(find_input, find_chunks[1]);

        // Match counter
        let match_text = if !find_state.matches.is_empty() {
            if let Some(idx) = find_state.current_match_index {
                format!(" {}/{} ", idx + 1, find_state.matches.len())
            } else {
                format!(" 0/{} ", find_state.matches.len())
            }
        } else if !find_state.find_query.is_empty() {
            " No match ".to_string()
        } else {
            String::new()
        };
        let match_counter = Paragraph::new(match_text)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM))
            .alignment(Alignment::Center);
        frame.render_widget(match_counter, find_chunks[2]);

        // Find Next button with padding
        let find_next_btn = Paragraph::new(" Find Next ")
            .style(
                Style::default()
                    .bg(Color::Rgb(60, 90, 120))
                    .fg(Color::White),
            )
            .alignment(Alignment::Center);
        frame.render_widget(find_next_btn, find_chunks[3]);

        // Case sensitive button
        let case_btn_style = if find_state.case_sensitive {
            Style::default()
                .bg(Color::Rgb(70, 120, 70))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(Color::Rgb(50, 50, 50))
                .fg(Color::Rgb(150, 150, 150))
        };
        let case_btn = Paragraph::new(" Aa ")
            .style(case_btn_style)
            .alignment(Alignment::Center);
        frame.render_widget(case_btn, find_chunks[4]);

        // Whole word button
        let word_btn_style = if find_state.whole_word {
            Style::default()
                .bg(Color::Rgb(70, 120, 70))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(Color::Rgb(50, 50, 50))
                .fg(Color::Rgb(150, 150, 150))
        };
        let word_btn = Paragraph::new(" W ")
            .style(word_btn_style)
            .alignment(Alignment::Center);
        frame.render_widget(word_btn, find_chunks[5]);

        // Right padding (no close button)
        // Close functionality is handled by pressing Escape

        // Draw replace row if in replace mode
        if find_state.is_replace_mode && rows.len() > 1 {
            let replace_row = rows[1];
            let replace_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(10), // "Replace:" label (aligned with Find)
                    Constraint::Min(20),    // Input field (flexible, same as Find)
                    Constraint::Length(12), // Space matching Find's match counter
                    Constraint::Length(12), // Replace button (matches Find Next position)
                    Constraint::Length(5),  // Space matching Case button
                    Constraint::Length(5),  // Space matching Whole word button
                    Constraint::Length(2),  // Right padding (same as Find)
                ])
                .split(replace_row);

            // Replace label
            let replace_label = Span::styled("  Replace:", Style::default().fg(Color::Gray));
            frame.render_widget(Paragraph::new(replace_label), replace_chunks[0]);

            // Replace input field
            let replace_input_style = if find_state.focused_field == FindFocusedField::Replace {
                Style::default().bg(Color::Rgb(60, 60, 60)).fg(Color::White)
            } else {
                Style::default().bg(Color::Rgb(50, 50, 50)).fg(Color::Gray)
            };

            let mut replace_text = find_state.replace_query.clone();
            if find_state.focused_field == FindFocusedField::Replace
                && find_state.replace_cursor_position <= replace_text.len()
            {
                replace_text.insert(find_state.replace_cursor_position, '│');
            }

            let replace_input = Paragraph::new(replace_text).style(replace_input_style);
            frame.render_widget(replace_input, replace_chunks[1]);

            // Empty space for alignment with Find row
            // (aligns with match counter in Find row)

            // Replace button (aligns with Find Next button)
            let replace_btn = Paragraph::new(" Replace ")
                .style(
                    Style::default()
                        .bg(Color::Rgb(50, 100, 50))
                        .fg(Color::White),
                )
                .alignment(Alignment::Center);
            frame.render_widget(replace_btn, replace_chunks[3]);

            // Replace All button (spans positions 4 and 5)
            let replace_all_area = Rect {
                x: replace_chunks[4].x,
                y: replace_chunks[4].y,
                width: replace_chunks[4].width + replace_chunks[5].width,
                height: replace_chunks[4].height,
            };
            let replace_all_btn = Paragraph::new(" Replace All ")
                .style(
                    Style::default()
                        .bg(Color::Rgb(50, 100, 50))
                        .fg(Color::White),
                )
                .alignment(Alignment::Center);
            frame.render_widget(replace_all_btn, replace_all_area);
        }
    }

    fn draw_file_picker(&self, frame: &mut Frame, picker_state: &crate::menu::FilePickerState) {
        let size = frame.area();

        // Center the file picker modal - make it slightly larger without border
        let modal_width = 80u16.min(size.width.saturating_sub(4));
        let modal_height = 28u16.min(size.height.saturating_sub(4));
        let modal_x = (size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (size.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        // Clear the area with a subtle background
        frame.render_widget(Clear, modal_area);

        // Fill background with a subtle color
        let background = Block::default().style(Style::default().bg(Color::Rgb(25, 25, 30)));
        frame.render_widget(background, modal_area);

        // Create layout with padding
        let modal_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Search input
                Constraint::Min(0),    // File list
            ])
            .split(modal_area);

        // Search input with padding
        let search_area = Rect {
            x: modal_chunks[0].x,
            y: modal_chunks[0].y,
            width: modal_chunks[0].width,
            height: 1,
        };

        // Draw search input with proper padding
        let search_text = if picker_state.search_query.is_empty() {
            "  Type to search files...".to_string()
        } else {
            format!("  {}", picker_state.search_query)
        };

        let search_style = if picker_state.search_query.is_empty() {
            Style::default()
                .fg(Color::Rgb(100, 100, 100))
                .bg(Color::Rgb(35, 35, 40))
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(35, 35, 40))
        };

        let mut search_spans = vec![Span::styled(&search_text, search_style)];
        // Add cursor at the end if there's a query
        if !picker_state.search_query.is_empty() {
            search_spans.push(Span::styled(
                "│",
                Style::default().fg(Color::Cyan).bg(Color::Rgb(35, 35, 40)),
            ));
        }

        let search_input = Paragraph::new(Line::from(search_spans))
            .style(Style::default().bg(Color::Rgb(35, 35, 40)));
        frame.render_widget(search_input, search_area);

        // File list with two lines per item when searching
        let is_searching = !picker_state.search_query.is_empty();
        let items_per_entry = if is_searching { 2 } else { 1 };

        let total_items = picker_state.filtered_items.len();

        // Calculate scrollbar area
        let scrollbar_width = if total_items * items_per_entry > modal_chunks[1].height as usize {
            1
        } else {
            0
        };

        let file_list_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(scrollbar_width)])
            .split(modal_chunks[1]);

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
                Style::default().bg(Color::Rgb(60, 60, 70)).fg(Color::White)
            } else {
                Style::default()
                    .fg(Color::Rgb(200, 200, 200))
                    .bg(Color::Rgb(25, 25, 30))
            };

            let dim_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(60, 60, 70))
                    .fg(Color::Rgb(150, 150, 150))
            } else {
                Style::default()
                    .fg(Color::Rgb(100, 100, 100))
                    .bg(Color::Rgb(25, 25, 30))
            };

            // Icon based on type using the modular icon system
            let icon = if item.name == ".." {
                "↑"
            } else if item.is_dir {
                file_icons::get_directory_icon(false) // Always show closed folder in file picker
            } else {
                file_icons::get_file_icon(&item.path)
            };

            // First line: icon and name (padded to content area width)
            let name_line = format!("  {}  {}", icon, item.name);
            let content_width = file_content_area.width as usize;
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
                let path_line = format!("      {}", path_to_show);
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
                .style(Style::default().fg(Color::Rgb(50, 50, 55)))
                .thumb_style(Style::default().fg(Color::Rgb(100, 100, 110)))
                .track_symbols(VerticalScrollbar::minimal());

            frame.render_widget(scrollbar, scrollbar_area);
        }
    }
}
