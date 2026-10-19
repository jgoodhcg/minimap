//! File-type icons (Nerd Font glyphs) and type colors.
//!
//! The table is `minimap_core::filetype`; this module converts it to Ratatui colors.
//! The glyphs need a Nerd Font in the terminal. `--no-icons` turns them off.

use minimap_core::filetype;
use ratatui::style::Color;

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon {
    pub glyph: char,
    pub color: Color,
}

impl From<filetype::Icon> for Icon {
    fn from(i: filetype::Icon) -> Icon {
        Icon {
            glyph: i.glyph,
            color: theme::rgb(i.rgb),
        }
    }
}

/// Folder glyphs for the file tree.
pub const FOLDER_CLOSED: char = filetype::FOLDER_CLOSED;
pub const FOLDER_OPEN: char = filetype::FOLDER_OPEN;

/// The icon for a file name (`minimap_core::filetype::for_name`).
pub fn for_name(name: &str) -> Icon {
    filetype::for_name(name).into()
}

/// The type color of a file: its icon's color. A binary file keeps the dim binary color.
/// Only icons carry it; names stay neutral (`docs/design.md` `Color`).
pub fn type_color(name: &str, binary: bool) -> Color {
    theme::rgb(filetype::type_rgb(name, binary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_table_converts() {
        assert_eq!(for_name("a.clj").glyph, '\u{e768}');
        assert_eq!(for_name("a.clj").color, Color::Rgb(0x98, 0xc5, 0x77));
        assert_eq!(type_color("a.bin", true), theme::BINARY);
        assert_eq!(type_color("a.clj", false), for_name("a.clj").color);
    }

    #[test]
    fn type_colors_are_low_chroma_readable_and_distinct() {
        for icon in filetype::ALL.map(Icon::from) {
            let (s, l) = theme::saturation_lightness(icon.color);
            assert!(s <= 0.405, "{icon:?} saturation {s:.3}");
            assert!((0.57..=0.88).contains(&l), "{icon:?} lightness {l:.3}");
        }
    }
}
