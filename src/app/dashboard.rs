//! The statistics dashboard: lists on the left, and on the right the
//! details of whatever is selected in the focused list.

use std::collections::BTreeMap;

use jiff::{SignedDuration, Timestamp, civil::Date, tz::TimeZone};

use crate::{
    config::Config,
    history::History,
    problem::Op,
    stats::{
        SkillModel,
        features::Feature,
        model::{Baselines, Skill},
        report::{self, Cost, Fingerprint, KindStats},
    },
    store::SessionRecord,
};

/// Attempts this recent count as recent.
pub const RECENT: SignedDuration = SignedDuration::from_hours(7 * 24);

/// Items seen fewer times than this are not listed as weak spots.
const MIN_SEEN: u32 = 2;

/// Most weak spots to list.
const WEAK_SPOTS: usize = 100;

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

/// Everything the dashboard shows that is costly to work out, computed
/// when it opens.
#[derive(Debug, Clone)]
pub struct Report {
    pub now: Timestamp,
    pub tz: TimeZone,
    pub today: Date,
    pub attempts: usize,
    pub damaged: usize,
    pub kinds: Vec<KindStats>,
    /// Newest first.
    pub sessions: Vec<SessionRecord>,
    /// Scores of finished sessions with the current settings.
    pub trend: Vec<u32>,
    pub overall: f64,
    pub grid: Vec<Vec<Option<Skill>>>,
    pub carries: Vec<Cost>,
    pub borrows: Vec<Cost>,
    pub activity: BTreeMap<Date, u32>,
    pub hardest: Vec<(Feature, Skill)>,
}

impl Report {
    pub fn build(history: &History, config: &Config, now: Timestamp, tz: &TimeZone) -> Self {
        let attempts = &history.attempts;
        let model = SkillModel::build(attempts, &config.scoring, now);
        let baselines = Baselines::build(attempts, config.scoring.by_submit);
        let problems = serde_json::to_value(&config.problems).unwrap_or_default();
        let session = &config.session;
        let fingerprint =
            Fingerprint::new(session.duration, session.submit, session.mode, &problems);
        Self {
            now,
            tz: tz.clone(),
            today: now.to_zoned(tz.clone()).date(),
            attempts: attempts.len(),
            damaged: history.damaged,
            kinds: report::by_kind(attempts),
            sessions: newest_first(&history.sessions),
            trend: report::trend(&history.sessions, &fingerprint),
            overall: model.overall(),
            grid: report::fact_grid(&model),
            carries: report::carry_costs(attempts, &baselines, Op::Add),
            borrows: report::carry_costs(attempts, &baselines, Op::Sub),
            activity: report::activity(attempts, tz),
            hardest: report::hardest(&model, MIN_SEEN, WEAK_SPOTS),
        }
    }

    /// Days in a row with practice, up to today, or up to yesterday if
    /// today has none yet.
    pub fn streak(&self) -> u32 {
        let practised = |d: Date| self.activity.get(&d).is_some_and(|&n| n > 0);
        let mut day = self.today;
        if !practised(day) {
            match day.yesterday() {
                Ok(d) => day = d,
                Err(_) => return 0,
            }
        }
        let mut streak = 0;
        while practised(day) {
            streak += 1;
            match day.yesterday() {
                Ok(d) => day = d,
                Err(_) => break,
            }
        }
        streak
    }

    /// How many rows the list for `side` has.
    pub fn len(&self, side: Side) -> usize {
        match side {
            Side::Summary => 1,
            Side::Kinds => self.kinds.len(),
            Side::Sessions => self.sessions.len(),
            Side::Weak => self.hardest.len(),
        }
    }
}

fn newest_first(sessions: &[SessionRecord]) -> Vec<SessionRecord> {
    let mut sessions = sessions.to_vec();
    sessions.sort_by_key(|s| std::cmp::Reverse(s.start));
    sessions
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub focus: Side,
    /// The selected row of each list, indexed like `Side::ALL`.
    selected: [usize; 4],
    /// How far the details pane is scrolled.
    pub scroll: u16,
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

    /// Moves the selection in the focused list up (`true`) or down.
    pub fn step(&mut self, up: bool) {
        let last = self.report.len(self.focus).saturating_sub(1);
        let row = &mut self.selected[self.focus.index()];
        *row = if up {
            row.saturating_sub(1)
        } else {
            (*row + 1).min(last)
        };
        self.scroll = 0;
    }

    /// Scrolls the details pane down (`true`) or up by `rows`.
    pub fn scroll(&mut self, down: bool, rows: u16) {
        self.scroll = if down {
            self.scroll.saturating_add(rows)
        } else {
            self.scroll.saturating_sub(rows)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(days: &[(&str, u32)]) -> Report {
        let mut r = Report::build(
            &History::default(),
            &Config::default(),
            "2026-11-13T12:00:00Z".parse().unwrap(),
            &TimeZone::UTC,
        );
        r.activity = days.iter().map(|(d, n)| (d.parse().unwrap(), *n)).collect();
        r
    }

    #[test]
    fn focus_cycles_and_selection_stays_in_range() {
        let mut d = Dashboard::new(report(&[]));
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
        d.scroll(true, 5);
        d.step(true);
        assert_eq!(d.scroll, 0);
    }

    #[test]
    fn streaks() {
        assert_eq!(report(&[]).streak(), 0);
        let r = report(&[("2026-11-13", 5), ("2026-11-12", 1), ("2026-11-10", 3)]);
        assert_eq!(r.streak(), 2);
        // Not having practised yet today does not break the streak.
        let r = report(&[("2026-11-12", 1), ("2026-11-11", 1)]);
        assert_eq!(r.streak(), 2);
        let r = report(&[("2026-11-11", 1)]);
        assert_eq!(r.streak(), 0);
    }
}
