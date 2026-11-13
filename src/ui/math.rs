//! Laying out an `Expr` as a bitmap.
//!
//! Every piece has a math axis: the row a fraction bar or a minus sign sits
//! on. Pieces in a row line up on their axes, so `1 + 3/8` puts the plus
//! level with the fraction bar.

use super::{bitmap::Bitmap, font};
use crate::expr::Expr;

/// The axis of a full-size glyph: the row of the bar in `−` and `+`.
const AXIS: usize = 3;

/// A laid-out expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laid {
    pub bitmap: Bitmap,
    pub axis: usize,
}

/// Lays out `expr`, or returns `None` if the font cannot draw part of it.
pub fn layout(expr: &Expr) -> Option<Laid> {
    lay(expr, false)
}

/// `small` draws in the exponent font, as exponents and anything inside
/// them are.
fn lay(expr: &Expr, small: bool) -> Option<Laid> {
    match expr {
        Expr::Text(s) if small => Some(Laid {
            bitmap: Bitmap::small_text(s)?,
            axis: font::SMALL_HEIGHT / 2,
        }),
        Expr::Text(s) => Some(Laid {
            bitmap: Bitmap::text(s)?,
            axis: AXIS,
        }),
        Expr::Row(parts) => {
            let parts: Vec<_> = parts.iter().map(|p| lay(p, small)).collect::<Option<_>>()?;
            Some(row(&parts))
        }
        Expr::Pow(base, exp) => {
            let (base, exp) = (lay(base, small)?, lay(exp, true)?);
            Some(pow(&base, &exp))
        }
        Expr::Frac(num, den) => Some(frac(&lay(num, small)?, &lay(den, small)?)),
        Expr::Sqrt(inner) => Some(sqrt(&lay(inner, small)?)),
    }
}

fn row(parts: &[Laid]) -> Laid {
    let above = parts.iter().map(|p| p.axis).max().unwrap_or(0);
    let below = parts
        .iter()
        .map(|p| p.bitmap.height() - p.axis)
        .max()
        .unwrap_or(0);
    let gaps = font::GAP * parts.len().saturating_sub(1);
    let width = parts.iter().map(|p| p.bitmap.width()).sum::<usize>() + gaps;
    let mut bitmap = Bitmap::new(width, above + below);
    let mut x = 0;
    for part in parts {
        bitmap.blit(&part.bitmap, x, above - part.axis);
        x += part.bitmap.width() + font::GAP;
    }
    Laid {
        bitmap,
        axis: above,
    }
}

/// The exponent sits up and to the right, its lower edge two rows into the
/// base.
fn pow(base: &Laid, exp: &Laid) -> Laid {
    const OVERLAP: usize = 2;
    let raise = exp.bitmap.height().saturating_sub(OVERLAP);
    let width = base.bitmap.width() + font::GAP + exp.bitmap.width();
    let mut bitmap = Bitmap::new(width, raise + base.bitmap.height());
    bitmap.blit(&base.bitmap, 0, raise);
    bitmap.blit(&exp.bitmap, base.bitmap.width() + font::GAP, 0);
    Laid {
        bitmap,
        axis: raise + base.axis,
    }
}

/// Numerator and denominator centred over a bar one pixel wider than
/// either on each side, with a blank row above and below the bar.
fn frac(num: &Laid, den: &Laid) -> Laid {
    let inner = num.bitmap.width().max(den.bitmap.width());
    let width = inner + 2;
    let bar = num.bitmap.height() + 1;
    let mut bitmap = Bitmap::new(width, bar + 2 + den.bitmap.height());
    bitmap.blit(&num.bitmap, (width - num.bitmap.width()) / 2, 0);
    (0..width).for_each(|x| bitmap.set(x, bar));
    bitmap.blit(&den.bitmap, (width - den.bitmap.width()) / 2, bar + 2);
    Laid { bitmap, axis: bar }
}

/// A radical sign whose overline covers the whole radicand.
///
/// The long stroke climbs in three steps from the bottom of the vee to the
/// overline. A short tick leads into the vee from the left.
fn sqrt(inner: &Laid) -> Laid {
    /// Columns before the radicand: the sign, then a blank one.
    const SIGN: usize = 6;
    let height = inner.bitmap.height() + 2;
    let width = SIGN + inner.bitmap.width() + 1;
    let mut bitmap = Bitmap::new(width, height);
    let step = height.div_ceil(3);
    for y in 0..height {
        bitmap.set(4 - (y / step).min(2), y);
    }
    bitmap.set(0, height - 4);
    bitmap.set(1, height - 3);
    bitmap.set(1, height - 2);
    (4..width).for_each(|x| bitmap.set(x, 0));
    bitmap.blit(&inner.bitmap, SIGN, 2);
    Laid {
        bitmap,
        axis: inner.axis + 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Expr {
        Expr::text(s)
    }

    fn art(laid: &Laid) -> Vec<String> {
        laid.bitmap.rows().collect()
    }

    #[test]
    fn exponent_is_small_and_raised() {
        let laid = layout(&Expr::pow(t("7"), t("2"))).unwrap();
        assert_eq!(
            art(&laid),
            [
                "      ###",
                "        #",
                "      ###",
                "##### #  ",
                "    # ###",
                "   #     ",
                "  #      ",
                " #       ",
                " #       ",
                " #       ",
            ]
        );
        assert_eq!(laid.axis, 3 + 3);
    }

    #[test]
    fn fraction_is_stacked_on_its_axis() {
        let laid = layout(&Expr::frac(t("1"), t("4"))).unwrap();
        assert_eq!(laid.axis, 8);
        assert_eq!(laid.bitmap.height(), 7 + 1 + 1 + 1 + 7);
        assert_eq!(art(&laid)[8], "#######");
        assert_eq!(art(&laid)[7], "       ");
        assert_eq!(art(&laid)[9], "       ");
    }

    #[test]
    fn row_aligns_axes() {
        let row = Expr::Row(vec![t("1+"), Expr::frac(t("1"), t("4"))]);
        let laid = layout(&row).unwrap();
        // The plus sign's bar is on the fraction bar's row.
        assert_eq!(laid.axis, 8);
        let bar_row = &art(&laid)[8];
        assert!(bar_row.starts_with("  #   #####"), "{bar_row:?}");
    }

    #[test]
    fn root_has_an_overline() {
        let laid = layout(&Expr::sqrt(t("1"))).unwrap();
        let rows = art(&laid);
        assert_eq!(rows.len(), 9);
        assert_eq!(rows[0], "    ########");
        assert_eq!(rows[8], "  #    ###  ");
        assert_eq!(laid.axis, 5);
    }

    #[test]
    fn unknown_characters_fail() {
        assert_eq!(layout(&t("x")), None);
    }
}
