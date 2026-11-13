//! The application: which screen is showing, and how it responds to keys
//! and time.
//!
//! Nothing here draws anything or touches files. Methods take the current
//! time as an argument rather than reading the clock, so tests can control
//! time. What should be saved is queued as records, which the caller drains
//! and writes to the store.

mod dashboard;
mod menu;
mod session;

use std::time::{Duration, Instant};

use jiff::{Timestamp, tz::TimeZone};
use rand::{SeedableRng, rngs::StdRng};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use self::{
    dashboard::{Dashboard, Report, Tab},
    menu::{Menu, Row},
    session::{Correction, Epoch, MAX_INPUT, Session, Summary},
};
use crate::{
    config::Config,
    history::History,
    stats::report::{self, Fingerprint},
    store::{Attempt, SessionRecord},
};

/// Something that happened and should be saved.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Attempt(Attempt),
    Session(SessionRecord),
}

/// Records waiting to be saved, and the history they are already part of.
#[derive(Debug, Default)]
pub struct Journal {
    pending: Vec<Record>,
    pub history: History,
}

impl Journal {
    pub fn push(&mut self, record: Record) {
        self.history.add(&record);
        self.pending.push(record);
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "there is only ever one screen, so boxing would gain nothing"
)]
pub enum Screen {
    Menu(Menu),
    Practice(Session),
    Summary(Summary),
    Dashboard(Dashboard),
}

pub struct App {
    /// Settings for the next session: the config file, overridden by
    /// command-line flags and then by changes made in the menu.
    pub config: Config,
    rng: StdRng,
    epoch: Epoch,
    journal: Journal,
    pub screen: Screen,
    pub quit: bool,
}

impl App {
    /// Opens on the menu. `now` and `wall` must describe the same moment.
    pub fn new(config: Config, rng: StdRng, now: Instant, wall: Timestamp) -> Self {
        Self {
            config,
            rng,
            epoch: Epoch { instant: now, wall },
            journal: Journal::default(),
            screen: Screen::Menu(Menu::default()),
            quit: false,
        }
    }

    /// Starts from what has been practised before.
    pub fn with_history(mut self, history: History) -> Self {
        self.journal.history = history;
        self
    }

    /// Everything practised, including records not yet drained.
    pub fn history(&self) -> &History {
        &self.journal.history
    }

    /// Skips the menu and starts a session.
    pub fn start_now(mut self, now: Instant) -> Self {
        self.start(now);
        self
    }

    /// Takes the records queued since the last call.
    pub fn drain_records(&mut self) -> impl Iterator<Item = Record> + '_ {
        self.journal.pending.drain(..)
    }

    /// Brings the clock up to date, ending the session if time is up.
    pub fn tick(&mut self, now: Instant) {
        if let Screen::Practice(session) = &mut self.screen
            && session.tick(now)
        {
            self.end_session(now, true);
        }
    }

    /// How long until something on screen changes with time, or `None` if
    /// nothing will.
    pub fn until_next_tick(&self) -> Option<Duration> {
        match &self.screen {
            Screen::Practice(session) => Some(session.until_next_tick()),
            Screen::Menu(_) | Screen::Summary(_) | Screen::Dashboard(_) => None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) {
        if !key.is_press() {
            return;
        }
        let practising = matches!(self.screen, Screen::Practice(_));
        self.tick(now);
        if practising && !matches!(self.screen, Screen::Practice(_)) {
            // Time ran out before this key arrived. It belongs to the old
            // screen, so it must not act on the new one.
            return;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.end_session(now, false);
            self.quit = true;
            return;
        }
        match (&mut self.screen, key.code) {
            (Screen::Menu(_), KeyCode::Esc | KeyCode::Char('q')) => self.quit = true,
            (Screen::Menu(_), KeyCode::Enter) => self.start(now),
            (Screen::Menu(_) | Screen::Summary(_), KeyCode::Char('d')) => {
                self.open_dashboard(now);
            }
            (Screen::Menu(menu), code) => match code {
                KeyCode::Up | KeyCode::Char('k') => menu.up(),
                KeyCode::Down | KeyCode::Char('j') => menu.down(),
                KeyCode::Left | KeyCode::Char('h') => menu.change(&mut self.config, false),
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                    menu.change(&mut self.config, true);
                }
                _ => {}
            },
            (Screen::Practice(_), KeyCode::Esc) => {
                self.end_session(now, false);
                self.screen = Screen::Menu(Menu::default());
            }
            (Screen::Practice(session), KeyCode::Char(c)) => {
                session.type_char(c, now, &mut self.journal);
            }
            (Screen::Practice(session), KeyCode::Backspace) => session.backspace(),
            (Screen::Practice(session), KeyCode::Enter) => {
                session.submit_answer(now, &mut self.journal);
            }
            (Screen::Practice(session), KeyCode::Tab) => session.skip(now, &mut self.journal),
            (Screen::Summary(_), KeyCode::Enter) => self.start(now),
            (Screen::Summary(_), KeyCode::Esc) => self.screen = Screen::Menu(Menu::default()),
            (Screen::Dashboard(_), KeyCode::Esc | KeyCode::Char('q')) => {
                self.screen = Screen::Menu(Menu::default());
            }
            (Screen::Dashboard(dash), code) => match code {
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => dash.next(),
                KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => dash.previous(),
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    dash.select(c.to_digit(10).unwrap_or(0) as usize);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn start(&mut self, now: Instant) {
        let session = Session::new(
            &self.config,
            &self.journal.history,
            StdRng::from_rng(&mut self.rng),
            self.epoch,
            now,
        );
        self.screen = Screen::Practice(session);
    }

    fn open_dashboard(&mut self, now: Instant) {
        let report = Report::build(
            &self.journal.history,
            &self.config,
            self.epoch.wall(now),
            &TimeZone::system(),
        );
        self.screen = Screen::Dashboard(Dashboard::new(report));
    }

    fn end_session(&mut self, now: Instant, completed: bool) {
        if let Screen::Practice(session) = &self.screen {
            let record = session.record(now, completed);
            let fingerprint = Fingerprint::of(&record);
            let previous_best = report::trend(&self.journal.history.sessions, &fingerprint)
                .into_iter()
                .max();
            self.journal.push(Record::Session(record));
            self.screen = Screen::Summary(Summary {
                previous_best,
                ..session.summary()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyEventKind;

    use super::*;
    use crate::config::Submit;

    const DURATION: Duration = Duration::from_secs(120);

    /// Never right: no default problem has a negative answer, and no prefix of
    /// it is a number.
    const WRONG: &str = "-1";

    /// An app that started at the returned instant.
    fn new_app() -> (App, Instant) {
        let t0 = Instant::now();
        let app = App::new(
            Config::default(),
            StdRng::seed_from_u64(0),
            t0,
            wall_epoch(),
        )
        .start_now(t0);
        (app, t0)
    }

    fn session(app: &App) -> &Session {
        match &app.screen {
            Screen::Practice(session) => session,
            _ => panic!("not practising"),
        }
    }

    fn summary(app: &App) -> &Summary {
        match &app.screen {
            Screen::Summary(summary) => summary,
            _ => panic!("not on the summary"),
        }
    }

    fn on_summary(app: &App) -> bool {
        matches!(app.screen, Screen::Summary(_))
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
        let first = session(&app).problem;
        type_str(&mut app, &first.answer().to_string(), t0);
        assert_eq!(session(&app).score, 1);
        assert!(session(&app).input.is_empty());
        assert_ne!(
            session(&app).problem,
            first,
            "seed 0 should not repeat the first problem"
        );
    }

    #[test]
    fn wrong_answer_stays_put() {
        let (mut app, t0) = new_app();
        let first = session(&app).problem;
        type_str(&mut app, WRONG, t0);
        assert_eq!(session(&app).score, 0);
        assert_eq!(session(&app).problem, first);
    }

    #[test]
    fn backspace_fixes_a_typo() {
        let (mut app, t0) = new_app();
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, "9999", t0);
        (0..4).for_each(|_| press(&mut app, KeyCode::Backspace, t0));
        type_str(&mut app, &answer, t0);
        assert_eq!(session(&app).score, 1);
    }

    #[test]
    fn input_is_printable_ascii_without_leading_space() {
        let (mut app, t0) = new_app();
        type_str(&mut app, " 1 ½/", t0);
        assert_eq!(session(&app).input, "1 /");
        type_str(&mut app, &"9".repeat(40), t0);
        assert_eq!(session(&app).input.len(), MAX_INPUT);
    }

    fn on_menu(app: &App) -> bool {
        matches!(app.screen, Screen::Menu(_))
    }

    fn enter_mode_app() -> (App, Instant) {
        let t0 = Instant::now();
        let mut config = Config::default();
        config.session.submit = Submit::Enter;
        let app = App::new(config, StdRng::seed_from_u64(0), t0, wall_epoch()).start_now(t0);
        (app, t0)
    }

    fn attempts(app: &mut App) -> Vec<Attempt> {
        records(app)
            .into_iter()
            .filter_map(|r| match r {
                Record::Attempt(a) => Some(a),
                Record::Session(_) => None,
            })
            .collect()
    }

    #[test]
    fn enter_mode_waits_for_enter() {
        let (mut app, t0) = enter_mode_app();
        let first = session(&app).problem;
        type_str(&mut app, &first.answer().to_string(), t0);
        assert_eq!(session(&app).problem, first);
        press(&mut app, KeyCode::Enter, t0 + secs(2.0));
        assert_eq!(session(&app).score, 1);
        assert!(session(&app).correction.is_none());
        let [attempt] = attempts(&mut app).try_into().unwrap();
        assert!(attempt.correct);
        assert_eq!(attempt.submit, Submit::Enter);
        assert_eq!(attempt.ms, 2000);
    }

    #[test]
    fn wrong_answer_counts_and_shows_the_correction() {
        let (mut app, t0) = enter_mode_app();
        let first = session(&app).problem;
        type_str(&mut app, WRONG, t0);
        press(&mut app, KeyCode::Enter, t0);
        assert_eq!(session(&app).wrong, 1);
        assert_eq!(session(&app).score, 0);
        assert_ne!(session(&app).problem, first);
        let correction = session(&app).correction.clone().unwrap();
        assert_eq!(correction.question, first.to_string());
        assert_eq!(correction.answer, first.answer().to_string());
        let [attempt] = attempts(&mut app).try_into().unwrap();
        assert!(!attempt.correct);
        assert_eq!(attempt.given, WRONG);

        // The next correct answer clears the correction.
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, &answer, t0);
        press(&mut app, KeyCode::Enter, t0);
        assert!(session(&app).correction.is_none());
    }

    #[test]
    fn enter_with_nothing_typed_does_nothing() {
        let (mut app, t0) = enter_mode_app();
        let first = session(&app).problem;
        type_str(&mut app, " ", t0);
        press(&mut app, KeyCode::Enter, t0);
        assert_eq!(session(&app).problem, first);
        assert!(attempts(&mut app).is_empty());
    }

    #[test]
    fn enter_submits_a_wrong_answer_in_auto_mode_too() {
        let (mut app, t0) = new_app();
        type_str(&mut app, WRONG, t0);
        press(&mut app, KeyCode::Enter, t0);
        assert_eq!(session(&app).wrong, 1);
    }

    #[test]
    fn tab_skips() {
        let (mut app, t0) = new_app();
        let first = session(&app).problem;
        type_str(&mut app, "1", t0);
        press(&mut app, KeyCode::Tab, t0);
        assert_eq!(session(&app).skipped, 1);
        assert_ne!(session(&app).problem, first);
        let [attempt] = attempts(&mut app).try_into().unwrap();
        assert!(attempt.skipped && !attempt.correct);
        assert_eq!(attempt.given, "1");
        app.tick(t0 + DURATION);
        assert_eq!(summary(&app).skipped, 1);
    }

    #[test]
    fn opens_on_the_menu() {
        let t0 = Instant::now();
        let app = App::new(
            Config::default(),
            StdRng::seed_from_u64(0),
            t0,
            wall_epoch(),
        );
        assert!(on_menu(&app));
        assert_eq!(app.until_next_tick(), None);
    }

    #[test]
    fn enter_on_menu_starts_with_its_settings() {
        let t0 = Instant::now();
        let mut app = App::new(
            Config::default(),
            StdRng::seed_from_u64(0),
            t0,
            wall_epoch(),
        );
        press(&mut app, KeyCode::Down, t0);
        press(&mut app, KeyCode::Right, t0);
        press(&mut app, KeyCode::Enter, t0);
        assert_eq!(session(&app).remaining, secs(180.0));
    }

    #[test]
    fn esc_leaves_practice_for_the_menu_then_quits() {
        let (mut app, t0) = new_app();
        press(&mut app, KeyCode::Esc, t0);
        assert!(on_menu(&app));
        assert!(!app.quit);
        press(&mut app, KeyCode::Esc, t0);
        assert!(app.quit);
    }

    #[test]
    fn esc_on_summary_returns_to_the_menu() {
        let (mut app, t0) = new_app();
        app.tick(t0 + DURATION);
        press(&mut app, KeyCode::Esc, t0 + DURATION);
        assert!(on_menu(&app));
    }

    #[test]
    fn ctrl_c_quits_from_anywhere() {
        let (mut app, t0) = new_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), t0);
        assert!(app.quit);
        let Record::Session(session) = only_record(&mut app) else {
            panic!("expected the session to be recorded");
        };
        assert!(!session.completed);
    }

    #[test]
    fn ignores_key_releases() {
        let (mut app, t0) = new_app();
        let mut key = KeyEvent::from(KeyCode::Char('1'));
        key.kind = KeyEventKind::Release;
        app.handle_key(key, t0);
        assert!(session(&app).input.is_empty());
    }

    #[test]
    fn session_ends_when_time_runs_out() {
        let (mut app, t0) = new_app();
        app.tick(t0 + secs(119.5));
        assert!(!on_summary(&app));
        assert_eq!(session(&app).remaining, secs(0.5));
        app.tick(t0 + DURATION);
        assert!(on_summary(&app));
    }

    #[test]
    fn answer_after_time_up_does_not_count() {
        let (mut app, t0) = new_app();
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, &answer, t0 + DURATION);
        assert!(on_summary(&app));
        assert_eq!(summary(&app).score, 0);
    }

    #[test]
    fn key_that_ends_the_session_does_not_restart_it() {
        let (mut app, t0) = new_app();
        press(&mut app, KeyCode::Enter, t0 + DURATION);
        assert!(on_summary(&app));
    }

    #[test]
    fn enter_on_summary_starts_a_fresh_session() {
        let (mut app, t0) = new_app();
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, &answer, t0);
        let t1 = t0 + DURATION;
        app.tick(t1);
        press(&mut app, KeyCode::Enter, t1);
        assert!(!on_summary(&app));
        assert_eq!(session(&app).score, 0);
        app.tick(t1 + secs(1.0));
        assert_eq!(session(&app).remaining, DURATION - secs(1.0));
    }

    #[test]
    fn summary_shows_time_per_answer() {
        let (mut app, t0) = new_app();
        assert_eq!(session(&app).summary().time_per_answer, None);
        for _ in 0..3 {
            let answer = session(&app).problem.answer().to_string();
            type_str(&mut app, &answer, t0);
        }
        app.tick(t0 + DURATION);
        assert_eq!(summary(&app).score, 3);
        assert_eq!(summary(&app).time_per_answer, Some(secs(40.0)));
    }

    #[test]
    fn summary_ignores_typing() {
        let (mut app, t0) = new_app();
        app.tick(t0 + DURATION);
        records(&mut app);
        type_str(&mut app, "12", t0 + DURATION);
        assert!(on_summary(&app));
        assert!(records(&mut app).is_empty());
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
        let problem = session(&app).problem;
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
        assert!(!attempt.skipped);
        assert_eq!(attempt.submit, Submit::Auto);
        assert_eq!(attempt.ts, "2026-11-13T10:00:02.5Z".parse().unwrap());
        assert_eq!(attempt.session, session(&app).id);
    }

    #[test]
    fn timing_restarts_with_each_problem() {
        let (mut app, t0) = new_app();
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, &answer, t0 + secs(3.0));
        let answer = session(&app).problem.answer().to_string();
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
        let answer = session(&app).problem.answer().to_string();
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
        let first = session(&app).id.clone();
        app.tick(t0 + DURATION);
        press(&mut app, KeyCode::Enter, t0 + DURATION);
        assert_ne!(session(&app).id, first);
    }

    #[test]
    fn history_includes_records_not_yet_drained() {
        let (mut app, t0) = new_app();
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, &answer, t0);
        assert_eq!(app.history().attempts.len(), 1);
        app.tick(t0 + DURATION);
        assert_eq!(app.history().sessions.len(), 1);
        assert_eq!(records(&mut app).len(), 2);
        // Draining saves them; the history keeps them.
        assert_eq!(app.history().attempts.len(), 1);
    }

    #[test]
    fn dashboard_opens_from_the_menu_and_summary() {
        let (mut app, t0) = new_app();
        app.tick(t0 + DURATION);
        press(&mut app, KeyCode::Char('d'), t0 + DURATION);
        let Screen::Dashboard(dash) = &app.screen else {
            panic!("not on the dashboard");
        };
        assert_eq!(dash.report.sessions, 1);
        press(&mut app, KeyCode::Right, t0 + DURATION);
        let Screen::Dashboard(dash) = &app.screen else {
            panic!("not on the dashboard");
        };
        assert_eq!(dash.tab, Tab::Speed);
        press(&mut app, KeyCode::Esc, t0 + DURATION);
        assert!(on_menu(&app));
        press(&mut app, KeyCode::Char('d'), t0 + DURATION);
        assert!(matches!(app.screen, Screen::Dashboard(_)));
    }

    #[test]
    fn summary_compares_with_the_best_so_far() {
        let (mut app, t0) = new_app();
        app.tick(t0 + DURATION);
        assert_eq!(summary(&app).previous_best, None);
        press(&mut app, KeyCode::Enter, t0 + DURATION);
        let t1 = t0 + DURATION;
        let answer = session(&app).problem.answer().to_string();
        type_str(&mut app, &answer, t1);
        app.tick(t1 + DURATION);
        assert_eq!(summary(&app).score, 1);
        assert_eq!(summary(&app).previous_best, Some(0));
        // An abandoned session does not count.
        press(&mut app, KeyCode::Enter, t1 + DURATION);
        press(&mut app, KeyCode::Esc, t1 + DURATION);
        press(&mut app, KeyCode::Enter, t1 + DURATION);
        app.tick(t1 + 2 * DURATION);
        assert_eq!(summary(&app).previous_best, Some(1));
    }
}
