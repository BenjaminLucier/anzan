//! Drawing the statistics dashboard, in the style of lazygit: lists down
//! the left, and the details of the selected row filling the right.
//!
//! Everything shown comes ready-made from the [`Report`]; nothing here
//! does any counting. Panes keep their shape from one selection to the
//! next, so moving through a list changes the numbers and not the layout.

use jiff::{Timestamp, ToSpan, ZonedDateTime, civil::Date};
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
            Cost, DigitGrid, Fingerprint, KindReport, Report, SessionReport, Speed, WeakSpot, Week,
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

fn local(report: &Report, ts: Timestamp) -> ZonedDateTime {
    ts.to_zoned_datetime(report.tz.clone())
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
    let (speed_area, progress_area) = pair(speed_area);
    if let Some(area) = speed_area {
        speed_chart(&report.speed, true, t, frame, area);
    }
    progress_chart(&report.progress, Measure::Effort, t, frame, progress_area);
}

/// Where the speed and progress charts go: side by side, or one above the
/// other in a pane too narrow for that, or, with room for only one, the
/// progress chart alone, since it says more about how practice is going.
fn pair(area: Rect) -> (Option<Rect>, Rect) {
    const MIN_WIDTH: u16 = 36;
    /// The fewest rows for each of two stacked charts.
    const MIN_STACKED: u16 = 7;
    if area.width >= 2 * MIN_WIDTH + 2 {
        let [speed, progress] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
            .spacing(2)
            .areas(area);
        (Some(speed), progress)
    } else if area.height > 2 * MIN_STACKED {
        let [speed, _, progress] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);
        (Some(speed), progress)
    } else {
        (None, area)
    }
}

/// What a progress chart plots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Measure {
    /// Median seconds per correct answer.
    Seconds,
    /// Effort against the overall average, where 1× is average.
    Effort,
}

/// The average of `values`, or of their first or last four.
fn mean_of(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0), |(s, n), v| (s + v, n + 1));
    sum / f64::from(n.max(1))
}

/// The title of a progress chart: what it plots, and where it started and
/// has got to, each the average of four weeks so that one odd week does
/// not decide it. Parts are dropped from the end if they do not fit in
/// `width`.
fn progress_title(weeks: &[Week], measure: Measure, width: usize) -> String {
    let span = 4.min(weeks.len() / 2).max(1);
    let then = |f: fn(&Week) -> f64| mean_of(weeks.iter().take(span).map(f));
    let now = |f: fn(&Week) -> f64| mean_of(weeks.iter().rev().take(span).map(f));
    let mut parts = match measure {
        Measure::Seconds => vec![
            "Seconds per week".to_owned(),
            format!("{:.1}s to {:.1}s", then(|w| w.value), now(|w| w.value)),
            format!(
                "{:.0}% to {:.0}% right",
                then(|w| w.accuracy) * 100.0,
                now(|w| w.accuracy) * 100.0
            ),
        ],
        Measure::Effort => vec![
            "Effort per week".to_owned(),
            format!(
                "{} to {}",
                times(then(|w| w.value)),
                times(now(|w| w.value))
            ),
            "1× is your average".to_owned(),
        ],
    };
    if weeks.len() < 2 {
        parts.truncate(1);
    }
    while parts.len() > 1 && parts.join(" · ").chars().count() > width {
        parts.pop();
    }
    parts.join(" · ")
}

/// Each value averaged with its neighbours either side, to steady a line
/// drawn from only a few answers a week.
fn smooth(values: &[f64]) -> Vec<f64> {
    (0..values.len())
        .map(|i| {
            let window = &values[i.saturating_sub(1)..(i + 2).min(values.len())];
            window.iter().sum::<f64>() / window.len() as f64
        })
        .collect()
}

/// A line through `weeks`, oldest on the left: median seconds as they
/// are, or effort averaged over three weeks with a line at 1×, the
/// average, to measure against. The y axis stops a little above most
/// weeks, so that a rare bad week does not flatten the rest.
fn progress_chart(weeks: &[Week], measure: Measure, t: &Theme, frame: &mut Frame, area: Rect) {
    if area.height < 6 {
        return;
    }
    let [title_area, chart_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    let title = progress_title(weeks, measure, usize::from(title_area.width));
    frame.render_widget(Line::from(title).style(t.heading()), title_area);
    let (Some(first), Some(last)) = (weeks.first(), weeks.last()) else {
        return;
    };
    if weeks.len() < 2 {
        let note = "Not enough weeks yet: a week counts once it has a few answers.";
        frame.render_widget(
            Paragraph::new(note)
                .style(t.muted())
                .wrap(Wrap { trim: true }),
            chart_area,
        );
        return;
    }

    let week_of = |w: &Week| f64::from((w.monday - first.monday).get_days()) / 7.0;
    let values: Vec<f64> = weeks.iter().map(|w| w.value).collect();
    let values = match measure {
        Measure::Seconds => values,
        Measure::Effort => smooth(&values),
    };
    let mut sorted = values.clone();
    sorted.sort_by(f64::total_cmp);
    let usual_high = sorted[(sorted.len() * 9 / 10).min(sorted.len() - 1)] * 1.2;
    let top = match measure {
        Measure::Seconds => usual_high.ceil().max(1.0),
        Measure::Effort => ((usual_high * 2.0).ceil() / 2.0).max(2.0),
    };
    let points: Vec<(f64, f64)> = weeks
        .iter()
        .zip(&values)
        .map(|(w, &v)| (week_of(w), v.min(top)))
        .collect();
    let end = week_of(last).max(1.0);
    let average = [(0.0, 1.0), (end, 1.0)];
    let mut datasets = Vec::new();
    if measure == Measure::Effort {
        datasets.push(
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(t.muted())
                .data(&average),
        );
    }
    datasets.push(
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(t.info())
            .data(&points),
    );

    let date = |weeks: f64| {
        let day = first.monday + ((weeks * 7.0).round() as i64).days();
        day.strftime("%b %-d").to_string()
    };
    let y_label = |v: f64| match measure {
        Measure::Seconds => axis_label(v),
        Measure::Effort => format!("{v:>4.1}×"),
    };
    let chart = Chart::new(datasets)
        .x_axis(Axis::default().style(t.muted()).bounds([0.0, end]).labels([
            date(0.0),
            date(end / 2.0),
            date(end),
        ]))
        .y_axis(
            Axis::default()
                .style(t.muted())
                .bounds([0.0, top])
                .labels([0.0, top / 2.0, top].map(y_label)),
        );
    frame.render_widget(chart, chart_area);
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
    // The longest title that fits.
    let mut titles = vec![];
    if numbers {
        let numbers = format!("median {:.1}s · p90 {:.1}s", speed.at(50), speed.at(90));
        titles.push(format!(
            "Seconds per correct answer, by percentile · {numbers}"
        ));
        titles.push(format!("Seconds by percentile · {numbers}"));
    }
    titles.push("Seconds per correct answer, by percentile".to_owned());
    titles.push("Seconds by percentile".to_owned());
    let title = titles
        .iter()
        .find(|title| title.chars().count() <= usize::from(area.width))
        .unwrap_or(&titles[titles.len() - 1])
        .clone();
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

/// Rows of a weak spot's progress chart.
const WEAK_CHART: u16 = 9;

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
    let (speed_area, progress_area) = pair(chart_area);
    if let Some(area) = speed_area {
        speed_chart(&kind.speed, false, t, frame, area);
    }
    progress_chart(&kind.progress, Measure::Seconds, t, frame, progress_area);
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
    // table starts on the same row for every weak spot. The chart of how
    // it has gone comes between them when the table keeps enough rows.
    let chart_height = if inner.height >= 4 + WEAK_CHART + 1 + MIN_CHART {
        WEAK_CHART
    } else {
        0
    };
    let [header_area, about_area, _, chart_area, _, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(chart_height),
        Constraint::Length(u16::from(chart_height > 0)),
        Constraint::Fill(1),
    ])
    .areas(inner);
    if chart_height > 0 {
        progress_chart(&weak.progress, Measure::Effort, t, frame, chart_area);
    }
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

    fn weeks(values: &[f64]) -> Vec<Week> {
        let first: Date = "2026-03-02".parse().unwrap();
        values
            .iter()
            .enumerate()
            .map(|(i, &value)| Week {
                monday: first + (i as i64).weeks(),
                value,
                count: 10,
                accuracy: 0.9,
            })
            .collect()
    }

    #[test]
    fn progress_title_compares_the_first_and_last_four_weeks() {
        let seconds = weeks(&[4.0, 4.0, 4.0, 4.0, 3.0, 3.0, 2.0, 2.0, 2.0, 2.0]);
        assert_eq!(
            progress_title(&seconds, Measure::Seconds, 80),
            "Seconds per week · 4.0s to 2.0s · 90% to 90% right"
        );
        assert_eq!(
            progress_title(&seconds, Measure::Seconds, 40),
            "Seconds per week · 4.0s to 2.0s"
        );
        assert_eq!(
            progress_title(&seconds, Measure::Seconds, 10),
            "Seconds per week"
        );
        let effort = weeks(&[1.5, 0.5]);
        assert_eq!(
            progress_title(&effort, Measure::Effort, 80),
            "Effort per week · 1.5× to 0.5× · 1× is your average"
        );
        assert_eq!(
            progress_title(&weeks(&[1.5]), Measure::Effort, 80),
            "Effort per week"
        );
    }

    #[test]
    fn smoothing_averages_each_week_with_its_neighbours() {
        assert_eq!(smooth(&[]), Vec::<f64>::new());
        assert_eq!(smooth(&[3.0]), [3.0]);
        assert_eq!(smooth(&[1.0, 3.0, 5.0, 1.0]), [2.0, 3.0, 3.0, 3.0]);
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
