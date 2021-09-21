use std::{
    fs,
    io::{Seek, Write},
    path::Path,
};

use anyhow::Result;

use crate::{
    files::{Locations, FileState},
    history::FileHistory,
};

use super::ActionOptions;

pub fn shift(command_options: ActionOptions, path: &str, new_cursor: usize) -> Result<()> {
    let locations = Locations::from(&command_options);

    match FileState::from_working(&locations, Path::new(path))? {
        FileState::Tracked(tracked) => {
            let mut history_file = tracked.load_history_file()?;

            let mut file_history = FileHistory::from_file(&mut history_file)?;
            file_history.set_cursor(new_cursor);

            if file_history.file_is_deleted() {
                fs::remove_file(path)?;
            } else {
                let new_content = file_history.get_content();
                let mut working_file = tracked.create_working_file()?;

                working_file.rewind()?;
                working_file.set_len(0)?;

                working_file.write_all(new_content.as_bytes())?;
            }

            file_history.write_to_file(&mut history_file)?;
        }
        FileState::Deleted(deleted) => {
            let mut history_file = deleted.load_history_file()?;

            let mut file_history = FileHistory::from_file(&mut history_file)?;
            file_history.set_cursor(new_cursor);

            if !file_history.file_is_deleted() {
                let mut new_working_file = deleted.create_working_file(&locations)?;
                let new_content = file_history.get_content();

                new_working_file.write_all(new_content.as_bytes())?;
            }
        }
        FileState::Untracked(_) => panic!("File is not tracked with Ka."),
    }

    Ok(())
}
