use crate::{actions::update, files::Locations, filesystem::Fs, history::RepositoryHistory};
use anyhow::Result;

use super::ActionOptions;

pub fn create(command_options: ActionOptions, fs: &impl Fs) -> Result<()> {
    let locations = Locations::from(&command_options);
    // FIXME: Re-add old repository deletion.
    // if locations.ka_path.exists() {
    //     fs::remove_dir_all(locations.ka_path.as_path())?;
    // }

    // fs::create_dir(&locations.ka_path)?;
    // fs::create_dir(&locations.ka_files_path)?;

    let mut index_file = fs.create_file(&locations.get_repository_index_path())?;
    let empty_history = RepositoryHistory::default();
    empty_history.write_to_file(fs, &mut index_file)?;

    update(command_options, fs)?;

    Ok(())
}
