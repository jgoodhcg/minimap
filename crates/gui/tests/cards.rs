//! Card detail levels and toggles (unit `037`), and line-scaled cards (unit `039`).

mod common;

use std::collections::HashMap;

use eframe::egui::{Event, Key, Modifiers, Rect, Vec2};
use minimap_core::details::{Details, FileDetails};
use minimap_core::shape::{LineShape, MEDIAN_ABOVE, ShapeRow};
use minimap_gui::GuiApp;
use minimap_gui::app::ZOOM_TO_TOP;
use minimap_gui::camera::Camera;
use minimap_gui::scene::{
    self, Body, CardGeometry, Entry, Item, Kind, Lookup, Map, Options, Toggles,
};

/// The berth component's core namespace: 547 lines and 43 outline items, and a name that fits beside its line count.
const BERTH_CORE: &str = "components/berth/src/quayside/berth/core.clj";

fn details() -> Details {
    Details::from_json(&common::fixture("quayside-details.json")).unwrap()
}

/// The index of the berth core card.
fn berth_card(map: &Map) -> usize {
    (0..map.tree.files.len())
        .find(|&f| map.tree.files[f].path == BERTH_CORE)
        .unwrap()
}

/// A camera that centers a card's name row at a screen width, so the top of a tall card is in view.
fn on_card(map: &Map, f: usize, width: f32) -> Camera {
    let r = map.card(f);
    Camera {
        center: eframe::egui::pos2(r.center().x, r.top() + 200.0 / (width / map.card_w())),
        zoom: width / map.card_w(),
    }
}

fn screen_card(map: &Map, f: usize, width: f32) -> Rect {
    on_card(map, f, width).rect_to_screen(common::region(), map.card(f))
}

/// One card's items.
fn card_items<'a>(
    map: &'a Map,
    f: usize,
    width: f32,
    toggles: Toggles,
    d: &dyn Lookup,
) -> Vec<Item<'a>> {
    let opts = Options {
        toggles,
        ..Options::default()
    };
    let cam = on_card(map, f, width);
    let path = map.tree.files[f].path.as_str();
    scene::scene(map, &cam, common::region(), &opts, d)
        .into_iter()
        .filter(|i| i.path == path)
        .collect()
}

/// A rect as shares of the card it lies in.
fn rel(r: Rect, card: Rect) -> [f32; 4] {
    let a = (r.min - card.min) / card.size();
    let b = (r.max - card.min) / card.size();
    [a.x, a.y, b.x, b.y].map(|v| (v * 1e4).round() / 1e4)
}

fn rect_of(items: &[Item], kind: Kind) -> Option<Rect> {
    items.iter().find(|i| i.kind == kind).map(|i| i.rect)
}

fn bars_of(items: &[Item]) -> Vec<Rect> {
    match &items
        .iter()
        .find(|i| i.kind == Kind::Shape)
        .expect("a shape")
        .body
    {
        Body::Bars(b) => b.clone(),
        b => panic!("{b:?}"),
    }
}

/// Measured lines of a synthetic file: indents and lengths that vary, and every seventh line blank.
fn measured(lines: usize) -> Vec<ShapeRow> {
    (0..lines)
        .map(|i| {
            if i % 7 == 6 {
                ShapeRow::default()
            } else {
                ShapeRow {
                    indent: (i % 4 * 2) as u16,
                    len: 40 + (i % 5 * 10) as u16,
                }
            }
        })
        .collect()
}

/// Details as the loader computes them: the 64-row shape and every measured line.
fn synthetic(lines: usize) -> FileDetails {
    let m = measured(lines);
    FileDetails {
        shape: Some(LineShape::sample(&m, MEDIAN_ABOVE)),
        measured: m,
        ..FileDetails::default()
    }
}

/// The first quayside files with these line counts.
fn file_with_lines(map: &Map, lines: u64) -> usize {
    (0..map.tree.files.len())
        .find(|&f| map.index.files[f].lines == lines && !map.index.files[f].binary)
        .unwrap_or_else(|| panic!("a {lines}-line file"))
}

#[test]
fn name_and_shape_keep_their_places_with_each_toggle_and_zoom() {
    let (map, d) = (common::quayside(), details());
    let f = berth_card(&map);
    let all = Toggles::default();
    let base = card_items(&map, f, 200.0, all, &d);
    let shape = rect_of(&base, Kind::Shape).expect("a shape");
    let name = rect_of(&base, Kind::Name).expect("a name");
    for t in [
        Toggles {
            names: false,
            ..all
        },
        Toggles {
            shape: false,
            ..all
        },
    ] {
        let items = card_items(&map, f, 200.0, t, &d);
        for (kind, want, on) in [(Kind::Shape, shape, t.shape), (Kind::Name, name, t.names)] {
            let got = rect_of(&items, kind);
            if on {
                assert_eq!(got, Some(want), "{kind:?} with {t:?}");
            } else {
                assert_eq!(got, None, "{kind:?} is off with {t:?}");
            }
        }
    }
    // At two zooms of the same level, the shape and the name sit at the same shares of the card: text scales with the card (unit 040).
    let wide = card_items(&map, f, 300.0, all, &d);
    let (a, b) = (
        rel(shape, screen_card(&map, f, 200.0)),
        rel(
            rect_of(&wide, Kind::Shape).unwrap(),
            screen_card(&map, f, 300.0),
        ),
    );
    assert_eq!(a, b);
    let (a, b) = (
        rel(name, screen_card(&map, f, 200.0)),
        rel(
            rect_of(&wide, Kind::Name).unwrap(),
            screen_card(&map, f, 300.0),
        ),
    );
    assert_eq!(a, b);
}

#[test]
fn each_level_adds_its_elements_and_no_card_has_an_outline() {
    let (map, d) = (common::quayside(), details());
    let f = berth_card(&map);
    let kinds = |w: f32| -> Vec<Kind> {
        let mut k: Vec<Kind> = card_items(&map, f, w, Toggles::default(), &d)
            .iter()
            .map(|i| i.kind)
            .collect();
        k.sort();
        k.dedup();
        k
    };
    assert_eq!(kinds(10.0), [Kind::Block]);
    assert_eq!(kinds(30.0), [Kind::Card, Kind::Icon]);
    assert_eq!(kinds(50.0), [Kind::Card, Kind::Icon, Kind::Name]);
    let shape = [Kind::Card, Kind::Icon, Kind::Name, Kind::Count, Kind::Shape];
    for w in [60.0, 80.0, 130.0, 200.0, 400.0, 1008.0] {
        assert_eq!(kinds(w), shape, "{w}");
    }
    // The berth core file has 43 outline items, and no text item on its card is one of them.
    let outline = &d.get(&map.tree.files[f].path).unwrap().outline;
    assert!(outline.len() > 40);
    for w in [20.0, 60.0, 110.0, 160.0, 200.0, 400.0, 1008.0] {
        for i in card_items(&map, f, w, Toggles::default(), &d) {
            if let Body::Str(s) = &i.body {
                assert!(
                    matches!(i.kind, Kind::Name | Kind::Count),
                    "{:?} {s:?} at {w}",
                    i.kind
                );
            }
        }
    }
}

#[test]
fn a_300_line_shape_is_6_times_as_high_as_a_50_line_shape_at_every_zoom() {
    let map = common::quayside();
    // Since unit 041 a shape never draws past its card's body, so each file's card must hold its synthetic lines.
    let (a, b) = (file_with_lines(&map, 50), file_with_lines(&map, 300));
    let d = HashMap::from([
        (map.tree.files[a].path.clone(), synthetic(50)),
        (map.tree.files[b].path.clone(), synthetic(300)),
    ]);
    for w in [110.0, 168.0, 200.0, 400.0, 1008.0] {
        let ha = rect_of(&card_items(&map, a, w, Toggles::default(), &d), Kind::Shape)
            .unwrap()
            .height();
        let hb = rect_of(&card_items(&map, b, w, Toggles::default(), &d), Kind::Shape)
            .unwrap()
            .height();
        assert!((hb / ha - 6.0).abs() < 1e-4, "{w}: {ha} and {hb}");
        // One line is LINE_PT world points on every card.
        let zoom = w / map.card_w();
        assert!((ha - 50.0 * map.line_pt() * zoom).abs() < 1e-3, "{w}");
    }
}

#[test]
fn each_line_has_its_own_bar_until_a_line_is_under_a_pixel() {
    let map = common::quayside();
    let f = file_with_lines(&map, 90);
    let d = HashMap::from([(map.tree.files[f].path.clone(), synthetic(90))]);
    let blank = measured(90).iter().filter(|l| l.len == 0).count();
    // At card width 400 a line is 2.4 points: one bar per non-blank line, with no gap under 3 points.
    let items = card_items(&map, f, 400.0, Toggles::default(), &d);
    let bars = bars_of(&items);
    assert_eq!(bars.len(), 90 - blank);
    let line_h = 400.0 / map.card_w() * map.line_pt();
    assert!(bars.iter().all(|b| (b.height() - line_h).abs() < 1e-3));
    // At card width 1008 a line is 6 points, and each bar is 1 point shorter than its row.
    let bars = bars_of(&card_items(&map, f, 1008.0, Toggles::default(), &d));
    assert!(bars.iter().all(|b| (b.height() - 5.0).abs() < 1e-3));
    // A bar spans the body from the line's indent to its end, 120 columns across.
    let card = screen_card(&map, f, 1008.0);
    let body = CardGeometry::new(card).body;
    let per = body.width() / 120.0;
    let first = bars[0];
    assert!((first.left() - body.left()).abs() < 1e-3);
    assert!((first.width() - 40.0 * per).abs() < 1e-3);
    // Below one pixel per line, runs of lines share a sample: at most one bar per pixel row.
    for (w, ppp) in [(60.0, 1.0), (110.0, 1.0), (110.0, 2.0), (130.0, 1.0)] {
        let opts = Options {
            pixels_per_point: ppp,
            ..Options::default()
        };
        let cam = on_card(&map, f, w);
        let path = map.tree.files[f].path.as_str();
        let items: Vec<Item> = scene::scene(&map, &cam, common::region(), &opts, &d)
            .into_iter()
            .filter(|i| i.path == path)
            .collect();
        let line_h = w / map.card_w() * map.line_pt();
        let k = scene::lines_per_sample(line_h, ppp);
        assert_eq!(k, (1.0 / (line_h * ppp)).ceil().max(1.0) as usize);
        for b in bars_of(&items) {
            assert!(
                b.height() * ppp >= 1.0 - 1e-3,
                "{w} at {ppp}: {}",
                b.height()
            );
        }
    }
}

#[test]
fn a_json_shape_draws_each_row_as_many_lines_high_as_it_covers() {
    let (map, d) = (common::quayside(), details());
    let f = berth_card(&map);
    let s = d
        .get(&map.tree.files[f].path)
        .unwrap()
        .shape
        .clone()
        .unwrap();
    assert_eq!(s.rows.len(), 64);
    let items = card_items(&map, f, 200.0, Toggles::default(), &d);
    let line_h = 200.0 / map.card_w() * map.line_pt();
    let shape = rect_of(&items, Kind::Shape).unwrap();
    assert!((shape.height() - s.lines as f32 * line_h).abs() < 1e-2);
    let rows = s.rows.len() as f32;
    let want = s.lines as f32 / rows * line_h;
    for b in bars_of(&items) {
        assert!(
            (b.height() + 1.0 - want).abs() < line_h + 1e-3,
            "{}",
            b.height()
        );
    }
}

#[test]
fn the_shape_starts_under_the_name_row_and_fits_the_card() {
    // The rounding leaves at most a quarter of the body empty under the shape, from 10 body rows (about 150 lines) up.
    let map = common::quayside();
    let line_pt = map.line_pt();
    let mut worst: f32 = 0.0;
    for f in 0..map.tree.files.len() {
        let file = &map.index.files[f];
        if file.binary || file.lines == 0 {
            continue;
        }
        let card = map.card(f);
        let g = CardGeometry::new(card);
        // A capped generated card shows its first lines only (unit 041); every other card shows all of them.
        let (shown, cut) = scene::shown_lines(&g, line_pt, 1.0, file.lines as usize);
        assert_eq!(cut, map.tree.files[f].generated, "{}", file.path);
        if !cut {
            assert_eq!(shown as u64, (g.body.height() / line_pt).round() as u64);
        }
        let shape = (file.lines as usize).min(shown) as f32 * line_pt;
        assert!(
            g.body.top() + shape <= card.bottom() + 1e-3,
            "{} does not fit its card",
            file.path
        );
        let rows = map.layout.tile_h[f] - 1;
        if rows >= 10 && !cut {
            worst = worst.max(1.0 - shape / g.body.height());
        }
    }
    eprintln!("the emptiest body from 10 rows: {:.1} %", worst * 100.0);
    assert!(worst <= 0.25, "{worst}");
}

#[test]
fn at_the_block_level_a_card_is_as_tall_as_its_lines() {
    let map = common::quayside();
    let (short, long) = (file_with_lines(&map, 12), file_with_lines(&map, 625));
    for f in [short, long] {
        let items = card_items(&map, f, 12.0, Toggles::default(), &scene::NoDetails);
        let block = rect_of(&items, Kind::Block).unwrap();
        assert_eq!(block, screen_card(&map, f, 12.0));
    }
    assert!(map.card(long).height() > 20.0 * map.card(short).height());
}

#[test]
fn card_text_is_its_world_size_times_the_zoom_and_every_glyph_is_on_the_ladder() {
    // World sizes on a card 168 points wide: the icon 16, the name and the line count 14 (unit 040).
    let (map, d) = (common::quayside(), details());
    let f = berth_card(&map);
    for w in [
        20.0, 33.0, 60.0, 77.0, 110.0, 133.0, 160.0, 210.0, 333.0, 600.0, 1008.0,
    ] {
        let zoom = w / map.card_w();
        for i in card_items(&map, f, w, Toggles::default(), &d) {
            let world = match i.kind {
                Kind::Icon => 16.0,
                Kind::Name | Kind::Count => 14.0,
                _ => continue,
            };
            assert!(
                (i.size - world * zoom).abs() < 1e-3,
                "{:?} at {w}: {}",
                i.kind,
                i.size
            );
            match &i.body {
                Body::Str(_) | Body::Glyph(_) => {
                    let n = (0..100)
                        .find(|&n| (scene::ladder_size(n) - i.glyph).abs() < 1e-4 * i.glyph);
                    assert!(n.is_some(), "{:?} at {w}: glyph {}", i.kind, i.glyph);
                    assert!(
                        i.glyph <= i.size * (1.0 + 1e-5) && i.size < 1.1 * i.glyph * (1.0 + 1e-5)
                    );
                    assert!(i.size >= scene::TEXT_FROM);
                }
                Body::Greek | Body::Square => {
                    assert!(i.size < scene::TEXT_FROM && i.size >= scene::GREEK_FROM)
                }
                b => panic!("{b:?}"),
            }
        }
    }
}

#[test]
fn a_loading_card_shows_a_rect_as_tall_as_its_shape_will_be() {
    let map = common::quayside();
    let f = file_with_lines(&map, 90);
    let items = card_items(&map, f, 200.0, Toggles::default(), &scene::NoDetails);
    let loading = rect_of(&items, Kind::Loading).expect("a loading rect");
    let body = CardGeometry::new(screen_card(&map, f, 200.0)).body;
    let want = 90.0 * map.line_pt() * 200.0 / map.card_w();
    assert!((loading.height() - want).abs() < 1e-3);
    assert_eq!(loading.min, body.min);
}

#[test]
fn one_card_at_width_200() {
    let (map, d) = (common::quayside(), details());
    let f = berth_card(&map);
    let items = card_items(&map, f, 200.0, Toggles::default(), &d);
    let card = items[0].rect;
    let lines: Vec<String> = items
        .iter()
        .map(|i| {
            let r = rel(i.rect, card);
            let body = match &i.body {
                Body::Fill => "fill".to_string(),
                Body::Glyph(c) => format!("glyph U+{:04X}", *c as u32),
                Body::Square => "square".to_string(),
                Body::Str(s) => format!("{s:?}"),
                Body::Bars(b) => format!("{} bars", b.len()),
                Body::Stroke(w) => format!("stroke {w}"),
                Body::Greek => "greek".to_string(),
                Body::Halo { filled, .. } => format!("halo {filled}"),
                Body::Dot => "dot".to_string(),
            };
            format!(
                "{:?} {:?} [{:.3} {:.3} {:.3} {:.3}] size {} #{:02x}{:02x}{:02x} {}",
                i.kind,
                i.level,
                r[0],
                r[1],
                r[2],
                r[3],
                i.size,
                i.color.r(),
                i.color.g(),
                i.color.b(),
                body
            )
        })
        .collect();
    insta::assert_snapshot!(format!(
        "{} ({} lines, {} x {} points)\n{}",
        map.tree.files[f].path,
        map.index.files[f].lines,
        card.width(),
        card.height(),
        lines.join("\n")
    ));
}

fn key(k: Key) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn a_toggle_never_changes_a_card_world_rect_and_9_changes_nothing() {
    let mut app = GuiApp::new(common::quayside());
    app.set_region(common::region());
    let all: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    for t in [
        minimap_gui::app::Toggle::Names,
        minimap_gui::app::Toggle::Shape,
    ] {
        app.toggle(t);
        let now: Vec<Rect> = (0..app.map.tree.files.len())
            .map(|f| app.map.card(f))
            .collect();
        assert_eq!(all, now, "{t:?}");
    }
    assert_eq!(
        app.opts.toggles,
        Toggles {
            trail: true,
            notes: true,
            names: false,
            shape: false,
        }
    );
    // `9` is unbound: no toggle, no camera move, the same items.
    let mut app = GuiApp::new(common::quayside());
    app.set_region(common::region());
    let (toggles, cam) = (app.opts.toggles, app.cam());
    let before: Vec<(Kind, Rect)> = app.items().iter().map(|i| (i.kind, i.rect)).collect();
    app.on_event(&key(Key::Num9), None, 1.0);
    let after: Vec<(Kind, Rect)> = app.items().iter().map(|i| (i.kind, i.rect)).collect();
    assert_eq!((app.opts.toggles, app.cam()), (toggles, cam));
    assert_eq!(before, after);
}

#[test]
fn a_zoom_to_a_card_centers_a_short_card_and_puts_a_tall_cards_top_near_the_region_top() {
    let mut app = GuiApp::new(common::quayside());
    app.set_region(common::region());
    let region = app.region;
    let (short, tall) = (
        file_with_lines(&app.map, 12),
        file_with_lines(&app.map, 625),
    );
    let cam = app.camera_for(Entry::File(short));
    let card = cam.rect_to_screen(region, app.map.card(short));
    assert!((card.width() - 420.0).abs() < 1e-3);
    assert!((card.center() - region.center()).length() < 1e-2);
    let cam = app.camera_for(Entry::File(tall));
    let card = cam.rect_to_screen(region, app.map.card(tall));
    assert!(card.height() > region.height());
    assert!((card.center().x - region.center().x).abs() < 1e-2);
    assert!((card.top() - (region.top() + ZOOM_TO_TOP)).abs() < 1e-2);
    // The block is still a card: its size in the world is fixed.
    assert_eq!(
        Vec2::new(app.map.card_w(), app.map.card(tall).height()),
        app.map.card(tall).size()
    );
}

/// quayside's `package-lock.json`: 12,487 lines, generated by its name (unit 041).
fn lock_card(map: &Map) -> usize {
    map.entry("package-lock.json")
        .and_then(|e| match e {
            Entry::File(f) => Some(f),
            Entry::Dir(_) => None,
        })
        .unwrap()
}

#[test]
fn a_capped_card_shows_its_first_lines_above_a_cut_mark() {
    let map = common::quayside();
    let lock = lock_card(&map);
    assert!(map.tree.files[lock].generated);
    let lines = map.index.files[lock].lines as usize;
    assert_eq!(lines, 12_487);
    let d = HashMap::from([(map.tree.files[lock].path.clone(), synthetic(lines))]);
    // The camera centers the whole card, so its bottom is in the region: a capped card is 396 world points high.
    let region = common::region();
    for w in [60.0, 130.0, 200.0, 400.0] {
        let cam = Camera {
            center: map.card(lock).center(),
            zoom: w / map.card_w(),
        };
        let items: Vec<Item> = scene::scene(&map, &cam, region, &Options::default(), &d)
            .into_iter()
            .filter(|i| i.path == "package-lock.json")
            .collect();
        let card = cam.rect_to_screen(region, map.card(lock));
        assert!(region.contains_rect(card), "{w}");
        let body = CardGeometry::new(card).body;
        let cut = rect_of(&items, Kind::Cut).expect("a cut mark");
        // The mark lies along the body's bottom, and every shape bar is above it.
        assert!((cut.bottom() - body.bottom()).abs() < 1e-3, "{w}");
        assert!(
            (cut.left() - body.left()).abs() < 1e-3 && (cut.right() - body.right()).abs() < 1e-3
        );
        let dashes = match &items.iter().find(|i| i.kind == Kind::Cut).unwrap().body {
            Body::Bars(b) => b.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(dashes.len(), scene::CUT_DASHES);
        let bars = bars_of(&items);
        assert!(!bars.is_empty());
        for b in &bars {
            assert!(
                b.top() >= body.top() - 1e-3 && b.bottom() <= cut.top() + 1e-3,
                "{w}: {b:?} under {cut:?}"
            );
        }
        // One line keeps one height: the shape is as many lines high as it shows, at the zoom's line height.
        let zoom = w / map.card_w();
        let shape = rect_of(&items, Kind::Shape).unwrap();
        let (shown, is_cut) =
            scene::shown_lines(&CardGeometry::new(card), map.line_pt() * zoom, zoom, lines);
        assert!(is_cut && shown < lines);
        assert!(
            (shape.height() - shown as f32 * map.line_pt() * zoom).abs() < 1e-2,
            "{w}"
        );
        assert!(shown > 350, "{w}: {shown} lines");
        // The name row's count is the real line count (a bar below card width 84).
        let count = items.iter().find(|i| i.kind == Kind::Count).unwrap();
        if w >= 84.0 {
            assert_eq!(count.body, Body::Str("12487".into()), "{w}");
        }
    }
}

#[test]
fn the_cut_mark_shows_from_the_icon_level_also_without_the_shape() {
    let map = common::quayside();
    let lock = lock_card(&map);
    let no_shape = Toggles {
        shape: false,
        ..Toggles::default()
    };
    for (w, toggles, want) in [
        (10.0, Toggles::default(), false),
        (30.0, Toggles::default(), true),
        (50.0, Toggles::default(), true),
        (200.0, no_shape, true),
        (200.0, Toggles::default(), true),
    ] {
        let items = card_items(&map, lock, w, toggles, &scene::NoDetails);
        assert_eq!(rect_of(&items, Kind::Cut).is_some(), want, "{w}");
        // While the details load, the loading rect stops above the mark too.
        if let (Some(l), Some(c)) = (rect_of(&items, Kind::Loading), rect_of(&items, Kind::Cut)) {
            assert!(l.bottom() <= c.top() + 1e-3, "{w}");
        }
    }
    // A card that holds its lines has no cut mark: a 300-line hand-written file, and every other quayside card.
    let f = file_with_lines(&map, 300);
    assert!(!map.tree.files[f].generated);
    let items = card_items(&map, f, 200.0, Toggles::default(), &scene::NoDetails);
    assert!(rect_of(&items, Kind::Cut).is_none());
    let opts = Options::default();
    let app = GuiApp::new(common::quayside());
    let fit = app.bounds.fit(common::region());
    for zoom in [fit.zoom * 4.0, 0.5, 1.0] {
        let cam = Camera { zoom, ..fit };
        let cuts: Vec<&str> = scene::scene(&map, &cam, common::region(), &opts, &scene::NoDetails)
            .into_iter()
            .filter(|i| i.kind == Kind::Cut)
            .map(|i| i.path)
            .collect();
        assert!(cuts.iter().all(|p| *p == "package-lock.json"), "{cuts:?}");
    }
}
