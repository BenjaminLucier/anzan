//! Questions drawn at twice the height, for terminals that cannot scale
//! their own font.
//!
//! The pixels are octants: the Unicode block characters that split a
//! cell into two columns of four. A terminal cell is about twice as tall
//! as it is wide, so octant pixels are square, and a font seven pixels
//! tall fits in two rows of cells with a pixel to spare. That is small
//! enough to read as large text rather than as a picture of text.
//!
//! Octants are recent (Unicode 16). Terminals that draw block characters
//! themselves, as Ghostty and Kitty do, show them whatever the font;
//! elsewhere they depend on the font, so this is only used where it is
//! known to work or is asked for.

use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::Style,
    symbols::pixel::OCTANTS,
    widgets::Widget,
};

use super::bitmap::Bitmap;
use crate::expr::Expr;

/// Pixels in a cell, across and down.
const ACROSS: usize = 2;
const DOWN: usize = 4;

/// Height of every full-size glyph, in pixels.
const HEIGHT: usize = 7;

/// Height of the small glyphs exponents are written in.
const SMALL_HEIGHT: usize = 5;

/// Pixels between neighbouring glyphs, and between pieces of a row.
const GAP: usize = 1;
const WORD: usize = 4;

/// The rows of `c`, as `#` (on) and `.` (off), or `None` if the font
/// cannot draw it. Glyphs differ in width.
#[rustfmt::skip]
fn glyph(c: char) -> Option<[&'static str; HEIGHT]> {
    Some(match c {
        '0' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        '1' => ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."],
        '2' => [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
        '3' => ["####.", "....#", "....#", ".###.", "....#", "....#", "####."],
        '4' => ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
        '5' => ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
        '6' => ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."],
        '7' => ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
        '8' => [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
        '9' => [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."],
        '+' => [".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."],
        '−' | '-' => [".....", ".....", ".....", "#####", ".....", ".....", "....."],
        '×' => [".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "....."],
        '÷' => [".....", "..#..", ".....", "#####", ".....", "..#..", "....."],
        '.' => [".", ".", ".", ".", ".", "#", "#"],
        ',' => ["..", "..", "..", "..", "..", ".#", "#."],
        '%' => ["##..#", "##.#.", "...#.", "..#..", ".#...", ".#.##", "#..##"],
        '→' => ["......", "...#..", "....#.", "######", "....#.", "...#..", "......"],
        'o' => [".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###."],
        'f' => ["..##", ".#..", "####", ".#..", ".#..", ".#..", ".#.."],
        'b' => ["#....", "#....", "####.", "#...#", "#...#", "#...#", "####."],
        'p' => [".....", ".....", "####.", "#...#", "####.", "#....", "#...."],
        ' ' => ["...", "...", "...", "...", "...", "...", "..."],
        _ => return None,
    })
}

/// The rows of `c` at the size of an exponent.
#[rustfmt::skip]
fn small_glyph(c: char) -> Option<[&'static str; SMALL_HEIGHT]> {
    Some(match c {
        '0' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###", "..#", "###", "#..", "###"],
        '3' => ["###", "..#", "###", "..#", "###"],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "###", "..#", "###"],
        '6' => ["###", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", "..#", "..#", "..#"],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "###"],
        _ => return None,
    })
}

/// An expression as pixels, and the row its math axis is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub bitmap: Bitmap,
    pub axis: usize,
}

/// Draws `expr`, or returns `None` if it uses a character the font does
/// not have.
pub fn draw(expr: &Expr) -> Option<Picture> {
    match expr {
        // Spacing between pieces is the row's job.
        Expr::Text(s) => Some(Picture {
            bitmap: line(s.trim(), glyph)?,
            axis: HEIGHT / 2,
        }),
        Expr::Row(parts) => Some(row(&parts.iter().map(draw).collect::<Option<Vec<_>>>()?)),
        Expr::Pow(base, exp) => pow(&draw(base)?, exp),
        Expr::Frac(num, den) => Some(frac(&draw(num)?, &draw(den)?)),
        Expr::Sqrt(inner) => Some(sqrt(&draw(inner)?)),
    }
}

/// `text` in one line of glyphs, a gap apart.
fn line<const H: usize>(
    text: &str,
    glyph: impl Fn(char) -> Option<[&'static str; H]>,
) -> Option<Bitmap> {
    let glyphs: Vec<[&str; H]> = text.chars().map(glyph).collect::<Option<_>>()?;
    if glyphs.is_empty() {
        // Nothing, but as tall as a line, so that it still has an axis.
        return Some(Bitmap::new(0, H));
    }
    Some(Bitmap::from_glyphs(glyphs.iter().map(|g| &g[..]), GAP))
}

/// Pieces side by side, lined up on their axes.
fn row(parts: &[Picture]) -> Picture {
    let above = parts.iter().map(|p| p.axis).max().unwrap_or(0);
    let below = parts
        .iter()
        .map(|p| p.bitmap.height() - p.axis)
        .max()
        .unwrap_or(0);
    let width = parts.iter().map(|p| p.bitmap.width()).sum::<usize>()
        + WORD * parts.len().saturating_sub(1);
    let mut bitmap = Bitmap::new(width, above + below);
    let mut x = 0;
    for part in parts {
        bitmap.blit(&part.bitmap, x, above - part.axis);
        x += part.bitmap.width() + WORD;
    }
    Picture {
        bitmap,
        axis: above,
    }
}

/// The exponent in small digits, level with the top of the base.
fn pow(base: &Picture, exp: &Expr) -> Option<Picture> {
    let exp = line(&exp.to_string(), small_glyph)?;
    let height = base.bitmap.height().max(exp.height());
    let mut bitmap = Bitmap::new(base.bitmap.width() + GAP + exp.width(), height);
    bitmap.blit(&base.bitmap, 0, 0);
    bitmap.blit(&exp, base.bitmap.width() + GAP, 0);
    Some(Picture {
        bitmap,
        axis: base.axis,
    })
}

/// Numerator and denominator centred over and under a bar a pixel
/// longer than either at each end.
fn frac(num: &Picture, den: &Picture) -> Picture {
    let (num, den) = (&num.bitmap, &den.bitmap);
    let width = num.width().max(den.width()) + 2;
    let bar = num.height() + GAP;
    let mut bitmap = Bitmap::new(width, bar + 1 + GAP + den.height());
    bitmap.blit(num, (width - num.width()) / 2, 0);
    (0..width).for_each(|x| bitmap.set(x, bar));
    bitmap.blit(den, (width - den.width()) / 2, bar + 1 + GAP);
    Picture { bitmap, axis: bar }
}

/// A radical: a tick, a stroke leaning up to the right, and a bar over
/// what is under the root.
fn sqrt(inner: &Picture) -> Picture {
    const RADICAL: usize = 5;
    let height = inner.bitmap.height() + 1 + GAP;
    let mut bitmap = Bitmap::new(RADICAL + inner.bitmap.width(), height);
    bitmap.set(0, height - 4);
    bitmap.set(1, height - 3);
    for y in 0..height {
        bitmap.set(if y < height / 2 { 3 } else { 2 }, y);
    }
    (3..bitmap.width()).for_each(|x| bitmap.set(x, 0));
    bitmap.blit(&inner.bitmap, RADICAL, 1 + GAP);
    Picture {
        bitmap,
        axis: inner.axis + 1 + GAP,
    }
}

/// A picture drawn in octants, centred across its area, with its axis
/// on the same pixel row whatever the picture: the one that puts a
/// single line of text exactly in the two middle rows of cells.
pub struct Drawn<'a> {
    picture: &'a Picture,
    style: Style,
}

impl<'a> Drawn<'a> {
    pub fn new(picture: &'a Picture, style: Style) -> Self {
        Self { picture, style }
    }

    /// The pixel row the axis goes on in an area `rows` cells high.
    fn axis_row(rows: u16) -> usize {
        (usize::from(rows) * DOWN / 2 + HEIGHT / 2).saturating_sub(DOWN)
    }

    /// How far down the picture starts, if it fits in `area`.
    fn top(&self, area: Size) -> Option<usize> {
        let bitmap = &self.picture.bitmap;
        let top = Self::axis_row(area.height).checked_sub(self.picture.axis)?;
        let fits = bitmap.width() <= usize::from(area.width) * ACROSS
            && top + bitmap.height() <= usize::from(area.height) * DOWN;
        fits.then_some(top)
    }

    pub fn fits(&self, area: Size) -> bool {
        self.top(area).is_some()
    }
}

impl Widget for &Drawn<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Some(top) = self.top(area.as_size()) else {
            return;
        };
        let bitmap = &self.picture.bitmap;
        let left = (usize::from(area.width) * ACROSS - bitmap.width()) / 2;
        // A pixel of the area, which is off outside the picture.
        let pixel = |x: usize, y: usize| match (x.checked_sub(left), y.checked_sub(top)) {
            (Some(x), Some(y)) => bitmap.get(x, y),
            _ => false,
        };
        for row in 0..area.height {
            for col in 0..area.width {
                let (x, y) = (usize::from(col) * ACROSS, usize::from(row) * DOWN);
                // The table is in row-major order: top left is bit 0.
                let pattern = (0..ACROSS * DOWN)
                    .filter(|bit| pixel(x + bit % ACROSS, y + bit / ACROSS))
                    .fold(0, |pattern, bit| pattern | 1 << bit);
                if pattern != 0 {
                    buf[(area.x + col, area.y + row)]
                        .set_char(OCTANTS[pattern])
                        .set_style(self.style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::{generate::Settings, problem::Kind};

    fn t(s: &str) -> Expr {
        Expr::text(s)
    }

    fn art(picture: &Picture) -> Vec<String> {
        picture
            .bitmap
            .rows()
            .map(|r| r.trim_end().to_owned())
            .collect()
    }

    #[test]
    fn glyphs_are_well_formed() {
        for c in "0123456789+−-×÷.,%→ofbp ".chars() {
            let rows = glyph(c).unwrap_or_else(|| panic!("no glyph for {c:?}"));
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{c:?}");
        }
        for c in "0123456789".chars() {
            let rows = small_glyph(c).unwrap();
            assert!(rows.iter().all(|r| r.len() == 3), "{c:?}");
        }
        assert_eq!(glyph('-'), glyph('−'));
        assert_eq!(glyph('a'), None);
    }

    #[test]
    fn text_is_a_line_of_glyphs() {
        let picture = draw(&t(" 1 + 1 ")).unwrap();
        assert_eq!(
            art(&picture),
            [
                "  #                   #",
                " ##         #        ##",
                "  #         #         #",
                "  #       #####       #",
                "  #         #         #",
                "  #         #         #",
                " ###                 ###",
            ]
        );
        assert_eq!(picture.axis, 3);
        assert_eq!(draw(&t("2 apples")), None);
    }

    #[test]
    fn exponent_is_small_and_raised() {
        let picture = draw(&Expr::pow(t("7"), t("2"))).unwrap();
        assert_eq!(
            art(&picture),
            [
                "##### ###",
                "    #   #",
                "   #  ###",
                "  #   #",
                " #    ###",
                " #",
                " #",
            ]
        );
        assert_eq!(picture.axis, 3);
        // An exponent the small font cannot write is no picture at all.
        assert_eq!(draw(&Expr::pow(t("2"), t("−1"))), None);
    }

    #[test]
    fn fraction_is_stacked_on_its_bar() {
        let picture = draw(&Expr::frac(t("1"), t("7"))).unwrap();
        let rows = art(&picture);
        assert_eq!(rows.len(), 7 + 1 + 1 + 1 + 7);
        assert_eq!(rows[picture.axis], "#######");
        assert_eq!((rows[7].as_str(), rows[9].as_str()), ("", ""));
        assert_eq!(rows[10], " #####");
    }

    #[test]
    fn root_has_a_radical_and_a_bar() {
        let picture = draw(&Expr::sqrt(t("7"))).unwrap();
        assert_eq!(
            art(&picture),
            [
                "   #######",
                "   #",
                "   # #####",
                "   #     #",
                "  #     #",
                "# #    #",
                " ##   #",
                "  #   #",
                "  #   #",
            ]
        );
        assert_eq!(picture.axis, 5);
    }

    fn render(picture: &Picture, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        Drawn::new(picture, Style::new()).render(area, &mut buf);
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn a_line_of_text_fills_the_two_middle_rows() {
        let picture = draw(&t("1")).unwrap();
        let rows = render(&picture, 5, 6);
        let blank = " ".repeat(5);
        assert_eq!(rows[..2], [blank.clone(), blank.clone()]);
        assert_eq!(rows[4..], [blank.clone(), blank]);
        // Five pixels wide, so three cells, centred with a cell each side.
        for row in &rows[2..4] {
            assert!(row.starts_with(' ') && row.ends_with(' ') && row.trim().chars().count() >= 2);
        }
        // The top-left pixels of a 1 are off, on, on over off, on, on.
        assert_eq!(rows[2].chars().nth(1), Some(OCTANTS[0b0000_1000]));
    }

    #[test]
    fn what_does_not_fit_is_not_drawn() {
        let picture = draw(&t("12345")).unwrap();
        let drawn = Drawn::new(&picture, Style::new());
        assert!(drawn.fits(Size::new(15, 2)));
        assert!(!drawn.fits(Size::new(14, 2)));
        assert!(!drawn.fits(Size::new(15, 1)));
        assert!(
            render(&picture, 14, 2)
                .iter()
                .all(|row| row.trim().is_empty())
        );
    }

    /// Every question the default settings ask can be drawn, fits in the
    /// rows set aside for the tallest, and has its axis on one row.
    #[test]
    fn every_question_draws_in_six_rows() {
        let settings = Settings::default();
        let mut rng = StdRng::seed_from_u64(0);
        for &kind in Kind::ALL {
            for _ in 0..50 {
                let problem = settings.generate_kind(kind, &mut rng);
                let picture = draw(&problem.expr()).unwrap_or_else(|| panic!("{problem}"));
                let drawn = Drawn::new(&picture, Style::new());
                assert!(drawn.fits(Size::new(200, 6)), "{problem}");
            }
        }
    }
}
