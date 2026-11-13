//! Laying out an `Expr` as a grid of characters.
//!
//! Every piece has a math axis: the row a fraction bar or a minus sign sits
//! on. Pieces in a row line up on their axes, so `1 + 3/8` puts the plus
//! level with the fraction bar.

use super::{font, grid::Grid};
use crate::expr::Expr;

/// The axis of a full-size glyph: its middle row.
const AXIS: usize = 1;

/// A laid-out expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laid {
    pub grid: Grid,
    pub axis: usize,
}

pub fn layout(expr: &Expr) -> Laid {
    lay(expr, false)
}

/// `small` writes plain characters on one row, as exponents and anything
/// inside them are.
fn lay(expr: &Expr, small: bool) -> Laid {
    match expr {
        Expr::Text(s) if small => Laid {
            grid: Grid::line(s),
            axis: 0,
        },
        Expr::Text(s) => Laid {
            grid: Grid::text(s),
            axis: AXIS,
        },
        Expr::Row(parts) => {
            let parts: Vec<_> = parts.iter().map(|p| lay(p, small)).collect();
            row(&parts, if small { 0 } else { font::GAP })
        }
        Expr::Pow(base, exp) => pow(&lay(base, small), &lay(exp, true)),
        Expr::Frac(num, den) => frac(&lay(num, small), &lay(den, small)),
        Expr::Sqrt(inner) => sqrt(&lay(inner, small)),
    }
}

fn row(parts: &[Laid], gap: usize) -> Laid {
    let above = parts.iter().map(|p| p.axis).max().unwrap_or(0);
    let below = parts
        .iter()
        .map(|p| p.grid.height() - p.axis)
        .max()
        .unwrap_or(0);
    let gaps = gap * parts.len().saturating_sub(1);
    let width = parts.iter().map(|p| p.grid.width()).sum::<usize>() + gaps;
    let mut grid = Grid::new(width, above + below);
    let mut x = 0;
    for part in parts {
        grid.blit(&part.grid, x, above - part.axis);
        x += part.grid.width() + gap;
    }
    Laid { grid, axis: above }
}

/// The exponent sits to the right with its last row level with the top
/// row of the base.
fn pow(base: &Laid, exp: &Laid) -> Laid {
    let raise = exp.grid.height().saturating_sub(1);
    let width = base.grid.width() + exp.grid.width();
    let mut grid = Grid::new(width, raise + base.grid.height());
    grid.blit(&base.grid, 0, raise);
    grid.blit(&exp.grid, base.grid.width(), 0);
    Laid {
        grid,
        axis: raise + base.axis,
    }
}

/// Numerator and denominator centred over and under a bar one column
/// wider than either on each side.
fn frac(num: &Laid, den: &Laid) -> Laid {
    let width = num.grid.width().max(den.grid.width()) + 2;
    let bar = num.grid.height();
    let mut grid = Grid::new(width, bar + 1 + den.grid.height());
    grid.blit(&num.grid, (width - num.grid.width()) / 2, 0);
    (0..width).for_each(|x| grid.put(x, bar, '━'));
    grid.blit(&den.grid, (width - den.grid.width()) / 2, bar + 1);
    Laid { grid, axis: bar }
}

/// A radical: a short `╲` into a `╱` that climbs to an overline covering
/// the radicand.
fn sqrt(inner: &Laid) -> Laid {
    let h = inner.grid.height();
    // The climb takes one column per row, from the bottom row to row 1.
    let left = h + 1;
    let width = left + 1 + inner.grid.width() + 1;
    let mut grid = Grid::new(width, h + 1);
    grid.put(0, h, '╲');
    for row in 1..=h {
        grid.put(1 + h - row, row, '╱');
    }
    (left..width).for_each(|x| grid.put(x, 0, '━'));
    grid.blit(&inner.grid, left + 1, 1);
    Laid {
        grid,
        axis: inner.axis + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Expr {
        Expr::text(s)
    }

    fn art(laid: &Laid) -> Vec<String> {
        laid.grid.rows().map(|r| r.trim_end().to_owned()).collect()
    }

    #[test]
    fn digits_and_operators() {
        let laid = layout(&t("96 × 38"));
        assert_eq!(
            art(&laid),
            [
                "┏━┓ ┏━╸       ╺━┓ ┏━┓",
                "┗━┫ ┣━┓   ╳    ━┫ ┣━┫",
                "╺━┛ ┗━┛       ╺━┛ ┗━┛",
            ]
        );
        assert_eq!(laid.axis, 1);
    }

    #[test]
    fn exponent_is_small_and_raised() {
        let laid = layout(&Expr::pow(t("17"), t("2")));
        assert_eq!(art(&laid), ["╺┓  ╺━┓2", " ┃    ┃", "╺┻╸   ╹"]);
        assert_eq!(laid.axis, 1);
    }

    #[test]
    fn fraction_is_stacked_on_its_axis() {
        let laid = layout(&Expr::frac(t("3"), t("8")));
        assert_eq!(
            art(&laid),
            [" ╺━┓", "  ━┫", " ╺━┛", "━━━━━", " ┏━┓", " ┣━┫", " ┗━┛"]
        );
        assert_eq!(laid.axis, 3);
    }

    #[test]
    fn row_aligns_axes() {
        let row = Expr::Row(vec![t("1+"), Expr::frac(t("1"), t("4"))]);
        let laid = layout(&row);
        assert_eq!(laid.axis, 3);
        // The plus sign's bar is on the fraction bar's row.
        assert_eq!(art(&laid)[3], " ┃  ╺╋╸ ━━━━━");
    }

    #[test]
    fn root_has_an_overline() {
        let laid = layout(&Expr::sqrt(t("7")));
        assert_eq!(
            art(&laid),
            ["    ━━━━━", "   ╱ ╺━┓", "  ╱    ┃", "╲╱     ╹"]
        );
        assert_eq!(laid.axis, 2);
    }

    #[test]
    fn words_and_punctuation_close_up() {
        let laid = layout(&t("1,000 of 5"));
        assert_eq!(art(&laid)[1], " ┃  ┃ ┃ ┃ ┃ ┃ ┃   of   ┗━┓");
        assert_eq!(art(&laid)[2], "╺┻╸,┗━┛ ┗━┛ ┗━┛        ╺━┛");
    }
}
