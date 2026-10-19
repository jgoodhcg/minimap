//! The colors of both front ends as `0xRRGGBB` values (`mockups/v3-template.html:171-182`, `mockups/v5-template.html` `C`,
//! and the state colors of `docs/design.md` `Color`). File-type colors are in [`crate::filetype`].
//! Each renderer converts them to its own color type.

pub const PANE: u32 = 0x10161c;
pub const TILE: u32 = 0x161e27;
/// A tile under the mouse.
pub const TILE_HI: u32 = 0x1e2a36;
pub const INK: u32 = 0xd4dce4;
pub const INK2: u32 = 0xaeb9c4;
pub const DIM: u32 = 0x7b8896;
pub const FAINT: u32 = 0x46525e;
pub const RULE: u32 = 0x26313c;
pub const LABEL: u32 = 0x8d9aa8;
pub const FOCUS: u32 = 0xf2d15c;
pub const BAR: u32 = 0x0b1015;
pub const SIDE: u32 = 0x0e1419;
pub const POP: u32 = 0x1a222b;
pub const CURSOR: u32 = 0x2d4155;
/// The selection color. The TUI pulses it from 30 % to 70 % over the pane.
pub const SEL_HI: u32 = 0x5f86ad;
/// The background of a selected directory's block.
pub const BLOCK_BG: u32 = 0x16222e;
/// The search line's background in the status bar.
pub const SEARCH_BG: u32 = 0x28402f;
/// The overview: its background, its dots, and the viewport fill and edge.
pub const MINI_BG: u32 = 0x0b1015;
pub const MINI_DOT: u32 = 0x71879b;
pub const VIEW_IN: u32 = 0x1a2632;
pub const VIEW_EDGE: u32 = 0x3a526a;
pub const WHITE: u32 = 0xf3f6f9;

/// The search match state color (high chroma).
pub const SEARCH_HI: u32 = 0x3fd67a;

/// The trail state colors of layer 1 (unit `010`). Read and grep are the v3 hues (`#6cb6d9`, `#c9a14a`)
/// raised to an HSL saturation of at least 60 %.
pub const READ: u32 = 0x52b3e0;
pub const GREP: u32 = 0xd6a63d;
pub const EDIT: u32 = 0xe2725b;

/// The heat state color of layers 4 and 5 (high chroma). They are exclusive, so they share it.
pub const HEAT: u32 = 0xe0529c;

/// The viewer's diff colors (unit `030`): added and removed rows.
pub const DIFF_ADD: u32 = 0x3fb950;
pub const DIFF_REMOVE: u32 = 0xf85149;

/// The type color of a binary file.
pub const BINARY: u32 = 0x66717c;

/// The red, green, and blue parts of a color.
pub const fn rgb(hex: u32) -> (u8, u8, u8) {
    ((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
