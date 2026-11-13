//! Drawing the practice screen.
//!
//! Styles use only modifiers (bold, dim), never colours, so the terminal's
//! own theme shows through.

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::Block,
};

use crate::app::{App, MAX_INPUT};

/// Width of the answer box: the longest input, the cursor after it, one
/// cell of padding, and the two borders.
const INPUT_WIDTH: u16 = MAX_INPUT as u16 + 4;

pub fn render(app: &App, frame: &mut Frame) {
    let [top, body] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(frame.area());
    render_top_bar(app, frame, top);

    let [question, input] = Layout::vertical([Constraint::Length(1), Constraint::Length(3)])
        .flex(Flex::Center)
        .spacing(1)
        .areas(body);
    let text = Line::from(app.problem.to_string()).bold().centered();
    frame.render_widget(text, question);
    let input = input.centered_horizontally(Constraint::Length(INPUT_WIDTH));
    render_input(app, frame, input);
}

fn render_top_bar(app: &App, frame: &mut Frame, area: Rect) {
    let score = Line::from(format!(" Score {}", app.score));
    let quit = Line::from("Esc quit ").dim().right_aligned();
    frame.render_widget(score, area);
    frame.render_widget(quit, area);
}

/// The answer box, with the terminal cursor just after what has been typed.
fn render_input(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().border_style(Style::new().dim());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Line::from(app.input.as_str()).centered(), inner);

    // Input is ASCII, so its length in bytes is its width in cells.
    let width = app.input.len() as u16;
    let x = inner.x + inner.width.saturating_sub(width) / 2 + width;
    frame.set_cursor_position(Position::new(x, inner.y));
}
