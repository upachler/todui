# TODUI - Terminal Todo List Manager

A simple terminal-based todo list application written in Rust.

## Features

- **Persistent Storage**: Todo lists are stored in Markdown files in your home directory (`~/.todui/`)
- **Date-based Organization**: Each todo list is associated with a specific date
- **Hierarchical Items**: Support for nested todo items with proper indentation
- **Text Wrapping**: Long todo items automatically wrap to fit terminal width
- **Interactive UI**: Terminal-based interface using ratatui
- **Process Locking**: Prevents multiple instances from running simultaneously

## Installation

Make sure you have Rust installed, then build the application:

```bash
cargo build --release
```

The binary will be available at `target/release/todui`.

## Usage

Simply run the application:

```bash
cargo run
```

Or if you built the release version:

```bash
./target/release/todui
```

### Controls

#### Selection Mode (default)
- `↑` or `k`: Move highlight up
- `↓` or `j`: Move highlight down  
- `x`: Toggle completion status of highlighted item
- `i`: Insert new todo item
- `Enter`: Edit the highlighted item
- `q`: Quit application

#### Edit Mode
- `Enter`: Confirm changes and return to selection mode
- `Esc`: Cancel changes and return to selection mode
- `←` / `→`: Move cursor left/right
- `Home` / `End`: Move cursor to beginning/end
- `Backspace` / `Delete`: Delete characters
- Any printable character: Insert text

## File Format

Todo lists are stored as Markdown files in `~/.todui/` with the naming convention:
- `TODO-YYYY-MM-DD.md` (e.g., `TODO-2025-08-14.md`)

Example file content:
```markdown
# TODO 2025-08-14

* [x] take out trash
* [ ] shop groceries
  * [x] Apples
  * [x] sausages  
  * [ ] cheese
* [x] fetch kids from school
```

## Behavior

- The application automatically loads the most recent todo list (not in the future)
- When you make changes, the list is automatically saved
- The date is updated to today's date when the file is modified
- Only one instance can run at a time (enforced by a lock file)
- Future-dated todo files are ignored with a warning

## Configuration Directory

- **Linux/Mac**: `~/.todui/`
- **Windows**: `%USERPROFILE%\.todui\`

The directory contains:
- Todo list files (`TODO-YYYY-MM-DD.md`)
- Lock file (`lockfile`) - automatically managed

## Development

Run tests:
```bash
cargo test
```

The code includes unit tests for the markdown parsing and todo item functionality.