//! User configuration, read from `config.toml`.
//!
//! Every field has a default, so a config file only needs the settings it
//! changes. Unknown keys are errors, so a typo cannot be silently ignored.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};

use crate::generate::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub session: Session,
    pub problems: Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Session {
    /// Length of a session, in seconds.
    pub duration: u64,
    pub submit: Submit,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            duration: 120,
            submit: Submit::Auto,
        }
    }
}

/// How an answer is handed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Submit {
    /// As soon as what is typed is correct, without pressing Enter. A wrong
    /// answer is never final, only slow; Enter still submits one on purpose.
    #[default]
    Auto,
    /// Only on Enter, so a wrong answer counts as wrong.
    Enter,
}

impl Submit {
    pub fn name(self) -> &'static str {
        match self {
            Submit::Auto => "auto",
            Submit::Enter => "enter",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Submit::Auto => Submit::Enter,
            Submit::Enter => Submit::Auto,
        }
    }
}

impl std::str::FromStr for Submit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        [Submit::Auto, Submit::Enter]
            .into_iter()
            .find(|m| m.name() == s)
            .ok_or_else(|| format!("unknown submit mode `{s}`: expected auto or enter"))
    }
}

impl Session {
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.duration)
    }
}

impl Config {
    /// A commented config file holding the defaults, for `anzan config init`.
    pub const TEMPLATE: &str = include_str!("default_config.toml");

    /// Reads the config at `path`, or returns the defaults if there is no
    /// file there.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).with_context(|| format!("in {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.session.duration == 0 {
            bail!("`session.duration` must be at least 1 second");
        }
        self.problems.validate().map_err(anyhow::Error::msg)
    }
}

/// `$ANZAN_CONFIG` if set, otherwise `$XDG_CONFIG_HOME/anzan/config.toml`.
/// XDG paths are used on macOS too, as most command-line tools do.
pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("ANZAN_CONFIG") {
        return Ok(path.into());
    }
    let dirs = etcetera::choose_base_strategy().context("finding the home directory")?;
    Ok(dirs.config_dir().join("anzan").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate::Range, problem::Op};

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn partial_override_keeps_other_defaults() {
        let config = Config::parse(
            "
            [session]
            duration = 60

            [problems]
            ops = ['add', 'sub']

            [problems.add_sub]
            lhs = [10, 99]
            ",
        )
        .unwrap();
        let defaults = Config::default();
        assert_eq!(config.session.duration, 60);
        assert_eq!(config.problems.ops, [Op::Add, Op::Sub]);
        assert_eq!(config.problems.add_sub.lhs, Range::new(10, 99));
        assert_eq!(config.problems.add_sub.rhs, defaults.problems.add_sub.rhs);
        assert_eq!(config.problems.mul_div, defaults.problems.mul_div);
    }

    #[test]
    fn rejects_typos() {
        let err = Config::parse("[session]\nduraton = 60").unwrap_err();
        assert!(err.to_string().contains("duraton"), "{err}");
    }

    #[test]
    fn rejects_empty_ranges() {
        let err = Config::parse("[problems.add_sub]\nlhs = [100, 2]").unwrap_err();
        assert!(err.to_string().contains("empty"), "{err}");
    }

    #[test]
    fn rejects_impossible_settings() {
        assert!(Config::parse("[problems]\nops = []").is_err());
        assert!(Config::parse("[session]\nduration = 0").is_err());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let config = Config::load(Path::new("/nonexistent/anzan/config.toml")).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn template_holds_the_defaults() {
        assert_eq!(Config::parse(Config::TEMPLATE).unwrap(), Config::default());
    }

    #[test]
    fn round_trips_through_toml() {
        let text = toml::to_string(&Config::default()).unwrap();
        assert_eq!(Config::parse(&text).unwrap(), Config::default());
    }
}
