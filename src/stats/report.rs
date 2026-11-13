//! The numbers the dashboard shows, computed from history.

use std::collections::{BTreeMap, HashMap};

use jiff::{SignedDuration, Timestamp, civil::Date, tz::TimeZone};

use super::{
    features::{Feature, features},
    model::{Baselines, Scoring, Skill, SkillModel, median},
};
use crate::{
    config::Submit,
    drill::Mode,
    history::History,
    problem::{Kind, Problem},
    store::{Attempt, SessionRecord},
};

/// Attempts this recent count as recent.
pub const RECENT: SignedDuration = SignedDuration::from_hours(7 * 24);

/// Questions seen fewer times than this are not listed as hard, and
/// skills seen fewer times than `MIN_SKILL` are not listed as weak: a
/// skill covers many questions, so it has to earn its place with more.
const MIN_ITEM: u32 = 2;
const MIN_SKILL: u32 = 5;

/// Most weak spots to list, and most hard questions for each kind.
const WEAK_SPOTS: usize = 100;
const HARDEST: usize = 10;

/// Everything the dashboard shows, worked out once when it opens so that
/// drawing it is only drawing.
#[derive(Debug, Clone)]
pub struct Report {
    pub now: Timestamp,
    pub tz: TimeZone,
    pub today: Date,
    pub attempts: usize,
    pub damaged: usize,
    /// In menu order.
    pub kinds: Vec<KindReport>,
    /// Newest first.
    pub sessions: Vec<SessionReport>,
    /// Scores of finished sessions with the current settings.
    pub trend: Vec<u32>,
    /// The average struggle, which every skill is measured against.
    pub overall: f64,
    pub speed: Speed,
    pub grid: Vec<Vec<Option<Skill>>>,
    pub activity: BTreeMap<Date, u32>,
    /// Weakest first.
    pub weak: Vec<WeakSpot>,
}

/// One kind of problem in depth.
#[derive(Debug, Clone)]
pub struct KindReport {
    pub stats: KindStats,
    pub speed: Speed,
    /// What its skills cost in time, dearest first.
    pub costs: Vec<Cost>,
    /// Its questions that have been the most struggle.
    pub hardest: Vec<(Problem, Skill)>,
}

/// A session and the attempts made in it, as indices into the history.
#[derive(Debug, Clone)]
pub struct SessionReport {
    pub record: SessionRecord,
    pub attempts: Vec<usize>,
}

impl Report {
    pub fn build(
        history: &History,
        scoring: &Scoring,
        fingerprint: &Fingerprint,
        now: Timestamp,
        tz: &TimeZone,
    ) -> Self {
        let attempts = &history.attempts;
        let model = SkillModel::build(attempts, scoring, now);
        let baselines = Baselines::build(attempts, scoring.by_submit);
        let since = now - RECENT;
        let mut costs = costs(attempts, &baselines);
        let mut hardest = hardest(&model, MIN_ITEM, HARDEST);
        let kinds = by_kind(attempts, since)
            .into_iter()
            .map(|stats| {
                let of_kind = attempts.iter().filter(|a| a.problem.kind() == stats.kind);
                KindReport {
                    speed: Speed::of(of_kind, since),
                    costs: costs.remove(&stats.kind).unwrap_or_default(),
                    hardest: hardest.remove(&stats.kind).unwrap_or_default(),
                    stats,
                }
            })
            .collect();
        Self {
            now,
            tz: tz.clone(),
            today: now.to_zoned(tz.clone()).date(),
            attempts: attempts.len(),
            damaged: history.damaged,
            kinds,
            sessions: sessions(history),
            trend: trend(&history.sessions, fingerprint),
            overall: model.overall(),
            speed: Speed::of(attempts, since),
            grid: fact_grid(&model),
            activity: activity(attempts, tz),
            weak: weak_spots(attempts, &model, WEAK_SPOTS),
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

/// Every session, newest first, with its attempts.
fn sessions(history: &History) -> Vec<SessionReport> {
    let mut by_session: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, a) in history.attempts.iter().enumerate() {
        by_session.entry(&a.session).or_default().push(i);
    }
    let mut sessions: Vec<SessionReport> = history
        .sessions
        .iter()
        .map(|record| SessionReport {
            record: record.clone(),
            attempts: by_session.remove(record.id.as_str()).unwrap_or_default(),
        })
        .collect();
    sessions.sort_by_key(|s| std::cmp::Reverse(s.record.start));
    sessions
}

/// How one kind of problem has gone.
#[derive(Debug, Clone, PartialEq)]
pub struct KindStats {
    pub kind: Kind,
    pub attempts: u32,
    pub correct: u32,
    /// Over correct answers, in milliseconds.
    pub median_ms: f64,
    pub p90_ms: f64,
    /// The median over recent correct answers, if there are any.
    pub recent_median_ms: Option<f64>,
}

impl KindStats {
    pub fn accuracy(&self) -> f64 {
        f64::from(self.correct) / f64::from(self.attempts.max(1))
    }
}

/// Stats for every kind practised, in menu order. Attempts from `since`
/// on count as recent.
pub fn by_kind<'a>(
    attempts: impl IntoIterator<Item = &'a Attempt>,
    since: Timestamp,
) -> Vec<KindStats> {
    #[derive(Default)]
    struct Group {
        attempts: u32,
        times: Vec<u64>,
        recent: Vec<u64>,
    }
    let mut groups: BTreeMap<Kind, Group> = BTreeMap::new();
    for a in attempts {
        let group = groups.entry(a.problem.kind()).or_default();
        group.attempts += 1;
        if a.correct {
            group.times.push(a.ms);
            if a.ts >= since {
                group.recent.push(a.ms);
            }
        }
    }
    groups
        .into_iter()
        .map(|(kind, mut group)| KindStats {
            kind,
            attempts: group.attempts,
            correct: group.times.len() as u32,
            median_ms: median(&mut group.times),
            p90_ms: percentile(&group.times, 90.0),
            recent_median_ms: (!group.recent.is_empty()).then(|| median(&mut group.recent)),
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

/// How fast correct answers come, over all time and recently.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Speed {
    pub all: Vec<(f64, f64)>,
    pub recent: Vec<(f64, f64)>,
}

impl Speed {
    /// The percentile curves of `attempts`, and of those from `since` on.
    pub fn of<'a>(
        attempts: impl IntoIterator<Item = &'a Attempt, IntoIter: Clone>,
        since: Timestamp,
    ) -> Self {
        let attempts = attempts.into_iter();
        Self {
            all: percentile_curve(attempts.clone()),
            recent: percentile_curve(attempts.filter(|a| a.ts >= since)),
        }
    }

    /// Seconds at the `p`th percentile over all time.
    pub fn at(&self, p: usize) -> f64 {
        self.all.get(p).map_or(0.0, |&(_, secs)| secs)
    }
}

/// What makes sessions comparable: the same length, submit mode and
/// problem settings. Scores are only compared between sessions that
/// share one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    pub duration: u64,
    pub submit: Submit,
    pub mode: Mode,
    problems: String,
}

impl Fingerprint {
    pub fn new(duration: u64, submit: Submit, mode: Mode, problems: &serde_json::Value) -> Self {
        Self {
            duration,
            submit,
            mode,
            problems: problems.to_string(),
        }
    }

    pub fn of(session: &SessionRecord) -> Self {
        Self::new(
            session.duration,
            session.submit,
            session.mode,
            &session.problems,
        )
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

/// For each kind, its `n` questions that have been the most struggle,
/// among those seen at least `min_count` times, hardest first.
pub fn hardest(
    model: &SkillModel,
    min_count: u32,
    n: usize,
) -> BTreeMap<Kind, Vec<(Problem, Skill)>> {
    let mut kinds: BTreeMap<Kind, Vec<(Problem, Skill)>> = BTreeMap::new();
    for (feature, skill) in model.features() {
        if let Feature::Item(problem) = feature
            && skill.count >= min_count
        {
            kinds
                .entry(problem.kind())
                .or_default()
                .push((problem, skill));
        }
    }
    for items in kinds.values_mut() {
        items.sort_by(|a, b| b.1.score.total_cmp(&a.1.score).then(a.0.cmp(&b.0)));
        items.truncate(n);
    }
    kinds
}

/// A question or a skill that is more of a struggle than average.
#[derive(Debug, Clone, PartialEq)]
pub struct WeakSpot {
    pub feature: Feature,
    pub skill: Skill,
    /// The attempts that exercised it, newest first, as indices into the
    /// history.
    pub attempts: Vec<usize>,
    pub correct: u32,
    /// Over correct answers, in milliseconds.
    pub median_ms: f64,
}

impl WeakSpot {
    pub fn accuracy(&self) -> f64 {
        f64::from(self.correct) / (self.attempts.len().max(1) as f64)
    }
}

/// The `n` questions and skills that are the most struggle, weakest
/// first. Only those above the average count, and only once seen often
/// enough to mean something.
pub fn weak_spots(attempts: &[Attempt], model: &SkillModel, n: usize) -> Vec<WeakSpot> {
    let overall = model.overall();
    let mut weak: Vec<(Feature, Skill)> = model
        .features()
        .filter(|(feature, skill)| {
            let min = match feature {
                Feature::Item(_) => MIN_ITEM,
                _ => MIN_SKILL,
            };
            skill.count >= min && skill.score > overall
        })
        .collect();
    weak.sort_by(|a, b| b.1.score.total_cmp(&a.1.score).then(a.0.cmp(&b.0)));
    weak.truncate(n);

    let rank: HashMap<Feature, usize> =
        weak.iter().enumerate().map(|(i, (f, _))| (*f, i)).collect();
    let mut found: Vec<(Vec<usize>, Vec<u64>)> = vec![Default::default(); weak.len()];
    for (i, a) in attempts.iter().enumerate().rev() {
        for feature in features(&a.problem) {
            if let Some(&at) = rank.get(&feature) {
                found[at].0.push(i);
                if a.correct {
                    found[at].1.push(a.ms);
                }
            }
        }
    }
    weak.into_iter()
        .zip(found)
        .map(|((feature, skill), (attempts, mut times))| WeakSpot {
            feature,
            skill,
            attempts,
            correct: times.len() as u32,
            median_ms: median(&mut times),
        })
        .collect()
}

/// What a skill costs: how much longer correct answers that use it take,
/// as a multiple of the usual time for problems of their kind and size.
#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
    pub feature: Feature,
    pub count: u32,
    /// The geometric mean of time over the usual time.
    pub factor: f64,
}

/// Correct answers a skill needs before its cost is worth showing.
const MIN_FOR_COST: u32 = 3;

/// The cost of each skill within each kind of problem, dearest first.
/// Single questions and single facts are left out: the first have the
/// weak spots list, and the second the times-table grid.
pub fn costs(attempts: &[Attempt], baselines: &Baselines) -> BTreeMap<Kind, Vec<Cost>> {
    let mut groups: BTreeMap<(Kind, Feature), (u32, f64)> = BTreeMap::new();
    for a in attempts.iter().filter(|a| a.correct) {
        let Some(usual) = baselines.usual(&a.problem, a.submit) else {
            continue;
        };
        let log = (a.ms.max(1) as f64 / usual.max(1.0)).ln();
        for feature in features(&a.problem) {
            if matches!(feature, Feature::Item(_) | Feature::Fact(..)) {
                continue;
            }
            let (count, logs) = groups.entry((a.problem.kind(), feature)).or_default();
            *count += 1;
            *logs += log;
        }
    }
    let mut kinds: BTreeMap<Kind, Vec<Cost>> = BTreeMap::new();
    for ((kind, feature), (count, logs)) in groups {
        if count >= MIN_FOR_COST {
            kinds.entry(kind).or_default().push(Cost {
                feature,
                count,
                factor: (logs / f64::from(count)).exp(),
            });
        }
    }
    for costs in kinds.values_mut() {
        costs.sort_by(|a, b| {
            b.factor
                .total_cmp(&a.factor)
                .then(a.feature.cmp(&b.feature))
        });
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::Problem;

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
        let stats = by_kind(&attempts, Timestamp::MAX);
        assert_eq!(stats.len(), 2);
        let add = &stats[0];
        assert_eq!(add.kind, Kind::Add);
        assert_eq!((add.attempts, add.correct), (3, 2));
        assert_eq!(add.median_ms, 2000.0);
        assert!((add.accuracy() - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(add.recent_median_ms, None);
        // Everything is recent when the week starts at the dawn of time.
        let stats = by_kind(&attempts, Timestamp::MIN);
        assert_eq!(stats[0].recent_median_ms, Some(2000.0));
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
            mode: Mode::Jiyu,
            problems: problems.clone(),
        };
        let sessions = [
            session(30, true, 120),
            session(5, false, 120),
            session(80, true, 300),
            session(34, true, 120),
        ];
        let fp = Fingerprint::new(120, Submit::Auto, Mode::Jiyu, &problems);
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
        // Too few to show, and a miss, which has no time worth counting.
        attempts.push(attempt(add(13, 37), 9000, true));
        attempts.push(attempt(add(12, 35), 60_000, false));
        let baselines = Baselines::build(&attempts, true);
        let costs = costs(&attempts, &baselines);
        assert_eq!(costs.keys().collect::<Vec<_>>(), [&Kind::Add]);
        let cost = |feature| {
            costs[&Kind::Add]
                .iter()
                .find(|c| c.feature == feature)
                .unwrap_or_else(|| panic!("no cost for {feature}"))
        };
        let (none, one) = (cost(Feature::Carries(0)), cost(Feature::Carries(1)));
        assert_eq!((none.count, one.count), (5, 6));
        assert!(one.factor > 1.9 * none.factor);
        // Dearest first, with no single questions and nothing seen once.
        let factors: Vec<f64> = costs[&Kind::Add].iter().map(|c| c.factor).collect();
        assert!(factors.is_sorted_by(|a, b| a >= b), "{factors:?}");
        for c in &costs[&Kind::Add] {
            assert!(!matches!(c.feature, Feature::Item(_)) && c.count >= MIN_FOR_COST);
        }
    }

    fn now() -> Timestamp {
        "2026-11-13T10:00:00Z".parse().unwrap()
    }

    fn mul(lhs: i64, rhs: i64) -> Problem {
        Problem::Mul { lhs, rhs }
    }

    #[test]
    fn grid_and_hardest_come_from_the_model() {
        let mut attempts = Vec::new();
        for _ in 0..3 {
            attempts.push(attempt(mul(7, 8), 4000, true));
            attempts.push(attempt(mul(3, 4), 1000, true));
        }
        attempts.push(attempt(mul(9, 9), 60_000, true));
        let model = SkillModel::build(&attempts, &Scoring::default(), now());
        let grid = fact_grid(&model);
        assert_eq!(grid.len(), 8);
        assert!(grid[7 - 2][8 - 2].is_some());
        assert_eq!(grid[7 - 2][8 - 2], grid[8 - 2][7 - 2]);
        assert!(grid[5 - 2][5 - 2].is_none());
        // 9 × 9 was slower still, but once is not enough to say so.
        let hardest = hardest(&model, 2, 1);
        assert_eq!(hardest[&Kind::Mul].len(), 1);
        assert_eq!(hardest[&Kind::Mul][0].0, mul(7, 8));
    }

    #[test]
    fn weak_spots_are_skills_and_questions_above_average() {
        // Multiplying by 7 is slow whatever it multiplies; 6 × 8 is slow
        // on its own; everything else, which is most things, is quick.
        let mut attempts = Vec::new();
        for round in 0..3 {
            for b in 2..=9 {
                attempts.push(attempt(mul(7, 10 * b + round), 6000, true));
                for quick in [3, 4, 5] {
                    attempts.push(attempt(mul(quick, 10 * b + round), 2000, true));
                }
            }
            attempts.push(attempt(mul(6, 8), 5000, round > 0));
            for quick in [mul(2, 4), mul(2, 3), mul(3, 3)] {
                attempts.push(attempt(quick, 2000, true));
            }
        }
        let model = SkillModel::build(&attempts, &Scoring::default(), now());
        let weak = weak_spots(&attempts, &model, 100);
        let names: Vec<String> = weak.iter().map(|w| w.feature.to_string()).collect();
        // The skill leads; the facts it drags down and the one slow
        // question follow.
        assert_eq!(names[0], "multiplying by 7", "{names:?}");
        assert!(names.contains(&"6 × 8".to_owned()), "{names:?}");
        assert!(!names.contains(&"multiplying by 3".to_owned()), "{names:?}");
        assert!(!names.contains(&"2 × 4".to_owned()), "{names:?}");
        // No question asked once is listed, however slow.
        assert!(!names.iter().any(|n| n.starts_with("7 × ")), "{names:?}");
        assert!(
            weak.windows(2)
                .all(|w| w[0].skill.score >= w[1].skill.score)
        );

        let sevens = weak
            .iter()
            .find(|w| w.feature == Feature::Times(7))
            .unwrap();
        assert_eq!(sevens.attempts.len(), 24);
        assert!(sevens.attempts.is_sorted_by(|a, b| a > b), "newest first");
        assert_eq!((sevens.correct, sevens.median_ms), (24, 6000.0));
        let six_eights = weak
            .iter()
            .find(|w| w.feature.to_string() == "6 × 8")
            .unwrap();
        assert!((six_eights.accuracy() - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(weak_spots(&attempts, &model, 1).len(), 1);
    }

    #[test]
    fn report_gathers_each_session_and_kind() {
        let mut attempts = vec![
            attempt(add(12, 34), 1000, true),
            attempt(mul(7, 8), 3000, true),
            attempt(add(18, 34), 2000, true),
        ];
        attempts[2].session = "later".into();
        let session = |id: &str, start: &str| SessionRecord {
            id: id.into(),
            start: start.parse().unwrap(),
            end: start.parse().unwrap(),
            duration: 120,
            score: 1,
            completed: true,
            submit: Submit::Auto,
            mode: Mode::Jiyu,
            problems: serde_json::Value::Null,
        };
        let history = History {
            attempts,
            sessions: vec![
                session("s", "2026-11-12T10:00:00Z"),
                session("later", "2026-11-13T09:00:00Z"),
                session("empty", "2026-11-11T09:00:00Z"),
            ],
            damaged: 3,
        };
        let fingerprint = Fingerprint::new(120, Submit::Auto, Mode::Jiyu, &serde_json::Value::Null);
        let report = Report::build(
            &history,
            &Scoring::default(),
            &fingerprint,
            now(),
            &TimeZone::UTC,
        );
        assert_eq!((report.attempts, report.damaged), (3, 3));
        let sessions: Vec<(&str, &[usize])> = report
            .sessions
            .iter()
            .map(|s| (s.record.id.as_str(), s.attempts.as_slice()))
            .collect();
        assert_eq!(
            sessions,
            [("later", &[2][..]), ("s", &[0, 1]), ("empty", &[])]
        );
        assert_eq!(report.trend, [1, 1, 1]);
        let kinds: Vec<Kind> = report.kinds.iter().map(|k| k.stats.kind).collect();
        assert_eq!(kinds, [Kind::Add, Kind::Mul]);
        assert_eq!(report.kinds[0].speed.all.len(), 101);
        assert_eq!(report.kinds[0].speed.at(100), 2.0);
        assert_eq!(report.speed.at(100), 3.0);
    }

    fn report_with(days: &[(&str, u32)]) -> Report {
        let fingerprint = Fingerprint::new(120, Submit::Auto, Mode::Jiyu, &serde_json::Value::Null);
        let mut report = Report::build(
            &History::default(),
            &Scoring::default(),
            &fingerprint,
            "2026-11-13T12:00:00Z".parse().unwrap(),
            &TimeZone::UTC,
        );
        report.activity = days.iter().map(|(d, n)| (d.parse().unwrap(), *n)).collect();
        report
    }

    #[test]
    fn streaks() {
        assert_eq!(report_with(&[]).streak(), 0);
        let r = report_with(&[("2026-11-13", 5), ("2026-11-12", 1), ("2026-11-10", 3)]);
        assert_eq!(r.streak(), 2);
        // Not having practised yet today does not break the streak.
        let r = report_with(&[("2026-11-12", 1), ("2026-11-11", 1)]);
        assert_eq!(r.streak(), 2);
        let r = report_with(&[("2026-11-11", 1)]);
        assert_eq!(r.streak(), 0);
    }
}
