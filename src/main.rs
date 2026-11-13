use std::{fs, io, path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use anzan::{
    app::{App, Record},
    cli::{Cli, Command, ConfigCommand, Table},
    config::{self, Config},
    export,
    store::{self, Store},
    ui,
};
use clap::Parser;
use jiff::Timestamp;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = config::config_path()?;
    match cli.command {
        Some(Command::Config(command)) => run_config(command, &config_path),
        Some(Command::Export { table, output }) => run_export(table, output.as_deref()),
        None => {
            let mut config = Config::load(&config_path)?;
            cli.session.apply(&mut config);
            config.validate().context("in command-line flags")?;
            practice(config)
        }
    }
}

fn practice(config: Config) -> Result<()> {
    let store = Store::new(store::data_dir()?);
    let app = App::new(config, rand::make_rng(), Instant::now(), Timestamp::now());
    ratatui::run(|terminal| run(terminal, app, &store))
}

/// Redraws after every key, and on each whole second while the clock runs.
/// Saves whatever the app records as soon as it is recorded.
fn run(terminal: &mut DefaultTerminal, mut app: App, store: &Store) -> Result<()> {
    while !app.quit {
        app.tick(Instant::now());
        save(&mut app, store)?;
        terminal.draw(|frame| ui::render(&app, frame))?;
        let ready = match app.until_next_tick() {
            Some(timeout) => event::poll(timeout)?,
            None => true,
        };
        if ready && let Event::Key(key) = event::read()? {
            app.handle_key(key, Instant::now());
        }
    }
    save(&mut app, store)
}

fn save(app: &mut App, store: &Store) -> Result<()> {
    for record in app.drain_records() {
        match record {
            Record::Attempt(attempt) => store.append_attempt(&attempt)?,
            Record::Session(session) => store.append_session(&session)?,
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

fn run_export(table: Table, output: Option<&Path>) -> Result<()> {
    let store = Store::new(store::data_dir()?);
    let out: Box<dyn io::Write> = match output {
        Some(path) => Box::new(
            fs::File::create(path).with_context(|| format!("creating {}", path.display()))?,
        ),
        None => Box::new(io::stdout().lock()),
    };
    let skipped = match table {
        Table::Attempts => {
            let loaded = store.attempts()?;
            export::attempts(&loaded.rows, out)?;
            loaded.skipped
        }
        Table::Sessions => {
            let loaded = store.sessions()?;
            export::sessions(&loaded.rows, out)?;
            loaded.skipped
        }
    };
    if skipped > 0 {
        eprintln!("skipped {skipped} damaged lines");
    }
    Ok(())
}
