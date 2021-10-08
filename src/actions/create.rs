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
