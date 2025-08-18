use chrono::{Local, NaiveDate};
use clap::Parser;
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
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    process,
};

slint::include_modules!();

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start with graphical user interface
    #[arg(long)]
    gui: bool,
}

#[derive(Debug, Clone)]
struct TodoItem {
    text: String,
    completed: bool,
    indent_level: usize,
}

impl TodoItem {
    fn new(text: String, completed: bool, indent_level: usize) -> Self {
        Self {
            text,
            completed,
            indent_level,
        }
    }

    fn to_markdown_line(&self) -> String {
        let indent = "  ".repeat(self.indent_level);
        let checkbox = if self.completed { "[x]" } else { "[ ]" };
        format!("{}* {} {}", indent, checkbox, self.text)
    }
}

#[derive(Debug)]
struct TodoList {
    date: NaiveDate,
    items: Vec<TodoItem>,
}

impl TodoList {
    fn new(date: NaiveDate) -> Self {
        Self {
            date,
            items: Vec::new(),
        }
    }

    fn from_markdown(content: &str) -> Result<Self, Box<dyn Error>> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Err("Empty todo file".into());
        }

        // Parse the header to get the date
        let header = lines[0];
        if !header.starts_with("# TODO ") {
            return Err("Invalid header format".into());
        }

        let date_str = header.strip_prefix("# TODO ").unwrap();
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| "Invalid date format in header")?;

        let mut todo_list = TodoList::new(date);

        // Parse todo items starting from line 2 (skip header and empty line)
        for line in lines.iter().skip(2) {
            if line.trim().is_empty() {
                continue;
            }

            let trimmed = line.trim_start();
            let indent_level = (line.len() - trimmed.len()) / 2;

            if !trimmed.starts_with("* ") {
                continue;
            }

            let content = trimmed.strip_prefix("* ").unwrap();

            let (completed, text) = if content.starts_with("[x] ") {
                (true, content.strip_prefix("[x] ").unwrap().to_string())
            } else if content.starts_with("[ ] ") {
                (false, content.strip_prefix("[ ] ").unwrap().to_string())
            } else {
                (false, content.to_string())
            };

            todo_list
                .items
                .push(TodoItem::new(text, completed, indent_level));
        }

        Ok(todo_list)
    }

    fn to_markdown(&self) -> String {
        let mut content = format!("# TODO {}\n\n", self.date.format("%Y-%m-%d"));

        for item in &self.items {
            content.push_str(&item.to_markdown_line());
            content.push('\n');
        }

        content
    }

    fn filename(&self) -> String {
        format!("TODO-{}.md", self.date.format("%Y-%m-%d"))
    }
}

#[derive(Debug, PartialEq)]
enum AppMode {
    Selection,
    Edit,
    Delete,
}

struct App {
    todo_list: TodoList,
    selected_index: usize,
    mode: AppMode,
    edit_text: String,
    edit_cursor: usize, // Character position, not byte position
    config_dir: PathBuf,
    _lock_file: File,
    should_quit: bool,
}

const CURSOR: char = '|';

impl App {
    fn new(config_dir: PathBuf, lock_file: File, todo_list: TodoList) -> Self {
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
        let current_date = Local::now().date_naive();
        if self.todo_list.date != current_date {
            self.todo_list.date = current_date;
        }

        let file_path = self.config_dir.join(self.todo_list.filename());
        let content = self.todo_list.to_markdown();
        fs::write(file_path, content)?;
        Ok(())
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
                if !self.todo_list.items.is_empty() && self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                {
                    self.selected_index += 1;
                }
            }
            KeyCode::Char('x') => {
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                {
                    self.todo_list.items[self.selected_index].completed =
                        !self.todo_list.items[self.selected_index].completed;
                    self.save_todo_list()?;
                }
            }
            KeyCode::Char('i') => {
                // Determine indentation level for new item
                let indent_level = if self.todo_list.items.is_empty() {
                    0
                } else if self.selected_index == 0 {
                    0
                } else {
                    let prev_index = (self.selected_index - 1).min(self.todo_list.items.len() - 1);
                    self.todo_list.items[prev_index].indent_level
                };

                let new_item = TodoItem::new(String::new(), false, indent_level);
                let insert_pos = if self.todo_list.items.is_empty() {
                    0
                } else {
                    self.selected_index.min(self.todo_list.items.len())
                };
                self.todo_list.items.insert(insert_pos, new_item);
                self.selected_index = insert_pos;
                self.edit_text = String::new();
                self.edit_cursor = 0;
                self.mode = AppMode::Edit;
            }
            KeyCode::Enter => {
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                {
                    self.edit_text = self.todo_list.items[self.selected_index].text.clone();
                    self.edit_cursor = self.edit_text.chars().count();
                    self.mode = AppMode::Edit;
                }
            }
            KeyCode::Tab => {
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                {
                    self.todo_list.items[self.selected_index].indent_level += 1;
                    self.save_todo_list()?;
                }
            }
            KeyCode::BackTab => {
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                    && self.todo_list.items[self.selected_index].indent_level > 0
                {
                    self.todo_list.items[self.selected_index].indent_level -= 1;
                    self.save_todo_list()?;
                }
            }
            KeyCode::Char('d') => {
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                {
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
                if self.todo_list.items[self.selected_index].text.is_empty() {
                    // Remove the item if it was newly created and still empty
                    self.todo_list.items.remove(self.selected_index);
                    if self.selected_index > 0 && self.selected_index >= self.todo_list.items.len()
                    {
                        self.selected_index -= 1;
                    }
                }
                self.mode = AppMode::Selection;
            }
            KeyCode::Enter => {
                // Confirm changes
                if self.selected_index < self.todo_list.items.len() {
                    self.todo_list.items[self.selected_index].text = self.edit_text.clone();
                    self.save_todo_list()?;
                }
                self.mode = AppMode::Selection;
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
                if self.edit_cursor > 0 {
                    // Find the byte position of the character before cursor
                    let char_indices: Vec<_> = self.edit_text.char_indices().collect();
                    if let Some(&(byte_pos, _)) = char_indices.get(self.edit_cursor - 1) {
                        // Find the next character's byte position (or end of string)
                        let next_byte_pos = char_indices
                            .get(self.edit_cursor)
                            .map(|(pos, _)| *pos)
                            .unwrap_or(self.edit_text.len());

                        // Remove the character by removing the range
                        self.edit_text.drain(byte_pos..next_byte_pos);
                        self.edit_cursor -= 1;
                    }
                }
            }
            KeyCode::Delete => {
                if self.edit_cursor < self.edit_text.chars().count() {
                    // Find the byte positions of current and next character
                    let char_indices: Vec<_> = self.edit_text.char_indices().collect();
                    if let Some(&(byte_pos, _)) = char_indices.get(self.edit_cursor) {
                        // Find the next character's byte position (or end of string)
                        let next_byte_pos = char_indices
                            .get(self.edit_cursor + 1)
                            .map(|(pos, _)| *pos)
                            .unwrap_or(self.edit_text.len());

                        // Remove the character by removing the range
                        self.edit_text.drain(byte_pos..next_byte_pos);
                    }
                }
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

    fn handle_delete_mode_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Char('y') => {
                // Confirm delete
                if !self.todo_list.items.is_empty()
                    && self.selected_index < self.todo_list.items.len()
                {
                    self.todo_list.items.remove(self.selected_index);

                    // Adjust selected index if necessary
                    if self.selected_index >= self.todo_list.items.len()
                        && !self.todo_list.items.is_empty()
                    {
                        self.selected_index = self.todo_list.items.len() - 1;
                    }

                    self.save_todo_list()?;
                }
                self.mode = AppMode::Selection;
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

impl Drop for App {
    fn drop(&mut self) {
        // Save todo list on exit
        let _ = self.save_todo_list();

        // Remove lock file
        let lock_path = self.config_dir.join("lockfile");
        let _ = fs::remove_file(lock_path);
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
    let title = format!("TODO {}", app.todo_list.date.format("%Y-%m-%d"));

    // Calculate available width for text (accounting for borders and padding)
    let available_width = todo_area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding

    let mut display_items = Vec::new();
    let mut logical_to_display_map = Vec::new(); // Maps logical item index to display item indices

    if app.todo_list.items.is_empty() {
        display_items.push(ListItem::new("No items"));
        logical_to_display_map.push(vec![0]);
    } else {
        for (logical_index, item) in app.todo_list.items.iter().enumerate() {
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
        if app.selected_index == app.todo_list.items.len() {
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
            if app.todo_list.items.is_empty() {
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

// Shared core for both TUI and GUI
struct TodoApp {
    todo_list: TodoList,
    config_dir: PathBuf,
    _lock_file: File,
}

impl TodoApp {
    fn new(config_dir: PathBuf, lock_file: File, todo_list: TodoList) -> Self {
        TodoApp {
            todo_list,
            config_dir,
            _lock_file: lock_file,
        }
    }

    fn save_todo_list(&mut self) -> Result<(), Box<dyn Error>> {
        // Update date to current date if needed
        let current_date = Local::now().date_naive();
        if self.todo_list.date != current_date {
            self.todo_list.date = current_date;
        }

        // Save to file
        let filename = self.config_dir.join(self.todo_list.filename());
        fs::write(filename, self.todo_list.to_markdown())?;
        Ok(())
    }

    fn toggle_item_completed(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items[index].completed = !self.todo_list.items[index].completed;
            self.save_todo_list()?;
        }
        Ok(())
    }

    fn update_item_text(&mut self, index: usize, text: String) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items[index].text = text;
            self.save_todo_list()?;
        }
        Ok(())
    }

    fn indent_item_left(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() && self.todo_list.items[index].indent_level > 0 {
            self.todo_list.items[index].indent_level -= 1;
            self.save_todo_list()?;
        }
        Ok(())
    }

    fn indent_item_right(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items[index].indent_level += 1;
            self.save_todo_list()?;
        }
        Ok(())
    }

    fn delete_item(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items.remove(index);
            self.save_todo_list()?;
        }
        Ok(())
    }

    fn add_new_item(&mut self) -> Result<(), Box<dyn Error>> {
        let indent_level = if let Some(last_item) = self.todo_list.items.last() {
            last_item.indent_level
        } else {
            0
        };

        let new_item = TodoItem::new(String::new(), false, indent_level);
        self.todo_list.items.push(new_item);
        self.save_todo_list()?;
        Ok(())
    }
}

impl Drop for TodoApp {
    fn drop(&mut self) {
        let _ = self.save_todo_list();
    }
}

fn run_gui(todo_app: TodoApp) -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    // Set initial title
    let title = format!("TODO {}", todo_app.todo_list.date.format("%Y-%m-%d"));
    ui.set_window_title(title.into());

    // Convert TodoItems to TodoItemData for Slint
    let convert_items = |items: &[TodoItem]| -> Vec<TodoItemData> {
        items
            .iter()
            .map(|item| TodoItemData {
                text: item.text.clone().into(),
                completed: item.completed,
                indent_level: item.indent_level as i32,
            })
            .collect()
    };

    // Set initial items
    let initial_items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(slint::VecModel::from(
        convert_items(&todo_app.todo_list.items),
    ));
    ui.set_todo_items(initial_items.clone());

    // Setup callbacks
    let ui_weak = ui.as_weak();
    let todo_app_rc = std::rc::Rc::new(std::cell::RefCell::new(todo_app));

    // Toggle completed callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_toggle_item_completed(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.toggle_item_completed(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Text changed callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_item_text_changed(move |index, text| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.update_item_text(index as usize, text.to_string());
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Indent left callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_indent_item_left(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.indent_item_left(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Indent right callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_indent_item_right(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.indent_item_right(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Delete item callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_delete_item(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.delete_item(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Add new item callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_add_new_item(move || {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.add_new_item();
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    ui.run()?;
    Ok(())
}

fn run_tui(
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

fn get_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
    Ok(home_dir.join(".todui"))
}

fn create_lock_file(config_dir: &PathBuf) -> Result<File, Box<dyn Error>> {
    let lock_path = config_dir.join("lockfile");

    if lock_path.exists() {
        return Err(format!(
            "Another instance of todui appears to be running. Lock file exists at: {}",
            lock_path.display()
        )
        .into());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)?;

    let pid = process::id();
    writeln!(file, "{}", pid)?;
    file.flush()?;

    Ok(file)
}

fn load_or_create_todo_list(
    config_dir: &PathBuf,
    target_date: NaiveDate,
) -> Result<TodoList, Box<dyn Error>> {
    // Find the newest todo file that's not in the future
    let mut newest_file: Option<(NaiveDate, PathBuf)> = None;

    if let Ok(entries) = fs::read_dir(config_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("TODO-") && file_name.ends_with(".md") {
                        let date_part = file_name
                            .strip_prefix("TODO-")
                            .unwrap()
                            .strip_suffix(".md")
                            .unwrap();

                        if let Ok(file_date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                            if file_date <= target_date {
                                if newest_file.is_none()
                                    || file_date > newest_file.as_ref().unwrap().0
                                {
                                    newest_file = Some((file_date, path));
                                }
                            } else {
                                eprintln!(
                                    "Warning: Found todo file with future date: {}",
                                    file_name
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((file_date, path)) = newest_file {
        let content = fs::read_to_string(&path)?;
        let mut todo_list = TodoList::from_markdown(&content)?;
        // Update the date to current date if it's different
        if file_date != target_date {
            todo_list.date = target_date;
        }
        Ok(todo_list)
    } else {
        // Create new todo list for today
        Ok(TodoList::new(target_date))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Common initialization
    let config_dir = get_config_dir()?;

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    // Create and hold lock file
    let lock_file = create_lock_file(&config_dir)?;

    // Load or create today's todo list
    let today = Local::now().date_naive();
    let todo_list = load_or_create_todo_list(&config_dir, today)?;

    if args.gui {
        let todo_app = TodoApp::new(config_dir, lock_file, todo_list);
        run_gui(todo_app)
    } else {
        run_tui(config_dir, lock_file, todo_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, AppMode, wrap_todo_item_text};
    use chrono::NaiveDate;

    #[test]
    fn test_empty_todo_list_to_markdown() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let todo_list = TodoList::new(date);
        let markdown = todo_list.to_markdown();
        assert_eq!(markdown, "# TODO 2025-08-14\n\n");
    }

    #[test]
    fn test_todo_list_with_items_to_markdown() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);

        todo_list
            .items
            .push(TodoItem::new("take out trash".to_string(), true, 0));
        todo_list
            .items
            .push(TodoItem::new("shop groceries".to_string(), false, 0));
        todo_list
            .items
            .push(TodoItem::new("Apples".to_string(), true, 1));

        let markdown = todo_list.to_markdown();
        let expected =
            "# TODO 2025-08-14\n\n* [x] take out trash\n* [ ] shop groceries\n  * [x] Apples\n";
        assert_eq!(markdown, expected);
    }

    #[test]
    fn test_parse_markdown_empty() {
        let content = "# TODO 2025-08-14\n\n";
        let todo_list = TodoList::from_markdown(content).unwrap();

        assert_eq!(
            todo_list.date,
            NaiveDate::from_ymd_opt(2025, 8, 14).unwrap()
        );
        assert_eq!(todo_list.items.len(), 0);
    }

    #[test]
    fn test_parse_markdown_with_items() {
        let content = "# TODO 2025-08-14\n\n* [x] take out trash\n* [ ] shop groceries\n  * [x] Apples\n  * [ ] cheese\n";
        let todo_list = TodoList::from_markdown(content).unwrap();

        assert_eq!(
            todo_list.date,
            NaiveDate::from_ymd_opt(2025, 8, 14).unwrap()
        );
        assert_eq!(todo_list.items.len(), 4);

        assert_eq!(todo_list.items[0].text, "take out trash");
        assert_eq!(todo_list.items[0].completed, true);
        assert_eq!(todo_list.items[0].indent_level, 0);

        assert_eq!(todo_list.items[1].text, "shop groceries");
        assert_eq!(todo_list.items[1].completed, false);
        assert_eq!(todo_list.items[1].indent_level, 0);

        assert_eq!(todo_list.items[2].text, "Apples");
        assert_eq!(todo_list.items[2].completed, true);
        assert_eq!(todo_list.items[2].indent_level, 1);

        assert_eq!(todo_list.items[3].text, "cheese");
        assert_eq!(todo_list.items[3].completed, false);
        assert_eq!(todo_list.items[3].indent_level, 1);
    }

    #[test]
    fn test_todo_item_to_markdown_line() {
        let item1 = TodoItem::new("test item".to_string(), false, 0);
        assert_eq!(item1.to_markdown_line(), "* [ ] test item");

        let item2 = TodoItem::new("nested item".to_string(), true, 2);
        assert_eq!(item2.to_markdown_line(), "    * [x] nested item");
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
    fn test_wrap_todo_item_text_editing() {
        let item = TodoItem::new("Original text".to_string(), false, 0);
        let edit_text = "Edited very long text that needs wrapping";
        let wrapped = wrap_todo_item_text(&item, 20, true, edit_text, 10, true);

        assert!(wrapped.len() > 1);
        assert!(wrapped[0].0.contains(CURSOR)); // Should contain cursor
        assert!(wrapped[0].0.starts_with("* [ ] Edited"));
    }

    #[test]
    fn test_todo_item_indentation() {
        let mut item = TodoItem::new("test item".to_string(), false, 0);
        assert_eq!(item.indent_level, 0);

        // Test increasing indentation
        item.indent_level += 1;
        assert_eq!(item.indent_level, 1);
        assert_eq!(item.to_markdown_line(), "  * [ ] test item");

        item.indent_level += 1;
        assert_eq!(item.indent_level, 2);
        assert_eq!(item.to_markdown_line(), "    * [ ] test item");

        // Test decreasing indentation
        item.indent_level -= 1;
        assert_eq!(item.indent_level, 1);
        assert_eq!(item.to_markdown_line(), "  * [ ] test item");
    }

    #[test]
    fn test_inherit_indentation_from_previous_item() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);

        // Add first item at level 0
        todo_list
            .items
            .push(TodoItem::new("First item".to_string(), false, 0));

        // Add second item at level 1
        todo_list
            .items
            .push(TodoItem::new("Second item".to_string(), false, 1));

        // Add third item at level 2
        todo_list
            .items
            .push(TodoItem::new("Third item".to_string(), false, 2));

        // Test that new items inherit indentation
        assert_eq!(todo_list.items[0].indent_level, 0);
        assert_eq!(todo_list.items[1].indent_level, 1);
        assert_eq!(todo_list.items[2].indent_level, 2);

        let markdown = todo_list.to_markdown();
        assert!(markdown.contains("* [ ] First item"));
        assert!(markdown.contains("  * [ ] Second item"));
        assert!(markdown.contains("    * [ ] Third item"));
    }

    #[test]
    fn test_delete_mode_transitions() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test item".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Test entering delete mode
        assert_eq!(app.mode, AppMode::Selection);
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        assert_eq!(app.mode, AppMode::Delete);

        // Test canceling delete
        app.handle_key_event(KeyCode::Esc).unwrap();
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.items.len(), 1); // Item should still exist
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

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Enter delete mode and confirm delete
        assert_eq!(app.todo_list.items.len(), 2);
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        assert_eq!(app.mode, AppMode::Delete);

        app.handle_key_event(KeyCode::Char('y')).unwrap();
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.items.len(), 1); // One item should be deleted
        assert_eq!(app.todo_list.items[0].text, "Item 2"); // Remaining item should be "Item 2"
    }

    #[test]
    fn test_delete_last_item_adjusts_selection() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Only item".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Delete the only item
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        app.handle_key_event(KeyCode::Char('y')).unwrap();

        assert_eq!(app.todo_list.items.len(), 0);
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

        let mut app = App {
            todo_list,
            selected_index: 2, // Select the last item
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Delete the last item
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        app.handle_key_event(KeyCode::Char('y')).unwrap();

        assert_eq!(app.todo_list.items.len(), 2);
        assert_eq!(app.selected_index, 1); // Should move to previous item
        assert_eq!(app.todo_list.items[1].text, "Item 2");
    }

    #[test]
    fn test_edit_mode_enter_key_confirms_changes() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Original text".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

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
        assert_eq!(app.todo_list.items[0].text, "Modified text");
    }

    #[test]
    fn test_enter_key_at_virtual_insertion_point() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Existing item".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 1, // At virtual insertion point (past last item)
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Try to enter edit mode from virtual insertion point
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should not enter edit mode (this might be the bug)
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.items.len(), 1);
    }

    #[test]
    fn test_insert_edit_confirm_workflow() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Existing item".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 1, // At virtual insertion point (past last item)
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Insert new item with 'i'
        app.handle_key_event(KeyCode::Char('i')).unwrap();
        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(app.todo_list.items.len(), 2);
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
        assert_eq!(app.todo_list.items.len(), 2);
        assert_eq!(app.todo_list.items[1].text, "New");
    }

    #[test]
    fn test_edit_mode_enter_key_bounds_check() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test item".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 5, // Invalid index - way past items.len()
            mode: AppMode::Edit,
            edit_text: "Modified text".to_string(),
            edit_cursor: 13, // Character count, not byte count
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

        // Try to confirm changes with invalid index
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should exit edit mode but not save changes due to bounds check
        assert_eq!(app.mode, AppMode::Selection);
        assert_eq!(app.todo_list.items[0].text, "Test item"); // Original text unchanged
    }

    #[test]
    fn test_unicode_character_handling() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Edit,
            edit_text: "Hallo".to_string(),
            edit_cursor: 5,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

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
    fn test_unicode_backspace_and_delete() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut todo_list = TodoList::new(date);
        todo_list
            .items
            .push(TodoItem::new("Test".to_string(), false, 0));

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Edit,
            edit_text: "Hällö Wörld".to_string(), // Contains umlauts
            edit_cursor: 5,                       // After the ö in "Hällö"
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

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

        let mut app = App {
            todo_list,
            selected_index: 0,
            mode: AppMode::Edit,
            edit_text: "Ümlaut test".to_string(), // Starts with umlaut
            edit_cursor: 0,
            config_dir: std::path::PathBuf::new(),
            _lock_file: tempfile::tempfile().unwrap(),
            should_quit: false,
        };

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
