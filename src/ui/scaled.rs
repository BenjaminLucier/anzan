//! Text at twice the size, in the terminal's own font.
//!
//! A terminal has one font size, but some can be asked to draw a run of
//! text larger: the text sizing protocol that Kitty introduced and
//! Ghostty also speaks. Each character then covers a block of cells two
//! wide and two high.
//!
//! Ratatui's buffer has no notion of that, so scaled text is not part of
//! a frame. The screen leaves room for it, blank, and a [`Painter`]
//! writes it over the frame once the frame is drawn, and wipes it when
//! it changes or goes away.

use std::io::{self, Write};

use ratatui::{
    crossterm::{
        cursor::{self, MoveTo, RestorePosition, SavePosition},
        queue,
        terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
    },
    layout::{Rect, Size},
    style::{Color, Modifier, Style},
};

/// How many cells each character covers, each way.
pub const SCALE: u16 = 2;

/// Text to draw at `SCALE` times the size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scaled {
    /// The cells it covers: `SCALE` rows for each of `rows`.
    pub area: Rect,
    /// Rows of text, each as wide as the area once scaled.
    pub rows: Vec<String>,
    pub style: Style,
}

/// Whether the terminal draws scaled text. Asks it: writes a scaled
/// space and sees how far the cursor moved. Call in raw mode, before
/// anything is drawn, and clear the screen afterwards.
pub fn probe() -> bool {
    let moved = || -> io::Result<u16> {
        let (before, _) = cursor::position()?;
        let mut out = io::stdout();
        write_scaled(&mut out, " ")?;
        out.flush()?;
        let (after, _) = cursor::position()?;
        Ok(after.saturating_sub(before))
    };
    moved().is_ok_and(|cells| cells == SCALE)
}

/// `text` at `SCALE` times the size, from the cursor on.
fn write_scaled(out: &mut impl Write, text: &str) -> io::Result<()> {
    write!(out, "\x1b]66;s={SCALE};{text}\x07")
}

/// Sets colours and boldness to `style`, from a clean slate.
fn write_style(out: &mut impl Write, style: Style) -> io::Result<()> {
    write!(out, "\x1b[0m")?;
    if style.add_modifier.contains(Modifier::BOLD) {
        write!(out, "\x1b[1m")?;
    }
    for (color, base) in [(style.fg, 38), (style.bg, 48)] {
        match color {
            Some(Color::Rgb(r, g, b)) => write!(out, "\x1b[{base};2;{r};{g};{b}m")?,
            Some(Color::Indexed(i)) => write!(out, "\x1b[{base};5;{i}m")?,
            // The terminal's own colours, which is what a clean slate is.
            _ => {}
        }
    }
    Ok(())
}

/// Keeps scaled text on screen in step with what should be there.
#[derive(Debug, Default)]
pub struct Painter {
    shown: Option<Scaled>,
    /// The size of the terminal when `shown` was painted.
    size: Size,
}

impl Painter {
    /// Makes the screen show `want`, on a terminal of `size` whose frame
    /// has just been drawn. Writes nothing when nothing has changed. The
    /// cursor is left where the frame put it.
    pub fn paint(
        &mut self,
        want: Option<Scaled>,
        size: Size,
        out: &mut impl Write,
    ) -> io::Result<()> {
        // A resize makes ratatui clear the screen, scaled text and all.
        if size != self.size {
            self.shown = None;
            self.size = size;
        }
        if want == self.shown {
            return Ok(());
        }
        queue!(out, BeginSynchronizedUpdate, SavePosition)?;
        if let Some(old) = self.shown.take() {
            // Writing over any cell of a scaled character removes all of it.
            write_style(out, Style::new().bg(old.style.bg.unwrap_or(Color::Reset)))?;
            let blank = " ".repeat(usize::from(old.area.width));
            for y in old.area.rows().map(|row| row.y) {
                queue!(out, MoveTo(old.area.x, y))?;
                out.write_all(blank.as_bytes())?;
            }
        }
        if let Some(new) = &want {
            write_style(out, new.style)?;
            for (i, row) in new.rows.iter().enumerate() {
                queue!(out, MoveTo(new.area.x, new.area.y + SCALE * i as u16))?;
                write_scaled(out, row)?;
            }
        }
        write!(out, "\x1b[0m")?;
        queue!(out, RestorePosition, EndSynchronizedUpdate)?;
        out.flush()?;
        self.shown = want;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size::new(80, 24);

    fn question(text: &str) -> Scaled {
        Scaled {
            area: Rect::new(10, 4, SCALE * text.chars().count() as u16, SCALE),
            rows: vec![text.to_owned()],
            style: Style::new()
                .fg(Color::Rgb(1, 2, 3))
                .bg(Color::Indexed(7))
                .add_modifier(Modifier::BOLD),
        }
    }

    fn paint(painter: &mut Painter, want: Option<Scaled>, size: Size) -> String {
        let mut out = Vec::new();
        painter.paint(want, size, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn paints_scaled_text_where_it_belongs_in_its_style() {
        let mut painter = Painter::default();
        let out = paint(&mut painter, Some(question("8 × 39")), SIZE);
        // Row 5, column 11, counting from 1 as terminals do.
        assert!(out.contains("\x1b[5;11H\x1b]66;s=2;8 × 39\x07"), "{out:?}");
        assert!(
            out.contains("\x1b[1m\x1b[38;2;1;2;3m\x1b[48;5;7m"),
            "{out:?}"
        );
        // The cursor goes back to the answer box, and styles do not leak.
        assert!(out.contains("\x1b[0m\x1b8"), "{out:?}");
    }

    #[test]
    fn stacked_rows_are_a_scaled_row_apart() {
        let mut painter = Painter::default();
        let fraction = Scaled {
            area: Rect::new(0, 0, 6, 6),
            rows: vec![" 3 ".into(), "───".into(), " 8 ".into()],
            style: Style::new(),
        };
        let out = paint(&mut painter, Some(fraction), SIZE);
        for at in [
            "\x1b[1;1H\x1b]66;s=2; 3 ",
            "\x1b[3;1H\x1b]66;s=2;───",
            "\x1b[5;1H\x1b]66;s=2; 8 ",
        ] {
            assert!(out.contains(at), "{at:?} in {out:?}");
        }
    }

    #[test]
    fn writes_nothing_until_something_changes() {
        let mut painter = Painter::default();
        assert_eq!(paint(&mut painter, None, SIZE), "");
        assert_ne!(paint(&mut painter, Some(question("2 + 2")), SIZE), "");
        assert_eq!(paint(&mut painter, Some(question("2 + 2")), SIZE), "");
    }

    #[test]
    fn wipes_the_old_text_before_the_new_and_when_it_goes() {
        let mut painter = Painter::default();
        paint(&mut painter, Some(question("12 + 34")), SIZE);
        // Both rows the old text covered, in its background, then the new.
        let blank = " ".repeat(14);
        let wipe = format!("\x1b[48;5;7m\x1b[5;11H{blank}\x1b[6;11H{blank}");
        let out = paint(&mut painter, Some(question("7 × 8")), SIZE);
        let (wiped, painted) = (out.find(&wipe), out.find("s=2;7 × 8"));
        assert!(wiped.is_some() && wiped < painted, "{out:?}");
        let out = paint(&mut painter, None, SIZE);
        assert!(
            out.contains(&format!("\x1b[5;11H{}", " ".repeat(10))),
            "{out:?}"
        );
        assert!(!out.contains("]66;"), "{out:?}");
        assert_eq!(paint(&mut painter, None, SIZE), "");
    }

    #[test]
    fn a_resize_repaints_without_wiping() {
        let mut painter = Painter::default();
        paint(&mut painter, Some(question("2 + 2")), SIZE);
        // The screen was cleared, so the same text has to be painted
        // again, and there is nothing of the old to wipe.
        let out = paint(&mut painter, Some(question("2 + 2")), Size::new(100, 30));
        assert!(out.contains("s=2;2 + 2"), "{out:?}");
        assert!(!out.contains("  "), "{out:?}");
    }
}
