use std::{
    io,
    time::{Duration, Instant},
};

use anzan::{app::App, generate::Settings, ui};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

const SESSION: Duration = Duration::from_secs(120);

fn main() -> io::Result<()> {
    let app = App::new(
        Settings::default(),
        SESSION,
        rand::make_rng(),
        Instant::now(),
    );
    ratatui::run(|terminal| run(terminal, app))
}

/// Redraws after every key, and on each whole second while the clock runs.
fn run(terminal: &mut DefaultTerminal, mut app: App) -> io::Result<()> {
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
