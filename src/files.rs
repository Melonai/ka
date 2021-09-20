use std::{
    fs::{self, DirEntry, File, OpenOptions, ReadDir},
    io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Error, Result};

use crate::actions::ActionOptions;

pub struct Locations {
    pub repository_path: PathBuf,
    pub ka_path: PathBuf,
    pub ka_files_path: PathBuf,
}

impl Locations {
    pub fn get_repository_paths(&self) -> Result<Vec<RepositoryPaths>, Error> {
        let repository_entries =
            fs::read_dir(&self.repository_path).context("Failed reading tracked file entries.")?;
        let history_entries =
            fs::read_dir(&self.ka_files_path).context("Failed reading history file entries.")?;

        let tracked_paths = Self::walk_directory(repository_entries, &|entry| {
            let file_path = entry.path();
            if file_path != self.ka_path {
                RepositoryPaths::from_tracked(&self, &file_path).ok()
            } else {
                None
            }
        })?;

        let deleted_paths = Self::walk_directory(history_entries, &|entry| {
            let file_path = entry.path();
            let file = RepositoryPaths::from_history(&self, &file_path).ok()?;
            match file {
                RepositoryPaths::Deleted { .. } => Some(file),
                RepositoryPaths::Tracked { .. } => None,
                _ => unreachable!(),
            }
        })?;

        let mut all_paths = tracked_paths;
        all_paths.extend(deleted_paths);

        Ok(all_paths)
    }

    pub fn tracked_from_history(&self, history_file_path: &Path) -> Result<PathBuf> {
        let raw_path = history_file_path.strip_prefix(&self.ka_files_path)?;
        Ok(self.repository_path.join(raw_path))
    }

    pub fn history_from_tracked(&self, tracked_file_path: &Path) -> Result<PathBuf> {
        let raw_path = tracked_file_path.strip_prefix(&self.repository_path)?;
        Ok(self.ka_files_path.join(raw_path))
    }

    fn walk_directory(
        directory: ReadDir,
        filter_map: &dyn Fn(DirEntry) -> Option<RepositoryPaths>,
    ) -> Result<Vec<RepositoryPaths>> {
        let mut entries = Vec::new();

        for res in directory {
            let entry = res?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let nested_directory = fs::read_dir(entry.path())?;
                let nested_paths = Self::walk_directory(nested_directory, filter_map)?;
                entries.extend(nested_paths);
            } else {
                if let Some(paths) = filter_map(entry) {
                    entries.push(paths);
                }
            }
        }

        Ok(entries)
    }
}

impl From<&ActionOptions> for Locations {
    fn from(options: &ActionOptions) -> Self {
        let ka_path = options.repository_path.join(".ka");
        let ka_files_path = ka_path.join("files");

        Self {
            repository_path: options.repository_path.clone(),
            ka_path,
            ka_files_path,
        }
    }
}

pub enum RepositoryPaths {
    Deleted(FileDeleted),
    Untracked(FileUntracked),
    Tracked(FileTracked),
}

impl RepositoryPaths {
    pub fn from_history(locations: &Locations, history_file_path: &Path) -> Result<Self> {
        let tracked_path = locations.tracked_from_history(history_file_path)?;
        Ok(if !tracked_path.exists() {
            RepositoryPaths::Deleted(FileDeleted {
                history_path: history_file_path.to_path_buf(),
            })
        } else {
            RepositoryPaths::Tracked(FileTracked {
                history_path: history_file_path.to_path_buf(),
                tracked_path,
            })
        })
    }

    pub fn from_tracked(locations: &Locations, tracked_file_path: &Path) -> Result<Self> {
        let history_path = locations.history_from_tracked(tracked_file_path)?;
        Ok(if !history_path.exists() {
            RepositoryPaths::Untracked(FileUntracked {
                path: tracked_file_path.to_path_buf(),
            })
        } else {
            RepositoryPaths::Tracked(FileTracked {
                history_path,
                tracked_path: tracked_file_path.to_path_buf(),
            })
        })
    }
}

pub struct FileDeleted {
    history_path: PathBuf,
}

impl FileDeleted {
    pub fn load_history_file(&self) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .read(true)
            .open(&self.history_path)
    }

    pub fn create_tracked_file(&self, locations: &Locations) -> Result<File> {
        Ok(File::create(
            locations.tracked_from_history(&self.history_path)?,
        )?)
    }
}

pub struct FileUntracked {
    path: PathBuf,
}

impl FileUntracked {
    pub fn load_file(&self) -> io::Result<File> {
        OpenOptions::new().read(true).open(&self.path)
    }

    pub fn create_history_file(&self, locations: &Locations) -> Result<File> {
        let history_path = locations.history_from_tracked(&self.path)?;

        if let Some(parent_path) = history_path.parent(){
            if !parent_path.exists() {
                fs::create_dir_all(parent_path)?;
            }
        } 

        Ok(File::create(history_path)?)
    }
}

pub struct FileTracked {
    history_path: PathBuf,
    tracked_path: PathBuf,
}

impl FileTracked {
    pub fn load_history_file(&self) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .read(true)
            .open(&self.history_path)
    }

    pub fn load_tracked_file(&self) -> io::Result<File> {
        File::open(&self.tracked_path)
    }

    pub fn create_tracked_file(&self) -> io::Result<File> {
        File::create(&self.tracked_path)
    }
}
