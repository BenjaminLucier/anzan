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
        model::Skill,
        report::{Cost, GRID, percentile_curve},
    },
    store::Attempt,
    theme::Theme,
};

/// Shades from least to most, for heatmaps.
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
        Side::Summary => overview(report, history, t, frame, details),
        Side::Kinds => match report.kinds.get(dash.selected(Side::Kinds)) {
            Some(stats) => kind_details(report, history, stats.kind, t, frame, details),
            None => empty(t, frame, details),
        },
        Side::Sessions => match report.sessions.get(dash.selected(Side::Sessions)) {
            Some(session) => session_details(dash, history, session, t, frame, details),
            None => empty(t, frame, details),
        },
        Side::Weak => match report.hardest.get(dash.selected(Side::Weak)) {
            Some(&(Feature::Item(problem), skill)) => {
                item_details(dash, history, problem, skill, t, frame, details);
            }
            _ => empty(t, frame, details),
        },
    }
}

/// A rounded panel, bold when focused and dim otherwise.
fn panel(t: &Theme, title: String, focused: bool) -> Block<'static> {
    let style = t.border(focused);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(style)
        .title(Line::from(format!(" {title} ")).style(style))
}

fn empty(t: &Theme, frame: &mut Frame, area: Rect) {
    frame.render_widget(panel(t, String::new(), false), area);
}

fn secs(ms: f64) -> String {
    format!("{:.1}s", ms / 1000.0)
}

fn local(report: &Report, ts: jiff::Timestamp) -> Zoned {
    ts.to_zoned(report.tz.clone())
}

/// The lists down the left.
fn render_side(dash: &Dashboard, t: &Theme, frame: &mut Frame, area: Rect) {
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
        lines
            .push(Line::from(format!("{} damaged lines skipped", report.damaged)).style(t.muted()));
    }
    let focused = dash.focus == Side::Summary;
    let block = numbered(t, Side::Summary, focused);
    let summary_text = Paragraph::new(lines).block(block);
    frame.render_widget(
        if focused {
            summary_text.style(t.heading())
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
    list(dash, Side::Kinds, rows.collect(), t, frame, kinds);

    let rows = report.sessions.iter().map(|s| {
        let when = local(report, s.start).strftime("%m-%d %H:%M").to_string();
        let score = if s.completed {
            s.score.to_string()
        } else {
            format!("({})", s.score)
        };
        spread(&format!("{when}  {}", s.mode.label()), &score, inner)
    });
    list(dash, Side::Sessions, rows.collect(), t, frame, sessions);

    let rows = report.hardest.iter().map(|(feature, skill)| {
        let question = match feature {
            Feature::Item(p) => p.to_string(),
            other => other.to_string(),
        };
        spread(&question, &format!("{:.1}×", skill.score.exp()), inner)
    });
    list(dash, Side::Weak, rows.collect(), t, frame, weak);
}

fn numbered(t: &Theme, side: Side, focused: bool) -> Block<'static> {
    panel(
        t,
        format!("[{}] {}", side.index() + 1, side.title()),
        focused,
    )
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
fn list(dash: &Dashboard, side: Side, rows: Vec<String>, t: &Theme, frame: &mut Frame, area: Rect) {
    let focused = dash.focus == side;
    let highlight = if focused {
        t.selected()
    } else {
        Style::new().bold()
    };
    let list = List::new(rows)
        .block(numbered(t, side, focused))
        .highlight_style(highlight);
    let mut state = ListState::default().with_selected(Some(dash.selected(side)));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Everything at a glance: the score trend, practice per day, and speed.
fn overview(report: &Report, history: &History, t: &Theme, frame: &mut Frame, area: Rect) {
    let block = panel(t, "Overview".into(), false);
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
    render_trend(report, t, frame, trend);
    activity(report, t, frame, activity_area);
    let all = percentile_curve(&history.attempts);
    let since = report.now - RECENT;
    let recent = percentile_curve(history.attempts.iter().filter(|a| a.ts >= since));
    speed_chart(&all, &recent, t, frame, speed_area);
}

fn render_trend(report: &Report, t: &Theme, frame: &mut Frame, area: Rect) {
    let [title, spark, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .areas(area);
    frame.render_widget(
        Line::from("Score with the current settings").style(t.heading()),
        title,
    );
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
    frame.render_widget(Sparkline::default().data(&shown).style(t.info()), spark);
    frame.render_widget(Line::from(note_text).style(t.muted()), note);
}

/// Response time against percentile, recent over all time.
fn speed_chart(
    all: &[(f64, f64)],
    recent: &[(f64, f64)],
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
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
            .style(t.muted())
            .data(&all_clipped),
    ];
    if !recent_clipped.is_empty() {
        datasets.push(
            Dataset::default()
                .name("last 7 days")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(t.heading())
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

/// One kind: its numbers and speed, with the facts behind it where there
/// are any, and its hardest questions.
fn kind_details(
    report: &Report,
    history: &History,
    kind: Kind,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    let block = panel(t, kind.label().into(), false);
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
        Kind::Mul | Kind::Div | Kind::LongMul => Some(fact_grid(t, report)),
        Kind::Add => Some(costs(t, "Carries", &report.carries, "carries")),
        Kind::Sub => Some(costs(t, "Borrows", &report.borrows, "borrows")),
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
    speed_chart(&all, &recent, t, frame, chart_area);

    if !hardest.is_empty() {
        let mut lines = vec![Line::default(), Line::from("Hardest").style(t.heading())];
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

/// The heat level, 0 to 4, for a struggle score `d` above (or below) the
/// average.
fn level(d: f64) -> usize {
    match d {
        d if d < -0.2 => 0,
        d if d < 0.0 => 1,
        d if d < 0.2 => 2,
        d if d < 0.5 => 3,
        _ => 4,
    }
}

/// The times-table heatmap.
fn fact_grid(t: &Theme, report: &Report) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("Times tables").style(t.heading())];
    let header: String = GRID.map(|b| format!("{b:>3}")).collect();
    lines.push(Line::from(format!("  ×{header}")).style(t.muted()));
    for (i, a) in GRID.enumerate() {
        let mut spans = vec![Span::from(format!("{a:>3} ")).style(t.muted())];
        for cell in &report.grid[i] {
            let text = match cell {
                Some(skill) => {
                    let level = level(skill.score - report.overall);
                    spans
                        .push(Span::from(SHADES[level].to_string().repeat(2)).style(t.heat(level)));
                    spans.push(Span::from(" "));
                    continue;
                }
                None => "  ".to_owned(),
            };
            spans.push(Span::from(format!("{text} ")));
        }
        lines.push(Line::from(spans));
    }
    let mut legend = vec![Span::from("easy ").style(t.muted())];
    legend.extend((0..SHADES.len()).map(|i| Span::from(SHADES[i].to_string()).style(t.heat(i))));
    legend.push(Span::from(" hard").style(t.muted()));
    lines.push(Line::from(legend));
    lines
}

/// How much carries or borrows slow things down.
fn costs(t: &Theme, title: &str, costs: &[Cost], what: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(title.to_owned()).style(t.heading())];
    if costs.is_empty() {
        lines.push(Line::from("not enough answers yet").style(t.muted()));
    }
    for c in costs {
        // Eight cells is the usual time; each cell either side is 1/16.
        let cells = ((c.factor - 0.5) * 16.0).round().clamp(0.0, 16.0) as usize;
        lines.push(Line::from(vec![
            Span::from(format!("{} {what:<8}{:>5.2}× ", c.steps, c.factor)),
            Span::from("█".repeat(cells)).style(t.muted()),
        ]));
    }
    lines.push(Line::from("× the usual time").style(t.muted()));
    lines
}

/// One session: its settings and every question asked.
fn session_details(
    dash: &Dashboard,
    history: &History,
    session: &crate::store::SessionRecord,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    let report = &dash.report;
    let when = local(report, session.start)
        .strftime("%a %-d %b %Y, %H:%M")
        .to_string();
    let block = panel(t, format!("Session · {when}"), false);
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
        t,
        frame,
        table_area,
    );
}

/// One question: how every attempt at it went.
fn item_details(
    dash: &Dashboard,
    history: &History,
    item: Problem,
    skill: Skill,
    t: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    let block = panel(t, item.to_string(), false);
    let inner = block.inner(area).inner(ratatui::layout::Margin::new(1, 0));
    frame.render_widget(block, area);
    let header = format!(
        "{} = {} · seen {} · {:.1}× the usual effort",
        item,
        item.answer(),
        skill.count,
        skill.score.exp()
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
    attempt_table(
        attempts,
        dash.scroll,
        true,
        &dash.report,
        t,
        frame,
        table_area,
    );
}

/// Attempts as a table, scrolled down by `scroll` rows. `dated` shows the
/// date rather than the question.
fn attempt_table<'a>(
    attempts: impl Iterator<Item = &'a Attempt>,
    scroll: u16,
    dated: bool,
    report: &Report,
    t: &Theme,
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
                (true, _) => Cell::from("✓").style(t.good()),
                (false, true) => Cell::from("skip").style(t.warn()),
                (false, false) => Cell::from("✗").style(t.bad()),
            };
            Row::new([
                Cell::from(first),
                Cell::from(a.answer.clone()).style(t.muted()),
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
        assert_eq!([-1.0, -0.1, 0.1, 0.3, 2.0].map(level), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn spread_fills_and_truncates() {
        assert_eq!(spread("ab", "9", 6), "ab   9");
        assert_eq!(spread("abcdef", "9", 4), "ab 9");
    }
}
