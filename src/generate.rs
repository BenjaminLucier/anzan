//! Generators that produce problems at random.

mod arithmetic;

use std::fmt;

use rand::{Rng, RngExt, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

pub use arithmetic::{AddSub, MulDiv};

use crate::problem::{Op, Problem};

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
    /// The operations to ask, each with equal odds.
    pub ops: Vec<Op>,
    pub add_sub: AddSub,
    pub mul_div: MulDiv,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ops: Op::ALL.to_vec(),
            add_sub: AddSub::default(),
            mul_div: MulDiv::default(),
        }
    }
}

impl Settings {
    /// Checks what types alone cannot express.
    pub fn validate(&self) -> Result<(), String> {
        if self.ops.is_empty() {
            return Err("`ops` must list at least one operation".into());
        }
        if self.mul_div.lhs.contains(0) && self.ops.contains(&Op::Div) {
            return Err("`mul_div.lhs` must not contain 0, since it is the divisor".into());
        }
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the settings do not pass `validate`.
    pub fn generate(&self, rng: &mut impl Rng) -> Problem {
        let op = *self.ops.choose(rng).expect("at least one operation");
        match op {
            Op::Add => self.add_sub.add(rng),
            Op::Sub => self.add_sub.sub(rng),
            Op::Mul => self.mul_div.mul(rng),
            Op::Div => self.mul_div.div(rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    fn ops_seen(settings: &Settings) -> HashSet<Op> {
        let mut rng = StdRng::seed_from_u64(0);
        (0..200).map(|_| settings.generate(&mut rng).op).collect()
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
        let no_ops = Settings {
            ops: vec![],
            ..Settings::default()
        };
        assert!(no_ops.validate().is_err());
        let mut zero_divisor = Settings::default();
        zero_divisor.mul_div.lhs = Range::new(0, 12);
        assert!(zero_divisor.validate().is_err());
    }

    #[test]
    fn mixes_every_operation() {
        assert_eq!(ops_seen(&Settings::default()), HashSet::from(Op::ALL));
    }

    #[test]
    fn asks_only_chosen_operations() {
        let settings = Settings {
            ops: vec![Op::Add, Op::Sub],
            ..Settings::default()
        };
        assert_eq!(ops_seen(&settings), HashSet::from([Op::Add, Op::Sub]));
    }
}
