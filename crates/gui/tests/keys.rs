//! The tree keys in the window (unit `043`): `j` `k` `l` `h` `g` `G` Home End Enter, and the map follow.

mod common;

use eframe::egui::{Event, Key, Modifiers};
use minimap_gui::GuiApp;
use minimap_gui::app::SHOW_MARGIN;
use minimap_gui::camera::Camera;
use minimap_gui::scene::Entry;

fn app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    // Card width 200, so a move often needs the map to follow.
    let zoom = 200.0 / app.map.card_w();
    let cam = Camera {
        center: (app.map.world() / 2.0).to_pos2(),
        zoom,
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app
}

fn key_with(k: Key, modifiers: Modifiers, repeat: bool) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}

fn key(k: Key) -> Event {
    key_with(k, Modifiers::NONE, false)
}

/// Presses a key, then ends any ease.
fn press(app: &mut GuiApp, k: Key, now: f64) {
    app.on_event(&key(k), None, now);
    app.motion.tick(now + 10.0);
}

fn index_of(app: &GuiApp) -> usize {
    let rows = app.tree_rows();
    rows.iter()
        .position(|r| Some(r.entry) == app.selection)
        .unwrap()
}

/// The selected entry is inside the region with the margin, or its top-left shows at the margin when it is larger; the zoom held.
fn assert_shown(app: &GuiApp, zoom: f32, what: &str) {
    let e = app.selection.unwrap();
    assert_eq!(app.cam().zoom, zoom, "{what}: no zoom change");
    let r = app.cam().rect_to_screen(app.region, app.map.rect_of(e));
    let inner = app.region.shrink(SHOW_MARGIN - 1e-2);
    if r.height() <= inner.height() && r.width() <= inner.width() {
        assert!(inner.contains_rect(r), "{what}: {e:?} at {r:?}");
    } else {
        assert!(inner.contains(r.min), "{what}: {e:?} at {r:?}");
    }
}

#[test]
fn j_and_k_move_one_row_and_the_map_follows() {
    let mut app = app();
    let zoom = app.cam().zoom;
    let rows = app.tree_rows();
    press(&mut app, Key::J, 1.0);
    assert_eq!(
        app.selection,
        Some(rows[0].entry),
        "with no selection, the first row"
    );
    assert_shown(&app, zoom, "first j");
    press(&mut app, Key::J, 2.0);
    press(&mut app, Key::J, 3.0);
    assert_eq!(index_of(&app), 2);
    assert_shown(&app, zoom, "j");
    press(&mut app, Key::K, 4.0);
    assert_eq!(index_of(&app), 1);
    // k at the first row stays there.
    press(&mut app, Key::K, 5.0);
    press(&mut app, Key::K, 6.0);
    assert_eq!(index_of(&app), 0);
    // The tree cursor follows the selection, and the selected row is on screen.
    assert_eq!(app.filetree.cursor, app.selection);
}

#[test]
fn held_keys_move_one_row_per_repeat_event() {
    let mut app = app();
    press(&mut app, Key::G, 1.0);
    let len = app.tree_rows().len();
    for k in 0..20 {
        app.on_event(
            &key_with(Key::J, Modifiers::NONE, true),
            None,
            2.0 + k as f64 * 0.03,
        );
    }
    assert_eq!(index_of(&app), 20.min(len - 1));
    let i = index_of(&app);
    assert!(
        i >= app.filetree.top && i < app.filetree.top + app.filetree.height,
        "the row is on screen"
    );
}

#[test]
fn l_opens_then_goes_in_and_h_closes_then_goes_up() {
    let mut app = app();
    let zoom = app.cam().zoom;
    press(&mut app, Key::J, 1.0);
    let Some(Entry::Dir(n)) = app.selection else {
        panic!("the first row is a directory")
    };
    press(&mut app, Key::L, 2.0);
    assert!(app.filetree.is_open(n));
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    press(&mut app, Key::L, 3.0);
    let child = app.selection.unwrap();
    assert_ne!(child, Entry::Dir(n));
    assert_eq!(
        app.filetree.parent_of(&app.map.tree, child),
        Some(Entry::Dir(n))
    );
    assert_shown(&app, zoom, "l into the first child");
    // h on the child goes up when it is a file or a closed directory, then h closes the parent.
    press(&mut app, Key::H, 4.0);
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    press(&mut app, Key::H, 5.0);
    assert!(!app.filetree.is_open(n));
    assert_eq!(app.selection, Some(Entry::Dir(n)));
    // h on a root child does nothing.
    press(&mut app, Key::H, 6.0);
    assert_eq!(app.selection, Some(Entry::Dir(n)));
}

#[test]
fn g_and_shift_g_and_home_and_end_go_to_the_ends() {
    let mut app = app();
    let zoom = app.cam().zoom;
    let rows = app.tree_rows();
    let shift_g = key_with(Key::G, Modifiers::SHIFT, false);
    app.on_event(&shift_g, None, 1.0);
    app.motion.tick(20.0);
    assert_eq!(app.selection, Some(rows.last().unwrap().entry));
    assert_shown(&app, zoom, "G");
    press(&mut app, Key::G, 30.0);
    assert_eq!(app.selection, Some(rows[0].entry));
    press(&mut app, Key::End, 40.0);
    assert_eq!(app.selection, Some(rows.last().unwrap().entry));
    press(&mut app, Key::Home, 50.0);
    assert_eq!(app.selection, Some(rows[0].entry));
}

#[test]
fn enter_zooms_to_the_selection_as_f_does() {
    let mut app = app();
    press(&mut app, Key::J, 1.0);
    let dir = app.selection.unwrap();
    app.on_event(&key(Key::Enter), None, 2.0);
    assert_eq!(app.motion.target(), app.camera_for(dir));
    app.motion.tick(20.0);
    press(&mut app, Key::L, 30.0);
    press(&mut app, Key::L, 31.0);
    press(&mut app, Key::G, 32.0);
    press(&mut app, Key::End, 33.0);
    let Some(Entry::File(f)) = app.selection else {
        panic!("the last row is a file")
    };
    app.on_event(&key(Key::Enter), None, 40.0);
    assert_eq!(app.motion.target(), app.camera_for(Entry::File(f)));
    // With no selection, Enter does nothing.
    app.on_event(&key(Key::Escape), None, 50.0);
    app.motion.tick(60.0);
    let cam = app.cam();
    app.on_event(&key(Key::Enter), None, 61.0);
    assert!(!app.motion.easing());
    assert_eq!(app.cam(), cam);
}

#[test]
fn the_keys_work_while_the_tree_is_hidden() {
    let mut app = app();
    let zoom = app.cam().zoom;
    app.on_event(&key(Key::T), None, 0.5);
    assert!(app.tree_view.is_none());
    press(&mut app, Key::J, 1.0);
    press(&mut app, Key::L, 2.0);
    press(&mut app, Key::L, 3.0);
    let e = app.selection.unwrap();
    assert_shown(&app, zoom, "hidden tree");
    // Shown again, the tree scrolls to the selection.
    app.on_event(&key(Key::T), None, 4.0);
    let i = index_of(&app);
    assert!(i >= app.filetree.top && i < app.filetree.top + app.filetree.height);
    assert_eq!(app.selection, Some(e));
}
