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
    pub fn get_repository_files(&self) -> Result<Vec<FileState>, Error> {
        let working_entries =
            fs::read_dir(&self.repository_path).context("Failed reading working file entries.")?;
        let history_entries =
            fs::read_dir(&self.ka_files_path).context("Failed reading history file entries.")?;

        let working_files = Self::walk_directory(working_entries, &|entry| {
            let file_path = entry.path();
            if file_path != self.ka_path {
                FileState::from_working(&self, &file_path).ok()
            } else {
                None
            }
        })?;

        let deleted_files = Self::walk_directory(history_entries, &|entry| {
            let file_path = entry.path();
            let file = FileState::from_history(&self, &file_path).ok()?;
            match file {
                FileState::Deleted { .. } => Some(file),
                FileState::Tracked { .. } => None,
                _ => unreachable!(),
            }
        })?;

        let mut all_files = working_files;
        all_files.extend(deleted_files);

        Ok(all_files)
    }

    pub fn working_from_history(&self, history_file_path: &Path) -> Result<PathBuf> {
        let raw_path = history_file_path.strip_prefix(&self.ka_files_path)?;
        Ok(self.repository_path.join(raw_path))
    }

    pub fn history_from_working(&self, working_file_path: &Path) -> Result<PathBuf> {
        let raw_path = working_file_path.strip_prefix(&self.repository_path)?;
        Ok(self.ka_files_path.join(raw_path))
    }

    fn walk_directory(
        directory: ReadDir,
        filter_map: &dyn Fn(DirEntry) -> Option<FileState>,
    ) -> Result<Vec<FileState>> {
        let mut entries = Vec::new();

        for res in directory {
            let entry = res?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let nested_directory = fs::read_dir(entry.path())?;
                let nested_files = Self::walk_directory(nested_directory, filter_map)?;
                entries.extend(nested_files);
            } else {
                if let Some(states) = filter_map(entry) {
                    entries.push(states);
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

pub enum FileState {
    Deleted(FileDeleted),
    Untracked(FileUntracked),
    Tracked(FileTracked),
}

impl FileState {
    pub fn from_history(locations: &Locations, history_file_path: &Path) -> Result<Self> {
        let working_path = locations.working_from_history(history_file_path)?;
        Ok(if !working_path.exists() {
            FileState::Deleted(FileDeleted {
                history_path: history_file_path.to_path_buf(),
            })
        } else {
            FileState::Tracked(FileTracked {
                history_path: history_file_path.to_path_buf(),
                working_path,
            })
        })
    }

    pub fn from_working(locations: &Locations, working_file_path: &Path) -> Result<Self> {
        let history_path = locations.history_from_working(working_file_path)?;
        Ok(if !history_path.exists() {
            FileState::Untracked(FileUntracked {
                path: working_file_path.to_path_buf(),
            })
        } else {
            FileState::Tracked(FileTracked {
                history_path,
                working_path: working_file_path.to_path_buf(),
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

    pub fn create_working_file(&self, locations: &Locations) -> Result<File> {
        Ok(File::create(
            locations.working_from_history(&self.history_path)?,
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
        let history_path = locations.history_from_working(&self.path)?;

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
    working_path: PathBuf,
}

impl FileTracked {
    pub fn load_history_file(&self) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .read(true)
            .open(&self.history_path)
    }

    pub fn load_working_file(&self) -> io::Result<File> {
        File::open(&self.working_path)
    }

    pub fn create_working_file(&self) -> io::Result<File> {
        File::create(&self.working_path)
    }
}
