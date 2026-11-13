//! Problems: what is asked, and its answer.

use std::fmt;

/// An arithmetic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
}

impl Op {
    pub fn symbol(self) -> char {
        match self {
            Op::Add => '+',
            Op::Sub => '−',
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
    pub fn answer(&self) -> i64 {
        match self.op {
            Op::Add => self.lhs + self.rhs,
            Op::Sub => self.lhs - self.rhs,
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

    #[test]
    fn answers() {
        let p = |op, lhs, rhs| Problem { op, lhs, rhs };
        assert_eq!(p(Op::Add, 47, 38).answer(), 85);
        assert_eq!(p(Op::Sub, 85, 38).answer(), 47);
    }

    #[test]
    fn display() {
        let p = Problem {
            op: Op::Sub,
            lhs: 85,
            rhs: 38,
        };
        assert_eq!(p.to_string(), "85 − 38");
    }
}
