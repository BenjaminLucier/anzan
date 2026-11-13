//! The numbers the dashboard shows, computed from history.

use std::collections::BTreeMap;

use jiff::{civil::Date, tz::TimeZone};

use super::{
    features::{Feature, borrows, carries},
    model::{Baselines, Skill, SkillModel, median},
};
use crate::{
    config::Submit,
    problem::{Kind, Op},
    store::{Attempt, SessionRecord},
};

/// How one kind of problem has gone.
#[derive(Debug, Clone, PartialEq)]
pub struct KindStats {
    pub kind: Kind,
    pub attempts: u32,
    pub correct: u32,
    /// Over correct answers, in milliseconds.
    pub median_ms: f64,
    pub p90_ms: f64,
}

impl KindStats {
    pub fn accuracy(&self) -> f64 {
        f64::from(self.correct) / f64::from(self.attempts.max(1))
    }
}

/// Stats for every kind practised, in menu order.
pub fn by_kind<'a>(attempts: impl IntoIterator<Item = &'a Attempt>) -> Vec<KindStats> {
    let mut groups: BTreeMap<Kind, (u32, Vec<u64>)> = BTreeMap::new();
    for a in attempts {
        let (count, times) = groups.entry(a.problem.kind()).or_default();
        *count += 1;
        if a.correct {
            times.push(a.ms);
        }
    }
    groups
        .into_iter()
        .map(|(kind, (attempts, mut times))| {
            let correct = times.len() as u32;
            let median_ms = median(&mut times);
            KindStats {
                kind,
                attempts,
                correct,
                median_ms,
                p90_ms: percentile(&times, 90.0),
            }
        })
        .collect()
}

/// The `p`th percentile of `sorted`, interpolating between neighbours.
pub fn percentile(sorted: &[u64], p: f64) -> f64 {
    match sorted {
        [] => 0.0,
        [only] => *only as f64,
        _ => {
            let rank = p.clamp(0.0, 100.0) / 100.0 * (sorted.len() - 1) as f64;
            let (lo, hi) = (rank.floor() as usize, rank.ceil() as usize);
            let frac = rank - lo as f64;
            sorted[lo] as f64 * (1.0 - frac) + sorted[hi] as f64 * frac
        }
    }
}

/// Response time against percentile, from 0 to 100, over correct answers:
/// the point (50, 2.1) means half the answers took 2.1 seconds or less.
pub fn percentile_curve<'a>(attempts: impl IntoIterator<Item = &'a Attempt>) -> Vec<(f64, f64)> {
    let mut times: Vec<u64> = attempts
        .into_iter()
        .filter(|a| a.correct)
        .map(|a| a.ms)
        .collect();
    if times.is_empty() {
        return Vec::new();
    }
    times.sort_unstable();
    (0..=100)
        .map(|p| (f64::from(p), percentile(&times, f64::from(p)) / 1000.0))
        .collect()
}

/// What makes sessions comparable: the same length, submit mode and
/// problem settings. Scores are only compared between sessions that
/// share one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    pub duration: u64,
    pub submit: Submit,
    problems: String,
}

impl Fingerprint {
    pub fn new(duration: u64, submit: Submit, problems: &serde_json::Value) -> Self {
        Self {
            duration,
            submit,
            problems: problems.to_string(),
        }
    }

    pub fn of(session: &SessionRecord) -> Self {
        Self::new(session.duration, session.submit, &session.problems)
    }
}

/// Scores of completed sessions with `fingerprint`, oldest first.
pub fn trend<'a>(
    sessions: impl IntoIterator<Item = &'a SessionRecord>,
    fingerprint: &Fingerprint,
) -> Vec<u32> {
    sessions
        .into_iter()
        .filter(|s| s.completed && Fingerprint::of(s) == *fingerprint)
        .map(|s| s.score)
        .collect()
}

/// Attempts per day, in `tz`.
pub fn activity<'a>(
    attempts: impl IntoIterator<Item = &'a Attempt>,
    tz: &TimeZone,
) -> BTreeMap<Date, u32> {
    let mut days = BTreeMap::new();
    for a in attempts {
        *days.entry(a.ts.to_zoned(tz.clone()).date()).or_default() += 1;
    }
    days
}

/// The smallest and largest digit in the times-table grid.
pub const GRID: std::ops::RangeInclusive<u8> = 2..=9;

/// Struggle with each single-digit multiplication fact, or `None` for
/// facts never seen. Indexed from 2, and symmetric.
pub fn fact_grid(model: &SkillModel) -> Vec<Vec<Option<Skill>>> {
    GRID.map(|a| {
        GRID.map(|b| {
            let skill = model.feature(&Feature::Fact(a.min(b), a.max(b)));
            (skill.count > 0).then_some(skill)
        })
        .collect()
    })
    .collect()
}

/// The `n` individual questions that have been the most struggle, among
/// those seen at least `min_count` times.
pub fn hardest(model: &SkillModel, min_count: u32, n: usize) -> Vec<(Feature, Skill)> {
    let mut items: Vec<_> = model
        .features()
        .filter(|(f, s)| matches!(f, Feature::Item(_)) && s.count >= min_count)
        .collect();
    items.sort_by(|a, b| b.1.score.total_cmp(&a.1.score).then(a.0.cmp(&b.0)));
    items.truncate(n);
    items
}

/// How much longer correct answers take, as a multiple of the usual time
/// for their kind, for each number of carries (in additions) or borrows
/// (in subtractions).
#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
    pub steps: u8,
    pub count: u32,
    /// The geometric mean of time over the usual time.
    pub factor: f64,
}

pub fn carry_costs(attempts: &[Attempt], baselines: &Baselines, op: Op) -> Vec<Cost> {
    let mut groups: BTreeMap<u8, (u32, f64)> = BTreeMap::new();
    for a in attempts.iter().filter(|a| a.correct) {
        let steps = match a.problem.arithmetic() {
            Some((Op::Add, l, r)) if op == Op::Add => carries(l, r),
            Some((Op::Sub, l, r)) if op == Op::Sub => borrows(l, r),
            _ => continue,
        };
        let Some(base) = baselines.median(a.problem.kind(), a.submit) else {
            continue;
        };
        let (count, logs) = groups.entry(steps).or_default();
        *count += 1;
        *logs += (a.ms.max(1) as f64 / base.max(1.0)).ln();
    }
    groups
        .into_iter()
        .map(|(steps, (count, logs))| Cost {
            steps,
            count,
            factor: (logs / f64::from(count)).exp(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;
    use crate::{problem::Problem, stats::Scoring};

    fn attempt(problem: Problem, ms: u64, correct: bool) -> Attempt {
        Attempt {
            ts: "2026-11-13T10:00:00Z".parse().unwrap(),
            session: "s".into(),
            problem,
            question: problem.to_string(),
            answer: problem.answer().to_string(),
            given: String::new(),
            correct,
            skipped: false,
            submit: Submit::Auto,
            ms,
            edits: 0,
        }
    }

    fn add(lhs: i64, rhs: i64) -> Problem {
        Problem::Add { lhs, rhs }
    }

    #[test]
    fn percentiles_interpolate() {
        assert_eq!(percentile(&[], 50.0), 0.0);
        assert_eq!(percentile(&[7], 90.0), 7.0);
        assert_eq!(percentile(&[1, 2, 3, 4, 5], 50.0), 3.0);
        assert_eq!(percentile(&[0, 10], 25.0), 2.5);
        assert_eq!(percentile(&[0, 10], 100.0), 10.0);
    }

    #[test]
    fn kind_stats_count_misses_but_time_only_hits() {
        let attempts = [
            attempt(add(1, 2), 1000, true),
            attempt(add(1, 3), 3000, true),
            attempt(add(1, 4), 9000, false),
            attempt(Problem::Mul { lhs: 2, rhs: 3 }, 500, true),
        ];
        let stats = by_kind(&attempts);
        assert_eq!(stats.len(), 2);
        let add = &stats[0];
        assert_eq!(add.kind, Kind::Add);
        assert_eq!((add.attempts, add.correct), (3, 2));
        assert_eq!(add.median_ms, 2000.0);
        assert!((add.accuracy() - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn curve_is_in_seconds_and_rises() {
        let attempts: Vec<_> = (1..=10)
            .map(|i| attempt(add(1, i), i as u64 * 1000, true))
            .collect();
        let curve = percentile_curve(&attempts);
        assert_eq!(curve.len(), 101);
        assert_eq!(curve[0], (0.0, 1.0));
        assert_eq!(curve[100], (100.0, 10.0));
        assert!(curve.windows(2).all(|w| w[0].1 <= w[1].1));
    }

    #[test]
    fn trend_keeps_matching_completed_sessions() {
        let problems = serde_json::json!({"kinds": ["add"]});
        let session = |score, completed, duration| SessionRecord {
            id: "s".into(),
            start: Timestamp::UNIX_EPOCH,
            end: Timestamp::UNIX_EPOCH,
            duration,
            score,
            completed,
            submit: Submit::Auto,
            problems: problems.clone(),
        };
        let sessions = [
            session(30, true, 120),
            session(5, false, 120),
            session(80, true, 300),
            session(34, true, 120),
        ];
        let fp = Fingerprint::new(120, Submit::Auto, &problems);
        assert_eq!(trend(&sessions, &fp), [30, 34]);
    }

    #[test]
    fn activity_counts_local_days() {
        let mut late = attempt(add(1, 2), 1000, true);
        late.ts = "2026-11-13T23:30:00Z".parse().unwrap();
        let attempts = [attempt(add(1, 2), 1000, true), late];
        let utc = activity(&attempts, &TimeZone::UTC);
        assert_eq!(utc.values().copied().collect::<Vec<_>>(), [2]);
        let tokyo = activity(&attempts, &TimeZone::fixed(jiff::tz::offset(9)));
        assert_eq!(tokyo.len(), 2);
    }

    #[test]
    fn carries_slow_additions_down() {
        let mut attempts = Vec::new();
        for _ in 0..5 {
            attempts.push(attempt(add(12, 34), 1000, true));
            attempts.push(attempt(add(18, 34), 2000, true));
        }
        let baselines = Baselines::build(&attempts, true);
        let costs = carry_costs(&attempts, &baselines, Op::Add);
        assert_eq!(costs.len(), 2);
        assert_eq!((costs[0].steps, costs[0].count), (0, 5));
        assert!((costs[1].factor / costs[0].factor - 2.0).abs() < 1e-9);
        assert!(carry_costs(&attempts, &baselines, Op::Sub).is_empty());
    }

    #[test]
    fn grid_and_hardest_come_from_the_model() {
        let now = "2026-11-13T10:00:00Z".parse().unwrap();
        let mut attempts = Vec::new();
        for _ in 0..3 {
            attempts.push(attempt(Problem::Mul { lhs: 7, rhs: 8 }, 4000, true));
            attempts.push(attempt(Problem::Mul { lhs: 3, rhs: 4 }, 1000, true));
        }
        let model = SkillModel::build(&attempts, &Scoring::default(), now);
        let grid = fact_grid(&model);
        assert_eq!(grid.len(), 8);
        assert!(grid[7 - 2][8 - 2].is_some());
        assert_eq!(grid[7 - 2][8 - 2], grid[8 - 2][7 - 2]);
        assert!(grid[5 - 2][5 - 2].is_none());
        let hardest = hardest(&model, 2, 1);
        assert_eq!(hardest[0].0, Feature::Item(Problem::Mul { lhs: 7, rhs: 8 }));
    }
}
