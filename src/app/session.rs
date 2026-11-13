//! One timed practice session.

use std::time::{Duration, Instant};

use jiff::{SignedDuration, Timestamp, Unit};
use rand::{RngExt, rngs::StdRng};

use super::Record;
use crate::{
    generate::Settings,
    problem::Problem,
    store::{Attempt, SessionRecord},
};

/// The longest answer the input accepts. Any answer we ask for fits easily.
pub const MAX_INPUT: usize = 12;

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
    /// Average time per correct answer over the whole session.
    pub time_per_answer: Option<Duration>,
}

pub struct Session {
    pub id: String,
    settings: Settings,
    duration: Duration,
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
    pub input: String,
    pub score: u32,
}

impl Session {
    pub fn new(
        settings: Settings,
        duration: Duration,
        mut rng: StdRng,
        epoch: Epoch,
        now: Instant,
    ) -> Self {
        let id = format!("{:016x}", rng.random::<u64>());
        let problem = settings.generate(&mut rng);
        Self {
            id,
            settings,
            duration,
            rng,
            epoch,
            started: now,
            asked: now,
            edits: 0,
            remaining: duration,
            problem,
            input: String::new(),
            score: 0,
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

    /// Accepts digits, plus a minus sign at the start. Checks the answer
    /// after every keystroke, so a correct answer moves on without Enter.
    pub fn type_char(&mut self, c: char, now: Instant, out: &mut Vec<Record>) {
        let allowed = c.is_ascii_digit() || (c == '-' && self.input.is_empty());
        if !allowed || self.input.len() >= MAX_INPUT {
            return;
        }
        self.input.push(c);
        if self.input.parse() == Ok(self.problem.answer()) {
            out.push(Record::Attempt(self.attempt(now)));
            self.score += 1;
            self.next_problem(now);
        }
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
            problems: self.settings.clone(),
        }
    }

    pub fn summary(&self) -> Summary {
        Summary {
            score: self.score,
            time_per_answer: (self.score > 0).then(|| self.duration / self.score),
        }
    }

    fn attempt(&self, now: Instant) -> Attempt {
        Attempt {
            ts: self.epoch.wall(now),
            session: self.id.clone(),
            problem: self.problem,
            question: self.problem.to_string(),
            answer: self.problem.answer().to_string(),
            given: self.input.clone(),
            // Auto-submit only ever accepts a correct answer.
            correct: true,
            ms: (now - self.asked).as_millis() as u64,
            edits: self.edits,
        }
    }

    fn next_problem(&mut self, now: Instant) {
        self.problem = self.settings.generate(&mut self.rng);
        self.input.clear();
        self.edits = 0;
        self.asked = now;
    }
}
