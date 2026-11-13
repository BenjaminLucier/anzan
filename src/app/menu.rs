//! The menu: settings for the next session, changed in place.
//!
//! Changes apply to this run of the program only. The config file is
//! never written.

use crate::{
    config::Config,
    drill::Mode,
    problem::{Group, Kind},
};

/// Session lengths offered, in seconds.
const DURATIONS: [u64; 7] = [30, 60, 90, 120, 180, 300, 600];

/// A bordered panel of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Every practice mode, one of them chosen.
    Mode,
    /// Duration and submit mode.
    Session,
    /// The kinds in a group, each on or off.
    Group(Group),
}

impl Panel {
    /// In the order Tab visits them.
    pub fn all() -> impl Iterator<Item = Panel> {
        [Panel::Mode, Panel::Session]
            .into_iter()
            .chain(Group::ALL.into_iter().map(Panel::Group))
    }

    pub fn title(self) -> &'static str {
        match self {
            Panel::Mode => "Mode",
            Panel::Session => "Session",
            Panel::Group(g) => g.label(),
        }
    }

    /// The rows inside, top to bottom.
    pub fn items(self) -> Vec<Item> {
        match self {
            Panel::Mode => Mode::ALL.into_iter().map(Item::Mode).collect(),
            Panel::Session => vec![Item::Duration, Item::Submit, Item::Theme],
            Panel::Group(g) => g.kinds().map(Item::Kind).collect(),
        }
    }
}

/// One row of a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Mode(Mode),
    Duration,
    Submit,
    /// The colour theme, which the app itself cycles.
    Theme,
    Kind(Kind),
}

/// Which panel has focus, and the row selected in it.
#[derive(Debug, Default)]
pub struct Menu {
    panel: usize,
    row: usize,
}

impl Menu {
    pub fn panel(&self) -> Panel {
        Panel::all().nth(self.panel).expect("panel in range")
    }

    /// The selected row of the focused panel. In the mode panel, that is
    /// always the mode in effect.
    pub fn selected(&self, config: &Config) -> Item {
        match self.panel() {
            Panel::Mode => Item::Mode(config.session.mode),
            panel => panel.items()[self.row],
        }
    }

    /// Moves focus to the next panel (`true`) or the previous one.
    pub fn switch(&mut self, forwards: bool) {
        let n = Panel::all().count();
        self.panel = if forwards {
            (self.panel + 1) % n
        } else {
            (self.panel + n - 1) % n
        };
        self.row = 0;
    }

    /// Focuses the panel numbered `n` from 1, if there is one.
    pub fn focus(&mut self, n: usize) {
        if (1..=Panel::all().count()).contains(&n) {
            self.panel = n - 1;
            self.row = 0;
        }
    }

    /// Moves the selection up (`true`) or down within the panel. In the
    /// mode panel, moving chooses the mode.
    pub fn step(&mut self, config: &mut Config, up: bool) {
        if self.panel() == Panel::Mode {
            let mode = config.session.mode;
            let i = Mode::ALL.iter().position(|&m| m == mode).unwrap_or(0);
            let i = if up {
                i.saturating_sub(1)
            } else {
                (i + 1).min(Mode::ALL.len() - 1)
            };
            config.session.mode = Mode::ALL[i];
            return;
        }
        let last = self.panel().items().len() - 1;
        self.row = if up {
            self.row.saturating_sub(1)
        } else {
            (self.row + 1).min(last)
        };
    }

    /// Toggles or steps the selected row.
    pub fn activate(&self, config: &mut Config) {
        match self.selected(config) {
            Item::Mode(_) | Item::Theme => {}
            Item::Duration => {
                let current = config.session.duration;
                config.session.duration = DURATIONS
                    .into_iter()
                    .find(|&d| d > current)
                    .unwrap_or(DURATIONS[0]);
            }
            Item::Submit => config.session.submit = config.session.submit.toggled(),
            Item::Kind(kind) => toggle(&mut config.problems.kinds, kind),
        }
    }

    /// Turns the whole focused group on or off.
    pub fn toggle_group(&self, config: &mut Config) {
        if let Panel::Group(group) = self.panel() {
            toggle_group(&mut config.problems.kinds, group);
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
    use crate::config::Submit;

    fn on(panel: Panel) -> Menu {
        Menu {
            panel: Panel::all().position(|p| p == panel).unwrap(),
            row: 0,
        }
    }

    #[test]
    fn panels_cycle() {
        let mut menu = Menu::default();
        assert_eq!(menu.panel(), Panel::Mode);
        menu.switch(false);
        assert_eq!(menu.panel(), Panel::Group(Group::Probability));
        menu.switch(true);
        menu.switch(true);
        assert_eq!(menu.panel(), Panel::Session);
        menu.focus(3);
        assert_eq!(menu.panel(), Panel::Group(Group::Arithmetic));
        menu.focus(99);
        assert_eq!(menu.panel(), Panel::Group(Group::Arithmetic));
    }

    #[test]
    fn moving_in_the_mode_panel_chooses_the_mode() {
        let mut menu = Menu::default();
        let mut config = Config::default();
        menu.step(&mut config, false);
        assert_eq!(config.session.mode, Mode::Kaizen);
        assert_eq!(menu.selected(&config), Item::Mode(Mode::Kaizen));
        menu.step(&mut config, true);
        menu.step(&mut config, true);
        assert_eq!(config.session.mode, Mode::Jiyu);
    }

    #[test]
    fn duration_cycles_through_presets() {
        let menu = on(Panel::Session);
        let mut config = Config::default();
        menu.activate(&mut config);
        assert_eq!(config.session.duration, 180);
        config.session.duration = 600;
        menu.activate(&mut config);
        assert_eq!(config.session.duration, 30);
        // A custom length steps to the next preset.
        config.session.duration = 75;
        menu.activate(&mut config);
        assert_eq!(config.session.duration, 90);
    }

    #[test]
    fn submit_toggles() {
        let mut menu = on(Panel::Session);
        let mut config = Config::default();
        menu.step(&mut config, false);
        assert_eq!(menu.selected(&config), Item::Submit);
        menu.step(&mut config, false);
        assert_eq!(menu.selected(&config), Item::Theme);
        menu.step(&mut config, false);
        assert_eq!(menu.selected(&config), Item::Theme, "stops at the end");
        menu.step(&mut config, true);
        menu.activate(&mut config);
        assert_eq!(config.session.submit, Submit::Enter);
    }

    #[test]
    fn kinds_toggle_singly_or_by_group() {
        let mut menu = on(Panel::Group(Group::Arithmetic));
        let mut config = Config::default();
        menu.step(&mut config, false);
        assert_eq!(menu.selected(&config), Item::Kind(Kind::Sub));
        menu.activate(&mut config);
        assert!(!config.problems.kinds.contains(&Kind::Sub));
        menu.toggle_group(&mut config);
        assert!(
            Group::Arithmetic
                .kinds()
                .all(|k| config.problems.kinds.contains(&k))
        );
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
