use std::{env, time::SystemTime};

use ka::{
    actions::{create, shift, update, ActionOptions},
    filesystem::FsImpl,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args[1].as_str();

    let options = ActionOptions::from_path("./repo");
    //let options = ActionOptions::from_pwd().expect("Could not get current path.");

    let filesystem = FsImpl {};

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Could not get current system time.")
        .as_secs();

    match command {
        "create" => {
            create(options, &filesystem, timestamp).expect("Failed executing Create action.");
        }
        "update" => {
            update(options, &filesystem, timestamp).expect("Failed executing Update action.");
        }
        "shift" => {
            let new_cursor: usize = args[2].as_str().parse().expect("Invalid cursor.");

            shift(options, &filesystem, new_cursor).expect("Failed executing Shift actions.");
        }
        _ => panic!("Unknown command: {}", command),
    }
}
