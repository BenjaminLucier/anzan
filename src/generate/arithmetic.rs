//! The four basic operations.
//!
//! Subtraction and division are generated as addition and multiplication
//! run backwards, so answers are never negative and divisions are exact.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::Range;
use crate::problem::Problem;

/// Settings for addition and subtraction.
///
/// An addition draws each operand from its range. A subtraction turns
/// `a + b = c` into `c − b`, so its answer always lies in `lhs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AddSub {
    pub lhs: Range,
    pub rhs: Range,
}

impl Default for AddSub {
    /// Both operands from 2 to 100.
    fn default() -> Self {
        Self {
            lhs: Range::new(2, 100),
            rhs: Range::new(2, 100),
        }
    }
}

impl AddSub {
    pub fn add(&self, rng: &mut impl Rng) -> Problem {
        Problem::Add {
            lhs: self.lhs.sample(rng),
            rhs: self.rhs.sample(rng),
        }
    }

    pub fn sub(&self, rng: &mut impl Rng) -> Problem {
        let (a, b) = (self.lhs.sample(rng), self.rhs.sample(rng));
        Problem::Sub { lhs: a + b, rhs: b }
    }
}

/// Settings for multiplication and division.
///
/// A multiplication draws each operand from its range. A division turns
/// `a × b = c` into `c ÷ a`, so it divides exactly by a number from `lhs`
/// and its answer lies in `rhs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MulDiv {
    pub lhs: Range,
    pub rhs: Range,
}

impl Default for MulDiv {
    /// A single-digit (up to 12) factor times one up to 100.
    fn default() -> Self {
        Self {
            lhs: Range::new(2, 12),
            rhs: Range::new(2, 100),
        }
    }
}

impl MulDiv {
    pub fn mul(&self, rng: &mut impl Rng) -> Problem {
        Problem::Mul {
            lhs: self.lhs.sample(rng),
            rhs: self.rhs.sample(rng),
        }
    }

    /// # Panics
    ///
    /// Panics if `lhs` contains zero.
    pub fn div(&self, rng: &mut impl Rng) -> Problem {
        let (a, b) = (self.lhs.sample(rng), self.rhs.sample(rng));
        Problem::Div { lhs: a * b, rhs: a }
    }
}

/// Settings for long multiplication: by default a two- or three-digit
/// number times a two-digit one, such as 47 × 38 or 365 × 24.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LongMul {
    pub lhs: Range,
    pub rhs: Range,
}

impl Default for LongMul {
    fn default() -> Self {
        Self {
            lhs: Range::new(10, 999),
            rhs: Range::new(10, 99),
        }
    }
}

impl LongMul {
    pub fn generate(&self, rng: &mut impl Rng) -> Problem {
        Problem::LongMul {
            lhs: self.lhs.sample(rng),
            rhs: self.rhs.sample(rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    /// Ranges of any sign, including ranges that hold a single value.
    fn range() -> impl Strategy<Value = Range> {
        (-1_000i64..1_000, 0i64..1_000).prop_map(|(lo, len)| Range::new(lo, lo + len))
    }

    /// Ranges of positive numbers, which divisors need.
    fn positive_range() -> impl Strategy<Value = Range> {
        (1i64..1_000, 0i64..1_000).prop_map(|(lo, len)| Range::new(lo, lo + len))
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
            let Problem::Add { lhs, rhs } = s.add(&mut StdRng::seed_from_u64(seed)) else {
                panic!("not an addition");
            };
            prop_assert!(s.lhs.contains(lhs));
            prop_assert!(s.rhs.contains(rhs));
        }

        #[test]
        fn sub_reverses_an_add(s in add_sub(), seed: u64) {
            let Problem::Sub { lhs, rhs } = s.sub(&mut StdRng::seed_from_u64(seed)) else {
                panic!("not a subtraction");
            };
            prop_assert!(s.lhs.contains(lhs - rhs));
            prop_assert!(s.rhs.contains(rhs));
        }

        #[test]
        fn mul_operands_stay_in_range(s in mul_div(), seed: u64) {
            let Problem::Mul { lhs, rhs } = s.mul(&mut StdRng::seed_from_u64(seed)) else {
                panic!("not a multiplication");
            };
            prop_assert!(s.lhs.contains(lhs));
            prop_assert!(s.rhs.contains(rhs));
        }

        #[test]
        fn div_is_exact_and_reverses_a_mul(s in mul_div(), seed: u64) {
            let Problem::Div { lhs, rhs } = s.div(&mut StdRng::seed_from_u64(seed)) else {
                panic!("not a division");
            };
            prop_assert_eq!(lhs % rhs, 0);
            prop_assert!(s.lhs.contains(rhs));
            prop_assert!(s.rhs.contains(lhs / rhs));
        }
    }

    #[test]
    fn long_mul_operands_stay_in_range() {
        let s = LongMul::default();
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..1_000 {
            let Problem::LongMul { lhs, rhs } = s.generate(&mut rng) else {
                panic!("not a long multiplication");
            };
            assert!(s.lhs.contains(lhs) && s.rhs.contains(rhs));
        }
    }

    #[test]
    fn defaults_never_go_negative() {
        let (add_sub, mul_div) = (AddSub::default(), MulDiv::default());
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..10_000 {
            for p in [add_sub.sub(&mut rng), mul_div.div(&mut rng)] {
                let (op, lhs, rhs) = p.arithmetic().unwrap();
                assert!(op.apply(lhs, rhs) >= 2, "{p}");
            }
        }
    }
}
