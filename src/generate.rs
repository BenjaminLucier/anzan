//! Generators that produce problems at random.

mod arithmetic;
mod finance;
mod fractions;
mod number_theory;
mod percent;
mod powers;
mod probability;

use std::fmt;

use rand::{Rng, RngExt, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

pub use self::{
    arithmetic::{AddSub, LongMul, MulDiv},
    finance::Finance,
    fractions::Fractions,
    number_theory::NumberTheory,
    percent::Percent,
    powers::Powers,
    probability::Probability,
};

use crate::problem::{Kind, Problem};

/// An inclusive range of integers that is never empty.
///
/// In config files it is written as a two-element array, `[2, 100]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "[i64; 2]", into = "[i64; 2]")]
pub struct Range {
    lo: i64,
    hi: i64,
}

impl Range {
    /// # Panics
    ///
    /// Panics if `lo > hi`. Use `try_from` for values that come from users.
    pub const fn new(lo: i64, hi: i64) -> Self {
        assert!(lo <= hi, "empty range");
        Self { lo, hi }
    }

    pub fn lo(self) -> i64 {
        self.lo
    }

    pub fn hi(self) -> i64 {
        self.hi
    }

    pub fn contains(self, n: i64) -> bool {
        (self.lo..=self.hi).contains(&n)
    }

    /// Whether every number in the range is at most `limit` from zero.
    pub fn within(self, limit: i64) -> bool {
        -limit <= self.lo && self.hi <= limit
    }

    pub fn sample(self, rng: &mut impl Rng) -> i64 {
        rng.random_range(self.lo..=self.hi)
    }
}

impl TryFrom<[i64; 2]> for Range {
    type Error = EmptyRange;

    fn try_from([lo, hi]: [i64; 2]) -> Result<Self, Self::Error> {
        if lo <= hi {
            Ok(Self { lo, hi })
        } else {
            Err(EmptyRange { lo, hi })
        }
    }
}

impl From<Range> for [i64; 2] {
    fn from(r: Range) -> Self {
        [r.lo, r.hi]
    }
}

#[derive(Debug)]
pub struct EmptyRange {
    lo: i64,
    hi: i64,
}

impl fmt::Display for EmptyRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "range [{}, {}] is empty: the first number must not exceed the second",
            self.lo, self.hi
        )
    }
}

/// Everything that decides which problems a session asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// The kinds of problem to ask, each with equal odds.
    pub kinds: Vec<Kind>,
    pub add_sub: AddSub,
    pub mul_div: MulDiv,
    pub long_mul: LongMul,
    pub powers: Powers,
    pub number_theory: NumberTheory,
    pub fractions: Fractions,
    pub percent: Percent,
    pub finance: Finance,
    pub probability: Probability,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            kinds: Kind::CORE.to_vec(),
            add_sub: AddSub::default(),
            mul_div: MulDiv::default(),
            long_mul: LongMul::default(),
            powers: Powers::default(),
            number_theory: NumberTheory::default(),
            fractions: Fractions::default(),
            percent: Percent::default(),
            finance: Finance::default(),
            probability: Probability::default(),
        }
    }
}

impl Settings {
    /// Checks what types alone cannot express.
    pub fn validate(&self) -> Result<(), String> {
        if self.kinds.is_empty() {
            return Err("`kinds` must list at least one kind of problem".into());
        }
        // Whether or not division is on now: the menu can turn it on.
        if self.mul_div.lhs.contains(0) {
            return Err("`mul_div.lhs` must not contain 0, since it is the divisor".into());
        }
        let operands = [
            ("add_sub", self.add_sub.lhs, self.add_sub.rhs),
            ("mul_div", self.mul_div.lhs, self.mul_div.rhs),
            ("long_mul", self.long_mul.lhs, self.long_mul.rhs),
        ];
        for (name, lhs, rhs) in operands {
            if ![lhs, rhs].iter().all(|r| r.within(Problem::MAX_OPERAND)) {
                return Err(format!(
                    "`{name}` operands must lie within ±{}",
                    Problem::MAX_OPERAND
                ));
            }
        }
        self.powers.validate()?;
        self.number_theory.validate()?;
        self.fractions.validate()?;
        self.percent.validate()?;
        self.finance.validate()?;
        self.probability.validate()
    }

    /// # Panics
    ///
    /// Panics if the settings do not pass `validate`.
    pub fn generate(&self, rng: &mut impl Rng) -> Problem {
        let kind = *self.kinds.choose(rng).expect("at least one kind");
        self.generate_kind(kind, rng)
    }

    /// A problem of the given kind, whether or not it is enabled.
    pub fn generate_kind(&self, kind: Kind, rng: &mut impl Rng) -> Problem {
        match kind {
            Kind::Add => self.add_sub.add(rng),
            Kind::Sub => self.add_sub.sub(rng),
            Kind::Mul => self.mul_div.mul(rng),
            Kind::Div => self.mul_div.div(rng),
            Kind::LongMul => self.long_mul.generate(rng),
            Kind::Square => self.powers.square(rng),
            Kind::Cube => self.powers.cube(rng),
            Kind::PowerOfTwo => self.powers.power_of_two(rng),
            Kind::Divisible => self.number_theory.divisible(rng),
            Kind::Prime => self.number_theory.is_prime(rng),
            Kind::Factorize => self.number_theory.factorize(rng),
            Kind::Convert => self.fractions.convert(rng),
            Kind::PercentOf => self.percent.of(rng),
            Kind::PercentChange => self.percent.change(rng),
            Kind::Successive => self.percent.successive(rng),
            Kind::Sqrt => self.powers.sqrt(rng),
            Kind::BasisPoints => self.finance.basis_points(rng),
            Kind::Compound => self.finance.compound(rng),
            Kind::Doubling => self.finance.doubling(rng),
            Kind::Dice => self.probability.dice(rng),
            Kind::Coins => self.probability.coins(rng),
            Kind::ExpectedValue => self.probability.expected_value(rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    fn kinds_seen(settings: &Settings) -> HashSet<Kind> {
        let mut rng = StdRng::seed_from_u64(0);
        (0..1_000)
            .map(|_| settings.generate(&mut rng).kind())
            .collect()
    }

    #[test]
    fn range_includes_both_ends() {
        let r = Range::new(1, 3);
        let mut rng = StdRng::seed_from_u64(0);
        let seen: HashSet<_> = (0..1_000).map(|_| r.sample(&mut rng)).collect();
        assert_eq!(seen, HashSet::from([1, 2, 3]));
    }

    #[test]
    fn range_rejects_empty() {
        assert!(Range::try_from([3, 2]).is_err());
        assert!(Range::try_from([2, 2]).is_ok());
    }

    #[test]
    fn validate_catches_impossible_settings() {
        assert!(Settings::default().validate().is_ok());
        let no_kinds = Settings {
            kinds: vec![],
            ..Settings::default()
        };
        assert!(no_kinds.validate().is_err());
        // Rejected even with division off, since the menu can turn it on.
        let mut zero_divisor = Settings {
            kinds: vec![Kind::Add],
            ..Settings::default()
        };
        zero_divisor.mul_div.lhs = Range::new(0, 12);
        assert!(zero_divisor.validate().is_err());
        let mut overflowing = Settings::default();
        overflowing.add_sub.rhs = Range::new(0, i64::MAX);
        assert!(overflowing.validate().is_err());
    }

    /// One setting just past each limit `validate` enforces. The error
    /// names the setting, so the reader of a config knows where to look.
    #[test]
    fn validate_names_the_setting_at_fault() {
        // A function, not a closure, so the cases below capture nothing.
        const fn r(lo: i64, hi: i64) -> Range {
            Range::new(lo, hi)
        }
        type Break = fn(&mut Settings);
        let cases: [(&str, Break); 22] = [
            ("add_sub", |s| s.add_sub.lhs = r(0, 1_000_000_001)),
            ("long_mul", |s| s.long_mul.rhs = r(-1_000_000_001, 0)),
            ("powers", |s| s.powers.cubes = r(2, 2_000_001)),
            ("powers.twos", |s| s.powers.twos = r(-1, 4)),
            ("powers.roots", |s| s.powers.roots = r(1, 50)),
            ("number_theory.divisors", |s| {
                s.number_theory.divisors = r(0, 9)
            }),
            ("number_theory.primes", |s| {
                s.number_theory.primes = r(-5, 9)
            }),
            ("number_theory.factorize", |s| {
                s.number_theory.factorize = r(3, 9)
            }),
            ("number_theory", |s| {
                s.number_theory.dividends = r(1, 1_000_000_001)
            }),
            ("fractions.denominators", |s| {
                s.fractions.denominators = r(1, 20)
            }),
            ("fractions.denominators", |s| {
                s.fractions.denominators = r(6, 7)
            }),
            ("percent.amounts", |s| s.percent.amounts = r(0, 200)),
            ("percent.changes", |s| s.percent.changes = r(-100, 50)),
            ("percent.changes", |s| s.percent.changes = r(-4, 4)),
            ("percent.rates", |s| s.percent.rates = r(-1, 99)),
            ("finance.bps", |s| s.finance.bps = r(0, 500)),
            ("finance.rates", |s| s.finance.rates = r(1, 101)),
            ("finance.years", |s| s.finance.years = r(0, 5)),
            ("finance.doubling", |s| s.finance.doubling = r(0, 24)),
            ("probability.flips", |s| s.probability.flips = r(2, 11)),
            ("probability.odds", |s| s.probability.odds = r(1, 6)),
            ("probability.stakes", |s| s.probability.stakes = r(2, 5)),
        ];
        for (name, break_it) in cases {
            let mut settings = Settings::default();
            break_it(&mut settings);
            let err = settings.validate().expect_err(name);
            assert!(err.contains(name), "{name}: {err}");
        }
    }

    #[test]
    fn defaults_to_the_core_kinds() {
        let core: HashSet<_> = Kind::CORE.iter().copied().collect();
        assert_eq!(kinds_seen(&Settings::default()), core);
    }

    #[test]
    fn mixes_every_kind() {
        let settings = Settings {
            kinds: Kind::ALL.to_vec(),
            ..Settings::default()
        };
        let all: HashSet<_> = Kind::ALL.iter().copied().collect();
        assert_eq!(kinds_seen(&settings), all);
    }

    /// Every kind's generator, with default settings, produces problems
    /// whose own answer accepts the answer's display.
    #[test]
    fn every_kind_answers_itself() {
        let settings = Settings::default();
        let mut rng = StdRng::seed_from_u64(0);
        for &kind in Kind::ALL {
            for _ in 0..200 {
                let p = settings.generate_kind(kind, &mut rng);
                assert_eq!(p.kind(), kind);
                assert!(p.is_well_formed(), "{p:?}");
                let json = serde_json::to_value(p).unwrap();
                assert_eq!(json["kind"], kind.name(), "stored tag of {p}");
                let answer = p.answer();
                assert!(answer.check(&answer.to_string()), "{p} = {answer}");
            }
        }
    }

    /// The widest settings that pass validation still ask problems with
    /// answers, so nothing a config can say makes a session panic.
    #[test]
    fn widest_settings_stay_well_formed() {
        let wide = |lo| Range::new(lo, Problem::MAX_OPERAND);
        let all = Range::new(-Problem::MAX_OPERAND, Problem::MAX_OPERAND);
        let settings = Settings {
            kinds: Kind::ALL.to_vec(),
            add_sub: AddSub { lhs: all, rhs: all },
            mul_div: MulDiv {
                lhs: wide(1),
                rhs: all,
            },
            long_mul: LongMul { lhs: all, rhs: all },
            powers: Powers {
                squares: all,
                cubes: Range::new(-2_000_000, 2_000_000),
                twos: Range::new(0, 62),
                roots: wide(2),
            },
            number_theory: NumberTheory {
                divisors: wide(1),
                dividends: wide(0),
                primes: wide(0),
                factorize: wide(4),
            },
            fractions: Fractions {
                denominators: Range::new(2, 1_000),
            },
            percent: Percent {
                rates: Range::new(0, 1_000),
                amounts: Range::new(1, 1_000_000),
                changes: Range::new(-99, 1_000),
            },
            finance: Finance {
                bps: Range::new(1, 10_000),
                rates: Range::new(1, 100),
                years: Range::new(1, 30),
                doubling: Range::new(1, 100),
            },
            probability: Probability {
                flips: Range::new(1, 10),
                odds: Range::new(2, 12),
                stakes: Range::new(1, 1_000_000),
            },
        };
        settings.validate().unwrap();
        let mut rng = StdRng::seed_from_u64(0);
        for &kind in Kind::ALL {
            for _ in 0..50 {
                let p = settings.generate_kind(kind, &mut rng);
                assert!(p.is_well_formed(), "{p:?}");
                let answer = p.answer();
                assert!(answer.check(&answer.to_string()), "{p} = {answer}");
            }
        }
    }

    #[test]
    fn asks_only_chosen_kinds() {
        let settings = Settings {
            kinds: vec![Kind::Add, Kind::Sub],
            ..Settings::default()
        };
        assert_eq!(kinds_seen(&settings), HashSet::from([Kind::Add, Kind::Sub]));
    }
}
