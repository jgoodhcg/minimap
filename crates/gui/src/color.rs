//! `minimap_core::palette` values as egui colors, and the TUI's mix.

use eframe::egui::Color32;
use minimap_core::palette;

/// The session colors of the agent history (unit `057`), by a session's arrival order modulo 8. Each reads on `SIDE`,
/// and each hue is at least 20° from `FOCUS` (47°), the trail colors (read 199°, search 41°, edit 10°), and `CURSOR` (210°).
/// Hues: green 111°, violet 262°, pink 327°, teal 176°, lime 75°, periwinkle 233°, orchid 293°, sea green 150°.
pub const SESSION: [u32; 8] = [
    0x78d468, 0xb08af2, 0xf080bd, 0x4cc9c0, 0xb5d653, 0x8592f5, 0xdc80e8, 0x58d696,
];

/// A `0xRRGGBB` value as an opaque color.
pub const fn rgb(hex: u32) -> Color32 {
    let (r, g, b) = palette::rgb(hex);
    Color32::from_rgb(r, g, b)
}

/// `a` mixed with `b`: `t` of `a` and `1 - t` of `b`, per channel, as the TUI mixes.
pub fn mix(a: u32, b: u32, t: f32) -> Color32 {
    rgb(mix_hex(a, b, t))
}

/// [`mix`] as a `0xRRGGBB` value, for draw items that hold one (unit `054`).
pub fn mix_hex(a: u32, b: u32, t: f32) -> u32 {
    let (a, b) = (palette::rgb(a), palette::rgb(b));
    let m = |x: u8, y: u8| u32::from((f32::from(x) * t + f32::from(y) * (1.0 - t)).round() as u8);
    (m(a.0, b.0) << 16) | (m(a.1, b.1) << 8) | m(a.2, b.2)
}

/// A color with an opacity from 0 to 1.
pub fn alpha(hex: u32, a: f32) -> Color32 {
    let (r, g, b) = palette::rgb(hex);
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// A color with an opacity from 0 to 1 in place of its own.
pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        c.r(),
        c.g(),
        c.b(),
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}
