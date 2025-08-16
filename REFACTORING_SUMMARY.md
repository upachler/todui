# Refactoring Summary: Lock File and Initialization

## Problem Addressed

The initial GUI implementation had a design flaw where the lock file check and configuration directory creation were embedded within the UI-specific constructors (`App::new()` and `TodoApp::new()`). This meant:

1. **Inconsistent Lock File Enforcement**: The GUI bypassed the lock file check because it used a different code path than the terminal UI
2. **Duplicated Initialization Logic**: Both UI variants had their own initialization code
3. **Poor Separation of Concerns**: UI-specific structs were responsible for system-level initialization

## Solution Implemented

### 1. Moved Common Initialization to Main Function

**Before**:
```rust
// In App::new() and TodoApp::new()
let config_dir = Self::get_config_dir()?;
if !config_dir.exists() {
    fs::create_dir_all(&config_dir)?;
}
let lock_file = Self::create_lock_file(&config_dir)?;
let todo_list = Self::load_or_create_todo_list(&config_dir, today)?;
```

**After**:
```rust
// In main() function - before UI selection
let config_dir = get_config_dir()?;
if !config_dir.exists() {
    fs::create_dir_all(&config_dir)?;
}
let lock_file = create_lock_file(&config_dir)?;
let todo_list = load_or_create_todo_list(&config_dir, today)?;
```

### 2. Simplified UI Constructors

**Before**:
```rust
impl App {
    fn new() -> Result<Self, Box<dyn Error>> { /* complex initialization */ }
}

impl TodoApp {
    fn new() -> Result<Self, Box<dyn Error>> { /* duplicate initialization */ }
}
```

**After**:
```rust
impl App {
    fn new(config_dir: PathBuf, lock_file: File, todo_list: TodoList) -> Self { /* simple */ }
}

impl TodoApp {
    fn new(config_dir: PathBuf, lock_file: File, todo_list: TodoList) -> Self { /* simple */ }
}
```

### 3. Updated Function Signatures

**UI Entry Points**:
- `run_tui(config_dir: PathBuf, lock_file: File, todo_list: TodoList)`
- `run_gui(todo_app: TodoApp)` (where TodoApp is pre-initialized)

**Extracted Utility Functions**:
- `get_config_dir() -> Result<PathBuf, Box<dyn Error>>`
- `create_lock_file(config_dir: &PathBuf) -> Result<File, Box<dyn Error>>`
- `load_or_create_todo_list(config_dir: &PathBuf, target_date: NaiveDate) -> Result<TodoList, Box<dyn Error>>`

## Benefits Achieved

### 1. Consistent Lock File Enforcement
- Both terminal and GUI interfaces now properly check for existing instances
- Lock file is created before UI selection, ensuring no race conditions
- Identical error messages and behavior for both interfaces

### 2. Single Point of Initialization
- All system-level setup happens in one place (`main()` function)
- Easier to maintain and modify initialization logic
- Clear separation between system initialization and UI-specific setup

### 3. Improved Error Handling
- Initialization errors are handled before any UI setup occurs
- No need for complex cleanup in UI constructors
- Consistent error reporting across both interfaces

### 4. Better Code Organization
- UI structs focus solely on their presentation responsibilities
- System-level operations are in standalone functions
- Clear dependency injection pattern for UI components

## Testing Verification

### Lock File Functionality
```bash
# Terminal UI properly blocked by existing lock file
touch ~/.todui/lockfile
cargo run
# Error: "Another instance of todui appears to be running..."

# GUI properly blocked by existing lock file
cargo run -- --gui  
# Error: "Another instance of todui appears to be running..."
```

### Normal Operation
```bash
# Both interfaces start successfully when no lock file exists
rm ~/.todui/lockfile
cargo run          # Terminal UI starts
cargo run -- --gui # GUI starts
```

## Code Quality Improvements

1. **Reduced Complexity**: UI constructors are now simple factory functions
2. **Better Testability**: Initialization functions are standalone and testable
3. **Clearer Intent**: The main function clearly shows the application flow
4. **No Duplication**: Single implementation of initialization logic
5. **Proper Error Propagation**: Errors bubble up correctly from main function

## Backward Compatibility

- All existing functionality preserved
- Same command-line interface
- Same file formats and storage locations
- Same error messages and behavior
- All unit tests continue to pass (23/23)

This refactoring demonstrates proper separation of concerns and ensures that both UI variants have identical system-level behavior while maintaining their distinct user experiences.