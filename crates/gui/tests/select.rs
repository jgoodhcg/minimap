//! Hover, selection, and zoom to a card (unit `038`).

mod common;

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use minimap_gui::GuiApp;
use minimap_gui::app::{ZOOM_TO_EASE, ZOOM_TO_WIDTH, crumb_line};
use minimap_gui::camera::Camera;
use minimap_gui::scene::{self, Entry, Kind, NoDetails, Options};

fn app_at(width: f32, center: Pos2) -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_region(common::region());
    let cam = Camera {
        center,
        zoom: width / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app
}

/// A file with neighbors on its right and below, so the gaps around it are between cards,
/// in the first row of its directory's grid, so its directory's header is just above it. The root's name is too large to draw near its files.
fn inner_card(app: &GuiApp) -> usize {
    let layout = &app.map.layout;
    let tiles = &layout.tiles;
    let tw = layout.cfg.tw;
    (0..tiles.len())
        .find(|&f| {
            let (x, y) = tiles[f];
            // Cards are as tall as their lines (unit 039): the card below starts one gap row under this one.
            let below = (x, y + layout.tile_h[f] + 1);
            let h = &layout.headers[app.map.header_of(app.map.tree.files[f].node)];
            let node = app.map.tree.files[f].node;
            node != 0 && tiles.contains(&(x + tw, y)) && tiles.contains(&below) && y == h.y + h.band
        })
        .unwrap()
}

#[test]
fn a_point_hits_the_right_card_header_or_nothing_at_three_zooms() {
    let probe = GuiApp::new(common::quayside());
    let f = inner_card(&probe);
    let center = probe.map.card(f).center();
    for width in [12.0, 80.0, 300.0] {
        let app = app_at(width, center);
        let cam = app.cam();
        let card = cam.rect_to_screen(app.region, app.map.card(f));
        assert_eq!(
            app.hit(card.center()),
            Some(Entry::File(f)),
            "width {width}"
        );
        assert_eq!(app.hit(card.min + vec2(0.5, 0.5)), Some(Entry::File(f)));
        // The gap right of the card and the gap below it hit nothing.
        let gap = cam.zoom * 8.0;
        assert_eq!(
            app.hit(pos2(card.right() + gap / 2.0, card.center().y)),
            None,
            "the gap right of the card at width {width}"
        );
        assert_eq!(
            app.hit(pos2(card.center().x, card.bottom() + 9.0 * cam.zoom)),
            None,
            "the gap below the card at width {width}"
        );
        // Every drawn header name hits its directory.
        let items = app.items();
        let mut names = 0;
        for i in items
            .iter()
            .filter(|i| i.kind == Kind::Header && app.region.contains(i.rect.center()))
        {
            let want = app.map.entry(i.path).unwrap();
            assert!(matches!(want, Entry::Dir(_)));
            assert_eq!(
                app.hit(i.rect.center()),
                Some(want),
                "{} at {width}",
                i.path
            );
            names += 1;
        }
        assert!(names > 0, "width {width} draws names");
    }
}

fn press(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
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

fn click(app: &mut GuiApp, at: Pos2, now: f64) {
    app.on_event(&press(at, true), Some(at), now);
    app.on_event(&press(at, false), Some(at), now + 0.05);
}

#[test]
fn a_click_selects_and_a_press_that_moves_5_points_pans() {
    let probe = GuiApp::new(common::quayside());
    let f = inner_card(&probe);
    let mut app = app_at(200.0, probe.map.card(f).center());
    let at = app
        .cam()
        .rect_to_screen(app.region, app.map.card(f))
        .center();
    click(&mut app, at, 1.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
    // A press that moves 5 points pans, and keeps the selection.
    app.on_event(&key(Key::Escape), None, 2.0);
    assert_eq!(app.selection, None);
    let before = app.cam();
    app.on_event(&press(at, true), Some(at), 3.0);
    let to = at + vec2(5.0, 0.0);
    app.on_event(&Event::PointerMoved(to), Some(to), 3.02);
    app.on_event(&press(to, false), Some(to), 3.05);
    assert_eq!(app.selection, None, "a pan does not select");
    let moved = (app.cam().center - before.center) * app.cam().zoom;
    assert!((moved - vec2(-5.0, 0.0)).length() < 1e-3, "{moved:?}");
    // A press that moves 3 points is still a click.
    app.on_event(&press(at, true), Some(at), 4.0);
    let near = at + vec2(3.0, 0.0);
    app.on_event(&Event::PointerMoved(near), Some(near), 4.02);
    app.on_event(&press(near, false), Some(near), 4.05);
    assert_eq!(app.selection, Some(Entry::File(f)));
    // A click on empty map keeps the selection.
    let empty = (0..1000)
        .map(|k| {
            pos2(
                app.region.left() + 1.0 + k as f32 * 1.6,
                app.region.top() + 2.0,
            )
        })
        .find(|&p| app.hit(p).is_none())
        .unwrap();
    click(&mut app, empty, 5.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
}

#[test]
fn a_double_click_zooms_to_the_card_and_f_zooms_to_the_selection() {
    let probe = GuiApp::new(common::quayside());
    let f = inner_card(&probe);
    let mut app = app_at(80.0, probe.map.card(f).center());
    let at = app
        .cam()
        .rect_to_screen(app.region, app.map.card(f))
        .center();
    click(&mut app, at, 1.0);
    assert!(!app.motion.easing());
    click(&mut app, at, 1.2);
    assert!(app.motion.easing());
    let target = app.motion.target();
    let want = app.camera_for(Entry::File(f));
    assert!((want.zoom - ZOOM_TO_WIDTH / app.map.card_w()).abs() < 1e-6);
    assert!((target.center - want.center).length() < 1e-3);
    assert!((target.zoom - want.zoom).abs() < 1e-6);
    // It eases over 240 ms from the second release.
    assert!(app.motion.tick(1.25 + ZOOM_TO_EASE / 2.0));
    assert!(!app.motion.tick(1.25 + ZOOM_TO_EASE));
    // Two clicks too far apart in time do not zoom.
    let mut slow = app_at(80.0, probe.map.card(f).center());
    click(&mut slow, at, 1.0);
    click(&mut slow, at, 1.6);
    assert!(!slow.motion.easing());
    // `f` with no selection does nothing; with one, it zooms to it.
    let mut none = app_at(80.0, probe.map.card(f).center());
    let cam = none.cam();
    none.on_event(&key(Key::F), None, 1.0);
    assert!(!none.motion.easing());
    assert_eq!(none.cam(), cam);
    slow.on_event(&key(Key::F), None, 2.0);
    assert!((slow.motion.target().zoom - want.zoom).abs() < 1e-6);
}

#[test]
fn a_double_click_on_a_header_fits_its_block() {
    let probe = GuiApp::new(common::quayside());
    let h = probe
        .map
        .layout
        .headers
        .iter()
        .position(|h| h.depth == 3)
        .unwrap();
    let mut app = app_at(300.0, probe.map.band(h).left_center() + vec2(200.0, 0.0));
    let probe_cam = app.cam();
    let items = app.items();
    let header = items
        .iter()
        .find(|i| i.kind == Kind::Header && !i.path.is_empty())
        .unwrap();
    let (at, path) = (header.rect.center(), header.path.to_string());
    let Some(Entry::Dir(n)) = app.map.entry(&path) else {
        panic!("{path} is a directory");
    };
    click(&mut app, at, 1.0);
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    click(&mut app, at, 1.1);
    let block = app.map.rect_of(Entry::Dir(n));
    let want = app.bounds.fit_rect(app.region, block);
    assert_eq!(app.motion.target(), want);
    // The selected block gets its fill, and its name is white where the zoom draws it.
    app.motion.tick(5.0);
    assert!(
        app.items()
            .iter()
            .any(|i| i.kind == Kind::Selected && i.path == path)
    );
    app.motion.jump(probe_cam);
    let items = app.items();
    assert!(
        items
            .iter()
            .any(|i| i.kind == Kind::Selected && i.path == path)
    );
    let white = minimap_gui::color::rgb(minimap_core::palette::WHITE);
    assert!(
        items
            .iter()
            .any(|i| i.kind == Kind::Header && i.path == path && i.color == white)
    );
}

#[test]
fn the_selected_card_has_a_ring_and_a_white_icon_and_the_hovered_card_is_lighter() {
    let map = common::quayside();
    let f = inner_card(&GuiApp::new(common::quayside()));
    let region = common::region();
    let cam = Camera {
        center: map.card(f).center(),
        zoom: 200.0 / map.card_w(),
    };
    let path = map.tree.files[f].path.as_str();
    let opts = Options {
        selected: Some(Entry::File(f)),
        hovered: Some(Entry::File(f)),
        ..Options::default()
    };
    let items = scene::scene(&map, &cam, region, &opts, &NoDetails);
    let mine: Vec<_> = items.iter().filter(|i| i.path == path).collect();
    let rgb = |c| minimap_gui::color::rgb(c);
    use minimap_core::palette::{SEL_HI, TILE_HI, WHITE};
    let ring = mine.iter().find(|i| i.kind == Kind::Ring).unwrap();
    assert_eq!(
        (ring.color, &ring.body),
        (rgb(SEL_HI), &scene::Body::Stroke(2.0))
    );
    assert_eq!(
        mine.iter().find(|i| i.kind == Kind::Icon).unwrap().color,
        rgb(WHITE)
    );
    assert_eq!(
        mine.iter().find(|i| i.kind == Kind::Card).unwrap().color,
        rgb(TILE_HI)
    );
    // Selection and hover never change a card's world rect, or its screen rect.
    let plain = scene::scene(&map, &cam, region, &Options::default(), &NoDetails);
    let rect = |items: &[scene::Item], kind| -> Vec<Rect> {
        items
            .iter()
            .filter(|i| i.kind == kind)
            .map(|i| i.rect)
            .collect()
    };
    assert_eq!(rect(&items, Kind::Card), rect(&plain, Kind::Card));
    assert_eq!(rect(&items, Kind::Icon), rect(&plain, Kind::Icon));
}

#[test]
fn selection_and_hover_never_change_a_card_world_rect() {
    let probe = GuiApp::new(common::quayside());
    let f = inner_card(&probe);
    let mut app = app_at(200.0, probe.map.card(f).center());
    let all: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|k| app.map.card(k))
        .collect();
    let at = app
        .cam()
        .rect_to_screen(app.region, app.map.card(f))
        .center();
    click(&mut app, at, 1.0);
    app.hover = Some(Entry::File(f));
    app.on_event(&key(Key::Escape), None, 2.0);
    let now: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|k| app.map.card(k))
        .collect();
    assert_eq!(all, now);
}

#[test]
fn the_breadcrumb_reads_as_the_tui_writes_it() {
    let app = GuiApp::new(common::quayside());
    let t = &app.map.tree;
    let f = t
        .files
        .iter()
        .position(|x| x.path == "components/berth/src/quayside/berth/scoring.clj")
        .unwrap();
    let bc = t.file_crumb(f);
    assert_eq!(
        bc.text(),
        "quayside › components › berth › src › quayside › berth › scoring.clj"
    );
    assert_eq!(bc.bright(), "scoring.clj");
    let (line, at) = crumb_line(&bc, 200);
    assert_eq!(line, bc.text());
    assert_eq!(at, Some(line.chars().count() - "scoring.clj".len()));
    // Cut at the left, as the TUI cuts it, and the file name stays bright.
    let (cut, at) = crumb_line(&bc, 30);
    assert_eq!(cut.chars().count(), 30);
    assert!(
        cut.starts_with('…') && cut.ends_with("berth › scoring.clj"),
        "{cut}"
    );
    assert_eq!(at, Some(30 - "scoring.clj".len()));
    let Some(Entry::Dir(n)) = app.map.entry("components/berth/src/quayside/berth/") else {
        panic!("a directory");
    };
    let bc = t.dir_crumb(n);
    assert_eq!(
        bc.text(),
        "quayside › components › berth › src › quayside › berth"
    );
    assert_eq!(bc.bright(), "berth");
    assert_eq!(t.root_crumb().text(), "quayside");
}

#[test]
fn a_click_anywhere_in_a_header_band_selects_its_directory() {
    let probe = GuiApp::new(common::quayside());
    let h = probe
        .map
        .layout
        .headers
        .iter()
        .position(|h| h.depth == 2 && h.band > 2)
        .unwrap();
    let node = probe.map.layout.headers[h].node;
    let band = probe.map.band(h);
    for zoom_w in [12.0, 80.0, 300.0] {
        let mut app = app_at(zoom_w, band.center());
        let cam = app.cam();
        // The band's right end, or the region's: the name spans 30 % of the block from its left, so no name is there.
        let end = cam.to_screen(app.region, band.right_center());
        let at = pos2(end.x.min(app.region.right()) - 4.0, end.y);
        assert_eq!(app.hit(at), Some(Entry::Dir(node)), "{zoom_w}");
        click(&mut app, at, 1.0);
        assert_eq!(app.selection, Some(Entry::Dir(node)));
    }
}
