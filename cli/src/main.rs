use std::env;

use ka::actions::{create, shift, update, ActionOptions};

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args[1].as_str();

    let options = ActionOptions::from_pwd().expect("Could not get current path.");

    match command {
        "create" => {
            create(options).unwrap();
        }
        "update" => {
            update(options).unwrap();
        }
        "shift" => {
            let file_path = args[2].as_str();

            let new_cursor: usize = args[3].as_str().parse().expect("Invalid cursor.");

            shift(options, file_path, new_cursor).unwrap();
        }
        _ => println!("Unknown command: {}", command),
    }
}
