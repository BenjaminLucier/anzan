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
    /// Multiplying by this number, from 2 to 12: its times table, and
    /// the same table stretched, as in 7 × 38.
    Times(u8),
    /// Dividing by this number, from 2 to 12.
    DividedBy(u8),
    /// The denominator of a fraction being converted.
    Denominator(i64),
    /// The divisor in a divisibility test, each of which has its own rule.
    Divisor(i64),
    /// A question with a well-known shortcut.
    Trick(Trick),
}

/// A shape of question that people answer with a shortcut rather than
/// head on. Being slow at one usually means not using the shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Trick {
    /// 47 + 29: adding a number that ends in 8 or 9.
    AddNearRound,
    /// 37 + 43: the units make ten.
    MakeTen,
    /// 83 − 29: taking away a number that ends in 8 or 9.
    SubtractNearRound,
    /// 80 − 26: taking away from a round number.
    FromRound,
    /// 7 × 39: multiplying by a number that ends in 8 or 9.
    MultiplyNearRound,
    /// 7 × 40: multiplying by a round number.
    RoundFactor,
    /// 16 × 35: an even number times one that ends in 5.
    DoubleAndHalve,
    /// 47 × 53: two numbers the same distance either side of an easy
    /// square.
    DifferenceOfSquares,
    /// 35²: squaring a number that ends in 5.
    SquareEndingInFive,
}

impl Trick {
    pub const ALL: [Trick; 9] = [
        Trick::AddNearRound,
        Trick::MakeTen,
        Trick::SubtractNearRound,
        Trick::FromRound,
        Trick::MultiplyNearRound,
        Trick::RoundFactor,
        Trick::DoubleAndHalve,
        Trick::DifferenceOfSquares,
        Trick::SquareEndingInFive,
    ];

    /// A short name, leading with the operation it belongs to.
    pub fn label(self) -> &'static str {
        match self {
            Trick::AddNearRound => "+ ending in 8 or 9",
            Trick::MakeTen => "+ units making ten",
            Trick::SubtractNearRound => "− ending in 8 or 9",
            Trick::FromRound => "− from a round number",
            Trick::MultiplyNearRound => "× ending in 8 or 9",
            Trick::RoundFactor => "× a round number",
            Trick::DoubleAndHalve => "× even by ending in 5",
            Trick::DifferenceOfSquares => "× difference of squares",
            Trick::SquareEndingInFive => "squares ending in 5",
        }
    }

    /// The shortcut, with an example.
    pub fn how(self) -> &'static str {
        match self {
            Trick::AddNearRound => "Round up, add, then take the extra off: 47 + 29 = 47 + 30 − 1.",
            Trick::MakeTen => "Pair the units into a ten first: 37 + 43 = 70 + 10.",
            Trick::SubtractNearRound => {
                "Round up, subtract, then add the extra back: 83 − 29 = 83 − 30 + 1."
            }
            Trick::FromRound => "Count up from the smaller number: 80 − 26 is 4 to 30, then 50.",
            Trick::MultiplyNearRound => {
                "Round up, multiply, then take the extra off: 7 × 39 = 7 × 40 − 7."
            }
            Trick::RoundFactor => {
                "Drop the zeros and put them back after: 7 × 40 = 7 × 4, then a 0."
            }
            Trick::DoubleAndHalve => "Halve one and double the other: 16 × 35 = 8 × 70.",
            Trick::DifferenceOfSquares => {
                "Square the middle and take off the square of the gap: 47 × 53 = 50² − 3²."
            }
            Trick::SquareEndingInFive => {
                "Multiply the tens by one more than itself, then write 25: 35² is 3 × 4, then 25."
            }
        }
    }
}

impl Feature {
    /// What the skill is, in a sentence, where the name does not say.
    pub fn about(&self) -> Option<&'static str> {
        match self {
            Feature::Fact(..) => Some("Every question that uses this fact along the way."),
            Feature::Times(_) => Some("Every multiplication with this as one of its numbers."),
            Feature::Trick(trick) => Some(trick.how()),
            _ => None,
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = |f: &mut fmt::Formatter<'_>, sign: char, n: u8, one: &str, many: &str| match n {
            0 => write!(f, "{sign} with no {many}"),
            1 => write!(f, "{sign} with 1 {one}"),
            n => write!(f, "{sign} with {n} {many}"),
        };
        match self {
            Feature::Item(problem) => write!(f, "{problem}"),
            Feature::Fact(a, b) => write!(f, "the fact {a} × {b}"),
            Feature::Carries(n) => count(f, '+', *n, "carry", "carries"),
            Feature::Borrows(n) => count(f, '−', *n, "borrow", "borrows"),
            Feature::Times(n) => write!(f, "multiplying by {n}"),
            Feature::DividedBy(n) => write!(f, "dividing by {n}"),
            Feature::Denominator(d) => write!(f, "fractions over {d}"),
            Feature::Divisor(d) => write!(f, "divisibility by {d}"),
            Feature::Trick(trick) => f.write_str(trick.label()),
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
            Op::Mul => {
                out.extend(facts(lhs, rhs));
                let mut tables: Vec<i64> = [lhs, rhs]
                    .into_iter()
                    .filter(|n| TABLES.contains(n))
                    .collect();
                tables.sort_unstable();
                tables.dedup();
                out.extend(tables.into_iter().map(|n| Feature::Times(n as u8)));
            }
            // A division undoes a multiplication: quotient × divisor.
            Op::Div if rhs != 0 => {
                out.extend(facts(lhs / rhs, rhs));
                if TABLES.contains(&rhs) {
                    out.push(Feature::DividedBy(rhs as u8));
                }
            }
            Op::Div => {}
        }
    }
    match *problem {
        Problem::Convert { den, .. } => out.push(Feature::Denominator(den)),
        Problem::Divisible { d, .. } => out.push(Feature::Divisor(d)),
        _ => {}
    }
    out.extend(tricks(problem).into_iter().map(Feature::Trick));
    out
}

/// The numbers with a times table worth tracking on its own.
const TABLES: std::ops::RangeInclusive<i64> = 2..=12;

/// The shortcuts `problem` invites.
pub fn tricks(problem: &Problem) -> Vec<Trick> {
    // Ends in 8 or 9, and is big enough for rounding up to be worth it.
    let near_round = |n: i64| n >= 18 && n % 10 >= 8;
    let round = |n: i64| n >= 10 && n % 10 == 0;
    let mut out = Vec::new();
    let mut add = |applies: bool, trick: Trick| {
        if applies {
            out.push(trick);
        }
    };
    match *problem {
        Problem::Add { lhs, rhs } if lhs >= 0 && rhs >= 0 => {
            add(near_round(lhs) || near_round(rhs), Trick::AddNearRound);
            add((lhs + rhs) % 10 == 0 && lhs % 10 != 0, Trick::MakeTen);
        }
        Problem::Sub { lhs, rhs } if lhs >= rhs && rhs >= 0 => {
            add(near_round(rhs), Trick::SubtractNearRound);
            add(round(lhs) && rhs % 10 != 0, Trick::FromRound);
        }
        Problem::Mul { lhs, rhs } | Problem::LongMul { lhs, rhs } if lhs > 0 && rhs > 0 => {
            let ends_in_five = |n: i64| n >= 15 && n % 10 == 5;
            let halves = |a: i64, b: i64| ends_in_five(a) && b % 2 == 0 && !round(b);
            add(round(lhs) || round(rhs), Trick::RoundFactor);
            add(near_round(lhs) || near_round(rhs), Trick::MultiplyNearRound);
            add(halves(lhs, rhs) || halves(rhs, lhs), Trick::DoubleAndHalve);
            // The middle must be easy to square, and the gap small.
            let (middle, gap) = ((lhs + rhs) / 2, (lhs - rhs).abs() / 2);
            let symmetric = (lhs + rhs) % 2 == 0 && middle >= 15 && middle % 5 == 0;
            add(
                symmetric && (1..=5).contains(&gap),
                Trick::DifferenceOfSquares,
            );
        }
        Problem::Square { base } => add(base >= 15 && base % 10 == 5, Trick::SquareEndingInFive),
        _ => {}
    }
    out
}

/// How long the numbers in an arithmetic problem are, as digit counts.
/// Problems of one kind and size are about as hard as each other, so
/// times are compared within a size: 12 × 97 against other two-digit by
/// two-digit products, not against 2 × 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Size(pub u8, pub u8);

impl Size {
    /// The size of `problem`, if it is arithmetic: the longer operand
    /// first for `+` and `×`, what is taken from and what is taken for
    /// `−`, and the answer and the divisor for `÷`.
    pub fn of(problem: &Problem) -> Option<Self> {
        let len = |n: i64| n.unsigned_abs().checked_ilog10().map_or(1, |d| d as u8 + 1);
        let (op, lhs, rhs) = problem.arithmetic()?;
        let (a, b) = match op {
            Op::Add | Op::Mul => (len(lhs).max(len(rhs)), len(lhs).min(len(rhs))),
            Op::Sub => (len(lhs), len(rhs)),
            Op::Div => (len(lhs.checked_div(rhs)?), len(rhs)),
        };
        Some(Self(a, b))
    }
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
                Feature::Fact(7, 8),
                Feature::Times(8)
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
            [
                Feature::Fact(3, 7),
                Feature::Fact(7, 8),
                Feature::DividedBy(7)
            ]
        );
    }

    #[test]
    fn tables_are_tracked_from_2_to_12() {
        let times = |lhs, rhs| -> Vec<Feature> {
            features(&Problem::Mul { lhs, rhs })
                .into_iter()
                .filter(|f| matches!(f, Feature::Times(_)))
                .collect()
        };
        assert_eq!(times(7, 8), [Feature::Times(7), Feature::Times(8)]);
        assert_eq!(times(7, 7), [Feature::Times(7)]);
        assert_eq!(times(12, 97), [Feature::Times(12)]);
        assert!(times(13, 97).is_empty());
        let by_13 = Problem::Div { lhs: 26, rhs: 13 };
        assert!(
            !features(&by_13)
                .iter()
                .any(|f| matches!(f, Feature::DividedBy(_)))
        );
    }

    /// Each trick, with questions that invite it and near misses that do
    /// not.
    #[test]
    fn tricks_are_recognised() {
        let add = |lhs, rhs| Problem::Add { lhs, rhs };
        let sub = |lhs, rhs| Problem::Sub { lhs, rhs };
        let mul = |lhs, rhs| Problem::Mul { lhs, rhs };
        let long = |lhs, rhs| Problem::LongMul { lhs, rhs };
        let square = |base| Problem::Square { base };
        let cases = [
            (
                Trick::AddNearRound,
                vec![add(47, 29), add(38, 5)],
                vec![add(47, 21), add(9, 8)],
            ),
            (
                Trick::MakeTen,
                vec![add(37, 43), add(6, 14)],
                vec![add(30, 40), add(37, 44)],
            ),
            (
                Trick::SubtractNearRound,
                vec![sub(83, 29)],
                vec![sub(89, 23), sub(17, 9)],
            ),
            (
                Trick::FromRound,
                vec![sub(80, 26), sub(100, 37)],
                vec![sub(80, 20), sub(81, 26)],
            ),
            (
                Trick::MultiplyNearRound,
                vec![mul(7, 39), long(28, 43)],
                vec![mul(7, 31), mul(7, 9)],
            ),
            (
                Trick::RoundFactor,
                vec![mul(7, 40), mul(10, 3)],
                vec![mul(7, 41)],
            ),
            (
                Trick::DoubleAndHalve,
                vec![mul(16, 35), mul(25, 4)],
                vec![mul(7, 35), mul(5, 6), mul(20, 35)],
            ),
            (
                Trick::DifferenceOfSquares,
                vec![long(47, 53), long(13, 17), long(98, 102)],
                vec![
                    long(50, 50),
                    long(47, 54),
                    long(40, 60),
                    mul(3, 7),
                    long(46, 48),
                ],
            ),
            (
                Trick::SquareEndingInFive,
                vec![square(35), square(15)],
                vec![square(5), square(36)],
            ),
        ];
        assert_eq!(cases.each_ref().map(|(trick, ..)| *trick), Trick::ALL);
        for (trick, yes, no) in cases {
            for p in yes {
                assert!(tricks(&p).contains(&trick), "{p} should be {trick:?}");
                assert!(features(&p).contains(&Feature::Trick(trick)));
            }
            for p in no {
                assert!(!tricks(&p).contains(&trick), "{p} should not be {trick:?}");
            }
        }
    }

    #[test]
    fn sizes_count_digits() {
        let size = |p| Size::of(&p);
        assert_eq!(size(Problem::Add { lhs: 7, rhs: 100 }), Some(Size(3, 1)));
        assert_eq!(size(Problem::Mul { lhs: 12, rhs: 97 }), Some(Size(2, 2)));
        assert_eq!(size(Problem::Sub { lhs: 105, rhs: 38 }), Some(Size(3, 2)));
        // A division is as hard as its answer is long.
        assert_eq!(size(Problem::Div { lhs: 266, rhs: 7 }), Some(Size(2, 1)));
        assert_eq!(size(Problem::Div { lhs: 266, rhs: 0 }), None);
        assert_eq!(size(Problem::Add { lhs: 0, rhs: 0 }), Some(Size(1, 1)));
        assert_eq!(size(Problem::Square { base: 17 }), None);
    }

    /// Labels share a narrow list with a score, so they must stay short.
    #[test]
    fn names_fit_the_weak_spots_list() {
        let mut all: Vec<Feature> = Trick::ALL.into_iter().map(Feature::Trick).collect();
        all.extend([
            Feature::Fact(7, 8),
            Feature::Carries(0),
            Feature::Carries(1),
            Feature::Borrows(3),
            Feature::Times(12),
            Feature::DividedBy(12),
            Feature::Denominator(20),
            Feature::Divisor(13),
        ]);
        for feature in all {
            let name = feature.to_string();
            assert!(name.chars().count() <= 24, "{name}");
        }
        assert_eq!(Feature::Carries(1).to_string(), "+ with 1 carry");
        assert_eq!(Feature::Borrows(0).to_string(), "− with no borrows");
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
        let p = Problem::Divisible { n: 91, d: 7 };
        assert_eq!(features(&p)[1], Feature::Divisor(7));
        assert_eq!(features(&Problem::Square { base: 17 }).len(), 1);
    }
}
