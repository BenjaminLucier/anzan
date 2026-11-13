use std::io;

use anzan::{app::App, generate::AddSub, ui};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

fn main() -> io::Result<()> {
    let app = App::new(AddSub::default(), rand::make_rng());
    ratatui::run(|terminal| run(terminal, app))
}

fn run(terminal: &mut DefaultTerminal, mut app: App) -> io::Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::render(&app, frame))?;
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
    }
    Ok(())
}
