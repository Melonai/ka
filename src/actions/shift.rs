use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{Seek, Write},
};

use anyhow::Result;

use crate::{
    files::{FileState, Locations},
    history::{FileHistory, RepositoryHistory},
};

use super::ActionOptions;

pub fn shift(command_options: ActionOptions, new_cursor: usize) -> Result<()> {
    let locations = Locations::from(&command_options);

    let repository_index_path = locations.get_repository_index();
    let mut repository_index_file = OpenOptions::new().write(true).open(repository_index_path)?;
    let mut repository_history = RepositoryHistory::from_file(&mut repository_index_file)?;

    let old_cursor = repository_history.cursor;

    repository_history.cursor = new_cursor;
    repository_history.write_to_file(&mut repository_index_file)?;

    let changes_between_cursors = if old_cursor < new_cursor {
        old_cursor..new_cursor
    } else {
        new_cursor..old_cursor
    };

    let affected_files_by_shift: Result<Vec<FileState>> = repository_history.get_changes()
        [changes_between_cursors]
        .iter()
        .fold(HashSet::new(), |mut acc, change| {
            for path in change.affected_files.iter() {
                acc.insert(path);
            }
            acc
        })
        .iter()
        .map(|path| FileState::from_working(&locations, path))
        .collect();

    for state in affected_files_by_shift? {
        match state {
            FileState::Tracked(tracked) => {
                let mut history_file = tracked.load_history_file()?;

                let file_history = FileHistory::from_file(&mut history_file)?;

                if file_history.is_file_deleted(new_cursor) {
                    fs::remove_file(tracked.working_path)?;
                } else {
                    let new_content = file_history.get_content(new_cursor);
                    let mut working_file = tracked.create_working_file()?;

                    working_file.rewind()?;
                    working_file.set_len(0)?;

                    working_file.write_all(&new_content)?;
                }
            }
            FileState::Deleted(deleted) => {
                let mut history_file = deleted.load_history_file()?;

                let file_history = FileHistory::from_file(&mut history_file)?;

                if !file_history.is_file_deleted(new_cursor) {
                    let mut new_working_file = deleted.create_working_file(&locations)?;
                    let new_content = file_history.get_content(new_cursor);

                    new_working_file.write_all(&new_content)?;
                }
            }
            // TODO: What do we do with untracked files on a shift? Delete them?
            _ => (),
        }
    }

    Ok(())
}
