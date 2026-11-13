//! The menu: settings for the next session, changed in place.
//!
//! Changes apply to this run of the program only. The config file is
//! never written.

use crate::{
    config::Config,
    problem::{Group, Kind},
};

/// Session lengths offered, in seconds.
const DURATIONS: [u64; 7] = [30, 60, 90, 120, 180, 300, 600];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Mode,
    Duration,
    Submit,
    /// Turns every kind in a group on or off at once.
    Group(Group),
    Kind(Kind),
}

impl Row {
    /// The settings rows, then each group followed by its kinds.
    pub fn all() -> impl Iterator<Item = Row> {
        let kinds = Group::ALL
            .into_iter()
            .flat_map(|g| std::iter::once(Row::Group(g)).chain(g.kinds().map(Row::Kind)));
        [Row::Mode, Row::Duration, Row::Submit]
            .into_iter()
            .chain(kinds)
    }

    pub fn count() -> usize {
        Row::all().count()
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
            Row::Mode => config.session.mode = config.session.mode.step(forwards),
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
            Row::Group(group) => toggle_group(&mut config.problems.kinds, group),
            Row::Kind(kind) => toggle(&mut config.problems.kinds, kind),
        }
    }
}

/// Turns on every kind in `group`, or if they are all on already, turns
/// them all off, unless that would leave none.
fn toggle_group(kinds: &mut Vec<Kind>, group: Group) {
    if group.kinds().all(|k| kinds.contains(&k)) {
        if kinds.iter().any(|k| k.group() != group) {
            kinds.retain(|k| k.group() != group);
        }
    } else {
        kinds.extend(group.kinds());
        kinds.sort();
        kinds.dedup();
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
    use crate::{config::Submit, drill::Mode};

    /// A menu with `row` selected.
    fn on(row: Row) -> Menu {
        Menu {
            selected: Row::all().position(|r| r == row).unwrap(),
        }
    }

    #[test]
    fn moves_wrap_around() {
        let mut menu = Menu::default();
        menu.up();
        assert_eq!(menu.row(), Row::Kind(*Kind::ALL.last().unwrap()));
        menu.down();
        assert_eq!(menu.row(), Row::Mode);
    }

    #[test]
    fn duration_steps_through_presets() {
        let menu = on(Row::Duration);
        let mut config = Config::default();
        menu.change(&mut config, true);
        assert_eq!(config.session.duration, 180);
        menu.change(&mut config, false);
        menu.change(&mut config, false);
        assert_eq!(config.session.duration, 90);
    }

    #[test]
    fn duration_stops_at_the_ends() {
        let menu = on(Row::Duration);
        let mut config = Config::default();
        config.session.duration = 600;
        menu.change(&mut config, true);
        assert_eq!(config.session.duration, 600);
    }

    #[test]
    fn custom_duration_steps_to_nearest_preset() {
        let menu = on(Row::Duration);
        let mut config = Config::default();
        config.session.duration = 75;
        menu.change(&mut config, true);
        assert_eq!(config.session.duration, 90);
    }

    #[test]
    fn mode_cycles() {
        let menu = Menu::default();
        let mut config = Config::default();
        menu.change(&mut config, true);
        assert_eq!(config.session.mode, Mode::Kaizen);
        menu.change(&mut config, false);
        menu.change(&mut config, false);
        assert_eq!(config.session.mode, Mode::Custom);
    }

    #[test]
    fn submit_toggles() {
        let menu = on(Row::Submit);
        let mut config = Config::default();
        menu.change(&mut config, true);
        assert_eq!(config.session.submit, Submit::Enter);
        menu.change(&mut config, false);
        assert_eq!(config.session.submit, Submit::Auto);
    }

    #[test]
    fn group_toggles_all_its_kinds() {
        let mut kinds = Kind::CORE.to_vec();
        toggle_group(&mut kinds, Group::Powers);
        assert!(Group::Powers.kinds().all(|k| kinds.contains(&k)));
        // The core kinds are only part of Arithmetic, so the first toggle
        // completes the group and the second clears it.
        toggle_group(&mut kinds, Group::Arithmetic);
        assert!(kinds.contains(&Kind::LongMul));
        toggle_group(&mut kinds, Group::Arithmetic);
        assert!(kinds.iter().all(|k| k.group() == Group::Powers));
        // Turning off the only group left does nothing.
        toggle_group(&mut kinds, Group::Powers);
        assert!(!kinds.is_empty());
        // A partly enabled group is completed, not cleared.
        let mut kinds = vec![Kind::Add];
        toggle_group(&mut kinds, Group::Arithmetic);
        assert_eq!(kinds, Group::Arithmetic.kinds().collect::<Vec<_>>());
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
