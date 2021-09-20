use difference::{Changeset, Difference};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, DirEntry, File, OpenOptions},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
    vec,
};

pub fn create_command() -> io::Result<()> {
    let repository_directory = Path::new("./repository");

    let ka_directory = repository_directory.join(".ka");
    let ka_files_directory = ka_directory.join("files");

    if ka_directory.exists() {
        fs::remove_dir_all(ka_directory.as_path())?;
    }

    fs::create_dir(ka_directory)?;
    fs::create_dir(ka_files_directory)?;

    update_command()?;

    Ok(())
}

pub fn update_command() -> io::Result<()> {
    let repository_directory = Path::new("./repository");

    let ka_directory = repository_directory.join(".ka");

    let entries: Vec<DirEntry> = fs::read_dir(repository_directory)?
        .map(Result::unwrap)
        .filter(|entry| entry.path() != ka_directory)
        .flat_map(flatten_directories)
        .collect();

    for element in entries.iter() {
        update_file(element.path())?;
    }

    Ok(())
}

pub fn shift_command(path: &str, new_cursor: usize) {
    let repository_directory = Path::new("./repository");

    let ka_directory = repository_directory.join(".ka");
    let ka_files_directory = ka_directory.join("files");

    let file_path = Path::new(path);

    let mut tracked_file = File::create(file_path).expect("Could not find file.");

    let history_file_path =
        ka_files_directory.join(file_path.strip_prefix(repository_directory).unwrap());
    let mut history_file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(history_file_path)
        .expect("Could not find corresponding history file");

    let mut current_history =
        read_history_from_file(&mut history_file).expect("Failed reading history from file.");
    let state_at_cursor = get_state_from_history_for_cursor(&current_history, new_cursor);

    println!("{}", state_at_cursor);
    println!("{:?}", current_history);

    tracked_file
        .write_all(state_at_cursor.as_bytes())
        .expect("Failed writing new state");

    current_history.cursor = new_cursor;
    write_history_to_file(&mut history_file, &current_history)
        .expect("Could not write new cursor to history file.");
}

fn flatten_directories(entry: DirEntry) -> Vec<DirEntry> {
    let file_type = entry.file_type().expect("Could not read a file type.");
    if file_type.is_dir() {
        fs::read_dir(entry.path())
            .expect("Could not read nested directory.")
            .map(Result::unwrap)
            .flat_map(flatten_directories)
            .collect()
    } else {
        vec![entry]
    }
}

fn update_file(file_path: PathBuf) -> io::Result<()> {
    let repository_directory = Path::new("./repository");

    let ka_directory = repository_directory.join(".ka");
    let ka_files_directory = ka_directory.join("files");

    let mut tracked_file = File::open(file_path.as_path()).expect("Could not find tracked file.");

    let history_file_path =
        ka_files_directory.join(file_path.strip_prefix(repository_directory).unwrap());

    let (mut history_file, current_file_history) = if history_file_path.exists() {
        let mut history_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(history_file_path)
            .expect("Could not open history file.");

        let file_history = read_history_from_file(&mut history_file).unwrap();

        history_file
            .set_len(0)
            .expect("Failed truncating old history file.");

        history_file
            .rewind()
            .expect("Failed rewinding history file.");

        (history_file, file_history)
    } else {
        history_file_path.parent().map(|dir_path| {
            if !dir_path.exists() {
                fs::create_dir_all(dir_path).unwrap();
            }
        });

        let file = File::create(history_file_path).expect("Could not create history file.");

        let empty_file_history = FileHistory {
            cursor: 0,
            changes: Vec::new(),
        };

        (file, empty_file_history)
    };

    let mut new_content = String::new();
    tracked_file
        .read_to_string(&mut new_content)
        .expect("Could not read tracked file.");

    let old_content =
        get_state_from_history_for_cursor(&current_file_history, current_file_history.cursor);

    let change_set = Changeset::new(&old_content, &new_content, "");

    let mut changes = Vec::new();
    let mut at: usize = 0;
    for diff in change_set.diffs.iter() {
        match diff {
            Difference::Add(new_content) => {
                let change = TextChange::Inserted(TextInserted {
                    at,
                    new_content: new_content.to_owned(),
                });
                at += new_content.len();
                changes.push(change);
            }
            Difference::Rem(removed_content) => {
                changes.push(TextChange::Deleted(TextDeleted {
                    at,
                    upto: removed_content.len(),
                }));
            }
            Difference::Same(same_content) => {
                at += same_content.len();
            }
        }
    }

    let mut file_changes = current_file_history.changes;
    println!("{:?}", file_changes);
    file_changes.push(FileChange::Updated(FileUpdated { changes }));

    let new_file_history = FileHistory {
        changes: file_changes,
        cursor: current_file_history.cursor + 1,
    };

    write_history_to_file(&mut history_file, &new_file_history)
        .expect("Failed writing new history.");

    Ok(())
}

fn write_history_to_file(file: &mut File, history: &FileHistory) -> io::Result<()> {
    let encoded: Vec<u8> = serde_json::to_vec(history).unwrap();
    file.write_all(encoded.as_ref())?;
    Ok(())
}

fn read_history_from_file(file: &mut File) -> Result<FileHistory, ()> {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Could not read file history,");

    let file_history = serde_json::from_slice::<FileHistory>(&buffer);
    Ok(file_history.expect("Corrupted file history."))
}

fn get_state_from_history_for_cursor(history: &FileHistory, cursor: usize) -> String {
    let mut buffer = String::new();
    for file_change in history.changes.iter().take(cursor) {
        if let FileChange::Updated(ref updated) = file_change {
            for change in updated.changes.iter() {
                match change {
                    TextChange::Deleted(deletion) => {
                        buffer.replace_range(deletion.at..deletion.upto, "");
                    }
                    TextChange::Inserted(insertion) => {
                        buffer.insert_str(insertion.at, insertion.new_content.as_str());
                    }
                }
            }
        } else {
            buffer = String::new();
        }
    }
    buffer
}

#[derive(Serialize, Deserialize, Debug)]
struct FileHistory {
    cursor: usize,
    changes: Vec<FileChange>,
}

#[derive(Serialize, Deserialize, Debug)]
enum FileChange {
    Updated(FileUpdated),
    Deleted,
}

#[derive(Serialize, Deserialize, Debug)]
struct FileUpdated {
    changes: Vec<TextChange>,
}

#[derive(Serialize, Deserialize, Debug)]
enum TextChange {
    Inserted(TextInserted),
    Deleted(TextDeleted),
}

#[derive(Serialize, Deserialize, Debug)]
struct TextInserted {
    at: usize,
    new_content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TextDeleted {
    at: usize,
    upto: usize,
}
