//! Converting between fractions, decimals and percentages.

use rand::{Rng, RngExt, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

use super::Range;
use crate::{
    problem::{Form, Problem},
    rational::Rational,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fractions {
    /// Denominators of the fractions asked about, in lowest terms.
    pub denominators: Range,
}

impl Default for Fractions {
    fn default() -> Self {
        Self {
            denominators: Range::new(2, 20),
        }
    }
}

/// Every conversion, from one form to another.
const CONVERSIONS: [(Form, Form); 6] = [
    (Form::Fraction, Form::Decimal),
    (Form::Fraction, Form::Percent),
    (Form::Decimal, Form::Fraction),
    (Form::Decimal, Form::Percent),
    (Form::Percent, Form::Fraction),
    (Form::Percent, Form::Decimal),
];

impl Fractions {
    /// A proper fraction in lowest terms, shown in one form and asked for
    /// in another. Decimals and percentages are only shown when they end,
    /// since 0.142857… cannot be shown exactly.
    pub fn convert(&self, rng: &mut impl Rng) -> Problem {
        let &(from, to) = CONVERSIONS.choose(rng).expect("not empty");
        let value = loop {
            let den = self.denominators.sample(rng);
            let num = rng.random_range(1..den);
            let value = Rational::new(num.into(), den.into()).expect("den > 1");
            if value.den() == i128::from(den) && (from == Form::Fraction || value.terminates()) {
                break value;
            }
        };
        Problem::Convert {
            num: value.num() as i64,
            den: value.den() as i64,
            from,
            to,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let (lo, hi) = (self.denominators.lo(), self.denominators.hi());
        // At least one terminating denominator (2, 4 or 5) is needed for
        // decimals and percentages to be shown.
        if lo < 2 || hi > 1_000 || !(lo..=hi).any(|d| [2, 4, 5].contains(&d)) {
            return Err(
                "`fractions.denominators` must lie within [2, 1000] and include 2, 4 or 5".into(),
            );
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
    fn fractions_are_proper_in_lowest_terms_and_shown_exactly() {
        let s = Fractions::default();
        let mut rng = StdRng::seed_from_u64(0);
        let mut seen = HashSet::new();
        for _ in 0..2_000 {
            let Problem::Convert { num, den, from, to } = s.convert(&mut rng) else {
                panic!("not a conversion");
            };
            seen.insert((from, to));
            assert_ne!(from, to);
            assert!(0 < num && num < den && s.denominators.contains(den));
            let value = Rational::new(num.into(), den.into()).unwrap();
            assert_eq!(
                value.den(),
                i128::from(den),
                "{num}/{den} is not in lowest terms"
            );
            if from != Form::Fraction {
                assert!(value.terminates(), "{num}/{den} shown as a {from:?}");
            }
        }
        assert_eq!(seen.len(), CONVERSIONS.len());
    }
}
