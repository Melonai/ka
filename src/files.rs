use std::{
    fs::{self, DirEntry, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use crate::actions::ActionOptions;

pub struct Locations {
    pub repository_path: PathBuf,
    pub ka_path: PathBuf,
    pub ka_files_path: PathBuf,
}

impl Locations {
    pub fn get_repository_paths(&self) -> Result<Vec<RepositoryPaths>, ()> {
        let repository_entries = fs::read_dir(&self.repository_path).map_err(|_| ())?;
        let history_entries = fs::read_dir(&self.ka_files_path).map_err(|_| ())?;

        let tracked_paths = repository_entries
            .map(Result::unwrap)
            .filter(|entry| entry.path() != self.ka_path)
            .flat_map(Self::flatten_directories)
            .map(|entry| {
                let path = entry.path();
                RepositoryPaths::from_tracked(&self, &path)
            });

        let deleted_paths = history_entries
            .map(Result::unwrap)
            .flat_map(Self::flatten_directories)
            .filter_map(|entry| {
                let path = entry.path();
                let file = RepositoryPaths::from_history(&self, &path);
                match file {
                    RepositoryPaths::Deleted { .. } => Some(file),
                    RepositoryPaths::Tracked { .. } => None,
                    _ => unreachable!(),
                }
            });

        let all_paths = tracked_paths.chain(deleted_paths);

        Ok(all_paths.collect())
    }

    pub fn tracked_from_history(&self, history_file_path: &Path) -> PathBuf {
        self.repository_path
            .join(history_file_path.strip_prefix(&self.ka_files_path).unwrap())
    }

    pub fn history_from_tracked(&self, tracked_file_path: &Path) -> PathBuf {
        self.ka_files_path.join(
            tracked_file_path
                .strip_prefix(&self.repository_path)
                .unwrap(),
        )
    }

    fn flatten_directories(entry: DirEntry) -> Vec<DirEntry> {
        let file_type = entry.file_type().expect("Could not read a file type.");
        if file_type.is_dir() {
            fs::read_dir(entry.path())
                .expect("Could not read nested directory.")
                .map(Result::unwrap)
                .flat_map(Self::flatten_directories)
                .collect()
        } else {
            vec![entry]
        }
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
    pub fn from_history(locations: &Locations, history_file_path: &Path) -> Self {
        let tracked_path = locations.tracked_from_history(history_file_path);
        if !tracked_path.exists() {
            RepositoryPaths::Deleted(FileDeleted {
                history_path: history_file_path.to_path_buf(),
            })
        } else {
            RepositoryPaths::Tracked(FileTracked {
                history_path: history_file_path.to_path_buf(),
                tracked_path,
            })
        }
    }

    pub fn from_tracked(locations: &Locations, tracked_file_path: &Path) -> Self {
        let history_path = locations.history_from_tracked(tracked_file_path);
        if !history_path.exists() {
            RepositoryPaths::Untracked(FileUntracked {
                path: tracked_file_path.to_path_buf(),
            })
        } else {
            RepositoryPaths::Tracked(FileTracked {
                history_path,
                tracked_path: tracked_file_path.to_path_buf(),
            })
        }
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

    pub fn create_tracked_file(&self, locations: &Locations) -> io::Result<File> {
        File::create(locations.tracked_from_history(&self.history_path))
    }
}

pub struct FileUntracked {
    path: PathBuf,
}

impl FileUntracked {
    pub fn load_file(&self) -> io::Result<File> {
        OpenOptions::new().read(true).open(&self.path)
    }

    pub fn create_history_file(&self, locations: &Locations) -> io::Result<File> {
        let history_path = locations.history_from_tracked(&self.path);

        history_path.parent().map(|dir_path| {
            if !dir_path.exists() {
                fs::create_dir_all(dir_path).unwrap();
            }
        });

        File::create(history_path)
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
