use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};

use crate::{diff::ContentChange, filesystem::Fs};

#[derive(Serialize, Deserialize, Debug)]
pub struct RepositoryHistory {
    pub cursor: usize,
    changes: Vec<RepositoryChange>,
}

impl RepositoryHistory {
    pub fn encode(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).context("Failed encoding repository history.")
    }

    pub fn decode(buffer: &[u8]) -> Result<Self> {
        serde_json::from_slice::<Self>(buffer).context("Failed decoding repository history.")
    }

    pub fn from_file<FS: Fs>(fs: &FS, file: &mut FS::File) -> Result<Self> {
        let buffer = fs
            .read_from_file(file)
            .context("Failed reading repository history.")?;

        Self::decode(&buffer)
    }

    pub fn write_to_file<FS: Fs>(&self, fs: &FS, file: &mut FS::File) -> Result<()> {
        let encoded: Vec<u8> = self.encode()?;
        fs.write_to_file(file, encoded)?;
        Ok(())
    }

    pub fn get_changes(&self) -> &Vec<RepositoryChange> {
        &self.changes
    }

    pub fn add_change(&mut self, change: RepositoryChange) {
        self.changes.push(change);
    }
}

impl Default for RepositoryHistory {
    fn default() -> Self {
        Self {
            cursor: 0,
            changes: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RepositoryChange {
    pub affected_files: Vec<PathBuf>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileHistory {
    changes: Vec<FileChange>,
}

impl FileHistory {
    pub fn encode(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).context("Failed encoding file history.")
    }

    pub fn decode(buffer: &[u8]) -> Result<Self> {
        serde_json::from_slice::<Self>(buffer).context("Failed decoding file history.")
    }

    pub fn from_file<FS: Fs>(fs: &FS, file: &mut FS::File) -> Result<Self> {
        let buffer = fs
            .read_from_file(file)
            .context("Failed reading file history.")?;

        Self::decode(&buffer)
    }

    pub fn write_to_file<FS: Fs>(&self, fs: &FS, file: &mut FS::File) -> Result<()> {
        let encoded: Vec<u8> = self.encode()?;
        fs.write_to_file(file, encoded)?;
        Ok(())
    }

    pub fn is_file_deleted(&self, at_cursor: usize) -> bool {
        match self
            .changes
            .iter()
            .take_while(|c| c.change_index <= at_cursor)
            .last()
        {
            Some(change) => match change.variant {
                FileChangeVariant::Deleted => true,
                FileChangeVariant::Updated(_) => false,
            },
            None => false,
        }
    }

    pub fn get_content(&self, at_cursor: usize) -> Vec<u8> {
        let mut buffer = Vec::new();

        for file_change in self
            .changes
            .iter()
            .take_while(|change| change.change_index <= at_cursor)
        {
            if let FileChangeVariant::Updated(ref updated) = file_change.variant {
                for change in updated.iter() {
                    change.apply(&mut buffer)
                }
            } else {
                buffer.drain(0..);
            }
        }
        buffer
    }

    pub fn add_change(&mut self, change: FileChange) {
        self.changes.push(change);
    }
}

impl Default for FileHistory {
    fn default() -> Self {
        Self {
            changes: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileChange {
    pub change_index: usize,
    pub variant: FileChangeVariant,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileChangeVariant {
    Updated(Vec<ContentChange>),
    Deleted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_content() {
        let stages = &[
            "",
            "hiii!",
            "yes hii? this is a test.",
            "yes bye! this is not a test.",
        ];

        let mut history = FileHistory::default();

        history.add_change(FileChange {
            change_index: 0,
            variant: FileChangeVariant::Updated(Vec::new()),
        });

        for old_index in 0..stages.len() - 1 {
            let old = stages[old_index].as_bytes();
            let new = stages[old_index + 1].as_bytes();

            let stage_difference = ContentChange::diff(old, new);

            history.add_change(FileChange {
                change_index: old_index + 1,
                variant: FileChangeVariant::Updated(stage_difference),
            });
        }

        for index in 0..stages.len() {
            assert_eq!(stages[index].as_bytes(), history.get_content(index));
        }
    }
}
