//! The statistics dashboard: lists on the left, and on the right the
//! details of whatever is selected in the focused list. What it shows is
//! a [`Report`]; this is only where the reader is looking.

use crate::stats::report::Report;

/// A list on the left of the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    Summary,
    Kinds,
    Sessions,
    Weak,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Summary, Side::Kinds, Side::Sessions, Side::Weak];

    pub fn title(self) -> &'static str {
        match self {
            Side::Summary => "Summary",
            Side::Kinds => "Kinds",
            Side::Sessions => "Sessions",
            Side::Weak => "Weak spots",
        }
    }

    pub fn index(self) -> usize {
        Side::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub focus: Side,
    /// The selected row of each list, indexed like `Side::ALL`.
    selected: [usize; 4],
    /// How far the details pane is scrolled.
    pub scroll: usize,
    pub report: Report,
}

impl Dashboard {
    pub fn new(report: Report) -> Self {
        Self {
            focus: Side::default(),
            selected: [0; 4],
            scroll: 0,
            report,
        }
    }

    pub fn selected(&self, side: Side) -> usize {
        self.selected[side.index()]
    }

    /// Moves focus to the next list (`true`) or the previous one.
    pub fn switch(&mut self, forwards: bool) {
        let n = Side::ALL.len();
        let i = self.focus.index();
        self.focus = Side::ALL[if forwards {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }];
        self.scroll = 0;
    }

    /// Focuses the list numbered `n` from 1.
    pub fn focus(&mut self, n: usize) {
        if let Some(&side) = n.checked_sub(1).and_then(|i| Side::ALL.get(i)) {
            self.focus = side;
            self.scroll = 0;
        }
    }

    /// How many rows the list for `side` has.
    pub fn rows(&self, side: Side) -> usize {
        match side {
            Side::Summary => 1,
            Side::Kinds => self.report.kinds.len(),
            Side::Sessions => self.report.sessions.len(),
            Side::Weak => self.report.weak.len(),
        }
    }

    /// How many rows the details of the selection can scroll through:
    /// the attempts of the selected session or weak spot.
    fn detail_rows(&self) -> usize {
        let report = &self.report;
        let selected = self.selected(self.focus);
        match self.focus {
            Side::Summary | Side::Kinds => 0,
            Side::Sessions => report
                .sessions
                .get(selected)
                .map_or(0, |s| s.attempts.len()),
            Side::Weak => report.weak.get(selected).map_or(0, |w| w.attempts.len()),
        }
    }

    /// Moves the selection in the focused list up (`true`) or down.
    pub fn step(&mut self, up: bool) {
        let last = self.rows(self.focus).saturating_sub(1);
        let row = &mut self.selected[self.focus.index()];
        *row = if up {
            row.saturating_sub(1)
        } else {
            (*row + 1).min(last)
        };
        self.scroll = 0;
    }

    /// Scrolls the details pane down (`true`) or up by `rows`, never past
    /// its last row, so that scrolling back takes effect at once.
    pub fn scroll(&mut self, down: bool, rows: usize) {
        let last = self.detail_rows().saturating_sub(1);
        self.scroll = if down {
            self.scroll.saturating_add(rows).min(last)
        } else {
            self.scroll.saturating_sub(rows)
        };
    }
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;

    use super::*;
    use crate::{
        config::Submit,
        drill::Mode,
        history::History,
        problem::Problem,
        stats::{Scoring, report::Fingerprint},
        store::{Attempt, SessionRecord},
    };

    /// A dashboard over one session of `attempts` attempts.
    fn dashboard(attempts: usize) -> Dashboard {
        let now = "2026-11-13T12:00:00Z".parse().unwrap();
        let problem = Problem::Add { lhs: 2, rhs: 3 };
        let attempt = Attempt {
            ts: now,
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
        let session = SessionRecord {
            id: "s".into(),
            start: now,
            end: now,
            duration: 120,
            score: attempts as u32,
            completed: true,
            submit: Submit::Auto,
            mode: Mode::Jiyu,
            problems: serde_json::Value::Null,
        };
        let history = History {
            attempts: vec![attempt; attempts],
            sessions: vec![session],
            damaged: 0,
        };
        let fingerprint = Fingerprint::of(&history.sessions[0]);
        Dashboard::new(Report::build(
            &history,
            &Scoring::default(),
            &fingerprint,
            now,
            &TimeZone::UTC,
        ))
    }

    #[test]
    fn focus_cycles_and_selection_stays_in_range() {
        let mut d = dashboard(0);
        d.switch(false);
        assert_eq!(d.focus, Side::Weak);
        d.switch(true);
        assert_eq!(d.focus, Side::Summary);
        d.focus(2);
        assert_eq!(d.focus, Side::Kinds);
        d.focus(9);
        assert_eq!(d.focus, Side::Kinds);
        // No kinds practised: the selection cannot move.
        d.step(false);
        assert_eq!(d.selected(Side::Kinds), 0);
    }

    #[test]
    fn scrolling_stops_at_the_last_row() {
        let mut d = dashboard(25);
        d.focus(3);
        for _ in 0..5 {
            d.scroll(true, 10);
        }
        assert_eq!(d.scroll, 24);
        // One step back shows, however far past the end was asked for.
        d.scroll(false, 10);
        assert_eq!(d.scroll, 14);
        d.scroll(false, 100);
        assert_eq!(d.scroll, 0);
        // Moving the selection, or the focus, starts from the top again.
        d.scroll(true, 10);
        d.step(true);
        assert_eq!(d.scroll, 0);
        d.scroll(true, 10);
        d.switch(true);
        assert_eq!(d.scroll, 0);
        // Lists with no table to scroll stay put.
        d.focus(2);
        d.scroll(true, 10);
        assert_eq!(d.scroll, 0);
    }
}
