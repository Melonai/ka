use anyhow::{Context, Result};
use std::{
    fs::{self, DirEntry, File, OpenOptions},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

pub trait Fs {
    type File;
    type Entry: FsEntry;

    fn create_file(&mut self, path: &Path) -> Result<Self::File>;
    fn delete_file(&mut self, path: &Path) -> Result<()>;
    fn open_readable_file(&mut self, path: &Path) -> Result<Self::File>;
    fn open_writable_file(&mut self, path: &Path) -> Result<Self::File>;

    fn read_directory(&mut self, path: &Path) -> Result<Vec<Self::Entry>>;

    fn write_to_file(&mut self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()>;
    fn read_from_file(&mut self, file: &mut Self::File) -> Result<Vec<u8>>;

    fn path_exists(&mut self, path: &Path) -> bool;
}

pub trait FsEntry {
    fn path(&self) -> PathBuf;
    fn is_directory(&self) -> Result<bool>;
}

pub struct FsImpl {}

impl Fs for FsImpl {
    type File = File;
    type Entry = DirEntry;

    fn create_file(&mut self, path: &Path) -> Result<Self::File> {
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

    fn delete_file(&mut self, path: &Path) -> Result<()> {
        fs::remove_file(path)?;
        Ok(())
    }

    fn open_readable_file(&mut self, path: &Path) -> Result<Self::File> {
        File::open(path)
            .with_context(|| format!("Failed opening '{}' for reading.", path.display()))
    }

    fn open_writable_file(&mut self, path: &Path) -> Result<Self::File> {
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

    fn read_directory(&mut self, path: &Path) -> Result<Vec<Self::Entry>> {
        let result: io::Result<_> = fs::read_dir(path)?.collect();
        result.with_context(|| format!("Failed reading directory {}", path.display()))
    }

    fn write_to_file(&mut self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
        file.rewind()?;
        file.set_len(0)?;
        file.write_all(&buffer)?;
        Ok(())
    }

    fn read_from_file(&mut self, file: &mut Self::File) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn path_exists(&mut self, path: &Path) -> bool {
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
    use anyhow::{Error, Result};
    use std::path::{Path, PathBuf};

    use super::{Fs, FsEntry};

    pub struct FsMock {
        expected_calls: Vec<ExpectedCall>,
        received_calls: Vec<ReceivedCall>,
    }

    impl FsMock {
        pub fn new() -> Self {
            FsMock {
                expected_calls: Vec::new(),
                received_calls: Vec::new(),
            }
        }
    }

    impl Fs for FsMock {
        type File = FileMock;
        type Entry = EntryMock;

        fn create_file(&mut self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant: ReceivedCallVariant::FileCreated,
            };

            self.received_calls.push(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn delete_file(&mut self, path: &Path) -> Result<()> {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant: ReceivedCallVariant::FileCreated,
            };

            self.received_calls.push(call);

            Ok(())
        }

        fn open_readable_file(&mut self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant: ReceivedCallVariant::ReadableFileOpened,
            };

            self.received_calls.push(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: false,
            })
        }

        fn open_writable_file(&mut self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant: ReceivedCallVariant::WritableFileOpened,
            };

            self.received_calls.push(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn read_directory(&mut self, path: &Path) -> Result<Vec<Self::Entry>> {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant: ReceivedCallVariant::DirectoryRead,
            };

            self.received_calls.push(call);

            let expected_call_option = self.expected_calls.get(self.received_calls.len() - 1);

            if let Some(expected_call) = expected_call_option {
                if let ExpectedCallVariant::ReadDirectory { entries } = &expected_call.variant {
                    return Ok(entries.to_vec());
                }
            }

            Err(Error::msg("No content to read."))
        }

        fn write_to_file(&mut self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
            let call = ReceivedCall {
                affected_path: file.path.clone(),
                variant: ReceivedCallVariant::FileWritten {
                    written_content: buffer,
                },
            };

            // TODO: Check whether file was opened with write flag.

            self.received_calls.push(call);

            Ok(())
        }

        fn read_from_file(&mut self, file: &mut Self::File) -> Result<Vec<u8>> {
            let call = ReceivedCall {
                affected_path: file.path.clone(),
                variant: ReceivedCallVariant::FileRead,
            };

            self.received_calls.push(call);

            let expected_call_option = self.expected_calls.get(self.received_calls.len() - 1);

            if let Some(expected_call) = expected_call_option {
                if let ExpectedCallVariant::ReadFile { read_content } = &expected_call.variant {
                    return Ok(read_content.clone());
                }
            }

            Err(Error::msg("No content to read."))
        }

        fn path_exists(&mut self, path: &Path) -> bool {
            let call = ReceivedCall {
                affected_path: path.to_path_buf(),
                variant: ReceivedCallVariant::DoesPathExist,
            };

            self.received_calls.push(call);

            let expected_call_option = self.expected_calls.get(self.received_calls.len() - 1);

            if let Some(expected_call) = expected_call_option {
                if let ExpectedCallVariant::PathExists(answer) = &expected_call.variant {
                    return *answer;
                }
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

    pub enum ExpectedCallVariant {
        CreateFile,
        DeleteFile,
        OpenReadableFile,
        OpenWritableFile,
        ReadDirectory { entries: Vec<EntryMock> },
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
        DirectoryRead,
        FileRead,
        FileWritten { written_content: Vec<u8> },
        DoesPathExist,
    }
}
