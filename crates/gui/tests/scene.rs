mod common;

use std::collections::BTreeMap;

use eframe::egui::pos2;
use minimap_gui::camera::{Bounds, Camera};
use minimap_gui::scene::{self, Body, Kind, Level, NoDetails, Options};

/// Item counts per kind and level at a card width, centered on a berth card near the middle of the map.
fn counts(width: Option<f32>) -> String {
    let map = common::quayside();
    let region = common::region();
    let b = Bounds { map: map.world() };
    let Some(scene::Entry::File(f)) = map.entry("components/berth/src/quayside/berth/scoring.clj")
    else {
        panic!("a quayside file");
    };
    let at = map.card(f).center();
    let cam = match width {
        None => b.fit(region),
        Some(w) => b.clamp(
            region,
            Camera {
                center: at,
                zoom: w / map.card_w(),
            },
        ),
    };
    let items = scene::scene(&map, &cam, region, &Options::default(), &NoDetails);
    let mut n: BTreeMap<String, usize> = BTreeMap::new();
    for i in &items {
        *n.entry(format!("{:?} {:?}", i.kind, i.level)).or_default() += 1;
        assert!(
            i.rect.intersects(region),
            "{:?} {} at {:?}",
            i.kind,
            i.path,
            i.rect
        );
        if i.glyph > 0.0 {
            assert!(on_ladder(i.glyph), "{:?} glyph {}", i.kind, i.glyph);
        }
    }
    let label = width.map_or("fit".to_string(), |w| format!("card width {w}"));
    let rows: Vec<String> = n.iter().map(|(k, v)| format!("  {k}: {v}")).collect();
    format!("{label}, zoom {:.4}\n{}", cam.zoom, rows.join("\n"))
}

#[test]
fn item_counts_at_four_zooms() {
    let out: Vec<String> = [None, Some(12.0), Some(40.0), Some(130.0)]
        .into_iter()
        .map(counts)
        .collect();
    insta::assert_snapshot!(out.join("\n"));
}

#[test]
fn the_level_changes_exactly_at_each_threshold() {
    for (below, at, level) in [
        (19.9, 20.0, Level::Icon),
        (39.9, 40.0, Level::Name),
        (59.9, 60.0, Level::Shape),
    ] {
        assert!(Level::for_width(below) < level, "{below}");
        assert_eq!(Level::for_width(at), level, "{at}");
    }
    assert_eq!(Level::for_width(0.0), Level::Block);
    assert_eq!(Level::for_width(1008.0), Level::Shape);
}

#[test]
fn every_item_lies_in_the_map_region_while_panning() {
    let map = common::quayside();
    let region = common::region();
    let b = Bounds { map: map.world() };
    for (x, y, w) in [
        (0.0, 0.0, 30.0),
        (0.5, 0.5, 80.0),
        (1.0, 1.0, 300.0),
        (0.2, 0.9, 8.0),
    ] {
        let cam = b.clamp(
            region,
            Camera {
                center: pos2(map.world().x * x, map.world().y * y),
                zoom: w / map.card_w(),
            },
        );
        let items = scene::scene(&map, &cam, region, &Options::default(), &NoDetails);
        for i in &items {
            assert!(
                i.rect.intersects(region),
                "{:?} {} at {:?}",
                i.kind,
                i.path,
                i.rect
            );
        }
        // Culling: every card that shows intersects the region, and every card that does not show lies outside it.
        let shown: std::collections::HashSet<&str> = items
            .iter()
            .filter(|i| i.kind == scene::Kind::Card || i.kind == scene::Kind::Block)
            .map(|i| i.path)
            .collect();
        for f in 0..map.tree.files.len() {
            let on = cam.rect_to_screen(region, map.card(f)).intersects(region) && map.visible(f);
            assert_eq!(on, shown.contains(map.tree.files[f].path.as_str()));
        }
    }
}

/// A glyph size is `7 * 1.1^n` for a whole `n`.
fn on_ladder(g: f32) -> bool {
    (0..200).any(|n| (scene::ladder_size(n) - g).abs() < 1e-4 * g)
}

#[test]
fn the_ladder_has_no_top_end_and_every_glyph_size_is_on_it() {
    assert_eq!(scene::ladder(6.99), None);
    assert_eq!(scene::ladder(7.0), Some(7.0));
    assert_eq!(scene::ladder(7.69), Some(7.0));
    assert!((scene::ladder(7.7).unwrap() - 7.7).abs() < 1e-4);
    for size in [7.0, 9.3, 14.4, 33.0, 96.0, 144.0, 1000.0, 25_000.0] {
        let g = scene::ladder(size).unwrap();
        assert!(on_ladder(g), "{size}");
        assert!(
            g <= size * (1.0 + 1e-5) && size < g * 1.1 * (1.0 + 1e-5),
            "{size}: {g}"
        );
    }
}

/// The header items at a camera.
fn headers_at<'a>(map: &'a scene::Map, cam: &Camera) -> Vec<scene::Item<'a>> {
    scene::scene(map, cam, common::region(), &Options::default(), &NoDetails)
        .into_iter()
        .filter(|i| i.kind == Kind::Header)
        .collect()
}

#[test]
fn a_directory_name_is_its_world_size_times_the_zoom_and_greeks_below_7_points() {
    let map = common::quayside();
    let region = common::region();
    let b = Bounds { map: map.world() };
    let (mut text, mut greek) = (0, 0);
    for zoom in [b.fit(region).zoom, 0.05, 0.199, 0.5, 1.0, 3.0] {
        let cam = b.clamp(
            region,
            Camera {
                center: pos2(map.world().x * 0.4, map.world().y * 0.3),
                zoom,
            },
        );
        for i in headers_at(&map, &cam) {
            let h = map.entry(i.path).map(|e| match e {
                scene::Entry::Dir(n) => map.header_of(n),
                scene::Entry::File(_) => panic!("{} is a file", i.path),
            });
            let want = map.name_size(h.unwrap()) * cam.zoom;
            assert!(
                (i.size - want).abs() < 1e-3 * want,
                "{}: {} against {want}",
                i.path,
                i.size
            );
            match &i.body {
                Body::Str(_) => {
                    assert!(i.size >= scene::TEXT_FROM);
                    text += 1;
                }
                Body::Greek => {
                    assert!(i.size >= scene::GREEK_FROM && i.size < scene::TEXT_FROM);
                    // As wide as the text, half as high as its size.
                    assert!((i.rect.height() - 0.5 * i.size).abs() < 1e-3);
                    assert_eq!(i.color.a(), (0.35f32 * 255.0).round() as u8);
                    greek += 1;
                }
                b => panic!("{b:?}"),
            }
        }
        // A name under 2 points draws nothing.
        let shown: std::collections::HashSet<&str> =
            headers_at(&map, &cam).iter().map(|i| i.path).collect();
        for (h, hd) in map.layout.headers.iter().enumerate() {
            let size = map.name_size(h) * cam.zoom;
            let band = cam.rect_to_screen(region, map.band(h));
            if size < scene::GREEK_FROM && band.intersects(region) {
                assert!(!shown.contains(map.tree.nodes[hd.node].path.as_str()));
            }
        }
    }
    assert!(text > 10 && greek > 10, "{text} texts, {greek} bars");
}

#[test]
fn a_directory_name_fades_from_200_points() {
    assert_eq!(scene::fade(50.0), 1.0);
    assert_eq!(scene::fade(200.0), 1.0);
    assert!((scene::fade(250.0) - 0.5).abs() < 1e-6);
    assert_eq!(scene::fade(300.0), 0.0);
    assert_eq!(scene::fade(5000.0), 0.0);
    let map = common::quayside();
    // At zoom 2 the root name is far past 300 points and draws nothing; the names that draw carry their fade.
    let cam = Camera {
        center: map.band(0).center(),
        zoom: 2.0,
    };
    let items = headers_at(&map, &cam);
    assert!(
        items.iter().all(|i| !i.path.is_empty()),
        "the root name is gone"
    );
    for i in &items {
        assert_eq!(i.opacity, scene::fade(i.size));
    }
}

#[test]
fn a_block_fitted_in_the_window_shows_its_name() {
    // A zoom to a directory fits its block. Its name grows with the square root of its count of files (unit 062),
    // and its block grows about as fast, so every quayside name reads and stays under the fade, the root's too.
    let app = {
        let mut a = minimap_gui::GuiApp::new(common::quayside());
        a.set_region(common::region());
        a
    };
    let (mut least, mut most) = (f32::MAX, 0f32);
    for (h, hd) in app.map.layout.headers.iter().enumerate() {
        let name = app.map.header_name(h);
        let cam = app.camera_for(scene::Entry::Dir(hd.node));
        let size = app.map.name_size(h) * cam.zoom;
        assert!(size >= scene::TEXT_FROM, "{name} at {size}");
        assert_eq!(scene::fade(size), 1.0, "{name} at {size}");
        (least, most) = (least.min(size), most.max(size));
    }
    eprintln!("names after a zoom to their directory: {least:.1} to {most:.1} points");
}

#[test]
fn no_name_overlaps_a_card_and_each_name_stays_in_its_band() {
    let map = common::quayside();
    let region = common::region();
    let b = Bounds { map: map.world() };
    for zoom in [0.03, 0.199, 0.6, 2.0] {
        let cam = b.clamp(
            region,
            Camera {
                center: pos2(map.world().x * 0.45, map.world().y * 0.35),
                zoom,
            },
        );
        let items = scene::scene(&map, &cam, region, &Options::default(), &NoDetails);
        let cards: Vec<_> = items
            .iter()
            .filter(|i| matches!(i.kind, Kind::Card | Kind::Block))
            .map(|i| i.rect)
            .collect();
        for i in items.iter().filter(|i| i.kind == Kind::Header) {
            assert!(
                cards
                    .iter()
                    .all(|c| !c.shrink(0.01).intersects(i.rect.shrink(0.01))),
                "{} at {zoom}",
                i.path
            );
            let Some(scene::Entry::Dir(n)) = map.entry(i.path) else {
                panic!("{}", i.path)
            };
            let band = cam.rect_to_screen(region, map.band(map.header_of(n)));
            assert!(
                band.expand(0.01).contains_rect(i.rect),
                "{} at {zoom}",
                i.path
            );
        }
    }
}
