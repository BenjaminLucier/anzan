//! Problems: what is asked, and its answer.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// An arithmetic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Op {
    pub const ALL: [Op; 4] = [Op::Add, Op::Sub, Op::Mul, Op::Div];

    /// The name used in config files, history and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Op::Add => "add",
            Op::Sub => "sub",
            Op::Mul => "mul",
            Op::Div => "div",
        }
    }

    pub fn symbol(self) -> char {
        match self {
            Op::Add => '+',
            Op::Sub => '−',
            Op::Mul => '×',
            Op::Div => '÷',
        }
    }
}

impl FromStr for Op {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(Op::Add),
            "sub" => Ok(Op::Sub),
            "mul" => Ok(Op::Mul),
            "div" => Ok(Op::Div),
            _ => Err(format!(
                "unknown operation `{s}`: expected add, sub, mul or div"
            )),
        }
    }
}

/// A single question, such as `47 + 38`.
///
/// Operands are stored as numbers rather than as display text. Statistics
/// derive features like carries and digit pairs from them later.
///
/// In stored history the operation is called `kind`, which leaves room for
/// problems that are not arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Problem {
    #[serde(rename = "kind")]
    pub op: Op,
    pub lhs: i64,
    pub rhs: i64,
}

impl Problem {
    /// The answer. Generators only produce divisions that come out exact.
    pub fn answer(&self) -> i64 {
        match self.op {
            Op::Add => self.lhs + self.rhs,
            Op::Sub => self.lhs - self.rhs,
            Op::Mul => self.lhs * self.rhs,
            Op::Div => self.lhs / self.rhs,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.op.symbol(), self.rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(op: Op, lhs: i64, rhs: i64) -> Problem {
        Problem { op, lhs, rhs }
    }

    #[test]
    fn answers() {
        assert_eq!(p(Op::Add, 47, 38).answer(), 85);
        assert_eq!(p(Op::Sub, 85, 38).answer(), 47);
        assert_eq!(p(Op::Mul, 7, 38).answer(), 266);
        assert_eq!(p(Op::Div, 266, 7).answer(), 38);
    }

    #[test]
    fn op_names_match_config() {
        for op in Op::ALL {
            let name = toml::Value::try_from(op).unwrap();
            assert_eq!(name.as_str(), Some(op.name()));
            assert_eq!(op.name().parse(), Ok(op));
        }
    }

    #[test]
    fn display() {
        assert_eq!(p(Op::Sub, 85, 38).to_string(), "85 − 38");
        assert_eq!(p(Op::Div, 266, 7).to_string(), "266 ÷ 7");
    }
}
