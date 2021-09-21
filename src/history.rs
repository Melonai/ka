use std::{
    fs::File,
    io::{Read, Seek, Write},
};

use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};

use crate::diff::ContentChange;

#[derive(Serialize, Deserialize, Debug)]
pub struct FileHistory {
    cursor: usize,
    changes: Vec<FileChange>,
}

impl FileHistory {
    pub fn from_file(file: &mut File) -> Result<FileHistory> {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .context("Failed reading file history.")?;

        let file_history = serde_json::from_slice::<FileHistory>(&buffer);
        Ok(file_history.context("Corrupted file history.")?)
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn file_is_deleted(&self) -> bool {
        match self.changes.last() {
            Some(change) => match change {
                FileChange::Deleted => true,
                FileChange::Updated(_) => false,
            },
            None => false,
        }
    }

    pub fn get_content(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        for file_change in self.changes.iter().take(self.cursor) {
            if let FileChange::Updated(ref updated) = file_change {
                for change in updated.iter() {
                    change.apply(&mut buffer)
                }
            } else {
                buffer.drain(0..);
            }
        }
        buffer
    }

    pub fn write_to_file(&self, file: &mut File) -> anyhow::Result<()> {
        let encoded: Vec<u8> = serde_json::to_vec(self)?;
        file.rewind()?;
        file.set_len(0)?;
        file.write_all(encoded.as_ref())?;
        Ok(())
    }

    pub fn set_cursor(&mut self, new_cursor: usize) {
        if new_cursor < self.changes.len() {
            self.cursor = new_cursor;
        } else {
            panic!(
                "Out-of-bounds cursor for file history: {}, can be at most {}",
                new_cursor,
                self.changes.len()
            );
        }
    }

    pub fn add_change(&mut self, change: FileChange) {
        self.changes.push(change);
    }
}

impl Default for FileHistory {
    fn default() -> Self {
        Self {
            cursor: 0,
            changes: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileChange {
    Updated(Vec<ContentChange>),
    Deleted,
}
