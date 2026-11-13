//! How much each skill is a struggle, learned from history.
//!
//! Every attempt gets a struggle score: the log of how much slower it was
//! than the usual time for its kind, plus penalties for a wrong answer, a
//! skip or backspaces. 0 is a typical attempt; ln 2 ≈ 0.7 took twice as
//! long. A skill's score is the average over the attempts that exercised
//! it, with recent attempts counting for more, pulled toward the overall
//! average when there are few attempts so one slip does not dominate.
//!
//! Everything is recomputed from the raw history, so changing `[scoring]`
//! rescores the past too.

use std::collections::HashMap;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::features::{Feature, features};
use crate::{
    config::Submit,
    problem::{Kind, Problem},
    store::Attempt,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Scoring {
    /// Added for a wrong answer: 1.5 is as bad as taking 4.5 times as long.
    pub wrong: f64,
    /// Added for a skip.
    pub skip: f64,
    /// Added for each backspace.
    pub edit: f64,
    /// Compare times only between attempts handed in the same way, since
    /// pressing Enter takes time of its own.
    pub by_submit: bool,
    /// How many attempts' worth of the overall average a skill starts
    /// with: higher trusts a few attempts less.
    pub prior: f64,
    /// Days for an attempt's weight to halve. 0 keeps every attempt at
    /// full weight.
    pub half_life: f64,
}

impl Default for Scoring {
    fn default() -> Self {
        Self {
            wrong: 1.5,
            skip: 2.0,
            edit: 0.15,
            by_submit: true,
            prior: 5.0,
            half_life: 30.0,
        }
    }
}

impl Scoring {
    pub fn validate(&self) -> Result<(), String> {
        let fields = [self.wrong, self.skip, self.edit, self.prior, self.half_life];
        if fields.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Err("`scoring` values must be finite and not negative".into());
        }
        Ok(())
    }
}

/// The median time for each kind of problem, and for each way of
/// submitting unless `by_submit` is off.
#[derive(Debug, Default)]
pub struct Baselines {
    medians: HashMap<(Kind, Option<Submit>), f64>,
    by_submit: bool,
}

impl Baselines {
    pub fn build(attempts: &[Attempt], by_submit: bool) -> Self {
        let mut times: HashMap<_, Vec<u64>> = HashMap::new();
        for a in attempts.iter().filter(|a| a.correct) {
            let key = (a.problem.kind(), by_submit.then_some(a.submit));
            times.entry(key).or_default().push(a.ms);
        }
        let medians = times
            .into_iter()
            .map(|(key, mut ms)| (key, median(&mut ms)))
            .collect();
        Self { medians, by_submit }
    }

    /// The usual time in milliseconds for `kind` handed in with `submit`,
    /// if any have been answered correctly.
    pub fn median(&self, kind: Kind, submit: Submit) -> Option<f64> {
        let key = (kind, self.by_submit.then_some(submit));
        self.medians.get(&key).copied()
    }
}

/// The median of `values`, which it sorts.
pub fn median(values: &mut [u64]) -> f64 {
    values.sort_unstable();
    match values.len() {
        0 => 0.0,
        n if n % 2 == 1 => values[n / 2] as f64,
        n => (values[n / 2 - 1] + values[n / 2]) as f64 / 2.0,
    }
}

/// How much of a struggle one attempt was. A miss never scores as fast,
/// however quickly it was given.
pub fn struggle(attempt: &Attempt, baseline_ms: f64, scoring: &Scoring) -> f64 {
    let time = (attempt.ms.max(1) as f64 / baseline_ms.max(1.0)).ln();
    let (time, miss) = match (attempt.correct, attempt.skipped) {
        (true, _) => (time, 0.0),
        (false, true) => (time.max(0.0), scoring.skip),
        (false, false) => (time.max(0.0), scoring.wrong),
    };
    time + miss + scoring.edit * f64::from(attempt.edits)
}

/// A weighted running total of struggle scores.
#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    weight: f64,
    sum: f64,
    count: u32,
}

impl Tally {
    fn add(&mut self, score: f64, weight: f64) {
        self.weight += weight;
        self.sum += score * weight;
        self.count += 1;
    }
}

/// What the model knows about one skill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skill {
    /// Average struggle, pulled toward the overall average when attempts
    /// are few. Higher is weaker.
    pub score: f64,
    /// Attempts that exercised it.
    pub count: u32,
}

#[derive(Debug, Default)]
pub struct SkillModel {
    overall: f64,
    prior: f64,
    features: HashMap<Feature, Tally>,
    kinds: HashMap<Kind, Tally>,
}

impl SkillModel {
    pub fn build(attempts: &[Attempt], scoring: &Scoring, now: Timestamp) -> Self {
        let baselines = Baselines::build(attempts, scoring.by_submit);
        let mut model = Self {
            prior: scoring.prior,
            ..Self::default()
        };
        let mut all = Tally::default();
        for a in attempts {
            let kind = a.problem.kind();
            // An attempt with nothing to compare against is typical.
            let baseline = baselines.median(kind, a.submit).unwrap_or(a.ms as f64);
            let score = struggle(a, baseline, scoring);
            let weight = recency(a.ts, now, scoring.half_life);
            all.add(score, weight);
            model.kinds.entry(kind).or_default().add(score, weight);
            for feature in features(&a.problem) {
                model
                    .features
                    .entry(feature)
                    .or_default()
                    .add(score, weight);
            }
        }
        model.overall = if all.weight > 0.0 {
            all.sum / all.weight
        } else {
            0.0
        };
        model
    }

    /// The average struggle over everything.
    pub fn overall(&self) -> f64 {
        self.overall
    }

    pub fn feature(&self, feature: &Feature) -> Skill {
        self.skill(self.features.get(feature))
    }

    pub fn kind(&self, kind: Kind) -> Skill {
        self.skill(self.kinds.get(&kind))
    }

    /// How hard `problem` is likely to be: the average over its features,
    /// so a new problem made of weak facts scores as weak.
    pub fn problem(&self, problem: &Problem) -> f64 {
        let features = features(problem);
        features.iter().map(|f| self.feature(f).score).sum::<f64>() / features.len() as f64
    }

    /// Every feature seen, with what is known about it.
    pub fn features(&self) -> impl Iterator<Item = (Feature, Skill)> + '_ {
        self.features
            .iter()
            .map(|(&f, tally)| (f, self.skill(Some(tally))))
    }

    fn skill(&self, tally: Option<&Tally>) -> Skill {
        let Tally { weight, sum, count } = tally.copied().unwrap_or_default();
        Skill {
            score: (sum + self.prior * self.overall) / (weight + self.prior).max(f64::EPSILON),
            count,
        }
    }
}

/// An attempt's weight: halved every `half_life` days of age.
fn recency(ts: Timestamp, now: Timestamp, half_life: f64) -> f64 {
    if half_life <= 0.0 {
        return 1.0;
    }
    let days = (now.duration_since(ts).as_secs_f64() / 86_400.0).max(0.0);
    0.5f64.powf(days / half_life)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Timestamp {
        "2026-11-13T12:00:00Z".parse().unwrap()
    }

    fn attempt(problem: Problem, ms: u64) -> Attempt {
        Attempt {
            ts: now(),
            session: "s".into(),
            problem,
            question: problem.to_string(),
            answer: problem.answer().to_string(),
            given: problem.answer().to_string(),
            correct: true,
            skipped: false,
            submit: Submit::Auto,
            ms,
            edits: 0,
        }
    }

    fn mul(lhs: i64, rhs: i64) -> Problem {
        Problem::Mul { lhs, rhs }
    }

    #[test]
    fn struggle_is_log_time_plus_penalties() {
        let s = Scoring::default();
        let a = attempt(mul(7, 8), 2000);
        assert!((struggle(&a, 1000.0, &s) - 2f64.ln()).abs() < 1e-9);
        assert!(struggle(&a, 2000.0, &s).abs() < 1e-9);
        let wrong = Attempt {
            correct: false,
            ms: 500,
            ..a.clone()
        };
        assert_eq!(struggle(&wrong, 1000.0, &s), s.wrong);
        let skip = Attempt {
            correct: false,
            skipped: true,
            ms: 500,
            ..a.clone()
        };
        assert_eq!(struggle(&skip, 1000.0, &s), s.skip);
        let edited = Attempt { edits: 2, ..a };
        assert!((struggle(&edited, 2000.0, &s) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn slow_facts_stand_out() {
        let mut attempts = Vec::new();
        for _ in 0..20 {
            attempts.push(attempt(mul(3, 4), 1000));
            attempts.push(attempt(mul(2, 6), 1000));
            attempts.push(attempt(mul(7, 8), 4000));
        }
        let model = SkillModel::build(&attempts, &Scoring::default(), now());
        let fact = |a, b| model.feature(&Feature::Fact(a, b)).score;
        assert!(fact(7, 8) > fact(3, 4) + 1.0);
        assert_eq!(model.feature(&Feature::Fact(7, 8)).count, 20);
        // A new problem using the weak fact is predicted to be hard.
        assert!(model.problem(&mul(78, 7)) > model.problem(&mul(43, 2)));
    }

    #[test]
    fn few_attempts_are_pulled_toward_the_average() {
        let mut attempts: Vec<_> = (0..50).map(|_| attempt(mul(3, 4), 1000)).collect();
        attempts.push(attempt(mul(6, 9), 8000));
        let model = SkillModel::build(&attempts, &Scoring::default(), now());
        let once = model.feature(&Feature::Fact(6, 9));
        let raw = (8000f64 / 1000.0).ln();
        assert!(
            once.score > model.overall() && once.score < raw / 2.0,
            "{once:?}"
        );
        assert!(model.feature(&Feature::Fact(5, 5)).score == model.overall());
    }

    #[test]
    fn old_attempts_fade() {
        let s = Scoring::default();
        let old = Attempt {
            ts: "2026-08-15T12:00:00Z".parse().unwrap(),
            ..attempt(mul(7, 8), 8000)
        };
        let mut attempts = vec![old];
        attempts.extend((0..10).map(|_| attempt(mul(7, 8), 1000)));
        let faded = SkillModel::build(&attempts, &s, now()).feature(&Feature::Fact(7, 8));
        let flat = Scoring {
            half_life: 0.0,
            ..s.clone()
        };
        let kept = SkillModel::build(&attempts, &flat, now()).feature(&Feature::Fact(7, 8));
        assert!(faded.score < kept.score);
    }

    #[test]
    fn baselines_separate_submit_modes() {
        let enter = |ms| Attempt {
            submit: Submit::Enter,
            ..attempt(mul(3, 4), ms)
        };
        let attempts = [attempt(mul(3, 4), 1000), enter(3000), enter(3000)];
        let split = Baselines::build(&attempts, true);
        assert_eq!(split.median(Kind::Mul, Submit::Auto), Some(1000.0));
        assert_eq!(split.median(Kind::Mul, Submit::Enter), Some(3000.0));
        let joint = Baselines::build(&attempts, false);
        assert_eq!(joint.median(Kind::Mul, Submit::Auto), Some(3000.0));
    }

    #[test]
    fn medians() {
        assert_eq!(median(&mut [3, 1, 2]), 2.0);
        assert_eq!(median(&mut [4, 1, 2, 3]), 2.5);
        assert_eq!(median(&mut []), 0.0);
    }
}
