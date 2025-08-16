# TODUI - Todo List Manager

A powerful todo list application written in Rust that supports both terminal (TUI) and graphical (GUI) interfaces, providing hierarchical task management with smart indentation and text wrapping.

## Features

### Core Functionality
- **Persistent Storage**: Todo lists stored as Markdown files in `~/.todui/`
- **Date-based Organization**: Each day gets its own todo list file
- **Hierarchical Structure**: Create nested todo items with unlimited depth
- **Smart Completion**: Toggle completion status with visual feedback
- **Safe Deletion**: Delete items with confirmation to prevent accidental removal
- **Unicode Support**: Full support for international characters (umlauts, accents, etc.)

### Enhanced User Experience  
- **Smart Indentation**: New items automatically inherit indentation from previous items
- **Manual Indentation Control**: Use Tab/Shift+Tab to adjust item hierarchy
- **Text Wrapping**: Long todo items automatically wrap to fit terminal width
- **Enhanced Navigation**: Navigate past the last item for easy insertion
- **Visual Insertion Point**: Clear indication of where new items will be added
- **International Text**: Proper handling of Unicode characters in all languages

### Technical Features
- **Interactive UI**: Modern terminal interface using ratatui
- **Process Safety**: Lock file prevents multiple instances
- **Markdown Compatibility**: Standard markdown format for external editing
- **Real-time Saving**: Changes automatically saved on modification
- **Future-proof**: Ignores todo files with future dates

## Installation

Make sure you have Rust installed, then build the application:

```bash
cargo build --release
```

The binary will be available at `target/release/todui`.

## Usage

### Terminal Interface (Default)

Run with the terminal interface:

```bash
cargo run
```

### Graphical Interface

Run with the GUI interface:

```bash
cargo run -- --gui
```

Or if you built the release version:

```bash
./target/release/todui         # Terminal interface
./target/release/todui --gui   # Graphical interface
```

### Command Line Options

- `--gui`: Start with graphical user interface
- `--help`: Show help message

## Interface Comparison

| Feature | Terminal UI | Graphical UI |
|---------|-------------|--------------|
| Item Selection | Modal with highlighted selection | Direct interaction with any item |
| Text Editing | Enter edit mode with Enter key | Click to edit in-place |
| Completion Toggle | `x` key on selected item | Click checkbox on any item |
| Indentation | Tab/Shift+Tab on selected item | `<`/`>` buttons on any item |
| Deletion | `d` key + confirmation | Direct click on trash button |
| Navigation | Keyboard arrows | Mouse/scroll |
| New Items | `i` key at selected position | "Add New Todo Item" button |

## Terminal Interface Controls

### Selection Mode (default)
- `↑` or `k`: Move highlight up
- `↓` or `j`: Move highlight down (can move past last item for insertion)
- `x`: Toggle completion status of highlighted item
- `i`: Insert new todo item (inherits indentation from previous item)
- `Tab`: Indent current item one level
- `Shift+Tab`: Unindent current item one level
- `d`: Enter delete mode for highlighted item
- `Enter`: Edit the highlighted item
- `q`: Quit application

### Edit Mode
- `Enter`: Confirm changes and return to selection mode
- `Esc`: Cancel changes and return to selection mode
- `←` / `→`: Move cursor left/right
- `Home` / `End`: Move cursor to beginning/end
- `Backspace` / `Delete`: Delete characters
- Any printable character: Insert text

### Delete Mode
- `y`: Confirm deletion and return to selection mode
- `Esc`: Cancel deletion and return to selection mode

## Graphical Interface Controls

### Todo Item Management
- **Checkbox**: Click to toggle completion status of any item
- **Text Editing**: Click on the text field to edit todo item text directly
- **Indentation Controls**: Use `<` and `>` buttons to decrease/increase indentation
- **Delete**: Click the trash (🗑) button to delete an item immediately

### Auto-Save
- Changes are automatically saved when:
  - Checkbox state changes
  - Text editing loses focus
  - Indentation is modified
  - Items are deleted

### Add New Items
- Click "Add New Todo Item" button to create a new item
- New items inherit the indentation level of the last item

## Terminal Interface Keyboard Shortcuts

### Complete Command Reference

| Mode | Key | Command | Description |
|------|-----|---------|-------------|
| Selection | `↑` or `k` | Navigate up | Move highlight to previous item |
| Selection | `↓` or `j` | Navigate down | Move highlight to next item (can go past last item) |
| Selection | `x` | Toggle completion | Toggle checkbox between `[ ]` and `[x]` |
| Selection | `i` | Insert item | Create new item with inherited indentation |
| Selection | `Enter` | Edit item | Enter edit mode for highlighted item |
| Selection | `Tab` | Indent | Increase item indentation by one level |
| Selection | `Shift+Tab` | Unindent | Decrease item indentation by one level |
| Selection | `d` | Delete | Enter delete mode for highlighted item |
| Selection | `q` | Quit | Exit application |
| Edit | `Enter` | Confirm | Save changes and return to selection mode |
| Edit | `Esc` | Cancel | Discard changes and return to selection mode |
| Edit | `←` / `→` | Move cursor | Navigate within text |
| Edit | `Home` / `End` | Jump cursor | Move to beginning/end of text |
| Edit | `Backspace` | Delete left | Remove character before cursor |
| Edit | `Delete` | Delete right | Remove character after cursor |
| Edit | Any character | Insert text | Add character at cursor position |
| Delete | `y` | Confirm delete | Remove item and return to selection mode |
| Delete | `Esc` | Cancel delete | Return to selection mode without changes |

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