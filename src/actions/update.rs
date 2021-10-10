use anyhow::{Context, Result};

use crate::{
    diff::ContentChange,
    files::{FileState, Locations},
    filesystem::Fs,
    history::{FileChange, FileChangeVariant, FileHistory, RepositoryChange, RepositoryHistory},
};

use super::ActionOptions;

pub fn update(command_options: ActionOptions, fs: &impl Fs, timestamp: u64) -> Result<()> {
    let locations = Locations::from(&command_options);

    let repository_index_path = locations.get_repository_index_path();
    let mut repository_index_file = fs.open_writable_file(&repository_index_path)?;
    let mut repository_history = RepositoryHistory::from_file(fs, &mut repository_index_file)?;

    let entries = locations
        .get_repository_files(fs)
        .context("Could not traverse files.")?;

    let mut affected_files = Vec::new();

    for state in entries {
        let changed_file =
            get_new_history_for_file(fs, repository_history.cursor, &state, &locations)?;
        if let Some((mut history_file, new_file_history)) = changed_file {
            new_file_history.write_to_file(fs, &mut history_file)?;
            affected_files.push(state.get_working_path(&locations)?);
        }
    }

    if !affected_files.is_empty() {
        repository_history.add_change(RepositoryChange {
            affected_files,
            timestamp,
        });
        repository_history.cursor += 1;

        repository_history.write_to_file(fs, &mut repository_index_file)?;
    }

    Ok(())
}

fn get_new_history_for_file<FS: Fs>(
    fs: &FS,
    cursor: usize,
    file_state: &FileState,
    locations: &Locations,
) -> Result<Option<(FS::File, FileHistory)>> {
    match file_state {
        FileState::Deleted(deleted) => {
            let mut history_file = deleted.load_history_file(fs)?;
            let file_history = FileHistory::from_file(fs, &mut history_file)?;
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
            let mut file = untracked.load_file(fs)?;

            let file_content = fs.read_from_file(&mut file)?;

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
                untracked.create_history_file(fs, locations)?,
                new_history,
            )))
        }
        FileState::Tracked(tracked) => {
            let mut history_file = tracked.load_history_file(fs)?;
            let mut working_file = tracked.load_working_file(fs)?;

            let file_history = FileHistory::from_file(fs, &mut history_file)?;

            let new_content = fs.read_from_file(&mut working_file)?;
            let old_content = file_history.get_content(cursor);

            let changes = ContentChange::diff(&old_content, &new_content);

            if !changes.is_empty() {
                let mut new_history = file_history;
                new_history.add_change(FileChange {
                    change_index: cursor + 1,
                    variant: FileChangeVariant::Updated(changes),
                });

                Ok(Some((history_file, new_history)))
            } else {
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        actions::{create, update, ActionOptions},
        diff::ContentChange,
        filesystem::mock::{EntryMock, FsMock, FsState},
        history::{
            FileChange, FileChangeVariant, FileHistory, RepositoryChange, RepositoryHistory,
        },
    };

    #[test]
    fn no_update_if_no_change() {
        let now = 0xC0FFEE;
        let mut fs_mock = FsMock::new();

        fs_mock.set_state(FsState::new(vec![EntryMock::file("./test", &[1, 2, 3])]));

        // We create the initial Fs state by running the Create action.
        create(ActionOptions::from_path("."), &fs_mock, now)
            .expect("Creating expected state failed.");
        let state = fs_mock.get_state();

        update(ActionOptions::from_path("."), &fs_mock, now + 1).expect("Action failed.");

        // No change should have happened.
        fs_mock.assert_match(state);
    }

    #[test]
    fn selective_update() {
        let now = 0xC0FFEE;
        let mut fs_mock = FsMock::new();
        let options = ActionOptions::from_path(".");

        let mut repo_history = RepositoryHistory::default();

        repo_history.add_change(RepositoryChange {
            affected_files: vec![
                Path::new("./changed_file").into(),
                Path::new("./unchanged_file").into(),
            ],
            timestamp: now,
        });
        repo_history.cursor = 1;
        let initial_index = repo_history.encode().unwrap();

        repo_history.add_change(RepositoryChange {
            affected_files: vec![Path::new("./changed_file").into()],
            timestamp: now + 1,
        });
        repo_history.cursor = 2;
        let updated_index = repo_history.encode().unwrap();

        let mut file_history = FileHistory::default();

        file_history.add_change(FileChange {
            change_index: 1,
            variant: FileChangeVariant::Updated(vec![ContentChange::Inserted {
                at: 0,
                new_content: vec![1, 2, 3],
            }]),
        });
        let initial_file_history = file_history.encode().unwrap();

        file_history.add_change(FileChange {
            change_index: 2,
            variant: FileChangeVariant::Updated(vec![ContentChange::Inserted {
                at: 3,
                new_content: vec![4, 5],
            }]),
        });
        let updated_file_history = file_history.encode().unwrap();

        fs_mock.set_state(FsState::new(vec![
            EntryMock::file("./changed_file", &[1, 2, 3, 4, 5]),
            EntryMock::file("./unchanged_file", &[1, 2, 3]),

            EntryMock::dir("./.ka"),
            EntryMock::file("./.ka/index", &initial_index),
            EntryMock::dir("./.ka/files"),
            EntryMock::file("./.ka/files/changed_file", &initial_file_history),
            EntryMock::file("./.ka/files/unchanged_file", &initial_file_history),
        ]));

        update(options, &fs_mock, now + 1).expect("Action failed.");

        fs_mock.assert_match(FsState::new(vec![
            EntryMock::file("./changed_file", &[1, 2, 3, 4, 5]),
            EntryMock::file("./unchanged_file", &[1, 2, 3]),

            EntryMock::dir("./.ka"),
            EntryMock::file("./.ka/index", &updated_index),
            EntryMock::dir("./.ka/files"),
            EntryMock::file("./.ka/files/changed_file", &updated_file_history),
            EntryMock::file("./.ka/files/unchanged_file", &initial_file_history),
        ]))
    }
}
