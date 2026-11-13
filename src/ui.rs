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
mod scaled;

use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Margin, Position, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};

pub use self::scaled::{Painter, Scaled, probe};
use self::{grid::Drawing, scaled::SCALE};
use crate::{
    app::{App, MAX_INPUT, Overlay, Screen, Session, Summary},
    config::Submit,
    theme::Theme,
};

/// Width of the answer box: the longest input, the cursor after it, one
/// cell of padding, and the two borders.
const INPUT_WIDTH: u16 = MAX_INPUT as u16 + 4;

pub fn render(app: &App, frame: &mut Frame) {
    render_screen(app, frame, false);
}

/// As [`render`], for a terminal that passed [`probe`]. The question is
/// then left out of the frame, with room for it at twice the size, and
/// returned for a [`Painter`] to draw once the frame is on screen.
pub fn render_scaled(app: &App, frame: &mut Frame) -> Option<Scaled> {
    render_screen(app, frame, true)
}

fn render_screen(app: &App, frame: &mut Frame, scaled: bool) -> Option<Scaled> {
    let t = app.theme();
    let mut question = None;
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
            question = render_practice(session, scaled, t, frame, body);
        }
        Screen::Summary(summary) => render_summary(summary, t, frame, body),
        Screen::Dashboard(dash) => dashboard::render(dash, app.history(), t, frame, body),
    }
    match app.overlay {
        Some(Overlay::Help) => render_help(&app.screen, &app.paths, t, frame),
        Some(Overlay::ConfirmDelete) => render_confirm_delete(app, t, frame),
        None => {}
    }
    question
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
    ("X", "delete all history, after asking"),
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
    popup(" Keys ", lines, t.border(true), t, frame);
}

/// Asks before deleting the history, saying what will go.
fn render_confirm_delete(app: &App, t: &Theme, frame: &mut Frame) {
    let history = app.history();
    let mut lines = vec![
        Line::from(format!(
            "This deletes all {} and {},",
            count(history.attempts.len(), "attempt"),
            count(history.sessions.len(), "session")
        )),
        Line::from("and they cannot be brought back."),
        Line::default(),
    ];
    if let Some((_, folder)) = app.paths.iter().find(|(what, _)| *what == "history") {
        lines.push(Line::from(vec![
            Span::from("They are kept in ").style(t.muted()),
            Span::from(folder.clone()).style(t.info()),
        ]));
    }
    lines.extend([
        Line::from("`anzan export` saves a copy first.").style(t.muted()),
        Line::default(),
        Line::from(vec![
            Span::from("y").style(t.bad().bold()),
            Span::from(" delete everything · any other key keeps it").style(t.muted()),
        ]),
    ]);
    popup(" Delete history? ", lines, t.bad().bold(), t, frame);
}

/// A bordered box in the middle of the screen, just big enough for
/// `lines`.
fn popup(title: &str, lines: Vec<Line>, border: Style, t: &Theme, frame: &mut Frame) {
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 4;
    let height = lines.len() as u16 + 2;
    let area = frame
        .area()
        .centered(Constraint::Length(width), Constraint::Length(height));
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .style(t.base())
        .title(Line::from(title).style(border))
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

/// The question with the answer box beneath it, any correction below
/// that, and key hints at the bottom.
///
/// Nothing moves from one question to the next: the prompt has its row
/// whether or not there is one, and the question has the rows of the
/// tallest, with its math axis on the middle one.
///
/// With `scaled`, and room for it, the question is twice the size: it is
/// returned rather than drawn, and its rows are left blank.
fn render_practice(
    session: &Session,
    scaled: bool,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) -> Option<Scaled> {
    const INPUT_HEIGHT: u16 = 3;
    const BELOW: u16 = 2;
    const SPACING: u16 = 1;
    const PROMPT_HEIGHT: u16 = 1;

    let [area, hints] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let hint = match session.submit {
        Submit::Auto => "Tab skip",
        Submit::Enter => "Enter submit · Tab skip",
    };
    frame.render_widget(Line::from(hint).style(t.muted()).centered(), hints);

    let text = session.problem.to_string();
    let grid = math::layout(&session.problem.expr()).grid;
    let style = t.base().bold();
    // The height depends on the terminal alone, never on the question.
    // On one too short for stacked notation, every question is one line
    // of plain text.
    let fixed = PROMPT_HEIGHT + INPUT_HEIGHT + BELOW + 3 * SPACING;
    let big = scaled && area.height >= fixed + SCALE * math::MAX_HEIGHT;
    let question_height = if big {
        SCALE * math::MAX_HEIGHT
    } else if area.height >= fixed + math::MAX_HEIGHT {
        math::MAX_HEIGHT
    } else {
        1
    };
    let prompt = session.problem.prompt();
    let [prompt_area, question_area, input, below] = Layout::vertical([
        Constraint::Length(PROMPT_HEIGHT),
        Constraint::Length(question_height),
        Constraint::Length(INPUT_HEIGHT),
        Constraint::Length(BELOW),
    ])
    .flex(Flex::Center)
    .spacing(SPACING)
    .areas(area);
    if let Some(prompt) = prompt {
        frame.render_widget(Line::from(prompt).style(t.muted()).centered(), prompt_area);
    }
    let room = question_area.inner(Margin::new(2, 0));
    let (width, height) = (SCALE * grid.width() as u16, SCALE * grid.height() as u16);
    // A question too wide to double is drawn at the usual size instead.
    let question = if big && width <= room.width && height <= room.height {
        Some(Scaled {
            area: Rect::new(
                room.x + (room.width - width) / 2,
                room.y + (room.height - height) / 2,
                width,
                height,
            ),
            rows: grid.rows().collect(),
            style,
        })
    } else {
        frame.render_widget(&Drawing::new(Some(grid), &text).style(style), room);
        None
    };
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
    question
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

/// `1 session`, `2 sessions`.
fn count(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// Formats time left as `m:ss`, rounding up so that the clock reads `0:00`
/// only once time is actually up.
fn clock(remaining: Duration) -> String {
    let secs = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rand::{SeedableRng, rngs::StdRng};
    use ratatui::{Terminal, backend::TestBackend};

    use jiff::{SignedDuration, Timestamp};
    use rand::RngExt;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};

    use super::*;
    use crate::{
        config::Config,
        history::History,
        problem::Kind,
        store::{Attempt, SessionRecord},
    };

    fn wall() -> Timestamp {
        "2026-11-13T10:00:00Z".parse().unwrap()
    }

    /// Twenty sessions over as many days, in kinds whose details differ:
    /// some have a times-table grid, some skills, some neither. Kinds
    /// take very different times, so their charts have different scales.
    fn history() -> History {
        let mut config = Config::default();
        config.problems.kinds = vec![
            Kind::Add,
            Kind::Mul,
            Kind::LongMul,
            Kind::Convert,
            Kind::Dice,
        ];
        let mut rng = StdRng::seed_from_u64(1);
        let mut history = History {
            damaged: 2,
            ..History::default()
        };
        for day in (0..20).rev() {
            let start = wall() - SignedDuration::from_hours(24 * day + 1);
            let id = format!("session-{day}");
            for i in 0..30 {
                let problem = config.problems.generate(&mut rng);
                let slow = if problem.kind() == Kind::LongMul {
                    40
                } else {
                    1
                };
                history.attempts.push(Attempt {
                    ts: start + SignedDuration::from_secs(4 * i),
                    session: id.clone(),
                    problem,
                    question: problem.to_string(),
                    answer: problem.answer().to_string(),
                    given: problem.answer().to_string(),
                    correct: i % 9 != 0,
                    skipped: false,
                    submit: Submit::Auto,
                    ms: rng.random_range(700..4000) * slow,
                    edits: 0,
                });
            }
            history.sessions.push(SessionRecord {
                id,
                start,
                end: start + SignedDuration::from_secs(120),
                duration: 120,
                score: 20 + (day % 7) as u32,
                completed: day % 5 != 0,
                submit: Submit::Auto,
                mode: Default::default(),
                problems: serde_json::to_value(&Config::default().problems).unwrap(),
            });
        }
        history
    }

    /// An app on the menu, with history behind it.
    fn app() -> App {
        App::new(
            Config::default(),
            StdRng::seed_from_u64(0),
            Instant::now(),
            wall(),
        )
        .with_history(history())
    }

    fn press(app: &mut App, keys: &[KeyCode]) {
        for &key in keys {
            app.handle_key(KeyEvent::from(key), Instant::now());
        }
    }

    /// Opens the dashboard on the list numbered `list`, `row` rows down.
    fn dashboard(list: char, row: usize) -> App {
        let mut app = app();
        press(&mut app, &[KeyCode::Char('d'), KeyCode::Char(list)]);
        press(&mut app, &vec![KeyCode::Down; row]);
        app
    }

    /// Every screen, at sizes from nothing up, draws without panicking:
    /// a terminal can be resized to anything at any moment.
    #[test]
    fn every_screen_draws_at_any_size() {
        let mut help = app();
        press(&mut help, &[KeyCode::Char('?')]);
        let mut summary = practising(Kind::Add);
        summary.tick(Instant::now() + Duration::from_secs(600));
        let mut scrolled = dashboard('3', 2);
        press(&mut scrolled, &[KeyCode::PageDown; 5]);
        let mut confirm = dashboard('1', 0);
        press(&mut confirm, &[KeyCode::Char('X')]);
        let mut screens = vec![
            app(),
            help,
            confirm,
            summary,
            scrolled,
            practising(Kind::ExpectedValue),
        ];
        screens.extend(['1', '2', '3', '4'].map(|list| dashboard(list, 1)));
        // A dashboard with nothing in it.
        let mut empty = App::new(
            Config::default(),
            StdRng::seed_from_u64(0),
            Instant::now(),
            wall(),
        );
        press(&mut empty, &[KeyCode::Char('d')]);
        screens.push(empty);
        for screen in &screens {
            for width in [1, 2, 9, 20, 37, 50, 80, 120, 250] {
                for height in [1, 2, 5, 9, 16, 24, 50, 90] {
                    draw(screen, width, height);
                    draw_scaled(screen, width, height);
                }
            }
        }
    }

    /// The speed chart sits in the same place whichever kind is selected,
    /// though their times, skills and grids all differ.
    #[test]
    fn chart_stays_put_between_kinds() {
        // The corner of the axes: the last `└` on screen that starts a line.
        let corner = |screen: &[String]| -> (usize, usize) {
            let row = screen
                .iter()
                .rposition(|row| row.contains("└──"))
                .expect("a chart");
            let col = screen[row].chars().position(|c| c == '└').unwrap();
            (row, col)
        };
        for (width, height) in [(110, 42), (160, 50), (80, 24)] {
            let corners: Vec<_> = (0..5)
                .map(|row| corner(&draw(&dashboard('2', row), width, height)))
                .collect();
            assert!(
                corners.iter().all(|&c| c == corners[0]),
                "{width}×{height}: {corners:?}"
            );
        }
    }

    #[test]
    fn deleting_says_what_will_go() {
        let mut app = dashboard('1', 0).with_paths(vec![("history", "~/data/anzan".into())]);
        press(&mut app, &[KeyCode::Char('X')]);
        let screen = draw(&app, 100, 30);
        row_of(&screen, "Delete history?");
        row_of(&screen, "all 600 attempts and 20 sessions");
        row_of(&screen, "~/data/anzan");
    }

    #[test]
    fn dashboard_says_when_history_is_damaged() {
        let screen = draw(&dashboard('1', 0), 100, 30);
        row_of(&screen, "2 damaged lines skipped");
    }

    /// The weak spots list names skills as well as single questions, and
    /// its details start their table on the same row for either.
    #[test]
    fn weak_spot_details_keep_their_shape() {
        let app = dashboard('4', 0);
        let Screen::Dashboard(dash) = &app.screen else {
            panic!("not on the dashboard");
        };
        let weak = dash.report.weak.len();
        assert!(weak > 1, "the fixture should have weak spots");
        let rows: Vec<usize> = (0..weak)
            .map(|row| row_of(&draw(&dashboard('4', row), 110, 40), "When "))
            .collect();
        assert!(rows.iter().all(|&row| row == rows[0]), "{rows:?}");
    }

    /// The screen as rows of text.
    fn draw(app: &App, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(app, frame)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    fn practising(kind: Kind) -> App {
        let mut config = Config::default();
        config.problems.kinds = vec![kind];
        let now = Instant::now();
        App::new(config, StdRng::seed_from_u64(0), now, wall()).start_now(now)
    }

    /// The row a screen first shows `needle` on.
    fn row_of(screen: &[String], needle: &str) -> usize {
        screen
            .iter()
            .position(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("no {needle:?} in {screen:#?}"))
    }

    /// The screen and the question as a terminal with scaled text gets
    /// them.
    fn draw_scaled(app: &App, width: u16, height: u16) -> (Vec<String>, Option<Scaled>) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut question = None;
        terminal
            .draw(|frame| question = render_scaled(app, frame))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let screen = (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        (screen, question)
    }

    #[test]
    fn scaled_question_is_left_out_of_the_frame_with_room_for_it() {
        let app = practising(Kind::Add);
        let Screen::Practice(session) = &app.screen else {
            panic!("not practising");
        };
        let text = session.problem.to_string();
        let (screen, question) = draw_scaled(&app, 80, 24);
        let question = question.expect("room for a scaled question");
        assert_eq!(question.rows, std::slice::from_ref(&text));
        // Twice as wide and as high as its text, centred, and nothing in
        // the frame where it will go or anywhere else says the same.
        let width = 2 * text.chars().count() as u16;
        assert_eq!((question.area.width, question.area.height), (width, 2));
        assert_eq!(question.area.x, (80 - width) / 2);
        assert!(!screen.iter().any(|row| row.contains(&text)), "{screen:#?}");
        assert!(question.area.bottom() < row_of(&screen, "╭") as u16);
        // The other screens have nothing to scale.
        assert_eq!(draw_scaled(&self::app(), 80, 24).1, None);
    }

    #[test]
    fn scaled_questions_fall_back_when_there_is_no_room() {
        // Too short for double height: the question is in the frame.
        let app = practising(Kind::Add);
        let (screen, question) = draw_scaled(&app, 80, 14);
        assert_eq!(question, None);
        let Screen::Practice(session) = &app.screen else {
            panic!("not practising");
        };
        row_of(&screen, &session.problem.to_string());
        // Too narrow for this one question: the same, and nothing moves.
        let (wide, question) = draw_scaled(&practising(Kind::ExpectedValue), 40, 24);
        assert_eq!(question, None);
        let (narrow, _) = draw_scaled(&practising(Kind::Add), 40, 24);
        assert_eq!(row_of(&wide, "╭"), row_of(&narrow, "╭"));
    }

    #[test]
    fn answer_box_stays_put_whatever_the_question() {
        // One line, one line with a prompt, an overline, a stacked fraction.
        let kinds = [Kind::Add, Kind::Prime, Kind::Sqrt, Kind::ExpectedValue];
        for (width, height) in [(80, 24), (60, 12), (40, 10)] {
            let rows: Vec<usize> = kinds
                .iter()
                .map(|&kind| row_of(&draw(&practising(kind), width, height), "╭"))
                .collect();
            assert!(
                rows.iter().all(|&row| row == rows[0]),
                "{width}×{height}: {rows:?}"
            );
            // And at twice the size, with every stacked row on its feet.
            let scaled: Vec<_> = kinds
                .iter()
                .map(|&kind| draw_scaled(&practising(kind), width, height))
                .collect();
            let rows: Vec<usize> = scaled.iter().map(|(s, _)| row_of(s, "╭")).collect();
            assert!(
                rows.iter().all(|&row| row == rows[0]),
                "scaled {width}×{height}: {rows:?}"
            );
            for question in scaled.iter().filter_map(|(_, q)| q.as_ref()) {
                // The math axis is the same scaled row for all of them.
                let axis = question.area.y + question.area.height / 2;
                let first = scaled.iter().find_map(|(_, q)| q.as_ref()).unwrap();
                assert_eq!(axis, first.area.y + first.area.height / 2);
            }
        }
    }

    #[test]
    fn counts_agree_in_number() {
        assert_eq!(count(0, "session"), "0 sessions");
        assert_eq!(count(1, "session"), "1 session");
        assert_eq!(count(2, "attempt"), "2 attempts");
    }

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
