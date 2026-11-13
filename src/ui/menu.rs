//! The home screen: the kanji, then the settings in bordered panels that
//! flow into as many columns as fit.

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, BorderType},
};

use super::{
    bitmap::{BigText, Scale},
    clock, kanji,
};
use crate::{
    app::{Item, Menu, Panel},
    config::Config,
    drill::Mode,
    theme::Theme,
};

/// Width of every panel, borders included.
const PANEL_WIDTH: u16 = 26;

/// Most columns of panels to use, however wide the terminal.
const MAX_COLUMNS: u16 = 4;

pub fn render(menu: &Menu, config: &Config, t: &Theme, frame: &mut Frame, area: Rect) {
    let columns = arrange(area.width);
    let grid_height = columns.iter().map(|c| column_height(c)).max().unwrap_or(0);
    const FOOTER: u16 = 3; // a gap, the mode, the hints
    let title = BigText::bitmap(Some(kanji::anzan()), "暗算").scales(&[Scale::Small]);
    let title_height = title.height(area.as_size());
    let show_title = area.height >= grid_height + FOOTER + title_height;
    let [title_area, grid, _, about, hints] = Layout::vertical([
        Constraint::Length(if show_title { title_height } else { 0 }),
        Constraint::Length(grid_height),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .flex(Flex::Center)
    .areas(area);
    if show_title {
        frame.render_widget(&title, title_area);
        frame.buffer_mut().set_style(title_area, t.accent());
    }

    let width = PANEL_WIDTH * columns.len() as u16;
    let grid = grid.centered_horizontally(Constraint::Length(width.min(area.width)));
    let focused = menu.panel();
    for (i, column) in columns.iter().enumerate() {
        let mut y = grid.y;
        for &panel in column {
            let height = panel_height(panel);
            let rect = Rect::new(grid.x + i as u16 * PANEL_WIDTH, y, PANEL_WIDTH, height)
                .intersection(grid);
            render_panel(panel, menu, config, t, panel == focused, frame, rect);
            y += height;
        }
    }

    let mode = config.session.mode;
    let about_text = match mode {
        Mode::Custom => mode.description().to_owned(),
        m => format!(
            "{} {}, {}: {}",
            m.kanji(),
            m.label(),
            m.meaning(),
            m.description()
        ),
    };
    frame.render_widget(Line::from(about_text).style(t.muted()).centered(), about);
    let hint = match focused {
        Panel::Mode => "↑↓ mode",
        Panel::Session => "↑↓ move · Space change · t theme",
        Panel::Group(_) => "↑↓ move · Space toggle · a all",
    };
    let hint = format!("←→ panel · {hint} · Enter start · d stats");
    frame.render_widget(Line::from(hint).style(t.muted()).centered(), hints);
}

/// The panels in each column for a terminal `width` cells wide: modes
/// and session first, then the groups, in order, split over the other
/// columns so that the tallest column is as short as it can be.
fn arrange(width: u16) -> Vec<Vec<Panel>> {
    let n = usize::from((width / PANEL_WIDTH).clamp(1, MAX_COLUMNS));
    let panels: Vec<Panel> = Panel::all().collect();
    if n == 1 {
        return vec![panels];
    }
    let (first, groups) = panels.split_at(2);
    let mut columns = vec![first.to_vec()];
    columns.extend(split(groups, n - 1).into_iter().map(<[Panel]>::to_vec));
    columns
}

/// `panels` cut into at most `parts` runs, minimising the tallest. There
/// are few enough panels to try every way.
fn split(panels: &[Panel], parts: usize) -> Vec<&[Panel]> {
    if parts <= 1 || panels.len() <= 1 {
        return vec![panels];
    }
    (1..panels.len())
        .map(|cut| {
            let mut rest = split(&panels[cut..], parts - 1);
            rest.insert(0, &panels[..cut]);
            rest
        })
        .chain(std::iter::once(vec![panels]))
        .min_by_key(|runs| runs.iter().map(|r| column_height(r)).max().unwrap_or(0))
        .expect("at least one way")
}

fn panel_height(panel: Panel) -> u16 {
    panel.items().len() as u16 + 2
}

fn column_height(column: &[Panel]) -> u16 {
    column.iter().map(|&p| panel_height(p)).sum()
}

fn render_panel(
    panel: Panel,
    menu: &Menu,
    config: &Config,
    t: &Theme,
    focused: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let number = Panel::all().position(|p| p == panel).unwrap_or(0) + 1;
    let border = t.border(focused);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(format!(" [{number}] {} ", panel.title())).style(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let selected = focused.then(|| menu.selected(config));
    let lines: Vec<Line> = panel
        .items()
        .into_iter()
        .map(|item| item_line(item, config, t, selected == Some(item), inner.width))
        .collect();
    frame.render_widget(Text::from(lines), inner);
}

fn item_line(item: Item, config: &Config, t: &Theme, selected: bool, width: u16) -> Line<'static> {
    let spans: Vec<Span> = match item {
        Item::Mode(mode) => {
            let current = config.session.mode == mode;
            let mark = if current { "●" } else { "○" };
            // Kanji are two cells wide; pad to the width of two of them.
            let kanji = format!("{:<w$}", mode.kanji(), w = 4 - mode.kanji().chars().count());
            let label = Span::from(mode.label().to_owned());
            vec![
                Span::from(format!(" {mark} ")).style(if current { t.accent() } else { t.muted() }),
                Span::from(format!("{kanji} ")).style(t.accent()),
                if current {
                    label.style(t.accent().bold())
                } else {
                    label
                },
            ]
        }
        Item::Duration => setting(t, "Duration", &clock(config.session.duration()), width),
        Item::Submit => setting(t, "Submit", config.session.submit.name(), width),
        Item::Theme => setting(t, "Theme", &t.name, width),
        Item::Kind(kind) => {
            let on = config.problems.kinds.contains(&kind);
            let (mark, style) = if on {
                ("[x]", t.good())
            } else {
                ("[ ]", t.muted())
            };
            vec![
                Span::from(" "),
                Span::from(mark).style(style),
                Span::from(format!(" {}", kind.label())),
            ]
        }
    };
    // The selection fills the whole row, as a bar.
    let used: usize = spans.iter().map(Span::width).sum();
    let mut spans = spans;
    spans.push(Span::from(
        " ".repeat(usize::from(width).saturating_sub(used)),
    ));
    let line = Line::from(spans);
    if selected {
        line.patch_style(t.selected())
    } else {
        line
    }
}

/// `label` on the left and `value` on the right of a row `width` wide.
fn setting(t: &Theme, label: &str, value: &str, width: u16) -> Vec<Span<'static>> {
    let pad = usize::from(width).saturating_sub(3 + label.len() + value.chars().count());
    vec![
        Span::from(format!(" {label}{} ", " ".repeat(pad))),
        Span::from(format!("{value} ")).style(t.info()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_reflow_with_width() {
        assert_eq!(arrange(20).len(), 1);
        assert_eq!(arrange(PANEL_WIDTH * 3).len(), 3);
        assert_eq!(arrange(500).len(), MAX_COLUMNS as usize);
        for width in [20, 60, 90, 120, 500] {
            let columns = arrange(width);
            let order: Vec<Panel> = columns.into_iter().flatten().collect();
            assert_eq!(order, Panel::all().collect::<Vec<_>>(), "width {width}");
        }
    }

    #[test]
    fn columns_are_balanced() {
        let heights =
            |width| -> Vec<u16> { arrange(width).iter().map(|c| column_height(c)).collect() };
        assert_eq!(heights(PANEL_WIDTH * 4), [14, 13, 11, 10]);
        assert_eq!(heights(80), [14, 18, 16]);
    }
}
