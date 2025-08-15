use chrono::{Local, NaiveDate};
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
}

struct App {
    todo_list: TodoList,
    selected_index: usize,
    mode: AppMode,
    edit_text: String,
    edit_cursor: usize,
    config_dir: PathBuf,
    _lock_file: File,
    should_quit: bool,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let config_dir = Self::get_config_dir()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        // Create and hold lock file
        let lock_file = Self::create_lock_file(&config_dir)?;

        // Load or create today's todo list
        let today = Local::now().date_naive();
        let todo_list = Self::load_or_create_todo_list(&config_dir, today)?;

        Ok(App {
            selected_index: 0,
            mode: AppMode::Selection,
            edit_text: String::new(),
            edit_cursor: 0,
            config_dir,
            _lock_file: lock_file,
            should_quit: false,
            todo_list,
        })
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

                            if let Ok(file_date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d")
                            {
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
                    && self.selected_index < self.todo_list.items.len() - 1
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
                let new_item = TodoItem::new(String::new(), false, 0);
                let insert_pos = if self.todo_list.items.is_empty() {
                    0
                } else {
                    self.selected_index
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
                    self.edit_cursor = self.edit_text.len();
                    self.mode = AppMode::Edit;
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
                if self.edit_cursor < self.edit_text.len() {
                    self.edit_cursor += 1;
                }
            }
            KeyCode::Backspace => {
                if self.edit_cursor > 0 {
                    self.edit_text.remove(self.edit_cursor - 1);
                    self.edit_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if self.edit_cursor < self.edit_text.len() {
                    self.edit_text.remove(self.edit_cursor);
                }
            }
            KeyCode::Home => {
                self.edit_cursor = 0;
            }
            KeyCode::End => {
                self.edit_cursor = self.edit_text.len();
            }
            KeyCode::Char(c) => {
                self.edit_text.insert(self.edit_cursor, c);
                self.edit_cursor += 1;
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
        if edit_cursor <= display_text.len() {
            display_text.insert(edit_cursor, '|');
        }
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
                let style = if is_selected && *is_main_line {
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
    }

    // Calculate which display item should be selected
    let selected_display_index =
        if !app.todo_list.items.is_empty() && app.selected_index < logical_to_display_map.len() {
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
                "Sel | ↑k:Up | ↓j:Down | x:Toggle | i:Insert | Enter:Edit | q:Quit"
            }
        }
        AppMode::Edit => "Edit | Enter:Confirm | Esc:Cancel | ←→:Move cursor",
    };

    let status_paragraph =
        Paragraph::new(status_text).style(Style::default().bg(Color::Blue).fg(Color::White));

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

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let app = App::new().map_err(|e| {
        // Clean up terminal before showing error
        let _ = disable_raw_mode();
        let _ = execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        e
    })?;

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
mod tests {
    use super::*;
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
        assert!(wrapped[0].0.contains("|")); // Should contain cursor
        assert!(wrapped[0].0.starts_with("* [ ] Edited"));
    }
}
