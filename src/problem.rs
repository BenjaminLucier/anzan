//! Problems: what is asked, and its answer.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::answer::Answer;

/// A kind of problem. Sessions choose among the kinds enabled in the
/// config, and statistics group by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Add,
    Sub,
    Mul,
    Div,
}

impl Kind {
    /// Every kind, in the order menus list them.
    pub const ALL: &[Kind] = &[Kind::Add, Kind::Sub, Kind::Mul, Kind::Div];

    /// The name used in config files, history and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Add => "add",
            Kind::Sub => "sub",
            Kind::Mul => "mul",
            Kind::Div => "div",
        }
    }

    /// The name shown in menus and statistics.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Add => "Addition",
            Kind::Sub => "Subtraction",
            Kind::Mul => "Multiplication",
            Kind::Div => "Division",
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
    Add { lhs: i64, rhs: i64 },
    Sub { lhs: i64, rhs: i64 },
    Mul { lhs: i64, rhs: i64 },
    Div { lhs: i64, rhs: i64 },
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
        }
    }

    /// The operator and operands, if this is a plain arithmetic problem.
    pub fn arithmetic(&self) -> Option<(Op, i64, i64)> {
        match *self {
            Problem::Add { lhs, rhs } => Some((Op::Add, lhs, rhs)),
            Problem::Sub { lhs, rhs } => Some((Op::Sub, lhs, rhs)),
            Problem::Mul { lhs, rhs } => Some((Op::Mul, lhs, rhs)),
            Problem::Div { lhs, rhs } => Some((Op::Div, lhs, rhs)),
        }
    }

    /// What counts as a correct answer.
    pub fn answer(&self) -> Answer {
        match self.arithmetic() {
            Some((op, lhs, rhs)) => Answer::int(op.apply(lhs, rhs)),
            None => unreachable!("every problem is arithmetic so far"),
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.arithmetic() {
            Some((op, lhs, rhs)) => write!(f, "{lhs} {} {rhs}", op.symbol()),
            None => unreachable!("every problem is arithmetic so far"),
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
    fn display() {
        assert_eq!(Problem::Sub { lhs: 85, rhs: 38 }.to_string(), "85 − 38");
        assert_eq!(Problem::Div { lhs: 266, rhs: 7 }.to_string(), "266 ÷ 7");
    }
}
