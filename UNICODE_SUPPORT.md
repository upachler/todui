# Unicode Support Documentation

This document describes the Unicode support implemented in TODUI, including the technical details of the fix and how it handles international characters.

## Overview

TODUI fully supports Unicode characters in todo item text, including international characters such as German umlauts (ä, ö, ü), French accents (é, è, ç), and characters from non-Latin scripts. The application properly handles Unicode in all text operations including editing, cursor positioning, and display.

## Problem Description

The original implementation had a critical bug when handling Unicode characters in edit mode. The issue occurred because the code treated string positions as byte indices rather than character indices. In UTF-8 encoding, Unicode characters can occupy multiple bytes:

- ASCII characters: 1 byte each
- German umlauts (ä, ö, ü): 2 bytes each  
- Many other international characters: 2-4 bytes each

## Technical Implementation

### Character vs. Byte Position Handling

The fix involves distinguishing between character positions (what the user sees) and byte positions (how Rust stores UTF-8 strings internally).

#### Cursor Position Management
```rust
// OLD (incorrect): Used byte positions
edit_cursor: usize, // This was treated as byte position

// NEW (correct): Use character positions
edit_cursor: usize, // Character position, not byte position
```

#### Character Insertion
```rust
// Convert character position to byte position for insertion
let byte_pos = self
    .edit_text
    .char_indices()
    .nth(self.edit_cursor)
    .map(|(pos, _)| pos)
    .unwrap_or(self.edit_text.len());

self.edit_text.insert(byte_pos, c);
self.edit_cursor += 1;
```

#### Character Deletion (Backspace)
```rust
if self.edit_cursor > 0 {
    // Find the byte position of the character before cursor
    let char_indices: Vec<_> = self.edit_text.char_indices().collect();
    if let Some(&(byte_pos, _)) = char_indices.get(self.edit_cursor - 1) {
        // Find the next character's byte position (or end of string)
        let next_byte_pos = char_indices
            .get(self.edit_cursor)
            .map(|(pos, _)| *pos)
            .unwrap_or(self.edit_text.len());

        // Remove the character by removing the byte range
        self.edit_text.drain(byte_pos..next_byte_pos);
        self.edit_cursor -= 1;
    }
}
```

#### Character Deletion (Delete Key)
```rust
if self.edit_cursor < self.edit_text.chars().count() {
    let char_indices: Vec<_> = self.edit_text.char_indices().collect();
    if let Some(&(byte_pos, _)) = char_indices.get(self.edit_cursor) {
        let next_byte_pos = char_indices
            .get(self.edit_cursor + 1)
            .map(|(pos, _)| *pos)
            .unwrap_or(self.edit_text.len());

        self.edit_text.drain(byte_pos..next_byte_pos);
    }
}
```

#### Cursor Movement
```rust
// Right arrow key
if self.edit_cursor < self.edit_text.chars().count() {
    self.edit_cursor += 1;
}

// End key
self.edit_cursor = self.edit_text.chars().count();
```

## Supported Features

### Text Input
- ✅ Type Unicode characters directly
- ✅ Insert Unicode characters at any cursor position
- ✅ Mix ASCII and Unicode characters freely

### Text Editing
- ✅ Backspace removes Unicode characters correctly
- ✅ Delete key removes Unicode characters correctly
- ✅ Cursor positioning works with Unicode characters
- ✅ Home/End keys work correctly with Unicode text

### Display
- ✅ Unicode characters display correctly in the terminal
- ✅ Text wrapping works with Unicode characters
- ✅ Cursor indicator (|) positions correctly in Unicode text
- ✅ Syntax highlighting preserved with Unicode text

## Examples

### German Text
```
Before: "Hallo Welt"
Insert 'ü' at position 4: "Hallü Welt"
```

### French Text  
```
Before: "Bonjour"
Insert 'ç' at position 3: "Bonçjour"
```

### Mixed Scripts
```
Before: "Hello"
Insert '世界' at end: "Hello世界"
```

## Error Prevention

### Original Error
```
thread 'main' panicked at src/main.rs:482:26:
assertion failed: self.is_char_boundary(idx)
```

This error occurred when trying to insert or remove characters at invalid byte boundaries in UTF-8 strings.

### Fix Strategy
1. **Always work with character indices** for user-visible operations
2. **Convert to byte indices** only when interfacing with Rust's string methods
3. **Use proper UTF-8 string methods** like `char_indices()` and `chars().count()`
4. **Handle edge cases** like empty strings and end-of-string positions

## Testing

### Comprehensive Test Suite

The Unicode support includes extensive tests covering:

#### Basic Character Handling
```rust
#[test]
fn test_unicode_character_handling() {
    // Tests insertion of umlauts at various positions
    // Verifies cursor position updates correctly
}
```

#### Deletion Operations
```rust
#[test]
fn test_unicode_backspace_and_delete() {
    // Tests backspace and delete with Unicode characters
    // Verifies correct character removal and cursor adjustment
}
```

#### Cursor Movement
```rust
#[test]
fn test_unicode_cursor_movement() {
    // Tests arrow keys, Home, End with Unicode text
    // Verifies cursor moves by characters, not bytes
}
```

#### Display Rendering
```rust
#[test]
fn test_unicode_display_with_cursor() {
    // Tests that cursor indicator displays correctly
    // in Unicode text during editing
}
```

## Performance Considerations

### Character Index Caching
The implementation uses `char_indices().collect()` to create a vector of character positions. This provides O(1) access to character positions but requires O(n) memory and time to build.

### Alternative Approaches Considered
1. **Real-time calculation**: Calculate byte positions on each operation (slower)
2. **Character boundary checking**: Use `is_char_boundary()` (more complex error handling)
3. **String slicing**: Use character-aware slicing (less precise for editing operations)

The chosen approach balances performance with correctness and maintainability.

## Compatibility

### Terminal Support
- ✅ Works with terminals that support Unicode display
- ✅ Graceful degradation in terminals with limited Unicode support
- ✅ Consistent behavior across different operating systems

### Character Encoding
- ✅ Input and output use UTF-8 encoding
- ✅ File storage maintains UTF-8 encoding
- ✅ Cross-platform compatibility maintained

## Limitations

### Current Limitations
- **Combining Characters**: Basic support; complex combining character sequences may not be handled perfectly
- **Right-to-Left Text**: Display follows terminal's RTL handling
- **Double-Width Characters**: Relies on terminal's width calculation

### Future Enhancements
- **Advanced Unicode Normalization**: Handle different Unicode normalization forms
- **Grapheme Cluster Support**: Proper handling of complex character combinations
- **Bidirectional Text**: Enhanced RTL and mixed-direction text support

## Migration Notes

### Backward Compatibility
- ✅ Existing ASCII-only todo files work unchanged
- ✅ No breaking changes to file format
- ✅ Seamless upgrade path for existing users

### File Format
The markdown file format naturally supports UTF-8, so Unicode characters are stored correctly:

```markdown
# TODO 2025-08-15

* [ ] Bücher kaufen
* [x] Café besuchen  
* [ ] Résumé überarbeiten
```

## Troubleshooting

### Common Issues

#### Terminal Display Problems
**Problem**: Unicode characters appear as question marks or boxes
**Solution**: Use a terminal that supports Unicode (most modern terminals do)

#### Input Method Issues  
**Problem**: Cannot type Unicode characters
**Solution**: Ensure your system's input method is properly configured

#### File Encoding Issues
**Problem**: Unicode characters corrupted when viewing files externally
**Solution**: Ensure external editors are set to UTF-8 encoding

### Debugging Unicode Issues

If you encounter Unicode-related problems:

1. **Check terminal capabilities**: `echo "Unicode test: äöü éèç 世界"`
2. **Verify file encoding**: Use `file` command to check encoding
3. **Test character input**: Try typing Unicode characters in other applications
4. **Check locale settings**: Ensure `LC_ALL` or `LANG` includes UTF-8

## References

- [Rust String Documentation](https://doc.rust-lang.org/std/string/struct.String.html)
- [UTF-8 Encoding Standard](https://tools.ietf.org/html/rfc3629)
- [Unicode Standard](https://unicode.org/standard/standard.html)
- [Rust Unicode Handling Best Practices](https://doc.rust-lang.org/book/ch08-02-strings.html)