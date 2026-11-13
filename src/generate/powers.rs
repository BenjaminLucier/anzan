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
}

impl Default for Powers {
    fn default() -> Self {
        Self {
            squares: Range::new(2, 30),
            cubes: Range::new(2, 12),
            twos: Range::new(1, 20),
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

    /// Checks that no answer overflows.
    pub fn validate(&self) -> Result<(), String> {
        let fits = |r: Range, limit: i64| r.lo() >= -limit && r.hi() <= limit;
        if !fits(self.squares, 3_000_000_000) || !fits(self.cubes, 2_000_000) {
            return Err("`powers` bases are too large".into());
        }
        if self.twos.lo() < 0 || self.twos.hi() > 62 {
            return Err("`powers.twos` must be within [0, 62]".into());
        }
        Ok(())
    }
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
    fn validate_rejects_overflow() {
        assert!(Powers::default().validate().is_ok());
        let p = Powers {
            twos: Range::new(1, 63),
            ..Powers::default()
        };
        assert!(p.validate().is_err());
    }
}
