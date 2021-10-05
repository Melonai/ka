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

    // TODO: This testing style is very imperative as we have to consider every single
    // call that happens in an action. (See actions::create::tests)
    // Could we instead emulate a fake in-memory file system for FsMock, requiring only
    // an input state and an output state, with no knowledge what happens in between.
    // That would greatly simplify making tests.

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

        pub fn set_expected_calls(&self, calls: Vec<ExpectedCall>) {
            let mut state = self.state.lock().expect("File system mock lock poisoned.");
            state.expected_calls = calls;
        }

        pub fn assert_calls(self) {
            let state = self.state.lock().expect("File system lock poisoned.");

            let longest_call_amount = state.received_calls.len().max(state.expected_calls.len());

            for call_index in 0..longest_call_amount {
                let expected_option = state.expected_calls.get(call_index);
                let received_option = state.received_calls.get(call_index);

                let expected_call = expected_option.unwrap_or_else(|| {
                    panic!(
                        "Received unexpected call: '{:?}'.",
                        received_option.unwrap()
                    )
                });
                let received_call = received_option.unwrap_or_else(|| {
                    panic!(
                        "Expected call: '{:?}', which was not received.",
                        expected_option.unwrap()
                    )
                });

                expected_call.assert_received(received_call);
            }
        }

        fn add_call(&self, call: ReceivedCall) {
            let mut state = self.state.lock().expect("File system mock lock poisoned.");
            state.received_calls.push(call);
        }

        fn get_expected_call(&self) -> Option<ExpectedCallVariant> {
            let state = self.state.lock().expect("File system lock poisoned.");

            state
                .expected_calls
                .get(state.received_calls.len())
                .map(|e| e.variant.clone())
        }
    }

    impl Fs for FsMock {
        type File = FileMock;
        type Entry = EntryMock;

        fn create_file(&self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::FileCreated);
            self.add_call(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn delete_file(&self, path: &Path) -> Result<()> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::FileDeleted);
            self.add_call(call);

            Ok(())
        }

        fn open_readable_file(&self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::ReadableFileOpened);
            self.add_call(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: false,
            })
        }

        fn open_writable_file(&self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::WritableFileOpened);
            self.add_call(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn create_directory(&self, path: &Path) -> Result<()> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::DirectoryCreated);
            self.add_call(call);

            Ok(())
        }

        fn read_directory(&self, path: &Path) -> Result<Vec<Self::Entry>> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::DirectoryRead);

            let return_value = if let Some(expected_call) = self.get_expected_call() {
                if let ExpectedCallVariant::ReadDirectory { returned } = expected_call {
                    Ok(returned.to_vec())
                } else {
                    panic!(
                        "Received unexpected call: {:?}, expected call: {:?}.",
                        call, expected_call
                    );
                }
            } else {
                panic!(
                    "Received unexpected call, with no expected call to compare it to: {:?}.",
                    call
                );
            };

            self.add_call(call);

            return_value
        }

        fn delete_directory(&self, path: &Path) -> Result<()> {
            let call = ReceivedCall::new(path, ReceivedCallVariant::DirectoryDeleted);
            self.add_call(call);

            Ok(())
        }

        fn write_to_file(&self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
            let call = ReceivedCall::new(
                &file.path,
                ReceivedCallVariant::FileWritten { received: buffer },
            );
            self.add_call(call);

            // TODO: Check whether file was opened with write flag.

            Ok(())
        }

        fn read_from_file(&self, file: &mut Self::File) -> Result<Vec<u8>> {
            let call = ReceivedCall::new(&file.path, ReceivedCallVariant::FileRead);

            let return_value = if let Some(expected_call) = self.get_expected_call() {
                if let ExpectedCallVariant::ReadFile { returned } = expected_call {
                    Ok(returned.clone())
                } else {
                    panic!(
                        "Received unexpected call: {:?}, expected call: {:?}.",
                        call, expected_call
                    );
                }
            } else {
                panic!(
                    "Received unexpected call, with no expected call to compare it to: {:?}.",
                    call
                );
            };

            self.add_call(call);

            return_value
        }

        fn path_exists(&self, path: &Path) -> bool {
            let call = ReceivedCall::new(path, ReceivedCallVariant::DoesPathExist);

            let return_value = if let Some(expected_call) = self.get_expected_call() {
                if let ExpectedCallVariant::PathExists(answer) = expected_call {
                    answer
                } else {
                    panic!(
                        "Received unexpected call: {:?}, expected call: {:?}.",
                        call, expected_call
                    );
                }
            } else {
                panic!(
                    "Received unexpected call, with no expected call to compare it to: {:?}.",
                    call
                );
            };

            self.add_call(call);

            return_value
        }
    }

    #[derive(Debug, Clone)]
    pub struct EntryMock {
        path: PathBuf,
        is_directory: bool,
    }

    impl EntryMock {
        pub fn new(path: &Path, is_directory: bool) -> Self {
            EntryMock {
                path: path.to_path_buf(),
                is_directory,
            }
        }
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
    #[derive(Debug)]
    pub struct ExpectedCall {
        pub affected_path: PathBuf,
        pub variant: ExpectedCallVariant,
    }

    impl ExpectedCall {
        pub fn new(path: &Path, variant: ExpectedCallVariant) -> Self {
            ExpectedCall {
                affected_path: path.to_path_buf(),
                variant,
            }
        }

        fn assert_received(&self, rec: &ReceivedCall) {
            if self.affected_path == rec.affected_path {
                use ExpectedCallVariant as E;
                use ReceivedCallVariant as R;

                let e = &self.variant;

                let equal = match rec.variant {
                    R::FileCreated => matches!(e, E::CreateFile),
                    R::FileDeleted => matches!(e, E::DeleteFile),
                    R::ReadableFileOpened => matches!(e, E::OpenReadableFile),
                    R::WritableFileOpened => matches!(e, E::OpenWritableFile),
                    R::DirectoryCreated => matches!(e, E::CreateDirectory),
                    R::DirectoryRead => matches!(e, E::ReadDirectory { .. }),
                    R::DirectoryDeleted => matches!(e, E::DeleteDirectory),
                    R::FileRead => matches!(e, E::ReadFile { .. }),
                    R::FileWritten { ref received } => {
                        if let E::WriteToFile { expected } = e {
                            expected == received
                        } else {
                            false
                        }
                    }
                    R::DoesPathExist => matches!(e, E::PathExists(_)),
                };

                if equal {
                    return;
                }
            }

            panic!(
                "
                Expected call does not equal received call.
                Expected: {:?}
                Recevied: {:?}
                ",
                self, rec
            );
        }
    }

    #[derive(Debug, Clone)]
    pub enum ExpectedCallVariant {
        CreateFile,
        DeleteFile,
        OpenReadableFile,
        OpenWritableFile,
        CreateDirectory,
        ReadDirectory { returned: Vec<EntryMock> },
        DeleteDirectory,
        ReadFile { returned: Vec<u8> },
        WriteToFile { expected: Vec<u8> },
        PathExists(bool),
    }

    #[derive(Debug)]
    struct ReceivedCall {
        affected_path: PathBuf,
        variant: ReceivedCallVariant,
    }

    impl ReceivedCall {
        fn new(path: &Path, variant: ReceivedCallVariant) -> Self {
            ReceivedCall {
                affected_path: path.to_path_buf(),
                variant,
            }
        }
    }

    #[derive(Debug)]
    enum ReceivedCallVariant {
        FileCreated,
        FileDeleted,
        ReadableFileOpened,
        WritableFileOpened,
        DirectoryCreated,
        DirectoryRead,
        DirectoryDeleted,
        FileRead,
        FileWritten { received: Vec<u8> },
        DoesPathExist,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::filesystem::{
        mock::{ExpectedCall, ExpectedCallVariant},
        Fs,
    };

    use super::mock::FsMock;

    #[test]
    fn mock_empty() {
        let fs_mock = FsMock::new();
        fs_mock.assert_calls();
    }

    #[test]
    fn mock_file_basic() {
        let fs_mock = FsMock::new();

        let path = Path::new("file").to_path_buf();

        fs_mock.set_expected_calls(vec![
            ExpectedCall::new(&path, ExpectedCallVariant::CreateFile),
            ExpectedCall::new(
                &path,
                ExpectedCallVariant::ReadFile {
                    returned: vec![1, 2, 3],
                },
            ),
        ]);

        let mut file = fs_mock.create_file(&path).unwrap();
        let received_content = fs_mock.read_from_file(&mut file).unwrap();

        assert_eq!(received_content, vec![1, 2, 3]);

        fs_mock.assert_calls();
    }

    #[test]
    #[should_panic]
    fn mock_unexpected_call() {
        let fs_mock = FsMock::new();

        let path = Path::new("file").to_path_buf();

        fs_mock.set_expected_calls(vec![ExpectedCall::new(
            &path,
            ExpectedCallVariant::CreateFile,
        )]);

        fs_mock.delete_file(&path).unwrap();
        fs_mock.assert_calls();
    }

    #[test]
    #[should_panic]
    fn mock_unequal_calls() {
        let fs_mock = FsMock::new();

        let path = Path::new("file").to_path_buf();

        fs_mock.set_expected_calls(vec![
            ExpectedCall::new(&path, ExpectedCallVariant::CreateFile),
            ExpectedCall::new(&path, ExpectedCallVariant::DeleteFile),
        ]);

        fs_mock.create_file(&path).unwrap();
        fs_mock.assert_calls();
    }
}
