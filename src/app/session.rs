//! One timed practice session.

use std::time::{Duration, Instant};

use jiff::{SignedDuration, Timestamp, Unit};
use rand::{RngExt, rngs::StdRng};

use super::Record;
use crate::{
    answer::Answer,
    config::{Config, Submit},
    generate::Settings,
    problem::Problem,
    store::{Attempt, SessionRecord},
};

/// The longest answer the input accepts. Any answer we ask for fits easily,
/// even a prime factorization written out in full.
pub const MAX_INPUT: usize = 20;

/// A moment known on both clocks, for turning an `Instant` into a timestamp.
#[derive(Debug, Clone, Copy)]
pub struct Epoch {
    pub instant: Instant,
    pub wall: Timestamp,
}

impl Epoch {
    /// The wall-clock time at `now`, to the millisecond.
    pub fn wall(self, now: Instant) -> Timestamp {
        let elapsed = SignedDuration::try_from(now - self.instant).unwrap_or(SignedDuration::MAX);
        self.wall
            .saturating_add(elapsed)
            .and_then(|t| t.round(Unit::Millisecond))
            .unwrap_or(self.wall)
    }
}

/// What the end-of-session screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub score: u32,
    pub wrong: u32,
    pub skipped: u32,
    /// Average time per correct answer over the whole session.
    pub time_per_answer: Option<Duration>,
}

/// The previous question, shown after a wrong answer or a skip so the
/// right answer can sink in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    pub question: String,
    pub answer: String,
}

pub struct Session {
    pub id: String,
    settings: Settings,
    duration: Duration,
    pub submit: Submit,
    rng: StdRng,
    epoch: Epoch,
    started: Instant,
    /// When the current problem appeared.
    asked: Instant,
    /// Backspaces pressed on the current problem.
    edits: u32,
    /// Time left, as of the last `tick`.
    pub remaining: Duration,
    pub problem: Problem,
    /// What counts as a correct answer to `problem`.
    answer: Answer,
    pub input: String,
    pub score: u32,
    pub wrong: u32,
    pub skipped: u32,
    /// Set after a wrong answer or a skip, cleared by the next answer.
    pub correction: Option<Correction>,
}

impl Session {
    pub fn new(config: &Config, mut rng: StdRng, epoch: Epoch, now: Instant) -> Self {
        let id = format!("{:016x}", rng.random::<u64>());
        let settings = config.problems.clone();
        let duration = config.session.duration();
        let problem = settings.generate(&mut rng);
        let answer = problem.answer();
        Self {
            id,
            settings,
            duration,
            submit: config.session.submit,
            rng,
            epoch,
            started: now,
            asked: now,
            edits: 0,
            remaining: duration,
            problem,
            answer,
            input: String::new(),
            score: 0,
            wrong: 0,
            skipped: 0,
            correction: None,
        }
    }

    /// Brings the clock up to date. Returns whether time is up.
    pub fn tick(&mut self, now: Instant) -> bool {
        self.remaining = self.duration.saturating_sub(now - self.started);
        self.remaining.is_zero()
    }

    /// How long until the countdown shows a different whole second.
    pub fn until_next_tick(&self) -> Duration {
        let fraction = Duration::from_nanos(self.remaining.subsec_nanos().into());
        if fraction.is_zero() {
            Duration::from_secs(1)
        } else {
            fraction
        }
    }

    /// Adds a character to the input. With auto-submit, the answer is
    /// checked after every keystroke, so a correct one moves on at once.
    pub fn type_char(&mut self, c: char, now: Instant, out: &mut Vec<Record>) {
        // Input stays ASCII, so its length in bytes is its width on screen.
        let allowed = c.is_ascii_graphic() || (c == ' ' && !self.input.is_empty());
        if !allowed || self.input.len() >= MAX_INPUT {
            return;
        }
        self.input.push(c);
        let done = self.answer.check(&self.input) || self.answer.is_final(&self.input);
        if self.submit == Submit::Auto && done {
            self.submit_answer(now, out);
        }
    }

    /// Hands in what has been typed, right or wrong. Does nothing if
    /// nothing has been typed, so a stray Enter costs nothing.
    pub fn submit_answer(&mut self, now: Instant, out: &mut Vec<Record>) {
        if self.input.trim().is_empty() {
            return;
        }
        let correct = self.answer.check(&self.input);
        self.finish_problem(now, correct, false, out);
    }

    /// Gives up on the current problem, showing its answer.
    pub fn skip(&mut self, now: Instant, out: &mut Vec<Record>) {
        self.finish_problem(now, false, true, out);
    }

    fn finish_problem(
        &mut self,
        now: Instant,
        correct: bool,
        skipped: bool,
        out: &mut Vec<Record>,
    ) {
        out.push(Record::Attempt(self.attempt(now, correct, skipped)));
        self.correction = (!correct).then(|| Correction {
            question: self.problem.to_string(),
            answer: self.answer.to_string(),
        });
        match (correct, skipped) {
            (true, _) => self.score += 1,
            (false, true) => self.skipped += 1,
            (false, false) => self.wrong += 1,
        }
        self.next_problem(now);
    }

    pub fn backspace(&mut self) {
        if self.input.pop().is_some() {
            self.edits += 1;
        }
    }

    /// The record of this session, ending at `end`.
    pub fn record(&self, end: Instant, completed: bool) -> SessionRecord {
        SessionRecord {
            id: self.id.clone(),
            start: self.epoch.wall(self.started),
            end: self.epoch.wall(end.min(self.started + self.duration)),
            duration: self.duration.as_secs(),
            score: self.score,
            completed,
            submit: self.submit,
            problems: self.settings.clone(),
        }
    }

    pub fn summary(&self) -> Summary {
        Summary {
            score: self.score,
            wrong: self.wrong,
            skipped: self.skipped,
            time_per_answer: (self.score > 0).then(|| self.duration / self.score),
        }
    }

    fn attempt(&self, now: Instant, correct: bool, skipped: bool) -> Attempt {
        Attempt {
            ts: self.epoch.wall(now),
            session: self.id.clone(),
            problem: self.problem,
            question: self.problem.to_string(),
            answer: self.answer.to_string(),
            given: self.input.clone(),
            correct,
            skipped,
            submit: self.submit,
            ms: (now - self.asked).as_millis() as u64,
            edits: self.edits,
        }
    }

    fn next_problem(&mut self, now: Instant) {
        self.problem = self.settings.generate(&mut self.rng);
        self.answer = self.problem.answer();
        self.input.clear();
        self.edits = 0;
        self.asked = now;
    }
}
