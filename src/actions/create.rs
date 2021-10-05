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
    use std::{path::Path, vec};

    use crate::{
        actions::{create, ActionOptions},
        diff::ContentChange,
        filesystem::mock::{EntryMock, ExpectedCall as Call, ExpectedCallVariant as Type, FsMock},
        history::{
            FileChange, FileChangeVariant, FileHistory, RepositoryChange, RepositoryHistory,
        },
    };

    #[test]
    fn create_empty() {
        let now = 0xC0FFEE;
        let fs_mock = FsMock::new();
        let options = ActionOptions::from_path(".");

        let pwd = Path::new(".");
        let working_file = Path::new("./test");
        let history_file = Path::new("./.ka/files/test");

        let ka = Path::new("./.ka");
        let ka_files = Path::new("./.ka/files");
        let ka_index = Path::new("./.ka/index");

        let empty_index = RepositoryHistory::default().encode().unwrap();
        let expected_index = {
            let mut history = RepositoryHistory::default();
            history.add_change(RepositoryChange {
                affected_files: vec![working_file.to_path_buf()],
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

        fs_mock.set_expected_calls(vec![
            // Create calls
            Call::new(ka, Type::PathExists(false)),
            Call::new(ka, Type::CreateDirectory),
            Call::new(ka_files, Type::CreateDirectory),
            Call::new(ka_index, Type::CreateFile),
            Call::new(
                ka_index,
                Type::WriteToFile {
                    expected: empty_index.clone(),
                },
            ),
            // Update calls
            Call::new(ka_index, Type::OpenWritableFile),
            Call::new(
                ka_index,
                Type::ReadFile {
                    returned: empty_index,
                },
            ),
            Call::new(
                pwd,
                Type::ReadDirectory {
                    returned: vec![
                        EntryMock::new(working_file, false),
                        EntryMock::new(ka, true),
                    ],
                },
            ),
            Call::new(ka_files, Type::ReadDirectory { returned: vec![] }),
            Call::new(history_file, Type::PathExists(false)),
            Call::new(working_file, Type::OpenReadableFile),
            Call::new(
                working_file,
                Type::ReadFile {
                    returned: vec![1, 2, 3],
                },
            ),
            Call::new(history_file, Type::CreateFile),
            Call::new(
                history_file,
                Type::WriteToFile {
                    expected: expected_file_history,
                },
            ),
            Call::new(
                ka_index,
                Type::WriteToFile {
                    expected: expected_index,
                },
            ),
        ]);

        create(options, &fs_mock, now).expect("Action failed.");

        fs_mock.assert_calls();
    }
}
