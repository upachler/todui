# Text Wrapping Implementation Summary

This document describes the text wrapping functionality implemented in TODUI to handle long todo items that exceed the terminal width.

## Overview

The text wrapping feature ensures that long todo item text is properly displayed across multiple lines when it exceeds the available terminal width, while maintaining proper formatting and alignment.

## Key Features

### 1. Word-Boundary Wrapping
- Text is wrapped at word boundaries whenever possible
- Long words that exceed the available width are broken across lines
- Preserves readability by avoiding mid-word breaks when feasible

### 2. Proper Indentation
- Continuation lines are properly aligned with the todo item text
- Nested items maintain their hierarchical indentation
- Checkbox and bullet formatting is preserved only on the first line

### 3. Edit Mode Support
- Text wrapping works during editing with cursor display
- Cursor position is maintained and visible in wrapped text
- Real-time wrapping as text is being edited

## Implementation Details

### Core Function: `wrap_todo_item_text`

```rust
fn wrap_todo_item_text(
    item: &TodoItem,
    available_width: usize,
    is_selected: bool,
    edit_text: &str,
    edit_cursor: usize,
    is_editing: bool,
) -> Vec<(String, bool)>
```

**Parameters:**
- `item`: The TodoItem to wrap
- `available_width`: Terminal width available for the item
- `is_selected`: Whether this item is currently selected
- `edit_text`: The text being edited (if in edit mode)
- `edit_cursor`: Position of the cursor in edit mode
- `is_editing`: Whether the item is currently being edited

**Returns:**
- Vector of `(display_line, is_main_line)` tuples
- `display_line`: The formatted text line to display
- `is_main_line`: Whether this is the first line (contains checkbox)

### Width Calculation

1. **Available Width**: Terminal width minus borders and padding (typically width - 4)
2. **Prefix Length**: Calculated based on indentation level and checkbox format
   - Format: `{indent}* {checkbox} ` 
   - Example: `  * [ ] ` for level 1 nested item
3. **Text Width**: Available width minus prefix length

### Wrapping Algorithm

1. **Text Preparation**: 
   - Use edit text if in editing mode, otherwise use item text
   - Insert cursor character `|` at cursor position if editing

2. **Word Processing**:
   - Split text into words using whitespace
   - Process each word to determine line placement
   - Handle three cases:
     - Word fits on current line: Add to current line
     - Word doesn't fit: Start new line with word
     - Word too long for any line: Break word across lines

3. **Line Formatting**:
   - First line: Include full prefix with indentation and checkbox
   - Continuation lines: Use continuation prefix (indentation + 3 spaces)
   - Example:
     ```
     * [ ] This is a very long todo item that needs to be wrapped
        across multiple lines to fit in the terminal width
     ```

### Integration with UI

The wrapping function is called during UI rendering in the `ui` function:

1. **Calculate Available Width**: Determine terminal width available for content
2. **Process Each Item**: Call wrapping function for each todo item
3. **Create Display Items**: Convert wrapped lines to `ListItem` widgets
4. **Handle Selection**: Map logical item selection to display line selection

## Example Output

### Input Todo Item
```markdown
* [ ] Create comprehensive documentation for the text wrapping feature including implementation details and examples
```

### Terminal Width: 50 characters
```
* [ ] Create comprehensive documentation for the
   text wrapping feature including implementation
   details and examples
```

### Terminal Width: 30 characters
```
* [ ] Create comprehensive
   documentation for the text
   wrapping feature including
   implementation details and
   examples
```

## Edge Cases Handled

### 1. Very Long Words
Words longer than available width are broken across lines:
```
* [ ] supercalifragilisticexpiali
   docious
```

### 2. Empty Text
Items with empty text display just the checkbox:
```
* [ ] 
```

### 3. Nested Items
Proper indentation is maintained:
```
* [ ] Main item
  * [ ] This is a nested item with very long text
     that wraps properly while maintaining the
     correct indentation level
```

### 4. Edit Mode with Cursor
Cursor position is preserved during wrapping:
```
* [ ] This is being edit|ed and the cursor is
   visible even when the text wraps across
   multiple lines
```

## Testing

The implementation includes comprehensive unit tests covering:
- Short text (no wrapping needed)
- Long text requiring wrapping
- Nested items with proper indentation
- Edit mode with cursor handling
- Edge cases (empty text, very long words)

## Performance Considerations

- Word splitting is done once per render
- Minimal string allocations through careful memory management
- Efficient calculation of continuation line prefixes
- No unnecessary recomputation of static prefixes

## Future Enhancements

Potential improvements for the text wrapping feature:

1. **Smart Breaking**: Implement more sophisticated word breaking (hyphens, etc.)
2. **Variable Width Handling**: Dynamic adjustment when terminal is resized
3. **RTL Language Support**: Handle right-to-left text properly
4. **Unicode Awareness**: Better handling of wide characters and combining marks