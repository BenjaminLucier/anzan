//! Command-line arguments.

use clap::{Args, Parser, Subcommand};

use crate::{config::Config, problem::Op};

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

/// Overrides for one run. They change nothing on disk.
#[derive(Debug, Default, Args)]
pub struct SessionArgs {
    /// Session length in seconds.
    #[arg(short, long, value_name = "SECONDS")]
    pub duration: Option<u64>,

    /// Operations to ask, comma separated: add, sub, mul, div.
    #[arg(long, value_delimiter = ',', value_name = "OPS")]
    pub ops: Option<Vec<Op>>,
}

impl SessionArgs {
    /// Applies these overrides on top of `config`.
    pub fn apply(&self, config: &mut Config) {
        if let Some(duration) = self.duration {
            config.session.duration = duration;
        }
        if let Some(ops) = &self.ops {
            config.problems.ops.clone_from(ops);
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
        let cli = parse(&["--duration", "60", "--ops", "add,sub"]).unwrap();
        let mut config = Config::default();
        cli.session.apply(&mut config);
        assert_eq!(config.session.duration, 60);
        assert_eq!(config.problems.ops, [Op::Add, Op::Sub]);
        assert_eq!(config.problems.add_sub, Config::default().problems.add_sub);
    }

    #[test]
    fn rejects_unknown_ops() {
        let err = parse(&["--ops", "add,pow"]).unwrap_err();
        assert!(err.to_string().contains("pow"), "{err}");
    }

    #[test]
    fn session_flags_do_not_mix_with_subcommands() {
        assert!(parse(&["--duration", "60", "config", "path"]).is_err());
    }
}
