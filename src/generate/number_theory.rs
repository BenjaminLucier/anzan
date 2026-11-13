//! Divisibility, primes and prime factorization.

use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};

use super::Range;
use crate::{number::is_prime, problem::Problem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NumberTheory {
    /// Divisors to test divisibility by.
    pub divisors: Range,
    /// Numbers to test for divisibility.
    pub dividends: Range,
    /// Numbers to test for primality.
    pub primes: Range,
    /// Numbers to factorize. Primes are never asked.
    pub factorize: Range,
}

impl Default for NumberTheory {
    fn default() -> Self {
        Self {
            divisors: Range::new(2, 13),
            dividends: Range::new(100, 9_999),
            primes: Range::new(2, 100),
            factorize: Range::new(4, 1_000),
        }
    }
}

/// Tries before giving up on a constraint, so impossible settings cannot
/// hang. Validation makes sure it is never reached.
const TRIES: usize = 10_000;

impl NumberTheory {
    /// "Is n divisible by d?", with even odds of yes and no.
    pub fn divisible(&self, rng: &mut impl Rng) -> Problem {
        let d = self.divisors.sample(rng);
        let want = rng.random_bool(0.5);
        let n = (0..TRIES)
            .map(|_| self.dividends.sample(rng))
            .find(|n| (n % d == 0) == want)
            .unwrap_or_else(|| self.dividends.sample(rng));
        Problem::Divisible { n, d }
    }

    /// "Is n prime?", with even odds of yes and no. Composites divisible by
    /// 2 or 5 are given away by their last digit, so they are left out
    /// when the range has others.
    pub fn is_prime(&self, rng: &mut impl Rng) -> Problem {
        let want = rng.random_bool(0.5);
        let tricky = |n: i64| want == is_prime(n as u64) && (want || (n % 2 != 0 && n % 5 != 0));
        let n = (0..TRIES)
            .map(|_| self.primes.sample(rng))
            .find(|&n| tricky(n))
            .unwrap_or_else(|| self.primes.sample(rng));
        Problem::IsPrime { n }
    }

    /// "Factorize n", for a composite n.
    pub fn factorize(&self, rng: &mut impl Rng) -> Problem {
        let n = (0..TRIES)
            .map(|_| self.factorize.sample(rng))
            .find(|&n| !is_prime(n as u64))
            .unwrap_or(4);
        Problem::Factorize { n }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.divisors.lo() < 1 || self.dividends.lo() < 0 {
            return Err("`number_theory.divisors` and `dividends` must be positive".into());
        }
        if self.primes.lo() < 0 {
            return Err("`number_theory.primes` must not be negative".into());
        }
        if self.factorize.lo() < 4 {
            return Err("`number_theory.factorize` must start at 4 or more".into());
        }
        if self.primes.hi() > 1_000_000_000 || self.factorize.hi() > 1_000_000_000 {
            return Err("`number_theory` numbers must be at most 1,000,000,000".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    fn many(f: impl Fn(&mut StdRng) -> Problem) -> Vec<Problem> {
        let mut rng = StdRng::seed_from_u64(0);
        (0..2_000).map(|_| f(&mut rng)).collect()
    }

    #[test]
    fn divisible_is_balanced() {
        let s = NumberTheory::default();
        let yes = many(|rng| s.divisible(rng))
            .into_iter()
            .filter(|p| matches!(p, Problem::Divisible { n, d } if n % d == 0))
            .count();
        assert!((800..1_200).contains(&yes), "{yes} of 2000");
    }

    #[test]
    fn prime_questions_are_balanced_and_not_given_away() {
        let s = NumberTheory::default();
        let mut primes = 0;
        for p in many(|rng| s.is_prime(rng)) {
            let Problem::IsPrime { n } = p else { panic!() };
            assert!(s.primes.contains(n));
            if is_prime(n as u64) {
                primes += 1;
            } else {
                assert!(n % 2 != 0 && n % 5 != 0, "{n} is given away");
            }
        }
        assert!((800..1_200).contains(&primes), "{primes} of 2000");
    }

    #[test]
    fn factorize_asks_composites() {
        let s = NumberTheory::default();
        for p in many(|rng| s.factorize(rng)) {
            let Problem::Factorize { n } = p else {
                panic!()
            };
            assert!(s.factorize.contains(n) && !is_prime(n as u64), "{n}");
        }
    }
}
