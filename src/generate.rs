//! Generators that produce problems at random.

mod arithmetic;
mod number_theory;
mod powers;

use std::fmt;

use rand::{Rng, RngExt, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

pub use self::{
    arithmetic::{AddSub, LongMul, MulDiv},
    number_theory::NumberTheory,
    powers::Powers,
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
        }
    }
}

impl Settings {
    /// Checks what types alone cannot express.
    pub fn validate(&self) -> Result<(), String> {
        if self.kinds.is_empty() {
            return Err("`kinds` must list at least one kind of problem".into());
        }
        if self.mul_div.lhs.contains(0) && self.kinds.contains(&Kind::Div) {
            return Err("`mul_div.lhs` must not contain 0, since it is the divisor".into());
        }
        self.powers.validate()?;
        self.number_theory.validate()
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
        let mut zero_divisor = Settings::default();
        zero_divisor.mul_div.lhs = Range::new(0, 12);
        assert!(zero_divisor.validate().is_err());
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
