use anyhow::{Context, Result};
use std::{
    fs::{self, DirEntry, File, OpenOptions},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

pub trait Fs {
    type File;
    type Entry: FsEntry;

    fn create_file(&self, path: &Path) -> Result<Self::File>;
    fn delete_file(&self, path: &Path) -> Result<()>;
    fn open_readable_file(&self, path: &Path) -> Result<Self::File>;
    fn open_writable_file(&self, path: &Path) -> Result<Self::File>;

    fn create_directory(&self, path: &Path) -> Result<()>;
    fn read_directory(&self, path: &Path) -> Result<Vec<Self::Entry>>;
    fn delete_directory(&self, path: &Path) -> Result<()>;

    fn write_to_file(&self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()>;
    fn read_from_file(&self, file: &mut Self::File) -> Result<Vec<u8>>;

    fn path_exists(&self, path: &Path) -> bool;
}

pub trait FsEntry {
    fn path(&self) -> PathBuf;
    fn is_directory(&self) -> Result<bool>;
}

pub struct FsImpl {}

impl Fs for FsImpl {
    type File = File;
    type Entry = DirEntry;

    fn create_file(&self, path: &Path) -> Result<Self::File> {
        if let Some(parent_path) = path.parent() {
            if !parent_path.exists() {
                fs::create_dir_all(parent_path)?;
            }
        }

        OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Failed creating '{}'.", path.display()))
    }

    fn delete_file(&self, path: &Path) -> Result<()> {
        fs::remove_file(path)?;
        Ok(())
    }

    fn open_readable_file(&self, path: &Path) -> Result<Self::File> {
        File::open(path)
            .with_context(|| format!("Failed opening '{}' for reading.", path.display()))
    }

    fn open_writable_file(&self, path: &Path) -> Result<Self::File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| {
                format!(
                    "Failed opening '{}' for reading and writing.",
                    path.display()
                )
            })
    }

    fn create_directory(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed creating directory '{}'.", path.display()))
    }

    fn read_directory(&self, path: &Path) -> Result<Vec<Self::Entry>> {
        let result: io::Result<_> = fs::read_dir(path)?.collect();
        result.with_context(|| format!("Failed reading directory {}", path.display()))
    }

    fn delete_directory(&self, path: &Path) -> Result<()> {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed deleting directory '{}'.", path.display()))
    }

    fn write_to_file(&self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
        file.rewind()?;
        file.set_len(0)?;
        file.write_all(&buffer)?;
        Ok(())
    }

    fn read_from_file(&self, file: &mut Self::File) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

impl FsEntry for DirEntry {
    fn path(&self) -> PathBuf {
        self.path()
    }

    fn is_directory(&self) -> Result<bool> {
        let file_type = self.file_type()?;
        Ok(file_type.is_dir())
    }
}

// TODO: This will be used for tests. Write them.
#[allow(dead_code)]
#[cfg(test)]
pub mod mock {
    use anyhow::{anyhow, Result};
    use std::{
        collections::{hash_map, HashMap, HashSet},
        path::{Path, PathBuf},
        sync::{Arc, Mutex, MutexGuard},
    };

    use super::{Fs, FsEntry};

    pub struct FsMock {
        state: Arc<Mutex<FsState>>,
    }

    impl FsMock {
        pub fn new() -> Self {
            let state = FsState {
                entries: HashMap::new(),
            };

            FsMock {
                state: Arc::new(Mutex::new(state)),
            }
        }

        pub fn set_state(&mut self, new_state: FsState) {
            let mut state = self.state.lock().expect("FsMock state lock poisoned.");
            *state = new_state;
        }

        pub fn assert_match(&self, expected_state: FsState) {
            let diff = expected_state.diff(&self.state());
            if !diff.is_empty() {
                panic!(
                    "Mock filesystem state does not match the expected state:\n {}",
                    diff.join("\n")
                )
            }
        }

        fn state(&self) -> MutexGuard<FsState> {
            self.state.lock().expect("FsMock state lock poisoned.")
        }
    }

    impl<'fs> Fs for FsMock {
        type File = FileMock;

        type Entry = EntryMock;

        fn create_file(&self, path: &Path) -> Result<Self::File> {
            let mut state = self.state();
            if let Some(file) = state.get_or_create_file(path) {
                Ok(file)
            } else {
                if state.is_directory(path) {
                    Err(anyhow!(
                        "The file '{}' can't be opened or created, because it is a directory.",
                        path.display()
                    ))
                } else {
                    Err(anyhow!(
                        "The file '{}' can't be opened or created, because one of it's parent paths which have to be created is occupied.",
                        path.display()
                    ))
                }
            }
        }

        fn delete_file(&self, path: &Path) -> Result<()> {
            let mut state = self.state();
            if state.delete_if_file(path) {
                Ok(())
            } else {
                if state.is_directory(path) {
                    Err(anyhow!(
                        "The file '{}' can't be deleted because it is a directory.",
                        path.display()
                    ))
                } else {
                    Err(anyhow!(
                        "The file '{}' can't be deleted because it doesn't exist.",
                        path.display()
                    ))
                }
            }
        }

        fn open_readable_file(&self, path: &Path) -> Result<Self::File> {
            let state = self.state();
            if let Some(file) = state.get_file_for_reading(path) {
                Ok(file)
            } else {
                if state.is_directory(path) {
                    Err(anyhow!(
                        "The file '{}' can't be opened for reading because it is a directory.",
                        path.display()
                    ))
                } else {
                    Err(anyhow!(
                        "The file '{}' can't be opened for reading because it doesn't exist.",
                        path.display()
                    ))
                }
            }
        }

        fn open_writable_file(&self, path: &Path) -> Result<Self::File> {
            let state = self.state();
            if let Some(file) = state.get_file(path) {
                Ok(file)
            } else {
                if state.is_directory(path) {
                    Err(anyhow!("The file '{}' can't be opened for reading and writing because it is a directory.", path.display()))
                } else {
                    Err(anyhow!("The file '{}' can't be opened for reading and writing because it doesn't exist.", path.display()))
                }
            }
        }

        fn create_directory(&self, path: &Path) -> Result<()> {
            let mut state = self.state();
            if state.create_directory(path) {
                Ok(())
            } else {
                if state.is_directory(path) {
                    Err(anyhow!(
                        "The directory '{}' can't be created because it already exists.",
                        path.display()
                    ))
                } else if state.is_file(path) {
                    Err(anyhow!("The directory '{}' can't be created because there is a file with the same path.", path.display()))
                } else {
                    Err(anyhow!(
                        "The directory '{}' can't be opened or created, because one of it's parent paths which have to be created is occupied.",
                        path.display()
                    ))
                }
            }
        }

        fn read_directory(&self, path: &Path) -> Result<Vec<Self::Entry>> {
            let state = self.state();
            if let Some(entries) = state.get_entries_if_directory(path) {
                Ok(entries)
            } else {
                if state.is_file(path) {
                    Err(anyhow!(
                        "The directory '{}' can't be read because it is a file.",
                        path.display()
                    ))
                } else {
                    Err(anyhow!(
                        "The directory '{}' can't be read because it doesn't exist.",
                        path.display()
                    ))
                }
            }
        }

        fn delete_directory(&self, path: &Path) -> Result<()> {
            let mut state = self.state();
            if state.delete_if_directory(path) {
                Ok(())
            } else {
                if state.is_file(path) {
                    Err(anyhow!(
                        "The directory '{}' can't be deleted because it is a file.",
                        path.display()
                    ))
                } else {
                    Err(anyhow!(
                        "The directory '{}' can't be deleted because it doesn't exist.",
                        path.display()
                    ))
                }
            }
        }

        fn write_to_file(&self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
            let mut state = self.state();
            if file.writable {
                if state.write_to_if_file(&file.path, buffer) {
                    Ok(())
                } else {
                    if state.is_directory(&file.path) {
                        Err(anyhow!(
                            "The file '{}' can't be written to because it is a directory.",
                            file.path.display()
                        ))
                    } else {
                        Err(anyhow!(
                            "The file '{}' can't be written to because it doesn't exist.",
                            file.path.display()
                        ))
                    }
                }
            } else {
                Err(anyhow!(
                    "The file '{}' is not writable.",
                    file.path.display()
                ))
            }
        }

        fn read_from_file(&self, file: &mut Self::File) -> Result<Vec<u8>> {
            let state = self.state();
            if let Some(content) = state.get_content_if_file(&file.path) {
                Ok(content)
            } else {
                if state.is_directory(&file.path) {
                    Err(anyhow!(
                        "The file '{}' can't be read from because it is a directory.",
                        file.path.display()
                    ))
                } else {
                    Err(anyhow!(
                        "The file '{}' can't be read from because it doesn't exist.",
                        file.path.display()
                    ))
                }
            }
        }

        fn path_exists(&self, path: &Path) -> bool {
            self.state().exists(path)
        }
    }

    pub struct FsState {
        entries: HashMap<PathBuf, EntryMock>,
    }

    impl FsState {
        pub fn new(entries: Vec<EntryMock>) -> Self {
            let mut map = HashMap::new();
            for entry in entries {
                map.insert(entry.path(), entry);
            }

            Self { entries: map }
        }

        fn diff(&self, other: &Self) -> Vec<String> {
            let mut differences = Vec::new();

            let mut keys = HashSet::new();
            keys.extend(self.entries.keys());
            keys.extend(other.entries.keys());

            for path in keys {
                match (self.entries.get(path), other.entries.get(path)) {
                    (Some(own_entry), Some(other_entry)) => match own_entry {
                        EntryMock::File(own_file) => {
                            if let EntryMock::File(other_file) = other_entry {
                                if own_file.content != other_file.content {
                                    differences.push(format!(
                                        "The contents of the file '{}' do not match.
                                    Excepted: {:?},
                                    Received: {:?}",
                                        path.display(),
                                        own_file.content,
                                        other_file.content
                                    ))
                                }
                            } else {
                                differences.push(format!(
                                    "Expected file at '{}', instead found a directory.",
                                    path.display(),
                                ))
                            }
                        }
                        EntryMock::Dir { .. } => {
                            if let EntryMock::File(_) = other_entry {
                                differences.push(format!(
                                    "Expected directory at '{}', instead found a file.",
                                    path.display(),
                                ))
                            }
                        }
                    },
                    (None, Some(missing_entry_for_own)) => {
                        differences.push(match missing_entry_for_own {
                            EntryMock::File(_) => {
                                format!("Found unexpected file at '{}'.", path.display())
                            }
                            EntryMock::Dir { .. } => {
                                format!("Found unexpected directory at '{}'.", path.display())
                            }
                        })
                    }
                    (Some(missing_entry_for_other), None) => {
                        differences.push(match missing_entry_for_other {
                            EntryMock::File(_) => {
                                format!("Expected file at '{}'.", path.display())
                            }
                            EntryMock::Dir { .. } => {
                                format!("Expected directory at '{}'.", path.display())
                            }
                        })
                    }
                    _ => unreachable!(),
                }
            }

            differences
        }

        fn get_or_create_file(&mut self, path: &Path) -> Option<FileMock> {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty()
                    && !self.is_directory(parent)
                    && !self.create_directory(parent)
                {
                    return None;
                }
            }

            let path_buf = path.to_path_buf();
            match self.entries.entry(path_buf.clone()) {
                hash_map::Entry::Occupied(occupied) => match occupied.get() {
                    EntryMock::File(file) => Some(file.clone()),
                    _ => None,
                },
                hash_map::Entry::Vacant(vacant) => {
                    let file = FileMock {
                        path: path_buf,
                        writable: true,
                        content: Vec::new(),
                    };
                    vacant.insert(EntryMock::File(file.clone()));
                    Some(file)
                }
            }
        }

        fn delete_if_file(&mut self, path: &Path) -> bool {
            if self.is_file(path) {
                self.entries.remove(path).is_some()
            } else {
                false
            }
        }

        fn get_file(&self, path: &Path) -> Option<FileMock> {
            match self.entries.get(path) {
                Some(entry) => match entry {
                    EntryMock::File(file) => Some(file.clone()),
                    _ => None,
                },
                _ => None,
            }
        }

        fn get_file_for_reading(&self, path: &Path) -> Option<FileMock> {
            self.get_file(path).map(|mut f| {
                f.writable = false;
                f
            })
        }

        fn get_content_if_file(&self, path: &Path) -> Option<Vec<u8>> {
            self.get_file(path).map(|f| f.content)
        }

        fn write_to_if_file(&mut self, path: &Path, buffer: Vec<u8>) -> bool {
            match self.entries.get_mut(path) {
                Some(entry) => match entry {
                    EntryMock::File(file) => {
                        file.content = buffer;
                        true
                    }
                    _ => false,
                },
                _ => false,
            }
        }

        fn create_directory(&mut self, path: &Path) -> bool {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !self.is_directory(parent) && !self.create_directory(parent) {
                    return false;
                }
            }

            let path_buf = path.to_path_buf();
            match self.entries.entry(path_buf.clone()) {
                hash_map::Entry::Vacant(vacant) => {
                    vacant.insert(EntryMock::Dir { path: path_buf });
                    true
                }
                _ => false,
            }
        }

        fn delete_if_directory(&mut self, path: &Path) -> bool {
            if self.is_directory(path) {
                self.entries.remove(path).is_some()
            } else {
                false
            }
        }

        fn get_entries_if_directory(&self, path: &Path) -> Option<Vec<EntryMock>> {
            if self.is_directory(path) {
                let directory_entries = self
                    .entries
                    .iter()
                    .filter(|&(path, _)| {
                        if let Some(parent) = path.parent() {
                            parent == path
                        } else {
                            false
                        }
                    })
                    .map(|(_, entry)| entry.clone())
                    .collect();

                Some(directory_entries)
            } else {
                None
            }
        }

        fn is_file(&self, path: &Path) -> bool {
            self.entries
                .get(path)
                .map_or(false, |e| matches!(e, EntryMock::File(_)))
        }

        fn is_directory(&self, path: &Path) -> bool {
            // We assume these exist.
            if path.as_os_str() == "." || path.as_os_str() == "/" {
                return true;
            }

            self.entries
                .get(path)
                .map_or(false, |e| matches!(e, EntryMock::Dir { .. }))
        }

        fn exists(&self, path: &Path) -> bool {
            self.entries.contains_key(path)
        }
    }

    #[derive(Clone)]
    pub struct FileMock {
        path: PathBuf,
        writable: bool,
        content: Vec<u8>,
    }

    #[derive(Clone)]
    pub enum EntryMock {
        File(FileMock),
        Dir { path: PathBuf },
    }

    impl EntryMock {
        pub fn file(path_str: &str, content: &[u8]) -> Self {
            EntryMock::File(FileMock {
                path: Path::new(path_str).to_path_buf(),
                writable: true,
                content: content.to_vec(),
            })
        }

        pub fn dir(path_str: &str) -> Self {
            EntryMock::Dir {
                path: Path::new(path_str).to_path_buf(),
            }
        }
    }

    impl FsEntry for EntryMock {
        fn path(&self) -> PathBuf {
            match self {
                EntryMock::File(FileMock { path, .. }) => path.clone(),
                EntryMock::Dir { path } => path.clone(),
            }
        }

        fn is_directory(&self) -> Result<bool> {
            Ok(matches!(self, EntryMock::Dir { .. }))
        }
    }

    mod tests {
        use std::path::Path;

        use crate::filesystem::{mock::EntryMock, Fs};

        use super::{FsMock, FsState};

        #[test]
        fn empty() {
            let mock = FsMock::new();
            mock.assert_match(FsState::new(Vec::new()))
        }

        #[test]
        fn basic() {
            let mock = FsMock::new();

            let mut file = mock.create_file(Path::new("./folder/file")).unwrap();
            mock.write_to_file(&mut file, "content".as_bytes().into())
                .unwrap();

            mock.assert_match(FsState::new(vec![
                EntryMock::dir("./folder"),
                EntryMock::file("./folder/file", "content".as_bytes()),
            ]))
        }

        #[test]
        fn deletion() {
            let mock = FsMock::new();

            mock.create_file(Path::new("./folder/file_to_delete")).unwrap();
            mock.create_directory(Path::new("./dir_to_delete")).unwrap();
            mock.delete_file(Path::new("./folder/file_to_delete")).unwrap();
            mock.delete_directory(Path::new("./dir_to_delete")).unwrap();

            mock.assert_match(FsState::new(vec![
                EntryMock::dir("./folder"),
            ]))
        }

        // TODO: Add more test coverage for FsMock, as it has to be as robust as possible
        // to ensure that tests depending on it are sane.
    }
}
