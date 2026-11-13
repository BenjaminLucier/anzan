//! Dice, coins and expected value.

use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};

use super::Range;
use crate::problem::Problem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Probability {
    /// How many fair coins are flipped.
    pub flips: Range,
    /// Odds in an expected-value question are a fraction with one of
    /// these denominators.
    pub odds: Range,
    /// Largest stake won or lost, in whole units.
    pub stakes: Range,
}

impl Default for Probability {
    fn default() -> Self {
        Self {
            flips: Range::new(2, 6),
            odds: Range::new(2, 6),
            stakes: Range::new(2, 100),
        }
    }
}

impl Probability {
    /// "Chance two dice total 8": 5/36.
    pub fn dice(&self, rng: &mut impl Rng) -> Problem {
        Problem::Dice {
            total: rng.random_range(2..=12),
        }
    }

    /// "Chance of exactly 3 heads in 5 flips": 10/32.
    pub fn coins(&self, rng: &mut impl Rng) -> Problem {
        let flips = self.flips.sample(rng);
        Problem::Coins {
            heads: rng.random_range(0..=flips),
            flips,
        }
    }

    /// "Win 30 with chance 1/4, else lose 8". Stakes are multiples of the
    /// denominator, so the expected value comes out whole.
    pub fn expected_value(&self, rng: &mut impl Rng) -> Problem {
        let den = self.odds.sample(rng);
        // Coprime, so both chances are in lowest terms: 1/4 and 3/4, never
        // 2/4 and 2/4. 1 always qualifies.
        let num = loop {
            let num = rng.random_range(1..den);
            if gcd(num, den) == 1 {
                break num;
            }
        };
        let units = (self.stakes.hi() / den).max(1);
        let lo = ((self.stakes.lo() + den - 1) / den).clamp(1, units);
        let stake = |rng: &mut _| den * Range::new(lo, units).sample(rng);
        Problem::ExpectedValue {
            win: stake(rng),
            num,
            den,
            lose: stake(rng),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.flips.lo() < 1 || self.flips.hi() > 10 {
            return Err("`probability.flips` must lie within [1, 10]".into());
        }
        if self.odds.lo() < 2 || self.odds.hi() > 12 {
            return Err("`probability.odds` must lie within [2, 12]".into());
        }
        if self.stakes.lo() < 1 || self.stakes.hi() > 1_000_000 {
            return Err("`probability.stakes` must lie within [1, 1000000]".into());
        }
        if self.stakes.hi() < self.odds.hi() {
            return Err("`probability.stakes` must reach the largest of `odds`".into());
        }
        Ok(())
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::rational::Rational;

    #[test]
    fn expected_values_are_whole() {
        let p = Probability::default();
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..500 {
            let problem = p.expected_value(&mut rng);
            let Problem::ExpectedValue {
                win,
                num,
                den,
                lose,
            } = problem
            else {
                panic!("{problem:?}");
            };
            assert!(0 < num && num < den && gcd(num, den) == 1, "{problem:?}");
            assert!(
                p.stakes.contains(win) && p.stakes.contains(lose),
                "{problem:?}"
            );
            let answer = problem.answer().to_string();
            assert!(
                answer.trim_start_matches('-').parse::<i64>().is_ok(),
                "{answer}"
            );
        }
    }

    #[test]
    fn known_answers() {
        let check = |p: Problem, want: &str| assert!(p.answer().check(want), "{p}: {want}");
        check(Problem::Dice { total: 7 }, "1/6");
        check(Problem::Dice { total: 2 }, "1/36");
        check(Problem::Dice { total: 8 }, "5/36");
        check(Problem::Coins { heads: 3, flips: 5 }, "10/32");
        check(Problem::Coins { heads: 3, flips: 5 }, "0.3125");
        check(Problem::Coins { heads: 0, flips: 4 }, "1/16");
        let bet = Problem::ExpectedValue {
            win: 32,
            num: 1,
            den: 4,
            lose: 8,
        };
        check(bet, "2");
        let losing = Problem::ExpectedValue {
            win: 4,
            num: 1,
            den: 2,
            lose: 10,
        };
        check(losing, "-3");
        assert_eq!(Rational::int(-3).to_string(), losing.answer().to_string());
    }
}
