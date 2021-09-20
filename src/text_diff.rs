use difference::{Changeset, Difference};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TextChange {
    Inserted { at: usize, new_content: String },
    Deleted { at: usize, upto: usize },
}

impl TextChange {
    pub fn diff(old: &str, new: &str) -> Vec<Self> {
        let change_set = Changeset::new(old, new, "");

        let mut changes = Vec::new();
        let mut at: usize = 0;

        for diff in change_set.diffs.iter() {
            match diff {
                Difference::Add(new_content) => {
                    let change = TextChange::Inserted {
                        at,
                        new_content: new_content.to_owned(),
                    };
                    at += new_content.len();
                    changes.push(change);
                }
                Difference::Rem(removed_content) => {
                    changes.push(TextChange::Deleted {
                        at,
                        upto: removed_content.len(),
                    });
                }
                Difference::Same(same_content) => {
                    at += same_content.len();
                }
            }
        }

        changes
    }

    pub fn apply(&self, buffer: &mut String) {
        match self {
            TextChange::Deleted { at, upto } => {
                buffer.replace_range(at..upto, "");
            }
            TextChange::Inserted { at, new_content } => {
                buffer.insert_str(*at, new_content);
            }
        }
    }
}
