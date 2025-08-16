# TODUI

Todui is a terminal application managing a todo lists stored in its configuration directory (`$HOME/.todui` on Linux and Mac or `%HOME%\.todui` on Windows).

Todo lists are stored in files with a name starting with `TODO-`, followed by an ISO 8601 date, followed by `.txt`. So as an example, a TODO file for the 14th of August 2025 is called `TODO-2025-08-14.md`.


## TODO list files

A Todo list is a markdown document with a specified structure:
* The first line is a header line, starting with `# TODO ` followed by ISO 8601 date when the TODO markdown document file was created.
* The second line in the document is empty.
* At the third line, the todo items start. If there are no items, the third line either does not exist or is empty.

So an empty TODO list file may look like this:

```markdown
# TODO 2025-08-14

```

Each todo item is a line starting with a bullet '* ', followed by a text indicating whether the todo item was completed or not (`[x]` when completed, `[ ]` if not ), followed by the actual TODO text. TODO items can be nested. See this example:

```markdown
# TODO 2025-08-14

* [x] take out trash
* [ ] shop groceries
  * [x] Apples
  * [x] saussages
  * [ ] cheese
* [x] fetch kids from scool
```


## Startup

When the application opens, it checks if it's configuration directory (see above) exists. If it doesn't, the app creates the directory. Failure to create the directory terminates the application with an error message.

Then, the application tries to create a lock file with process-exclusive access titled `lockfile` in it's configuration directory (see above). If the file already exists, the application terminates with an error message on the terminal saying another instance of the program appears to be running already because the lock file already exists. The error message should include a path to the lock file. If the lockfile was created successfully, the app writes it's process ID as text into the file and flushes it to disk. The file is kept open until the program terminates.

After creating the lock file, the newest TODO list file is opened (as per the date that is encoded in it's filename). TODO list files with a date in the future are ignored; if they exist, a warning is printed to the terminal. When opened, the date for the in-memory representation is changed to the current date, so that, when the TODO list is changed by the user


## The Terminal UI of TODUI

The application shows the open TODO list on its main screen. The list consists of the title `TODO <date>`, where `<date>` stands for the current ISO date, and a list of individual items that represent the structures of the TODO list and it's hierarchy. Each Item consists of:
* an indentation (2 spaces per hierarchy level), depending on how deep it is in the hierarchy
* a checkbox `[ ]` or `[x]`
* the TODO text (a single line). TODO text that is longer than the screen width permis is word-wrapped, not cut off. There is no limit on how long the TODO text can be.

Besides the TODO list, the application shows a status bar at the bottom indicating the currently available commands. Each command is linked with one or more keys. Commands may be available depending on the current context, which means the status bar will change when different commands are available.

The context is determined by the current application mode. Here is a list of modes the application can be in:

| Name | Short name | Description |
|----- |------------|-------------|
| Selection Mode |Sel | In this mode the user can highlight TODO items. By default, the first TODO item is highlighted. The highlighted item is shown in inverted text |
| Edit Mode | Edit | The todo text of the highlighted item is currently being edited by the user |
| Delete Mode | Delete | The highlighted item is about to be deleted |


A list of commands and in which modes they are available:

| Mode short name | Command | Keys | effect |
|-----|----|----|----|
| Sel | Highlight previous item | <cursor up>, `k` | moves the highlight to the todo item above |
| Sel | Highlight next item | <cursor up>, `j` | moves the highlight to the todo item below. The highlight can move one element past the last item (this is to allow insertions after the last item). |
| Sel | Quit | `q` | terminates application |
| Sel | Toggle complete | `x` | Toggles the check box to change the state of the TODO item (complete / incomplete). Triggers saving the TODO list file |
| Sel | Insert | `i` | inserts a new, unchecked TODO item with blank text and enters Edit Mode. The new item has the same indentation as the item before it, if one exists, otherwise it has the outermost indentation |
| Sel | Edit | <enter> | enters Edit Mode to edit the currently highlighted TODO item |
| Sel | Indent | <tab> | Intents the currently highlighted TODO item by one level |
| Sel | Unindent | <shift>+<tab> | Unindent the currently highlighted TODO item by one level | Sel | Delete | `d` | Enter Delete mode for highlighted entry |
| Del | Confirm Delete | `y` | The highlighted element is deleted. Changes back to Selection Mode |
| Del | Cancel Delete | <esc> | Changes back to Selection mode without deleting the item |
| Edit | Cancel | <esc> | Cancels edit mode, discarding any changes to the TODO item text. Changes back to Selection Mode |
| Edit | Confirm | <enter> | Confirm changes to the TODO text. Triggers saving the TODO list file. Changes back to Selection Mode |
| Edit | Change TODO item text | any other key | Changes the edit item text - so entering keys will enter text, the cursor left/right keys will move the text editing cursor, as expected. Any key that is not bound to another command in edit mode will cause text to be entered (except for keys that result in non-printable characters, those are ignored) |


## Saving the TODO list file

The TODO list file is saved when saving is triggered. This happens either via one of the commands or when the application terminates. When the file is saved, the application performs these steps:
* Check if the current date, and if it differs from the date of the TODO list in memory, update it'd date.
* Derive the file name to save to from the date in the loaded TODO list as per the TODO file name rule above.
* Write the TODO list as markdown to the specified filename. If a todo list file already exists, it is overwritten.


## Technical information

The application is written in the Rust programming language. It uses the `ratatui` crate for rendering the terminal UI and the `markdown` crate for parsing Markdown files.
