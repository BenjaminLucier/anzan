//! Laying out an `Expr` in ordinary text, on as many rows as it needs.
//!
//! Most questions are one line, such as `96 × 38` or `17²`. Fractions
//! stack over a bar, and every piece has a math axis, the row a fraction
//! bar sits on, so that `1 + 3/8` puts the plus level with the bar.

use super::grid::Grid;
use crate::expr::{Expr, superscript};

/// A laid-out expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laid {
    pub grid: Grid,
    pub axis: usize,
}

pub fn layout(expr: &Expr) -> Laid {
    match expr {
        // Spacing between pieces is the row's job.
        Expr::Text(s) => Laid {
            grid: Grid::line(s.trim()),
            axis: 0,
        },
        Expr::Row(parts) => row(&parts.iter().map(layout).collect::<Vec<_>>()),
        Expr::Pow(base, exp) => pow(&layout(base), exp),
        Expr::Frac(num, den) => frac(&layout(num), &layout(den)),
        Expr::Sqrt(inner) => sqrt(&layout(inner)),
    }
}

/// Pieces side by side, a space apart, lined up on their axes.
fn row(parts: &[Laid]) -> Laid {
    let above = parts.iter().map(|p| p.axis).max().unwrap_or(0);
    let below = parts
        .iter()
        .map(|p| p.grid.height() - p.axis)
        .max()
        .unwrap_or(0);
    let width = parts.iter().map(|p| p.grid.width()).sum::<usize>() + parts.len().saturating_sub(1);
    let mut grid = Grid::new(width, above + below);
    let mut x = 0;
    for part in parts {
        grid.blit(&part.grid, x, above - part.axis);
        x += part.grid.width() + 1;
    }
    Laid { grid, axis: above }
}

/// Superscript digits where Unicode has them, as in `17²`; otherwise the
/// exponent on the row above.
fn pow(base: &Laid, exp: &Expr) -> Laid {
    let (exp, raise) = match superscript(&exp.to_string()) {
        Some(sup) => (Grid::line(&sup), 0),
        None => {
            let exp = layout(exp).grid;
            let raise = exp.height();
            (exp, raise)
        }
    };
    let mut grid = Grid::new(base.grid.width() + exp.width(), raise + base.grid.height());
    grid.blit(&base.grid, 0, raise);
    grid.blit(&exp, base.grid.width(), 0);
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
    grid.blit(&num.grid, (width - num.grid.width()).div_ceil(2), 0);
    (0..width).for_each(|x| grid.put(x, bar, '─'));
    grid.blit(&den.grid, (width - den.grid.width()).div_ceil(2), bar + 1);
    Laid { grid, axis: bar }
}

/// `√` with an overline, drawn with underscores on the row above, over
/// the radicand.
fn sqrt(inner: &Laid) -> Laid {
    let h = inner.grid.height();
    let mut grid = Grid::new(1 + inner.grid.width(), h + 1);
    grid.put(0, h, '√');
    (1..grid.width()).for_each(|x| grid.put(x, 0, '_'));
    grid.blit(&inner.grid, 1, 1);
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
    fn plain_questions_are_one_line() {
        let laid = layout(&t("96 × 38"));
        assert_eq!(art(&laid), ["96 × 38"]);
        assert_eq!(laid.axis, 0);
    }

    #[test]
    fn exponents_are_superscript() {
        assert_eq!(art(&layout(&Expr::pow(t("17"), t("2")))), ["17²"]);
        let row = Expr::Row(vec![t("1,000 × "), Expr::pow(t("1.1"), t("3"))]);
        assert_eq!(art(&layout(&row)), ["1,000 × 1.1³"]);
        // Without superscript characters, the exponent goes up a row.
        assert_eq!(art(&layout(&Expr::pow(t("2"), t("n")))), [" n", "2"]);
    }

    #[test]
    fn fraction_is_stacked_on_its_axis() {
        let laid = layout(&Expr::frac(t("3"), t("16")));
        assert_eq!(art(&laid), ["  3", "────", " 16"]);
        assert_eq!(laid.axis, 1);
    }

    #[test]
    fn row_aligns_axes() {
        let row = Expr::Row(vec![
            Expr::frac(t("1"), t("4")),
            t("→ +30,"),
            Expr::frac(t("3"), t("4")),
            t("→ −8"),
        ]);
        assert_eq!(
            art(&layout(&row)),
            [" 1          3", "─── → +30, ─── → −8", " 4          4"]
        );
    }

    #[test]
    fn root_has_an_overline() {
        assert_eq!(art(&layout(&Expr::sqrt(t("50")))), [" __", "√50"]);
    }
}
