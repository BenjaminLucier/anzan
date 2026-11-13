//! Basis points, compound growth and the rule of 72.

use rand::{Rng, RngExt, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

use super::Range;
use crate::problem::{BpTo, Problem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Finance {
    /// Basis points to convert or apply.
    pub bps: Range,
    /// Annual growth rates, in percent, for compounding.
    pub rates: Range,
    /// Years of compounding.
    pub years: Range,
    /// Annual rates, in percent, for doubling-time questions.
    pub doubling: Range,
}

impl Default for Finance {
    fn default() -> Self {
        Self {
            bps: Range::new(1, 500),
            rates: Range::new(1, 20),
            years: Range::new(2, 5),
            doubling: Range::new(1, 24),
        }
    }
}

/// Round amounts that basis points are taken of.
const AMOUNTS: [i64; 8] = [
    10_000,
    50_000,
    100_000,
    250_000,
    1_000_000,
    5_000_000,
    10_000_000,
    100_000_000,
];

/// Starting amounts for compounding.
const PRINCIPALS: [i64; 3] = [100, 1_000, 10_000];

impl Finance {
    /// "150 bp as a percentage", "0.25% in basis points", or "35 bp of
    /// 1,000,000".
    pub fn basis_points(&self, rng: &mut impl Rng) -> Problem {
        let bp = self.bps.sample(rng);
        let (to, amount) = match rng.random_range(0..3) {
            0 => (BpTo::Percent, None),
            1 => (BpTo::Points, None),
            _ => (BpTo::Amount, AMOUNTS.choose(rng).copied()),
        };
        Problem::BasisPoints { bp, to, amount }
    }

    /// "1000 × 1.1³": what a sum grows to at a yearly rate.
    pub fn compound(&self, rng: &mut impl Rng) -> Problem {
        Problem::Compound {
            principal: *PRINCIPALS.choose(rng).expect("not empty"),
            rate: self.rates.sample(rng),
            years: self.years.sample(rng),
        }
    }

    /// "Years to double at 8%": 72 ÷ 8 = 9.
    pub fn doubling(&self, rng: &mut impl Rng) -> Problem {
        Problem::Doubling {
            rate: self.doubling.sample(rng),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.bps.lo() < 1 || self.bps.hi() > 10_000 {
            return Err("`finance.bps` must lie within [1, 10000]".into());
        }
        if self.rates.lo() < 1 || self.rates.hi() > 100 {
            return Err("`finance.rates` must lie within [1, 100]".into());
        }
        if self.years.lo() < 1 || self.years.hi() > 30 {
            return Err("`finance.years` must lie within [1, 30]".into());
        }
        if self.doubling.lo() < 1 || self.doubling.hi() > 100 {
            return Err("`finance.doubling` must lie within [1, 100]".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    #[test]
    fn basis_points_ask_every_way() {
        let s = Finance::default();
        let mut rng = StdRng::seed_from_u64(0);
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            let Problem::BasisPoints { bp, to, amount } = s.basis_points(&mut rng) else {
                panic!("not basis points");
            };
            assert!(s.bps.contains(bp));
            assert_eq!(amount.is_some(), to == BpTo::Amount);
            seen.insert(to);
        }
        assert_eq!(seen.len(), 3);
    }
}
