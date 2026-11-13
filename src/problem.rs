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
    Sqrt,
    Divisible,
    Prime,
    Factorize,
    Convert,
    PercentOf,
    PercentChange,
    Successive,
    #[serde(rename = "bps")]
    BasisPoints,
    Compound,
    #[serde(rename = "rule_of_72")]
    Doubling,
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
        Kind::Sqrt,
        Kind::Divisible,
        Kind::Prime,
        Kind::Factorize,
        Kind::Convert,
        Kind::PercentOf,
        Kind::PercentChange,
        Kind::Successive,
        Kind::BasisPoints,
        Kind::Compound,
        Kind::Doubling,
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
            Kind::PercentOf => "percent_of",
            Kind::PercentChange => "percent_change",
            Kind::Successive => "successive",
            Kind::Sqrt => "sqrt",
            Kind::BasisPoints => "bps",
            Kind::Compound => "compound",
            Kind::Doubling => "rule_of_72",
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
            Kind::Convert => "Conversions",
            Kind::PercentOf => "Percent of",
            Kind::PercentChange => "Percent change",
            Kind::Successive => "Successive changes",
            Kind::Sqrt => "Square roots",
            Kind::BasisPoints => "Basis points",
            Kind::Compound => "Compound growth",
            Kind::Doubling => "Rule of 72",
        }
    }
}

impl Kind {
    pub fn group(self) -> Group {
        match self {
            Kind::Add | Kind::Sub | Kind::Mul | Kind::Div | Kind::LongMul => Group::Arithmetic,
            Kind::Square | Kind::Cube | Kind::PowerOfTwo | Kind::Sqrt => Group::Powers,
            Kind::Divisible | Kind::Prime | Kind::Factorize => Group::NumberTheory,
            Kind::Convert | Kind::PercentOf | Kind::PercentChange | Kind::Successive => {
                Group::Percentages
            }
            Kind::BasisPoints | Kind::Compound | Kind::Doubling => Group::Finance,
        }
    }
}

/// Kinds of problem that belong together, for menus and statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    Arithmetic,
    Powers,
    NumberTheory,
    Percentages,
    Finance,
}

impl Group {
    pub const ALL: [Group; 5] = [
        Group::Arithmetic,
        Group::Powers,
        Group::NumberTheory,
        Group::Percentages,
        Group::Finance,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Arithmetic => "Arithmetic",
            Group::Powers => "Powers and roots",
            Group::NumberTheory => "Number theory",
            Group::Percentages => "Fractions and percentages",
            Group::Finance => "Finance",
        }
    }

    /// The kinds in this group, in menu order.
    pub fn kinds(self) -> impl Iterator<Item = Kind> {
        Kind::ALL.iter().copied().filter(move |k| k.group() == self)
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

/// What a basis-point question asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BpTo {
    /// 150 bp as a percentage: 1.5.
    Percent,
    /// 0.25% in basis points: 25.
    Points,
    /// 35 bp of an amount.
    Amount,
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
    /// `rate`% of `amount`.
    PercentOf {
        rate: i64,
        amount: i64,
    },
    /// The percentage change from `from` to `to`.
    PercentChange {
        from: i64,
        to: i64,
    },
    /// The net percentage change of `first`% followed by `second`%.
    Successive {
        first: i64,
        second: i64,
    },
    /// √n, to two decimal places.
    Sqrt {
        n: i64,
    },
    /// `bp` basis points, converted or applied to `amount`.
    #[serde(rename = "bps")]
    BasisPoints {
        bp: i64,
        to: BpTo,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        amount: Option<i64>,
    },
    /// `principal` grown at `rate`% a year for `years` years.
    Compound {
        principal: i64,
        rate: i64,
        years: i64,
    },
    /// Years to double at `rate`% a year, by the rule of 72.
    #[serde(rename = "rule_of_72")]
    Doubling {
        rate: i64,
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
            Problem::PercentOf { .. } => Kind::PercentOf,
            Problem::PercentChange { .. } => Kind::PercentChange,
            Problem::Successive { .. } => Kind::Successive,
            Problem::Sqrt { .. } => Kind::Sqrt,
            Problem::BasisPoints { .. } => Kind::BasisPoints,
            Problem::Compound { .. } => Kind::Compound,
            Problem::Doubling { .. } => Kind::Doubling,
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
            | Problem::Convert { .. }
            | Problem::PercentOf { .. }
            | Problem::PercentChange { .. }
            | Problem::Successive { .. }
            | Problem::Sqrt { .. }
            | Problem::BasisPoints { .. }
            | Problem::Compound { .. }
            | Problem::Doubling { .. } => None,
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
            Problem::PercentOf { rate, amount } => Answer::Exact(ratio(rate * amount, 100)),
            Problem::PercentChange { from, to } => Answer::Exact(ratio(100 * (to - from), from)),
            Problem::Successive { first, second } => {
                // (1 + a)(1 + b) − 1 = a + b + ab, in percent.
                Answer::Exact(ratio(100 * (first + second) + first * second, 100))
            }
            Problem::Sqrt { n } => Answer::Approx(approx((n as f64).sqrt()), Tolerance::Places(2)),
            Problem::BasisPoints { bp, to, amount } => match to {
                BpTo::Percent => Answer::Exact(ratio(bp, 100)),
                BpTo::Points => Answer::int(bp),
                BpTo::Amount => Answer::Exact(ratio(bp * amount.unwrap_or(0), 10_000)),
            },
            Problem::Compound {
                principal,
                rate,
                years,
            } => {
                let growth = (1.0 + rate as f64 / 100.0_f64).powi(years as i32);
                Answer::Approx(
                    approx(principal as f64 * growth),
                    Tolerance::Relative(0.005),
                )
            }
            Problem::Doubling { rate } => {
                Answer::Approx(ratio(72, rate), Tolerance::Relative(0.05))
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
            Problem::PercentOf { rate, amount } => Expr::text(format!("{rate}% of {amount}")),
            Problem::PercentChange { from, to } => Expr::text(format!("{from} → {to}")),
            Problem::Successive { first, second } => Expr::text(format!(
                "{}, {}",
                signed_percent(first),
                signed_percent(second)
            )),
            Problem::Sqrt { n } => Expr::sqrt(Expr::text(n.to_string())),
            Problem::BasisPoints { bp, to, amount } => Expr::text(match to {
                BpTo::Percent => format!("{bp} bp"),
                BpTo::Points => format!("{}%", ratio(bp, 100)),
                BpTo::Amount => format!("{bp} bp of {}", thousands(amount.unwrap_or(0))),
            }),
            Problem::Compound {
                principal,
                rate,
                years,
            } => Expr::Row(vec![
                Expr::text(format!("{} × ", thousands(principal))),
                Expr::pow(
                    Expr::text(ratio(100 + rate, 100).to_string()),
                    Expr::text(years.to_string()),
                ),
            ]),
            Problem::Doubling { rate } => Expr::text(format!("{rate}%")),
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
            Problem::PercentChange { .. } => Some("percentage change"),
            Problem::Successive { .. } => Some("net percentage change"),
            Problem::Sqrt { .. } => Some("to 2 decimal places"),
            Problem::BasisPoints { to, .. } => match to {
                BpTo::Percent => Some("as a percentage"),
                BpTo::Points => Some("in basis points"),
                BpTo::Amount => None,
            },
            Problem::Compound { .. } => Some("compound growth, within 0.5%"),
            Problem::Doubling { .. } => Some("years to double, by the rule of 72"),
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

/// `num / den`, which callers keep small enough not to overflow.
fn ratio(num: i64, den: i64) -> Rational {
    Rational::new(num.into(), den.into()).unwrap_or(Rational::ZERO)
}

/// `x` rounded to 6 decimal places, as an exact value to compare with.
fn approx(x: f64) -> Rational {
    Rational::new((x * 1e6).round() as i128, 1_000_000).unwrap_or(Rational::ZERO)
}

/// `1,000,000`.
fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("−{out}") } else { out }
}

/// `+20%` or `−10%`.
fn signed_percent(p: i64) -> String {
    if p < 0 {
        format!("−{}%", -p)
    } else {
        format!("+{p}%")
    }
}

/// The question on one line, as stored in history.
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Divisible { n, d } => write!(f, "Is {n} divisible by {d}?"),
            Problem::IsPrime { n } => write!(f, "Is {n} prime?"),
            Problem::Factorize { n } => write!(f, "Prime factors of {n}"),
            Problem::Convert { to, .. } => write!(f, "{} as {}", self.expr(), to.name()),
            Problem::PercentChange { from, to } => write!(f, "% change from {from} to {to}"),
            Problem::Successive { .. } => write!(f, "Net % change of {}", self.expr()),
            Problem::BasisPoints { to, .. } => match to {
                BpTo::Percent => write!(f, "{} as a percentage", self.expr()),
                BpTo::Points => write!(f, "{} in basis points", self.expr()),
                BpTo::Amount => write!(f, "{}", self.expr()),
            },
            Problem::Doubling { rate } => write!(f, "Years to double at {rate}%"),
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
    fn groups_list_kinds_in_menu_order() {
        let grouped: Vec<Kind> = Group::ALL.iter().flat_map(|g| g.kinds()).collect();
        assert_eq!(grouped, Kind::ALL);
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
    fn percentages() {
        let p = Problem::PercentOf {
            rate: 8,
            amount: 50,
        };
        assert_eq!(p.to_string(), "8% of 50");
        assert_eq!(p.answer().to_string(), "4");
        let p = Problem::PercentOf {
            rate: 15,
            amount: 37,
        };
        assert_eq!(p.answer().to_string(), "5.55");
        let p = Problem::PercentChange { from: 40, to: 30 };
        assert_eq!(p.to_string(), "% change from 40 to 30");
        assert!(p.answer().check("-25") && p.answer().check("-25%"));
        let p = Problem::Successive {
            first: 20,
            second: -10,
        };
        assert_eq!(p.to_string(), "Net % change of +20%, −10%");
        assert_eq!(p.answer().to_string(), "8");
        let p = Problem::Successive {
            first: 15,
            second: 15,
        };
        assert_eq!(p.answer().to_string(), "32.25");
    }

    #[test]
    fn estimation_and_finance() {
        let p = Problem::Sqrt { n: 50 };
        assert_eq!(p.to_string(), "√50");
        assert!(p.answer().check("7.07") && p.answer().check("7.071"));
        assert!(!p.answer().check("7.1"));

        let bps = |bp, to, amount| Problem::BasisPoints { bp, to, amount };
        let p = bps(150, BpTo::Percent, None);
        assert_eq!(
            (p.to_string().as_str(), p.answer().to_string()),
            ("150 bp as a percentage", "1.5".into())
        );
        let p = bps(25, BpTo::Points, None);
        assert_eq!(
            (p.to_string().as_str(), p.answer().to_string()),
            ("0.25% in basis points", "25".into())
        );
        let p = bps(35, BpTo::Amount, Some(2_000_000));
        assert_eq!(
            (p.to_string().as_str(), p.answer().to_string()),
            ("35 bp of 2,000,000", "7000".into())
        );

        let p = Problem::Compound {
            principal: 1000,
            rate: 10,
            years: 3,
        };
        assert_eq!(p.to_string(), "1,000 × 1.1³");
        assert!(p.answer().check("1331") && p.answer().check("1330") && !p.answer().check("1300"));

        let p = Problem::Doubling { rate: 8 };
        assert_eq!(p.to_string(), "Years to double at 8%");
        assert!(p.answer().check("9") && !p.answer().check("10"));
    }

    #[test]
    fn display() {
        assert_eq!(Problem::Sub { lhs: 85, rhs: 38 }.to_string(), "85 − 38");
        assert_eq!(Problem::Div { lhs: 266, rhs: 7 }.to_string(), "266 ÷ 7");
        assert_eq!(Problem::Square { base: 17 }.to_string(), "17²");
        assert_eq!(Problem::PowerOfTwo { exp: 16 }.to_string(), "2¹⁶");
    }
}
