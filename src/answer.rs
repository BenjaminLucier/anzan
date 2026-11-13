//! Expected answers, and checking what was typed against them.

use std::fmt;

use crate::{expr::superscript, rational::Rational};

#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// Exactly this number, written any way: `0.125`, `1/8` and `.125` all
    /// match 1/8.
    Exact(Rational),
    /// A number close enough to this one.
    Approx(Rational, Tolerance),
    YesNo(bool),
    /// Prime factors, repeated by multiplicity, in ascending order. Matches
    /// `2 2 2 3`, `2*2*2*3`, `2^3 × 3` and so on, in any order.
    Factors(Vec<u64>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tolerance {
    /// Within one unit of the last of this many decimal places, so 0.1428
    /// and 0.1429 both match 1/7 to 4 places.
    Places(u32),
    /// Within this fraction of the true value: 0.01 is 1%.
    Relative(f64),
}

impl Answer {
    pub fn int(n: i64) -> Self {
        Self::Exact(Rational::int(n))
    }

    /// Whether `input` is a correct answer.
    pub fn check(&self, input: &str) -> bool {
        match self {
            Answer::Exact(value) => parse_number(input) == Some(*value),
            Answer::Approx(value, tolerance) => {
                parse_number(input).is_some_and(|x| tolerance.accepts(*value, x))
            }
            Answer::YesNo(value) => parse_yes_no(input) == Some(*value),
            Answer::Factors(factors) => parse_factors(input).is_some_and(|mut given| {
                given.sort_unstable();
                given == *factors
            }),
        }
    }

    /// Whether `input` is a complete answer even if it is wrong, so that
    /// submitting on each keystroke can move on without Enter. Only a
    /// yes/no answer can tell: any other wrong input might be the start of
    /// a right one.
    pub fn is_final(&self, input: &str) -> bool {
        matches!(self, Answer::YesNo(_)) && parse_yes_no(input).is_some()
    }
}

impl Tolerance {
    fn accepts(self, value: Rational, x: Rational) -> bool {
        let diff = (x.to_f64() - value.to_f64()).abs();
        match self {
            Tolerance::Places(n) => diff < 10f64.powi(-(n as i32)),
            Tolerance::Relative(r) => diff <= r * value.to_f64().abs(),
        }
    }
}

/// The answer as it would be written: `85`, `1/7`, `≈ 7.07`, `yes`,
/// `2³ × 3`.
impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Answer::Exact(value) => write!(f, "{value}"),
            Answer::Approx(value, Tolerance::Places(n)) => {
                write!(f, "≈ {}", value.to_decimal(*n))
            }
            Answer::Approx(value, Tolerance::Relative(_)) => {
                write!(f, "≈ {}", value.to_decimal(2))
            }
            Answer::YesNo(value) => f.write_str(if *value { "yes" } else { "no" }),
            Answer::Factors(factors) => {
                let mut first = true;
                for group in factors.chunk_by(|a, b| a == b) {
                    if !first {
                        f.write_str(" × ")?;
                    }
                    first = false;
                    write!(f, "{}", group[0])?;
                    if group.len() > 1 {
                        let exp = group.len().to_string();
                        f.write_str(&superscript(&exp).unwrap_or(exp))?;
                    }
                }
                Ok(())
            }
        }
    }
}

/// A number, allowing a leading `$` and a trailing `%`, which are only
/// decoration: the question says what unit it wants.
fn parse_number(input: &str) -> Option<Rational> {
    let s = input.trim();
    let s = s.strip_prefix('$').unwrap_or(s);
    let s = s.strip_suffix('%').unwrap_or(s);
    s.parse().ok()
}

fn parse_yes_no(input: &str) -> Option<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        _ => None,
    }
}

/// Factors separated by spaces, commas, `*`, `x` or `×`, each optionally
/// raised to a power with `^`.
fn parse_factors(input: &str) -> Option<Vec<u64>> {
    let mut factors = Vec::new();
    let tokens = input
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '*' | 'x' | '×' | '·'))
        .filter(|t| !t.is_empty());
    for token in tokens {
        let (base, exp) = token.split_once('^').unwrap_or((token, "1"));
        let base: u64 = base.parse().ok()?;
        let exp: usize = exp.parse().ok()?;
        if exp == 0 || exp > 64 {
            return None;
        }
        factors.extend(std::iter::repeat_n(base, exp));
    }
    (!factors.is_empty()).then_some(factors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(num: i128, den: i128) -> Rational {
        Rational::new(num, den).unwrap()
    }

    #[test]
    fn exact_accepts_any_spelling() {
        let eighth = Answer::Exact(r(1, 8));
        for input in ["0.125", ".125", "1/8", "2/16", " 0.125 ", "0.1250"] {
            assert!(eighth.check(input), "{input:?}");
        }
        for input in ["0.12", "0.13", "", "1/9", "eighth"] {
            assert!(!eighth.check(input), "{input:?}");
        }
        assert!(Answer::int(85).check("85"));
        assert!(Answer::int(-3).check("-3"));
        assert!(Answer::int(1000).check("1,000"));
    }

    #[test]
    fn units_are_decoration() {
        assert!(Answer::Exact(r(25, 2)).check("12.5%"));
        assert!(Answer::int(1210).check("$1210"));
    }

    #[test]
    fn approx_to_decimal_places() {
        let seventh = Answer::Approx(r(1, 7), Tolerance::Places(4));
        for input in ["0.1428", "0.1429", "0.142857", "1/7"] {
            assert!(seventh.check(input), "{input:?}");
        }
        for input in ["0.143", "0.14", "0.1427", "0.1430"] {
            assert!(!seventh.check(input), "{input:?}");
        }
    }

    #[test]
    fn approx_relative() {
        // √50 ≈ 7.0711
        let root = Answer::Approx(r(70711, 10000), Tolerance::Relative(0.005));
        assert!(root.check("7.07"));
        assert!(root.check("7.1"));
        assert!(!root.check("7"));
        assert!(!root.check("7.2"));
    }

    #[test]
    fn yes_no() {
        let yes = Answer::YesNo(true);
        assert!(yes.check("y") && yes.check("Yes") && !yes.check("n"));
        assert!(yes.is_final("n"));
        assert!(!yes.is_final("ye"));
        assert!(!Answer::int(4).is_final("5"));
    }

    #[test]
    fn factors_in_any_notation() {
        let f = Answer::Factors(vec![2, 2, 2, 3]);
        for input in [
            "2 2 2 3", "3 2 2 2", "2*2*2*3", "2x2x2x3", "2^3 3", "2^3*3", "2^3 × 3", "2,2,2,3",
        ] {
            assert!(f.check(input), "{input:?}");
        }
        for input in ["2 2 3", "2^3", "24", "2^3 3 1", "2^ 3", ""] {
            assert!(!f.check(input), "{input:?}");
        }
    }

    #[test]
    fn display() {
        assert_eq!(Answer::int(85).to_string(), "85");
        assert_eq!(Answer::Exact(r(1, 7)).to_string(), "1/7");
        assert_eq!(
            Answer::Approx(r(1, 7), Tolerance::Places(4)).to_string(),
            "≈ 0.1429"
        );
        assert_eq!(Answer::YesNo(false).to_string(), "no");
        assert_eq!(
            Answer::Factors(vec![2, 2, 2, 3, 5]).to_string(),
            "2³ × 3 × 5"
        );
    }
}
