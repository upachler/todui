use chrono::{Local, NaiveDate};
use clap::Parser;

use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    process,
};

mod gui;
mod tui;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start with graphical user interface
    #[arg(long)]
    gui: bool,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub text: String,
    pub completed: bool,
    pub indent_level: usize,
}

impl TodoItem {
    pub fn new(text: String, completed: bool, indent_level: usize) -> Self {
        Self {
            text,
            completed,
            indent_level,
        }
    }

    pub fn to_markdown_line(&self) -> String {
        let indent = "  ".repeat(self.indent_level);
        let checkbox = if self.completed { "[x]" } else { "[ ]" };
        format!("{}* {} {}", indent, checkbox, self.text)
    }
}

#[derive(Debug)]
pub struct TodoList {
    pub date: NaiveDate,
    pub items: Vec<TodoItem>,
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

// Shared core for both TUI and GUI
pub struct TodoApp {
    todo_list: TodoList,
    config_dir: PathBuf,
}

impl TodoApp {
    fn new(config_dir: PathBuf, todo_list: TodoList) -> Self {
        TodoApp {
            todo_list,
            config_dir,
        }
    }

    pub fn save_todo_list(&mut self) -> Result<(), Box<dyn Error>> {
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

    pub fn toggle_item_completed(&mut self, index: usize) -> Result<bool, Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items[index].completed = !self.todo_list.items[index].completed;
            self.save_todo_list()?;
        }
        Ok(self.todo_list.items[index].completed)
    }

    pub fn update_item_text(&mut self, index: usize, text: String) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items[index].text = text;
            self.save_todo_list()?;
        }
        Ok(())
    }

    pub fn indent_item_left(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.indent_item_by(index, -1)
    }

    pub fn indent_item_right(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.indent_item_by(index, 1)
    }

    fn indent_item_by(&mut self, index: usize, delta: isize) -> Result<(), Box<dyn Error>> {
        self.todo_list.items[index]
            .indent_level
            .saturating_add_signed(delta);
        self.save_todo_list()?;
        Ok(())
    }

    pub fn delete_item(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        if index < self.todo_list.items.len() {
            self.todo_list.items.remove(index);
            self.save_todo_list()?;
        }
        Ok(())
    }

    pub fn insert_new_item(&mut self, at: usize) -> Result<(), Box<dyn Error>> {
        let indent_level = if let Some(last_item) = self.todo_list.items.last() {
            last_item.indent_level
        } else {
            0
        };

        let new_item = TodoItem::new(String::new(), false, indent_level);
        self.todo_list.items.insert(at, new_item);
        self.save_todo_list()?;
        Ok(())
    }
}

impl Drop for TodoApp {
    fn drop(&mut self) {
        let _ = self.save_todo_list();
    }
}

fn get_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
    Ok(home_dir.join(".todui"))
}

fn create_lock_file(config_dir: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
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

    Ok(lock_path)
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

    let todo_app = TodoApp::new(config_dir, todo_list);
    if args.gui {
        gui::run_gui(todo_app)?
    } else {
        tui::run_tui(todo_app)?
    }

    fs::remove_file(lock_file)?;

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
}
