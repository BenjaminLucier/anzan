//! Drawing the statistics dashboard, in the style of lazygit: lists down
//! the left, and the details of the selected row filling the right.
//!
//! Like everything else, it uses no colours: shading comes from the
//! density of block characters, and emphasis from bold, dim and reverse.

use jiff::{ToSpan, Zoned, civil::Date};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{
        Axis, Block, BorderType, Cell, Chart, Dataset, GraphType, LegendPosition, List, ListState,
        Padding, Paragraph, Row, Sparkline, Table,
    },
};

use crate::{
    app::{Dashboard, RECENT, Report, Side},
    history::History,
    problem::{Kind, Problem},
    stats::{
        features::{Feature, canonical},
        report::{Cost, GRID, percentile_curve},
    },
    store::Attempt,
};

/// Shades from least to most, for heatmaps.
const SHADES: [char; 5] = ['·', '░', '▒', '▓', '█'];

/// Width of the column of lists.
const SIDE_WIDTH: u16 = 36;

pub fn render(dash: &Dashboard, history: &History, frame: &mut Frame, area: Rect) {
    let [main, hints] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let hint = "←→ list · ↑↓ select · PgUp/PgDn scroll · Esc menu";
    frame.render_widget(Line::from(hint).dim().centered(), hints);

    let report = &dash.report;
    if report.attempts == 0 {
        let text = Text::from(vec![
            Line::from("Nothing practised yet."),
            Line::from("Finish a session and come back.").dim(),
        ])
        .centered();
        frame.render_widget(text, main.centered_vertically(Constraint::Length(2)));
        return;
    }

    let side_width = SIDE_WIDTH.min(main.width / 2);
    let [side, details] =
        Layout::horizontal([Constraint::Length(side_width), Constraint::Fill(1)]).areas(main);
    render_side(dash, frame, side);
    match dash.focus {
        Side::Summary => overview(report, history, frame, details),
        Side::Kinds => match report.kinds.get(dash.selected(Side::Kinds)) {
            Some(stats) => kind_details(report, history, stats.kind, frame, details),
            None => empty(frame, details),
        },
        Side::Sessions => match report.sessions.get(dash.selected(Side::Sessions)) {
            Some(session) => session_details(dash, history, session, frame, details),
            None => empty(frame, details),
        },
        Side::Weak => match report.hardest.get(dash.selected(Side::Weak)) {
            Some(&(Feature::Item(problem), skill)) => {
                item_details(
                    dash,
                    history,
                    problem,
                    skill.score,
                    skill.count,
                    frame,
                    details,
                );
            }
            _ => empty(frame, details),
        },
    }
}

/// A rounded panel, bold when focused and dim otherwise.
fn panel(title: String, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::new().bold()
    } else {
        Style::new().dim()
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(style)
        .title(Line::from(format!(" {title} ")).style(style))
}

fn empty(frame: &mut Frame, area: Rect) {
    frame.render_widget(panel(String::new(), false), area);
}

fn secs(ms: f64) -> String {
    format!("{:.1}s", ms / 1000.0)
}

fn local(report: &Report, ts: jiff::Timestamp) -> Zoned {
    ts.to_zoned(report.tz.clone())
}

/// The lists down the left.
fn render_side(dash: &Dashboard, frame: &mut Frame, area: Rect) {
    let report = &dash.report;
    let kinds_height = report.kinds.len().min(10) as u16 + 2;
    let [summary, kinds, sessions, weak] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(kinds_height),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(area);

    let mut lines = vec![
        Line::from(format!("{} attempts", report.attempts)),
        Line::from(format!("{} sessions", report.sessions.len())),
        Line::from(format!(
            "{} day streak · {} days in all",
            report.streak(),
            report.activity.len()
        )),
    ];
    if report.damaged > 0 {
        lines.push(Line::from(format!("{} damaged lines skipped", report.damaged)).dim());
    }
    let focused = dash.focus == Side::Summary;
    let block = numbered(Side::Summary, focused);
    let summary_text = Paragraph::new(lines).block(block);
    frame.render_widget(
        if focused {
            summary_text.bold()
        } else {
            summary_text
        },
        summary,
    );

    let inner = usize::from(area.width.saturating_sub(4));
    let rows = report.kinds.iter().map(|k| {
        let stats = format!("{:.0}% {:>5}", k.accuracy() * 100.0, secs(k.median_ms));
        spread(k.kind.label(), &stats, inner)
    });
    list(dash, Side::Kinds, rows.collect(), frame, kinds);

    let rows = report.sessions.iter().map(|s| {
        let when = local(report, s.start).strftime("%m-%d %H:%M").to_string();
        let score = if s.completed {
            s.score.to_string()
        } else {
            format!("({})", s.score)
        };
        spread(&format!("{when}  {}", s.mode.label()), &score, inner)
    });
    list(dash, Side::Sessions, rows.collect(), frame, sessions);

    let rows = report.hardest.iter().map(|(feature, skill)| {
        let question = match feature {
            Feature::Item(p) => p.to_string(),
            other => other.to_string(),
        };
        spread(&question, &format!("{:.1}×", skill.score.exp()), inner)
    });
    list(dash, Side::Weak, rows.collect(), frame, weak);
}

fn numbered(side: Side, focused: bool) -> Block<'static> {
    panel(format!("[{}] {}", side.index() + 1, side.title()), focused)
        .padding(Padding::horizontal(1))
}

/// `left` and `right` at either end of `width` cells, `left` cut short if
/// they do not both fit.
fn spread(left: &str, right: &str, width: usize) -> String {
    let room = width.saturating_sub(right.chars().count() + 1);
    let left: String = left.chars().take(room).collect();
    let pad = width.saturating_sub(left.chars().count() + right.chars().count());
    format!("{left}{}{right}", " ".repeat(pad))
}

/// A list down the side, scrolled to keep its selection in view.
fn list(dash: &Dashboard, side: Side, rows: Vec<String>, frame: &mut Frame, area: Rect) {
    let focused = dash.focus == side;
    let highlight = if focused {
        Style::new().reversed()
    } else {
        Style::new().bold()
    };
    let list = List::new(rows)
        .block(numbered(side, focused))
        .highlight_style(highlight);
    let mut state = ListState::default().with_selected(Some(dash.selected(side)));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Everything at a glance: the score trend, practice per day, and speed.
fn overview(report: &Report, history: &History, frame: &mut Frame, area: Rect) {
    let block = panel("Overview".into(), false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let inner = inner.inner(ratatui::layout::Margin::new(1, 0));
    let [trend, _, activity_area, _, speed_area] = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    render_trend(report, frame, trend);
    activity(report, frame, activity_area);
    let all = percentile_curve(&history.attempts);
    let since = report.now - RECENT;
    let recent = percentile_curve(history.attempts.iter().filter(|a| a.ts >= since));
    speed_chart(&all, &recent, frame, speed_area);
}

fn render_trend(report: &Report, frame: &mut Frame, area: Rect) {
    let [title, spark, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .areas(area);
    frame.render_widget(Line::from("Score with the current settings").bold(), title);
    let note_text = match report.trend.as_slice() {
        [] => "No finished sessions with these settings yet.".to_owned(),
        scores => {
            let best = scores.iter().max().copied().unwrap_or(0);
            let last = scores.last().copied().unwrap_or(0);
            let mean = scores.iter().sum::<u32>() as f64 / scores.len() as f64;
            format!(
                "{} sessions · last {last} · best {best} · mean {mean:.1}",
                scores.len()
            )
        }
    };
    // The newest sessions that fit, one column each.
    let shown: Vec<u64> = report
        .trend
        .iter()
        .rev()
        .take(usize::from(spark.width))
        .rev()
        .map(|&s| u64::from(s))
        .collect();
    frame.render_widget(Sparkline::default().data(&shown), spark);
    frame.render_widget(Line::from(note_text).dim(), note);
}

/// Response time against percentile, recent over all time.
fn speed_chart(all: &[(f64, f64)], recent: &[(f64, f64)], frame: &mut Frame, area: Rect) {
    if all.is_empty() || area.height < 6 {
        return;
    }
    // Cap the y axis at the 95th percentile so the slow tail does not
    // flatten everything else.
    let top = all
        .iter()
        .chain(recent)
        .filter(|(p, _)| *p <= 95.0)
        .map(|(_, s)| *s)
        .fold(1.0, f64::max)
        .ceil();
    let clip = |curve: &[(f64, f64)]| -> Vec<(f64, f64)> {
        curve.iter().map(|&(p, s)| (p, s.min(top))).collect()
    };
    let (all_clipped, recent_clipped) = (clip(all), clip(recent));
    let mut datasets = vec![
        Dataset::default()
            .name("all time")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::new().dim())
            .data(&all_clipped),
    ];
    if !recent_clipped.is_empty() {
        datasets.push(
            Dataset::default()
                .name("last 7 days")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::new().bold())
                .data(&recent_clipped),
        );
    }
    let at = |p: usize| all.get(p).map_or(0.0, |&(_, s)| s);
    let title = format!(
        "Seconds per correct answer, by percentile · median {:.1}s · p90 {:.1}s",
        at(50),
        at(90)
    );
    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(["0", "25", "50", "75", "100"]),
        )
        .y_axis(Axis::default().bounds([0.0, top]).labels([
            "0".to_owned(),
            format!("{:.0}", top / 2.0),
            format!("{top:.0}"),
        ]))
        .legend_position(Some(LegendPosition::TopLeft));
    let [title_area, chart_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    frame.render_widget(Line::from(title).bold(), title_area);
    frame.render_widget(chart, chart_area);
}

/// Attempts per day, a column per week, oldest on the left.
fn activity(report: &Report, frame: &mut Frame, area: Rect) {
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

    let mut lines = vec![Line::from("Practice per day").bold()];
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
    lines.push(Line::from(months.into_iter().collect::<String>()).dim());
    for (d, name) in ["Mon", "", "Wed", "", "Fri", "", "Sun"].iter().enumerate() {
        let mut row = vec![Span::from(format!("{name:<4}")).dim()];
        for w in 0..weeks {
            let day: Date = start + (w * 7 + d as i64).days();
            if day > today {
                break;
            }
            let n = report.activity.get(&day).copied().unwrap_or(0);
            let cell = Span::from(format!("{} ", level(n)));
            row.push(if n == 0 { cell.dim() } else { cell });
        }
        lines.push(Line::from(row));
    }
    lines.push(Line::from(format!("most in a day: {max}")).dim());
    frame.render_widget(Paragraph::new(lines), area);
}

fn month(m: i8) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES[usize::try_from(m - 1).unwrap_or(0)]
}

/// One kind: its numbers and speed, with the facts behind it where there
/// are any, and its hardest questions.
fn kind_details(report: &Report, history: &History, kind: Kind, frame: &mut Frame, area: Rect) {
    let block = panel(kind.label().into(), false);
    let inner = block.inner(area).inner(ratatui::layout::Margin::new(1, 0));
    frame.render_widget(block, area);

    let attempts: Vec<&Attempt> = history
        .attempts
        .iter()
        .filter(|a| a.problem.kind() == kind)
        .collect();
    let stats = report.kinds.iter().find(|k| k.kind == kind);
    let summary = stats.map_or_else(String::new, |k| {
        format!(
            "{} seen · {:.0}% right · median {} · p90 {}",
            k.attempts,
            k.accuracy() * 100.0,
            secs(k.median_ms),
            secs(k.p90_ms)
        )
    });
    let hardest: Vec<_> = report
        .hardest
        .iter()
        .filter(|(f, _)| matches!(f, Feature::Item(p) if p.kind() == kind))
        .take(8)
        .collect();
    let [summary_area, _, middle, hard_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(if hardest.is_empty() {
            0
        } else {
            hardest.len() as u16 + 2
        }),
    ])
    .areas(inner);
    frame.render_widget(Line::from(summary), summary_area);

    let side_lines = match kind {
        Kind::Mul | Kind::Div | Kind::LongMul => Some(fact_grid(report)),
        Kind::Add => Some(costs("Carries", &report.carries, "carries")),
        Kind::Sub => Some(costs("Borrows", &report.borrows, "borrows")),
        _ => None,
    };
    let chart_area = match side_lines {
        Some(lines) if middle.width >= 70 => {
            let [chart, _, side] = Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Length(2),
                Constraint::Length(30),
            ])
            .areas(middle);
            frame.render_widget(Text::from(lines), side);
            chart
        }
        _ => middle,
    };
    let since = report.now - RECENT;
    let all = percentile_curve(attempts.iter().copied());
    let recent = percentile_curve(attempts.iter().copied().filter(|a| a.ts >= since));
    speed_chart(&all, &recent, frame, chart_area);

    if !hardest.is_empty() {
        let mut lines = vec![Line::default(), Line::from("Hardest").bold()];
        for (feature, skill) in hardest {
            lines.push(Line::from(spread(
                &feature.to_string(),
                &format!("{:.1}× · seen {}", skill.score.exp(), skill.count),
                usize::from(hard_area.width.min(50)),
            )));
        }
        frame.render_widget(Text::from(lines), hard_area);
    }
}

/// The shade for a struggle score `d` above (or below) the average.
fn shade(d: f64) -> char {
    match d {
        d if d < -0.2 => SHADES[0],
        d if d < 0.0 => SHADES[1],
        d if d < 0.2 => SHADES[2],
        d if d < 0.5 => SHADES[3],
        _ => SHADES[4],
    }
}

/// The times-table heatmap.
fn fact_grid(report: &Report) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("Times tables").bold()];
    let header: String = GRID.map(|b| format!("{b:>3}")).collect();
    lines.push(Line::from(format!("  ×{header}")).dim());
    for (i, a) in GRID.enumerate() {
        let mut spans = vec![Span::from(format!("{a:>3} ")).dim()];
        for cell in &report.grid[i] {
            let text = match cell {
                Some(skill) => shade(skill.score - report.overall).to_string().repeat(2),
                None => "  ".to_owned(),
            };
            spans.push(Span::from(format!("{text} ")));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(format!("easy {} hard", SHADES.iter().collect::<String>())).dim());
    lines
}

/// How much carries or borrows slow things down.
fn costs(title: &str, costs: &[Cost], what: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(title.to_owned()).bold()];
    if costs.is_empty() {
        lines.push(Line::from("not enough answers yet").dim());
    }
    for c in costs {
        // Eight cells is the usual time; each cell either side is 1/16.
        let cells = ((c.factor - 0.5) * 16.0).round().clamp(0.0, 16.0) as usize;
        lines.push(Line::from(vec![
            Span::from(format!("{} {what:<8}{:>5.2}× ", c.steps, c.factor)),
            Span::from("█".repeat(cells)).dim(),
        ]));
    }
    lines.push(Line::from("× the usual time").dim());
    lines
}

/// One session: its settings and every question asked.
fn session_details(
    dash: &Dashboard,
    history: &History,
    session: &crate::store::SessionRecord,
    frame: &mut Frame,
    area: Rect,
) {
    let report = &dash.report;
    let when = local(report, session.start)
        .strftime("%a %-d %b %Y, %H:%M")
        .to_string();
    let block = panel(format!("Session · {when}"), false);
    let inner = block.inner(area).inner(ratatui::layout::Margin::new(1, 0));
    frame.render_widget(block, area);
    let status = if session.completed {
        "finished"
    } else {
        "abandoned"
    };
    let header = format!(
        "{} · {} s · {} submit · score {} · {status}",
        session.mode.label(),
        session.duration,
        session.submit.name(),
        session.score,
    );
    let attempts: Vec<&Attempt> = history
        .attempts
        .iter()
        .filter(|a| a.session == session.id)
        .collect();
    let [header_area, _, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    frame.render_widget(Line::from(header), header_area);
    attempt_table(
        attempts.into_iter(),
        dash.scroll,
        false,
        report,
        frame,
        table_area,
    );
}

/// One question: how every attempt at it went.
fn item_details(
    dash: &Dashboard,
    history: &History,
    item: Problem,
    score: f64,
    count: u32,
    frame: &mut Frame,
    area: Rect,
) {
    let block = panel(item.to_string(), false);
    let inner = block.inner(area).inner(ratatui::layout::Margin::new(1, 0));
    frame.render_widget(block, area);
    let header = format!(
        "{} = {} · seen {count} · {:.1}× the usual effort",
        item,
        item.answer(),
        score.exp()
    );
    let [header_area, _, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    frame.render_widget(Line::from(header), header_area);
    let attempts = history
        .attempts
        .iter()
        .rev()
        .filter(|a| canonical(&a.problem) == item);
    attempt_table(attempts, dash.scroll, true, &dash.report, frame, table_area);
}

/// Attempts as a table, scrolled down by `scroll` rows. `dated` shows the
/// date rather than the question.
fn attempt_table<'a>(
    attempts: impl Iterator<Item = &'a Attempt>,
    scroll: u16,
    dated: bool,
    report: &Report,
    frame: &mut Frame,
    area: Rect,
) {
    let attempts: Vec<&Attempt> = attempts.collect();
    // Never scroll past the last row.
    let skip = usize::from(scroll).min(attempts.len().saturating_sub(1));
    let rows: Vec<Row> = attempts[skip..]
        .iter()
        .map(|a| {
            let first = if dated {
                local(report, a.ts).strftime("%Y-%m-%d %H:%M").to_string()
            } else {
                a.question.clone()
            };
            let mark = match (a.correct, a.skipped) {
                (true, _) => Cell::from("✓"),
                (false, true) => Cell::from("skip").dim(),
                (false, false) => Cell::from("✗").bold(),
            };
            Row::new([
                Cell::from(first),
                Cell::from(a.answer.clone()).dim(),
                Cell::from(a.given.clone()),
                mark,
                Cell::from(secs(a.ms as f64)),
            ])
        })
        .collect();
    let header = Row::new([
        if dated { "When" } else { "Question" },
        "Answer",
        "Given",
        "",
        "Time",
    ])
    .bold();
    let widths = [
        Constraint::Fill(2),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Length(4),
        Constraint::Length(7),
    ];
    frame.render_widget(Table::new(rows, widths).header(header), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shades_rise_with_struggle() {
        let shades: String = [-1.0, -0.1, 0.1, 0.3, 2.0].map(shade).iter().collect();
        assert_eq!(shades, "·░▒▓█");
    }

    #[test]
    fn spread_fills_and_truncates() {
        assert_eq!(spread("ab", "9", 6), "ab   9");
        assert_eq!(spread("abcdef", "9", 4), "ab 9");
    }
}
