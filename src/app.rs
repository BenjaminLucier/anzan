//! The practice session: its state, and how it responds to keys and time.
//!
//! Nothing here draws anything; see `ui` for that. Methods take the current
//! time as an argument rather than reading the clock, so tests can control
//! time.

use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{generate::Settings, problem::Problem};

/// The longest answer the input accepts. Any answer we ask for fits easily.
pub const MAX_INPUT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Practice,
    Summary,
}

pub struct App {
    settings: Settings,
    rng: StdRng,
    duration: Duration,
    started: Instant,
    pub screen: Screen,
    /// Time left in the session, as of the last `tick`.
    pub remaining: Duration,
    pub problem: Problem,
    pub input: String,
    pub score: u32,
    pub quit: bool,
}

impl App {
    pub fn new(settings: Settings, duration: Duration, mut rng: StdRng, now: Instant) -> Self {
        let problem = settings.generate(&mut rng);
        Self {
            settings,
            rng,
            duration,
            started: now,
            screen: Screen::Practice,
            remaining: duration,
            problem,
            input: String::new(),
            score: 0,
            quit: false,
        }
    }

    /// Brings the clock up to date, ending the session if time is up.
    pub fn tick(&mut self, now: Instant) {
        self.remaining = self.duration.saturating_sub(now - self.started);
        if self.screen == Screen::Practice && self.remaining.is_zero() {
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
            (_, KeyCode::Esc) => self.quit = true,
            (_, KeyCode::Char('c')) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
            }
            (Screen::Practice, KeyCode::Char(c)) => self.type_char(c),
            (Screen::Practice, KeyCode::Backspace) => {
                self.input.pop();
            }
            (Screen::Summary, KeyCode::Enter) => self.restart(now),
            _ => {}
        }
    }

    /// Accepts digits, plus a minus sign at the start. Checks the answer
    /// after every keystroke, so a correct answer moves on without Enter.
    fn type_char(&mut self, c: char) {
        let allowed = c.is_ascii_digit() || (c == '-' && self.input.is_empty());
        if !allowed || self.input.len() >= MAX_INPUT {
            return;
        }
        self.input.push(c);
        if self.input.parse() == Ok(self.problem.answer()) {
            self.score += 1;
            self.input.clear();
            self.problem = self.settings.generate(&mut self.rng);
        }
    }

    /// Average time per correct answer over the whole session.
    pub fn time_per_answer(&self) -> Option<Duration> {
        (self.score > 0).then(|| self.duration / self.score)
    }

    fn restart(&mut self, now: Instant) {
        self.started = now;
        self.remaining = self.duration;
        self.screen = Screen::Practice;
        self.problem = self.settings.generate(&mut self.rng);
        self.input.clear();
        self.score = 0;
    }
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
        let app = App::new(Settings::default(), DURATION, StdRng::seed_from_u64(0), t0);
        (app, t0)
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
}
