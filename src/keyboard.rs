use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

use crate::{cursor::Cursor, rope_buffer::RopeBuffer};

// Simple in-memory clipboard
static CLIPBOARD: OnceLock<Arc<Mutex<String>>> = OnceLock::new();

pub fn handle_key_event(
    key: KeyEvent,
    buffer: &mut RopeBuffer,
    cursor: &mut Cursor,
) -> Option<EditorCommand> {
    // Check for any modifier key
    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    let has_super = key.modifiers.contains(KeyModifiers::SUPER);
    let has_meta = key.modifiers.contains(KeyModifiers::META);

    // On macOS, Option key might be reported as ALT or META
    let has_option = has_alt || has_meta;
    
    // On macOS, Cmd key is often not passed through terminal emulators
    // Some terminals map Cmd to other modifiers or don't pass it at all
    // We'll use Ctrl as the primary modifier for all platforms
    #[cfg(target_os = "macos")]
    let has_cmd = has_super || has_meta;
    #[cfg(not(target_os = "macos"))]
    let has_cmd = false;
    
    // Use either Ctrl or Cmd (on macOS) for standard shortcuts
    let has_primary_modifier = has_ctrl || has_cmd;

    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        // Quit - Ctrl+Q
        KeyCode::Char('q') if has_ctrl => Some(EditorCommand::Quit),

        // Save - Ctrl+S
        KeyCode::Char('s') if has_ctrl => Some(EditorCommand::Save),

        // New Tab - Ctrl+N
        KeyCode::Char('n') if has_ctrl => Some(EditorCommand::NewTab),

        // Close Tab - Ctrl+W
        KeyCode::Char('w') if has_ctrl => Some(EditorCommand::CloseTab),

        // Next Tab - Ctrl+]
        KeyCode::Char(']') if has_ctrl => Some(EditorCommand::NextTab),

        // Previous Tab - Ctrl+[
        KeyCode::Char('[') if has_ctrl => Some(EditorCommand::PrevTab),

        // Select all - Ctrl+A or Cmd+A
        KeyCode::Char('a') if has_primary_modifier => {
            cursor.select_all(buffer);
            None
        }

        // Copy - Ctrl+C or Cmd+C
        KeyCode::Char('c') if has_primary_modifier => {
            copy_selection(buffer, cursor);
            None
        }

        // Cut - Ctrl+X or Cmd+X (cuts selection or current line if no selection)
        KeyCode::Char('x') if has_primary_modifier => {
            if cursor.has_selection() {
                cut_selection(buffer, cursor);
            } else {
                cut_current_line(buffer, cursor);
            }
            Some(EditorCommand::Modified)
        }
        
        // Cut current line - Ctrl+K or Cmd+K (alternative shortcut)
        KeyCode::Char('k') if has_primary_modifier => {
            if cursor.has_selection() {
                cut_selection(buffer, cursor);
            } else {
                cut_current_line(buffer, cursor);
            }
            Some(EditorCommand::Modified)
        }

        // Paste - Ctrl+V or Cmd+V
        KeyCode::Char('v') if has_primary_modifier => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            }
            paste_from_clipboard(buffer, cursor);
            Some(EditorCommand::Modified)
        }

        // Undo - Ctrl+Z or Cmd+Z
        KeyCode::Char('z') if has_primary_modifier && !has_shift => Some(EditorCommand::Undo),

        // Redo - Ctrl+Shift+Z, Cmd+Shift+Z, or Ctrl+Y
        KeyCode::Char('z') if has_primary_modifier && has_shift => Some(EditorCommand::Redo),
        KeyCode::Char('y') if has_primary_modifier => Some(EditorCommand::Redo),

        // Toggle Preview - Ctrl+U (for markdown files)
        KeyCode::Char('u') if has_ctrl => Some(EditorCommand::TogglePreview),

        // Toggle Word Wrap - Alt+W
        KeyCode::Char('w') if has_alt => Some(EditorCommand::ToggleWordWrap),

        // Menu - F1
        KeyCode::F(1) => Some(EditorCommand::ToggleMenu),

        // Open File - Ctrl+P
        KeyCode::Char('p') if has_ctrl => Some(EditorCommand::OpenFile),

        // Current Tab - Ctrl+G
        KeyCode::Char('g') if has_ctrl => Some(EditorCommand::CurrentTab),

        // Find - Ctrl+F
        KeyCode::Char('f') if has_ctrl && !has_shift => Some(EditorCommand::Find),

        // Find and Replace - Ctrl+Shift+F
        KeyCode::Char('f') if has_ctrl && has_shift => Some(EditorCommand::FindReplace),

        // Word navigation with selection - Shift+Option/Alt + Arrow
        KeyCode::Left if has_option && has_shift => {
            cursor.move_word_left_with_selection(buffer, true);
            None
        }
        KeyCode::Right if has_option && has_shift => {
            cursor.move_word_right_with_selection(buffer, true);
            None
        }

        // Word navigation with selection - Shift+Ctrl + Arrow
        KeyCode::Left if has_ctrl && has_shift => {
            cursor.move_word_left_with_selection(buffer, true);
            None
        }
        KeyCode::Right if has_ctrl && has_shift => {
            cursor.move_word_right_with_selection(buffer, true);
            None
        }

        // Word navigation - Option/Alt + Arrow (works when terminal is configured)
        KeyCode::Left if has_option && !has_shift => {
            cursor.move_word_left_with_selection(buffer, false);
            None
        }
        KeyCode::Right if has_option && !has_shift => {
            cursor.move_word_right_with_selection(buffer, false);
            None
        }

        // Alternative word navigation - Ctrl + Arrow (always works)
        KeyCode::Left if has_ctrl && !has_shift => {
            cursor.move_word_left_with_selection(buffer, false);
            None
        }
        KeyCode::Right if has_ctrl && !has_shift => {
            cursor.move_word_right_with_selection(buffer, false);
            None
        }

        // Escape sequences for configured terminals (Option+Arrow sends these)
        KeyCode::Char('b') if (has_alt || has_meta) && !has_shift => {
            cursor.move_word_left_with_selection(buffer, false);
            None
        }
        KeyCode::Char('f') if (has_alt || has_meta) && !has_shift => {
            cursor.move_word_right_with_selection(buffer, false);
            None
        }

        // Basic navigation with selection - Shift + Arrow
        KeyCode::Left if has_shift && !has_ctrl && !has_option => {
            cursor.move_left_with_selection(buffer, true);
            None
        }
        KeyCode::Right if has_shift && !has_ctrl && !has_option => {
            cursor.move_right_with_selection(buffer, true);
            None
        }
        KeyCode::Up if has_shift => {
            cursor.move_up_with_selection(buffer, true);
            None
        }
        KeyCode::Down if has_shift => {
            cursor.move_down_with_selection(buffer, true);
            None
        }

        // Basic navigation without selection
        KeyCode::Left if !has_ctrl && !has_option && !has_shift => {
            cursor.move_left_with_selection(buffer, false);
            None
        }
        KeyCode::Right if !has_ctrl && !has_option && !has_shift => {
            cursor.move_right_with_selection(buffer, false);
            None
        }
        KeyCode::Up if !has_shift => {
            cursor.move_up_with_selection(buffer, false);
            None
        }
        KeyCode::Down if !has_shift => {
            cursor.move_down_with_selection(buffer, false);
            None
        }

        // Home/End with selection
        KeyCode::Home if has_shift => {
            cursor.move_to_line_start_with_selection(true);
            None
        }
        KeyCode::End if has_shift => {
            cursor.move_to_line_end_with_selection(buffer, true);
            None
        }
        KeyCode::Home if !has_shift => {
            cursor.move_to_line_start_with_selection(false);
            None
        }
        KeyCode::End if !has_shift => {
            cursor.move_to_line_end_with_selection(buffer, false);
            None
        }
        KeyCode::PageUp => Some(EditorCommand::PageUp),
        KeyCode::PageDown => Some(EditorCommand::PageDown),

        // Word deletion - Option/Alt + Backspace or Ctrl + Backspace
        KeyCode::Backspace if has_option || has_ctrl => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            } else {
                delete_word_backward(buffer, cursor);
            }
            Some(EditorCommand::Modified)
        }
        KeyCode::Backspace => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            } else {
                delete_char_backward(buffer, cursor);
            }
            Some(EditorCommand::Modified)
        }

        // Word deletion forward - Option/Alt + Delete or Ctrl + Delete
        KeyCode::Delete if has_option || has_ctrl => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            } else {
                delete_word_forward(buffer, cursor);
            }
            Some(EditorCommand::Modified)
        }
        KeyCode::Delete => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            } else {
                delete_char_forward(buffer, cursor);
            }
            Some(EditorCommand::Modified)
        }

        // Text insertion
        KeyCode::Enter => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            }
            insert_newline(buffer, cursor);
            Some(EditorCommand::Modified)
        }
        KeyCode::Tab if !has_ctrl => {
            // Let the app handle Tab for focus switching - only insert tab if not handled by app
            None
        }

        // Character insertion - ignore if Alt/Option is pressed (prevents 'b' from Alt+Arrow)
        KeyCode::Char(c) if !has_ctrl && !has_super && !has_option => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            }
            insert_char(buffer, cursor, c);
            Some(EditorCommand::Modified)
        }

        _ => None,
    }
}

fn insert_char(buffer: &mut RopeBuffer, cursor: &mut Cursor, ch: char) {
    let char_idx = cursor.to_char_index(buffer);
    buffer.insert_char(char_idx, ch);
    cursor.move_right(buffer);
}

#[allow(dead_code)]
fn insert_tab(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    let char_idx = cursor.to_char_index(buffer);
    buffer.insert(char_idx, "    ");
    for _ in 0..4 {
        cursor.move_right(buffer);
    }
}

fn insert_newline(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    let char_idx = cursor.to_char_index(buffer);
    buffer.insert_char(char_idx, '\n');
    cursor.move_right(buffer);
}

fn delete_char_backward(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    if cursor.position.line > 0 || cursor.position.column > 0 {
        cursor.move_left(buffer);
        let char_idx = cursor.to_char_index(buffer);
        if char_idx < buffer.len_chars() {
            buffer.remove(char_idx..char_idx + 1);
        }
    }
}

fn delete_char_forward(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    let char_idx = cursor.to_char_index(buffer);
    if char_idx < buffer.len_chars() {
        buffer.remove(char_idx..char_idx + 1);
    }
}

fn delete_word_backward(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    let start_idx = cursor.to_char_index(buffer);
    cursor.move_word_left(buffer);
    let end_idx = cursor.to_char_index(buffer);

    if start_idx > end_idx {
        buffer.remove(end_idx..start_idx);
    }
}

fn delete_word_forward(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    let start_idx = cursor.to_char_index(buffer);
    let original_pos = cursor.position;
    cursor.move_word_right(buffer);
    let end_idx = cursor.to_char_index(buffer);
    cursor.position = original_pos;

    if end_idx > start_idx {
        buffer.remove(start_idx..end_idx);
    }
}

fn delete_selection(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    if let Some((start, end)) = cursor.get_selection() {
        let start_idx = buffer.line_to_char(start.line)
            + start.column.min(buffer.get_line_text(start.line).len());
        let end_idx =
            buffer.line_to_char(end.line) + end.column.min(buffer.get_line_text(end.line).len());

        if end_idx > start_idx {
            buffer.remove(start_idx..end_idx);
            cursor.position = start;
        }
        cursor.clear_selection();
    }
}

fn get_clipboard() -> Arc<Mutex<String>> {
    CLIPBOARD
        .get_or_init(|| Arc::new(Mutex::new(String::new())))
        .clone()
}

fn copy_selection(buffer: &RopeBuffer, cursor: &Cursor) {
    if let Some((start, end)) = cursor.get_selection() {
        let start_idx = buffer.line_to_char(start.line)
            + start.column.min(buffer.get_line_text(start.line).len());
        let end_idx =
            buffer.line_to_char(end.line) + end.column.min(buffer.get_line_text(end.line).len());

        if end_idx > start_idx {
            let selected_text = buffer.slice(start_idx..end_idx).to_string();
            
            // Copy to internal clipboard
            if let Ok(mut clipboard) = get_clipboard().lock() {
                *clipboard = selected_text.clone();
            }
            
            // Also copy to system clipboard
            if let Ok(mut system_clipboard) = Clipboard::new() {
                let _ = system_clipboard.set_text(&selected_text);
            }
        }
    }
}

fn cut_selection(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    copy_selection(buffer, cursor);
    delete_selection(buffer, cursor);
}

fn cut_current_line(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    let current_line = cursor.position.line;
    
    // Get the entire line including the newline character
    let line_start_idx = buffer.line_to_char(current_line);
    let next_line_start = if current_line + 1 < buffer.len_lines() {
        buffer.line_to_char(current_line + 1)
    } else {
        // Last line - just go to end of buffer
        buffer.len_chars()
    };
    
    // Copy the line to clipboard
    if next_line_start > line_start_idx {
        let line_text = buffer.slice(line_start_idx..next_line_start).to_string();
        
        // Copy to internal clipboard
        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = line_text.clone();
        }
        
        // Also copy to system clipboard
        if let Ok(mut system_clipboard) = Clipboard::new() {
            let _ = system_clipboard.set_text(&line_text);
        }
        
        // Delete the line
        buffer.remove(line_start_idx..next_line_start);
        
        // Move cursor to the beginning of the line (which is now the next line)
        cursor.position.column = 0;
        // Adjust line position if we deleted the last line
        if cursor.position.line >= buffer.len_lines() && buffer.len_lines() > 0 {
            cursor.position.line = buffer.len_lines() - 1;
        }
    }
}

fn paste_from_clipboard(buffer: &mut RopeBuffer, cursor: &mut Cursor) {
    // Try system clipboard first
    let text_to_paste = if let Ok(mut system_clipboard) = Clipboard::new() {
        if let Ok(text) = system_clipboard.get_text() {
            // Update internal clipboard with system clipboard content
            if let Ok(mut clipboard) = get_clipboard().lock() {
                *clipboard = text.clone();
            }
            text
        } else {
            // Fall back to internal clipboard
            if let Ok(clipboard) = get_clipboard().lock() {
                clipboard.clone()
            } else {
                return;
            }
        }
    } else {
        // Fall back to internal clipboard
        if let Ok(clipboard) = get_clipboard().lock() {
            clipboard.clone()
        } else {
            return;
        }
    };

    if !text_to_paste.is_empty() {
        let char_idx = cursor.to_char_index(buffer);
        let initial_column = cursor.position.column;
        
        // Insert the text all at once - this is already efficient in ropey
        buffer.insert(char_idx, &text_to_paste);

        // Calculate new cursor position efficiently without iterating through characters
        let lines: Vec<&str> = text_to_paste.lines().collect();
        let num_new_lines = lines.len().saturating_sub(1);
        
        if num_new_lines > 0 {
            // Multi-line paste: cursor goes to the end of the last pasted line
            cursor.position.line += num_new_lines;
            // For multi-line paste, we need to account for text after cursor on original line
            // The last line length is where the cursor should be
            cursor.position.column = lines.last().unwrap_or(&"").len();
            
            // If we pasted in the middle of a line, the remaining text is now after our cursor
            // on the last line of the pasted content, so we don't need to adjust further
        } else {
            // Single line paste: just advance by the pasted text length
            cursor.position.column = initial_column + text_to_paste.len();
        }
        
        // Clear selection after paste
        cursor.clear_selection();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorCommand {
    Quit,
    Save,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    PageUp,
    PageDown,
    Modified,
    ToggleMenu,
    OpenFile,
    CurrentTab,
    Undo,
    Redo,
    TogglePreview,
    ToggleWordWrap,
    FocusTreeView,
    FocusEditor,
    Find,
    FindReplace,
}
