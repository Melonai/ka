use std::{
    fs::{File, OpenOptions},
    io::Read,
    time::SystemTime,
};

use anyhow::{Context, Result};

use crate::{
    diff::ContentChange,
    files::{FileState, Locations},
    history::{FileChange, FileChangeVariant, FileHistory, RepositoryChange, RepositoryHistory},
};

use super::ActionOptions;

pub fn update(command_options: ActionOptions) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let locations = Locations::from(&command_options);

    let repository_index_path = locations.get_repository_index();
    let mut repository_index_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(repository_index_path)?;
    let mut repository_history = RepositoryHistory::from_file(&mut repository_index_file)?;

    let entries = locations
        .get_repository_files()
        .context("Could not traverse files.")?;

    let mut affected_files = Vec::new();

    for state in entries {
        let changed_file = get_new_history_for_file(repository_history.cursor, &state, &locations)?;
        if let Some((mut history_file, new_file_history)) = changed_file {
            new_file_history.write_to_file(&mut history_file)?;
            affected_files.push(state.get_working_path(&locations)?);
        }
    }

    if affected_files.len() > 0 {
        repository_history.add_change(RepositoryChange {
            affected_files,
            timestamp,
        });
        repository_history.cursor += 1;

        repository_history.write_to_file(&mut repository_index_file)?;
    }

    Ok(())
}

fn get_new_history_for_file(
    cursor: usize,
    file_state: &FileState,
    locations: &Locations,
) -> Result<Option<(File, FileHistory)>> {
    match file_state {
        FileState::Deleted(deleted) => {
            let mut history_file = deleted.load_history_file()?;
            let file_history = FileHistory::from_file(&mut history_file)?;
            if !file_history.is_file_deleted(cursor) {
                let mut new_history = file_history;
                new_history.add_change(FileChange {
                    change_index: cursor + 1,
                    variant: FileChangeVariant::Deleted,
                });
                Ok(Some((history_file, new_history)))
            } else {
                Ok(None)
            }
        }
        FileState::Untracked(untracked) => {
            let mut file = untracked.load_file()?;

            let mut file_content = Vec::new();
            file.read_to_end(&mut file_content)?;

            let change = FileChange {
                change_index: cursor + 1,
                variant: FileChangeVariant::Updated(vec![ContentChange::Inserted {
                    at: 0,
                    new_content: file_content,
                }]),
            };

            let mut new_history = FileHistory::default();
            new_history.add_change(change);

            Ok(Some((
                untracked.create_history_file(locations)?,
                new_history,
            )))
        }
        FileState::Tracked(tracked) => {
            let mut history_file = tracked.load_history_file()?;
            let mut working_file = tracked.load_working_file()?;

            let file_history = FileHistory::from_file(&mut history_file)?;

            let mut new_content = Vec::new();
            working_file.read_to_end(&mut new_content)?;

            let old_content = file_history.get_content(cursor);

            let changes = ContentChange::diff(&old_content, &new_content);

            // TODO: Check if file was changed.

            let mut new_history = file_history;
            new_history.add_change(FileChange {
                change_index: cursor + 1,
                variant: FileChangeVariant::Updated(changes),
            });

            Ok(Some((history_file, new_history)))
        }
    }
}
