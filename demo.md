# TODUI Demo

This document demonstrates the TODUI application functionality.

## Running the Application

To start TODUI:

```bash
cargo run
```

## Demo Walkthrough

### 1. First Run
When you first run TODUI, it will:
- Create the `~/.todui` directory if it doesn't exist
- Create a lock file to prevent multiple instances
- Load today's todo list (or create a new empty one)

### 2. Basic Navigation
- Use `↑`/`k` and `↓`/`j` to navigate between items
- The highlighted item is shown with inverted colors
- Press `q` to quit

### 3. Adding Items
- Press `i` to insert a new todo item
- Type your todo text
- Press `Enter` to confirm or `Esc` to cancel

### 4. Editing Items
- Navigate to an item and press `Enter` to edit
- Use arrow keys to move cursor within the text
- Press `Enter` to save changes or `Esc` to cancel

### 5. Toggling Completion
- Navigate to an item and press `x` to toggle completion
- Completed items are shown with `[x]` and in gray text
- Incomplete items are shown with `[ ]`

### 6. File Structure
Your todo list is automatically saved to:
```
~/.todui/TODO-YYYY-MM-DD.md
```

Example content:
```markdown
# TODO 2025-08-14

* [x] Morning routine
* [ ] Work tasks
  * [x] Review code
  * [ ] Write documentation
  * [ ] Team meeting
* [ ] Personal tasks
  * [ ] Grocery shopping
  * [ ] Call dentist
```

### 7. Text Wrapping
Long todo items automatically wrap to fit your terminal width:
- Text wraps at word boundaries when possible
- Continuation lines are properly indented to align with the todo text
- Works with both regular and nested items
- Maintains proper formatting during editing

### 8. Hierarchical Items
The application supports nested items with proper indentation:
- Main items start at the left margin
- Sub-items are indented with 2 spaces per level
- You can create nested structures manually by editing the markdown file

### 9. Process Safety
- Only one instance can run at a time
- Lock file prevents conflicts: `~/.todui/lockfile`
- Application automatically cleans up on exit

### 10. Date Handling
- Application loads the most recent todo file (not in the future)
- When modified, the date updates to today
- Future-dated files are ignored with warnings

## Tips

1. **Keyboard Shortcuts**: Learn the vim-style navigation (`j`/`k` for up/down)
2. **Quick Entry**: Use `i` to quickly add items without leaving selection mode
3. **Long Text**: Don't worry about text length - it will wrap automatically
4. **Hierarchical Organization**: Edit the markdown file directly for complex nested structures
5. **Daily Planning**: Each day gets its own file, making it easy to track daily progress
6. **Markdown Compatibility**: Files are standard markdown and can be viewed/edited in any text editor

## Troubleshooting

- **Lock file error**: Another instance is running, or previous instance crashed. Delete `~/.todui/lockfile`
- **Permission errors**: Check write permissions for `~/.todui` directory
- **Invalid date format**: Ensure todo files follow the `TODO-YYYY-MM-DD.md` naming convention