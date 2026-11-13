//! The menu: settings for the next session, changed in place.
//!
//! Changes apply to this run of the program only. The config file is
//! never written.

use crate::{config::Config, problem::Op};

/// Session lengths offered, in seconds.
const DURATIONS: [u64; 7] = [30, 60, 90, 120, 180, 300, 600];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Duration,
    Submit,
    Op(Op),
}

impl Row {
    pub const ALL: [Row; 6] = [
        Row::Duration,
        Row::Submit,
        Row::Op(Op::Add),
        Row::Op(Op::Sub),
        Row::Op(Op::Mul),
        Row::Op(Op::Div),
    ];
}

#[derive(Debug, Default)]
pub struct Menu {
    pub selected: usize,
}

impl Menu {
    pub fn row(&self) -> Row {
        Row::ALL[self.selected]
    }

    pub fn up(&mut self) {
        self.selected = self.selected.checked_sub(1).unwrap_or(Row::ALL.len() - 1);
    }

    pub fn down(&mut self) {
        self.selected = (self.selected + 1) % Row::ALL.len();
    }

    /// Steps the selected setting forwards (`true`) or backwards. Toggles
    /// ignore the direction.
    pub fn change(&self, config: &mut Config, forwards: bool) {
        match self.row() {
            Row::Duration => {
                let current = config.session.duration;
                let next = if forwards {
                    DURATIONS.into_iter().find(|&d| d > current)
                } else {
                    DURATIONS.into_iter().rev().find(|&d| d < current)
                };
                config.session.duration = next.unwrap_or(current);
            }
            Row::Submit => config.session.submit = config.session.submit.toggled(),
            Row::Op(op) => toggle(&mut config.problems.ops, op),
        }
    }
}

/// Adds or removes `op`, but never removes the last one.
fn toggle(ops: &mut Vec<Op>, op: Op) {
    if let Some(i) = ops.iter().position(|&o| o == op) {
        if ops.len() > 1 {
            ops.remove(i);
        }
    } else {
        ops.push(op);
        ops.sort_by_key(|o| Op::ALL.iter().position(|a| a == o));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Submit;

    #[test]
    fn moves_wrap_around() {
        let mut menu = Menu::default();
        menu.up();
        assert_eq!(menu.row(), Row::Op(Op::Div));
        menu.down();
        assert_eq!(menu.row(), Row::Duration);
    }

    #[test]
    fn duration_steps_through_presets() {
        let menu = Menu::default();
        let mut config = Config::default();
        menu.change(&mut config, true);
        assert_eq!(config.session.duration, 180);
        menu.change(&mut config, false);
        menu.change(&mut config, false);
        assert_eq!(config.session.duration, 90);
    }

    #[test]
    fn duration_stops_at_the_ends() {
        let menu = Menu::default();
        let mut config = Config::default();
        config.session.duration = 600;
        menu.change(&mut config, true);
        assert_eq!(config.session.duration, 600);
    }

    #[test]
    fn custom_duration_steps_to_nearest_preset() {
        let menu = Menu::default();
        let mut config = Config::default();
        config.session.duration = 75;
        menu.change(&mut config, true);
        assert_eq!(config.session.duration, 90);
    }

    #[test]
    fn submit_toggles() {
        let mut menu = Menu::default();
        menu.down();
        let mut config = Config::default();
        menu.change(&mut config, true);
        assert_eq!(config.session.submit, Submit::Enter);
        menu.change(&mut config, false);
        assert_eq!(config.session.submit, Submit::Auto);
    }

    #[test]
    fn toggles_keep_order_and_at_least_one_op() {
        let mut ops = vec![Op::Add, Op::Div];
        toggle(&mut ops, Op::Sub);
        assert_eq!(ops, [Op::Add, Op::Sub, Op::Div]);
        toggle(&mut ops, Op::Add);
        toggle(&mut ops, Op::Sub);
        assert_eq!(ops, [Op::Div]);
        toggle(&mut ops, Op::Div);
        assert_eq!(ops, [Op::Div]);
    }
}
