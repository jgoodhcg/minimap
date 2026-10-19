//! Outline clicks in the selection panel (unit `046`): the pick, the zoom to its line, the card's mark, the outline's scroll,
//! and clicks on a directory's largest files.

mod common;

use eframe::egui::{self, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, vec2};
use minimap_core::details::{Details, Loader};
use minimap_core::palette;
use minimap_gui::GuiApp;
use minimap_gui::app::{SHOW_MARGIN, ZOOM_TO_TOP, ZOOM_TO_WIDTH};
use minimap_gui::camera::Camera;
use minimap_gui::panel::{Draw, GitHistory, Hit, OUTLINE_LABEL};
use minimap_gui::scene::{CardGeometry, Entry, Kind, MARK_MIN};

/// 276 lines and 17 outline items: its card fits the region at card width 420.
const SHORT: &str = "components/berth/src/quayside/berth/state.clj";
/// 547 lines and 43 outline items: its card is taller than the region at card width 420.
const TALL: &str = "components/berth/src/quayside/berth/core.clj";

/// The quayside map in the capture window's screen, with every file's details and the fixture's git history.
fn app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    // The map region of unit 046, without the agent history below it (unit 051), so SHORT's card fits at card width 420.
    app.set_activity(false);
    let text = common::fixture("quayside-details.json");
    let mut loader = Loader::with_workers(Details::from_json(&text).unwrap(), || {}, 0);
    loader.request(app.map.tree.files.iter().map(|f| f.path.clone()));
    loader.poll();
    app.loader = Some(loader);
    app.git = GitHistory::Ready(minimap_core::History::from_json(&text, 1_767_225_600).unwrap());
    app
}

fn file(app: &GuiApp, path: &str) -> usize {
    match app.map.entry(path) {
        Some(Entry::File(f)) => f,
        e => panic!("{path}: {e:?}"),
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

fn click(app: &mut GuiApp, at: Pos2, now: f64) {
    app.on_event(&press(at, true), Some(at), now);
    app.on_event(&press(at, false), Some(at), now);
}

/// Clicks a panel row, which must be on the screen.
fn click_hit(app: &mut GuiApp, h: Hit, now: f64) {
    let r = app
        .panel()
        .unwrap()
        .rect_of(h)
        .expect("the row is on screen");
    click(app, r.center(), now);
}

fn settle(app: &mut GuiApp, now: f64) {
    app.motion.tick(now + 10.0);
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

fn wheel(dy: f32) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, dy),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn a_click_on_an_item_of_a_tall_card_puts_its_line_near_the_regions_top() {
    let mut app = app();
    let f = file(&app, TALL);
    app.select(Some(Entry::File(f)), false, 0.0);
    let zoom = ZOOM_TO_WIDTH / app.map.card_w();
    assert!(
        app.map.card(f).height() * zoom > app.region.height(),
        "a tall card"
    );
    let k = 9;
    let line = app.outline()[k].line;
    click_hit(&mut app, Hit::Outline(k), 1.0);
    assert_eq!(app.pick, Some(k));
    assert!(app.motion.easing());
    settle(&mut app, 1.0);
    let cam = app.cam();
    assert_eq!(cam.zoom, zoom);
    let card = cam.rect_to_screen(app.region, app.map.card(f));
    assert!(
        (card.center().x - app.region.center().x).abs() < 0.1,
        "centered across"
    );
    let y = cam
        .to_screen(
            app.region,
            eframe::egui::pos2(0.0, minimap_gui::scene::line_top(&app.map, f, line)),
        )
        .y;
    assert!(
        (y - (app.region.top() + ZOOM_TO_TOP)).abs() < 0.1,
        "line {line} at {y}"
    );
    assert_eq!(
        app.selection,
        Some(Entry::File(f)),
        "the pick keeps the selection"
    );
}

#[test]
fn a_click_on_an_item_of_a_short_card_centers_the_card() {
    let mut app = app();
    let f = file(&app, SHORT);
    app.select(Some(Entry::File(f)), false, 0.0);
    let zoom = ZOOM_TO_WIDTH / app.map.card_w();
    assert!(
        app.map.card(f).height() * zoom <= app.region.height(),
        "a short card"
    );
    click_hit(&mut app, Hit::Outline(3), 1.0);
    assert_eq!(app.motion.target(), app.card_camera(f, zoom));
}

#[test]
fn the_card_marks_the_picked_line_from_the_shape_level_with_the_shape_on() {
    let mut app = app();
    let f = file(&app, TALL);
    let cards: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|c| app.map.card(c))
        .collect();
    app.select(Some(Entry::File(f)), false, 0.0);
    let k = 9;
    let line = app.outline()[k].line;
    click_hit(&mut app, Hit::Outline(k), 1.0);
    settle(&mut app, 1.0);
    let marks = |app: &GuiApp| -> Vec<Rect> {
        app.items()
            .into_iter()
            .filter(|i| i.kind == Kind::Mark)
            .map(|i| i.rect)
            .collect()
    };
    let m = marks(&app);
    assert_eq!(m.len(), 1);
    let cam = app.cam();
    let card = cam.rect_to_screen(app.region, app.map.card(f));
    let g = CardGeometry::new(card);
    let line_h = app.map.line_pt() * cam.zoom;
    // Inside the left padding, centered on the line, as tall as a line and at least 2 points.
    assert!(
        m[0].left() > card.left() && m[0].right() < g.body.left(),
        "{:?} in {card:?}",
        m[0]
    );
    let cy = g.body.top() + (line as f32 - 0.5) * line_h;
    assert!((m[0].center().y - cy).abs() < 1e-2, "{:?} at {cy}", m[0]);
    assert!((m[0].height() - line_h.max(MARK_MIN)).abs() < 1e-3);
    // With the shape off, or below the shape level, no mark.
    app.opts.toggles.shape = false;
    assert!(marks(&app).is_empty());
    app.opts.toggles.shape = true;
    let small = Camera {
        zoom: 50.0 / app.map.card_w(),
        ..app.cam()
    };
    app.motion.jump(app.bounds.clamp(app.region, small));
    assert!(marks(&app).is_empty());
    // No card moved in the world.
    let after: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|c| app.map.card(c))
        .collect();
    assert_eq!(cards, after);
}

#[test]
fn another_selection_or_esc_clears_the_pick_and_a_second_click_moves_the_map_again() {
    let mut app = app();
    let f = file(&app, TALL);
    app.select(Some(Entry::File(f)), false, 0.0);
    click_hit(&mut app, Hit::Outline(9), 1.0);
    settle(&mut app, 1.0);
    let at_line = app.cam();
    // Pan away, then click the picked row again: the map goes back to the line.
    app.pan_view(0.0, 0.4);
    assert_ne!(app.cam(), at_line);
    click_hit(&mut app, Hit::Outline(9), 5.0);
    assert_eq!(app.pick, Some(9));
    assert_eq!(app.motion.target(), at_line);
    // The same selection keeps the pick; another selection clears it.
    app.select(Some(Entry::File(f)), false, 6.0);
    assert_eq!(app.pick, Some(9));
    let other = file(&app, SHORT);
    app.select(Some(Entry::File(other)), false, 6.1);
    assert_eq!(app.pick, None);
    assert!(app.picked_line().is_none());
    click_hit(&mut app, Hit::Outline(2), 7.0);
    assert_eq!(app.pick, Some(2));
    app.on_event(&key(Key::Escape), None, 8.0);
    assert_eq!(app.pick, None);
    assert_eq!(app.selection, None);
}

#[test]
fn the_wheel_over_the_outline_scrolls_it_within_its_limits() {
    let mut app = app();
    let f = file(&app, TALL);
    app.select(Some(Entry::File(f)), false, 0.0);
    let cam = app.cam();
    let o = app.panel().unwrap().outline.unwrap();
    assert_eq!(o.top, 0);
    assert!(o.max_top > 3, "43 items do not fit: {o:?}");
    let over = o.rows.center();
    app.on_event(&wheel(-1.0), Some(over), 1.0);
    assert_eq!(app.outline_top, 3);
    let p = app.panel().unwrap();
    let t = p.texts();
    assert!(t.contains(&"3 above"), "{t:#?}");
    let first = format!("{:>4}  {}", app.outline()[3].line, app.outline()[3].name);
    let label = t.iter().position(|s| *s == OUTLINE_LABEL).unwrap();
    assert_eq!(t[label + 2], first, "the label, its note, then item 3");
    let shown = o.rows.height() / 20.0;
    let more = format!("  … {} more", 43 - 3 - shown as usize);
    assert!(t.contains(&more.as_str()), "{more} in {t:#?}");
    app.on_event(&wheel(-100.0), Some(over), 1.1);
    assert_eq!(app.outline_top, o.max_top);
    assert!(
        !app.panel()
            .unwrap()
            .texts()
            .iter()
            .any(|s| s.starts_with("  … ")),
        "no more row at the end"
    );
    app.on_event(&wheel(100.0), Some(over), 1.2);
    assert_eq!(app.outline_top, 0);
    assert_eq!(app.cam(), cam, "the map did not move");
    // The wheel over the panel outside the outline does nothing.
    app.on_event(
        &wheel(-1.0),
        Some(app.panel_view.unwrap().rect.left_top() + vec2(30.0, 10.0)),
        1.3,
    );
    assert_eq!(app.outline_top, 0);
    // A new selection shows the outline from its first item.
    app.on_event(&wheel(-1.0), Some(over), 1.4);
    assert_eq!(app.outline_top, 3);
    app.select(Some(Entry::File(file(&app, SHORT))), false, 2.0);
    assert_eq!(app.outline_top, 0);
}

#[test]
fn a_click_on_a_largest_file_selects_it_and_a_double_click_zooms_to_it() {
    let mut app = app();
    let Some(Entry::Dir(n)) = app.map.entry("components") else {
        panic!()
    };
    app.select(Some(Entry::Dir(n)), false, 0.0);
    let zoom = 200.0 / app.map.card_w();
    app.motion.jump(app.bounds.clamp(
        app.region,
        Camera {
            center: eframe::egui::pos2(0.0, 0.0),
            zoom,
        },
    ));
    let p = app.panel().unwrap();
    let Some(&(r, Hit::File(f))) = p.hits.first() else {
        panic!("{:?}", p.hits)
    };
    click(&mut app, r.center(), 1.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert_eq!(app.filetree.cursor, Some(Entry::File(f)));
    assert!(
        app.tree_rows()
            .iter()
            .any(|row| row.entry == Entry::File(f)),
        "the tree opened to it"
    );
    settle(&mut app, 1.0);
    assert_eq!(app.cam().zoom, zoom, "no zoom change");
    let card = app.cam().rect_to_screen(app.region, app.map.card(f));
    let inner = app.region.shrink(SHOW_MARGIN - 1e-2);
    if card.height() <= inner.height() {
        assert!(inner.contains_rect(card), "{card:?} in {inner:?}");
    } else {
        assert!(inner.contains(card.min), "{card:?} in {inner:?}");
    }
    // A double-click: the first click selects the file, so its panel replaces the directory's under the pointer,
    // and the second click there zooms to the file. It picks no outline item.
    app.select(Some(Entry::Dir(n)), false, 2.0);
    click(&mut app, r.center(), 3.0);
    click(&mut app, r.center(), 3.2);
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert_eq!(app.motion.target(), app.camera_for(Entry::File(f)));
    assert_eq!(app.pick, None);
}

#[test]
fn the_panel_draws_the_pick_on_its_row_and_its_shape() {
    let mut app = app();
    let f = file(&app, SHORT);
    app.select(Some(Entry::File(f)), false, 0.0);
    click_hit(&mut app, Hit::Outline(2), 1.0);
    let p = app.panel().unwrap();
    let row = p.rect_of(Hit::Outline(2)).unwrap();
    let item = &app.outline()[2];
    let text = format!("{:>4}  {}", item.line, item.name);
    // The row's fill, then its text in white.
    let fill = p
        .draws
        .iter()
        .position(|d| matches!(d, Draw::Fill { rect, color } if *rect == row && *color == palette::CURSOR))
        .expect("the picked row's fill");
    let at = p.draws.iter().position(|d| matches!(d, Draw::Text { text: t, color, .. } if *t == text && *color == palette::WHITE));
    assert!(at.is_some_and(|a| a > fill), "{text}");
    // The shape: `◀` in white in place of the item's tick, and one white bar.
    assert!(p.texts().contains(&"◀"));
    let white_bars = p
        .draws
        .iter()
        .filter(|d| matches!(d, Draw::Fill { color, .. } if *color == palette::WHITE))
        .count();
    assert_eq!(white_bars, 1);
    let bx = p
        .draws
        .iter()
        .find_map(|d| match d {
            Draw::Fill { rect, color } if *color == palette::MINI_BG => Some(*rect),
            _ => None,
        })
        .unwrap();
    let ticks = p
        .draws
        .iter()
        .filter(|d| matches!(d, Draw::Fill { rect, color } if *color == palette::FAINT && rect.left() > bx.right()))
        .count();
    assert_eq!(ticks, app.outline().len() - 1);
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
    out.textures_delta.clear();
    out
}

#[test]
fn headless_frames_hover_and_click_panel_rows() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = app();
    let f = file(&app, SHORT);
    app.select(Some(Entry::File(f)), false, 0.0);
    let _ = frame(&ctx, &mut app, vec![], 0.0);
    let r = app.panel().unwrap().rect_of(Hit::Outline(4)).unwrap();
    let _ = frame(&ctx, &mut app, vec![Event::PointerMoved(r.center())], 0.1);
    assert_eq!(app.panel_hover, Some(Hit::Outline(4)));
    assert_eq!(app.hover, None, "an outline row hovers no card");
    let p = app.panel().unwrap();
    assert!(p.draws.iter().any(
        |d| matches!(d, Draw::Fill { rect, color } if *rect == r && *color == palette::TILE_HI)
    ));
    let _ = frame(&ctx, &mut app, vec![press(r.center(), true)], 0.2);
    let out = frame(&ctx, &mut app, vec![press(r.center(), false)], 0.3);
    assert_eq!(app.pick, Some(4));
    assert!(app.motion.easing());
    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    assert!(!prims.is_empty());
    // A largest-file row hovers its card, as a tree row does.
    let Some(Entry::Dir(n)) = app.map.entry("components") else {
        panic!()
    };
    app.select(Some(Entry::Dir(n)), false, 1.0);
    let _ = frame(&ctx, &mut app, vec![], 1.0);
    let (r, h) = app.panel().unwrap().hits[0];
    let _ = frame(&ctx, &mut app, vec![Event::PointerMoved(r.center())], 1.1);
    let Hit::File(g) = h else { panic!() };
    assert_eq!(app.panel_hover, Some(h));
    assert_eq!(app.hover, Some(Entry::File(g)));
}
