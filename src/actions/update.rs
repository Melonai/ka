use std::io::{self, Read};

use crate::{
    files::{Locations, RepositoryPaths},
    history::{FileChange, FileHistory},
    text_diff::TextChange,
};

use super::ActionOptions;

pub fn update(command_options: ActionOptions) -> io::Result<()> {
    let locations = Locations::from(&command_options);

    let entries = locations
        .get_repository_paths()
        .expect("Could not traverse files.");

    for element in entries {
        update_file(element, &locations)?;
    }

    Ok(())
}

fn update_file(paths: RepositoryPaths, locations: &Locations) -> io::Result<()> {
    let new_state = match paths {
        RepositoryPaths::Deleted(deleted) => {
            let mut history_file = deleted.load_history_file()?;
            let file_history = FileHistory::from_file(&mut history_file).unwrap();
            if !file_history.file_is_deleted() {
                let mut new_history = file_history;
                new_history.add_change(FileChange::Deleted);
                Some((history_file, new_history))
            } else {
                None
            }
        }
        RepositoryPaths::Untracked(untracked) => {
            let mut file = untracked.load_file()?;

            let mut file_content = String::new();
            file.read_to_string(&mut file_content)?;

            let change = FileChange::Updated(vec![TextChange::Inserted {
                at: 0,
                new_content: file_content,
            }]);

            let mut new_history = FileHistory::default();
            new_history.add_change(change);

            Some((untracked.create_history_file(locations)?, new_history))
        }
        RepositoryPaths::Tracked(tracked) => {
            let mut history_file = tracked.load_history_file()?;
            let mut tracked_file = tracked.load_tracked_file()?;

            let file_history = FileHistory::from_file(&mut history_file).unwrap();

            let mut new_content = String::new();
            tracked_file.read_to_string(&mut new_content)?;

            let old_content = file_history.get_content();

            let changes = TextChange::diff(&old_content, &new_content);

            let mut new_history = file_history;
            new_history.add_change(FileChange::Updated(changes));
            new_history.set_cursor(new_history.cursor() + 1);

            Some((history_file, new_history))
        }
    };

    if let Some((mut history_file, new_file_history)) = new_state {
        new_file_history.write_to_file(&mut history_file)?;
    }

    Ok(())
}
