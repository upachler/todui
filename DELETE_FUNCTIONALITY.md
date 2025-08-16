# Delete Functionality Documentation

This document provides comprehensive documentation for the delete functionality implemented in TODUI.

## Overview

The delete functionality allows users to safely remove todo items from their list with a confirmation step to prevent accidental deletions. The feature introduces a new application mode specifically for handling item deletion.

## Feature Description

### Three-Step Deletion Process

1. **Select Item**: Navigate to the todo item you want to delete using arrow keys or `j`/`k`
2. **Enter Delete Mode**: Press `d` to enter delete mode for the highlighted item
3. **Confirm or Cancel**: Press `y` to confirm deletion or `Esc` to cancel

## Application Modes

### Delete Mode

Delete Mode is a new application mode that provides a safety mechanism for item deletion:

- **Purpose**: Prevents accidental deletion by requiring explicit confirmation
- **Visual Feedback**: Selected item is highlighted in red to indicate impending deletion
- **Commands Available**: Only confirmation (`y`) or cancellation (`Esc`) are allowed

## User Interface Elements

### Visual Indicators

- **Normal Selection**: Items highlighted in dark gray background
- **Delete Mode**: Items highlighted in red background with white text
- **Status Bar**: Shows "Delete | y:Confirm Delete | Esc:Cancel" in delete mode

### Status Bar Messages

| Mode | Status Bar Text |
|------|----------------|
| Selection (with items) | `Sel \| ↑k:Up \| ↓j:Down \| x:Toggle \| i:Insert \| Enter:Edit \| Tab:Indent \| Shift+Tab:Unindent \| d:Delete \| q:Quit` |
| Delete | `Delete \| y:Confirm Delete \| Esc:Cancel` |

## Keyboard Commands

### Selection Mode
- **`d`**: Enter delete mode for the currently highlighted item
  - Only available when an actual item is selected (not the virtual insertion point)
  - Transitions from Selection Mode to Delete Mode

### Delete Mode
- **`y`**: Confirm deletion
  - Removes the selected item from the todo list
  - Automatically saves the updated list to file
  - Returns to Selection Mode
  - Adjusts selection index if necessary

- **`Esc`**: Cancel deletion
  - Returns to Selection Mode without making any changes
  - Item remains unmodified in the list

## Technical Implementation

### Mode Management

```rust
#[derive(Debug, PartialEq)]
enum AppMode {
    Selection,
    Edit,
    Delete,
}
```

### Key Event Handling

The application uses separate handler functions for each mode:

```rust
fn handle_key_event(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
    match self.mode {
        AppMode::Selection => self.handle_selection_mode_key(key)?,
        AppMode::Edit => self.handle_edit_mode_key(key)?,
        AppMode::Delete => self.handle_delete_mode_key(key)?,
    }
    Ok(())
}
```

### Selection Index Management

When an item is deleted, the application intelligently manages the selection index:

1. **Middle Item Deletion**: Selection remains at the same index (now pointing to the next item)
2. **Last Item Deletion**: Selection moves to the previous item
3. **Only Item Deletion**: Selection resets to index 0 (empty list state)

## Safety Features

### Confirmation Required
- No immediate deletion - always requires explicit confirmation
- Clear visual feedback shows which item will be deleted
- Easy cancellation with `Esc` key

### Automatic Saving
- Confirmed deletions automatically trigger file saving
- No manual save action required
- Maintains data consistency

### Bounds Checking
- Delete mode only available when a valid item is selected
- Cannot enter delete mode when at the virtual insertion point
- Proper handling of empty lists

## Usage Examples

### Basic Deletion
```
1. Navigate to item: "* [ ] Buy groceries"
2. Press 'd' - item turns red, status shows delete commands
3. Press 'y' - item is deleted and file is saved
```

### Canceling Deletion
```
1. Navigate to item: "* [ ] Important meeting"
2. Press 'd' - item turns red
3. Press 'Esc' - returns to normal selection, item unchanged
```

### Deleting Last Item in List
```
Initial list:
* [ ] Task 1
* [ ] Task 2  <- selected

1. Press 'd' - Task 2 turns red
2. Press 'y' - Task 2 deleted, selection moves to Task 1
```

## Error Handling

### Edge Cases Handled
- **Empty List**: Delete command (`d`) has no effect
- **Invalid Selection**: Bounds checking prevents invalid operations
- **File Save Errors**: Proper error propagation and handling

### Recovery Scenarios
- **Accidental Delete Mode**: Easy cancellation with `Esc`
- **Selection Out of Bounds**: Automatic adjustment after deletion
- **Last Item Deletion**: Graceful handling of empty list state

## Integration with Other Features

### Text Wrapping Compatibility
- Delete mode works seamlessly with wrapped text items
- Visual highlighting applies to all wrapped lines of the item
- Selection management accounts for display vs. logical item mapping

### Hierarchical Items
- Deletion works at any indentation level
- No automatic deletion of child items (each item deleted individually)
- Indentation structure preserved after deletion

### Auto-Save Integration
- Deletion triggers the same save mechanism as other modifications
- Date updating logic applies (changes date to today if needed)
- File overwriting follows standard save procedures

## Testing

### Unit Test Coverage
The delete functionality includes comprehensive unit tests:

- **Mode Transitions**: `test_delete_mode_transitions`
- **Deletion Confirmation**: `test_delete_confirmation`
- **Selection Adjustment**: `test_delete_last_item_adjusts_selection`
- **Index Management**: `test_delete_adjusts_selection_when_deleting_last_item`

### Test Scenarios
1. Entering and exiting delete mode
2. Confirming deletion and verifying item removal
3. Canceling deletion and verifying no changes
4. Selection index adjustment after deletion
5. Handling deletion of the last item in the list

## Performance Considerations

### Minimal Overhead
- Delete mode adds minimal computational overhead
- Visual highlighting reuses existing styling system
- No additional memory allocation for delete operations

### Efficient Updates
- Direct vector element removal (`Vec::remove`)
- Single file save operation per deletion
- Immediate UI updates after state changes

## Future Enhancements

### Potential Improvements
1. **Batch Deletion**: Select multiple items for deletion
2. **Undo Functionality**: Ability to undo recent deletions
3. **Confirmation Dialog**: More detailed confirmation with item preview
4. **Delete with Dependencies**: Handle deletion of parent items with children
5. **Keyboard Shortcuts**: Alternative key combinations for power users

### Backward Compatibility
- Current implementation maintains full backward compatibility
- Existing todo files work unchanged
- No breaking changes to file format or structure