use std::collections::HashSet;

use anyhow::Result;

use crate::{
    files::{FileState, Locations},
    filesystem::Fs,
    history::{FileHistory, RepositoryHistory},
};

use super::ActionOptions;

pub fn shift(command_options: ActionOptions, fs: &impl Fs, new_cursor: usize) -> Result<()> {
    let locations = Locations::from(&command_options);

    let repository_index_path = locations.get_repository_index_path();
    let mut repository_index_file = fs.open_writable_file(&repository_index_path)?;
    let mut repository_history = RepositoryHistory::from_file(fs, &mut repository_index_file)?;

    let old_cursor = repository_history.cursor;

    repository_history.cursor = new_cursor;
    repository_history.write_to_file(fs, &mut repository_index_file)?;

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
        .map(|path| FileState::from_working(fs, &locations, path))
        .collect();

    for state in affected_files_by_shift? {
        match state {
            FileState::Tracked(tracked) => {
                let mut history_file = tracked.load_history_file(fs)?;

                let file_history = FileHistory::from_file(fs, &mut history_file)?;

                if file_history.is_file_deleted(new_cursor) {
                    fs.delete_file(&tracked.working_path)?;
                } else {
                    let new_content = file_history.get_content(new_cursor);
                    let mut working_file = tracked.create_working_file(fs)?;
                    fs.write_to_file(&mut working_file, new_content)?;
                }
            }
            FileState::Deleted(deleted) => {
                let mut history_file = deleted.load_history_file(fs)?;

                let file_history = FileHistory::from_file(fs, &mut history_file)?;

                if !file_history.is_file_deleted(new_cursor) {
                    let mut new_working_file = deleted.create_working_file(fs, &locations)?;
                    let new_content = file_history.get_content(new_cursor);
                    fs.write_to_file(&mut new_working_file, new_content)?;
                }
            }
            // TODO: What do we do with untracked files on a shift? Delete them?
            _ => (),
        }
    }

    Ok(())
}
