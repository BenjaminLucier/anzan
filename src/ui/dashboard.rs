//! Drawing the statistics dashboard.
//!
//! Like everything else, it uses no colours: shading comes from the
//! density of block characters, and emphasis from bold and dim.

use jiff::{ToSpan, civil::Date};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Row, Sparkline, Table, Tabs},
};

use crate::{
    app::{Dashboard, Report, Tab},
    stats::{features::Feature, report::GRID},
};

/// Shades from least to most, for heatmaps.
const SHADES: [char; 5] = ['·', '░', '▒', '▓', '█'];

pub fn render(dash: &Dashboard, frame: &mut Frame, area: Rect) {
    let [tabs, _, body, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let titles = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} {}", i + 1, t.label()));
    let tabs_widget = Tabs::new(titles)
        .select(dash.tab.index())
        .style(Style::new().dim())
        .highlight_style(Style::new().not_dim().bold().underlined());
    frame.render_widget(tabs_widget, centred(tabs, 70));
    let hint = Line::from("←→ tab · Esc menu").dim().centered();
    frame.render_widget(hint, hints);

    let report = &dash.report;
    if report.attempts == 0 {
        let text = Text::from(vec![
            Line::from("Nothing practised yet."),
            Line::from("Finish a session and come back.").dim(),
        ])
        .centered();
        let area = body.centered_vertically(Constraint::Length(2));
        frame.render_widget(text, area);
        return;
    }
    match dash.tab {
        Tab::Overview => overview(report, frame, body),
        Tab::Speed => speed(report, frame, body),
        Tab::Facts => facts(report, frame, body),
        Tab::Activity => activity(report, frame, body),
        Tab::Weak => weak(report, frame, body),
    }
}

/// `area` narrowed to at most `width`, centred.
fn centred(area: Rect, width: u16) -> Rect {
    area.centered_horizontally(Constraint::Length(width.min(area.width)))
}

fn secs(ms: f64) -> String {
    format!("{:.1}s", ms / 1000.0)
}

fn overview(report: &Report, frame: &mut Frame, area: Rect) {
    let area = centred(area, 64);
    let rows = report.kinds.len() as u16 + 1;
    let [totals, _, trend_title, trend, trend_note, _, table] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(rows),
    ])
    .areas(area);

    let mut summary = format!(
        "{} attempts · {} sessions · {} day streak",
        report.attempts,
        report.sessions,
        report.streak()
    );
    if report.damaged > 0 {
        summary.push_str(&format!(" · {} damaged lines skipped", report.damaged));
    }
    frame.render_widget(Line::from(summary).centered(), totals);

    frame.render_widget(
        Line::from("Score with the current settings").bold(),
        trend_title,
    );
    let note = match report.trend.as_slice() {
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
    let shown = report
        .trend
        .iter()
        .rev()
        .take(usize::from(trend.width))
        .rev()
        .map(|&s| u64::from(s))
        .collect::<Vec<_>>();
    frame.render_widget(Sparkline::default().data(&shown), trend);
    frame.render_widget(Line::from(note).dim(), trend_note);

    let header = Row::new(["Kind", "Seen", "Right", "Median", "p90"]).bold();
    let rows = report.kinds.iter().map(|k| {
        Row::new([
            k.kind.label().to_owned(),
            k.attempts.to_string(),
            format!("{:.0}%", k.accuracy() * 100.0),
            secs(k.median_ms),
            secs(k.p90_ms),
        ])
    });
    let widths = [
        Constraint::Fill(1),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Length(7),
    ];
    frame.render_widget(Table::new(rows, widths).header(header), table);
}

/// Response time against percentile, recent over all time.
fn speed(report: &Report, frame: &mut Frame, area: Rect) {
    let area = centred(area, 80);
    // Cap the y axis at the 95th percentile so the slow tail does not
    // flatten everything else.
    let top = report
        .curve_all
        .iter()
        .chain(&report.curve_recent)
        .filter(|(p, _)| *p <= 95.0)
        .map(|(_, s)| *s)
        .fold(1.0, f64::max)
        .ceil();
    let clip = |curve: &[(f64, f64)]| -> Vec<(f64, f64)> {
        curve.iter().map(|&(p, s)| (p, s.min(top))).collect()
    };
    let (all, recent) = (clip(&report.curve_all), clip(&report.curve_recent));
    let mut datasets = vec![
        Dataset::default()
            .name("all time")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::new().dim())
            .data(&all),
    ];
    if !recent.is_empty() {
        datasets.push(
            Dataset::default()
                .name("last 7 days")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::new().bold())
                .data(&recent),
        );
    }
    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .title("percentile")
                .bounds([0.0, 100.0])
                .labels(["0", "25", "50", "75", "100"]),
        )
        .y_axis(Axis::default().title("seconds").bounds([0.0, top]).labels([
            "0".to_owned(),
            format!("{:.0}", top / 2.0),
            format!("{top:.0}"),
        ]))
        .legend_position(Some(ratatui::widgets::LegendPosition::TopLeft));
    let [chart_area, note] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    frame.render_widget(chart, chart_area);
    let at = |curve: &[(f64, f64)], p: usize| curve.get(p).map_or(0.0, |&(_, s)| s);
    let line = format!(
        "all time: half within {:.1}s, 90% within {:.1}s",
        at(&report.curve_all, 50),
        at(&report.curve_all, 90)
    );
    frame.render_widget(Line::from(line).dim().centered(), note);
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

/// The times-table heatmap, and what carries and borrows cost.
fn facts(report: &Report, frame: &mut Frame, area: Rect) {
    let [grid_area, costs_area] =
        Layout::horizontal([Constraint::Length(32), Constraint::Length(36)])
            .flex(Flex::Center)
            .spacing(4)
            .areas(area);

    let mut lines = vec![Line::from("Times tables").bold(), Line::default()];
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
    lines.push(Line::default());
    lines.push(Line::from(format!("easy {} hard", SHADES.iter().collect::<String>())).dim());
    lines.push(Line::from("blank: not seen yet").dim());
    frame.render_widget(Text::from(lines), grid_area);

    let mut lines = vec![Line::from("Carrying and borrowing").bold(), Line::default()];
    let mut section = |title: &str, costs: &[crate::stats::report::Cost], what: &str| {
        lines.push(Line::from(title.to_owned()).underlined());
        if costs.is_empty() {
            lines.push(Line::from("  not enough answers yet").dim());
        }
        for c in costs {
            // Eight cells is the usual time; each cell either side is 1/16.
            let cells = ((c.factor - 0.5) * 16.0).round().clamp(0.0, 16.0) as usize;
            let bar = "█".repeat(cells);
            lines.push(Line::from(vec![
                Span::from(format!("  {} {what:<8}", c.steps)),
                Span::from(format!("{:>5.2}× ", c.factor)),
                Span::from(bar).dim(),
                Span::from(format!(" {}", c.count)).dim(),
            ]));
        }
        lines.push(Line::default());
    };
    section("Addition", &report.carries, "carries");
    section("Subtraction", &report.borrows, "borrows");
    lines.push(Line::from("× the usual time · then how many").dim());
    frame.render_widget(Text::from(lines), costs_area);
}

/// Attempts per day, a column per week, oldest on the left.
fn activity(report: &Report, frame: &mut Frame, area: Rect) {
    const LABEL: u16 = 4;
    let area = centred(area, 2 * 53 + LABEL);
    let weeks = i64::from(area.width.saturating_sub(LABEL) / 2).max(1);
    let today = report.today;
    let monday = today - i64::from(today.weekday().to_monday_zero_offset()).days();
    let start = monday - (weeks - 1).weeks();
    let max = report.activity.values().copied().max().unwrap_or(1).max(1);
    let level = |n: u32| -> char {
        if n == 0 {
            return SHADES[0];
        }
        let f = f64::from(n) / f64::from(max);
        SHADES[1 + ((f * 4.0).ceil() as usize).clamp(1, 4) - 1]
    };

    let mut lines = vec![Line::from("Practice per day").bold(), Line::default()];
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
    let months: String = months.into_iter().collect();
    lines.push(Line::from(months).dim());
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
    lines.push(Line::default());
    let days = report.activity.len();
    lines.push(
        Line::from(format!(
            "{days} days practised · {} day streak · most in a day: {max}",
            report.streak()
        ))
        .dim(),
    );
    let height = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines),
        area.centered_vertically(Constraint::Length(height)),
    );
}

fn month(m: i8) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES[usize::try_from(m - 1).unwrap_or(0)]
}

/// The questions that have been the most trouble.
fn weak(report: &Report, frame: &mut Frame, area: Rect) {
    let area = centred(area, 60);
    let [title, _, table, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);
    frame.render_widget(Line::from("Hardest questions").bold(), title);
    if report.hardest.is_empty() {
        frame.render_widget(Line::from("Nothing has come up twice yet.").dim(), table);
        return;
    }
    let header = Row::new(["Question", "Seen", "Effort"]).bold();
    let rows = report.hardest.iter().map(|(feature, skill)| {
        let question = match feature {
            Feature::Item(p) => p.to_string(),
            other => other.to_string(),
        };
        Row::new([
            question,
            skill.count.to_string(),
            format!("{:.1}×", skill.score.exp()),
        ])
    });
    let widths = [
        Constraint::Fill(1),
        Constraint::Length(6),
        Constraint::Length(7),
    ];
    frame.render_widget(Table::new(rows, widths).header(header), table);
    frame.render_widget(
        Line::from("Effort: time over the usual, with misses counted extra").dim(),
        note,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shades_rise_with_struggle() {
        let shades: String = [-1.0, -0.1, 0.1, 0.3, 2.0].map(shade).iter().collect();
        assert_eq!(shades, "·░▒▓█");
    }
}
