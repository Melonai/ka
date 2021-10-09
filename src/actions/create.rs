use crate::{actions::update, files::Locations, filesystem::Fs, history::RepositoryHistory};
use anyhow::Result;

use super::ActionOptions;

pub fn create(command_options: ActionOptions, fs: &impl Fs, timestamp: u64) -> Result<()> {
    let locations = Locations::from(&command_options);

    if fs.path_exists(&locations.ka_path) {
        fs.delete_directory(&locations.ka_path)?;
    }

    fs.create_directory(&locations.ka_path)?;
    fs.create_directory(&locations.ka_files_path)?;

    let mut index_file = fs.create_file(&locations.get_repository_index_path())?;
    let empty_history = RepositoryHistory::default();
    empty_history.write_to_file(fs, &mut index_file)?;

    update(command_options, fs, timestamp)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{actions::ActionOptions, diff::ContentChange, filesystem::mock::{EntryMock, FsMock, FsState}, history::{FileChange, FileChangeVariant, FileHistory, RepositoryChange, RepositoryHistory}};

    use super::create;

    #[test]
    fn create_empty() {
        let now = 0xC0FFEE;
        let fs_mock = FsMock::new();
        let options = ActionOptions::from_path(".");

        let expected_index = RepositoryHistory::default().encode().unwrap();
        
        create(options, &fs_mock, now).expect("Action failed.");

        fs_mock.assert_match(FsState::new(vec![
            EntryMock::dir("./.ka"),
            EntryMock::file("./.ka/index", &expected_index),
            EntryMock::dir("./.ka/files"),
        ]));
    }

    #[test]
    fn create_basic() {
        let now = 0xC0FFEE;
        let mut fs_mock = FsMock::new();
        let options = ActionOptions::from_path(".");

        let expected_index = {
            let mut history = RepositoryHistory::default();
            history.add_change(RepositoryChange {
                affected_files: vec![Path::new("./test").into()],
                timestamp: now,
            });
            history.cursor = 1;
            history.encode().unwrap()
        };

        let expected_file_history = {
            let mut history = FileHistory::default();
            let change = ContentChange::Inserted {
                at: 0,
                new_content: vec![1, 2, 3],
            };
            history.add_change(FileChange {
                change_index: 1,
                variant: FileChangeVariant::Updated(vec![change]),
            });
            history.encode().unwrap()
        };

        fs_mock.set_state(FsState::new(vec![
            EntryMock::file("./test", &vec![1, 2, 3])
        ]));

        create(options, &fs_mock, now).expect("Action failed.");

        fs_mock.assert_match(FsState::new(vec![
            EntryMock::file("./test", &vec![1, 2, 3]),

            EntryMock::dir("./.ka"),
            EntryMock::file("./.ka/index", &expected_index),
            EntryMock::dir("./.ka/files"),
            EntryMock::file("./.ka/files/test", &expected_file_history),
        ]))
    }
}