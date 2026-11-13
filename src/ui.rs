//! Drawing the screens.
//!
//! Every style comes from the theme in effect, which may be the terminal's
//! own colours.

mod bitmap;
mod dashboard;
mod grid;
mod kanji;
mod math;
mod menu;

use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Position, Rect, Size},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};

use self::grid::Drawing;
use crate::{
    app::{App, MAX_INPUT, Screen, Session, Summary},
    config::Submit,
    theme::Theme,
};

/// Width of the answer box: the longest input, the cursor after it, one
/// cell of padding, and the two borders.
const INPUT_WIDTH: u16 = MAX_INPUT as u16 + 4;

pub fn render(app: &App, frame: &mut Frame) {
    let t = app.theme();
    frame.render_widget(Block::new().style(t.base()), frame.area());
    let [top, body] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(frame.area());
    let hint = match app.screen {
        Screen::Menu(_) => "? help · Esc quit ",
        Screen::Practice(_) => "Esc menu ",
        Screen::Summary(_) => "Enter again · d stats · Esc menu ",
        Screen::Dashboard(_) => "? help · Esc menu ",
    };
    frame.render_widget(Line::from(hint).style(t.muted()).right_aligned(), top);
    match &app.screen {
        Screen::Menu(menu) => menu::render(menu, &app.config, t, frame, body),
        Screen::Practice(session) => {
            render_status(session, t, frame, top);
            render_practice(session, t, frame, body);
        }
        Screen::Summary(summary) => render_summary(summary, t, frame, body),
        Screen::Dashboard(dash) => dashboard::render(dash, app.history(), t, frame, body),
    }
    if app.help {
        render_help(&app.screen, &app.paths, t, frame);
    }
}

/// Keys for the menu, as (keys, what they do).
const MENU_KEYS: &[(&str, &str)] = &[
    ("← → h l Tab", "switch panel"),
    ("1–8", "jump to a panel"),
    ("↑ ↓ j k", "move; in Mode, choose the mode"),
    ("Space", "toggle a kind, or change a setting"),
    ("a", "toggle every kind in the group"),
    ("t", "next colour theme"),
    ("r", "undo every change made here"),
    ("Enter", "start a session"),
    ("d", "open the statistics"),
    ("?", "this help"),
    ("Esc q", "quit"),
];

/// Keys for the dashboard.
const DASHBOARD_KEYS: &[(&str, &str)] = &[
    ("← → h l Tab", "switch list"),
    ("1–4", "jump to a list"),
    ("↑ ↓ j k", "select"),
    ("PgDn PgUp J K", "scroll the details"),
    ("t", "next colour theme"),
    ("?", "this help"),
    ("Esc q", "back to the menu"),
];

/// Keys while practising, shown with the others for reference.
const PRACTICE_KEYS: &[(&str, &str)] = &[
    ("Enter", "submit (auto mode submits a right answer itself)"),
    ("Tab", "skip, showing the answer"),
    ("Esc", "end the session early"),
];

/// A heading, then each key and what it does.
fn help_section(
    t: &Theme,
    name: &str,
    keys: &[(&'static str, &'static str)],
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(name.to_owned()).style(t.heading())];
    lines.extend(keys.iter().map(|&(k, what)| {
        Line::from(vec![
            Span::from(format!("  {k:<14}")).style(t.accent()),
            Span::from(what),
        ])
    }));
    lines
}

/// A box over the screen listing its keys.
fn render_help(screen: &Screen, paths: &[(&str, String)], t: &Theme, frame: &mut Frame) {
    let (title, keys): (&str, &[(&str, &str)]) = match screen {
        Screen::Dashboard(_) => ("Statistics", DASHBOARD_KEYS),
        _ => ("Menu", MENU_KEYS),
    };
    let mut lines = help_section(t, title, keys);
    lines.push(Line::default());
    lines.extend(help_section(t, "Practice", PRACTICE_KEYS));
    lines.push(Line::default());
    if !paths.is_empty() {
        lines.push(Line::from("Files").style(t.heading()));
        lines.extend(paths.iter().map(|(what, path)| {
            Line::from(vec![
                Span::from(format!("  {what:<14}")).style(t.accent()),
                Span::from(path.clone()).style(t.info()),
            ])
        }));
        lines.push(Line::default());
    }
    lines.push(Line::from("Esc or ? to close").style(t.muted()));
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 4;
    let height = lines.len() as u16 + 2;
    let area = frame
        .area()
        .centered(Constraint::Length(width), Constraint::Length(height));
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(t.border(true))
        .style(t.base())
        .title(Line::from(" Keys ").style(t.border(true)))
        .padding(Padding::horizontal(1));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// The clock in the top left, and the score in the middle.
fn render_status(session: &Session, t: &Theme, frame: &mut Frame, area: Rect) {
    // The clock turns to a warning for the last ten seconds.
    let urgent = session.remaining <= Duration::from_secs(10);
    let style = if urgent { t.bad() } else { t.heading() };
    let clock = Line::from(format!(" {}", clock(session.remaining))).style(style.bold());
    let score = Line::from(vec![
        Span::from("Score ").style(t.muted()),
        Span::from(session.score.to_string()).style(t.good().bold()),
    ])
    .centered();
    frame.render_widget(clock, area);
    frame.render_widget(score, area);
}

/// The question as large as fits, with the answer box beneath it, any
/// correction below that, and key hints at the bottom.
fn render_practice(session: &Session, t: &Theme, frame: &mut Frame, area: Rect) {
    const INPUT_HEIGHT: u16 = 3;
    const BELOW: u16 = 2;
    const SPACING: u16 = 1;

    let [area, hints] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let hint = match session.submit {
        Submit::Auto => "Tab skip",
        Submit::Enter => "Enter submit · Tab skip",
    };
    frame.render_widget(Line::from(hint).style(t.muted()).centered(), hints);

    let text = session.problem.to_string();
    let grid = math::layout(&session.problem.expr()).grid;
    let question = Drawing::new(Some(grid), &text).style(t.base().bold());
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
        frame.render_widget(Line::from(prompt).style(t.muted()).centered(), prompt_area);
    }
    frame.render_widget(&question, question_area.inner(Margin::new(2, 0)));
    let input = input.centered_horizontally(Constraint::Length(INPUT_WIDTH));
    render_input(&session.input, t, frame, input);
    if let Some(correction) = &session.correction {
        let line = Line::from(vec![
            Span::from("✗ ").style(t.bad()),
            Span::from(correction.question.as_str()).style(t.muted()),
            // Approximate answers carry their own `≈`.
            Span::from(if correction.answer.starts_with('≈') {
                " "
            } else {
                " = "
            })
            .style(t.muted()),
            Span::from(correction.answer.as_str()).style(t.good().bold()),
        ]);
        frame.render_widget(line.centered(), below);
    }
}

/// The answer box, with the terminal cursor just after what has been typed.
fn render_input(input: &str, t: &Theme, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(t.border(true));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Line::from(input).centered(), inner);

    // Input is ASCII, so its length in bytes is its width in cells.
    let width = input.len() as u16;
    let x = inner.x + inner.width.saturating_sub(width) / 2 + width;
    frame.set_cursor_position(Position::new(x, inner.y));
}

fn render_summary(summary: &Summary, t: &Theme, frame: &mut Frame, area: Rect) {
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
        None => Line::from("first session with these settings").style(t.muted()),
        Some(best) if summary.score > best => {
            Line::from(format!("new best, up from {best}")).style(t.warn().bold())
        }
        Some(best) => Line::from(format!("best {best}")).style(t.muted()),
    };
    let text = Text::from(vec![
        Line::from(format!("Time's up · {}", summary.mode.label())).style(t.heading()),
        Line::default(),
        Line::from(summary.score.to_string()).style(t.good().bold()),
        Line::from("correct"),
        Line::from(misses.join(" · ")).style(t.bad()),
        best,
        Line::default(),
        Line::from(pace).style(t.muted()),
        Line::default(),
        Line::from("Enter play again").style(t.muted()),
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
