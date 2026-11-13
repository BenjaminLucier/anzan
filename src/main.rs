use std::{fs, path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use anzan::{
    app::App,
    cli::{Cli, Command, ConfigCommand},
    config::{self, Config},
    ui,
};
use clap::Parser;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = config::config_path()?;
    match cli.command {
        Some(Command::Config(command)) => run_config(command, &config_path),
        None => {
            let mut config = Config::load(&config_path)?;
            cli.session.apply(&mut config);
            config.validate().context("in command-line flags")?;
            practice(config)
        }
    }
}

fn practice(config: Config) -> Result<()> {
    let app = App::new(
        config.problems,
        config.session.duration(),
        rand::make_rng(),
        Instant::now(),
    );
    ratatui::run(|terminal| run(terminal, app))?;
    Ok(())
}

/// Redraws after every key, and on each whole second while the clock runs.
fn run(terminal: &mut DefaultTerminal, mut app: App) -> std::io::Result<()> {
    while !app.quit {
        app.tick(Instant::now());
        terminal.draw(|frame| ui::render(&app, frame))?;
        let ready = match app.until_next_tick() {
            Some(timeout) => event::poll(timeout)?,
            None => true,
        };
        if ready && let Event::Key(key) = event::read()? {
            app.handle_key(key, Instant::now());
        }
    }
    Ok(())
}

fn run_config(command: ConfigCommand, path: &Path) -> Result<()> {
    match command {
        ConfigCommand::Path => println!("{}", path.display()),
        ConfigCommand::Init { force } => {
            if path.exists() && !force {
                bail!(
                    "{} already exists; pass --force to overwrite it",
                    path.display()
                );
            }
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
            }
            fs::write(path, Config::TEMPLATE)
                .with_context(|| format!("writing {}", path.display()))?;
            println!("wrote {}", path.display());
        }
        ConfigCommand::Show => print!("{}", toml::to_string(&Config::load(path)?)?),
    }
    Ok(())
}
