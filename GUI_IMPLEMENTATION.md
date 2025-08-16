# GUI Implementation for TODUI

## Overview

TODUI now supports both terminal (TUI) and graphical (GUI) user interfaces. The implementation uses Slint for the GUI framework while maintaining the original ratatui-based terminal interface.

## Usage

### Terminal UI (Default)
```bash
cargo run
```

### Graphical UI
```bash
cargo run -- --gui
```

## GUI Features

The graphical user interface provides a more context-based approach compared to the modal terminal UI:

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

## Architecture

### Shared Core (`TodoApp`)
Both UI implementations use a shared `TodoApp` struct that handles:
- File I/O operations
- Todo list management
- Lock file management
- Auto-save functionality

### UI-Specific Code
- **Terminal UI**: Uses the original `App` struct with modal editing
- **Graphical UI**: Uses Slint callbacks with direct item manipulation

## Key Differences from Terminal UI

| Feature | Terminal UI | Graphical UI |
|---------|-------------|--------------|
| Item Selection | Modal with highlighted selection | Direct interaction with any item |
| Text Editing | Enter edit mode | Click to edit in-place |
| Completion Toggle | `x` key on selected item | Click checkbox on any item |
| Indentation | Tab/Shift+Tab on selected item | `<`/`>` buttons on any item |
| Deletion | `d` key + confirmation | Direct click on trash button |
| Navigation | Keyboard arrows | Mouse/scroll |

## File Structure

```
ui/
└── appwindow.slint     # Slint UI definition
src/
└── main.rs            # Main application with both UI implementations
build.rs               # Slint build script
```

## Dependencies

### Core
- `chrono`: Date handling
- `dirs`: Configuration directory management

### Terminal UI
- `ratatui`: Terminal UI framework
- `crossterm`: Terminal control

### Graphical UI
- `slint`: GUI framework
- `slint-build`: Build-time UI compilation

### Command Line
- `clap`: Argument parsing

## Error Handling

Both interfaces share the same error handling for:
- Lock file conflicts
- File system permissions
- Configuration directory creation