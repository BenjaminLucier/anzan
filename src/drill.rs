//! Practice modes: how each session chooses its next problem.
//!
//! Every mode comes down to a [`Plan`]: where problems come from, and how
//! hard to lean toward weak spots. Weak spots come from the
//! [`SkillModel`], built from history when a session starts. With no
//! history to go on, every mode asks fresh problems at random.

use std::{collections::HashSet, fmt, str::FromStr};

use jiff::{SignedDuration, Timestamp};
use rand::{Rng, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

use crate::{
    generate::Settings,
    history::History,
    problem::Problem,
    stats::{
        Scoring, SkillModel,
        features::{Feature, canonical, features},
    },
};

/// Where problems come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// New problems from the configured generators.
    #[default]
    Fresh,
    /// New problems that exercise a chosen skill, such as a times-table
    /// fact or adding with two carries.
    Similar,
    /// Questions asked before, asked again.
    Replay,
}

/// What a mode does: where problems come from, and the focus. A focus of
/// 0 ignores history; positive leans toward weak spots and negative toward
/// strong ones, each step multiplying the odds by e per unit of struggle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    pub source: Source,
    pub focus: f64,
}

/// Settings for the custom mode, and the history window every mode uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DrillConfig {
    pub source: Source,
    pub focus: f64,
    /// Days of history to draw questions and skills from. 0 for all.
    pub window: u32,
}

impl Default for DrillConfig {
    fn default() -> Self {
        Self {
            source: Source::Similar,
            focus: 1.5,
            window: 60,
        }
    }
}

impl DrillConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.focus.is_finite() || self.focus.abs() > 10.0 {
            return Err("`drill.focus` must be between -10 and 10".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Jiyu,
    Kaizen,
    Keiko,
    Fukushu,
    Oni,
    Rakusho,
    Custom,
}

impl Mode {
    pub const ALL: [Mode; 7] = [
        Mode::Jiyu,
        Mode::Kaizen,
        Mode::Keiko,
        Mode::Fukushu,
        Mode::Oni,
        Mode::Rakusho,
        Mode::Custom,
    ];

    /// The name used in config files, history and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Jiyu => "jiyu",
            Mode::Kaizen => "kaizen",
            Mode::Keiko => "keiko",
            Mode::Fukushu => "fukushu",
            Mode::Oni => "oni",
            Mode::Rakusho => "rakusho",
            Mode::Custom => "custom",
        }
    }

    /// The name as shown, with long vowels marked.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Jiyu => "Jiyū",
            Mode::Kaizen => "Kaizen",
            Mode::Keiko => "Keiko",
            Mode::Fukushu => "Fukushū",
            Mode::Oni => "Oni",
            Mode::Rakusho => "Rakushō",
            Mode::Custom => "Custom",
        }
    }

    pub fn kanji(self) -> &'static str {
        match self {
            Mode::Jiyu => "自由",
            Mode::Kaizen => "改善",
            Mode::Keiko => "稽古",
            Mode::Fukushu => "復習",
            Mode::Oni => "鬼",
            Mode::Rakusho => "楽勝",
            Mode::Custom => "",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Mode::Jiyu => "freedom: problems at random",
            Mode::Kaizen => "improvement: leans gently toward weak spots",
            Mode::Keiko => "practice: new problems built on weak skills",
            Mode::Fukushu => "review: the questions that gave you trouble",
            Mode::Oni => "demon: relentlessly on your weakest skills",
            Mode::Rakusho => "easy win: questions you know, to warm up",
            Mode::Custom => "as set in the [drill] section of the config",
        }
    }

    pub fn plan(self, custom: &DrillConfig) -> Plan {
        let plan = |source, focus| Plan { source, focus };
        match self {
            Mode::Jiyu => plan(Source::Fresh, 0.0),
            Mode::Kaizen => plan(Source::Fresh, 1.0),
            Mode::Keiko => plan(Source::Similar, 2.0),
            Mode::Fukushu => plan(Source::Replay, 2.0),
            Mode::Oni => plan(Source::Similar, 4.0),
            Mode::Rakusho => plan(Source::Replay, -2.0),
            Mode::Custom => plan(custom.source, custom.focus),
        }
    }

    pub fn step(self, forwards: bool) -> Mode {
        let n = Mode::ALL.len();
        let i = Mode::ALL.iter().position(|&m| m == self).unwrap_or(0);
        Mode::ALL[if forwards {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }]
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Mode::ALL
            .into_iter()
            .find(|m| m.name() == s)
            .ok_or_else(|| {
                let names: Vec<_> = Mode::ALL.iter().map(|m| m.name()).collect();
                format!("unknown mode `{s}`: expected one of {}", names.join(", "))
            })
    }
}

/// How many fresh problems to weigh against each other when leaning
/// toward weak spots.
const CANDIDATES: usize = 8;

/// How many fresh problems to try when looking for one that exercises a
/// skill before settling for any.
const TRIES: usize = 300;

/// Chooses each problem in a session.
pub struct Drill {
    plan: Plan,
    model: SkillModel,
    /// Skills seen recently in the kinds being asked, with their scores.
    targets: Vec<(Feature, f64)>,
    /// Questions seen recently in the kinds being asked, with their scores.
    items: Vec<(Problem, f64)>,
}

impl Drill {
    pub fn new(
        plan: Plan,
        settings: &Settings,
        history: &History,
        scoring: &Scoring,
        window: u32,
        now: Timestamp,
    ) -> Self {
        let mut drill = Self {
            plan,
            model: SkillModel::default(),
            targets: Vec::new(),
            items: Vec::new(),
        };
        if plan.source == Source::Fresh && plan.focus == 0.0 {
            return drill;
        }
        drill.model = SkillModel::build(&history.attempts, scoring, now);
        let since = match window {
            0 => Timestamp::MIN,
            days => now - SignedDuration::from_hours(24 * i64::from(days)),
        };
        let mut seen = HashSet::new();
        for a in history.attempts.iter().rev() {
            if a.ts < since {
                break;
            }
            if !settings.kinds.contains(&a.problem.kind()) {
                continue;
            }
            for feature in features(&a.problem) {
                if seen.insert(feature) {
                    let score = drill.model.feature(&feature).score;
                    match feature {
                        Feature::Item(p) => drill.items.push((p, score)),
                        f => drill.targets.push((f, score)),
                    }
                }
            }
        }
        // History is newest first here; keep a stable order regardless.
        drill.targets.sort_by_key(|a| a.0);
        drill.items.sort_by_key(|a| a.0);
        drill
    }

    pub fn plan(&self) -> Plan {
        self.plan
    }

    /// The next problem, never the same question as `previous`.
    pub fn next(
        &self,
        settings: &Settings,
        rng: &mut impl Rng,
        previous: Option<&Problem>,
    ) -> Problem {
        let previous = previous.map(canonical);
        let fresh = |rng: &mut _| self.fresh(settings, rng, previous.as_ref());
        match self.plan.source {
            Source::Fresh => fresh(rng),
            Source::Similar => {
                let Some(target) = self.pick(&self.targets, rng) else {
                    return fresh(rng);
                };
                (0..TRIES)
                    .map(|_| settings.generate(rng))
                    .find(|p| features(p).contains(&target) && Some(canonical(p)) != previous)
                    .unwrap_or_else(|| fresh(rng))
            }
            Source::Replay => {
                let pool: Vec<_> = self
                    .items
                    .iter()
                    .filter(|(p, _)| Some(*p) != previous)
                    .copied()
                    .collect();
                self.pick(&pool, rng).unwrap_or_else(|| fresh(rng))
            }
        }
    }

    /// A new problem: at random, or the weakest-looking of a few.
    fn fresh(
        &self,
        settings: &Settings,
        rng: &mut impl Rng,
        previous: Option<&Problem>,
    ) -> Problem {
        let new = |rng: &mut _| {
            let mut p = settings.generate(rng);
            // Ask again once rather than repeat; small ranges may have to.
            if Some(&canonical(&p)) == previous {
                p = settings.generate(rng);
            }
            p
        };
        if self.plan.focus == 0.0 {
            return new(rng);
        }
        let candidates: Vec<_> = (0..CANDIDATES)
            .map(|_| {
                let p = new(rng);
                (p, self.model.problem(&p))
            })
            .collect();
        self.pick(&candidates, rng).unwrap_or_else(|| new(rng))
    }

    /// One of `options`, with odds growing as e^(focus × score).
    fn pick<T: Copy>(&self, options: &[(T, f64)], rng: &mut impl Rng) -> Option<T> {
        let focus = self.plan.focus;
        // Measured from the most favoured option, so nothing overflows.
        let top = options
            .iter()
            .map(|(_, s)| focus * s)
            .fold(f64::NEG_INFINITY, f64::max);
        options
            .choose_weighted(rng, |(_, s)| (focus * s - top).exp())
            .ok()
            .map(|(t, _)| *t)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::{config::Submit, problem::Kind, store::Attempt};

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
            given: String::new(),
            correct: true,
            skipped: false,
            submit: Submit::Auto,
            ms,
            edits: 0,
        }
    }

    /// Multiplication history where 7 × 8 is slow and everything else easy.
    fn history() -> History {
        let mut attempts = Vec::new();
        for a in 2..=9 {
            for b in 2..=9 {
                let ms = if (a, b) == (7, 8) || (a, b) == (8, 7) {
                    6000
                } else {
                    1000
                };
                attempts.push(attempt(Problem::Mul { lhs: a, rhs: b }, ms));
                attempts.push(attempt(Problem::Mul { lhs: a, rhs: b }, ms));
            }
        }
        History {
            attempts,
            ..History::default()
        }
    }

    fn settings() -> Settings {
        Settings {
            kinds: vec![Kind::Mul],
            ..Settings::default()
        }
    }

    fn drill(mode: Mode, history: &History) -> Drill {
        let plan = mode.plan(&DrillConfig::default());
        Drill::new(plan, &settings(), history, &Scoring::default(), 30, now())
    }

    fn uses_fact(p: &Problem, a: u8, b: u8) -> bool {
        features(p).contains(&Feature::Fact(a, b))
    }

    /// How often `drill` asks something using 7 × 8, out of 400.
    fn rate(drill: &Drill) -> usize {
        let mut rng = StdRng::seed_from_u64(1);
        let mut previous = None;
        (0..400)
            .filter(|_| {
                let p = drill.next(&settings(), &mut rng, previous.as_ref());
                previous = Some(p);
                uses_fact(&p, 7, 8)
            })
            .count()
    }

    #[test]
    fn modes_round_trip_by_name() {
        for mode in Mode::ALL {
            assert_eq!(mode.name().parse::<Mode>(), Ok(mode));
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{}\"", mode.name()));
        }
        assert!("zen".parse::<Mode>().is_err());
        assert_eq!(Mode::Jiyu.step(false), Mode::Custom);
    }

    #[test]
    fn focus_finds_the_weak_fact() {
        let history = history();
        let random = rate(&drill(Mode::Jiyu, &history));
        let kaizen = rate(&drill(Mode::Kaizen, &history));
        let oni = rate(&drill(Mode::Oni, &history));
        assert!(kaizen > random, "{kaizen} vs {random}");
        assert!(oni > 3 * random && oni > kaizen, "{oni} vs {random}");
    }

    #[test]
    fn review_replays_hard_questions_and_easy_win_avoids_them() {
        let history = history();
        let hard = rate(&drill(Mode::Fukushu, &history));
        let easy = rate(&drill(Mode::Rakusho, &history));
        // By chance it would be 1 in 36, about 11.
        assert!(hard > 30, "{hard}");
        assert!(easy < 10, "{easy}");
        // Replayed questions all come from history.
        let d = drill(Mode::Fukushu, &history);
        let mut rng = StdRng::seed_from_u64(2);
        let p = d.next(&settings(), &mut rng, None);
        let Problem::Mul { lhs, rhs } = p else {
            panic!("{p:?}")
        };
        assert!((2..=9).contains(&lhs) && (2..=9).contains(&rhs));
    }

    #[test]
    fn never_repeats_the_previous_question() {
        let history = history();
        let d = drill(Mode::Fukushu, &history);
        let mut rng = StdRng::seed_from_u64(3);
        let mut previous = d.next(&settings(), &mut rng, None);
        for _ in 0..200 {
            let p = d.next(&settings(), &mut rng, Some(&previous));
            assert_ne!(canonical(&p), canonical(&previous));
            previous = p;
        }
    }

    #[test]
    fn without_history_every_mode_asks_fresh_problems() {
        let empty = History::default();
        let mut rng = StdRng::seed_from_u64(4);
        for mode in Mode::ALL {
            let d = drill(mode, &empty);
            let mut kinds = HashMap::new();
            for _ in 0..20 {
                *kinds
                    .entry(d.next(&settings(), &mut rng, None).kind())
                    .or_insert(0) += 1;
            }
            assert_eq!(kinds.get(&Kind::Mul), Some(&20), "{mode:?}");
        }
    }

    #[test]
    fn window_leaves_out_old_history() {
        let mut history = history();
        for a in &mut history.attempts {
            a.ts = "2026-01-01T00:00:00Z".parse().unwrap();
        }
        let d = drill(Mode::Fukushu, &history);
        assert!(d.items.is_empty() && d.targets.is_empty());
    }
}
