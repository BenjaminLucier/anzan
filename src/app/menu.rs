//! The menu: settings for the next session, changed in place.
//!
//! Changes apply to this run of the program only. The config file is
//! never written.

use crate::{config::Config, problem::Kind};

/// Session lengths offered, in seconds.
const DURATIONS: [u64; 7] = [30, 60, 90, 120, 180, 300, 600];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Duration,
    Submit,
    Kind(Kind),
}

impl Row {
    /// The settings rows, then a row for each kind of problem.
    pub fn all() -> impl Iterator<Item = Row> {
        [Row::Duration, Row::Submit]
            .into_iter()
            .chain(Kind::ALL.iter().map(|&k| Row::Kind(k)))
    }

    fn count() -> usize {
        2 + Kind::ALL.len()
    }
}

#[derive(Debug, Default)]
pub struct Menu {
    pub selected: usize,
}

impl Menu {
    pub fn row(&self) -> Row {
        Row::all().nth(self.selected).expect("selection in range")
    }

    pub fn up(&mut self) {
        self.selected = self.selected.checked_sub(1).unwrap_or(Row::count() - 1);
    }

    pub fn down(&mut self) {
        self.selected = (self.selected + 1) % Row::count();
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
            Row::Kind(kind) => toggle(&mut config.problems.kinds, kind),
        }
    }
}

/// Adds or removes `kind`, but never removes the last one.
fn toggle(kinds: &mut Vec<Kind>, kind: Kind) {
    if let Some(i) = kinds.iter().position(|&k| k == kind) {
        if kinds.len() > 1 {
            kinds.remove(i);
        }
    } else {
        kinds.push(kind);
        kinds.sort();
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
        assert_eq!(menu.row(), Row::Kind(*Kind::ALL.last().unwrap()));
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
    fn toggles_keep_order_and_at_least_one_kind() {
        let mut kinds = vec![Kind::Add, Kind::Div];
        toggle(&mut kinds, Kind::Sub);
        assert_eq!(kinds, [Kind::Add, Kind::Sub, Kind::Div]);
        toggle(&mut kinds, Kind::Add);
        toggle(&mut kinds, Kind::Sub);
        assert_eq!(kinds, [Kind::Div]);
        toggle(&mut kinds, Kind::Div);
        assert_eq!(kinds, [Kind::Div]);
    }
}
