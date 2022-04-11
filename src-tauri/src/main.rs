#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use parking_lot::Mutex;

use nu_cli::{gather_parent_env_vars, CliError};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, Span, Value, CONFIG_VARIABLE_ID,
};
use tauri::{command, Menu, MenuItem, State, Submenu};

pub struct MyState {
    engine_state: Mutex<EngineState>,
    stack: Mutex<Stack>,
}

// #[derive(Debug, serde::Serialize)]
// enum MyError {
//     #[allow(dead_code)]
//     FooError,
// }

mod nushell;

fn main() {
    let cwd = std::env::current_dir().unwrap();
    let mut engine_state = nu_command::create_default_context(cwd);
    gather_parent_env_vars(&mut engine_state);

    let mut stack = Stack::new();
    stack.vars.insert(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::new(0, 0),
        },
    );

    tauri::Builder::default()
        .manage(MyState {
            engine_state: Mutex::new(engine_state),
            stack: Mutex::new(stack),
        })
        .invoke_handler(tauri::generate_handler![simple_command_with_result])
        .menu(
            Menu::new()
                .add_submenu(Submenu::new(
                    "Nushell",
                    Menu::new()
                        .add_native_item(MenuItem::EnterFullScreen)
                        .add_native_item(MenuItem::Quit),
                ))
                .add_submenu(Submenu::new(
                    "Edit",
                    Menu::new()
                        .add_native_item(MenuItem::Copy)
                        .add_native_item(MenuItem::Paste)
                        .add_native_item(MenuItem::Cut)
                        .add_native_item(MenuItem::Undo)
                        .add_native_item(MenuItem::Redo)
                        .add_native_item(MenuItem::SelectAll),
                )),
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[command]
fn simple_command_with_result(argument: String, state: State<MyState>) -> Result<String, String> {
    let mut engine_state = state.engine_state.lock();
    let mut stack = state.stack.lock();

    let result = nushell::eval_nushell(
        &mut engine_state,
        &mut stack,
        argument.as_bytes(),
        "nana",
        PipelineData::new(Span { start: 0, end: 0 }),
    );

    let result = result.map(|x| x.into_value(Span { start: 0, end: 0 }));

    match result {
        Ok(value) => {
            let output = serde_json::to_string(&value);

            match output {
                Ok(s) => Ok(s),
                Err(e) => Ok(format!("\"{}\"", e)),
            }
        }
        Err(e) => {
            use strip_ansi_escapes::strip;

            let working_set = StateWorkingSet::new(&engine_state);

            let error_msg = format!("{:?}", CliError(&e, &working_set));

            let result = strip(error_msg).expect("internal error: can't strip ansi from error");
            Err(String::from_utf8_lossy(&result).to_string())
        }
    }
}