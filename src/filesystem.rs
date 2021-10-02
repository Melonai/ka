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
    use anyhow::Result;
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use super::{Fs, FsEntry};

    pub struct FsMock {
        state: Arc<Mutex<FsMockState>>,
    }

    struct FsMockState {
        expected_calls: Vec<ExpectedCall>,
        received_calls: Vec<ReceivedCall>,
    }

    impl FsMock {
        pub fn new() -> Self {
            let state = FsMockState {
                expected_calls: Vec::new(),
                received_calls: Vec::new(),
            };

            FsMock {
                state: Arc::new(Mutex::new(state)),
            }
        }

        fn add_call(&self, path: &Path, variant: ReceivedCallVariant) {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant,
            };

            let mut state = self.state.lock().expect("File system mock lock poisoned.");
            state.received_calls.push(call);
        }

        fn get_expected_call(&self) -> ExpectedCallVariant {
            let state = self.state.lock().expect("File system lock poisoned.");

            let expected_call = state
                .expected_calls
                .get(state.received_calls.len() - 1)
                .expect("Unexpected call.");

            expected_call.variant.clone()
        }
    }

    impl Fs for FsMock {
        type File = FileMock;
        type Entry = EntryMock;

        fn create_file(&self, path: &Path) -> Result<Self::File> {
            self.add_call(path, ReceivedCallVariant::FileCreated);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn delete_file(&self, path: &Path) -> Result<()> {
            self.add_call(path, ReceivedCallVariant::FileDeleted);

            Ok(())
        }

        fn open_readable_file(&self, path: &Path) -> Result<Self::File> {
            self.add_call(path, ReceivedCallVariant::ReadableFileOpened);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: false,
            })
        }

        fn open_writable_file(&self, path: &Path) -> Result<Self::File> {
            self.add_call(path, ReceivedCallVariant::WritableFileOpened);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn create_directory(&self, path: &Path) -> Result<()> {
            self.add_call(path, ReceivedCallVariant::DirectoryCreated);
            Ok(())
        }

        fn read_directory(&self, path: &Path) -> Result<Vec<Self::Entry>> {
            self.add_call(path, ReceivedCallVariant::DirectoryRead);

            if let ExpectedCallVariant::ReadDirectory { entries } = self.get_expected_call() {
                Ok(entries.to_vec())
            } else {
                panic!("No content to read.");
            }
        }

        fn delete_directory(&self, path: &Path) -> Result<()> {
            self.add_call(path, ReceivedCallVariant::DirectoryDeleted);
            Ok(())
        }

        fn write_to_file(&self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
            self.add_call(
                &file.path,
                ReceivedCallVariant::FileWritten {
                    written_content: buffer,
                },
            );

            // TODO: Check whether file was opened with write flag.

            Ok(())
        }

        fn read_from_file(&self, file: &mut Self::File) -> Result<Vec<u8>> {
            self.add_call(&file.path, ReceivedCallVariant::FileRead);

            if let ExpectedCallVariant::ReadFile { read_content } = self.get_expected_call() {
                Ok(read_content.clone())
            } else {
                panic!("No content to read.");
            }
        }

        fn path_exists(&self, path: &Path) -> bool {
            self.add_call(path, ReceivedCallVariant::DoesPathExist);

            if let ExpectedCallVariant::PathExists(answer) = self.get_expected_call() {
                return answer;
            }

            panic!("No mocked return value given for call.");
        }
    }

    #[derive(Clone)]
    pub struct EntryMock {
        path: PathBuf,
        is_directory: bool,
    }

    impl FsEntry for EntryMock {
        fn path(&self) -> PathBuf {
            self.path.clone()
        }

        fn is_directory(&self) -> Result<bool> {
            Ok(self.is_directory)
        }
    }

    pub struct FileMock {
        path: PathBuf,
        writable: bool,
    }

    // TODO: Do we need ways to return errors?
    pub struct ExpectedCall {
        pub affected_path: PathBuf,
        pub variant: ExpectedCallVariant,
    }

    #[derive(Clone)]
    pub enum ExpectedCallVariant {
        CreateFile,
        DeleteFile,
        OpenReadableFile,
        OpenWritableFile,
        CreateDirectory,
        ReadDirectory { entries: Vec<EntryMock> },
        DeleteDirectory,
        ReadFile { read_content: Vec<u8> },
        WriteToFile,
        PathExists(bool),
    }

    pub struct ReceivedCall {
        pub affected_path: PathBuf,
        pub variant: ReceivedCallVariant,
    }

    pub enum ReceivedCallVariant {
        FileCreated,
        FileDeleted,
        ReadableFileOpened,
        WritableFileOpened,
        DirectoryCreated,
        DirectoryRead,
        DirectoryDeleted,
        FileRead,
        FileWritten { written_content: Vec<u8> },
        DoesPathExist,
    }
}
