//! Drawing the statistics dashboard, in the style of lazygit: lists down
//! the left, and the details of the selected row filling the right.
//!
//! Everything shown comes ready-made from the [`Report`]; nothing here
//! does any counting. Panes keep their shape from one selection to the
//! next, so moving through a list changes the numbers and not the layout.

use jiff::{Timestamp, ToSpan, Zoned, civil::Date};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::Style,
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{
        Axis, Block, BorderType, Cell, Chart, Dataset, GraphType, LegendPosition, List, ListState,
        Padding, Paragraph, Row, Sparkline, Table, Wrap,
    },
};

use std::time::Duration;

use super::{clock, count};
use crate::{
    app::{Dashboard, Side},
    history::History,
    stats::{
        features::Feature,
        report::{
            Cost, DigitGrid, Fingerprint, KindReport, Report, SessionReport, Speed, WeakSpot,
        },
    },
    store::Attempt,
    theme::Theme,
};

/// Shades from least to most, for the activity heatmap.
const SHADES: [char; 5] = ['·', '░', '▒', '▓', '█'];

/// Width of the column of lists.
const SIDE_WIDTH: u16 = 36;

pub fn render(dash: &Dashboard, history: &History, t: &Theme, frame: &mut Frame, area: Rect) {
    let [main, hints] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let hint = "←→ list · ↑↓ select · PgUp/PgDn scroll · Esc menu";
    frame.render_widget(Line::from(hint).style(t.muted()).centered(), hints);

    let report = &dash.report;
    if report.attempts == 0 {
        let text = Text::from(vec![
            Line::from("Nothing practised yet."),
            Line::from("Finish a session and come back.").style(t.muted()),
        ])
        .centered();
        frame.render_widget(text, main.centered_vertically(Constraint::Length(2)));
        return;
    }

    let side_width = SIDE_WIDTH.min(main.width / 2);
    let [side, details] =
        Layout::horizontal([Constraint::Length(side_width), Constraint::Fill(1)]).areas(main);
    render_side(dash, t, frame, side);
    match dash.focus {
        Side::Summary => overview(report, t, frame, details),
        Side::Kinds => match report.kinds.get(dash.selected(Side::Kinds)) {
            Some(kind) => kind_details(report, kind, t, frame, details),
            None => empty(t, "", frame, details),
        },
        Side::Sessions => match report.sessions.get(dash.selected(Side::Sessions)) {
            Some(session) => session_details(dash, history, session, t, frame, details),
            None => empty(t, "", frame, details),
        },
        Side::Weak => match report.weak.get(dash.selected(Side::Weak)) {
            Some(weak) => weak_details(dash, history, weak, t, frame, details),
            None => empty(
                t,
                "Nothing stands out yet. A question shows up here once it has \
                 been asked twice, and a skill once it has come up five times, \
                 if it takes you more effort than your average.",
                frame,
                details,
            ),
        },
    }
}

/// A rounded panel, bold when focused and dim otherwise.
fn panel(t: &Theme, title: &str, focused: bool) -> Block<'static> {
    let style = t.border(focused);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(style);
    if title.is_empty() {
        block
    } else {
        block.title(Line::from(format!(" {title} ")).style(style))
    }
}

/// The details panel, drawn, and the room inside it.
fn details_panel(t: &Theme, title: &str, frame: &mut Frame, area: Rect) -> Rect {
    let block = panel(t, title, false);
    let inner = block.inner(area).inner(Margin::new(1, 0));
    frame.render_widget(block, area);
    inner
}

/// A details panel holding only a note.
fn empty(t: &Theme, note: &str, frame: &mut Frame, area: Rect) {
    let inner = details_panel(t, "", frame, area);
    let note = Paragraph::new(note)
        .style(t.muted())
        .wrap(Wrap { trim: true });
    frame.render_widget(note, inner);
}

fn secs(ms: f64) -> String {
    format!("{:.1}s", ms / 1000.0)
}

/// A median time, or a dash when nothing was answered correctly, since
/// only correct answers are timed.
fn median_secs(ms: f64, correct: u32) -> String {
    if correct == 0 {
        "–".to_owned()
    } else {
        secs(ms)
    }
}

/// `1.4×`.
fn times(factor: f64) -> String {
    format!("{factor:.1}×")
}

fn local(report: &Report, ts: Timestamp) -> Zoned {
    ts.to_zoned(report.tz.clone())
}

/// The lists down the left.
fn render_side(dash: &Dashboard, t: &Theme, frame: &mut Frame, area: Rect) {
    let report = &dash.report;
    let mut lines = vec![
        Line::from(count(report.attempts, "attempt")),
        Line::from(count(report.sessions.len(), "session")),
        Line::from(format!(
            "{} day streak · {} in all",
            report.streak(),
            count(report.activity.len(), "day")
        )),
    ];
    if report.damaged > 0 {
        lines.push(
            Line::from(format!("{} skipped", count(report.damaged, "damaged line")))
                .style(t.warn()),
        );
    }
    // Every kind if there is room, but never so many that the lists
    // below are squeezed to nothing: the list scrolls instead.
    const BELOW: u16 = 2 * 5;
    let summary_height = lines.len() as u16 + 2;
    let kinds_height = (report.kinds.len() as u16 + 2)
        .min(area.height.saturating_sub(summary_height + BELOW))
        .max(3);
    let [summary, kinds, sessions, weak] = Layout::vertical([
        Constraint::Length(summary_height),
        Constraint::Length(kinds_height),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(area);

    let focused = dash.focus == Side::Summary;
    let mut summary_text = Paragraph::new(lines).block(numbered(t, Side::Summary, focused));
    if focused {
        summary_text = summary_text.style(t.heading());
    }
    frame.render_widget(summary_text, summary);

    let inner = usize::from(area.width.saturating_sub(4));
    let rows = report.kinds.iter().map(|k| {
        let stats = &k.stats;
        let numbers = format!(
            "{:.0}% {:>5}",
            stats.accuracy() * 100.0,
            median_secs(stats.median_ms, stats.correct)
        );
        spread(stats.kind.label(), &numbers, inner)
    });
    list(dash, Side::Kinds, rows.collect(), t, frame, kinds);

    let rows = report.sessions.iter().map(|s| {
        let s = &s.record;
        let when = local(report, s.start).strftime("%m-%d %H:%M").to_string();
        let score = if s.completed {
            s.score.to_string()
        } else {
            format!("({})", s.score)
        };
        spread(&format!("{when}  {}", s.mode.label()), &score, inner)
    });
    list(dash, Side::Sessions, rows.collect(), t, frame, sessions);

    let rows = report.weak.iter().map(|weak| {
        let effort = times(weak.skill.factor(report.overall));
        spread(&weak.feature.to_string(), &effort, inner)
    });
    list(dash, Side::Weak, rows.collect(), t, frame, weak);
}

fn numbered(t: &Theme, side: Side, focused: bool) -> Block<'static> {
    let title = format!("[{}] {}", side.index() + 1, side.title());
    panel(t, &title, focused).padding(Padding::horizontal(1))
}

/// `left` and `right` at either end of `width` cells, `left` cut short if
/// they do not both fit.
fn spread(left: &str, right: &str, width: usize) -> String {
    let room = width.saturating_sub(right.chars().count() + 1);
    let left: String = left.chars().take(room).collect();
    let pad = width.saturating_sub(left.chars().count() + right.chars().count());
    format!("{left}{}{right}", " ".repeat(pad))
}

/// Where a list of `len` rows showing `visible` of them starts, to keep
/// row `selected` near the middle: rows above and below it stay in view
/// until the list runs out.
fn window(selected: usize, len: usize, visible: usize) -> usize {
    selected
        .saturating_sub(visible / 2)
        .min(len.saturating_sub(visible))
}

/// A list down the side, scrolled to keep its selection in view.
fn list(dash: &Dashboard, side: Side, rows: Vec<String>, t: &Theme, frame: &mut Frame, area: Rect) {
    let focused = dash.focus == side;
    let highlight = if focused {
        t.selected()
    } else {
        Style::new().bold()
    };
    let selected = dash.selected(side);
    let offset = window(
        selected,
        rows.len(),
        usize::from(area.height.saturating_sub(2)),
    );
    let list = List::new(rows)
        .block(numbered(t, side, focused))
        .highlight_style(highlight);
    let mut state = ListState::default()
        .with_offset(offset)
        .with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Everything at a glance: the score trend, practice per day, and speed.
fn overview(report: &Report, t: &Theme, frame: &mut Frame, area: Rect) {
    let inner = details_panel(t, "Overview", frame, area);
    let [trend, _, activity_area, _, speed_area] = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    render_trend(report, t, frame, trend);
    activity(report, t, frame, activity_area);
    speed_chart(&report.speed, true, t, frame, speed_area);
}

/// What the score trend is of: sessions like the one the menu is set
/// up for, named by its settings. Scores are only comparable between
/// those. The settings are dropped from the end if they do not fit in
/// `width`, the kinds first.
fn trend_title(settings: &Fingerprint, width: usize) -> String {
    let kinds = settings.kinds();
    let kinds = match kinds.len() {
        0 => None,
        1..=4 => Some(kinds.join(", ")),
        n => Some(format!("{n} kinds")),
    };
    let mut parts = vec![
        "Score per session".to_owned(),
        clock(Duration::from_secs(settings.duration)),
        settings.mode.label().to_owned(),
        format!("{} submit", settings.submit.name()),
    ];
    parts.extend(kinds);
    while parts.len() > 1 && parts.join(" · ").chars().count() > width {
        parts.pop();
    }
    parts.join(" · ")
}

fn render_trend(report: &Report, t: &Theme, frame: &mut Frame, area: Rect) {
    let [title, spark, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .areas(area);
    let heading = trend_title(&report.settings, usize::from(title.width));
    frame.render_widget(Line::from(heading).style(t.heading()), title);
    let scores = &report.trend;
    let (lowest, best) = (
        scores.iter().min().copied().unwrap_or(0),
        scores.iter().max().copied().unwrap_or(0),
    );
    let note_text = match scores.last() {
        None => "No finished sessions with these settings yet.".to_owned(),
        Some(last) => {
            let mean = scores.iter().sum::<u32>() as f64 / scores.len() as f64;
            format!(
                "{} · last {last} · best {best} · mean {mean:.0}",
                count(scores.len(), "session")
            )
        }
    };
    // Bars rise from a little under the lowest score rather than from
    // zero, so that the differences between sessions are what shows.
    let floor = lowest.saturating_sub(((best - lowest) / 4).max(1));
    // The newest sessions that fit, one column each.
    let shown: Vec<u64> = scores
        .iter()
        .rev()
        .take(usize::from(spark.width))
        .rev()
        .map(|&s| u64::from(s - floor))
        .collect();
    frame.render_widget(Sparkline::default().data(&shown).style(t.info()), spark);
    frame.render_widget(Line::from(note_text).style(t.muted()), note);
}

/// The y-axis label for `secs`, the same width whatever the number, so
/// that the plot does not move sideways between one chart and the next.
fn axis_label(secs: f64) -> String {
    format!("{secs:>5.1}")
}

/// Response time against percentile, recent over all time. `numbers`
/// puts the median and 90th percentile in the title, for where nothing
/// else on screen says them.
fn speed_chart(speed: &Speed, numbers: bool, t: &Theme, frame: &mut Frame, area: Rect) {
    if speed.all.is_empty() || area.height < 6 {
        return;
    }
    // Cap the y axis at the 95th percentile so the slow tail does not
    // flatten everything else.
    let top = speed
        .all
        .iter()
        .chain(&speed.recent)
        .filter(|(p, _)| *p <= 95.0)
        .map(|(_, s)| *s)
        .fold(1.0, f64::max)
        .ceil();
    let clip = |curve: &[(f64, f64)]| -> Vec<(f64, f64)> {
        curve.iter().map(|&(p, s)| (p, s.min(top))).collect()
    };
    let (all, recent) = (clip(&speed.all), clip(&speed.recent));
    let mut datasets = vec![
        Dataset::default()
            .name("all time")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(t.muted())
            .data(&all),
    ];
    if !recent.is_empty() {
        datasets.push(
            Dataset::default()
                .name("last 7 days")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(t.heading())
                .data(&recent),
        );
    }
    let mut title = "Seconds per correct answer, by percentile".to_owned();
    if numbers {
        let numbers = format!("median {:.1}s · p90 {:.1}s", speed.at(50), speed.at(90));
        title = format!("{title} · {numbers}");
        if title.chars().count() > usize::from(area.width) {
            title = format!("Seconds by percentile · {numbers}");
        }
    }
    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .style(t.muted())
                .bounds([0.0, 100.0])
                .labels(["0", "25", "50", "75", "100"]),
        )
        .y_axis(
            Axis::default()
                .style(t.muted())
                .bounds([0.0, top])
                .labels([0.0, top / 2.0, top].map(axis_label)),
        )
        .legend_position(Some(LegendPosition::TopLeft));
    let [title_area, chart_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    frame.render_widget(Line::from(title).style(t.heading()), title_area);
    frame.render_widget(chart, chart_area);
}

/// Attempts per day, a column per week, oldest on the left.
fn activity(report: &Report, t: &Theme, frame: &mut Frame, area: Rect) {
    const LABEL: u16 = 4;
    let weeks = i64::from(area.width.saturating_sub(LABEL) / 2).clamp(1, 53);
    let today = report.today;
    let monday = today - i64::from(today.weekday().to_monday_zero_offset()).days();
    let start = monday - (weeks - 1).weeks();
    let max = report.activity.values().copied().max().unwrap_or(1).max(1);
    let level = |n: u32| -> char {
        if n == 0 {
            return SHADES[0];
        }
        let f = f64::from(n) / f64::from(max);
        SHADES[((f * 4.0).ceil() as usize).clamp(1, 4)]
    };

    let mut lines = vec![Line::from("Practice per day").style(t.heading())];
    // Month names over the first week of each month, where they fit.
    let mut months = vec![' '; usize::from(LABEL) + 2 * weeks as usize + 2];
    for w in 0..weeks {
        let monday: Date = start + w.weeks();
        let at = usize::from(LABEL) + 2 * w as usize;
        if monday.day() <= 7 && months[at.saturating_sub(1)] == ' ' {
            for (i, c) in month(monday.month()).chars().enumerate() {
                months[at + i] = c;
            }
        }
    }
    lines.push(Line::from(months.into_iter().collect::<String>()).style(t.muted()));
    for (d, name) in ["Mon", "", "Wed", "", "Fri", "", "Sun"].iter().enumerate() {
        let mut row = vec![Span::from(format!("{name:<4}")).style(t.muted())];
        for w in 0..weeks {
            let day: Date = start + (w * 7 + d as i64).days();
            if day > today {
                break;
            }
            let n = report.activity.get(&day).copied().unwrap_or(0);
            let cell = Span::from(format!("{} ", level(n)));
            row.push(if n == 0 {
                cell.style(t.muted())
            } else {
                cell.style(t.good())
            });
        }
        lines.push(Line::from(row));
    }
    lines.push(Line::from(format!("most in a day: {max}")).style(t.muted()));
    frame.render_widget(Paragraph::new(lines), area);
}

fn month(m: i8) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES[usize::try_from(m - 1).unwrap_or(0)]
}

/// Rows of the band under a kind's speed chart: the tallest grid, which
/// is a title, a heading, ten digits, a gap and a legend. Lists in the
/// band have a title, a note, and the rows between.
const BAND: u16 = 14;

/// The fewest rows worth giving the speed chart.
const MIN_CHART: u16 = 8;

/// Width of a grid of `digits` digits: a label, then three cells each.
fn grid_width(grid: &DigitGrid) -> u16 {
    4 + 3 * grid.digits.clone().count() as u16
}

/// One kind: its numbers, its speed, and beneath them what its skills
/// cost, its hardest questions and, where there are any, the facts
/// behind it. Every kind is laid out alike, so the chart stays where it
/// is from one kind to the next.
fn kind_details(report: &Report, kind: &KindReport, t: &Theme, frame: &mut Frame, area: Rect) {
    let stats = &kind.stats;
    let inner = details_panel(t, stats.kind.label(), frame, area);
    let mut summary = format!(
        "{} seen · {:.0}% right · median {} · p90 {}",
        stats.attempts,
        stats.accuracy() * 100.0,
        median_secs(stats.median_ms, stats.correct),
        median_secs(stats.p90_ms, stats.correct)
    );
    if let Some(recent) = stats.recent_median_ms {
        summary.push_str(&format!(" · last 7 days {}", secs(recent)));
    }
    let band_height = if inner.height >= 2 + MIN_CHART + 1 + BAND {
        BAND
    } else {
        0
    };
    let [summary_area, _, chart_area, _, band] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(u16::from(band_height > 0)),
        Constraint::Length(band_height),
    ])
    .areas(inner);
    frame.render_widget(Line::from(summary), summary_area);
    speed_chart(&kind.speed, false, t, frame, chart_area);
    if band_height == 0 {
        return;
    }

    let grid_width = kind.grid.as_ref().map_or(0, grid_width);
    let has_grid = kind.grid.is_some();
    // The grid takes the place of the hardest questions unless there is
    // room for all three.
    let wide = band.width >= 2 * 32 + grid_width + 4;
    let [costs_area, hardest_area, grid_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(u16::from(!has_grid || wide)),
        Constraint::Length(grid_width),
    ])
    .spacing(2)
    .areas(band);
    frame.render_widget(
        Text::from(costs(t, &kind.costs, costs_area.width)),
        costs_area,
    );
    if hardest_area.width > 0 {
        let lines = hardest(t, kind, report.overall, hardest_area.width);
        frame.render_widget(Text::from(lines), hardest_area);
    }
    if let Some(grid) = &kind.grid {
        frame.render_widget(Text::from(digit_grid(t, grid)), grid_area);
    }
}

/// A titled list of `rows`, as (name, numbers), under column headings
/// and over a note saying what the numbers are. Fits the band.
fn ranking(
    t: &Theme,
    title: &str,
    rows: Vec<(String, String)>,
    note: &str,
    width: u16,
) -> Vec<Line<'static>> {
    let width = usize::from(width.min(44));
    let mut lines = vec![Line::from(title.to_owned()).style(t.heading())];
    if rows.is_empty() {
        lines.push(Line::from("not enough answers yet").style(t.muted()));
        return lines;
    }
    lines.extend(
        rows.into_iter()
            .take(usize::from(BAND) - 2)
            .map(|(name, numbers)| Line::from(spread(&name, &numbers, width))),
    );
    lines.push(Line::from(note.to_owned()).style(t.muted()));
    lines
}

/// What each skill in a kind costs in time.
fn costs(t: &Theme, costs: &[Cost], width: u16) -> Vec<Line<'static>> {
    let rows = costs
        .iter()
        .map(|c| {
            (
                c.feature.to_string(),
                format!("{:.2}× {:>4}", c.factor, c.count),
            )
        })
        .collect();
    ranking(
        t,
        "What takes time",
        rows,
        "× the usual time · times right",
        width,
    )
}

/// A kind's hardest questions.
fn hardest(t: &Theme, kind: &KindReport, overall: f64, width: u16) -> Vec<Line<'static>> {
    let rows = kind
        .hardest
        .iter()
        .map(|(problem, skill)| {
            let numbers = format!("{} {:>4}", times(skill.factor(overall)), skill.count);
            (problem.to_string(), numbers)
        })
        .collect();
    ranking(
        t,
        "Hardest questions",
        rows,
        "× your average · times seen",
        width,
    )
}

/// The heat level, 0 to 4, of a skill `d` above (or below) the average
/// struggle: 2 is about average, and each step either way is a third or
/// so more or less effort.
fn level(d: f64) -> usize {
    match d {
        d if d < -0.3 => 0,
        d if d < -0.1 => 1,
        d if d < 0.1 => 2,
        d if d < 0.3 => 3,
        _ => 4,
    }
}

/// A kind's heatmap: a solid cell for each pair of digits seen, coloured
/// from easier than the kind's average to harder.
fn digit_grid(t: &Theme, grid: &DigitGrid) -> Vec<Line<'static>> {
    const CELL: &str = "██";
    let mut lines = vec![Line::from(grid.title).style(t.heading())];
    let header: String = grid.digits.clone().map(|b| format!("{b:>3}")).collect();
    lines.push(Line::from(format!("  {}{header}", grid.sign)).style(t.muted()));
    for (a, row) in grid.digits.clone().zip(&grid.cells) {
        let mut spans = vec![Span::from(format!("{a:>3} ")).style(t.muted())];
        for cell in row {
            spans.push(match cell {
                Some(skill) => Span::from(CELL).style(t.heat(level(skill.score - grid.overall))),
                None => Span::from("  "),
            });
            spans.push(Span::from(" "));
        }
        lines.push(Line::from(spans));
    }
    // A row apart, or the legend reads as one more row of the grid.
    lines.push(Line::default());
    let mut legend = vec![Span::from("  easier ").style(t.muted())];
    legend.extend((0..5).map(|level| Span::from(CELL).style(t.heat(level))));
    legend.push(Span::from(" harder").style(t.muted()));
    lines.push(Line::from(legend));
    lines
}

/// One session: its settings and every question asked.
fn session_details(
    dash: &Dashboard,
    history: &History,
    session: &SessionReport,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    let report = &dash.report;
    let record = &session.record;
    let when = local(report, record.start)
        .strftime("%a %-d %b %Y, %H:%M")
        .to_string();
    let inner = details_panel(t, &format!("Session · {when}"), frame, area);
    let status = if record.completed {
        "finished"
    } else {
        "abandoned"
    };
    let header = format!(
        "{} · {} s · {} submit · score {} · {status}",
        record.mode.label(),
        record.duration,
        record.submit.name(),
        record.score,
    );
    let [header_area, _, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    frame.render_widget(Line::from(header), header_area);
    let attempts = attempts(history, &session.attempts);
    attempt_table(&attempts, dash.scroll, false, report, t, frame, table_area);
}

/// One weak spot: what it is, how it has gone, and every attempt at it.
fn weak_details(
    dash: &Dashboard,
    history: &History,
    weak: &WeakSpot,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    let report = &dash.report;
    let inner = details_panel(t, &weak.feature.to_string(), frame, area);
    let header = format!(
        "seen {} · {:.0}% right · median {} · {} your average effort",
        weak.attempts.len(),
        weak.accuracy() * 100.0,
        median_secs(weak.median_ms, weak.correct),
        times(weak.skill.factor(report.overall)),
    );
    let about = match weak.feature {
        Feature::Item(problem) => {
            let answer = problem.answer().to_string();
            // Approximate answers carry their own `≈`.
            let equals = if answer.starts_with('≈') { "" } else { "= " };
            format!("{problem} {equals}{answer}")
        }
        feature => feature.about().unwrap_or_default().to_owned(),
    };
    // The note has its two rows whether or not it fills them, so the
    // table starts on the same row for every weak spot.
    let [header_area, about_area, _, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    frame.render_widget(Line::from(header), header_area);
    let about = Paragraph::new(about)
        .style(t.muted())
        .wrap(Wrap { trim: true });
    frame.render_widget(about, about_area);
    let attempts = attempts(history, &weak.attempts);
    attempt_table(&attempts, dash.scroll, true, report, t, frame, table_area);
}

/// The attempts at `indices` in the history. An index past the end, which
/// a report older than the history could hold, is left out.
fn attempts<'a>(history: &'a History, indices: &[usize]) -> Vec<&'a Attempt> {
    indices
        .iter()
        .filter_map(|&i| history.attempts.get(i))
        .collect()
}

/// Attempts as a table, scrolled down by `scroll` rows, with where that
/// leaves it in the bottom corner when there is more than fits. `dated`
/// adds when each was made.
fn attempt_table(
    attempts: &[&Attempt],
    scroll: usize,
    dated: bool,
    report: &Report,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    // In a narrow pane the day is enough, and the question needs the room.
    let (when_format, when_width) = if area.width >= 64 {
        ("%Y-%m-%d %H:%M", 16)
    } else {
        ("%m-%d", 5)
    };
    let skip = scroll.min(attempts.len().saturating_sub(1));
    let rows = attempts[skip..].iter().map(|a| {
        let mark = match (a.correct, a.skipped) {
            (true, _) => Cell::from("✓").style(t.good()),
            (false, true) => Cell::from("skip").style(t.warn()),
            (false, false) => Cell::from("✗").style(t.bad()),
        };
        let when = local(report, a.ts).strftime(when_format).to_string();
        let cells = [
            Cell::from(when).style(t.muted()),
            Cell::from(a.question.clone()),
            Cell::from(a.answer.clone()).style(t.muted()),
            Cell::from(a.given.clone()),
            mark,
            Cell::from(Line::from(secs(a.ms as f64)).right_aligned()),
        ];
        Row::new(cells.into_iter().skip(usize::from(!dated)))
    });
    let titles = [
        Cell::from("When"),
        Cell::from("Question"),
        Cell::from("Answer"),
        Cell::from("Given"),
        Cell::from(""),
        Cell::from(Line::from("Time").right_aligned()),
    ];
    let widths = [
        Constraint::Length(when_width),
        Constraint::Fill(2),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Length(4),
        Constraint::Length(7),
    ];
    let first = usize::from(!dated);
    let header = Row::new(titles.into_iter().skip(first)).style(t.heading());
    frame.render_widget(Table::new(rows, &widths[first..]).header(header), area);

    // Where the table has got to, on the panel's bottom border.
    let visible = usize::from(area.height.saturating_sub(1));
    if attempts.len() > visible && area.height > 0 {
        let last = (skip + visible).min(attempts.len());
        let position = format!(" {}–{last} of {} ", skip + 1, attempts.len());
        let border = Rect::new(area.x, area.bottom(), area.width, 1).intersection(frame.area());
        frame.render_widget(
            Line::from(position).style(t.muted()).right_aligned(),
            border,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_levels_are_symmetric_about_the_average() {
        assert_eq!([-1.0, -0.2, 0.0, 0.2, 2.0].map(level), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn medians_need_a_correct_answer() {
        assert_eq!(median_secs(2140.0, 3), "2.1s");
        assert_eq!(median_secs(0.0, 0), "–");
    }

    #[test]
    fn trend_title_names_the_settings_that_fit() {
        use crate::{config::Submit, drill::Mode};

        let settings = |kinds: &[&str]| {
            let problems = serde_json::json!({ "kinds": kinds });
            Fingerprint::new(120, Submit::Auto, Mode::Fukushu, &problems)
        };
        let core = settings(&["add", "sub", "mul", "div"]);
        assert_eq!(
            trend_title(&core, 80),
            "Score per session · 2:00 · Fukushū · auto submit · add, div, mul, sub"
        );
        // Too many kinds to list are counted, and what does not fit goes.
        let many = settings(&["add", "sub", "mul", "div", "square"]);
        assert!(trend_title(&many, 80).ends_with("auto submit · 5 kinds"));
        assert_eq!(trend_title(&core, 40), "Score per session · 2:00 · Fukushū");
        assert_eq!(trend_title(&core, 5), "Score per session");
    }

    #[test]
    fn spread_fills_and_truncates() {
        assert_eq!(spread("ab", "9", 6), "ab   9");
        assert_eq!(spread("abcdef", "9", 4), "ab 9");
    }

    #[test]
    fn axis_labels_are_all_one_width() {
        assert_eq!(
            [0.0, 2.5, 11.0, 5.5].map(axis_label),
            ["  0.0", "  2.5", " 11.0", "  5.5"]
        );
    }

    #[test]
    fn window_keeps_the_selection_mid_list() {
        // Everything fits: no scrolling.
        assert_eq!(window(3, 5, 10), 0);
        // Near the top, in the middle, and at the end of 100 rows.
        assert_eq!(window(2, 100, 10), 0);
        assert_eq!(window(50, 100, 10), 45);
        assert_eq!(window(99, 100, 10), 90);
        assert_eq!(window(0, 0, 0), 0);
    }
}
