use std::io;

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::Constraint,
    style::Stylize,
    text::Line,
};

fn main() -> io::Result<()> {
    ratatui::run(run)
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(render)?;
        if let Event::Key(key) = event::read()?
            && key.is_press()
            && is_quit(key)
        {
            return Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    let area = frame.area().centered_vertically(Constraint::Length(1));
    frame.render_widget(Line::from("暗算 anzan").bold().centered(), area);
}

/// Esc quits. In raw mode Ctrl-C arrives as an ordinary key event, so it is
/// handled here too.
fn is_quit(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}
