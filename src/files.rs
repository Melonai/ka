use std::path::{Path, PathBuf};

use anyhow::{Context, Error, Result};

use crate::{
    actions::ActionOptions,
    filesystem::{Fs, FsEntry},
};

pub struct Locations {
    pub repository_path: PathBuf,
    pub ka_path: PathBuf,
    pub ka_files_path: PathBuf,
}

impl Locations {
    pub fn get_repository_index_path(&self) -> PathBuf {
        return self.ka_path.join("index");
    }

    pub fn get_repository_files<FS: Fs>(&self, fs: &FS) -> Result<Vec<FileState>, Error> {
        let working_entries = fs
            .read_directory(&self.repository_path)
            .context("Failed reading working file entries.")?
            .into_iter()
            .filter(|e| e.path() != self.ka_path)
            .collect();
        let history_entries = fs
            .read_directory(&self.ka_files_path)
            .context("Failed reading history file entries.")?;

        let working_files = Self::walk_directory(fs, working_entries, &|entry| {
            FileState::from_working(fs, self, &entry.path()).ok()
        })?;

        let deleted_files = Self::walk_directory(fs, history_entries, &|entry| {
            let file_path = entry.path();
            let file = FileState::from_history(fs, self, &file_path).ok()?;
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

    fn walk_directory<FS: Fs>(
        fs: &FS,
        directory: Vec<FS::Entry>,
        filter_map: &dyn Fn(&FS::Entry) -> Option<FileState>,
    ) -> Result<Vec<FileState>> {
        let mut entries = Vec::new();

        for entry in directory {
            if entry.is_directory()? {
                let nested_directory = fs.read_directory(&entry.path())?;
                let nested_files = Self::walk_directory(fs, nested_directory, filter_map)?;
                entries.extend(nested_files);
            } else if let Some(states) = filter_map(&entry) {
                entries.push(states);
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
    pub fn from_history<FS: Fs>(
        fs: &FS,
        locations: &Locations,
        history_file_path: &Path,
    ) -> Result<Self> {
        let working_path = locations.working_from_history(history_file_path)?;
        Ok(if !fs.path_exists(&working_path) {
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

    pub fn from_working<FS: Fs>(
        fs: &FS,
        locations: &Locations,
        working_file_path: &Path,
    ) -> Result<Self> {
        let history_path = locations.history_from_working(working_file_path)?;
        // TODO: Think whether abstracting Path would be needed for Fs abstraction.
        Ok(if !fs.path_exists(&history_path) {
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

    pub fn get_working_path(&self, locations: &Locations) -> Result<PathBuf> {
        match self {
            FileState::Deleted(deleted) => locations.working_from_history(&deleted.history_path),
            FileState::Untracked(untracked) => Ok(untracked.path.clone()),
            FileState::Tracked(tracked) => Ok(tracked.working_path.clone()),
        }
    }
}

pub struct FileDeleted {
    pub history_path: PathBuf,
}

impl FileDeleted {
    pub fn load_history_file<FS: Fs>(&self, fs: &FS) -> Result<FS::File> {
        fs.open_writable_file(&self.history_path)
    }

    pub fn create_working_file<FS: Fs>(&self, fs: &FS, locations: &Locations) -> Result<FS::File> {
        let working_path = locations.working_from_history(&self.history_path)?;
        fs.create_file(&working_path)
    }
}

pub struct FileUntracked {
    pub path: PathBuf,
}

impl FileUntracked {
    pub fn load_file<FS: Fs>(&self, fs: &FS) -> Result<FS::File> {
        fs.open_readable_file(&self.path)
    }

    pub fn create_history_file<FS: Fs>(&self, fs: &FS, locations: &Locations) -> Result<FS::File> {
        let history_path = locations.history_from_working(&self.path)?;
        Ok(fs.create_file(&history_path)?)
    }
}

pub struct FileTracked {
    pub history_path: PathBuf,
    pub working_path: PathBuf,
}

impl FileTracked {
    pub fn load_history_file<FS: Fs>(&self, fs: &FS) -> Result<FS::File> {
        fs.open_writable_file(&self.history_path)
    }

    pub fn load_working_file<FS: Fs>(&self, fs: &FS) -> Result<FS::File> {
        fs.open_readable_file(&self.working_path)
    }

    pub fn create_working_file<FS: Fs>(&self, fs: &FS) -> Result<FS::File> {
        fs.create_file(&self.working_path)
    }
}
