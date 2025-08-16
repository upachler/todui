use crate::{TodoItem, TodoList};

pub(super) const CURSOR: &str = "█";

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
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{error::Error, fs::File, io, path::PathBuf};

#[derive(Debug, PartialEq)]
pub enum AppMode {
    Selection,
    Edit,
    Delete,
}

pub struct App {
    pub todo_list: TodoList,
    pub selected_index: usize,
    pub mode: AppMode,
    pub edit_text: String,
    pub edit_cursor: usize, // Character position, not byte position
    pub config_dir: PathBuf,
    pub _lock_file: File,
    pub should_quit: bool,
}

impl App {
    pub fn new(config_dir: PathBuf, lock_file: File, todo_list: TodoList) -> Self {
        App {
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir,
            _lock_file: lock_file,
            should_quit: false,
            todo_list,
        }
    }

    fn save_todo_list(&mut self) -> Result<(), Box<dyn Error>> {
        // Update date to current date if needed
        let current_date = chrono::Local::now().date_naive();
        if self.todo_list.date != current_date {
            self.todo_list.date = current_date;
        }

        // Save to file
        let filename = self.config_dir.join(self.todo_list.filename());
        std::fs::write(filename, self.todo_list.to_markdown())?;
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match self.mode {
            AppMode::Selection => self.handle_selection_mode_key(key),
            AppMode::Edit => self.handle_edit_mode_key(key),
            AppMode::Delete => self.handle_delete_mode_key(key),
        }
    }

    fn handle_selection_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Allow selecting one position past the last item for insertion
                if self.selected_index < self.todo_list.items.len() {
                    self.selected_index += 1;
                }
            }
            KeyCode::Char('x') => {
                if self.selected_index < self.todo_list.items.len() {
                    self.todo_list.items[self.selected_index].completed =
                        !self.todo_list.items[self.selected_index].completed;
                    self.save_todo_list()?;
                }
            }
            KeyCode::Char('i') => {
                // Insert new item with inherited indentation
                let inherit_indent = if self.selected_index > 0 {
                    self.todo_list.items[self.selected_index - 1].indent_level
                } else if self.selected_index < self.todo_list.items.len() {
                    self.todo_list.items[self.selected_index].indent_level
                } else {
                    0
                };

                let new_item = TodoItem::new(String::new(), false, inherit_indent);
                self.todo_list.items.insert(self.selected_index, new_item);
                self.mode = AppMode::Edit;
                self.edit_text = String::new();
                self.edit_cursor = 0;
            }
            KeyCode::Enter => {
                if self.selected_index < self.todo_list.items.len() {
                    // Edit existing item
                    self.mode = AppMode::Edit;
                    self.edit_text = self.todo_list.items[self.selected_index].text.clone();
                    self.edit_cursor = self.edit_text.chars().count();
                } else if self.selected_index == self.todo_list.items.len() {
                    // Insert new item at the end
                    let inherit_indent = if let Some(last_item) = self.todo_list.items.last() {
                        last_item.indent_level
                    } else {
                        0
                    };
                    let new_item = TodoItem::new(String::new(), false, inherit_indent);
                    self.todo_list.items.push(new_item);
                    self.mode = AppMode::Edit;
                    self.edit_text = String::new();
                    self.edit_cursor = 0;
                }
            }
            KeyCode::Tab => {
                if self.selected_index < self.todo_list.items.len() {
                    self.todo_list.items[self.selected_index].indent_level += 1;
                    self.save_todo_list()?;
                }
            }
            KeyCode::BackTab => {
                if self.selected_index < self.todo_list.items.len()
                    && self.todo_list.items[self.selected_index].indent_level > 0
                {
                    self.todo_list.items[self.selected_index].indent_level -= 1;
                    self.save_todo_list()?;
                }
            }
            KeyCode::Char('d') => {
                if self.selected_index < self.todo_list.items.len() {
                    self.mode = AppMode::Delete;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_edit_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Enter => {
                // Confirm edit
                if self.selected_index < self.todo_list.items.len() {
                    self.todo_list.items[self.selected_index].text = self.edit_text.clone();
                }
                self.save_todo_list()?;
                self.mode = AppMode::Selection;
            }
            KeyCode::Esc => {
                // Cancel edit - if we were inserting a new item, remove it
                if self.selected_index < self.todo_list.items.len()
                    && self.todo_list.items[self.selected_index].text.is_empty()
                {
                    self.todo_list.items.remove(self.selected_index);
                    // Adjust selected index if we removed the last item
                    if self.selected_index >= self.todo_list.items.len() && self.selected_index > 0
                    {
                        self.selected_index -= 1;
                    }
                }
                self.mode = AppMode::Selection;
            }
            KeyCode::Left => {
                if self.edit_cursor > 0 {
                    // Move cursor left by one character
                    self.edit_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = self.edit_text.chars().count();
                if self.edit_cursor < char_count {
                    // Move cursor right by one character
                    self.edit_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.edit_cursor = 0;
            }
            KeyCode::End => {
                self.edit_cursor = self.edit_text.chars().count();
            }
            KeyCode::Backspace => {
                if self.edit_cursor > 0 {
                    // Convert to chars, remove character, convert back to string
                    let mut chars: Vec<char> = self.edit_text.chars().collect();
                    chars.remove(self.edit_cursor - 1);
                    self.edit_text = chars.into_iter().collect();
                    self.edit_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                let char_count = self.edit_text.chars().count();
                if self.edit_cursor < char_count {
                    // Convert to chars, remove character, convert back to string
                    let mut chars: Vec<char> = self.edit_text.chars().collect();
                    chars.remove(self.edit_cursor);
                    self.edit_text = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                // Insert character at cursor position
                let mut chars: Vec<char> = self.edit_text.chars().collect();
                chars.insert(self.edit_cursor, c);
                self.edit_text = chars.into_iter().collect();
                self.edit_cursor += 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_delete_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Char('y') => {
                // Confirm deletion
                if self.selected_index < self.todo_list.items.len() {
                    self.todo_list.items.remove(self.selected_index);

                    // Adjust selection after deletion
                    if self.selected_index >= self.todo_list.items.len() && self.selected_index > 0
                    {
                        self.selected_index = self.todo_list.items.len().saturating_sub(1);
                        if self.todo_list.items.is_empty() {
                            self.selected_index = 0;
                        }
                    }

                    self.save_todo_list()?;
                }
                self.mode = AppMode::Selection;
            }
            KeyCode::Esc => {
                // Cancel deletion
                self.mode = AppMode::Selection;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.save_todo_list();
    }
}

pub fn wrap_todo_item_text(
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
    let prefix_width = prefix.chars().count();

    if available_width <= prefix_width {
        return vec![(prefix, false)];
    }

    let content_width = available_width - prefix_width;
    let text_to_display = if is_editing { edit_text } else { &item.text };

    if text_to_display.is_empty() {
        let display_text = if is_editing {
            // Show cursor in editing mode even with empty text
            let cursor_indicator = if is_selected { CURSOR } else { "" };
            format!("{}{}", prefix, cursor_indicator)
        } else {
            prefix
        };
        return vec![(display_text, true)];
    }

    let words: Vec<&str> = text_to_display.split_whitespace().collect();
    if words.is_empty() {
        return vec![(prefix, true)];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    let mut is_first_line = true;

    for (word_idx, word) in words.iter().enumerate() {
        let word_width = word.chars().count();
        let space_needed = if current_width == 0 { 0 } else { 1 }; // Space before word

        if current_width + space_needed + word_width <= content_width {
            // Word fits on current line
            if current_width > 0 {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        } else {
            // Need to wrap to next line
            if !current_line.is_empty() {
                let full_line = if is_first_line {
                    format!("{}{}", prefix, current_line)
                } else {
                    format!("{}  {}", indent, current_line)
                };
                lines.push((full_line, is_first_line));
                is_first_line = false;
                current_line.clear();
                current_width = 0;
            }

            // Start new line with current word
            current_line.push_str(word);
            current_width = word_width;
        }

        // Handle cursor display for editing mode
        if is_editing && is_selected {
            let chars_processed = words[..=word_idx].join(" ").chars().count();
            if edit_cursor <= chars_processed {
                // Cursor is within or at the end of this word/line
                if !current_line.is_empty() {
                    let line_prefix = if is_first_line {
                        &prefix
                    } else {
                        &format!("{}  ", indent)
                    };
                    let cursor_pos_in_line = if edit_cursor == 0 {
                        0
                    } else {
                        edit_cursor.min(current_line.chars().count())
                    };

                    let (before_cursor, after_cursor) =
                        if cursor_pos_in_line <= current_line.chars().count() {
                            let chars: Vec<char> = current_line.chars().collect();
                            let before: String = chars[..cursor_pos_in_line.min(chars.len())]
                                .iter()
                                .collect();
                            let after: String = chars[cursor_pos_in_line.min(chars.len())..]
                                .iter()
                                .collect();
                            (before, after)
                        } else {
                            (current_line.clone(), String::new())
                        };

                    let line_with_cursor =
                        format!("{line_prefix}{before_cursor}{}{after_cursor}", CURSOR);
                    lines.push((line_with_cursor, is_first_line));
                    is_first_line = false;
                    current_line.clear();
                    current_width = 0;
                    break;
                }
            }
        }
    }

    // Add remaining text if any
    if !current_line.is_empty() {
        let full_line = if is_first_line {
            let line_with_content = format!("{}{}", prefix, current_line);
            if is_editing && is_selected && edit_cursor >= text_to_display.chars().count() {
                format!("{line_with_content}{}", CURSOR)
            } else {
                line_with_content
            }
        } else {
            format!("{}  {}", indent, current_line)
        };
        lines.push((full_line, is_first_line));
    }

    if lines.is_empty() {
        lines.push((prefix, true));
    }

    lines
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
    let title = format!("TODO {}", app.todo_list.date.format("%Y-%m-%d"));
    let available_width = todo_area.width as usize;
    let content_width = available_width.saturating_sub(2); // Account for borders

    let mut list_items = Vec::new();
    let mut display_line_to_item_index = Vec::new();

    for (item_index, item) in app.todo_list.items.iter().enumerate() {
        let is_selected = item_index == app.selected_index;
        let is_editing = app.mode == AppMode::Edit && is_selected;

        let wrapped_lines = wrap_todo_item_text(
            item,
            content_width,
            is_selected,
            &app.edit_text,
            app.edit_cursor,
            is_editing,
        );

        for (line_index, (line_text, _)) in wrapped_lines.iter().enumerate() {
            let style = if is_selected && line_index == 0 {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else if item.completed {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            list_items.push(ListItem::new(line_text.clone()).style(style));
            display_line_to_item_index.push(item_index);
        }
    }

    // Add virtual insertion point
    if app.selected_index >= app.todo_list.items.len() {
        let style = Style::default().bg(Color::Blue).fg(Color::White);
        list_items.push(ListItem::new("+ Add new item").style(style));
    }

    let list = List::new(list_items).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, todo_area);

    // Render status bar
    let status_text = match app.mode {
        AppMode::Selection => {
            "q: Quit | ↑/k: Up | ↓/j: Down | x: Toggle | i: Insert | Enter: Edit | Tab: Indent | Shift+Tab: Unindent | d: Delete"
        }
        AppMode::Edit => {
            "Enter: Confirm | Esc: Cancel | ←/→: Move cursor | Home/End: Jump | Backspace/Delete: Remove"
        }
        AppMode::Delete => "y: Confirm deletion | Esc: Cancel",
    };

    let status = Paragraph::new(status_text).block(Block::default().borders(Borders::ALL));

    f.render_widget(status, status_area);
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

pub fn run_tui(
    config_dir: PathBuf,
    lock_file: File,
    todo_list: TodoList,
) -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let app = App::new(config_dir, lock_file, todo_list);

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
