use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use similar::{Algorithm, DiffOp};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ContentChange {
    Inserted { at: usize, new_content: Vec<u8> },
    Deleted { at: usize, upto: usize },
}

impl ContentChange {
    pub fn diff(old: &[u8], new: &[u8]) -> Vec<Self> {
        let deadline = Instant::now() + Duration::from_millis(100);
        let change_set =
            similar::capture_diff_slices_deadline(Algorithm::Myers, old, new, Some(deadline));

        let mut at = 0;
        let mut changes = Vec::new();

        for diff in change_set {
            match diff {
                DiffOp::Delete { old_len, .. } => {
                    changes.push(ContentChange::Deleted {
                        at,
                        upto: at + old_len,
                    });
                }
                DiffOp::Insert {
                    new_index, new_len, ..
                } => {
                    let new_content = &new[new_index..new_index + new_len];
                    let change = ContentChange::Inserted {
                        at,
                        new_content: new_content.to_vec(),
                    };
                    at += new_len;
                    changes.push(change);
                }
                DiffOp::Replace {
                    old_len,
                    new_index,
                    new_len,
                    ..
                } => {
                    let new_content = &new[new_index..new_index + new_len];

                    let removed_change = ContentChange::Deleted {
                        at,
                        upto: at + old_len,
                    };
                    let added_change = ContentChange::Inserted {
                        at,
                        new_content: new_content.to_vec(),
                    };

                    changes.push(removed_change);
                    changes.push(added_change);

                    at += new_len;
                }
                DiffOp::Equal { len, .. } => {
                    at += len;
                }
            }
        }

        changes
    }

    pub fn apply(&self, buffer: &mut Vec<u8>) {
        match self {
            ContentChange::Deleted { at, upto } => {
                buffer.drain(at..upto);
            }
            ContentChange::Inserted { at, new_content } => {
                buffer.splice(at..at, new_content.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentChange::*, *};

    #[test]
    fn test_diff() {
        let old = "This is an old string...";
        let new = "This is a new string...!";

        let changes = ContentChange::diff(old.as_bytes(), new.as_bytes());
        assert_eq!(
            changes.as_slice(),
            [
                Inserted {
                    at: 9,
                    new_content: " ".into()
                },
                Deleted { at: 11, upto: 15 },
                Inserted {
                    at: 11,
                    new_content: "ew".into()
                },
                Inserted {
                    at: 23,
                    new_content: "!".into()
                }
            ],
        );
    }

    #[test]
    fn test_apply() {
        let old = "This is an old string...";
        let new = "This is a new text...!";

        let changes = ContentChange::diff(old.as_bytes(), new.as_bytes());

        let mut buffer = old.as_bytes().to_vec();
        for change in changes {
            change.apply(&mut buffer);
        }

        assert_eq!(&buffer, new.as_bytes());
    }
}
