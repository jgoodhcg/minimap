//! The colors of `minimap_core::palette` as Ratatui colors, and the TUI's mixes of them. File-type colors are in `icons`.

use minimap_core::palette as p;
use ratatui::style::Color;

/// A `0xRRGGBB` value as a Ratatui color.
pub const fn rgb(hex: u32) -> Color {
    let (r, g, b) = p::rgb(hex);
    Color::Rgb(r, g, b)
}

pub const PANE: Color = rgb(p::PANE);
pub const TILE: Color = rgb(p::TILE);
/// A tile under the mouse.
pub const TILE_HI: Color = rgb(p::TILE_HI);
pub const INK: Color = rgb(p::INK);
pub const INK2: Color = rgb(p::INK2);
pub const DIM: Color = rgb(p::DIM);
pub const FAINT: Color = rgb(p::FAINT);
pub const RULE: Color = rgb(p::RULE);
pub const LABEL: Color = rgb(p::LABEL);
pub const FOCUS: Color = rgb(p::FOCUS);
pub const BAR: Color = rgb(p::BAR);
pub const SIDE: Color = rgb(p::SIDE);
pub const POP: Color = rgb(p::POP);
pub const CURSOR: Color = rgb(p::CURSOR);
/// The selection pulse color, drawn from 30 % to 70 % over the pane.
pub const SEL_HI: Color = rgb(p::SEL_HI);
/// The background of a selected directory's block.
pub const BLOCK_BG: Color = rgb(p::BLOCK_BG);
/// The search line's background in the status bar.
pub const SEARCH_BG: Color = rgb(p::SEARCH_BG);
/// The overview: its background, its dots, and the viewport fill and edge.
pub const MINI_BG: Color = rgb(p::MINI_BG);
pub const MINI_DOT: Color = rgb(p::MINI_DOT);
pub const VIEW_IN: Color = rgb(p::VIEW_IN);
pub const VIEW_EDGE: Color = rgb(p::VIEW_EDGE);
pub const WHITE: Color = rgb(p::WHITE);

/// `a` mixed with `b`: `t` of `a` and `1 - t` of `b`.
pub fn mix(a: Color, b: Color, t: f64) -> Color {
    let (Color::Rgb(r, g, bl), Color::Rgb(r2, g2, b2)) = (a, b) else {
        return a;
    };
    let m = |x: u8, y: u8| (f64::from(x) * t + f64::from(y) * (1.0 - t)).round() as u8;
    Color::Rgb(m(r, r2), m(g, g2), m(bl, b2))
}

/// The selected cell's background for a pulse level from 0 to 1 (`selBg` in the v5 mockup).
pub fn selection_bg(level: f64) -> Color {
    mix(SEL_HI, PANE, 0.3 + 0.4 * level)
}

/// The HSL saturation and lightness of an RGB color, from 0 to 1.
pub fn saturation_lightness(c: Color) -> (f64, f64) {
    let Color::Rgb(r, g, b) = c else {
        return (0.0, 0.0);
    };
    let [r, g, b] = [r, g, b].map(|v| f64::from(v) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    (s, l)
}

/// State colors fill the cell background at this share over the pane (`docs/design.md` `Color`).
pub const STATE_MIX: f64 = 0.45;

/// A state base color as a cell background.
pub fn state_bg(base: Color) -> Color {
    mix(base, PANE, STATE_MIX)
}

/// The search match state color (high chroma).
pub const SEARCH_HI: Color = rgb(p::SEARCH_HI);

/// The trail state colors of layer 1 (unit `010`). Read and grep are the v3 hues (`#6cb6d9`, `#c9a14a`)
/// raised to an HSL saturation of at least 60 %.
pub const READ: Color = rgb(p::READ);
pub const GREP: Color = rgb(p::GREP);
pub const EDIT: Color = rgb(p::EDIT);
/// A trail cell's background share over the pane: fresh (among the 3 newest events), then older.
pub const TRAIL_FRESH_MIX: f64 = STATE_MIX;
pub const TRAIL_OLD_MIX: f64 = 0.20;

/// A trail kind's base color.
pub fn trail_color(kind: minimap_core::events::Kind) -> Color {
    use minimap_core::events::Kind;
    match kind {
        Kind::Read => READ,
        Kind::Grep => GREP,
        Kind::Edit => EDIT,
    }
}

/// A trail cell's background.
pub fn trail_bg(kind: minimap_core::events::Kind, fresh: bool) -> Color {
    let t = if fresh {
        TRAIL_FRESH_MIX
    } else {
        TRAIL_OLD_MIX
    };
    mix(trail_color(kind), PANE, t)
}

/// The heat state color of layers 4 and 5 (high chroma). They are exclusive, so they share it.
pub const HEAT: Color = rgb(p::HEAT);
/// The heat ramp steps, strongest first, as shares over the pane.
pub const HEAT_MIX: [f64; 3] = [0.45, 0.30, 0.18];

/// The background of a heat ramp step, 0 for the strongest.
pub fn heat_bg(step: usize) -> Color {
    mix(HEAT, PANE, HEAT_MIX[step.min(2)])
}

/// The viewer's diff colors (unit `030`): added and removed rows, drawn at [`DIFF_MIX`] over the pane.
pub const DIFF_ADD: Color = rgb(p::DIFF_ADD);
pub const DIFF_REMOVE: Color = rgb(p::DIFF_REMOVE);
pub const DIFF_MIX: f64 = 0.22;

/// The name color of a binary file.
pub const BINARY: Color = rgb(p::BINARY);
