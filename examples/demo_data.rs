//! Writes eight months of made-up practice history for one keen user, for
//! the README's screenshots and demo (see `demo/README.md`).
//!
//! ```sh
//! cargo run --example demo_data -- target/demo/data
//! ```
//!
//! The history is the same on every run, apart from ending yesterday. It
//! has a habit with a few gaps and a two-week streak at the end, answers
//! that speed up over the months, and some deliberate weak spots: the
//! 7 × 8 family of facts, subtractions that borrow, and additions that
//! carry. Review and improvement sessions return to the slow questions,
//! so they repeat often enough to be listed.

use std::{env, path::PathBuf};

use anyhow::{Context, Result, bail};
use anzan::{
    config::Submit,
    drill::Mode,
    generate::Settings,
    problem::{Kind, Problem},
    store::{Attempt, SessionRecord, Store},
};
use jiff::{SignedDuration, Timestamp, ToSpan, ZonedDateTime, civil::Weekday, tz::TimeZone};
use rand::{RngExt, SeedableRng, rngs::StdRng, seq::IndexedRandom};

/// "anzan" in ASCII.
const SEED: u64 = 0x0061_6e7a_616e;
const DAYS: i64 = 240;
const SESSION_SECS: u64 = 120;
/// Between an answer and the next question appearing.
const GAP_MS: u64 = 250;

/// Single-digit facts this user is slow at, smaller digit first.
const WEAK_FACTS: &[(i64, i64)] = &[(6, 7), (6, 8), (7, 7), (7, 8), (7, 9), (8, 9)];

fn main() -> Result<()> {
    let dir: PathBuf = env::args_os()
        .nth(1)
        .context("usage: cargo run --example demo_data -- <data dir>")?
        .into();
    let store = Store::new(&dir);
    if store.attempts_path().exists() || store.sessions_path().exists() {
        bail!("{} already has history; delete it first", dir.display());
    }

    let mut rng = StdRng::seed_from_u64(SEED);
    let tz = TimeZone::system();
    let today = ZonedDateTime::now().date();
    let setups = Setups::new();
    let mut slow: Vec<(Problem, f64)> = Vec::new();
    let (mut sessions, mut attempts) = (0, 0);

    for days_ago in (1..=DAYS).rev() {
        let date = today.checked_sub(days_ago.days())?;
        // 0 at the start, 1 by the end: how far the user has come along.
        let progress = (DAYS - days_ago) as f64 / DAYS as f64;
        let weekend = matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday);
        let holiday = (52..=59).contains(&days_ago) || (150..=161).contains(&days_ago);
        let streak = days_ago <= 14;
        let practises = streak || (!holiday && rng.random_bool(if weekend { 0.9 } else { 0.72 }));
        if !practises {
            continue;
        }

        let count = *[1, 1, 2, 2, 3].choose(&mut rng).unwrap() + usize::from(weekend);
        let (hour, minute) = if rng.random_bool(0.6) {
            (rng.random_range(19..=21), rng.random_range(0..60))
        } else {
            (7, rng.random_range(0..60))
        };
        let mut start = date
            .at(hour, minute, rng.random_range(0..60), 0)
            .to_zoned_datetime(tz.clone())?
            .timestamp();

        for _ in 0..count {
            let setup = setups.pick(&mut rng);
            let (record, rows) = session(&mut rng, setup, start, progress, &mut slow);
            for row in &rows {
                store.append_attempt(row)?;
            }
            store.append_session(&record)?;
            sessions += 1;
            attempts += rows.len();
            start = record.end + SignedDuration::from_secs(rng.random_range(30..300));
        }
    }

    println!(
        "wrote {sessions} sessions and {attempts} attempts to {}",
        dir.display()
    );
    Ok(())
}

/// The kinds of session this user runs, and how often.
struct Setups {
    all: Vec<(Setup, f64)>,
}

struct Setup {
    mode: Mode,
    settings: Settings,
}

impl Setups {
    fn new() -> Self {
        let with = |kinds: &[Kind]| Settings {
            kinds: kinds.to_vec(),
            ..Settings::default()
        };
        let all = vec![
            // The everyday warm-up: the defaults, so its trend is long.
            (setup(Mode::Jiyu, Settings::default()), 0.62),
            (setup(Mode::Jiyu, with(&[Kind::Mul, Kind::Div])), 0.1),
            (setup(Mode::Kaizen, Settings::default()), 0.1),
            (setup(Mode::Fukushu, Settings::default()), 0.06),
            (
                setup(
                    Mode::Jiyu,
                    with(&[
                        Kind::Square,
                        Kind::Sqrt,
                        Kind::PowerOfTwo,
                        Kind::Prime,
                        Kind::Divisible,
                        Kind::Convert,
                        Kind::PercentOf,
                        Kind::PercentChange,
                        Kind::BasisPoints,
                        Kind::Doubling,
                        Kind::Dice,
                        Kind::LongMul,
                    ]),
                ),
                0.12,
            ),
        ];
        Self { all }
    }

    fn pick(&self, rng: &mut StdRng) -> &Setup {
        let mut roll = rng.random::<f64>() * self.all.iter().map(|(_, w)| w).sum::<f64>();
        for (setup, weight) in &self.all {
            if roll < *weight {
                return setup;
            }
            roll -= weight;
        }
        &self.all[0].0
    }
}

fn setup(mode: Mode, settings: Settings) -> Setup {
    Setup { mode, settings }
}

/// One two-minute session starting at `start`, and its attempts.
fn session(
    rng: &mut StdRng,
    setup: &Setup,
    start: Timestamp,
    progress: f64,
    slow: &mut Vec<(Problem, f64)>,
) -> (SessionRecord, Vec<Attempt>) {
    let id = format!("{:016x}", rng.random::<u64>());
    // Better on some days than others.
    let form = lognormal(rng, 0.08);
    let speed = (1.6 - 0.65 * progress) * form;
    // Now and then a session is left early.
    let abandon_at = rng
        .random_bool(0.03)
        .then(|| rng.random_range(15_000..90_000));
    let limit = abandon_at.unwrap_or(SESSION_SECS * 1000);

    let mut rows = Vec::new();
    let mut elapsed = 0;
    let mut score = 0;
    loop {
        let problem = next_problem(rng, setup, slow);
        let effort = effort(&problem);
        let mut ms = (base_ms(problem.kind()) * effort * speed * lognormal(rng, 0.3)) as u64;
        ms = ms.max(550);
        let edits = if rng.random_bool(0.05 + 0.05 * (effort - 1.0).max(0.0)) {
            rng.random_range(1..=2)
        } else {
            0
        };
        ms += 350 * u64::from(edits);
        if elapsed + ms > limit {
            break;
        }
        elapsed += ms + GAP_MS;

        let answer = problem.answer().to_string();
        let struggling = effort > 1.6;
        let skipped = rng.random_bool(if struggling { 0.07 } else { 0.008 });
        let wrong = !skipped && rng.random_bool(if struggling { 0.05 } else { 0.012 });
        let given = if skipped {
            String::new()
        } else if wrong {
            mistake(&answer).unwrap_or_else(|| answer.clone())
        } else {
            answer.clone()
        };
        let correct = given == answer;
        score += u32::from(correct);
        if effort > 1.3 {
            slow.push((problem, effort));
        }
        rows.push(Attempt {
            ts: start + SignedDuration::from_millis(elapsed as i64),
            session: id.clone(),
            problem,
            question: problem.to_string(),
            answer,
            given,
            correct,
            skipped,
            submit: Submit::Auto,
            ms,
            edits,
        });
    }

    let end = match abandon_at {
        Some(_) => rows.last().map_or(start, |a| a.ts),
        None => start + SignedDuration::from_secs(SESSION_SECS as i64),
    };
    let record = SessionRecord {
        id,
        start,
        end,
        duration: SESSION_SECS,
        score,
        completed: abandon_at.is_none(),
        submit: Submit::Auto,
        mode: setup.mode,
        problems: serde_json::to_value(&setup.settings).expect("settings serialise"),
    };
    (record, rows)
}

/// What a session of this mode would ask next, roughly: review sessions
/// return to slow questions, and improvement sessions lean towards them.
fn next_problem(rng: &mut StdRng, setup: &Setup, slow: &[(Problem, f64)]) -> Problem {
    let recent = &slow[slow.len().saturating_sub(300)..];
    match setup.mode {
        Mode::Fukushu if !recent.is_empty() && rng.random_bool(0.8) => {
            recent.choose(rng).expect("not empty").0
        }
        Mode::Kaizen => (0..3)
            .map(|_| setup.settings.generate(rng))
            .max_by(|a, b| effort(a).total_cmp(&effort(b)))
            .expect("three candidates"),
        _ => setup.settings.generate(rng),
    }
}

/// Typical milliseconds for a kind, before effort and learning.
fn base_ms(kind: Kind) -> f64 {
    match kind {
        Kind::Add => 1500.0,
        Kind::Sub => 1700.0,
        Kind::Mul => 1600.0,
        Kind::Div => 1900.0,
        Kind::LongMul => 9000.0,
        Kind::Square => 3200.0,
        Kind::Cube => 5500.0,
        Kind::PowerOfTwo => 2200.0,
        Kind::Sqrt => 7500.0,
        Kind::Divisible => 2800.0,
        Kind::Prime => 3300.0,
        Kind::Factorize => 8000.0,
        Kind::Convert => 3400.0,
        Kind::PercentOf => 4200.0,
        Kind::PercentChange => 5600.0,
        Kind::Successive => 6500.0,
        Kind::BasisPoints => 4200.0,
        Kind::Compound => 8500.0,
        Kind::Doubling => 3600.0,
        Kind::Dice => 3800.0,
        Kind::Coins => 5800.0,
        Kind::ExpectedValue => 6800.0,
    }
}

/// How much harder than usual this problem is for this user: 1 is
/// typical, 2 twice as slow.
fn effort(problem: &Problem) -> f64 {
    match *problem {
        Problem::Add { lhs, rhs } => {
            let carries = carries(lhs, rhs);
            0.8 + 0.35 * carries as f64
        }
        Problem::Sub { lhs, rhs } => {
            let borrows = borrows(lhs, rhs);
            0.8 + 0.55 * borrows as f64
        }
        Problem::Mul { lhs, rhs } => times(lhs, rhs),
        Problem::Div { lhs, rhs } if rhs != 0 => times(rhs, lhs / rhs) * 1.1,
        _ => 1.0,
    }
}

/// Effort for `table` × `n`: each single-digit fact it needs, with the
/// weak ones much slower.
fn times(table: i64, n: i64) -> f64 {
    let digits: Vec<i64> = n
        .abs()
        .to_string()
        .bytes()
        .map(|b| i64::from(b - b'0'))
        .filter(|d| *d > 1)
        .collect();
    let table_cost = match table {
        2 | 5 | 10 | 11 => 0.6,
        12 => 1.25,
        _ => 0.9,
    };
    let facts: f64 = digits
        .iter()
        .map(|&d| {
            let pair = (table.min(d), table.max(d));
            if WEAK_FACTS.contains(&pair) {
                1.4
            } else {
                0.35
            }
        })
        .sum();
    table_cost + facts
}

fn carries(a: i64, b: i64) -> u32 {
    let (mut a, mut b, mut carry, mut count) = (a.abs(), b.abs(), 0, 0);
    while a > 0 || b > 0 {
        carry = i64::from(a % 10 + b % 10 + carry >= 10);
        count += carry as u32;
        a /= 10;
        b /= 10;
    }
    count
}

fn borrows(a: i64, b: i64) -> u32 {
    let (mut a, mut b, mut borrow, mut count) = (a, b, 0, 0);
    while a > 0 || b > 0 {
        borrow = i64::from(a % 10 - borrow < b % 10);
        count += borrow as u32;
        a /= 10;
        b /= 10;
    }
    count
}

/// A plausible wrong answer: the last digit off, or the other of yes and
/// no.
fn mistake(answer: &str) -> Option<String> {
    match answer {
        "y" => return Some("n".into()),
        "n" => return Some("y".into()),
        _ => {}
    }
    let last = answer.chars().last()?.to_digit(10)?;
    let mut wrong = answer[..answer.len() - 1].to_owned();
    wrong.push(char::from_digit((last + 2) % 10, 10)?);
    Some(wrong)
}

/// A random factor around 1, spread by `sigma`.
fn lognormal(rng: &mut StdRng, sigma: f64) -> f64 {
    // Box–Muller.
    let (u, v): (f64, f64) = (rng.random::<f64>().max(f64::MIN_POSITIVE), rng.random());
    let normal = (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos();
    (sigma * normal).exp()
}
