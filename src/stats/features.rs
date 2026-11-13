//! What a problem exercises: the skills an attempt says something about.
//!
//! Features are derived from a problem's operands whenever history is
//! loaded rather than stored with it, so they can be improved later
//! without rewriting any history.

use std::fmt;

use crate::problem::{Op, Problem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Feature {
    /// This exact question, with the operands of `+` and `×` in order, so
    /// 7 × 38 and 38 × 7 count as the same item.
    Item(Problem),
    /// A single-digit multiplication fact used along the way, smaller digit
    /// first. 47 × 8 uses 4 × 8 and 7 × 8. Facts with 0 or 1 are left out.
    Fact(u8, u8),
    /// The number of carries in an addition.
    Carries(u8),
    /// The number of borrows in a subtraction.
    Borrows(u8),
    /// The denominator of a fraction being converted.
    Denominator(i64),
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Feature::Item(problem) => write!(f, "{problem}"),
            Feature::Fact(a, b) => write!(f, "{a} × {b}"),
            Feature::Carries(n) => write!(f, "additions with {n} carries"),
            Feature::Borrows(n) => write!(f, "subtractions with {n} borrows"),
            Feature::Denominator(d) => write!(f, "fractions over {d}"),
        }
    }
}

/// Every feature `problem` exercises, starting with the item itself.
pub fn features(problem: &Problem) -> Vec<Feature> {
    let mut out = vec![Feature::Item(canonical(problem))];
    if let Some((op, lhs, rhs)) = problem.arithmetic() {
        match op {
            Op::Add => out.push(Feature::Carries(carries(lhs, rhs))),
            Op::Sub => out.push(Feature::Borrows(borrows(lhs, rhs))),
            Op::Mul => out.extend(facts(lhs, rhs)),
            // A division undoes a multiplication: quotient × divisor.
            Op::Div if rhs != 0 => out.extend(facts(lhs / rhs, rhs)),
            Op::Div => {}
        }
    }
    if let Problem::Convert { den, .. } = problem {
        out.push(Feature::Denominator(*den));
    }
    out
}

/// `problem` with the operands of commutative operations in ascending
/// order.
pub fn canonical(problem: &Problem) -> Problem {
    match *problem {
        Problem::Add { lhs, rhs } => Problem::Add {
            lhs: lhs.min(rhs),
            rhs: lhs.max(rhs),
        },
        Problem::Mul { lhs, rhs } => Problem::Mul {
            lhs: lhs.min(rhs),
            rhs: lhs.max(rhs),
        },
        Problem::LongMul { lhs, rhs } => Problem::LongMul {
            lhs: lhs.min(rhs),
            rhs: lhs.max(rhs),
        },
        other => other,
    }
}

fn digits(n: i64) -> impl Iterator<Item = u8> {
    n.unsigned_abs()
        .to_string()
        .into_bytes()
        .into_iter()
        .map(|b| b - b'0')
}

/// The distinct single-digit facts in `a × b`, from 2 × 2 up.
fn facts(a: i64, b: i64) -> Vec<Feature> {
    let mut facts: Vec<_> = digits(a)
        .flat_map(|x| digits(b).map(move |y| (x.min(y), x.max(y))))
        .filter(|&(x, _)| x >= 2)
        .map(|(x, y)| Feature::Fact(x, y))
        .collect();
    facts.sort();
    facts.dedup();
    facts
}

/// Column carries when adding two non-negative numbers.
pub fn carries(a: i64, b: i64) -> u8 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    let (mut carry, mut count) = (0, 0);
    while a > 0 || b > 0 {
        carry = u64::from(a % 10 + b % 10 + carry >= 10);
        count += carry as u8;
        (a, b) = (a / 10, b / 10);
    }
    count
}

/// Column borrows when subtracting `b` from `a`, for `a ≥ b ≥ 0`.
pub fn borrows(a: i64, b: i64) -> u8 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    let (mut borrow, mut count) = (0, 0);
    while a > 0 || b > 0 {
        borrow = u64::from(a % 10 < b % 10 + borrow);
        count += borrow as u8;
        (a, b) = (a / 10, b / 10);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::Form;

    #[test]
    fn carries_and_borrows() {
        assert_eq!(carries(47, 38), 1);
        assert_eq!(carries(45, 32), 0);
        assert_eq!(carries(99, 1), 2);
        assert_eq!(carries(999, 999), 3);
        assert_eq!(borrows(85, 38), 1);
        assert_eq!(borrows(100, 1), 2);
        assert_eq!(borrows(57, 23), 0);
        assert_eq!(borrows(1000, 999), 3);
    }

    #[test]
    fn multiplication_facts() {
        let p = Problem::Mul { lhs: 47, rhs: 8 };
        let item = Problem::Mul { lhs: 8, rhs: 47 };
        assert_eq!(
            features(&p),
            [
                Feature::Item(item),
                Feature::Fact(4, 8),
                Feature::Fact(7, 8)
            ]
        );
        // Facts with 0 or 1 are not worth tracking.
        assert!(facts(10, 31).is_empty());
    }

    #[test]
    fn division_uses_the_multiplication_it_undoes() {
        let p = Problem::Div { lhs: 266, rhs: 7 };
        assert_eq!(
            features(&p)[1..],
            [Feature::Fact(3, 7), Feature::Fact(7, 8)]
        );
    }

    #[test]
    fn commutative_items_match() {
        let a = Problem::Mul { lhs: 38, rhs: 7 };
        let b = Problem::Mul { lhs: 7, rhs: 38 };
        assert_eq!(features(&a)[0], features(&b)[0]);
        let c = Problem::Sub { lhs: 38, rhs: 7 };
        assert_eq!(canonical(&c), c);
    }

    #[test]
    fn other_features() {
        let p = Problem::Add { lhs: 47, rhs: 38 };
        assert_eq!(features(&p)[1], Feature::Carries(1));
        let p = Problem::Convert {
            num: 1,
            den: 7,
            from: Form::Fraction,
            to: Form::Decimal,
        };
        assert_eq!(features(&p)[1], Feature::Denominator(7));
        assert_eq!(features(&Problem::Square { base: 17 }).len(), 1);
    }
}
