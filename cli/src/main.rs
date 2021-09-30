use std::env;

use ka::actions::{create, shift, update, ActionOptions};

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args[1].as_str();

    let options = ActionOptions::from_path("./repo");
    //let options = ActionOptions::from_pwd().expect("Could not get current path.");

    match command {
        "create" => {
            create(options).expect("Failed executing Create action.");
        }
        "update" => {
            update(options).expect("Failed executing Update action.");
        }
        "shift" => {
            let new_cursor: usize = args[2].as_str().parse().expect("Invalid cursor.");

            shift(options, new_cursor).expect("Failed executing Shift actions.");
        }
        _ => panic!("Unknown command: {}", command),
    }
}
