//! Colour themes.
//!
//! A theme is either the terminal's own sixteen colours, so whatever theme
//! the terminal uses shows through, or a fixed palette in the base16
//! layout: sixteen slots, from the background (`base00`) through text
//! (`base05`) to eight accents (`base08`–`base0F`). That is the format
//! of the large collection of schemes at
//! <https://github.com/tinted-theming/schemes>, so any of those can be
//! dropped into the themes folder as a `.yaml` file.
//!
//! The palettes bundled here, and their licenses, are listed in
//! `THEMES.md`.

use std::{fs, path::Path};

use ratatui::style::{Color, Modifier, Style};

/// The name of the theme that uses the terminal's own colours.
pub const TERMINAL: &str = "terminal";

/// Sixteen colours in the base16 layout.
pub type Palette = [u32; 16];

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    /// `None` uses the terminal's own colours.
    palette: Option<Palette>,
    /// Whether the terminal shows 24-bit colour. Without it, palette
    /// colours are rounded to the nearest of the standard 256.
    truecolor: bool,
}

impl Theme {
    pub fn terminal() -> Self {
        Self {
            name: TERMINAL.into(),
            palette: None,
            truecolor: true,
        }
    }

    pub fn palette(name: &str, palette: Palette) -> Self {
        Self {
            name: name.into(),
            palette: Some(palette),
            truecolor: true,
        }
    }

    /// Rounds palette colours to the standard 256 unless `truecolor`.
    pub fn with_truecolor(mut self, truecolor: bool) -> Self {
        self.truecolor = truecolor;
        self
    }

    /// Whether the background is light, if the theme has its own.
    pub fn is_light(&self) -> Option<bool> {
        self.palette.map(|p| luminance(p[0]) > 0.5)
    }

    fn slot(&self, i: usize) -> Option<Color> {
        let rgb = self.palette?[i];
        let (r, g, b) = ((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
        Some(if self.truecolor {
            Color::Rgb(r, g, b)
        } else {
            Color::Indexed(nearest_256(r, g, b))
        })
    }

    /// `slot` as the foreground, or `fallback` with the terminal theme.
    fn fg(&self, slot: usize, fallback: Style) -> Style {
        self.slot(slot).map_or(fallback, |c| Style::new().fg(c))
    }

    /// The whole screen: background and ordinary text.
    pub fn base(&self) -> Style {
        match (self.slot(0), self.slot(5)) {
            (Some(bg), Some(fg)) => Style::new().bg(bg).fg(fg),
            _ => Style::new(),
        }
    }

    /// Secondary text: hints, labels, units.
    pub fn muted(&self) -> Style {
        self.fg(4, Style::new().add_modifier(Modifier::DIM))
    }

    /// Panel borders, brighter and bold around the focused panel.
    pub fn border(&self, focused: bool) -> Style {
        if focused {
            self.fg(0xD, Style::new().fg(Color::Blue))
                .add_modifier(Modifier::BOLD)
        } else {
            self.fg(3, Style::new().add_modifier(Modifier::DIM))
        }
    }

    /// The selected row of a focused list.
    pub fn selected(&self) -> Style {
        match (self.slot(2), self.slot(5)) {
            (Some(bg), Some(fg)) => Style::new().bg(bg).fg(fg).add_modifier(Modifier::BOLD),
            _ => Style::new().add_modifier(Modifier::REVERSED),
        }
    }

    /// Emphasis that is not good or bad news: the kanji, the current mode.
    pub fn accent(&self) -> Style {
        self.fg(0xE, Style::new().fg(Color::Magenta))
    }

    /// Headings and the question itself.
    pub fn heading(&self) -> Style {
        self.fg(0xD, Style::new().fg(Color::Blue))
            .add_modifier(Modifier::BOLD)
    }

    pub fn good(&self) -> Style {
        self.fg(0xB, Style::new().fg(Color::Green))
    }

    pub fn bad(&self) -> Style {
        self.fg(8, Style::new().fg(Color::Red))
    }

    pub fn warn(&self) -> Style {
        self.fg(0xA, Style::new().fg(Color::Yellow))
    }

    pub fn info(&self) -> Style {
        self.fg(0xC, Style::new().fg(Color::Cyan))
    }

    /// Five steps from easy to hard, for heatmaps of struggle.
    pub fn heat(&self, level: usize) -> Style {
        let fallback = [
            Style::new().add_modifier(Modifier::DIM),
            Style::new().fg(Color::Green),
            Style::new().fg(Color::Yellow),
            Style::new().fg(Color::Magenta),
            Style::new().fg(Color::Red),
        ];
        let level = level.min(4);
        self.fg([3, 0xB, 0xA, 9, 8][level], fallback[level])
    }
}

/// Relative luminance of `rgb`, from 0 (black) to 1 (white).
fn luminance(rgb: u32) -> f64 {
    let channel = |shift: u32| {
        let c = f64::from((rgb >> shift) as u8) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// The closest colour in the xterm 256-colour palette: its 6×6×6 cube or
/// its grey ramp.
fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let step = |c: u8| {
        (0..6)
            .min_by_key(|&i| (i32::from(LEVELS[i]) - i32::from(c)).abs())
            .unwrap_or(0)
    };
    let (ri, gi, bi) = (step(r), step(g), step(b));
    let cube = (LEVELS[ri], LEVELS[gi], LEVELS[bi]);
    let grey_level = ((u32::from(r) + u32::from(g) + u32::from(b)) / 3).saturating_sub(8) / 10;
    let grey_level = grey_level.min(23) as u8;
    let grey = 8 + 10 * grey_level;
    let distance = |(x, y, z): (u8, u8, u8)| {
        let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        d(x, r) + d(y, g) + d(z, b)
    };
    if distance((grey, grey, grey)) < distance(cube) {
        232 + grey_level
    } else {
        16 + 36 * ri as u8 + 6 * gi as u8 + bi as u8
    }
}

/// The palettes that come with Anzan, by name. See `THEMES.md` for where
/// each comes from.
#[rustfmt::skip]
pub const BUNDLED: &[(&str, Palette)] = &[
    // Catppuccin: base, mantle, surface0, surface1, surface2, text,
    // rosewater, lavender, red, peach, yellow, green, teal, blue, mauve,
    // flamingo.
    ("catppuccin-latte", [
        0xeff1f5, 0xe6e9ef, 0xccd0da, 0xbcc0cc, 0xacb0be, 0x4c4f69, 0xdc8a78, 0x7287fd,
        0xd20f39, 0xfe640b, 0xdf8e1d, 0x40a02b, 0x179299, 0x1e66f5, 0x8839ef, 0xdd7878,
    ]),
    ("catppuccin-frappe", [
        0x303446, 0x292c3c, 0x414559, 0x51576d, 0x626880, 0xc6d0f5, 0xf2d5cf, 0xbabbf1,
        0xe78284, 0xef9f76, 0xe5c890, 0xa6d189, 0x81c8be, 0x8caaee, 0xca9ee6, 0xeebebe,
    ]),
    ("catppuccin-macchiato", [
        0x24273a, 0x1e2030, 0x363a4f, 0x494d64, 0x5b6078, 0xcad3f5, 0xf4dbd6, 0xb7bdf8,
        0xed8796, 0xf5a97f, 0xeed49f, 0xa6da95, 0x8bd5ca, 0x8aadf4, 0xc6a0f6, 0xf0c6c6,
    ]),
    ("catppuccin-mocha", [
        0x1e1e2e, 0x181825, 0x313244, 0x45475a, 0x585b70, 0xcdd6f4, 0xf5e0dc, 0xb4befe,
        0xf38ba8, 0xfab387, 0xf9e2af, 0xa6e3a1, 0x94e2d5, 0x89b4fa, 0xcba6f7, 0xf2cdcd,
    ]),
    // Dracula: background, line highlight, selection, comment, comment,
    // foreground ×3, red, orange, yellow, green, cyan, purple, pink, orange.
    ("dracula", [
        0x282a36, 0x353747, 0x44475a, 0x6272a4, 0x6272a4, 0xf8f8f2, 0xf8f8f2, 0xf8f8f2,
        0xff5555, 0xffb86c, 0xf1fa8c, 0x50fa7b, 0x8be9fd, 0xbd93f9, 0xff79c6, 0xffb86c,
    ]),
    // Rosé Pine: base, surface, overlay, muted, subtle, text ×3, love,
    // rose, gold, foam, pine, iris, rose, love.
    ("rose-pine", [
        0x191724, 0x1f1d2e, 0x26233a, 0x6e6a86, 0x908caa, 0xe0def4, 0xe0def4, 0xe0def4,
        0xeb6f92, 0xebbcba, 0xf6c177, 0x9ccfd8, 0x31748f, 0xc4a7e7, 0xebbcba, 0xeb6f92,
    ]),
    ("rose-pine-dawn", [
        0xfaf4ed, 0xfffaf3, 0xf2e9e1, 0x9893a5, 0x797593, 0x464261, 0x464261, 0x464261,
        0xb4637a, 0xd7827e, 0xea9d34, 0x56949f, 0x286983, 0x907aa9, 0xd7827e, 0xb4637a,
    ]),
    // Solarized: base03 to base3 (reversed for light), then yellow,
    // orange, red, magenta, violet, blue, cyan, green in base16 order.
    ("solarized-dark", [
        0x002b36, 0x073642, 0x586e75, 0x657b83, 0x839496, 0x93a1a1, 0xeee8d5, 0xfdf6e3,
        0xdc322f, 0xcb4b16, 0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0x6c71c4, 0xd33682,
    ]),
    ("solarized-light", [
        0xfdf6e3, 0xeee8d5, 0x93a1a1, 0x839496, 0x657b83, 0x586e75, 0x073642, 0x002b36,
        0xdc322f, 0xcb4b16, 0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0x6c71c4, 0xd33682,
    ]),
];

/// Every theme on offer: the terminal's colours, the bundled palettes,
/// then any from the themes folder, in that order.
pub fn catalog(folder: Option<&Path>) -> Vec<Theme> {
    let mut themes = vec![Theme::terminal()];
    themes.extend(BUNDLED.iter().map(|(name, p)| Theme::palette(name, *p)));
    if let Some(folder) = folder {
        themes.extend(load_folder(folder));
    }
    themes
}

/// Base16 or base24 schemes in `folder`, named after their files.
/// Files that do not parse are skipped.
fn load_folder(folder: &Path) -> Vec<Theme> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut themes: Vec<Theme> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_owned();
            let palette = parse_base16(&fs::read_to_string(&path).ok()?)?;
            Some(Theme::palette(&name, palette))
        })
        .collect();
    themes.sort_by(|a, b| a.name.cmp(&b.name));
    themes
}

/// The sixteen `base0X` colours from a base16 or base24 scheme file,
/// written either flat or under `palette:`, with or without quotes and
/// `#`. Extra base24 slots are ignored.
pub fn parse_base16(text: &str) -> Option<Palette> {
    let mut palette = [None; 16];
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        let Some(slot) = key
            .trim()
            .strip_prefix("base0")
            .and_then(|h| usize::from_str_radix(h, 16).ok())
        else {
            continue;
        };
        // The first word, so a trailing `# comment` is ignored.
        let value = value.trim();
        let word = match value.chars().next() {
            Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
            _ => value.split_whitespace().next().unwrap_or(""),
        };
        let hex = word.trim_start_matches('#');
        if hex.len() == 6 {
            palette[slot] = u32::from_str_radix(hex, 16).ok();
        }
    }
    let mut out = [0; 16];
    for (o, p) in out.iter_mut().zip(palette) {
        *o = p?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_names_are_unique_and_light_ones_are_light() {
        let themes = catalog(None);
        let mut names: Vec<_> = themes.iter().map(|t| t.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), themes.len());
        for theme in &themes[1..] {
            let light = ["latte", "dawn", "light"]
                .iter()
                .any(|w| theme.name.contains(w));
            assert_eq!(theme.is_light(), Some(light), "{}", theme.name);
        }
        assert_eq!(Theme::terminal().is_light(), None);
    }

    #[test]
    fn terminal_theme_uses_named_colours_and_modifiers() {
        let t = Theme::terminal();
        assert_eq!(t.base(), Style::new());
        assert_eq!(t.good().fg, Some(Color::Green));
        assert_eq!(t.selected(), Style::new().add_modifier(Modifier::REVERSED));
    }

    #[test]
    fn palette_colours_round_without_truecolor() {
        let mocha = catalog(None)
            .into_iter()
            .find(|t| t.name == "catppuccin-mocha")
            .unwrap();
        assert_eq!(mocha.good().fg, Some(Color::Rgb(0xa6, 0xe3, 0xa1)));
        let rounded = mocha.with_truecolor(false);
        assert!(matches!(rounded.good().fg, Some(Color::Indexed(_))));
        assert_eq!(nearest_256(255, 0, 0), 196);
        assert_eq!(nearest_256(0, 0, 0), 16);
        assert_eq!(nearest_256(128, 128, 128), 244);
    }

    #[test]
    fn parses_base16_schemes_in_both_layouts() {
        let flat = (0..16)
            .map(|i| format!("base0{i:X}: \"{:06x}\"\n", i * 0x111111))
            .collect::<String>();
        let nested = format!(
            "system: \"base16\"\nname: \"Test\"\npalette:\n{}",
            (0..16)
                .map(|i| format!("  base0{i:X}: \"#{:06x}\" # slot {i}\n", i * 0x111111))
                .collect::<String>()
        );
        for text in [flat, nested] {
            let palette = parse_base16(&text).unwrap();
            assert_eq!(palette[0xF], 0xffffff);
            assert_eq!(palette[1], 0x111111);
        }
        assert_eq!(parse_base16("base00: 000000"), None);
    }

    #[test]
    fn loads_schemes_from_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let scheme: String = (0..16)
            .map(|i| format!("base0{i:X}: \"#10{i:02x}20\"\n"))
            .collect();
        fs::write(dir.path().join("gruvbox-dark.yaml"), scheme).unwrap();
        fs::write(dir.path().join("broken.yaml"), "nope").unwrap();
        fs::write(dir.path().join("notes.txt"), "base00: 000000").unwrap();
        let themes = catalog(Some(dir.path()));
        assert_eq!(themes.last().unwrap().name, "gruvbox-dark");
        assert_eq!(themes.len(), 1 + BUNDLED.len() + 1);
    }
}
