use std::io;
use std::{error::Error, fs::File, path::PathBuf};

use crate::{TodoApp, TodoItem, TodoList};
use chrono::Local;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(Debug, PartialEq)]
enum AppMode {
    Selection,
    Edit,
    Delete,
}

struct App {
    todo_list: TodoApp,
    selected_index: usize,
    mode: AppMode,
    edit_text: String,
    edit_cursor: usize, // Character position, not byte position
    should_quit: bool,
}

const CURSOR: char = '|';

impl App {
    fn new(todo_list: TodoApp) -> Self {
        App {
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            should_quit: false,
            todo_list,
        }
    }

    fn handle_key_event(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match self.mode {
            AppMode::Selection => self.handle_selection_mode_key(key)?,
            AppMode::Edit => self.handle_edit_mode_key(key)?,
            AppMode::Delete => self.handle_delete_mode_key(key)?,
        }
        Ok(())
    }

    fn handle_selection_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < self.todo_list.todo_list.items.len() {
                    self.selected_index += 1;
                }
            }
            KeyCode::Char('x') => {
                if !self.todo_list.todo_list.items.is_empty() {
                    self.todo_list.toggle_item_completed(self.selected_index)?;
                }
            }
            KeyCode::Char('i') => {
                self.todo_list.insert_new_item(self.selected_index)?;
                self.mode = AppMode::Edit;
            }
            KeyCode::Enter => {
                if self.selected_index < self.todo_list.todo_list.items.len() {
                    self.edit_text = self
                        .todo_list
                        .todo_list
                        .items
                        .get(self.selected_index)
                        .map(|i| i.text.clone())
                        .unwrap_or(String::new());

                    self.edit_cursor = self.edit_text.chars().count();
                    self.mode = AppMode::Edit;
                }
            }
            KeyCode::Tab => {
                self.todo_list.indent_item_right(self.selected_index)?;
            }
            KeyCode::BackTab => {
                self.todo_list.indent_item_left(self.selected_index)?;
            }
            KeyCode::Char('d') => {
                if !self.todo_list.todo_list.items.is_empty() {
                    self.mode = AppMode::Delete;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_edit_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Esc => {
                // Cancel edit mode
                if self.todo_list.todo_list.items[self.selected_index]
                    .text
                    .is_empty()
                {
                    // Remove the item if it was newly created and still empty
                    self.todo_list.delete_item(self.selected_index)?;
                    self.selected_index = self
                        .selected_index
                        .min(self.todo_list.todo_list.items.len());
                }
                self.mode = AppMode::Selection;
            }
            KeyCode::Enter => {
                // Confirm changes
                self.todo_list
                    .update_item_text(self.selected_index, self.edit_text.clone())?;
                self.mode = AppMode::Selection;
                self.edit_cursor = 0;
                self.edit_text.clear();
            }
            KeyCode::Left => {
                if self.edit_cursor > 0 {
                    self.edit_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.edit_cursor < self.edit_text.chars().count() {
                    self.edit_cursor += 1;
                }
            }
            KeyCode::Backspace => {
                self.edit_cursor = self.edit_cursor.saturating_sub(1);
                Self::remove_edit_text_char_at(&mut self.edit_text, self.edit_cursor);
            }
            KeyCode::Delete => {
                Self::remove_edit_text_char_at(&mut self.edit_text, self.edit_cursor);
            }
            KeyCode::Home => {
                self.edit_cursor = 0;
            }
            KeyCode::End => {
                self.edit_cursor = self.edit_text.chars().count();
            }
            KeyCode::Char(c) => {
                // Convert character position to byte position for insertion
                let byte_pos = self
                    .edit_text
                    .char_indices()
                    .nth(self.edit_cursor)
                    .map(|(pos, _)| pos)
                    .unwrap_or(self.edit_text.len());

                self.edit_text.insert(byte_pos, c);
                self.edit_cursor += 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn remove_edit_text_char_at(text: &mut String, char_index: usize) -> bool {
        // Find the byte positions of current and next character
        let char_indices: Vec<_> = text.char_indices().collect();
        if let Some(&(byte_pos, _)) = char_indices.get(char_index) {
            // Find the next character's byte position (or end of string)
            let next_byte_pos = char_indices
                .get(char_index + 1)
                .map(|(pos, _)| *pos)
                .unwrap_or(text.len());

            // Remove the character by removing the range
            text.drain(byte_pos..next_byte_pos).count() > 0
        } else {
            false
        }
    }
    fn handle_delete_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Char('y') => {
                // Confirm delete
                self.todo_list.delete_item(self.selected_index)?;
                self.mode = AppMode::Selection;
                if self.selected_index >= self.todo_list.todo_list.items.len() {
                    self.selected_index = self.selected_index.saturating_sub(1);
                }
            }
            KeyCode::Esc => {
                // Cancel delete
                self.mode = AppMode::Selection;
            }
            _ => {}
        }
        Ok(())
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }
}

// Helper function to wrap text based on available width
fn wrap_todo_item_text(
    item: &TodoItem,
    available_width: usize,
    is_selected: bool,
    edit_text: &str,
    edit_cursor: usize,
    is_editing: bool,
) -> Vec<(String, bool)> {
    let indent = "  ".repeat(item.indent_level);
    let checkbox = if item.completed { "[x]" } else { "[ ]" };
    let prefix = format!("{}* {} ", indent, checkbox);
    let prefix_len = prefix.len();

    let text = if is_editing && is_selected {
        let mut display_text = edit_text.to_string();
        // Insert cursor at character position, not byte position
        let byte_pos = edit_text
            .char_indices()
            .nth(edit_cursor)
            .map(|(pos, _)| pos)
            .unwrap_or(edit_text.len());
        display_text.insert(byte_pos, CURSOR);
        display_text
    } else {
        item.text.clone()
    };

    if available_width <= prefix_len {
        return vec![(format!("{}{}", prefix, text), true)];
    }

    let text_width = available_width - prefix_len;
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return vec![(prefix, true)];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if word.len() > text_width {
            // Handle very long words by breaking them
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }

            let mut remaining = word;
            while remaining.len() > text_width {
                let chunk = &remaining[..text_width];
                lines.push(chunk.to_string());
                remaining = &remaining[text_width..];
            }
            if !remaining.is_empty() {
                current_line = remaining.to_string();
            }
        } else if current_line.len() + word.len() + (if current_line.is_empty() { 0 } else { 1 })
            > text_width
        {
            // Word doesn't fit on current line
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                current_line = word.to_string();
            }
        } else {
            // Word fits on current line
            if current_line.is_empty() {
                current_line = word.to_string();
            } else {
                current_line.push(' ');
                current_line.push_str(word);
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        return vec![(prefix, true)];
    }

    // Build the result with proper formatting
    let mut result = Vec::new();
    let continuation_prefix = format!("{}   ", indent);

    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            result.push((format!("{}{}", prefix, line), true));
        } else {
            result.push((format!("{}{}", continuation_prefix, line), false));
        }
    }

    result
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(f.area());

    // Main todo list area
    let todo_area = chunks[0];
    let status_area = chunks[1];

    // Render todo list
    let title = format!("TODO {}", app.todo_list.todo_list.date.format("%Y-%m-%d"));

    // Calculate available width for text (accounting for borders and padding)
    let available_width = todo_area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding

    let mut display_items = Vec::new();
    let mut logical_to_display_map = Vec::new(); // Maps logical item index to display item indices

    if app.todo_list.todo_list.items.is_empty() {
        display_items.push(ListItem::new("No items"));
        logical_to_display_map.push(vec![0]);
    } else {
        for (logical_index, item) in app.todo_list.todo_list.items.iter().enumerate() {
            let is_selected = logical_index == app.selected_index;
            let is_editing = app.mode == AppMode::Edit && is_selected;
            let is_delete_mode = app.mode == AppMode::Delete && is_selected;

            let wrapped_lines = wrap_todo_item_text(
                item,
                available_width,
                is_selected,
                &app.edit_text,
                app.edit_cursor,
                is_editing,
            );

            let start_display_index = display_items.len();
            let mut display_indices = Vec::new();

            for (line_index, (line_text, is_main_line)) in wrapped_lines.iter().enumerate() {
                let style = if is_delete_mode && *is_main_line {
                    Style::default().bg(Color::Red).fg(Color::White)
                } else if is_selected && *is_main_line {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else if item.completed {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                display_items.push(ListItem::new(line_text.clone()).style(style));
                display_indices.push(start_display_index + line_index);
            }

            logical_to_display_map.push(display_indices);
        }

        // Add a virtual item for insertion past the last item
        if app.selected_index == app.todo_list.todo_list.items.len() {
            let style = Style::default().bg(Color::DarkGray).fg(Color::Yellow);
            display_items.push(ListItem::new("--- Insert new item here ---").style(style));
            logical_to_display_map.push(vec![display_items.len() - 1]);
        }
    }

    // Calculate which display item should be selected
    let selected_display_index = if app.selected_index < logical_to_display_map.len() {
        logical_to_display_map[app.selected_index].first().copied()
    } else {
        None
    };

    let todo_list = List::new(display_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut list_state = ListState::default();
    list_state.select(selected_display_index);

    f.render_stateful_widget(todo_list, todo_area, &mut list_state);

    // Status bar
    let status_text = match app.mode {
        AppMode::Selection => {
            if app.todo_list.todo_list.items.is_empty() {
                "Sel | i:Insert | q:Quit"
            } else {
                "Sel | ↑k:Up | ↓j:Down | x:Toggle | i:Insert | Enter:Edit | Tab:Indent | Shift+Tab:Unindent | d:Delete | q:Quit"
            }
        }
        AppMode::Edit => "Edit | Enter:Confirm | Esc:Cancel | ←→:Move cursor",
        AppMode::Delete => "Delete | y:Confirm Delete | Esc:Cancel",
    };

    let status_paragraph = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Blue).fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(status_paragraph, status_area);
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if let Err(err) = app.handle_key_event(key.code) {
                    eprintln!("Error handling key event: {}", err);
                }
            }
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

pub fn run_tui(todo_list: TodoApp) -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let app = App::new(todo_list);

    // Run the app
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use crossterm::event::KeyCode;

    use crate::{TodoApp, TodoList, tui::CURSOR};

    use super::{App, AppMode, TodoItem, wrap_todo_item_text};

    #[test]
    fn test_wrap_todo_item_text_editing() {
        let item = TodoItem::new("Original text".to_string(), false, 0);
        let edit_text = "Edited very long text that needs wrapping";
        let wrapped = wrap_todo_item_text(&item, 20, true, edit_text, 10, true);

        assert!(wrapped.len() > 1);
        assert!(wrapped[0].0.contains(CURSOR)); // Should contain cursor
        assert!(wrapped[0].0.starts_with("* [ ] Edited"));
    }

    #[test]
    fn test_wrap_todo_item_text_short() {
        let item = TodoItem::new("Short text".to_string(), false, 0);
        let wrapped = wrap_todo_item_text(&item, 50, false, "", 0, false);

        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].0, "* [ ] Short text");
        assert_eq!(wrapped[0].1, true);
    }

    #[test]
    fn test_wrap_todo_item_text_long() {
        let item = TodoItem::new(
            "This is a very long todo item that should wrap".to_string(),
            false,
            0,
        );
        let wrapped = wrap_todo_item_text(&item, 20, false, "", 0, false);

        assert!(wrapped.len() > 1);
        assert!(wrapped[0].0.starts_with("* [ ] This"));
        assert!(wrapped[1].0.starts_with("   ")); // continuation line should be indented
        assert_eq!(wrapped[0].1, true); // first line is main line
        assert_eq!(wrapped[1].1, false); // continuation line is not main line
    }

    #[test]
    fn test_wrap_todo_item_text_nested() {
        let item = TodoItem::new(
            "Long nested item text that should wrap properly".to_string(),
            true,
            1,
        );
        let wrapped = wrap_todo_item_text(&item, 25, false, "", 0, false);

        assert!(wrapped.len() > 1);
        assert!(wrapped[0].0.starts_with("  * [x] Long"));
        assert!(wrapped[1].0.starts_with("     ")); // continuation should align with text
    }

    #[test]
    fn test_delete_mode_transitions() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test item".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));

        // Test entering delete mode
        assert_eq!(app.mode, AppMode::Selection);
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        assert_eq!(app.mode, AppMode::Delete);

        // Test canceling delete
        app.handle_key_event(KeyCode::Esc).unwrap();
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.todo_list.items.len(), 1); // Item should still exist
    }

    #[test]
    fn test_delete_confirmation() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Item 1".to_string(), false, 0));
        todo_list
            .items
            .push(TodoItem::new("Item 2".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));

        // Enter delete mode and confirm delete
        assert_eq!(app.todo_list.todo_list.items.len(), 2);
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        assert_eq!(app.mode, AppMode::Delete);

        app.handle_key_event(KeyCode::Char('y')).unwrap();
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.todo_list.items.len(), 1); // One item should be deleted
        assert_eq!(app.todo_list.todo_list.items[0].text, "Item 2"); // Remaining item should be "Item 2"
    }

    #[test]
    fn test_delete_last_item_adjusts_selection() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Only item".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));

        // Delete the only item
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        app.handle_key_event(KeyCode::Char('y')).unwrap();

        assert_eq!(app.todo_list.todo_list.items.len(), 0);
        assert_eq!(app.selected_index, 0); // Should be 0 when no items
    }

    #[test]
    fn test_delete_adjusts_selection_when_deleting_last_item() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Item 1".to_string(), false, 0));
        todo_list
            .items
            .push(TodoItem::new("Item 2".to_string(), false, 0));
        todo_list
            .items
            .push(TodoItem::new("Item 3".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.selected_index = 2;

        // Delete the last item
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        app.handle_key_event(KeyCode::Char('y')).unwrap();

        assert_eq!(app.todo_list.todo_list.items.len(), 2);
        assert_eq!(app.selected_index, 1); // Should move to previous item
        assert_eq!(app.todo_list.todo_list.items[1].text, "Item 2");
    }

    #[test]
    fn test_edit_mode_enter_key_confirms_changes() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Original text".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));

        // Enter edit mode
        app.handle_key_event(KeyCode::Enter).unwrap();
        assert_eq!(app.mode, AppMode::Edit);

        // Simulate typing some text
        app.edit_text = "Modified text".to_string();
        app.edit_cursor = app.edit_text.chars().count();

        // Confirm with Enter key
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should return to selection mode and save changes
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.todo_list.items[0].text, "Modified text");
    }

    #[test]
    fn test_enter_key_at_virtual_insertion_point() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Existing item".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.selected_index = 1;

        // Try to enter edit mode from virtual insertion point
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should not enter edit mode (this might be the bug)
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.todo_list.items.len(), 1);
    }

    #[test]
    fn test_insert_edit_confirm_workflow() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Existing item".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.selected_index = 1;

        // Insert new item with 'i'
        app.handle_key_event(KeyCode::Char('i')).unwrap();
        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(app.todo_list.todo_list.items.len(), 2);
        assert_eq!(app.selected_index, 1);

        // Type some text
        app.handle_key_event(KeyCode::Char('N')).unwrap();
        app.handle_key_event(KeyCode::Char('e')).unwrap();
        app.handle_key_event(KeyCode::Char('w')).unwrap();
        assert_eq!(app.edit_text, "New");

        // Confirm with Enter - this is where the bug might be
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should return to selection mode and save the text
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.todo_list.items.len(), 2);
        assert_eq!(app.todo_list.todo_list.items[1].text, "New");
    }

    #[test]
    fn test_edit_mode_enter_key_bounds_check() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test item".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.selected_index = 5; // Invalid index - way past items.len()
        app.mode = AppMode::Edit;
        app.edit_text = "Modified text".to_string();
        app.edit_cursor = 13; // Character count, not byte count

        // Try to confirm changes with invalid index
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should exit edit mode but not save changes due to bounds check
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.todo_list.items[0].text, "Test item"); // Original text unchanged
    }

    #[test]
    fn test_unicode_character_handling() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.mode = AppMode::Edit;
        app.edit_text = "Hallo".to_string();
        app.edit_cursor = 5;

        // Insert German umlaut ü at the end
        app.handle_key_event(KeyCode::Char('ü')).unwrap();
        assert_eq!(app.edit_text, "Halloü");
        assert_eq!(app.edit_cursor, 6);

        // Move cursor to position 2 (between 'a' and 'l')
        app.edit_cursor = 2;

        // Insert another unicode character
        app.handle_key_event(KeyCode::Char('ö')).unwrap();
        assert_eq!(app.edit_text, "Haölloü");
        assert_eq!(app.edit_cursor, 3);
    }

    #[test]
    fn test_app_remove_edit_text_char_at() {
        let mut text = "Hällö Wörld".to_string();
        assert!(App::remove_edit_text_char_at(&mut text, 0));
        assert_eq!("ällö Wörld", text);

        let mut text = "Hällö Wörld".to_string();
        assert!(App::remove_edit_text_char_at(&mut text, 5));
        assert_eq!("HällöWörld", text);

        let mut text = "Hällö Wörld".to_string();
        let text_len = text.len();
        assert!(!App::remove_edit_text_char_at(&mut text, text_len));
        assert_eq!("Hällö Wörld", text);
    }

    #[test]
    fn test_unicode_backspace_and_delete() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.mode = AppMode::Edit;
        app.edit_text = "Hällö Wörld".to_string(); // Contains umlauts
        app.edit_cursor = 5; // After the ö in "Hällö"

        // Test backspace on unicode character (should remove 'ö')
        app.handle_key_event(KeyCode::Backspace).unwrap();
        assert_eq!(app.edit_text, "Häll Wörld");
        assert_eq!(app.edit_cursor, 4);

        // Move cursor to position after 'ö' in "Wörld" (character position 7)
        app.edit_cursor = 7; // After 'ö' in "Wörld"

        // Test delete on unicode character (should remove 'r')
        app.handle_key_event(KeyCode::Delete).unwrap();
        assert_eq!(app.edit_text, "Häll Wöld");
        assert_eq!(app.edit_cursor, 7);
    }

    #[test]
    fn test_unicode_cursor_movement() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test".to_string(), false, 0));

        let mut app = App::new(TodoApp::new(PathBuf::new(), todo_list));
        app.mode = AppMode::Edit;
        app.edit_text = "Ümlaut test".to_string(); // Starts with umlaut

        // Move right from start (should move past 'Ü')
        app.handle_key_event(KeyCode::Right).unwrap();
        assert_eq!(app.edit_cursor, 1);

        // Move to end
        app.handle_key_event(KeyCode::End).unwrap();
        assert_eq!(app.edit_cursor, app.edit_text.chars().count());

        // Move to home
        app.handle_key_event(KeyCode::Home).unwrap();
        assert_eq!(app.edit_cursor, 0);
    }

    #[test]
    fn test_unicode_display_with_cursor() {
        let item = TodoItem::new("Test".to_string(), false, 0);
        let edit_text = "Hallö";
        let edit_cursor = 4; // After 'l', before 'ö'

        let wrapped = wrap_todo_item_text(&item, 50, true, edit_text, edit_cursor, true);

        assert_eq!(wrapped.len(), 1);
        let term = format!("Hall{}ö", CURSOR);
        assert!(wrapped[0].0.contains(&term)); // Cursor should be positioned correctly
    }
}
