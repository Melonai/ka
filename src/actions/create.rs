use std::{fs, io};

use crate::{actions::update, files::Locations};

use super::ActionOptions;

pub fn create(command_options: ActionOptions) -> io::Result<()> {
    let locations = Locations::from(&command_options);

    if locations.ka_path.exists() {
        fs::remove_dir_all(locations.ka_path.as_path())?;
    }

    fs::create_dir(locations.ka_path)?;
    fs::create_dir(locations.ka_files_path)?;

    update(command_options)?;

    Ok(())
}
