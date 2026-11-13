//! Practice history, kept as JSON Lines: one JSON object per line.
//!
//! `attempts.jsonl` gets a row per question answered, and `sessions.jsonl`
//! a row per session. Rows are appended as they happen, so a crash loses
//! at most the line being written, and the files are easy to inspect or
//! process with other tools.

use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use etcetera::BaseStrategy;
use jiff::Timestamp;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{config::Submit, drill::Mode, problem::Problem};

/// One question, as it was answered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    /// When the answer was given.
    pub ts: Timestamp,
    /// The session this attempt belongs to.
    pub session: String,
    #[serde(flatten)]
    pub problem: Problem,
    /// The question as it was shown, such as `47 + 38`.
    pub question: String,
    pub answer: String,
    /// What was typed.
    pub given: String,
    pub correct: bool,
    /// Given up on with Tab. A skip is never correct.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    /// How the answer was handed in.
    #[serde(default)]
    pub submit: Submit,
    /// Milliseconds from the question appearing to the answer.
    pub ms: u64,
    /// How many times Backspace was pressed.
    pub edits: u32,
}

/// One session, written when it ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub start: Timestamp,
    pub end: Timestamp,
    /// Planned length, in seconds.
    pub duration: u64,
    pub score: u32,
    /// Whether the clock ran out, as opposed to the session being abandoned.
    pub completed: bool,
    #[serde(default)]
    pub submit: Submit,
    /// How problems were chosen.
    #[serde(default)]
    pub mode: Mode,
    /// The problem settings the session used, kept as plain JSON so that
    /// old rows still load after the settings change shape.
    pub problems: serde_json::Value,
}

/// Something that happened and should be saved.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Attempt(Attempt),
    Session(SessionRecord),
}

/// A change to what is stored.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Save(Record),
    /// Delete the whole history.
    Clear,
}

/// Rows read back from a file, minus any that could not be parsed.
#[derive(Debug)]
pub struct Loaded<T> {
    pub rows: Vec<T>,
    /// Lines that could not be parsed, such as one cut short by a crash.
    pub skipped: usize,
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn attempts_path(&self) -> PathBuf {
        self.dir.join("attempts.jsonl")
    }

    pub fn sessions_path(&self) -> PathBuf {
        self.dir.join("sessions.jsonl")
    }

    pub fn apply(&self, change: &Change) -> Result<()> {
        match change {
            Change::Save(record) => self.save(record),
            Change::Clear => self.clear(),
        }
    }

    /// Deletes every attempt and session. Nothing else in the folder is
    /// touched, and having nothing to delete is not an error.
    pub fn clear(&self) -> Result<()> {
        for path in [self.attempts_path(), self.sessions_path()] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(e).with_context(|| format!("deleting {}", path.display())),
            }
        }
        Ok(())
    }

    pub fn save(&self, record: &Record) -> Result<()> {
        match record {
            Record::Attempt(attempt) => self.append_attempt(attempt),
            Record::Session(session) => self.append_session(session),
        }
    }

    pub fn append_attempt(&self, attempt: &Attempt) -> Result<()> {
        self.append(&self.attempts_path(), attempt)
    }

    pub fn append_session(&self, session: &SessionRecord) -> Result<()> {
        self.append(&self.sessions_path(), session)
    }

    /// Every attempt on record. A row whose problem has no answer, as
    /// only editing the file by hand could leave, counts as damaged.
    pub fn attempts(&self) -> Result<Loaded<Attempt>> {
        let mut loaded: Loaded<Attempt> = read(&self.attempts_path())?;
        let read = loaded.rows.len();
        loaded.rows.retain(|a| a.problem.is_well_formed());
        loaded.skipped += read - loaded.rows.len();
        Ok(loaded)
    }

    pub fn sessions(&self) -> Result<Loaded<SessionRecord>> {
        read(&self.sessions_path())
    }

    fn append(&self, path: &Path, row: &impl Serialize) -> Result<()> {
        let mut line = serde_json::to_string(row)?;
        line.push('\n');
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;
        // One write per row, so with O_APPEND each row lands whole.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(line.as_bytes()))
            .with_context(|| format!("writing {}", path.display()))
    }
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<Loaded<T>> {
    let mut loaded = Loaded {
        rows: Vec::new(),
        skipped: 0,
    };
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(loaded),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("reading {}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(&line) {
            Ok(row) => loaded.rows.push(row),
            Err(_) => loaded.skipped += 1,
        }
    }
    Ok(loaded)
}

/// `$ANZAN_DATA_DIR` if set, otherwise `$XDG_DATA_HOME/anzan`.
pub fn data_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("ANZAN_DATA_DIR") {
        return Ok(dir.into());
    }
    let dirs = etcetera::choose_base_strategy().context("finding the home directory")?;
    Ok(dirs.data_dir().join("anzan"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt() -> Attempt {
        let problem = Problem::Add { lhs: 47, rhs: 38 };
        Attempt {
            ts: "2026-11-13T10:00:00Z".parse().unwrap(),
            session: "00c0ffee00c0ffee".into(),
            problem,
            question: problem.to_string(),
            answer: problem.answer().to_string(),
            given: "85".into(),
            correct: true,
            skipped: false,
            submit: Submit::Auto,
            ms: 2140,
            edits: 1,
        }
    }

    #[test]
    fn attempt_row_format() {
        let json = serde_json::to_string(&attempt()).unwrap();
        assert_eq!(
            json,
            r#"{"ts":"2026-11-13T10:00:00Z","session":"00c0ffee00c0ffee","kind":"add","lhs":47,"rhs":38,"question":"47 + 38","answer":"85","given":"85","correct":true,"submit":"auto","ms":2140,"edits":1}"#
        );
    }

    #[test]
    fn rows_from_before_submit_modes_still_load() {
        let old = r#"{"ts":"2026-11-13T10:00:00Z","session":"00c0ffee00c0ffee","kind":"add","lhs":47,"rhs":38,"question":"47 + 38","answer":"85","given":"85","correct":true,"ms":2140,"edits":1}"#;
        let attempt: Attempt = serde_json::from_str(old).unwrap();
        assert_eq!(attempt, self::attempt());
    }

    #[test]
    fn appends_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("nested"));
        assert!(store.attempts().unwrap().rows.is_empty());

        let first = attempt();
        let second = Attempt {
            given: "84".into(),
            correct: false,
            ..attempt()
        };
        store.append_attempt(&first).unwrap();
        store.append_attempt(&second).unwrap();
        assert_eq!(store.attempts().unwrap().rows, [first, second]);

        let skip = Attempt {
            given: String::new(),
            correct: false,
            skipped: true,
            submit: Submit::Enter,
            ..attempt()
        };
        store.append_attempt(&skip).unwrap();
        assert_eq!(store.attempts().unwrap().rows.last(), Some(&skip));
    }

    fn session() -> SessionRecord {
        SessionRecord {
            id: "00c0ffee00c0ffee".into(),
            start: "2026-11-13T10:00:00Z".parse().unwrap(),
            end: "2026-11-13T10:02:00Z".parse().unwrap(),
            duration: 120,
            score: 41,
            completed: true,
            submit: Submit::Enter,
            mode: Mode::Oni,
            problems: serde_json::json!({"kinds": ["add"]}),
        }
    }

    #[test]
    fn sessions_round_trip_and_old_rows_take_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store
            .apply(&Change::Save(Record::Session(session())))
            .unwrap();
        assert_eq!(store.sessions().unwrap().rows, [session()]);
        assert!(store.attempts().unwrap().rows.is_empty());
        // Rows written before sessions recorded how they were played.
        let old = r#"{"id":"a","start":"2026-11-13T10:00:00Z","end":"2026-11-13T10:02:00Z","duration":120,"score":3,"completed":false,"problems":{}}"#;
        let old: SessionRecord = serde_json::from_str(old).unwrap();
        assert_eq!((old.submit, old.mode), (Submit::Auto, Mode::Jiyu));
    }

    #[test]
    fn clear_deletes_the_history_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        // With nothing there, not even the folder, there is nothing to do.
        Store::new(dir.path().join("missing")).clear().unwrap();
        store.clear().unwrap();

        store.append_attempt(&attempt()).unwrap();
        store.append_session(&session()).unwrap();
        fs::write(dir.path().join("notes.txt"), "mine").unwrap();
        store.apply(&Change::Clear).unwrap();
        assert!(store.attempts().unwrap().rows.is_empty());
        assert!(store.sessions().unwrap().rows.is_empty());
        assert!(!store.attempts_path().exists() && !store.sessions_path().exists());
        assert!(dir.path().join("notes.txt").exists());
        // And it can be written to again.
        store.append_attempt(&attempt()).unwrap();
        assert_eq!(store.attempts().unwrap().rows.len(), 1);
    }

    #[test]
    fn skips_damaged_lines() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store.append_attempt(&attempt()).unwrap();
        let mut file = OpenOptions::new()
            .append(true)
            .open(store.attempts_path())
            .unwrap();
        // A row that parses but asks the impossible, then one cut short.
        let impossible = serde_json::to_string(&attempt()).unwrap().replace(
            r#""kind":"add","lhs":47,"rhs":38"#,
            r#""kind":"div","lhs":47,"rhs":0"#,
        );
        file.write_all(format!("{impossible}\n").as_bytes())
            .unwrap();
        file.write_all(b"\n{\"ts\":\"2026-11-1").unwrap();

        let loaded = store.attempts().unwrap();
        assert_eq!(loaded.rows, [attempt()]);
        assert_eq!(loaded.skipped, 2);
    }
}
