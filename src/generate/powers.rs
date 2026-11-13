//! Squares, cubes and powers of two: facts to know by heart.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::Range;
use crate::problem::Problem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Powers {
    /// Numbers to square.
    pub squares: Range,
    /// Numbers to cube.
    pub cubes: Range,
    /// Exponents of 2.
    pub twos: Range,
    /// Numbers to estimate square roots of. Perfect squares are skipped.
    pub roots: Range,
}

impl Default for Powers {
    fn default() -> Self {
        Self {
            squares: Range::new(2, 30),
            cubes: Range::new(2, 12),
            twos: Range::new(1, 20),
            roots: Range::new(2, 1_000),
        }
    }
}

impl Powers {
    pub fn square(&self, rng: &mut impl Rng) -> Problem {
        Problem::Square {
            base: self.squares.sample(rng),
        }
    }

    pub fn cube(&self, rng: &mut impl Rng) -> Problem {
        Problem::Cube {
            base: self.cubes.sample(rng),
        }
    }

    pub fn power_of_two(&self, rng: &mut impl Rng) -> Problem {
        Problem::PowerOfTwo {
            exp: self.twos.sample(rng),
        }
    }

    /// "√50": estimate a square root that is not a whole number.
    pub fn sqrt(&self, rng: &mut impl Rng) -> Problem {
        let n = (0..10_000)
            .map(|_| self.roots.sample(rng))
            .find(|&n| !is_square(n))
            .unwrap_or(2);
        Problem::Sqrt { n }
    }

    /// Checks that no answer overflows.
    pub fn validate(&self) -> Result<(), String> {
        if !self.squares.within(1_000_000_000) || !self.cubes.within(2_000_000) {
            return Err("`powers` bases are too large".into());
        }
        if self.twos.lo() < 0 || self.twos.hi() > 62 {
            return Err("`powers.twos` must be within [0, 62]".into());
        }
        // Of any two neighbours, at most one is a square.
        let lone_square = self.roots.lo() == self.roots.hi() && is_square(self.roots.lo());
        if self.roots.lo() < 2 || self.roots.hi() > 1_000_000_000 || lone_square {
            return Err(
                "`powers.roots` must lie within [2, 1000000000] and include a non-square".into(),
            );
        }
        Ok(())
    }
}

fn is_square(n: i64) -> bool {
    let root = (n as f64).sqrt().round() as i64;
    root * root == n
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    proptest! {
        #[test]
        fn bases_stay_in_range(seed: u64) {
            let p = Powers::default();
            let mut rng = StdRng::seed_from_u64(seed);
            let Problem::Square { base } = p.square(&mut rng) else { panic!() };
            prop_assert!(p.squares.contains(base));
            let Problem::Cube { base } = p.cube(&mut rng) else { panic!() };
            prop_assert!(p.cubes.contains(base));
            let Problem::PowerOfTwo { exp } = p.power_of_two(&mut rng) else { panic!() };
            prop_assert!(p.twos.contains(exp));
        }
    }

    #[test]
    fn roots_are_never_whole() {
        let p = Powers::default();
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..2_000 {
            let Problem::Sqrt { n } = p.sqrt(&mut rng) else {
                panic!()
            };
            assert!(p.roots.contains(n) && !is_square(n), "{n}");
        }
    }

    #[test]
    fn validate_rejects_overflow() {
        assert!(Powers::default().validate().is_ok());
        let p = Powers {
            twos: Range::new(1, 63),
            ..Powers::default()
        };
        assert!(p.validate().is_err());
        let only_a_square = Powers {
            roots: Range::new(9, 9),
            ..Powers::default()
        };
        assert!(only_a_square.validate().is_err());
    }
}
