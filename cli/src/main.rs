use std::env;

use ka::{create_command, shift_command, update_command};

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args[1].as_str();
    match command {
        "create" => {
            create_command().unwrap();
        }
        "update" => {
            update_command().unwrap();
        }
        "shift" => {
            let file_path = args[2].as_str();

            let new_cursor: usize = args[3].as_str().parse().expect("Invalid cursor.");

            shift_command(file_path, new_cursor);
        }
        _ => println!("Unknown command: {}", command),
    }
}
