use std::{
    fs,
    io::{self, Seek, Write},
    path::Path,
};

use crate::{
    files::{Locations, RepositoryPaths},
    history::FileHistory,
};

use super::ActionOptions;

pub fn shift(command_options: ActionOptions, path: &str, new_cursor: usize) -> io::Result<()> {
    let locations = Locations::from(&command_options);

    match RepositoryPaths::from_tracked(&locations, Path::new(path)) {
        RepositoryPaths::Tracked(tracked) => {
            let mut history_file = tracked.load_history_file()?;

            let mut file_history = FileHistory::from_file(&mut history_file).unwrap();
            file_history.set_cursor(new_cursor);

            if file_history.file_is_deleted() {
                fs::remove_file(path)?;
            } else {
                let new_content = file_history.get_content();
                let mut tracked_file = tracked.create_tracked_file()?;

                tracked_file.rewind()?;
                tracked_file.set_len(0)?;

                tracked_file.write_all(new_content.as_bytes())?;
            }

            file_history.write_to_file(&mut history_file)?;
        }
        RepositoryPaths::Deleted(deleted) => {
            let mut history_file = deleted.load_history_file()?;

            let mut file_history = FileHistory::from_file(&mut history_file).unwrap();
            file_history.set_cursor(new_cursor);

            if !file_history.file_is_deleted() {
                let mut new_tracked_file = deleted.create_tracked_file(&locations)?;
                let new_content = file_history.get_content();

                new_tracked_file.write_all(new_content.as_bytes())?;
            }
        }
        RepositoryPaths::Untracked(_) => panic!("File is not tracked with Ka."),
    }

    Ok(())
}
