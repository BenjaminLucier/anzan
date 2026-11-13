//! Addition and subtraction.

use std::ops::RangeInclusive;

use rand::{Rng, RngExt};

use crate::problem::{Op, Problem};

/// Settings for addition and subtraction.
///
/// An addition draws each operand from its range. A subtraction is an
/// addition run backwards (`a + b = c` asks `c − b`), so its answer always
/// lies in `lhs` and is never negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddSub {
    pub lhs: RangeInclusive<i64>,
    pub rhs: RangeInclusive<i64>,
}

impl Default for AddSub {
    /// Both operands from 2 to 100.
    fn default() -> Self {
        Self {
            lhs: 2..=100,
            rhs: 2..=100,
        }
    }
}

impl AddSub {
    /// # Panics
    ///
    /// Panics if either range is empty.
    pub fn add(&self, rng: &mut impl Rng) -> Problem {
        Problem {
            op: Op::Add,
            lhs: rng.random_range(self.lhs.clone()),
            rhs: rng.random_range(self.rhs.clone()),
        }
    }

    /// # Panics
    ///
    /// Panics if either range is empty.
    pub fn sub(&self, rng: &mut impl Rng) -> Problem {
        let sum = self.add(rng);
        Problem {
            op: Op::Sub,
            lhs: sum.answer(),
            rhs: sum.rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    /// Non-empty ranges, including ranges that hold a single value.
    fn range() -> impl Strategy<Value = RangeInclusive<i64>> {
        (-1_000i64..1_000, 0i64..1_000).prop_map(|(lo, len)| lo..=lo + len)
    }

    fn settings() -> impl Strategy<Value = AddSub> {
        (range(), range()).prop_map(|(lhs, rhs)| AddSub { lhs, rhs })
    }

    proptest! {
        #[test]
        fn add_operands_stay_in_range(s in settings(), seed: u64) {
            let p = s.add(&mut StdRng::seed_from_u64(seed));
            prop_assert_eq!(p.op, Op::Add);
            prop_assert!(s.lhs.contains(&p.lhs));
            prop_assert!(s.rhs.contains(&p.rhs));
        }

        #[test]
        fn sub_reverses_an_add(s in settings(), seed: u64) {
            let p = s.sub(&mut StdRng::seed_from_u64(seed));
            prop_assert_eq!(p.op, Op::Sub);
            prop_assert!(s.lhs.contains(&p.answer()));
            prop_assert!(s.rhs.contains(&p.rhs));
        }
    }

    #[test]
    fn default_sub_is_never_negative() {
        let s = AddSub::default();
        let mut rng = StdRng::seed_from_u64(0);
        assert!((0..10_000).all(|_| s.sub(&mut rng).answer() >= 2));
    }

    /// Catches a half-open range sneaking in where an inclusive one belongs.
    #[test]
    fn ranges_include_both_ends() {
        let s = AddSub {
            lhs: 1..=3,
            rhs: 1..=3,
        };
        let mut rng = StdRng::seed_from_u64(0);
        let seen: Vec<_> = (0..1_000).map(|_| s.add(&mut rng).lhs).collect();
        assert!(seen.contains(&1) && seen.contains(&3));
    }
}
