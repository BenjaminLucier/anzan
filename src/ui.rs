//! Drawing the screens.
//!
//! Styles use only modifiers (bold, dim), never colours, so the terminal's
//! own theme shows through.

mod bitmap;
mod dashboard;
mod font;
mod grid;
mod kanji;
mod math;
mod menu;

use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Position, Rect, Size},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::Block,
};

use self::grid::Drawing;
use crate::{
    app::{App, MAX_INPUT, Screen, Session, Summary},
    config::Submit,
};

/// Width of the answer box: the longest input, the cursor after it, one
/// cell of padding, and the two borders.
const INPUT_WIDTH: u16 = MAX_INPUT as u16 + 4;

pub fn render(app: &App, frame: &mut Frame) {
    let [top, body] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(frame.area());
    let hint = match app.screen {
        Screen::Menu(_) => "Esc quit ",
        Screen::Practice(_) => "Esc menu ",
        Screen::Summary(_) => "Enter again · d stats · Esc menu ",
        Screen::Dashboard(_) => "Esc menu ",
    };
    frame.render_widget(Line::from(hint).dim().right_aligned(), top);
    match &app.screen {
        Screen::Menu(menu) => menu::render(menu, &app.config, frame, body),
        Screen::Practice(session) => {
            render_status(session, frame, top);
            render_practice(session, frame, body);
        }
        Screen::Summary(summary) => render_summary(summary, frame, body),
        Screen::Dashboard(dash) => dashboard::render(dash, frame, body),
    }
}

/// The clock in the top left, and the score in the middle.
fn render_status(session: &Session, frame: &mut Frame, area: Rect) {
    let clock = Line::from(format!(" {}", clock(session.remaining))).bold();
    let score = Line::from(format!("Score {}", session.score)).centered();
    frame.render_widget(clock, area);
    frame.render_widget(score, area);
}

/// The question as large as fits, with the answer box beneath it, any
/// correction below that, and key hints at the bottom.
fn render_practice(session: &Session, frame: &mut Frame, area: Rect) {
    const INPUT_HEIGHT: u16 = 3;
    const BELOW: u16 = 2;
    const SPACING: u16 = 1;

    let [area, hints] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let hint = match session.submit {
        Submit::Auto => "Tab skip",
        Submit::Enter => "Enter submit · Tab skip",
    };
    frame.render_widget(Line::from(hint).dim().centered(), hints);

    let text = session.problem.to_string();
    let grid = math::layout(&session.problem.expr()).grid;
    let question = Drawing::new(Some(grid), &text);
    let room = area.inner(Margin::new(2, 1));
    let room = Size::new(
        room.width,
        room.height
            .saturating_sub(INPUT_HEIGHT + BELOW + 2 * SPACING),
    );
    let prompt = session.problem.prompt();
    let prompt_height = u16::from(prompt.is_some());
    let room = Size::new(room.width, room.height.saturating_sub(prompt_height));
    let [prompt_area, question_area, input, below] = Layout::vertical([
        Constraint::Length(prompt_height),
        Constraint::Length(question.height(room)),
        Constraint::Length(INPUT_HEIGHT),
        Constraint::Length(BELOW),
    ])
    .flex(Flex::Center)
    .spacing(SPACING)
    .areas(area);
    if let Some(prompt) = prompt {
        frame.render_widget(Line::from(prompt).dim().centered(), prompt_area);
    }
    frame.render_widget(&question, question_area.inner(Margin::new(2, 0)));
    let input = input.centered_horizontally(Constraint::Length(INPUT_WIDTH));
    render_input(&session.input, frame, input);
    if let Some(correction) = &session.correction {
        let line = Line::from(vec![
            Span::from("✗ ").red(),
            Span::from(correction.question.as_str()).dim(),
            // Approximate answers carry their own `≈`.
            Span::from(if correction.answer.starts_with('≈') {
                " "
            } else {
                " = "
            })
            .dim(),
            Span::from(correction.answer.as_str()).bold(),
        ]);
        frame.render_widget(line.centered(), below);
    }
}

/// The answer box, with the terminal cursor just after what has been typed.
fn render_input(input: &str, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().border_style(Style::new().dim());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Line::from(input).centered(), inner);

    // Input is ASCII, so its length in bytes is its width in cells.
    let width = input.len() as u16;
    let x = inner.x + inner.width.saturating_sub(width) / 2 + width;
    frame.set_cursor_position(Position::new(x, inner.y));
}

fn render_summary(summary: &Summary, frame: &mut Frame, area: Rect) {
    let pace = match summary.time_per_answer {
        Some(t) => format!("{:.1} s per answer", t.as_secs_f64()),
        None => "no correct answers".to_owned(),
    };
    let mut misses = Vec::new();
    if summary.wrong > 0 {
        misses.push(format!("{} wrong", summary.wrong));
    }
    if summary.skipped > 0 {
        misses.push(format!("{} skipped", summary.skipped));
    }
    let best = match summary.previous_best {
        None => Line::from("first session with these settings").dim(),
        Some(best) if summary.score > best => {
            Line::from(format!("new best, up from {best}")).bold()
        }
        Some(best) => Line::from(format!("best {best}")).dim(),
    };
    let text = Text::from(vec![
        Line::from(format!("Time's up · {}", summary.mode.label())),
        Line::default(),
        Line::from(summary.score.to_string()).bold(),
        Line::from("correct"),
        Line::from(misses.join(" · ")).dim(),
        best,
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
