mod common;

use eframe::egui::{Event, Modifiers, MouseWheelUnit, Pos2, TouchPhase, Vec2, pos2, vec2};
use minimap_gui::GuiApp;
use minimap_gui::camera::{Bounds, Camera, FIT_MARGIN, KEY_EASE, MAX_ZOOM, Motion};

fn bounds() -> Bounds {
    Bounds {
        map: vec2(22_000.0, 13_000.0),
    }
}

#[test]
fn a_zoom_around_a_point_keeps_its_world_point_under_it() {
    let (b, region) = (bounds(), common::region());
    let cam = Camera {
        center: pos2(9_000.0, 6_000.0),
        zoom: 0.5,
    };
    for (at, factor) in [
        (pos2(300.0, 200.0), 1.15),
        (pos2(1500.0, 900.0), 1.0 / 1.15),
        (pos2(800.0, 500.0), 2.7),
        (pos2(10.0, 40.0), 0.6),
    ] {
        let before = cam.to_world(region, at);
        let after = b.zoom_around(region, cam, at, factor);
        let moved = after.to_screen(region, before) - at;
        assert!(moved.length() < 0.01, "{at:?} x{factor}: moved {moved:?}");
        assert!((after.zoom - cam.zoom * factor).abs() < 1e-6);
    }
}

#[test]
fn the_fit_view_contains_every_card() {
    let map = common::quayside();
    let region = common::region();
    let b = Bounds { map: map.world() };
    let fit = b.fit(region);
    let inner = region.shrink(FIT_MARGIN - 0.01);
    for f in 0..map.tree.files.len() {
        let r = fit.rect_to_screen(region, map.card(f));
        assert!(
            inner.contains_rect(r),
            "{} at {r:?}",
            map.tree.files[f].path
        );
    }
    // The map touches the margin on one axis.
    let whole = fit.rect_to_screen(
        region,
        eframe::egui::Rect::from_min_size(Pos2::ZERO, map.world()),
    );
    let slack = (inner.width() - whole.width()).min(inner.height() - whole.height());
    assert!(slack.abs() < 0.1, "slack {slack}");
}

#[test]
fn the_clamp_keeps_the_center_inside_the_map_and_the_zoom_in_range() {
    let (b, region) = (bounds(), common::region());
    let fit = b.fit(region).zoom;
    let (lo, hi) = b.zoom_range(region);
    assert_eq!((lo, hi), (fit / 2.0, MAX_ZOOM));
    let wild = Camera {
        center: pos2(-5_000.0, 90_000.0),
        zoom: 100.0,
    };
    let c = b.clamp(region, wild);
    assert_eq!(c.center, pos2(0.0, 13_000.0));
    assert_eq!(c.zoom, MAX_ZOOM);
    let c = b.clamp(region, Camera { zoom: 1e-6, ..wild });
    assert_eq!(c.zoom, fit / 2.0);
    // A pan past the edge stops at it.
    let c = b.pan(region, b.fit(region), vec2(1e7, -1e7));
    assert_eq!(c.center, pos2(0.0, 13_000.0));
}

#[test]
fn a_key_zoom_eases_out_over_160_ms() {
    let (b, region) = (bounds(), common::region());
    let from = b.fit(region);
    let to = b.zoom_around(region, from, region.center(), 1.5);
    let mut m = Motion::new(from);
    m.ease_to(to, 10.0, KEY_EASE);
    assert!(m.tick(10.0 + KEY_EASE / 2.0));
    // Ease-out: past half the zoom change (in log space) at half the time.
    let done = (m.cam.zoom / from.zoom).ln() / (to.zoom / from.zoom).ln();
    assert!(done > 0.8 && done < 1.0, "{done}");
    assert!(!m.tick(10.0 + KEY_EASE));
    assert_eq!(m.cam, to);
}

fn app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_region(common::region());
    app
}

fn wheel(unit: MouseWheelUnit, delta: Vec2, command: bool) -> Event {
    Event::MouseWheel {
        unit,
        delta,
        phase: TouchPhase::Move,
        modifiers: Modifiers {
            command,
            mac_cmd: command,
            ..Modifiers::NONE
        },
    }
}

#[test]
fn a_wheel_notch_zooms_around_the_pointer_and_a_trackpad_scroll_pans() {
    let mut a = app();
    a.motion.jump(Camera {
        center: pos2(10_000.0, 3_000.0),
        zoom: 0.5,
    });
    let at = pos2(400.0, 300.0);
    let world = a.cam().to_world(a.region, at);
    a.on_event(
        &wheel(MouseWheelUnit::Line, vec2(0.0, 1.0), false),
        Some(at),
        0.0,
    );
    assert!((a.cam().zoom - 0.5 * 1.15).abs() < 1e-6);
    assert!((a.cam().to_screen(a.region, world) - at).length() < 0.01);
    let before = a.cam();
    a.on_event(
        &wheel(MouseWheelUnit::Point, vec2(30.0, -20.0), false),
        Some(at),
        0.0,
    );
    assert_eq!(a.cam().zoom, before.zoom);
    assert!((a.cam().center - (before.center - vec2(30.0, -20.0) / before.zoom)).length() < 1e-3);
    // Cmd with a trackpad scroll zooms.
    a.on_event(
        &wheel(MouseWheelUnit::Point, vec2(0.0, 200.0), true),
        Some(at),
        0.0,
    );
    assert!((a.cam().zoom / before.zoom - std::f32::consts::E).abs() < 1e-3);
    // A pinch zooms around the pointer, at once.
    let z = a.cam().zoom;
    a.on_event(&Event::Zoom(0.5), Some(at), 0.0);
    assert!((a.cam().zoom - z * 0.5).abs() < 1e-6);
    assert!(!a.motion.easing());
}

#[test]
fn keys_zoom_fit_and_pan() {
    let mut a = app();
    let fit = a.cam();
    let key = |k| Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    a.on_event(&key(eframe::egui::Key::Plus), None, 1.0);
    assert!(a.motion.easing());
    assert!((a.motion.target().zoom - fit.zoom * 1.5).abs() < 1e-6);
    // A second press zooms from where the camera is going.
    a.on_event(&key(eframe::egui::Key::Equals), None, 1.05);
    assert!((a.motion.target().zoom - fit.zoom * 2.25).abs() < 1e-5);
    a.motion.tick(2.0);
    let view = a.cam().view(a.region).size();
    let c = a.cam().center;
    a.on_event(&key(eframe::egui::Key::D), None, 2.0);
    assert!((a.cam().center.x - (c.x + 0.1 * view.x)).abs() < 1e-2);
    a.on_event(&key(eframe::egui::Key::Num0), None, 3.0);
    a.motion.tick(4.0);
    assert_eq!(a.cam(), fit);
}

#[test]
fn a_drag_pans_with_the_pointer() {
    let mut a = app();
    a.motion.jump(Camera {
        center: pos2(10_000.0, 3_000.0),
        zoom: 0.5,
    });
    a.on_drag(vec2(50.0, -10.0));
    assert_eq!(a.cam().center, pos2(10_000.0 - 100.0, 3_000.0 + 20.0));
}

#[test]
fn a_zoom_never_changes_a_card_world_rect() {
    let mut a = app();
    let before: Vec<_> = (0..a.map.tree.files.len()).map(|f| a.map.card(f)).collect();
    for factor in [3.0, 0.2, 7.0] {
        let at = a.region.center();
        a.on_event(&Event::Zoom(factor), Some(at), 0.0);
        let after: Vec<_> = (0..a.map.tree.files.len()).map(|f| a.map.card(f)).collect();
        assert_eq!(before, after);
    }
}
