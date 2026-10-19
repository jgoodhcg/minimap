//! The glows of agent events (unit `049`): the strength over time, the halo at the fit view, the ripple, the edge marker,
//! the hit test, the toggle, the clear, a re-index, and the repaints of headless frames.

mod common;

use std::time::Duration;

use eframe::egui::{self, Event, Key, Modifiers, Pos2, Rect, ViewportId, pos2};
use minimap_core::Index;
use minimap_core::events::{Event as AgentEvent, Kind as TrailKind};
use minimap_core::watch::Update;
use minimap_gui::GuiApp;
use minimap_gui::camera::Camera;
use minimap_gui::scene::{
    Body, Entry, GLOW_MIN, GLOW_SECS, Item, Kind, Level, RIPPLE_SECS, glow_strength,
};

fn key(k: Key) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// The quayside map in the capture window, at the fit view.
fn fit_app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    app
}

/// The quayside map at card width 200, centered on the root's first file.
fn near_app() -> GuiApp {
    let mut app = fit_app();
    let f = app.map.tree.nodes[0].files[0];
    let cam = Camera {
        center: app.map.card(f).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app
}

fn event(app: &mut GuiApp, kind: TrailKind, f: usize, now: f64) {
    let path = app.map.tree.files[f].path.clone();
    app.record_event(&AgentEvent::trail(kind, vec![path]), now);
}

fn glow_items<'a>(items: &'a [Item<'a>], kind: Kind) -> Vec<&'a Item<'a>> {
    items.iter().filter(|i| i.kind == kind).collect()
}

const GLOW_KINDS: [Kind; 4] = [Kind::Glow, Kind::GlowCore, Kind::Ripple, Kind::GlowEdge];

#[test]
fn the_strength_holds_then_fades_out() {
    assert_eq!(glow_strength(0.0), 1.0);
    assert_eq!(glow_strength(1.0), 1.0);
    assert!((glow_strength(2.5) - 0.5).abs() < 1e-6);
    assert!(glow_strength(2.0) > glow_strength(3.0));
    assert_eq!(glow_strength(4.0), 0.0);
    assert_eq!(glow_strength(9.0), 0.0);
    assert_eq!(GLOW_SECS, 4.0);
}

#[test]
fn a_glow_at_the_fit_view_is_large_bright_and_ripples() {
    let mut app = fit_app();
    assert_eq!(
        minimap_gui::scene::level(&app.map, &app.cam()),
        Level::Block
    );
    let f = app.map.tree.nodes[0].files[0];
    let card = app.cam().rect_to_screen(app.region, app.map.card(f));
    assert!(card.width() < 20.0, "a block: {card:?}");
    event(&mut app, TrailKind::Read, f, 1.0);
    app.now = 1.3;
    let items = app.items();
    let halo = glow_items(&items, Kind::Glow);
    assert_eq!(halo.len(), 1);
    let Body::Halo { inner, filled } = halo[0].body else {
        panic!("{:?}", halo[0].body)
    };
    assert!(filled, "the block level fills the glow box");
    assert!(inner.width() >= GLOW_MIN - 1e-3 && inner.height() >= GLOW_MIN - 1e-3);
    assert!(inner.contains(card.center()));
    assert!(
        halo[0].rect.width() > inner.width(),
        "the halo reaches past the box"
    );
    assert_eq!(halo[0].color.a(), (0.85f32 * 255.0).round() as u8);
    let core = glow_items(&items, Kind::GlowCore);
    assert_eq!(core.len(), 1);
    assert_eq!(core[0].body, Body::Fill);
    let c = core[0].color;
    assert!(
        c.r().min(c.g()).min(c.b()) > 190,
        "a near-white core: {c:?}"
    );
    assert_eq!(
        glow_items(&items, Kind::Ripple).len(),
        1,
        "the ripple at 0.3 s"
    );
    // The glow draws above every card and header.
    let first = items.iter().position(|i| i.kind == Kind::Glow).unwrap();
    assert!(items[first..].iter().all(|i| GLOW_KINDS.contains(&i.kind)));
    app.now = 1.0 + RIPPLE_SECS + 0.1;
    let items = app.items();
    assert_eq!(
        glow_items(&items, Kind::Ripple).len(),
        0,
        "no ripple at 1.0 s"
    );
    assert_eq!(glow_items(&items, Kind::Glow).len(), 1);
    // Half-way through the fade, half the opacity; gone at the end.
    app.now = 3.5;
    let items = app.items();
    let a = glow_items(&items, Kind::Glow)[0].color.a();
    assert!(
        (i32::from(a) - (0.85f32 * 0.5 * 255.0).round() as i32).abs() <= 1,
        "{a}"
    );
    app.now = 5.0;
    assert!(glow_items(&app.items(), Kind::Glow).is_empty());
}

#[test]
fn a_near_glow_rings_the_card_and_keeps_its_content() {
    let mut app = near_app();
    let f = app.map.tree.nodes[0].files[0];
    event(&mut app, TrailKind::Edit, f, 1.0);
    app.now = 1.5;
    let items = app.items();
    let card = app.cam().rect_to_screen(app.region, app.map.card(f));
    let halo = glow_items(&items, Kind::Glow)[0];
    assert_eq!(
        halo.body,
        Body::Halo {
            inner: card,
            filled: false
        }
    );
    let core = glow_items(&items, Kind::GlowCore)[0];
    assert!(matches!(core.body, Body::Stroke(_)));
    assert_eq!(core.rect, card);
}

#[test]
fn an_off_screen_glow_marks_the_regions_edge() {
    let mut app = near_app();
    let region = app.region;
    let off = (0..app.map.tree.files.len())
        .find(|&f| {
            app.map.visible(f)
                && !app
                    .cam()
                    .rect_to_screen(region, app.map.card(f))
                    .intersects(region)
        })
        .expect("a card off screen");
    event(&mut app, TrailKind::Grep, off, 1.0);
    app.now = 1.2;
    let items = app.items();
    let edge = glow_items(&items, Kind::GlowEdge);
    assert_eq!(edge.len(), 2, "the dot and its white center");
    for e in edge {
        assert!(region.contains_rect(e.rect), "{:?} in {region:?}", e.rect);
    }
    assert!(glow_items(&items, Kind::Glow).is_empty());
}

#[test]
fn a_glow_changes_no_hit_and_moves_no_card() {
    let mut app = fit_app();
    let cards: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    let rects = |app: &GuiApp| -> Vec<(Kind, Rect)> {
        app.items()
            .iter()
            .filter(|i| !GLOW_KINDS.contains(&i.kind))
            .map(|i| (i.kind, i.rect))
            .collect()
    };
    let before = rects(&app);
    let r = app.region;
    let points: Vec<Pos2> = (0..40)
        .flat_map(|x| (0..25).map(move |y| (x, y)))
        .map(|(x, y)| {
            pos2(
                r.left() + r.width() * (x as f32 + 0.5) / 40.0,
                r.top() + r.height() * (y as f32 + 0.5) / 25.0,
            )
        })
        .collect();
    let hits: Vec<Option<Entry>> = points.iter().map(|&p| app.hit(p)).collect();
    for (k, f) in app.map.tree.nodes[0].files.clone().into_iter().enumerate() {
        event(&mut app, TrailKind::Read, f, 1.0 + k as f64 * 0.01);
    }
    app.now = 1.2;
    assert!(!glow_items(&app.items(), Kind::Glow).is_empty());
    assert_eq!(before, rects(&app));
    let after: Vec<Option<Entry>> = points.iter().map(|&p| app.hit(p)).collect();
    assert_eq!(hits, after);
    let moved: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    assert_eq!(cards, moved);
}

#[test]
fn a_new_event_restarts_the_glow_and_one_and_x_hide_it() {
    let mut app = fit_app();
    let f = app.map.tree.nodes[0].files[0];
    event(&mut app, TrailKind::Read, f, 1.0);
    event(&mut app, TrailKind::Grep, f, 3.0);
    assert_eq!(app.glows.len(), 1);
    assert_eq!(app.glows[0].at, 3.0);
    assert_eq!(
        app.glows[0].color,
        minimap_core::palette::GREP,
        "the event's kind, not the strongest"
    );
    app.now = 4.5;
    assert!(app.glowing(), "the second event restarted it");
    assert_eq!(glow_items(&app.items(), Kind::Glow).len(), 1);
    app.on_event(&key(Key::Num1), None, 4.5);
    assert!(!app.glowing());
    assert!(glow_items(&app.items(), Kind::Glow).is_empty());
    app.on_event(&key(Key::Num1), None, 4.6);
    assert!(app.glowing());
    app.on_event(&key(Key::X), None, 4.7);
    assert!(app.glows.is_empty());
    assert!(!app.glowing());
    event(&mut app, TrailKind::Read, f, 5.0);
    app.record_event(&AgentEvent::Clear, 5.1);
    assert!(app.glows.is_empty(), "a clear event clears the glows");
}

#[test]
fn a_reindex_keeps_the_glows_by_path() {
    let mut app = fit_app();
    let f = app.map.tree.nodes[0].files[0];
    let path = app.map.tree.files[f].path.clone();
    event(&mut app, TrailKind::Edit, f, 1.0);
    let index: Index = app.map.index.clone();
    app.apply_index(
        Update {
            index,
            changed: vec![path.clone()],
            head_moved: false,
        },
        1.2,
    );
    app.now = 1.3;
    let items = app.items();
    let halo = glow_items(&items, Kind::Glow);
    assert_eq!(halo.len(), 1);
    assert_eq!(halo[0].path, path);
}

#[test]
fn frames_repaint_only_while_a_glow_runs() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = fit_app();
    let frame = |app: &mut GuiApp, time: f64| -> Duration {
        let input = egui::RawInput {
            screen_rect: Some(common::screen()),
            time: Some(time),
            max_texture_side: Some(8192),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
        out.textures_delta.clear();
        let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
        assert!(!prims.is_empty());
        out.viewport_output[&ViewportId::ROOT].repaint_delay
    };
    for k in 0..3 {
        frame(&mut app, 0.1 * k as f64);
    }
    assert!(
        frame(&mut app, 0.5) > Duration::from_secs(60),
        "an idle window"
    );
    let f = app.map.tree.nodes[0].files[0];
    event(&mut app, TrailKind::Read, f, 1.0);
    assert_eq!(frame(&mut app, 1.0), Duration::ZERO, "the glow starts");
    assert_eq!(frame(&mut app, 3.0), Duration::ZERO, "the glow fades");
    // egui paints one more frame after the last request.
    frame(&mut app, 5.1);
    assert!(
        frame(&mut app, 5.2) > Duration::from_secs(60),
        "the glow ended"
    );
    assert!(app.glows.is_empty(), "an ended glow is dropped");
}
