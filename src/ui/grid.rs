//! A grid of characters, and a widget that draws one centred.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect, Size},
    style::{Style, Stylize},
    text::Line,
    widgets::Widget,
};

/// Rows of characters, where a space is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    width: usize,
    cells: Vec<Vec<char>>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            cells: vec![vec![' '; width]; height],
        }
    }

    /// One row of characters as they are.
    pub fn line(text: &str) -> Self {
        let row: Vec<char> = text.chars().collect();
        Self {
            width: row.len(),
            cells: vec![row],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.cells.len()
    }

    pub fn put(&mut self, x: usize, y: usize, c: char) {
        if let Some(cell) = self.cells.get_mut(y).and_then(|row| row.get_mut(x)) {
            *cell = c;
        }
    }

    /// Copies `other` in with its top left at (`x`, `y`). Its spaces leave
    /// what is underneath alone.
    pub fn blit(&mut self, other: &Grid, x: usize, y: usize) {
        for (dy, row) in other.cells.iter().enumerate() {
            for (dx, &c) in row.iter().enumerate() {
                if c != ' ' {
                    self.put(x + dx, y + dy, c);
                }
            }
        }
    }

    pub fn rows(&self) -> impl Iterator<Item = String> + '_ {
        self.cells.iter().map(|row| row.iter().collect())
    }
}

/// A grid drawn bold and centred, or plain text when it does not fit.
pub struct Drawing<'a> {
    grid: Option<Grid>,
    text: &'a str,
}

impl<'a> Drawing<'a> {
    pub fn new(grid: Option<Grid>, text: &'a str) -> Self {
        Self { grid, text }
    }

    fn fitting(&self, area: Size) -> Option<&Grid> {
        self.grid.as_ref().filter(|g| {
            g.width() <= usize::from(area.width) && g.height() <= usize::from(area.height)
        })
    }

    /// Rows needed in `area`.
    pub fn height(&self, area: Size) -> u16 {
        self.fitting(area).map_or(1, |g| g.height() as u16)
    }
}

impl Widget for &Drawing<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Some(grid) = self.fitting(area.as_size()) else {
            let line = Line::from(self.text).bold().centered();
            line.render(area.centered_vertically(Constraint::Length(1)), buf);
            return;
        };
        let x0 = area.x + (area.width - grid.width() as u16) / 2;
        let y0 = area.y + (area.height - grid.height() as u16) / 2;
        for (y, row) in grid.cells.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if c != ' ' {
                    buf[(x0 + x as u16, y0 + y as u16)]
                        .set_char(c)
                        .set_style(Style::new().bold());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(drawing: &Drawing, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        drawing.render(area, &mut buf);
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn blit_keeps_what_spaces_cover() {
        let mut grid = Grid::line("ab");
        grid.blit(&Grid::line(" c"), 0, 0);
        assert_eq!(grid.rows().collect::<Vec<_>>(), ["ac"]);
    }

    #[test]
    fn centres_or_falls_back_to_text() {
        let drawing = Drawing::new(Some(Grid::line("xy")), "xy plain");
        assert_eq!(render(&drawing, 4, 3), ["    ", " xy ", "    "]);
        // Too narrow for the grid: one row of plain text instead.
        let wide = Drawing::new(Some(Grid::line("0123456789")), "short");
        assert_eq!(wide.height(Size::new(6, 3)), 1);
        assert_eq!(render(&wide, 6, 1)[0].trim(), "short");
    }
}
