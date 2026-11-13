//! The practice session: its state, and how it responds to keys and time.
//!
//! Nothing here draws anything or touches files. Methods take the current
//! time as an argument rather than reading the clock, so tests can control
//! time. What should be saved is queued as records, which the caller drains
//! and writes to the store.

use std::time::{Duration, Instant};

use jiff::{SignedDuration, Timestamp, Unit};
use rand::{RngExt, rngs::StdRng};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    generate::Settings,
    problem::Problem,
    store::{Attempt, Session},
};

/// The longest answer the input accepts. Any answer we ask for fits easily.
pub const MAX_INPUT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Practice,
    Summary,
}

/// Something that happened and should be saved.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Attempt(Attempt),
    Session(Session),
}

pub struct App {
    settings: Settings,
    rng: StdRng,
    duration: Duration,
    /// A moment known on both clocks, for turning an `Instant` into a
    /// timestamp.
    epoch: (Instant, Timestamp),
    session: String,
    started: Instant,
    /// When the current problem appeared.
    asked: Instant,
    /// Backspaces pressed on the current problem.
    edits: u32,
    records: Vec<Record>,
    pub screen: Screen,
    /// Time left in the session, as of the last `tick`.
    pub remaining: Duration,
    pub problem: Problem,
    pub input: String,
    pub score: u32,
    pub quit: bool,
}

impl App {
    /// Starts a session. `now` and `wall` must describe the same moment.
    pub fn new(
        settings: Settings,
        duration: Duration,
        mut rng: StdRng,
        now: Instant,
        wall: Timestamp,
    ) -> Self {
        let problem = settings.generate(&mut rng);
        let session = new_session_id(&mut rng);
        Self {
            settings,
            rng,
            duration,
            epoch: (now, wall),
            session,
            started: now,
            asked: now,
            edits: 0,
            records: Vec::new(),
            screen: Screen::Practice,
            remaining: duration,
            problem,
            input: String::new(),
            score: 0,
            quit: false,
        }
    }

    /// Takes the records queued since the last call.
    pub fn drain_records(&mut self) -> impl Iterator<Item = Record> + '_ {
        self.records.drain(..)
    }

    /// Brings the clock up to date, ending the session if time is up.
    pub fn tick(&mut self, now: Instant) {
        self.remaining = self.duration.saturating_sub(now - self.started);
        if self.screen == Screen::Practice && self.remaining.is_zero() {
            self.end_session(self.started + self.duration, true);
            self.screen = Screen::Summary;
        }
    }

    /// How long until the countdown shows a different whole second, or
    /// `None` when nothing on screen changes with time.
    pub fn until_next_tick(&self) -> Option<Duration> {
        if self.screen != Screen::Practice {
            return None;
        }
        let fraction = Duration::from_nanos(self.remaining.subsec_nanos().into());
        Some(if fraction.is_zero() {
            Duration::from_secs(1)
        } else {
            fraction
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) {
        if !key.is_press() {
            return;
        }
        let screen = self.screen;
        self.tick(now);
        if self.screen != screen {
            // Time ran out before this key arrived. It belongs to the old
            // screen, so it must not act on the new one.
            return;
        }
        match (self.screen, key.code) {
            (_, KeyCode::Esc) => self.quit(now),
            (_, KeyCode::Char('c')) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit(now);
            }
            (Screen::Practice, KeyCode::Char(c)) => self.type_char(c, now),
            (Screen::Practice, KeyCode::Backspace) => {
                if self.input.pop().is_some() {
                    self.edits += 1;
                }
            }
            (Screen::Summary, KeyCode::Enter) => self.restart(now),
            _ => {}
        }
    }

    /// Accepts digits, plus a minus sign at the start. Checks the answer
    /// after every keystroke, so a correct answer moves on without Enter.
    fn type_char(&mut self, c: char, now: Instant) {
        let allowed = c.is_ascii_digit() || (c == '-' && self.input.is_empty());
        if !allowed || self.input.len() >= MAX_INPUT {
            return;
        }
        self.input.push(c);
        if self.input.parse() == Ok(self.problem.answer()) {
            self.record_attempt(now);
            self.score += 1;
            self.next_problem(now);
        }
    }

    fn record_attempt(&mut self, now: Instant) {
        let answer = self.problem.answer().to_string();
        self.records.push(Record::Attempt(Attempt {
            ts: self.wall(now),
            session: self.session.clone(),
            problem: self.problem,
            question: self.problem.to_string(),
            // Auto-submit only ever accepts a correct answer.
            correct: true,
            answer,
            given: self.input.clone(),
            ms: (now - self.asked).as_millis() as u64,
            edits: self.edits,
        }));
    }

    fn next_problem(&mut self, now: Instant) {
        self.problem = self.settings.generate(&mut self.rng);
        self.input.clear();
        self.edits = 0;
        self.asked = now;
    }

    fn end_session(&mut self, end: Instant, completed: bool) {
        self.records.push(Record::Session(Session {
            id: self.session.clone(),
            start: self.wall(self.started),
            end: self.wall(end),
            duration: self.duration.as_secs(),
            score: self.score,
            completed,
            problems: self.settings.clone(),
        }));
    }

    fn quit(&mut self, now: Instant) {
        if self.screen == Screen::Practice {
            self.end_session(now, false);
        }
        self.quit = true;
    }

    /// Average time per correct answer over the whole session.
    pub fn time_per_answer(&self) -> Option<Duration> {
        (self.score > 0).then(|| self.duration / self.score)
    }

    fn restart(&mut self, now: Instant) {
        self.session = new_session_id(&mut self.rng);
        self.started = now;
        self.remaining = self.duration;
        self.screen = Screen::Practice;
        self.score = 0;
        self.next_problem(now);
    }

    /// The wall-clock time at `now`, to the millisecond.
    fn wall(&self, now: Instant) -> Timestamp {
        let (instant, timestamp) = self.epoch;
        let elapsed = SignedDuration::try_from(now - instant).unwrap_or(SignedDuration::MAX);
        timestamp
            .saturating_add(elapsed)
            .and_then(|t| t.round(Unit::Millisecond))
            .unwrap_or(timestamp)
    }
}

/// A random 16-digit hex string. Only needs to be unique within one
/// person's history.
fn new_session_id(rng: &mut StdRng) -> String {
    format!("{:016x}", rng.random::<u64>())
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use ratatui::crossterm::event::KeyEventKind;

    use super::*;

    const DURATION: Duration = Duration::from_secs(120);

    /// An app that started at the returned instant.
    fn new_app() -> (App, Instant) {
        let t0 = Instant::now();
        let app = App::new(
            Settings::default(),
            DURATION,
            StdRng::seed_from_u64(0),
            t0,
            wall_epoch(),
        );
        (app, t0)
    }

    fn wall_epoch() -> Timestamp {
        "2026-11-13T10:00:00Z".parse().unwrap()
    }

    fn records(app: &mut App) -> Vec<Record> {
        app.drain_records().collect()
    }

    fn only_record(app: &mut App) -> Record {
        let mut records = records(app);
        assert_eq!(records.len(), 1, "{records:?}");
        records.remove(0)
    }

    fn press(app: &mut App, code: KeyCode, now: Instant) {
        app.handle_key(KeyEvent::from(code), now);
    }

    fn type_str(app: &mut App, s: &str, now: Instant) {
        s.chars().for_each(|c| press(app, KeyCode::Char(c), now));
    }

    fn secs(s: f64) -> Duration {
        Duration::from_secs_f64(s)
    }

    #[test]
    fn correct_answer_advances_without_enter() {
        let (mut app, t0) = new_app();
        let first = app.problem;
        type_str(&mut app, &first.answer().to_string(), t0);
        assert_eq!(app.score, 1);
        assert!(app.input.is_empty());
        assert_ne!(
            app.problem, first,
            "seed 0 should not repeat the first problem"
        );
    }

    #[test]
    fn wrong_answer_stays_put() {
        let (mut app, t0) = new_app();
        let first = app.problem;
        type_str(&mut app, &(first.answer() + 1).to_string(), t0);
        assert_eq!(app.score, 0);
        assert_eq!(app.problem, first);
    }

    #[test]
    fn backspace_fixes_a_typo() {
        let (mut app, t0) = new_app();
        let answer = app.problem.answer().to_string();
        type_str(&mut app, "9999", t0);
        (0..4).for_each(|_| press(&mut app, KeyCode::Backspace, t0));
        type_str(&mut app, &answer, t0);
        assert_eq!(app.score, 1);
    }

    #[test]
    fn ignores_non_answer_characters() {
        let (mut app, t0) = new_app();
        type_str(&mut app, "a 1-.", t0);
        assert_eq!(app.input, "1");
        type_str(&mut app, &"9".repeat(20), t0);
        assert_eq!(app.input.len(), MAX_INPUT);
    }

    #[test]
    fn minus_only_at_the_start() {
        let (mut app, t0) = new_app();
        type_str(&mut app, "-", t0);
        assert_eq!(app.input, "-");
    }

    #[test]
    fn esc_and_ctrl_c_quit() {
        let (mut app, t0) = new_app();
        press(&mut app, KeyCode::Esc, t0);
        assert!(app.quit);

        let (mut app, t0) = new_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), t0);
        assert!(app.quit);
        assert!(app.input.is_empty());
    }

    #[test]
    fn ignores_key_releases() {
        let (mut app, t0) = new_app();
        let mut key = KeyEvent::from(KeyCode::Char('1'));
        key.kind = KeyEventKind::Release;
        app.handle_key(key, t0);
        assert!(app.input.is_empty());
    }

    #[test]
    fn session_ends_when_time_runs_out() {
        let (mut app, t0) = new_app();
        app.tick(t0 + secs(119.5));
        assert_eq!(app.screen, Screen::Practice);
        assert_eq!(app.remaining, secs(0.5));
        app.tick(t0 + DURATION);
        assert_eq!(app.screen, Screen::Summary);
        assert!(app.remaining.is_zero());
    }

    #[test]
    fn answer_after_time_up_does_not_count() {
        let (mut app, t0) = new_app();
        let answer = app.problem.answer().to_string();
        type_str(&mut app, &answer, t0 + DURATION);
        assert_eq!(app.score, 0);
        assert_eq!(app.screen, Screen::Summary);
    }

    #[test]
    fn key_that_ends_the_session_does_not_restart_it() {
        let (mut app, t0) = new_app();
        press(&mut app, KeyCode::Enter, t0 + DURATION);
        assert_eq!(app.screen, Screen::Summary);
    }

    #[test]
    fn enter_on_summary_starts_a_fresh_session() {
        let (mut app, t0) = new_app();
        let answer = app.problem.answer().to_string();
        type_str(&mut app, &answer, t0);
        let t1 = t0 + DURATION;
        app.tick(t1);
        press(&mut app, KeyCode::Enter, t1);
        assert_eq!(app.screen, Screen::Practice);
        assert_eq!(app.score, 0);
        app.tick(t1 + secs(1.0));
        assert_eq!(app.remaining, DURATION - secs(1.0));
    }

    #[test]
    fn time_per_answer() {
        let (mut app, t0) = new_app();
        assert_eq!(app.time_per_answer(), None);
        for _ in 0..3 {
            let answer = app.problem.answer().to_string();
            type_str(&mut app, &answer, t0);
        }
        assert_eq!(app.time_per_answer(), Some(secs(40.0)));
    }

    #[test]
    fn summary_ignores_typing() {
        let (mut app, t0) = new_app();
        app.tick(t0 + DURATION);
        type_str(&mut app, "12", t0 + DURATION);
        assert!(app.input.is_empty());
    }

    #[test]
    fn next_tick_lands_on_a_whole_second() {
        let (mut app, t0) = new_app();
        assert_eq!(app.until_next_tick(), Some(secs(1.0)));
        app.tick(t0 + secs(0.25));
        assert_eq!(app.until_next_tick(), Some(secs(0.75)));
        app.tick(t0 + DURATION);
        assert_eq!(app.until_next_tick(), None);
    }

    #[test]
    fn correct_answer_is_recorded() {
        let (mut app, t0) = new_app();
        let problem = app.problem;
        let answer = problem.answer().to_string();
        type_str(&mut app, "9", t0);
        press(&mut app, KeyCode::Backspace, t0);
        type_str(&mut app, &answer, t0 + secs(2.5));

        let Record::Attempt(attempt) = only_record(&mut app) else {
            panic!("expected one attempt");
        };
        assert_eq!(attempt.problem, problem);
        assert_eq!(attempt.given, answer);
        assert!(attempt.correct);
        assert_eq!(attempt.ms, 2500);
        assert_eq!(attempt.edits, 1);
        assert_eq!(attempt.ts, "2026-11-13T10:00:02.5Z".parse().unwrap());
        assert_eq!(attempt.session, app.session);
    }

    #[test]
    fn timing_restarts_with_each_problem() {
        let (mut app, t0) = new_app();
        let answer = app.problem.answer().to_string();
        type_str(&mut app, &answer, t0 + secs(3.0));
        let answer = app.problem.answer().to_string();
        type_str(&mut app, &answer, t0 + secs(4.0));
        let ms: Vec<_> = records(&mut app)
            .into_iter()
            .map(|r| match r {
                Record::Attempt(a) => a.ms,
                Record::Session(_) => panic!("unexpected session"),
            })
            .collect();
        assert_eq!(ms, [3000, 1000]);
    }

    #[test]
    fn finished_session_is_recorded() {
        let (mut app, t0) = new_app();
        let answer = app.problem.answer().to_string();
        type_str(&mut app, &answer, t0);
        records(&mut app);
        app.tick(t0 + DURATION + secs(5.0));

        let Record::Session(session) = only_record(&mut app) else {
            panic!("expected one session");
        };
        assert!(session.completed);
        assert_eq!(session.score, 1);
        assert_eq!(session.duration, 120);
        assert_eq!(session.start, wall_epoch());
        assert_eq!(session.end, "2026-11-13T10:02:00Z".parse().unwrap());

        // Leaving from the summary does not record the session again.
        press(&mut app, KeyCode::Esc, t0 + DURATION + secs(6.0));
        assert!(records(&mut app).is_empty());
    }

    #[test]
    fn abandoned_session_is_recorded_as_incomplete() {
        let (mut app, t0) = new_app();
        press(&mut app, KeyCode::Esc, t0 + secs(30.0));
        let Record::Session(session) = only_record(&mut app) else {
            panic!("expected one session");
        };
        assert!(!session.completed);
        assert_eq!(session.end, "2026-11-13T10:00:30Z".parse().unwrap());
    }

    #[test]
    fn restart_starts_a_new_session() {
        let (mut app, t0) = new_app();
        let first = app.session.clone();
        app.tick(t0 + DURATION);
        press(&mut app, KeyCode::Enter, t0 + DURATION);
        assert_ne!(app.session, first);
    }
}
