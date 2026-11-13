//! Generators that produce problems at random.

mod arithmetic;

use rand::{Rng, seq::IndexedRandom};

pub use arithmetic::{AddSub, MulDiv};

use crate::problem::{Op, Problem};

/// Everything that decides which problems a session asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The operations to ask, each with equal odds.
    pub ops: Vec<Op>,
    pub add_sub: AddSub,
    pub mul_div: MulDiv,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ops: Op::ALL.to_vec(),
            add_sub: AddSub::default(),
            mul_div: MulDiv::default(),
        }
    }
}

impl Settings {
    /// # Panics
    ///
    /// Panics if `ops` is empty, or if a range it needs is empty.
    pub fn generate(&self, rng: &mut impl Rng) -> Problem {
        let op = *self.ops.choose(rng).expect("at least one operation");
        match op {
            Op::Add => self.add_sub.add(rng),
            Op::Sub => self.add_sub.sub(rng),
            Op::Mul => self.mul_div.mul(rng),
            Op::Div => self.mul_div.div(rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    fn ops_seen(settings: &Settings) -> HashSet<Op> {
        let mut rng = StdRng::seed_from_u64(0);
        (0..200).map(|_| settings.generate(&mut rng).op).collect()
    }

    #[test]
    fn mixes_every_operation() {
        assert_eq!(ops_seen(&Settings::default()), HashSet::from(Op::ALL));
    }

    #[test]
    fn asks_only_chosen_operations() {
        let settings = Settings {
            ops: vec![Op::Add, Op::Sub],
            ..Settings::default()
        };
        assert_eq!(ops_seen(&settings), HashSet::from([Op::Add, Op::Sub]));
    }
}
