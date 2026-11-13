//! A font drawn with box-drawing characters, three rows tall.
//!
//! Heavy strokes (`━ ┃ ┏ ┓ ┗ ┛ ┣ ┫ ╋`) join cleanly in any decent
//! terminal font, so digits read as smooth lines rather than blocks of
//! pixels, and stay compact. Bold makes them heavier still.

/// Rows in every glyph.
pub const HEIGHT: usize = 3;

/// Columns between neighbouring glyphs.
pub const GAP: usize = 1;

/// The rows of `c`, all the same width, or `None` for characters without
/// a glyph, such as the letters in `3 of 5`.
#[rustfmt::skip]
pub fn glyph(c: char) -> Option<[&'static str; HEIGHT]> {
    let rows: [&str; HEIGHT] = match c {
        '0' => ["┏━┓",
                "┃ ┃",
                "┗━┛"],
        '1' => ["╺┓ ",
                " ┃ ",
                "╺┻╸"],
        '2' => ["╺━┓",
                "┏━┛",
                "┗━╸"],
        '3' => ["╺━┓",
                " ━┫",
                "╺━┛"],
        '4' => ["╻ ╻",
                "┗━┫",
                "  ╹"],
        '5' => ["┏━╸",
                "┗━┓",
                "╺━┛"],
        '6' => ["┏━╸",
                "┣━┓",
                "┗━┛"],
        '7' => ["╺━┓",
                "  ┃",
                "  ╹"],
        '8' => ["┏━┓",
                "┣━┫",
                "┗━┛"],
        '9' => ["┏━┓",
                "┗━┫",
                "╺━┛"],
        '+' => [" ╻ ",
                "╺╋╸",
                " ╹ "],
        '−' | '-' => ["   ",
                      "╺━╸",
                      "   "],
        '×' => [" ",
                "╳",
                " "],
        '÷' => [" • ",
                "╺━╸",
                " • "],
        '=' => ["   ",
                "═══",
                "   "],
        '.' => [" ",
                " ",
                "•"],
        ',' => [" ",
                " ",
                ","],
        '%' => ["• ╱",
                " ╱ ",
                "╱ •"],
        ' ' => [" ",
                " ",
                " "],
        '→' => ["   ",
                "╺━▸",
                "   "],
        _ => return None,
    };
    Some(rows)
}

/// Whether `c` sits tight against its neighbours instead of a gap apart.
pub fn is_tight(c: char) -> bool {
    matches!(c, '.' | ',')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_rows_are_even() {
        for c in "0123456789+−-×÷=.,%→ ".chars() {
            let rows = glyph(c).unwrap();
            let width = rows[0].chars().count();
            assert!(rows.iter().all(|r| r.chars().count() == width), "{c:?}");
        }
    }
}
