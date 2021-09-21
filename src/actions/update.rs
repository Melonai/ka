use std::io::Read;

use anyhow::{Context, Result};

use crate::{
    files::{Locations, FileState},
    history::{FileChange, FileHistory},
    diff::ContentChange,
};

use super::ActionOptions;

pub fn update(command_options: ActionOptions) -> Result<()> {
    let locations = Locations::from(&command_options);

    let entries = locations
        .get_repository_files()
        .context("Could not traverse files.")?;

    for element in entries {
        update_file(element, &locations)?;
    }

    Ok(())
}

fn update_file(file_state: FileState, locations: &Locations) -> Result<()> {
    let new_state = match file_state {
        FileState::Deleted(deleted) => {
            let mut history_file = deleted.load_history_file()?;
            let file_history = FileHistory::from_file(&mut history_file)?;
            if !file_history.file_is_deleted() {
                let mut new_history = file_history;
                new_history.add_change(FileChange::Deleted);
                Some((history_file, new_history))
            } else {
                None
            }
        }
        FileState::Untracked(untracked) => {
            let mut file = untracked.load_file()?;

            let mut file_content = Vec::new();
            file.read_to_end(&mut file_content)?;

            let change = FileChange::Updated(vec![ContentChange::Inserted {
                at: 0,
                new_content: file_content,
            }]);

            let mut new_history = FileHistory::default();
            new_history.add_change(change);

            Some((untracked.create_history_file(locations)?, new_history))
        }
        FileState::Tracked(tracked) => {
            let mut history_file = tracked.load_history_file()?;
            let mut working_file = tracked.load_working_file()?;

            let file_history = FileHistory::from_file(&mut history_file)?;

            let mut new_content = Vec::new();
            working_file.read_to_end(&mut new_content)?;

            let old_content = file_history.get_content();

            let changes = ContentChange::diff(&old_content, &new_content);

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
