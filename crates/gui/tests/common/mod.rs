//! Shared fixtures for the GUI tests: the quayside map and a fixed map region.
#![allow(dead_code)]

use std::fs;

use eframe::egui::{Rect, pos2};
use minimap_core::Index;
use minimap_gui::scene::Map;

pub fn fixture(name: &str) -> String {
    fs::read_to_string(format!(
        "{}/../core/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

pub fn quayside() -> Map {
    Map::new(Index::from_json(&fixture("quayside-files.json")).unwrap())
}

/// The capture window's map region: 1,600 x 1,000 points less the two 24-point bars.
pub fn region() -> Rect {
    Rect::from_min_max(pos2(0.0, 24.0), pos2(1600.0, 976.0))
}

/// The capture window's screen: 1,600 x 1,000 points.
pub fn screen() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(1600.0, 1000.0))
}
