//! The agent's trail in the window (unit `047`): tinted cards, the tree's mark, the panel's line, the toggle, and the listener.

mod common;

use std::process::Command;
use std::time::{Duration, Instant};

use eframe::egui::{self, Event, Key, Modifiers, Rect, epaint};
use minimap_core::events::{self, Event as AgentEvent, Kind as TrailKind, Listener};
use minimap_core::{Index, IndexOptions, palette};
use minimap_gui::GuiApp;
use minimap_gui::camera::Camera;
use minimap_gui::color::{mix, rgb};
use minimap_gui::scene::{
    Entry, Kind, Map, TRAIL_BLOCK_FRESH_MIX, TRAIL_BLOCK_OLD_MIX, TRAIL_FRESH_MIX, TRAIL_OLD_MIX,
    trail_rgb,
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

/// The quayside map at card width 200, centered on the root's first file.
fn app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    let f = app.map.tree.nodes[0].files[0];
    let cam = Camera {
        center: app.map.card(f).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app
}

/// The root's first files, each on screen at card width 200.
fn files(app: &GuiApp, n: usize) -> Vec<usize> {
    let root = &app.map.tree.nodes[0];
    let on: Vec<usize> = root
        .files
        .iter()
        .copied()
        .filter(|&f| {
            app.cam()
                .rect_to_screen(app.region, app.map.card(f))
                .intersects(app.region)
        })
        .take(n)
        .collect();
    assert_eq!(on.len(), n, "{n} root cards on screen");
    on
}

fn read(app: &mut GuiApp, f: usize, now: f64) {
    let path = app.map.tree.files[f].path.clone();
    app.record_event(&AgentEvent::trail(TrailKind::Read, vec![path]), now);
}

/// A card's fill in the current scene.
fn fill(app: &GuiApp, f: usize, kind: Kind) -> egui::Color32 {
    let path = app.map.tree.files[f].path.clone();
    app.items()
        .into_iter()
        .find(|i| i.kind == kind && i.path == path)
        .unwrap_or_else(|| panic!("no {kind:?} for {path}"))
        .color
}

#[test]
fn events_tint_cards_fresh_then_old_and_the_hover_wins() {
    let mut app = app();
    let fs = files(&app, 4);
    let cards: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    let untouched = fill(&app, fs[0], Kind::Card);
    assert_eq!(untouched, rgb(palette::TILE));
    for (k, &f) in fs.iter().enumerate() {
        read(&mut app, f, 1.0 + k as f64);
    }
    // Four events: the first file's event is no longer among the 3 newest.
    let read_rgb = trail_rgb(TrailKind::Read);
    assert_eq!(
        fill(&app, fs[0], Kind::Card),
        mix(read_rgb, palette::TILE, TRAIL_OLD_MIX)
    );
    assert_eq!(
        fill(&app, fs[3], Kind::Card),
        mix(read_rgb, palette::TILE, TRAIL_FRESH_MIX)
    );
    // An edit is the strongest kind: a read after an edit keeps the edit's color.
    let path = app.map.tree.files[fs[1]].path.clone();
    app.record_event(&AgentEvent::trail(TrailKind::Edit, vec![path.clone()]), 6.0);
    app.record_event(&AgentEvent::trail(TrailKind::Read, vec![path]), 7.0);
    assert_eq!(
        fill(&app, fs[1], Kind::Card),
        mix(trail_rgb(TrailKind::Edit), palette::TILE, TRAIL_FRESH_MIX)
    );
    // The hover wins over the trail.
    app.hover = Some(Entry::File(fs[3]));
    assert_eq!(fill(&app, fs[3], Kind::Card), rgb(palette::TILE_HI));
    app.hover = None;
    // At the block level, the strong mixes over the pane.
    let small = Camera {
        zoom: 10.0 / app.map.card_w(),
        ..app.cam()
    };
    app.motion.jump(app.bounds.clamp(app.region, small));
    assert_eq!(
        fill(&app, fs[0], Kind::Block),
        mix(read_rgb, palette::PANE, TRAIL_BLOCK_OLD_MIX)
    );
    assert_eq!(
        fill(&app, fs[1], Kind::Block),
        mix(
            trail_rgb(TrailKind::Edit),
            palette::PANE,
            TRAIL_BLOCK_FRESH_MIX
        )
    );
    // No card moved.
    let after: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    assert_eq!(cards, after);
}

#[test]
fn one_hides_the_trail_and_x_clears_it() {
    let mut app = app();
    let f = files(&app, 1)[0];
    read(&mut app, f, 1.0);
    let tinted = fill(&app, f, Kind::Card);
    assert_ne!(tinted, rgb(palette::TILE));
    app.on_event(&key(Key::Num1), None, 2.0);
    assert!(!app.opts.toggles.trail);
    assert_eq!(fill(&app, f, Kind::Card), rgb(palette::TILE));
    app.on_event(&key(Key::Num1), None, 3.0);
    assert_eq!(fill(&app, f, Kind::Card), tinted);
    app.on_event(&key(Key::X), None, 4.0);
    assert!(app.trail.is_empty());
    assert_eq!(fill(&app, f, Kind::Card), rgb(palette::TILE));
}

#[test]
fn the_panel_shows_the_selected_files_trail_line() {
    let mut app = app();
    let f = files(&app, 1)[0];
    app.select(Some(Entry::File(f)), false, 0.0);
    assert!(app.trail_line().is_none());
    let path = app.map.tree.files[f].path.clone();
    app.record_event(
        &AgentEvent::Trail {
            kind: TrailKind::Edit,
            paths: vec![path],
            lines: Some((40, 58)),
            source: Some("claude-code".to_string()),
            session: None,
        },
        10.0,
    );
    app.now = 15.0;
    let (line, color) = app.trail_line().unwrap();
    assert_eq!(line, "CC edited lines 40–58 · 5s ago");
    assert_eq!(color, palette::EDIT);
    let texts: Vec<String> = app
        .panel()
        .unwrap()
        .texts()
        .into_iter()
        .map(String::from)
        .collect();
    let size = texts.iter().rposition(|t| t.ends_with(" lines")).unwrap();
    assert_eq!(texts[size + 1], line, "{texts:#?}");
    // With layer 1 off, no line.
    app.on_event(&key(Key::Num1), None, 16.0);
    assert!(app.trail_line().is_none());
}

#[test]
fn a_headless_frame_draws_the_trees_trail_mark() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = app();
    let f = files(&app, 1)[0];
    read(&mut app, f, 0.5);
    let marks = |app: &mut GuiApp, time: f64| {
        let input = egui::RawInput {
            screen_rect: Some(common::screen()),
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
        out.textures_delta.clear();
        out.shapes
            .iter()
            .filter(|c| matches!(&c.shape, epaint::Shape::Text(t) if t.galley.job.text == "▌"))
            .count()
    };
    assert_eq!(marks(&mut app, 1.0), 1, "the root file's row has the mark");
    app.on_event(&key(Key::Num1), None, 1.5);
    assert_eq!(marks(&mut app, 2.0), 0, "layer 1 off");
}

#[test]
fn the_listener_feeds_the_trail() {
    // A fresh git repo in the temp directory; the GUI has no tempfile dependency.
    let dir = std::env::temp_dir().join(format!("minimap-gui-trail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = dir.canonicalize().unwrap();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(args)
            .current_dir(&root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q"]);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.clj"), "(ns a)\n").unwrap();
    std::fs::write(root.join("src/b.clj"), "(ns b)\n").unwrap();
    let index = Index::from_repo(&root, &IndexOptions::default()).unwrap();
    let mut app = GuiApp::new(Map::new(index));
    app.listener = Some(Listener::spawn(&root).unwrap());
    let abs = root.join("src/b.clj").to_string_lossy().into_owned();
    events::send(&root, &AgentEvent::trail(TrailKind::Grep, vec![abs])).unwrap();
    let t0 = Instant::now();
    while !app.poll_events(1.0) {
        assert!(t0.elapsed() < Duration::from_secs(2), "no event");
        std::thread::sleep(Duration::from_millis(5));
    }
    let Some(Entry::File(b)) = app.map.entry("src/b.clj") else {
        panic!()
    };
    assert_eq!(app.map.tint[b], Some((TrailKind::Grep, true)));
    drop(app);
    let _ = std::fs::remove_dir_all(&dir);
}
