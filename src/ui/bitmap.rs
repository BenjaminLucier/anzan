//! Monochrome bitmaps, such as the kanji, and a widget that draws one
//! as large as fits.
//!
//! A terminal has no font sizes, so big text is drawn out of block
//! characters. Terminal cells are about twice as tall as they are wide,
//! which gives three useful scales:
//!
//! - `Large`: each pixel is two cells side by side, `██`, so pixels are
//!   square.
//! - `Medium`: each pixel is one cell, so pixels are tall and narrow.
//! - `Small`: each cell holds two pixels, one above the other, using the
//!   half blocks `▀` and `▄`, so pixels are square again.

use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    text::Line,
    widgets::Widget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    width: usize,
    height: usize,
    pixels: Vec<bool>,
}

impl Bitmap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![false; width * height],
        }
    }

    /// Lays glyphs out left to right, `gap` pixels apart, aligned at the
    /// top. Each glyph is rows of `#` (on) and `.` (off).
    pub fn from_glyphs<'a>(glyphs: impl IntoIterator<Item = &'a [&'a str]>, gap: usize) -> Self {
        let glyphs: Vec<_> = glyphs.into_iter().collect();
        let width =
            glyphs.iter().map(|g| g[0].len()).sum::<usize>() + gap * glyphs.len().saturating_sub(1);
        let height = glyphs.iter().map(|g| g.len()).max().unwrap_or(0);
        let mut bitmap = Self::new(width, height);
        let mut x0 = 0;
        for rows in glyphs {
            for (y, row) in rows.iter().enumerate() {
                for (x, pixel) in row.bytes().enumerate() {
                    if pixel == b'#' {
                        bitmap.set(x0 + x, y);
                    }
                }
            }
            x0 += rows[0].len() + gap;
        }
        bitmap
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Copies the pixels that are on in `other`, with its top left corner
    /// at `(x, y)`. It must fit.
    pub fn blit(&mut self, other: &Bitmap, x: usize, y: usize) {
        for oy in 0..other.height {
            for ox in 0..other.width {
                if other.get(ox, oy) {
                    self.set(x + ox, y + oy);
                }
            }
        }
    }

    /// Each row as `#` for on and a space for off, for tests and debugging.
    pub fn rows(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.height).map(|y| {
            (0..self.width)
                .map(|x| if self.get(x, y) { '#' } else { ' ' })
                .collect()
        })
    }

    /// Whether the pixel at `(x, y)` is on. Pixels outside are off.
    pub fn get(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.pixels[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize) {
        self.pixels[y * self.width + x] = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Large,
    Medium,
    Small,
}

impl Scale {
    /// Largest first.
    pub const ALL: &[Scale] = &[Scale::Large, Scale::Medium, Scale::Small];

    /// The scales whose pixels are square, largest first.
    pub const SQUARE: &[Scale] = &[Scale::Large, Scale::Small];

    /// The cells a bitmap takes up at this scale.
    fn size(self, bitmap: &Bitmap) -> Size {
        let (w, h) = (bitmap.width, bitmap.height);
        let (w, h) = match self {
            Scale::Large => (2 * w, h),
            Scale::Medium => (w, h),
            Scale::Small => (w, h.div_ceil(2)),
        };
        Size::new(w as u16, h as u16)
    }
}

/// A bitmap drawn at the largest scale that fits, or ordinary text if
/// even the smallest does not.
pub struct BigText<'a> {
    text: &'a str,
    bitmap: Option<Bitmap>,
    scales: &'static [Scale],
}

impl<'a> BigText<'a> {
    /// Any bitmap, with `text` to show if it does not fit.
    pub fn bitmap(bitmap: Option<Bitmap>, text: &'a str) -> Self {
        Self {
            text,
            bitmap,
            scales: Scale::ALL,
        }
    }

    /// Limits the scales it may be drawn at.
    pub fn scales(mut self, scales: &'static [Scale]) -> Self {
        self.scales = scales;
        self
    }

    /// The scale it would be drawn at in `area`, and the cells it would
    /// take up. `None` means it would be drawn as ordinary text.
    pub fn fit(&self, area: Size) -> Option<(Scale, Size)> {
        let bitmap = self.bitmap.as_ref()?;
        self.scales
            .iter()
            .map(|&scale| (scale, scale.size(bitmap)))
            .find(|(_, size)| size.width <= area.width && size.height <= area.height)
    }

    /// Rows needed in `area`.
    pub fn height(&self, area: Size) -> u16 {
        self.fit(area).map_or(1, |(_, size)| size.height)
    }
}

impl Widget for &BigText<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (Some(bitmap), Some((scale, size))) = (&self.bitmap, self.fit(area.as_size())) else {
            let line = Line::from(self.text).centered();
            line.render(
                area.centered_vertically(ratatui::layout::Constraint::Length(1)),
                buf,
            );
            return;
        };
        let x0 = area.x + (area.width - size.width) / 2;
        let y0 = area.y + (area.height - size.height) / 2;
        for row in 0..size.height {
            for col in 0..size.width {
                let (x, y) = (usize::from(col), usize::from(row));
                let symbol = match scale {
                    Scale::Large => full(bitmap.get(x / 2, y)),
                    Scale::Medium => full(bitmap.get(x, y)),
                    Scale::Small => half(bitmap.get(x, 2 * y), bitmap.get(x, 2 * y + 1)),
                };
                if let Some(symbol) = symbol {
                    buf[(x0 + col, y0 + row)].set_symbol(symbol);
                }
            }
        }
    }
}

fn full(on: bool) -> Option<&'static str> {
    on.then_some("█")
}

fn half(top: bool, bottom: bool) -> Option<&'static str> {
    match (top, bottom) {
        (true, true) => Some("█"),
        (true, false) => Some("▀"),
        (false, true) => Some("▄"),
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    const ONE: [&str; 7] = [
        "..#..",
        ".##..",
        "..#..",
        "..#..",
        "..#..",
        "..#..",
        ".###.",
    ];

    /// Ones in a row, as a bitmap with `text` to fall back on.
    fn ones(text: &str) -> BigText<'_> {
        let glyphs = text.chars().map(|_| &ONE[..]);
        BigText::bitmap(Some(Bitmap::from_glyphs(glyphs, 1)), text)
    }

    fn render(text: &str, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        (&ones(text)).render(area, &mut buf);
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn text_width_includes_gaps() {
        let bitmap = Bitmap::from_glyphs([&ONE[..], &ONE[..]], 1);
        assert_eq!((bitmap.width(), bitmap.height()), (11, 7));
    }

    #[test]
    fn picks_the_largest_scale_that_fits() {
        let big = ones("1");
        let fit = |w, h| big.fit(Size::new(w, h)).map(|(scale, _)| scale);
        assert_eq!(fit(10, 7), Some(Scale::Large));
        assert_eq!(fit(9, 7), Some(Scale::Medium));
        assert_eq!(fit(10, 6), Some(Scale::Small));
        assert_eq!(fit(4, 4), None);
    }

    #[test]
    fn draws_large() {
        assert_eq!(
            render("1", 10, 7),
            [
                "    ██    ",
                "  ████    ",
                "    ██    ",
                "    ██    ",
                "    ██    ",
                "    ██    ",
                "  ██████  ",
            ]
        );
    }

    #[test]
    fn draws_small_with_half_blocks() {
        assert_eq!(render("1", 5, 4), [" ▄█  ", "  █  ", "  █  ", " ▀▀▀ "]);
    }

    #[test]
    fn centres_in_a_larger_area() {
        let rows = render("1", 7, 6);
        // Medium is too tall for 6 rows, so Small (4 rows) is centred.
        assert_eq!(rows[0], "       ");
        assert_eq!(rows[1], "  ▄█   ");
    }

    #[test]
    fn falls_back_to_plain_text() {
        assert_eq!(render("12", 4, 1), [" 12 "]);
    }
}
