//! The file tree in the window (unit `042`): rows, one selection with the map, the map follow, and stability.

mod common;

use eframe::egui::{
    self, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, pos2, vec2,
};
use minimap_gui::GuiApp;
use minimap_gui::app::SHOW_MARGIN;
use minimap_gui::camera::Camera;
use minimap_gui::scene::Entry;
use minimap_gui::tree::{ROW_H, TREE_MAX, TREE_MIN, TREE_W};

fn app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    app
}

fn press(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn click(app: &mut GuiApp, at: Pos2, now: f64) {
    app.on_event(&press(at, true), Some(at), now);
    app.on_event(&press(at, false), Some(at), now);
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

fn wheel(unit: MouseWheelUnit, dy: f32) -> Event {
    Event::MouseWheel {
        unit,
        delta: vec2(0.0, dy),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    }
}

/// The screen point of an entry's tree row, or `None` when the row is not on screen.
fn row_point(app: &GuiApp, e: Entry) -> Option<Pos2> {
    let v = app.tree_view?;
    let i = app.tree_rows().iter().position(|r| r.entry == e)?;
    let k = i.checked_sub(app.filetree.top)?;
    (k < v.visible_rows()).then(|| v.row_rect(k).center())
}

/// The deepest file of quayside, first by path.
fn deep_file(app: &GuiApp) -> usize {
    let t = &app.map.tree;
    (0..t.files.len())
        .max_by_key(|&f| (t.nodes[t.files[f].node].depth, std::cmp::Reverse(f)))
        .unwrap()
}

/// Ends any ease.
fn settle(app: &mut GuiApp, now: f64) {
    app.motion.tick(now + 10.0);
}

#[test]
fn the_first_rows_are_the_root_directories_then_its_files() {
    let app = app();
    let t = &app.map.tree;
    let rows = app.tree_rows();
    let root = &t.nodes[0];
    let want: Vec<Entry> = root
        .kids
        .iter()
        .map(|&k| Entry::Dir(k))
        .chain(root.files.iter().map(|&f| Entry::File(f)))
        .collect();
    let got: Vec<Entry> = rows.iter().map(|r| r.entry).collect();
    assert_eq!(got, want);
    assert!(rows.iter().all(|r| r.depth == 0));
    // A directory row's count is every file under it that the map draws: ignored files have no row and no card.
    let shown = |prefix: &str| {
        t.files
            .iter()
            .filter(|f| f.path.starts_with(prefix) && !f.ignored)
            .count()
    };
    for &k in &root.kids {
        let path = format!("{}/", t.nodes[k].path);
        assert_eq!(app.filetree.count(k, true), shown(&path), "{path}");
    }
    assert_eq!(app.filetree.count(0, true), shown(""));
    // The default width, the head and hint rows, and the rows between them.
    let v = app.tree_view.unwrap();
    assert_eq!(v.rect.width(), TREE_W);
    assert_eq!(app.region.left(), TREE_W);
    assert_eq!(v.visible_rows(), ((952.0 - 2.0 * ROW_H) / ROW_H) as usize);
    assert_eq!(app.filetree.height, v.visible_rows());
    assert_eq!(app.selection, None);
}

#[test]
fn a_map_click_opens_the_tree_to_the_card_and_scrolls_its_row_into_view() {
    let mut app = app();
    let f = deep_file(&app);
    let cam = Camera {
        center: app.map.card(f).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    let before = app.cam();
    let at = before.rect_to_screen(app.region, app.map.card(f)).center();
    assert!(
        row_point(&app, Entry::File(f)).is_none(),
        "closed at the start"
    );
    click(&mut app, at, 1.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert_eq!(app.filetree.cursor, Some(Entry::File(f)));
    assert!(
        row_point(&app, Entry::File(f)).is_some(),
        "the row is on screen"
    );
    // Every directory above the file is open, and the map does not move.
    let t = &app.map.tree;
    let mut n = t.files[f].node;
    while n != 0 {
        assert!(app.filetree.is_open(n), "{}", t.nodes[n].path);
        n = (0..t.nodes.len())
            .find(|&p| t.nodes[p].kids.contains(&n))
            .unwrap();
    }
    assert!(!app.motion.easing());
    assert_eq!(app.cam(), before);
}

#[test]
fn a_tree_click_selects_the_file_and_the_map_moves_only_as_needed() {
    let mut app = app();
    let f = deep_file(&app);
    // Open the tree to the file without the map, then look far from it at card width 200.
    app.select(Some(Entry::File(f)), false, 0.0);
    app.select(None, false, 0.0);
    let zoom = 200.0 / app.map.card_w();
    app.motion.jump(app.bounds.clamp(
        app.region,
        Camera {
            center: pos2(0.0, 0.0),
            zoom,
        },
    ));
    let at = row_point(&app, Entry::File(f)).unwrap();
    click(&mut app, at, 1.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert!(app.motion.easing());
    settle(&mut app, 1.0);
    let cam = app.cam();
    assert_eq!(cam.zoom, zoom, "no zoom change");
    let card = cam.rect_to_screen(app.region, app.map.card(f));
    let inner = app.region.shrink(SHOW_MARGIN - 1e-2);
    assert!(inner.contains_rect(card), "{card:?} in {inner:?}");
    // It moved only as far as needed: one edge of the card is on the margin.
    let gaps = [
        card.left() - app.region.left(),
        app.region.right() - card.right(),
        card.top() - app.region.top(),
        app.region.bottom() - card.bottom(),
    ];
    assert!(
        gaps.iter().any(|g| (g - SHOW_MARGIN).abs() < 0.1),
        "{gaps:?}"
    );
    // A card that is already inside the margins does not move the map: a file row on screen near this one, with its card centered.
    let rows = app.tree_rows();
    let i = rows.iter().position(|r| r.entry == Entry::File(f)).unwrap();
    let mut by_distance: Vec<usize> = (0..rows.len()).filter(|&j| j != i).collect();
    by_distance.sort_by_key(|&j| j.abs_diff(i));
    let near = by_distance
        .into_iter()
        .map(|j| rows[j].entry)
        .find(|&e| matches!(e, Entry::File(_)) && row_point(&app, e).is_some())
        .expect("another file row on screen");
    let centered = Camera {
        center: app.map.rect_of(near).center(),
        zoom,
    };
    app.motion.jump(app.bounds.clamp(app.region, centered));
    let before = app.cam();
    let at = row_point(&app, near).unwrap();
    click(&mut app, at, 5.0);
    assert_eq!(app.selection, Some(near));
    assert!(!app.motion.easing());
    assert_eq!(app.cam(), before);
}

#[test]
fn a_tree_click_on_a_directory_opens_or_closes_it_and_shows_the_block_top() {
    let mut app = app();
    let zoom = 200.0 / app.map.card_w();
    let world = app.map.world();
    app.motion.jump(app.bounds.clamp(
        app.region,
        Camera {
            center: world.to_pos2(),
            zoom,
        },
    ));
    let n = app.map.tree.nodes[0].kids[0];
    assert!(!app.filetree.is_open(n));
    let at = row_point(&app, Entry::Dir(n)).unwrap();
    click(&mut app, at, 1.0);
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    assert!(app.filetree.is_open(n));
    settle(&mut app, 1.0);
    let cam = app.cam();
    assert_eq!(cam.zoom, zoom);
    // The block is larger than the region, so its top-left corner shows at the margin.
    let block = cam.rect_to_screen(app.region, app.map.rect_of(Entry::Dir(n)));
    assert!(block.height() > app.region.height());
    assert!(
        (block.top() - (app.region.top() + SHOW_MARGIN)).abs() < 0.1,
        "{block:?}"
    );
    assert!(
        (block.left() - (app.region.left() + SHOW_MARGIN)).abs() < 0.1,
        "{block:?}"
    );
    // A second click later closes it.
    let at = row_point(&app, Entry::Dir(n)).unwrap();
    click(&mut app, at, 3.0);
    assert!(!app.filetree.is_open(n));
}

#[test]
fn a_double_click_on_a_row_zooms_to_its_entry() {
    for e in [
        Entry::Dir(GuiApp::new(common::quayside()).map.tree.nodes[0].kids[1]),
        Entry::File(GuiApp::new(common::quayside()).map.tree.nodes[0].files[0]),
    ] {
        let mut app = app();
        let at = row_point(&app, e).unwrap();
        click(&mut app, at, 1.0);
        click(&mut app, at, 1.2);
        assert_eq!(app.motion.target(), app.camera_for(e), "{e:?}");
        if let Entry::Dir(n) = e {
            // The first click opened it; the click that completes the double-click does not close it.
            assert!(app.filetree.is_open(n));
        }
    }
}

/// The screen points of three world points: the map content's place on the screen.
fn content(app: &GuiApp) -> [Pos2; 3] {
    let w = app.map.world();
    [pos2(0.3, 0.3), pos2(0.5, 0.5), pos2(0.6, 0.4)]
        .map(|k| app.cam().to_screen(app.region, pos2(k.x * w.x, k.y * w.y)))
}

fn assert_in_place(a: [Pos2; 3], b: [Pos2; 3], what: &str) {
    for (p, q) in a.iter().zip(&b) {
        assert!(p.distance(*q) < 1e-2, "{what}: {p:?} to {q:?}");
    }
}

#[test]
fn the_tree_never_moves_a_card_and_keeps_the_map_content_in_place() {
    let mut app = app();
    let cards: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    let cam = Camera {
        center: (app.map.world() / 2.0).to_pos2(),
        zoom: 80.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    let start = content(&app);
    app.on_event(&key(Key::T), None, 1.0);
    assert!(!app.tree_on && app.tree_view.is_none());
    assert_eq!(app.region.left(), 0.0);
    assert_in_place(start, content(&app), "t hides");
    app.on_event(&key(Key::T), None, 1.1);
    assert_eq!(app.region.left(), TREE_W);
    assert_in_place(start, content(&app), "t shows");
    app.on_event(&key(Key::CloseBracket), None, 1.2);
    assert_eq!(app.tree_width(), TREE_W + 40.0);
    assert_in_place(start, content(&app), "] widens");
    app.on_event(&key(Key::OpenBracket), None, 1.3);
    app.on_event(&key(Key::OpenBracket), None, 1.4);
    assert_eq!(app.tree_width(), TREE_W - 40.0);
    assert_in_place(start, content(&app), "[ narrows");
    // A drag on the edge sets the width, within its limits.
    let edge = app.tree_view.unwrap().edge.center();
    app.on_event(&press(edge, true), Some(edge), 2.0);
    for x in [edge.x + 100.0, edge.x + 2000.0, 0.0, edge.x + 60.0] {
        let p = pos2(x, edge.y);
        app.on_event(&Event::PointerMoved(p), Some(p), 2.1);
        let w = app.tree_width();
        assert!((TREE_MIN..=TREE_MAX).contains(&w), "{w}");
        assert_in_place(start, content(&app), "an edge drag");
    }
    let end = pos2(edge.x + 60.0, edge.y);
    app.on_event(&press(end, false), Some(end), 2.2);
    assert_eq!(app.tree_width(), TREE_W - 40.0 + 60.0);
    assert_eq!(app.selection, None, "a drag on the edge selects nothing");
    // The map's cards never moved in the world.
    let after: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    assert_eq!(cards, after);
    // A narrow window narrows the tree, down to its least width, so the map keeps 240 points.
    // With the selection panel shown, the panel narrows first (unit 044, `tests/panel.rs`).
    app.set_panel(false, app.panel_w);
    app.set_screen(Rect::from_min_size(Pos2::ZERO, vec2(500.0, 600.0)));
    assert_eq!(app.tree_width(), 260.0);
    app.set_screen(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 600.0)));
    assert_eq!(app.tree_width(), TREE_MIN);
}

#[test]
fn the_wheel_over_the_tree_scrolls_its_rows_and_not_the_map() {
    let mut app = app();
    // Open every directory, so the rows are longer than the tree.
    for n in 1..app.map.tree.nodes.len() {
        app.set_open(n, true);
    }
    let len = app.tree_rows().len();
    assert!(len > 2 * app.filetree.height);
    let cam = app.cam();
    let over = app.tree_view.unwrap().rows.center();
    app.on_event(&wheel(MouseWheelUnit::Line, -1.0), Some(over), 1.0);
    assert_eq!(app.filetree.top, 3);
    // Trackpad points scroll by the row height, keeping the rest for the next scroll.
    app.on_event(&wheel(MouseWheelUnit::Point, -30.0), Some(over), 1.1);
    assert_eq!(app.filetree.top, 4);
    app.on_event(&wheel(MouseWheelUnit::Point, -10.0), Some(over), 1.2);
    assert_eq!(app.filetree.top, 5);
    app.on_event(&wheel(MouseWheelUnit::Line, 10.0), Some(over), 1.3);
    assert_eq!(app.filetree.top, 0);
    app.on_event(&wheel(MouseWheelUnit::Line, -10_000.0), Some(over), 1.4);
    assert_eq!(app.filetree.top, len - app.filetree.height);
    assert_eq!(app.cam(), cam, "the map did not move");
    // The wheel over the map still zooms it.
    let map = app.region.center();
    app.on_event(&wheel(MouseWheelUnit::Line, 1.0), Some(map), 1.5);
    assert!(app.cam().zoom > cam.zoom);
}

/// One headless frame of the whole window (`GuiApp::frame`), with these events at a time in seconds.
fn frame(ctx: &egui::Context, app: &mut GuiApp, events: Vec<Event>, time: f64) -> egui::FullOutput {
    let input = egui::RawInput {
        screen_rect: Some(common::screen()),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
    // No renderer takes the font atlas's texture deltas.
    out.textures_delta.clear();
    out
}

#[test]
fn headless_frames_click_a_row_and_drag_the_edge() {
    let ctx = egui::Context::default();
    // The app's font families, with egui's built-in fonts.
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = GuiApp::new(common::quayside());
    let _ = frame(&ctx, &mut app, vec![], 0.0);
    assert_eq!(app.region.left(), TREE_W, "the frame laid out the tree");
    let n = app.map.tree.nodes[0].kids[0];
    let at = row_point(&app, Entry::Dir(n)).unwrap();
    let _ = frame(&ctx, &mut app, vec![Event::PointerMoved(at)], 0.1);
    assert_eq!(
        app.hover,
        Some(Entry::Dir(n)),
        "a hovered row hovers its entry"
    );
    let _ = frame(&ctx, &mut app, vec![press(at, true)], 0.2);
    let _ = frame(&ctx, &mut app, vec![press(at, false)], 0.3);
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    assert!(app.filetree.is_open(n));
    // Over the edge, the cursor is a resize cursor; a drag sets the width.
    let edge = app.tree_view.unwrap().edge.center();
    let out = frame(&ctx, &mut app, vec![Event::PointerMoved(edge)], 1.0);
    assert_eq!(
        out.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    let to = edge + vec2(100.0, 0.0);
    let _ = frame(&ctx, &mut app, vec![press(edge, true)], 1.1);
    let out = frame(&ctx, &mut app, vec![Event::PointerMoved(to)], 1.2);
    assert_eq!(
        out.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    let out = frame(&ctx, &mut app, vec![press(to, false)], 1.3);
    assert_eq!(app.tree_width(), TREE_W + 100.0);
    assert_eq!(app.region.left(), TREE_W + 100.0);
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    // The frame tessellates.
    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    assert!(!prims.is_empty());
}
