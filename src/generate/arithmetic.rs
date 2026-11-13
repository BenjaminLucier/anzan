//! The four basic operations.
//!
//! Subtraction and division are generated as addition and multiplication
//! run backwards, so answers are never negative and divisions are exact.

use std::ops::RangeInclusive;

use rand::{Rng, RngExt};

use crate::problem::{Op, Problem};

/// Settings for addition and subtraction.
///
/// An addition draws each operand from its range. A subtraction turns
/// `a + b = c` into `c − b`, so its answer always lies in `lhs`.
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

/// Settings for multiplication and division.
///
/// A multiplication draws each operand from its range. A division turns
/// `a × b = c` into `c ÷ a`, so it divides exactly by a number from `lhs`
/// and its answer lies in `rhs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulDiv {
    pub lhs: RangeInclusive<i64>,
    pub rhs: RangeInclusive<i64>,
}

impl Default for MulDiv {
    /// A single-digit (up to 12) factor times one up to 100.
    fn default() -> Self {
        Self {
            lhs: 2..=12,
            rhs: 2..=100,
        }
    }
}

impl MulDiv {
    /// # Panics
    ///
    /// Panics if either range is empty.
    pub fn mul(&self, rng: &mut impl Rng) -> Problem {
        Problem {
            op: Op::Mul,
            lhs: rng.random_range(self.lhs.clone()),
            rhs: rng.random_range(self.rhs.clone()),
        }
    }

    /// # Panics
    ///
    /// Panics if either range is empty or `lhs` contains zero.
    pub fn div(&self, rng: &mut impl Rng) -> Problem {
        let product = self.mul(rng);
        Problem {
            op: Op::Div,
            lhs: product.answer(),
            rhs: product.lhs,
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

    /// Non-empty ranges of positive numbers, which divisors need.
    fn positive_range() -> impl Strategy<Value = RangeInclusive<i64>> {
        (1i64..1_000, 0i64..1_000).prop_map(|(lo, len)| lo..=lo + len)
    }

    fn add_sub() -> impl Strategy<Value = AddSub> {
        (range(), range()).prop_map(|(lhs, rhs)| AddSub { lhs, rhs })
    }

    fn mul_div() -> impl Strategy<Value = MulDiv> {
        (positive_range(), range()).prop_map(|(lhs, rhs)| MulDiv { lhs, rhs })
    }

    proptest! {
        #[test]
        fn add_operands_stay_in_range(s in add_sub(), seed: u64) {
            let p = s.add(&mut StdRng::seed_from_u64(seed));
            prop_assert_eq!(p.op, Op::Add);
            prop_assert!(s.lhs.contains(&p.lhs));
            prop_assert!(s.rhs.contains(&p.rhs));
        }

        #[test]
        fn sub_reverses_an_add(s in add_sub(), seed: u64) {
            let p = s.sub(&mut StdRng::seed_from_u64(seed));
            prop_assert_eq!(p.op, Op::Sub);
            prop_assert!(s.lhs.contains(&p.answer()));
            prop_assert!(s.rhs.contains(&p.rhs));
        }

        #[test]
        fn mul_operands_stay_in_range(s in mul_div(), seed: u64) {
            let p = s.mul(&mut StdRng::seed_from_u64(seed));
            prop_assert_eq!(p.op, Op::Mul);
            prop_assert!(s.lhs.contains(&p.lhs));
            prop_assert!(s.rhs.contains(&p.rhs));
        }

        #[test]
        fn div_is_exact_and_reverses_a_mul(s in mul_div(), seed: u64) {
            let p = s.div(&mut StdRng::seed_from_u64(seed));
            prop_assert_eq!(p.op, Op::Div);
            prop_assert_eq!(p.lhs % p.rhs, 0);
            prop_assert!(s.lhs.contains(&p.rhs));
            prop_assert!(s.rhs.contains(&p.answer()));
        }
    }

    #[test]
    fn defaults_never_go_negative() {
        let (add_sub, mul_div) = (AddSub::default(), MulDiv::default());
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..10_000 {
            assert!(add_sub.sub(&mut rng).answer() >= 2);
            assert!(mul_div.div(&mut rng).answer() >= 2);
        }
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
