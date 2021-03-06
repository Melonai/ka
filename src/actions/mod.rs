mod create;
mod shift;
mod update;

use std::path::{Path, PathBuf};

use anyhow::Result;
pub use create::create;
pub use shift::shift;
pub use update::update;

pub struct ActionOptions {
    pub repository_path: PathBuf,
}

impl ActionOptions {
    pub fn from_path(path: &str) -> Self {
        ActionOptions {
            repository_path: Path::new(path).to_path_buf(),
        }
    }

    pub fn from_pwd() -> Result<Self> {
        let repository_path = std::env::current_dir()?;
        Ok(ActionOptions { repository_path })
    }
}
