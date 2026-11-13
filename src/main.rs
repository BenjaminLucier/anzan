use std::{fs, io, path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use anzan::{
    app::App,
    cli::{Cli, Command, ConfigCommand, Table},
    config::{self, Config},
    export,
    history::History,
    store::{self, Store},
    theme::{self, AUTO, Question, Theme},
    ui::{self, QuestionSize},
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
        Some(Command::Themes) => run_themes(&config_path),
        None => {
            let mut config = Config::load(&config_path)?;
            cli.session.apply(&mut config);
            config.validate().context("in command-line flags")?;
            practice(config, &config_path, cli.session.is_set())
        }
    }
}

/// Opens the menu, or goes straight into a session if `start` is set.
fn practice(config: Config, config_path: &Path, start: bool) -> Result<()> {
    let data_dir = store::data_dir()?;
    let store = Store::new(data_dir.clone());
    let now = Instant::now();
    let history = History::load(&store)?;
    let config_note = if config_path.exists() {
        String::new()
    } else {
        " (none yet: anzan config init)".to_owned()
    };
    let paths = vec![
        ("config", format!("{}{config_note}", tilde(config_path))),
        ("themes", tilde(&config::themes_dir(config_path))),
        ("history", tilde(&data_dir)),
    ];
    let (themes, current) = themes(&config, config_path)?;
    let question = config.theme.question;
    let mut app = App::new(config, rand::make_rng(), now, Timestamp::now())
        .with_history(history)
        .with_themes(themes, current)
        .with_paths(paths);
    if start {
        app = app.start_now(now);
    }
    ratatui::run(|terminal| {
        let size = question_size(question);
        // Asking the terminal leaves a mark on screen: wipe it.
        terminal.clear()?;
        run(terminal, app, &store, size)
    })
}

/// Redraws after every key, and on each whole second while the clock runs.
/// Saves whatever the app records as soon as it is recorded, and deletes
/// the history when the app says to.
fn run(
    terminal: &mut DefaultTerminal,
    mut app: App,
    store: &Store,
    size: QuestionSize,
) -> Result<()> {
    // A scaled question is not part of the frame: it is painted over it.
    let mut painter = ui::Painter::default();
    while !app.quit {
        app.tick(Instant::now());
        save(&mut app, store)?;
        let mut question = None;
        terminal.draw(|frame| question = ui::render_at(&app, frame, size))?;
        if size == QuestionSize::Scaled {
            painter.paint(question, terminal.size()?, &mut io::stdout())?;
        }
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

/// How to write the question in this terminal. Call in raw mode, since
/// it may ask the terminal what it can do.
fn question_size(wanted: Question) -> QuestionSize {
    // Ghostty draws block characters itself, octants among them,
    // whatever the font. So does Kitty, but it can scale its font.
    let draws_octants = || {
        let is = |name: &str, value: &str| std::env::var(name).is_ok_and(|v| v.contains(value));
        is("TERM_PROGRAM", "ghostty") || is("TERM", "ghostty")
    };
    match wanted {
        Question::Plain => QuestionSize::Plain,
        _ if ui::probe() => QuestionSize::Scaled,
        Question::Large => QuestionSize::Drawn,
        Question::Auto if draws_octants() => QuestionSize::Drawn,
        Question::Auto => QuestionSize::Plain,
    }
}

fn save(app: &mut App, store: &Store) -> Result<()> {
    app.drain_changes()
        .try_for_each(|change| store.apply(&change))
}

/// `path` with the home directory written as `~`, to keep it short.
fn tilde(path: &Path) -> String {
    match etcetera::home_dir()
        .ok()
        .and_then(|home| path.strip_prefix(home).ok().map(Path::to_owned))
    {
        Some(rest) => Path::new("~").join(rest).display().to_string(),
        None => path.display().to_string(),
    }
}

/// Every theme, and which one the config chooses. Asks the terminal for
/// its background only when the choice depends on it.
fn themes(config: &Config, config_path: &Path) -> Result<(Vec<Theme>, usize)> {
    let truecolor =
        std::env::var("COLORTERM").is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"));
    let themes: Vec<Theme> = theme::catalog(Some(&config::themes_dir(config_path)))
        .into_iter()
        .map(|t| t.with_truecolor(truecolor))
        .collect();
    let is_light = if config.theme.name == AUTO {
        use terminal_colorsaurus::{QueryOptions, ThemeMode, theme_mode};
        theme_mode(QueryOptions::default())
            .ok()
            .map(|mode| mode == ThemeMode::Light)
    } else {
        None
    };
    let current = theme::choose(&themes, &config.theme, is_light)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("in the [theme] section of {}", config_path.display()))?;
    Ok((themes, current))
}

fn run_themes(config_path: &Path) -> Result<()> {
    let dir = config::themes_dir(config_path);
    for theme in theme::catalog(Some(&dir)) {
        let kind = match theme.is_light() {
            None => "the terminal's own colours",
            Some(true) => "light",
            Some(false) => "dark",
        };
        println!("{:<20} {kind}", theme.name);
    }
    println!();
    println!("Add base16 or base24 .yaml schemes to {}", dir.display());
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
