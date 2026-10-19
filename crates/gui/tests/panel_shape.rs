//! The selection panel's shape (unit `045`): the box, the bars, the line numbers, and the definition ticks.

mod common;

use eframe::egui::{self, Pos2, Rect, vec2};
use minimap_core::details::{Details, Loader};
use minimap_core::palette;
use minimap_gui::GuiApp;
use minimap_gui::panel::{
    Draw, GitHistory, OUTLINE_LABEL, Panel, SHAPE_GAP_FROM, SHAPE_LINE, TICK_W,
};
use minimap_gui::scene::Entry;
use minimap_gui::tree::ROW_H;

/// A Clojure file of 547 lines with 43 outline items in `quayside-details.json`.
const BERTH: &str = "components/berth/src/quayside/berth/core.clj";

/// The quayside map in a window of this height, with every file's details and the fixture's git history, and `path` selected.
fn app(path: &str, height: f32) -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, height)));
    let text = common::fixture("quayside-details.json");
    let mut loader = Loader::with_workers(Details::from_json(&text).unwrap(), || {}, 0);
    loader.request(app.map.tree.files.iter().map(|f| f.path.clone()));
    loader.poll();
    app.loader = Some(loader);
    app.git = GitHistory::Ready(minimap_core::History::from_json(&text, 1_767_225_600).unwrap());
    let e = app.map.entry(path).unwrap();
    app.select(Some(e), false, 0.0);
    app
}

fn fills(p: &Panel, color: u32) -> Vec<Rect> {
    p.draws
        .iter()
        .filter_map(|d| match d {
            Draw::Fill { rect, color: c } if *c == color => Some(*rect),
            _ => None,
        })
        .collect()
}

fn y_of(p: &Panel, s: &str) -> f32 {
    match p.find(s) {
        Some(Draw::Text { at, .. }) => at.y,
        _ => panic!("no {s:?} in {:#?}", p.texts()),
    }
}

fn lines(app: &GuiApp, path: &str) -> usize {
    match app.map.entry(path) {
        Some(Entry::File(f)) => app.map.index.files[f].lines as usize,
        e => panic!("{path}: {e:?}"),
    }
}

#[test]
fn the_shape_lies_between_the_facts_and_the_outline_with_its_bars_in_the_box() {
    let app = app(BERTH, 1000.0);
    let n = lines(&app, BERTH);
    let p = app.panel().unwrap();
    let shape = y_of(&p, "SHAPE");
    assert!(y_of(&p, &format!("{n} lines")) < shape && shape < y_of(&p, OUTLINE_LABEL));
    let boxes = fills(&p, palette::MINI_BG);
    assert_eq!(boxes.len(), 1);
    let bx = boxes[0];
    // The box starts on the row below the label, and the outline label comes after the box and a blank row.
    assert!((bx.top() - (shape + ROW_H / 2.0)).abs() < 1e-3);
    assert!((y_of(&p, OUTLINE_LABEL) - (bx.bottom() + ROW_H * 1.5)).abs() < 1e-3);
    let bars = fills(&p, palette::MINI_DOT);
    assert!(bars.len() > 10, "{}", bars.len());
    for b in &bars {
        assert!(bx.expand(1e-3).contains_rect(*b), "{b:?} in {bx:?}");
    }
    // A file of more than 48 lines fills the 12 rows of the box: 240 points.
    assert!(n > 48, "{n}");
    assert_eq!(bx.height(), 12.0 * ROW_H);
    let bottom = bars.iter().map(|b| b.bottom()).fold(f32::MIN, f32::max);
    assert!(
        bx.bottom() - bottom < SHAPE_GAP_FROM + 1.0,
        "{bottom} in {bx:?}"
    );
}

#[test]
fn a_short_file_takes_five_points_a_line_from_the_top_of_the_box() {
    // The first file with a shape and 20 to 47 lines: `berth/core_test.clj`, 44 lines of 5 points, under the 240 points that would stretch it.
    let probe = app(BERTH, 1000.0);
    let short = (0..probe.map.tree.files.len())
        .find(|&f| {
            let n = probe.map.index.files[f].lines;
            (20..=47).contains(&n)
                && probe
                    .file_details(f)
                    .and_then(|d| d.shape.as_ref())
                    .is_some_and(|s| !s.rows.is_empty())
        })
        .unwrap();
    let path = probe.map.tree.files[short].path.clone();
    let app = app(&path, 1000.0);
    let n = lines(&app, &path);
    let p = app.panel().unwrap();
    let bx = fills(&p, palette::MINI_BG)[0];
    let line_h = SHAPE_LINE * ROW_H;
    // The box is as tall as its lines' rows: a short file does not stretch.
    let used = (n as f32 * line_h / ROW_H).ceil();
    assert_eq!(bx.height(), used * ROW_H, "{n} lines");
    for b in fills(&p, palette::MINI_DOT) {
        // Each bar covers whole lines of 5 points, less the 1-point gap, from the box's top.
        let lines_high = (b.height() + 1.0) / line_h;
        assert!((lines_high - lines_high.round()).abs() < 1e-3, "{b:?}");
        let from = (b.top() - bx.top()) / line_h;
        assert!((from - from.round()).abs() < 1e-3, "{b:?}");
        assert!(b.bottom() <= bx.top() + n as f32 * line_h + 1e-3);
    }
}

#[test]
fn line_numbers_read_the_first_the_middle_rows_first_and_the_last_line() {
    let app = app(BERTH, 1000.0);
    let n = lines(&app, BERTH);
    let last = n.to_string();
    let p = app.panel().unwrap();
    let t = p.texts();
    let at = t.iter().position(|s| *s == "SHAPE").unwrap();
    // 12 rows of 240 / n points per line: the middle row is row 5, whose first line is 5 * 20 / line_h + 1.
    let line_h = 240.0f32 / n as f32;
    let mid = ((5.0 * ROW_H) / line_h).floor() as usize + 1;
    assert_eq!(t[at + 1..at + 4], ["1", &mid.to_string()[..], &last[..]]);
    let bx = fills(&p, palette::MINI_BG)[0];
    // The numbers are right-aligned in the gutter, left of the box, on their rows.
    for (s, row) in [("1", 0.0), (&last[..], 11.0)] {
        assert!(
            (y_of(&p, s) - (bx.top() + (row + 0.5) * ROW_H)).abs() < 1e-3,
            "{s}"
        );
    }
    let Some(Draw::Text { at: one, .. }) = p.find(&last) else {
        panic!()
    };
    assert!(one.x < bx.left());
}

#[test]
fn one_tick_per_outline_item_at_its_line() {
    let app = app(BERTH, 1000.0);
    let p = app.panel().unwrap();
    let bx = fills(&p, palette::MINI_BG)[0];
    let line_h = bx.height() / lines(&app, BERTH) as f32;
    // Ticks are the faint fills right of the box.
    let ticks: Vec<Rect> = fills(&p, palette::FAINT)
        .into_iter()
        .filter(|r| r.left() > bx.right())
        .collect();
    let Some(Entry::File(f)) = app.map.entry(BERTH) else {
        panic!()
    };
    let outline = &app.file_details(f).unwrap().outline;
    assert_eq!(ticks.len(), outline.len());
    for (t, item) in ticks.iter().zip(outline) {
        let y = bx.top() + (item.line - 1) as f32 * line_h;
        assert!((t.top() - y).abs() < 1e-3, "{} at {t:?}", item.line);
        assert_eq!(t.width(), TICK_W);
        assert!(t.height() >= 1.0);
    }
}

#[test]
fn the_box_takes_4_to_12_rows_and_drops_when_7_rows_or_fewer_are_free() {
    let rows = |height: f32| {
        let app = app(BERTH, height);
        let p = app.panel().unwrap();
        fills(&p, palette::MINI_BG)
            .first()
            .map(|b| (b.height() / ROW_H).round() as usize)
    };
    // 470 points: 7 rows are free above the git group, so no shape. 490: 8 free, 40 % is 3, at least 4.
    assert_eq!(rows(470.0), None);
    assert_eq!(rows(490.0), Some(4));
    assert_eq!(rows(2000.0), Some(12));
    // A binary file has no shape.
    let probe = app(BERTH, 1000.0);
    let bin = (0..probe.map.index.files.len())
        .find(|&f| probe.map.index.files[f].binary)
        .unwrap();
    let path = probe.map.tree.files[bin].path.clone();
    let app = app(&path, 1000.0);
    assert!(!app.panel().unwrap().texts().contains(&"SHAPE"));
}

#[test]
fn a_headless_frame_with_the_shape_tessellates() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = app(BERTH, 1000.0);
    let input = egui::RawInput {
        screen_rect: Some(common::screen()),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
    out.textures_delta.clear();
    assert!(app.panel().unwrap().texts().contains(&"SHAPE"));
    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    assert!(!prims.is_empty());
}
