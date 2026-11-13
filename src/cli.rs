//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::{
    config::{Config, Submit},
    drill::Mode,
    problem::Kind,
};

#[derive(Debug, Parser)]
#[command(version, about = "暗算 anzan: a terminal trainer for mental math")]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub session: SessionArgs,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect or create the config file.
    #[command(subcommand)]
    Config(ConfigCommand),
    /// Export practice history as CSV.
    Export {
        /// Which history to export.
        #[arg(value_enum, default_value_t = Table::Attempts)]
        table: Table,
        /// Write to this file instead of standard output.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Table {
    /// One row per question answered.
    Attempts,
    /// One row per session.
    Sessions,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print where the config file is read from.
    Path,
    /// Write a commented config file holding the defaults.
    Init {
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
    },
    /// Print the config in effect, defaults included.
    Show,
}

/// Overrides for one run. They change nothing on disk, and stay in effect
/// when returning to the menu.
#[derive(Debug, Default, Args)]
pub struct SessionArgs {
    /// Session length in seconds.
    #[arg(short, long, value_name = "SECONDS")]
    pub duration: Option<u64>,

    /// Kinds of problem to ask, comma separated, such as add,sub.
    #[arg(short, long, value_delimiter = ',', value_name = "KINDS")]
    pub kinds: Option<Vec<Kind>>,

    /// How answers are handed in: auto (as soon as correct) or enter.
    #[arg(long, value_name = "MODE")]
    pub submit: Option<Submit>,

    /// How problems are chosen: jiyu, kaizen, keiko, fukushu, oni, rakusho
    /// or custom.
    #[arg(short, long, value_name = "MODE")]
    pub mode: Option<Mode>,
}

impl SessionArgs {
    /// Whether any flag was given, in which case a session starts straight
    /// away instead of opening the menu.
    pub fn is_set(&self) -> bool {
        self.duration.is_some()
            || self.kinds.is_some()
            || self.submit.is_some()
            || self.mode.is_some()
    }

    /// Applies these overrides on top of `config`.
    pub fn apply(&self, config: &mut Config) {
        if let Some(duration) = self.duration {
            config.session.duration = duration;
        }
        if let Some(kinds) = &self.kinds {
            config.problems.kinds.clone_from(kinds);
        }
        if let Some(submit) = self.submit {
            config.session.submit = submit;
        }
        if let Some(mode) = self.mode {
            config.session.mode = mode;
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("anzan").chain(args.iter().copied()))
    }

    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn flags_override_config() {
        let cli = parse(&[
            "--duration",
            "60",
            "--kinds",
            "add,sub",
            "--submit",
            "enter",
            "--mode",
            "oni",
        ])
        .unwrap();
        let mut config = Config::default();
        cli.session.apply(&mut config);
        assert_eq!(config.session.duration, 60);
        assert_eq!(config.problems.kinds, [Kind::Add, Kind::Sub]);
        assert_eq!(config.session.submit, Submit::Enter);
        assert_eq!(config.session.mode, Mode::Oni);
        assert_eq!(config.problems.add_sub, Config::default().problems.add_sub);
        assert!(cli.session.is_set());
        assert!(!parse(&[]).unwrap().session.is_set());
    }

    #[test]
    fn rejects_unknown_kinds() {
        let err = parse(&["--kinds", "add,pow"]).unwrap_err();
        assert!(err.to_string().contains("pow"), "{err}");
    }

    #[test]
    fn export_defaults_to_attempts_on_stdout() {
        let cli = parse(&["export"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Export {
                table: Table::Attempts,
                output: None
            })
        ));
    }

    #[test]
    fn session_flags_do_not_mix_with_subcommands() {
        assert!(parse(&["--duration", "60", "config", "path"]).is_err());
    }
}
