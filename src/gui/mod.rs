use std::error::Error;

use crate::{TodoApp, TodoItem};

slint::include_modules!();

pub fn run_gui(todo_app: TodoApp) -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    // Set initial title
    let title = format!("TODO {}", todo_app.todo_list.date.format("%Y-%m-%d"));
    ui.set_window_title(title.into());

    // Convert TodoItems to TodoItemData for Slint
    let convert_items = |items: &[TodoItem]| -> Vec<TodoItemData> {
        items
            .iter()
            .map(|item| TodoItemData {
                text: item.text.clone().into(),
                completed: item.completed,
                indent_level: item.indent_level as i32,
            })
            .collect()
    };

    // Set initial items
    let initial_items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(slint::VecModel::from(
        convert_items(&todo_app.todo_list.items),
    ));
    ui.set_todo_items(initial_items.clone());

    // Setup callbacks
    let ui_weak = ui.as_weak();
    let todo_app_rc = std::rc::Rc::new(std::cell::RefCell::new(todo_app));

    // Toggle completed callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_toggle_item_completed(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.toggle_item_completed(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Text changed callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_item_text_changed(move |index, text| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.update_item_text(index as usize, text.to_string());
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Indent left callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_indent_item_left(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.indent_item_left(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Indent right callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_indent_item_right(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.indent_item_right(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Delete item callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_delete_item(move |index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.delete_item(index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Add new item callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_add_new_item(move || {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.insert_new_item(0);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    // Move item callback
    {
        let todo_app_rc = todo_app_rc.clone();
        let ui_weak = ui_weak.clone();
        ui.on_move_item(move |from_index, to_index| {
            if let Ok(mut app) = todo_app_rc.try_borrow_mut() {
                let _ = app.move_item(from_index as usize, to_index as usize);
                if let Some(ui) = ui_weak.upgrade() {
                    let items: slint::ModelRc<TodoItemData> = slint::ModelRc::new(
                        slint::VecModel::from(convert_items(&app.todo_list.items)),
                    );
                    ui.set_todo_items(items);
                }
            }
        });
    }

    ui.run()?;
    Ok(())
}
