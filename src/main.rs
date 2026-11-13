use std::time::Instant;

use anyhow::Result;
use anzan::{app::App, config::Config, ui};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

fn main() -> Result<()> {
    let config = Config::load(&anzan::config::config_path()?)?;
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
