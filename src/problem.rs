//! Problems: what is asked, and its answer.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    answer::{Answer, Tolerance},
    expr::Expr,
    number::{is_prime, prime_factors},
    rational::Rational,
};

/// A kind of problem. Sessions choose among the kinds enabled in the
/// config, and statistics group by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Add,
    Sub,
    Mul,
    Div,
    LongMul,
    Square,
    Cube,
    PowerOfTwo,
    Divisible,
    Prime,
    Factorize,
    Convert,
}

impl Kind {
    /// Every kind, in the order menus list them.
    pub const ALL: &[Kind] = &[
        Kind::Add,
        Kind::Sub,
        Kind::Mul,
        Kind::Div,
        Kind::LongMul,
        Kind::Square,
        Kind::Cube,
        Kind::PowerOfTwo,
        Kind::Divisible,
        Kind::Prime,
        Kind::Factorize,
        Kind::Convert,
    ];

    /// The kinds a session asks unless told otherwise: the four basic
    /// operations.
    pub const CORE: &[Kind] = &[Kind::Add, Kind::Sub, Kind::Mul, Kind::Div];

    /// The name used in config files, history and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Add => "add",
            Kind::Sub => "sub",
            Kind::Mul => "mul",
            Kind::Div => "div",
            Kind::LongMul => "long_mul",
            Kind::Square => "square",
            Kind::Cube => "cube",
            Kind::PowerOfTwo => "power_of_two",
            Kind::Divisible => "divisible",
            Kind::Prime => "prime",
            Kind::Factorize => "factorize",
            Kind::Convert => "convert",
        }
    }

    /// The name shown in menus and statistics.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Add => "Addition",
            Kind::Sub => "Subtraction",
            Kind::Mul => "Multiplication",
            Kind::Div => "Division",
            Kind::LongMul => "Long multiplication",
            Kind::Square => "Squares",
            Kind::Cube => "Cubes",
            Kind::PowerOfTwo => "Powers of 2",
            Kind::Divisible => "Divisibility",
            Kind::Prime => "Primes",
            Kind::Factorize => "Prime factors",
            Kind::Convert => "Fractions ↔ decimals ↔ %",
        }
    }
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Kind::ALL
            .iter()
            .copied()
            .find(|k| k.name() == s)
            .ok_or_else(|| {
                let names: Vec<_> = Kind::ALL.iter().map(|k| k.name()).collect();
                format!("unknown kind `{s}`: expected one of {}", names.join(", "))
            })
    }
}

/// A way of writing a number between 0 and 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    Fraction,
    Decimal,
    Percent,
}

impl Form {
    fn name(self) -> &'static str {
        match self {
            Form::Fraction => "a fraction",
            Form::Decimal => "a decimal",
            Form::Percent => "a percentage",
        }
    }
}

/// Decimal places wanted when an answer does not terminate: 1/7 ≈ 0.1429.
pub const REPEATING_PLACES: u32 = 4;

/// An arithmetic operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Op {
    pub fn symbol(self) -> char {
        match self {
            Op::Add => '+',
            Op::Sub => '−',
            Op::Mul => '×',
            Op::Div => '÷',
        }
    }

    /// `lhs op rhs`. Division truncates; generators only ask exact ones.
    pub fn apply(self, lhs: i64, rhs: i64) -> i64 {
        match self {
            Op::Add => lhs + rhs,
            Op::Sub => lhs - rhs,
            Op::Mul => lhs * rhs,
            Op::Div => lhs / rhs,
        }
    }
}

/// A single question, such as `47 + 38`.
///
/// Operands are stored as numbers rather than as display text. Statistics
/// derive features like carries and digit pairs from them. In stored
/// history each problem is tagged with its `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Problem {
    Add {
        lhs: i64,
        rhs: i64,
    },
    Sub {
        lhs: i64,
        rhs: i64,
    },
    Mul {
        lhs: i64,
        rhs: i64,
    },
    Div {
        lhs: i64,
        rhs: i64,
    },
    LongMul {
        lhs: i64,
        rhs: i64,
    },
    Square {
        base: i64,
    },
    Cube {
        base: i64,
    },
    PowerOfTwo {
        exp: i64,
    },
    /// Is `n` divisible by `d`?
    Divisible {
        n: i64,
        d: i64,
    },
    /// Is `n` prime?
    #[serde(rename = "prime")]
    IsPrime {
        n: i64,
    },
    /// The prime factors of `n`.
    Factorize {
        n: i64,
    },
    /// `num/den`, shown in one form and asked for in another.
    Convert {
        num: i64,
        den: i64,
        from: Form,
        to: Form,
    },
}

impl Problem {
    pub fn binary(op: Op, lhs: i64, rhs: i64) -> Self {
        match op {
            Op::Add => Problem::Add { lhs, rhs },
            Op::Sub => Problem::Sub { lhs, rhs },
            Op::Mul => Problem::Mul { lhs, rhs },
            Op::Div => Problem::Div { lhs, rhs },
        }
    }

    pub fn kind(&self) -> Kind {
        match self {
            Problem::Add { .. } => Kind::Add,
            Problem::Sub { .. } => Kind::Sub,
            Problem::Mul { .. } => Kind::Mul,
            Problem::Div { .. } => Kind::Div,
            Problem::LongMul { .. } => Kind::LongMul,
            Problem::Square { .. } => Kind::Square,
            Problem::Cube { .. } => Kind::Cube,
            Problem::PowerOfTwo { .. } => Kind::PowerOfTwo,
            Problem::Divisible { .. } => Kind::Divisible,
            Problem::IsPrime { .. } => Kind::Prime,
            Problem::Factorize { .. } => Kind::Factorize,
            Problem::Convert { .. } => Kind::Convert,
        }
    }

    /// The operator and operands, if this is a plain arithmetic problem.
    /// Long multiplication counts as multiplication.
    pub fn arithmetic(&self) -> Option<(Op, i64, i64)> {
        match *self {
            Problem::Add { lhs, rhs } => Some((Op::Add, lhs, rhs)),
            Problem::Sub { lhs, rhs } => Some((Op::Sub, lhs, rhs)),
            Problem::Mul { lhs, rhs } => Some((Op::Mul, lhs, rhs)),
            Problem::Div { lhs, rhs } => Some((Op::Div, lhs, rhs)),
            Problem::LongMul { lhs, rhs } => Some((Op::Mul, lhs, rhs)),
            Problem::Square { .. }
            | Problem::Cube { .. }
            | Problem::PowerOfTwo { .. }
            | Problem::Divisible { .. }
            | Problem::IsPrime { .. }
            | Problem::Factorize { .. }
            | Problem::Convert { .. } => None,
        }
    }

    /// What counts as a correct answer.
    pub fn answer(&self) -> Answer {
        match *self {
            Problem::Add { .. }
            | Problem::Sub { .. }
            | Problem::Mul { .. }
            | Problem::Div { .. }
            | Problem::LongMul { .. } => {
                let (op, lhs, rhs) = self.arithmetic().expect("arithmetic");
                Answer::int(op.apply(lhs, rhs))
            }
            Problem::Square { base } => Answer::int(base * base),
            Problem::Cube { base } => Answer::int(base * base * base),
            Problem::PowerOfTwo { exp } => Answer::int(1 << exp),
            Problem::Divisible { n, d } => Answer::YesNo(n % d == 0),
            Problem::IsPrime { n } => Answer::YesNo(is_prime(n as u64)),
            Problem::Factorize { n } => Answer::Factors(prime_factors(n as u64)),
            Problem::Convert { to, .. } => {
                let value = self.value();
                match to {
                    Form::Fraction => Answer::Fraction(value),
                    Form::Decimal => decimal(value, REPEATING_PLACES),
                    Form::Percent => decimal(percent(value), REPEATING_PLACES - 2),
                }
            }
        }
    }
}

impl Problem {
    /// The question in mathematical notation.
    pub fn expr(&self) -> Expr {
        fn pow(base: i64, exp: i64) -> Expr {
            Expr::pow(Expr::text(base.to_string()), Expr::text(exp.to_string()))
        }
        match *self {
            Problem::Add { .. }
            | Problem::Sub { .. }
            | Problem::Mul { .. }
            | Problem::Div { .. }
            | Problem::LongMul { .. } => {
                let (op, lhs, rhs) = self.arithmetic().expect("arithmetic");
                Expr::text(format!("{lhs} {} {rhs}", op.symbol()))
            }
            Problem::Square { base } => pow(base, 2),
            Problem::Cube { base } => pow(base, 3),
            Problem::PowerOfTwo { exp } => pow(2, exp),
            Problem::Divisible { n, d } => Expr::text(format!("{n} ÷ {d}")),
            Problem::IsPrime { n } | Problem::Factorize { n } => Expr::text(n.to_string()),
            Problem::Convert { num, den, from, .. } => {
                let value = self.value();
                match from {
                    Form::Fraction => {
                        Expr::frac(Expr::text(num.to_string()), Expr::text(den.to_string()))
                    }
                    Form::Decimal => Expr::text(value.to_string()),
                    Form::Percent => Expr::text(format!("{}%", percent(value))),
                }
            }
        }
    }

    /// The number a conversion is about.
    fn value(&self) -> Rational {
        match *self {
            Problem::Convert { num, den, .. } => {
                Rational::new(num.into(), den.into()).unwrap_or(Rational::ZERO)
            }
            _ => Rational::ZERO,
        }
    }

    /// Words shown with the notation when it does not say what to do.
    pub fn prompt(&self) -> Option<&'static str> {
        match self {
            Problem::Divisible { .. } => Some("divides exactly? y/n"),
            Problem::IsPrime { .. } => Some("prime? y/n"),
            Problem::Factorize { .. } => Some("prime factors, such as 2 2 3 or 2^2 3"),
            Problem::Convert { to, .. } => Some(match to {
                Form::Fraction => "as a fraction in lowest terms",
                Form::Decimal => "as a decimal",
                Form::Percent => "as a percentage",
            }),
            _ => None,
        }
    }
}

/// `value` exactly if its decimal ends, or else to `places` places.
fn decimal(value: Rational, places: u32) -> Answer {
    if value.terminates() {
        Answer::Exact(value)
    } else {
        Answer::Approx(value, Tolerance::Places(places))
    }
}

/// `value` as a percentage: 3/8 is 37.5.
fn percent(value: Rational) -> Rational {
    value
        .checked_mul(Rational::int(100))
        .unwrap_or(Rational::ZERO)
}

/// The question on one line, as stored in history.
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Divisible { n, d } => write!(f, "Is {n} divisible by {d}?"),
            Problem::IsPrime { n } => write!(f, "Is {n} prime?"),
            Problem::Factorize { n } => write!(f, "Prime factors of {n}"),
            Problem::Convert { to, .. } => write!(f, "{} as {}", self.expr(), to.name()),
            _ => write!(f, "{}", self.expr()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers() {
        let answer = |op, lhs, rhs| Problem::binary(op, lhs, rhs).answer().to_string();
        assert_eq!(answer(Op::Add, 47, 38), "85");
        assert_eq!(answer(Op::Sub, 85, 38), "47");
        assert_eq!(answer(Op::Mul, 7, 38), "266");
        assert_eq!(answer(Op::Div, 266, 7), "38");
    }

    #[test]
    fn kind_names_match_history_and_config() {
        for &kind in Kind::ALL {
            let name = toml::Value::try_from(kind).unwrap();
            assert_eq!(name.as_str(), Some(kind.name()));
            assert_eq!(kind.name().parse(), Ok(kind));
        }
    }

    #[test]
    fn stored_tag_is_the_kind_name() {
        let p = Problem::Mul { lhs: 7, rhs: 38 };
        let json = serde_json::to_value(p).unwrap();
        assert_eq!(json["kind"], p.kind().name());
    }

    #[test]
    fn powers() {
        let answer = |p: Problem| p.answer().to_string();
        assert_eq!(answer(Problem::Square { base: 17 }), "289");
        assert_eq!(answer(Problem::Cube { base: 12 }), "1728");
        assert_eq!(answer(Problem::PowerOfTwo { exp: 20 }), "1048576");
    }

    #[test]
    fn number_theory() {
        let answer = |p: Problem| p.answer().to_string();
        assert_eq!(answer(Problem::Divisible { n: 2431, d: 11 }), "yes");
        assert_eq!(answer(Problem::Divisible { n: 2432, d: 11 }), "no");
        assert_eq!(answer(Problem::IsPrime { n: 91 }), "no");
        assert_eq!(answer(Problem::IsPrime { n: 97 }), "yes");
        assert_eq!(answer(Problem::Factorize { n: 360 }), "2³ × 3² × 5");
        assert_eq!(Problem::IsPrime { n: 91 }.to_string(), "Is 91 prime?");
    }

    #[test]
    fn conversions() {
        let convert = |num, den, from, to| Problem::Convert { num, den, from, to };
        let p = convert(1, 7, Form::Fraction, Form::Decimal);
        assert_eq!(p.to_string(), "1/7 as a decimal");
        assert_eq!(p.answer().to_string(), "≈ 0.1429");
        assert!(p.answer().check("0.142857"));
        let p = convert(1, 8, Form::Fraction, Form::Percent);
        assert!(p.answer().check("12.5") && p.answer().check("12.5%"));
        let p = convert(1, 3, Form::Fraction, Form::Percent);
        assert!(p.answer().check("33.33"));
        let p = convert(3, 8, Form::Decimal, Form::Fraction);
        assert_eq!(p.to_string(), "0.375 as a fraction");
        assert!(p.answer().check("3/8") && !p.answer().check("0.375"));
        let p = convert(1, 16, Form::Percent, Form::Decimal);
        assert_eq!(p.to_string(), "6.25% as a decimal");
        assert!(p.answer().check("0.0625"));
    }

    #[test]
    fn display() {
        assert_eq!(Problem::Sub { lhs: 85, rhs: 38 }.to_string(), "85 − 38");
        assert_eq!(Problem::Div { lhs: 266, rhs: 7 }.to_string(), "266 ÷ 7");
        assert_eq!(Problem::Square { base: 17 }.to_string(), "17²");
        assert_eq!(Problem::PowerOfTwo { exp: 16 }.to_string(), "2¹⁶");
    }
}
