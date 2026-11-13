//! Problems: what is asked, and its answer.

use std::fmt;

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

    pub fn symbol(self) -> char {
        match self {
            Op::Add => '+',
            Op::Sub => '−',
            Op::Mul => '×',
            Op::Div => '÷',
        }
    }
}

/// A single question, such as `47 + 38`.
///
/// Operands are stored as numbers rather than as display text. Statistics
/// derive features like carries and digit pairs from them later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Problem {
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
    fn display() {
        assert_eq!(p(Op::Sub, 85, 38).to_string(), "85 − 38");
        assert_eq!(p(Op::Div, 266, 7).to_string(), "266 ÷ 7");
    }
}
