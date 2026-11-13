//! Everything practised so far, in memory.
//!
//! Loaded once at startup and kept up to date as new records arrive, so
//! the dashboard and the practice modes never read the files again.

use anyhow::Result;

use crate::{
    app::Record,
    store::{Attempt, SessionRecord, Store},
};

#[derive(Debug, Default, Clone)]
pub struct History {
    /// In the order they were made.
    pub attempts: Vec<Attempt>,
    pub sessions: Vec<SessionRecord>,
    /// Lines that could not be read, such as one cut short by a crash.
    pub damaged: usize,
}

impl History {
    pub fn load(store: &Store) -> Result<Self> {
        let attempts = store.attempts()?;
        let sessions = store.sessions()?;
        Ok(Self {
            attempts: attempts.rows,
            sessions: sessions.rows,
            damaged: attempts.skipped + sessions.skipped,
        })
    }

    pub fn add(&mut self, record: &Record) {
        match record {
            Record::Attempt(a) => self.attempts.push(a.clone()),
            Record::Session(s) => self.sessions.push(s.clone()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.attempts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Submit, problem::Problem};

    #[test]
    fn loads_what_was_stored() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        assert!(History::load(&store).unwrap().is_empty());
        let problem = Problem::Add { lhs: 2, rhs: 3 };
        let attempt = Attempt {
            ts: "2026-11-13T10:00:00Z".parse().unwrap(),
            session: "s".into(),
            problem,
            question: problem.to_string(),
            answer: "5".into(),
            given: "5".into(),
            correct: true,
            skipped: false,
            submit: Submit::Auto,
            ms: 900,
            edits: 0,
        };
        store.append_attempt(&attempt).unwrap();
        let mut history = History::load(&store).unwrap();
        assert_eq!(history.attempts, std::slice::from_ref(&attempt));
        history.add(&Record::Attempt(attempt));
        assert_eq!(history.attempts.len(), 2);
        assert_eq!(history.damaged, 0);
    }
}
