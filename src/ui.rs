//! Drawing the screens.
//!
//! Styles use only modifiers (bold, dim), never colours, so the terminal's
//! own theme shows through.

mod bitmap;
mod font;

use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Position, Rect, Size},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::Block,
};

use self::bitmap::BigText;
use crate::app::{App, MAX_INPUT, Screen};

/// Width of the answer box: the longest input, the cursor after it, one
/// cell of padding, and the two borders.
const INPUT_WIDTH: u16 = MAX_INPUT as u16 + 4;

pub fn render(app: &App, frame: &mut Frame) {
    let [top, body] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(frame.area());
    render_top_bar(app, frame, top);
    match app.screen {
        Screen::Practice => render_practice(app, frame, body),
        Screen::Summary => render_summary(app, frame, body),
    }
}

fn render_top_bar(app: &App, frame: &mut Frame, area: Rect) {
    let clock = Line::from(format!(" {}", clock(app.remaining))).bold();
    let score = Line::from(format!("Score {}", app.score)).centered();
    let quit = Line::from("Esc quit ").dim().right_aligned();
    frame.render_widget(clock, area);
    frame.render_widget(score, area);
    frame.render_widget(quit, area);
}

/// The question as large as fits, with the answer box beneath it.
fn render_practice(app: &App, frame: &mut Frame, area: Rect) {
    const INPUT_HEIGHT: u16 = 3;
    const SPACING: u16 = 1;

    let text = app.problem.to_string();
    let question = BigText::new(&text);
    let room = area.inner(Margin::new(2, 1));
    let room = Size::new(
        room.width,
        room.height.saturating_sub(INPUT_HEIGHT + SPACING),
    );
    let [question_area, input] = Layout::vertical([
        Constraint::Length(question.height(room)),
        Constraint::Length(INPUT_HEIGHT),
    ])
    .flex(Flex::Center)
    .spacing(SPACING)
    .areas(area);
    frame.render_widget(&question, question_area.inner(Margin::new(2, 0)));
    let input = input.centered_horizontally(Constraint::Length(INPUT_WIDTH));
    render_input(app, frame, input);
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

fn render_summary(app: &App, frame: &mut Frame, area: Rect) {
    let pace = match app.time_per_answer() {
        Some(t) => format!("{:.1} s per answer", t.as_secs_f64()),
        None => "no answers".to_owned(),
    };
    let text = Text::from(vec![
        Line::from("Time's up"),
        Line::default(),
        Line::from(app.score.to_string()).bold(),
        Line::from("correct"),
        Line::default(),
        Line::from(pace).dim(),
        Line::default(),
        Line::from("Enter play again").dim(),
    ])
    .centered();
    let area = area.centered_vertically(Constraint::Length(text.height() as u16));
    frame.render_widget(text, area);
}

/// Formats time left as `m:ss`, rounding up so that the clock reads `0:00`
/// only once time is actually up.
fn clock(remaining: Duration) -> String {
    let secs = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_rounds_up() {
        let t = Duration::from_secs_f64;
        assert_eq!(clock(t(120.0)), "2:00");
        assert_eq!(clock(t(119.4)), "2:00");
        assert_eq!(clock(t(119.0)), "1:59");
        assert_eq!(clock(t(59.0)), "0:59");
        assert_eq!(clock(t(0.2)), "0:01");
        assert_eq!(clock(t(0.0)), "0:00");
    }
}
