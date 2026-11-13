//! The statistics dashboard: tabs of numbers about past practice.

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
};

/// Attempts this recent count as recent on the speed tab.
const RECENT: SignedDuration = SignedDuration::from_hours(7 * 24);

/// Items seen fewer times than this are not listed as weak spots.
const MIN_SEEN: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Overview,
    Speed,
    Facts,
    Activity,
    Weak,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Overview,
        Tab::Speed,
        Tab::Facts,
        Tab::Activity,
        Tab::Weak,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Speed => "Speed",
            Tab::Facts => "Facts",
            Tab::Activity => "Activity",
            Tab::Weak => "Weak spots",
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    fn step(self, by: isize) -> Tab {
        let n = Tab::ALL.len() as isize;
        Tab::ALL[(self.index() as isize + by).rem_euclid(n) as usize]
    }
}

/// Everything the dashboard shows, worked out when it opens.
#[derive(Debug, Clone)]
pub struct Report {
    pub attempts: usize,
    pub sessions: usize,
    pub damaged: usize,
    pub kinds: Vec<KindStats>,
    /// Scores of finished sessions with the current settings.
    pub trend: Vec<u32>,
    pub curve_all: Vec<(f64, f64)>,
    pub curve_recent: Vec<(f64, f64)>,
    pub overall: f64,
    pub grid: Vec<Vec<Option<Skill>>>,
    pub carries: Vec<Cost>,
    pub borrows: Vec<Cost>,
    pub activity: BTreeMap<Date, u32>,
    pub today: Date,
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
        let since = now - RECENT;
        Self {
            attempts: attempts.len(),
            sessions: history.sessions.len(),
            damaged: history.damaged,
            kinds: report::by_kind(attempts),
            trend: report::trend(&history.sessions, &fingerprint),
            curve_all: report::percentile_curve(attempts),
            curve_recent: report::percentile_curve(attempts.iter().filter(|a| a.ts >= since)),
            overall: model.overall(),
            grid: report::fact_grid(&model),
            carries: report::carry_costs(attempts, &baselines, Op::Add),
            borrows: report::carry_costs(attempts, &baselines, Op::Sub),
            activity: report::activity(attempts, tz),
            today: now.to_zoned(tz.clone()).date(),
            hardest: report::hardest(&model, MIN_SEEN, 20),
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
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub tab: Tab,
    pub report: Report,
}

impl Dashboard {
    pub fn new(report: Report) -> Self {
        Self {
            tab: Tab::default(),
            report,
        }
    }

    pub fn next(&mut self) {
        self.tab = self.tab.step(1);
    }

    pub fn previous(&mut self) {
        self.tab = self.tab.step(-1);
    }

    /// Jumps to the tab numbered `n`, counting from 1.
    pub fn select(&mut self, n: usize) {
        if let Some(&tab) = n.checked_sub(1).and_then(|i| Tab::ALL.get(i)) {
            self.tab = tab;
        }
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
    fn tabs_wrap() {
        let mut d = Dashboard::new(report(&[]));
        d.previous();
        assert_eq!(d.tab, Tab::Weak);
        d.next();
        assert_eq!(d.tab, Tab::Overview);
        d.select(3);
        assert_eq!(d.tab, Tab::Facts);
        d.select(9);
        assert_eq!(d.tab, Tab::Facts);
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
