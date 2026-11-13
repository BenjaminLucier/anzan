//! 暗算, drawn by hand at 16×16 pixels.
//!
//! 暗 (an, "dark, unseen") is 日 (sun) beside 音 (sound). 算 (san,
//! "calculate") is ⺮ (bamboo, as in counting rods) over 目 (eye) over 廾
//! (two hands): reckoning with the hands, which mental calculation does
//! without.

use super::bitmap::Bitmap;

#[rustfmt::skip]
const AN: [&str; 16] = [
    "..........#.....",
    "..........#.....",
    "......#########.",
    "####...#....#...",
    "#..#...#...#....",
    "#..#....#..#....",
    "#..#.###########",
    "#..#............",
    "####...#######..",
    "#..#...#.....#..",
    "#..#...#.....#..",
    "#..#...#######..",
    "####...#.....#..",
    ".......#.....#..",
    ".......#######..",
    "................",
];

#[rustfmt::skip]
const SAN: [&str; 16] = [
    "..#.......#.....",
    ".#####...######.",
    "#..#....#..#....",
    "...#.......#....",
    "...##########...",
    "...#........#...",
    "...##########...",
    "...#........#...",
    "...##########...",
    "...#........#...",
    "...##########...",
    ".....#....#.....",
    "################",
    ".....#....#.....",
    "....#.....#.....",
    "..##......#.....",
];

/// Pixels between the two characters.
const GAP: usize = 3;

/// 暗算, side by side.
pub fn anzan() -> Bitmap {
    Bitmap::from_glyphs([&AN[..], &SAN[..]], GAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_square() {
        for glyph in [AN, SAN] {
            assert!(glyph.iter().all(|row| row.len() == 16));
        }
    }
}
