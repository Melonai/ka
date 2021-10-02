use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    path::Path,
};

pub trait Fs {
    type File;

    fn create_file(&mut self, path: &Path) -> Result<Self::File>;
    fn open_readable_file(&mut self, path: &Path) -> Result<Self::File>;
    fn open_writable_file(&mut self, path: &Path) -> Result<Self::File>;

    fn write_to_file(&mut self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()>;
    fn read_from_file(&mut self, file: &mut Self::File) -> Result<Vec<u8>>;
}

pub struct FsImpl {}

impl Fs for FsImpl {
    type File = File;

    fn create_file(&mut self, path: &Path) -> Result<Self::File> {
        OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Failed creating '{}'.", path.display()))
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
}

#[cfg(test)]
pub mod mock {
    use std::path::{Path, PathBuf};
    use anyhow::{Error, Result};

    use super::Fs;

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

        fn create_file(&mut self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall {
                affected_file: path.to_path_buf(),
                variant: ReceivedCallVariant::FileCreated,
            };

            self.received_calls.push(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn open_readable_file(&mut self, path: &Path) -> Result<Self::File> {
            let call = ReceivedCall {
                affected_file: path.to_path_buf(),
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
                affected_file: path.to_path_buf(),
                variant: ReceivedCallVariant::WritableFileOpened,
            };

            self.received_calls.push(call);

            Ok(FileMock {
                path: path.to_path_buf(),
                writable: true,
            })
        }

        fn write_to_file(&mut self, file: &mut Self::File, buffer: Vec<u8>) -> Result<()> {
            let call = ReceivedCall {
                affected_file: file.path.clone(),
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
                affected_file: file.path.clone(),
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
    }

    pub struct FileMock {
        path: PathBuf,
        writable: bool,
    }

    // TODO: Do we need ways to return errors?
    pub struct ExpectedCall {
        pub affected_file: PathBuf,
        pub variant: ExpectedCallVariant,
    }

    pub enum ExpectedCallVariant {
        CreateFile,
        OpenReadableFile,
        OpenWritableFile,
        ReadFile { read_content: Vec<u8> },
        WriteToFile,
    }

    pub struct ReceivedCall {
        pub affected_file: PathBuf,
        pub variant: ReceivedCallVariant,
    }

    pub enum ReceivedCallVariant {
        FileCreated,
        ReadableFileOpened,
        WritableFileOpened,
        FileRead,
        FileWritten { written_content: Vec<u8> },
    }
}
