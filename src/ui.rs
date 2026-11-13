//! Drawing the screens.
//!
//! Styles use only modifiers (bold, dim), never colours, so the terminal's
//! own theme shows through.

mod bitmap;
mod font;
mod kanji;

use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Position, Rect, Size},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::Block,
};

use self::bitmap::{BigText, Scale};
use crate::{
    app::{App, MAX_INPUT, Menu, Row, Screen, Session, Summary},
    config::Config,
    problem::Op,
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
        Screen::Summary(_) => "Enter again · Esc menu ",
    };
    frame.render_widget(Line::from(hint).dim().right_aligned(), top);
    match &app.screen {
        Screen::Menu(menu) => render_menu(menu, &app.config, frame, body),
        Screen::Practice(session) => {
            render_status(session, frame, top);
            render_practice(session, frame, body);
        }
        Screen::Summary(summary) => render_summary(summary, frame, body),
    }
}

/// The name in large kanji, then the settings for the next session.
fn render_menu(menu: &Menu, config: &Config, frame: &mut Frame, area: Rect) {
    let rows: Vec<Line> = Row::ALL
        .iter()
        .enumerate()
        .map(|(i, &row)| menu_line(row, config, i == menu.selected))
        .collect();
    let rows = Text::from(rows);
    let hints = Line::from("↑↓ choose · ←→ change · Enter start")
        .dim()
        .centered();

    let title = BigText::bitmap(Some(kanji::anzan()), "暗算").scales(Scale::SQUARE);
    let fixed = 1 + 1 + rows.height() as u16 + 1 + 1;
    let room = Size::new(area.width, area.height.saturating_sub(fixed + 2));
    let [title_area, name, _, rows_area, _, hints_area] = Layout::vertical([
        Constraint::Length(title.height(room)),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(rows.height() as u16),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .flex(Flex::Center)
    .areas(area);

    frame.render_widget(&title, title_area);
    frame.render_widget(Line::from("anzan · mental math").dim().centered(), name);
    let rows_area = rows_area.centered_horizontally(Constraint::Length(MENU_WIDTH));
    frame.render_widget(rows, rows_area);
    frame.render_widget(hints, hints_area);
}

/// Width of the settings list.
const MENU_WIDTH: u16 = 32;

fn menu_line(row: Row, config: &Config, selected: bool) -> Line<'static> {
    let (label, value) = match row {
        Row::Duration => (
            "Duration",
            format!("◂ {} ▸", clock(config.session.duration())),
        ),
        Row::Op(op) => {
            let on = config.problems.ops.contains(&op);
            (op_label(op), if on { "[x]" } else { "[ ]" }.to_owned())
        }
    };
    let marker = if selected { "› " } else { "  " };
    // Everything here is one cell per character, so counting characters
    // gives the width in cells.
    let width = usize::from(MENU_WIDTH) - marker.chars().count() - label.len();
    let line = Line::from(format!("{marker}{label}{value:>width$}"));
    if selected { line.bold() } else { line }
}

fn op_label(op: Op) -> &'static str {
    match op {
        Op::Add => "Addition",
        Op::Sub => "Subtraction",
        Op::Mul => "Multiplication",
        Op::Div => "Division",
    }
}

/// The clock in the top left, and the score in the middle.
fn render_status(session: &Session, frame: &mut Frame, area: Rect) {
    let clock = Line::from(format!(" {}", clock(session.remaining))).bold();
    let score = Line::from(format!("Score {}", session.score)).centered();
    frame.render_widget(clock, area);
    frame.render_widget(score, area);
}

/// The question as large as fits, with the answer box beneath it.
fn render_practice(session: &Session, frame: &mut Frame, area: Rect) {
    const INPUT_HEIGHT: u16 = 3;
    const SPACING: u16 = 1;

    let text = session.problem.to_string();
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
    render_input(&session.input, frame, input);
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
        None => "no answers".to_owned(),
    };
    let text = Text::from(vec![
        Line::from("Time's up"),
        Line::default(),
        Line::from(summary.score.to_string()).bold(),
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
