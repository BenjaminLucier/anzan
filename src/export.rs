//! CSV export of practice history.

use std::io::Write;

use anyhow::Result;
use serde::Serialize;

use crate::store::{Attempt, SessionRecord};

/// The columns of an exported attempt. Problem-specific operands are left
/// out, since they differ between kinds of problem; `question` shows them.
#[derive(Serialize)]
struct AttemptRow<'a> {
    ts: String,
    session: &'a str,
    kind: &'static str,
    question: &'a str,
    answer: &'a str,
    given: &'a str,
    correct: bool,
    skipped: bool,
    submit: &'static str,
    ms: u64,
    edits: u32,
}

#[derive(Serialize)]
struct SessionRow<'a> {
    id: &'a str,
    start: String,
    end: String,
    duration: u64,
    score: u32,
    completed: bool,
    submit: &'static str,
    mode: &'static str,
}

pub fn attempts(rows: &[Attempt], out: impl Write) -> Result<()> {
    let mut csv = csv::Writer::from_writer(out);
    for a in rows {
        csv.serialize(AttemptRow {
            ts: a.ts.to_string(),
            session: &a.session,
            kind: a.problem.kind().name(),
            question: &a.question,
            answer: &a.answer,
            given: &a.given,
            correct: a.correct,
            skipped: a.skipped,
            submit: a.submit.name(),
            ms: a.ms,
            edits: a.edits,
        })?;
    }
    csv.flush()?;
    Ok(())
}

pub fn sessions(rows: &[SessionRecord], out: impl Write) -> Result<()> {
    let mut csv = csv::Writer::from_writer(out);
    for s in rows {
        csv.serialize(SessionRow {
            id: &s.id,
            start: s.start.to_string(),
            end: s.end.to_string(),
            duration: s.duration,
            score: s.score,
            completed: s.completed,
            submit: s.submit.name(),
            mode: s.mode.name(),
        })?;
    }
    csv.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Submit, drill::Mode, problem::Problem};

    #[test]
    fn attempts_as_csv() {
        let problem = Problem::Mul { lhs: 7, rhs: 38 };
        let attempt = Attempt {
            ts: "2026-11-13T10:00:00.25Z".parse().unwrap(),
            session: "abc".into(),
            problem,
            question: problem.to_string(),
            answer: "266".into(),
            given: "266".into(),
            correct: true,
            skipped: false,
            submit: Submit::Enter,
            ms: 3100,
            edits: 0,
        };
        let mut out = Vec::new();
        attempts(&[attempt], &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "ts,session,kind,question,answer,given,correct,skipped,submit,ms,edits\n\
             2026-11-13T10:00:00.25Z,abc,mul,7 × 38,266,266,true,false,enter,3100,0\n"
        );
    }

    #[test]
    fn sessions_as_csv() {
        let session = SessionRecord {
            id: "abc".into(),
            start: "2026-11-13T10:00:00Z".parse().unwrap(),
            end: "2026-11-13T10:02:00Z".parse().unwrap(),
            duration: 120,
            score: 41,
            completed: true,
            submit: Submit::Auto,
            mode: Mode::Fukushu,
            problems: serde_json::json!({}),
        };
        let mut out = Vec::new();
        sessions(&[session], &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "id,start,end,duration,score,completed,submit,mode\n\
             abc,2026-11-13T10:00:00Z,2026-11-13T10:02:00Z,120,41,true,auto,fukushu\n"
        );
    }
}
