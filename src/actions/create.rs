use std::fs::{self, File};

use anyhow::Result;

use crate::{actions::update, files::Locations, history::RepositoryHistory};

use super::ActionOptions;

pub fn create(command_options: ActionOptions) -> Result<()> {
    let locations = Locations::from(&command_options);

    if locations.ka_path.exists() {
        fs::remove_dir_all(locations.ka_path.as_path())?;
    }

    fs::create_dir(&locations.ka_path)?;
    fs::create_dir(&locations.ka_files_path)?;

    let mut index_file = File::create(locations.get_repository_index())?;
    let empty_history = RepositoryHistory::default();
    empty_history.write_to_file(&mut index_file)?;

    update(command_options)?;

    Ok(())
}
