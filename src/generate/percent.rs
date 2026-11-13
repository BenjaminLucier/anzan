//! Percentages: of an amount, changes, and changes on top of changes.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::Range;
use crate::problem::Problem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Percent {
    /// Percentages to take of an amount.
    pub rates: Range,
    /// Amounts to take a percentage of, and starting values for changes.
    pub amounts: Range,
    /// Percentage changes, rounded to multiples of 5 so results stay whole.
    pub changes: Range,
}

impl Default for Percent {
    fn default() -> Self {
        Self {
            rates: Range::new(1, 99),
            amounts: Range::new(10, 200),
            changes: Range::new(-50, 100),
        }
    }
}

impl Percent {
    /// "8% of 50". Swapping makes many easy: 8% of 50 is 50% of 8.
    pub fn of(&self, rng: &mut impl Rng) -> Problem {
        Problem::PercentOf {
            rate: self.rates.sample(rng),
            amount: self.amounts.sample(rng),
        }
    }

    /// "40 → 50": the percentage change. The start is a multiple of 20 and
    /// the change a multiple of 5, so the end is a whole number.
    pub fn change(&self, rng: &mut impl Rng) -> Problem {
        let from = (self.amounts.sample(rng) / 20).max(1) * 20;
        let change = self.nonzero_change(rng);
        Problem::PercentChange {
            from,
            to: from * (100 + change) / 100,
        }
    }

    /// "+20%, −10%": the net change of one change after another.
    pub fn successive(&self, rng: &mut impl Rng) -> Problem {
        Problem::Successive {
            first: self.nonzero_change(rng),
            second: self.nonzero_change(rng),
        }
    }

    fn nonzero_change(&self, rng: &mut impl Rng) -> i64 {
        loop {
            let change = self.changes.sample(rng) / 5 * 5;
            if change != 0 {
                return change;
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.amounts.lo() < 1 || self.amounts.hi() > 1_000_000 {
            return Err("`percent.amounts` must lie within [1, 1000000]".into());
        }
        if self.changes.lo() <= -100 || self.changes.hi() > 1_000 {
            return Err("`percent.changes` must lie within [-99, 1000]".into());
        }
        if !(self.changes.lo()..=self.changes.hi()).any(|c| c != 0 && c % 5 == 0) {
            return Err("`percent.changes` must include a nonzero multiple of 5".into());
        }
        if self.rates.lo() < 0 || self.rates.hi() > 1_000 {
            return Err("`percent.rates` must lie within [0, 1000]".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    #[test]
    fn changes_end_on_whole_numbers() {
        let s = Percent::default();
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..2_000 {
            let Problem::PercentChange { from, to } = s.change(&mut rng) else {
                panic!("not a change");
            };
            assert!(from > 0 && from % 20 == 0);
            let change = (to - from) * 100 / from;
            assert_eq!(from * (100 + change), to * 100, "{from} → {to}");
            assert!(change != 0 && change % 5 == 0);
        }
    }

    #[test]
    fn successive_changes_are_nonzero_multiples_of_5() {
        let s = Percent::default();
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..2_000 {
            let Problem::Successive { first, second } = s.successive(&mut rng) else {
                panic!("not successive");
            };
            for c in [first, second] {
                assert!(c != 0 && c % 5 == 0 && s.changes.contains(c));
            }
        }
    }
}
