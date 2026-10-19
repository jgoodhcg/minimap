//! The trail layer (unit `010`): agent reads, searches, and edits on the map, the file tree, the overview, and the panel.

mod common;

use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

use common::*;
use minimap_core::events::{Event, Kind};
use minimap_core::watch::Update;
use minimap_core::{Index, IndexFile};
use minimap_tui::App;
use minimap_tui::filetree::Entry;
use minimap_tui::theme;
use minimap_tui::trail::{self, Trail};

const RULES: &str = "components/berth/src/quayside/berth/rules.clj";
const ALLOC: &str = "components/berth/src/quayside/berth/allocation.clj";
const SCORING: &str = "components/berth/src/quayside/berth/scoring.clj";
const PLANNER: &str = "bases/worker/src/quayside/worker/jobs/berth_planner.clj";
const UI: &str = "bases/console/src/quayside/console/views/berth/planner.cljs";
const HANDLER: &str = "bases/api/src/quayside/api/handlers/berth.clj";
const TEST: &str = "components/berth/test/quayside/berth/rules_test.clj";

fn ev(kind: Kind, paths: &[&str]) -> Event {
    Event::Trail {
        kind,
        paths: paths.iter().map(|p| p.to_string()).collect(),
        lines: None,
        source: Some("claude-code".into()),
        session: None,
    }
}

/// The trail of `mockups/v3-template.html` `seed`: one grep of 3 files, 4 reads, then 1 edit.
fn seed(app: &mut App) {
    app.record_event(&ev(Kind::Grep, &[ALLOC, HANDLER, SCORING]));
    for p in [RULES, ALLOC, PLANNER, UI] {
        app.record_event(&ev(Kind::Read, &[p]));
    }
    app.record_event(&Event::Trail {
        kind: Kind::Edit,
        paths: vec![TEST.into()],
        lines: Some((40, 58)),
        source: Some("claude-code".into()),
        session: None,
    });
}

/// Quayside at 150 x 45 with the seeded trail, the berth directories open, and the edited test file selected.
fn seeded() -> (Terminal<TestBackend>, App) {
    let (mut t, mut app) = quayside(150, 45);
    seed(&mut app);
    app.select(Entry::File(file(&app, RULES)), true);
    app.select(Entry::File(file(&app, TEST)), true);
    draw(&mut t, &mut app);
    (t, app)
}

fn cell_bg(t: &Terminal<TestBackend>, (x, y): (i32, i32)) -> Color {
    t.backend().buffer()[(x as u16, y as u16)].bg
}

/// A file's overview cell on screen.
fn overview_cell(app: &App, f: usize) -> (i32, i32) {
    let o = app.overview().unwrap();
    let (x, y) = app.layout.tiles[f];
    let dx = (f64::from(x) * o.s * 2.0).floor() as i32;
    let dy = (f64::from(y) * o.s * 4.0).floor() as i32;
    (o.x + 1 + (dx >> 1), o.y + 1 + (dy >> 2))
}

#[test]
fn the_strongest_kind_wins_per_path() {
    let mut tr = Trail::default();
    let now = Duration::ZERO;
    tr.record(&ev(Kind::Grep, &["a"]), now);
    tr.record(&ev(Kind::Read, &["a"]), now);
    tr.record(&ev(Kind::Grep, &["a"]), now);
    assert_eq!(tr.get("a").unwrap().kind, Kind::Read);
    tr.record(&ev(Kind::Edit, &["a"]), now);
    tr.record(&ev(Kind::Read, &["a"]), now);
    tr.record(&ev(Kind::Grep, &["a"]), now);
    assert_eq!(tr.get("a").unwrap().kind, Kind::Edit);
}

#[test]
fn an_edit_keeps_its_lines_and_the_newest_event_sets_the_time() {
    let mut tr = Trail::default();
    let edit = Event::Trail {
        kind: Kind::Edit,
        paths: vec!["a".into()],
        lines: Some((3, 9)),
        source: None,
        session: None,
    };
    tr.record(&edit, Duration::from_secs(1));
    tr.record(&ev(Kind::Read, &["a"]), Duration::from_secs(5));
    let m = tr.get("a").unwrap();
    assert_eq!(
        (m.kind, m.lines, m.at),
        (Kind::Edit, Some((3, 9)), Duration::from_secs(5))
    );
}

#[test]
fn freshness_moves_with_the_three_newest_events() {
    let mut tr = Trail::default();
    let now = Duration::ZERO;
    tr.record(&ev(Kind::Grep, &["g1", "g2"]), now);
    for p in ["a", "b", "c"] {
        tr.record(&ev(Kind::Read, &[p]), now);
    }
    let fresh = |tr: &Trail, p: &str| tr.is_fresh(tr.get(p).unwrap());
    assert!(!fresh(&tr, "g1") && !fresh(&tr, "g2"));
    assert!(fresh(&tr, "a") && fresh(&tr, "b") && fresh(&tr, "c"));
    tr.record(&ev(Kind::Read, &["d"]), now);
    assert!(!fresh(&tr, "a"));
    // A new event on an old path makes it fresh again.
    tr.record(&ev(Kind::Grep, &["g1"]), now);
    assert!(fresh(&tr, "g1") && !fresh(&tr, "g2") && !fresh(&tr, "b"));
}

#[test]
fn x_and_a_clear_event_empty_the_trail() {
    let (mut t, mut app) = seeded();
    assert_eq!(app.trail.len(), 7);
    keys(&mut app, "x");
    assert!(app.trail.is_empty());
    assert_eq!(app.flash_text(), Some("Cleared the agent's trail."));
    seed(&mut app);
    app.record_event(&Event::Clear);
    assert!(app.trail.is_empty());
    draw(&mut t, &mut app);
}

#[test]
fn layer_1_off_hides_the_trail_and_keeps_it() {
    let (mut t, mut app) = seeded();
    let f = file(&app, TEST);
    let at = screen(&app, app.layout.tiles[file(&app, TEST)]);
    app.select(Entry::File(file(&app, RULES)), false);
    draw(&mut t, &mut app);
    assert_eq!(cell_bg(&t, at), theme::trail_bg(Kind::Edit, true));
    keys(&mut app, "1");
    draw(&mut t, &mut app);
    assert!(!app.trail_on);
    assert_eq!(app.trail.len(), 7);
    assert_eq!(cell_bg(&t, at), theme::PANE);
    assert!(app.trail_mark(f).is_none());
    assert!(row(&t, 44).starts_with(" 1 ○ trail "), "{}", row(&t, 44));
    keys(&mut app, "1");
    draw(&mut t, &mut app);
    assert_eq!(cell_bg(&t, at), theme::trail_bg(Kind::Edit, true));
}

#[test]
fn trail_events_and_x_move_no_tile_and_keep_the_camera() {
    let (mut t, mut app) = quayside(150, 45);
    let (tiles, headers, cam) = (
        app.layout.tiles.clone(),
        app.layout.headers.clone(),
        (app.cam_x, app.cam_y),
    );
    seed(&mut app);
    draw(&mut t, &mut app);
    keys(&mut app, "x");
    draw(&mut t, &mut app);
    assert_eq!(app.layout.tiles, tiles);
    assert_eq!(app.layout.headers, headers);
    assert_eq!((app.cam_x, app.cam_y), cam);
}

#[test]
fn an_event_for_a_new_path_shows_after_the_index_adds_it() {
    let (mut t, mut app) = quayside(150, 45);
    let new = "components/berth/src/quayside/berth/windows.clj";
    app.record_event(&ev(Kind::Edit, &[new]));
    draw(&mut t, &mut app);
    let mut index: Index = app.index.clone();
    index.files.push(IndexFile {
        lines: 20,
        untracked: true,
        ..IndexFile::new(new)
    });
    app.apply_index(Update {
        index,
        changed: vec![],
        head_moved: false,
    });
    app.select(Entry::File(file(&app, new)), true);
    draw(&mut t, &mut app);
    let f = file(&app, new);
    assert_eq!(app.trail_mark(f).map(|(m, _)| m.kind), Some(Kind::Edit));
    // Deselect, so the pulse does not cover the cell.
    app.select(Entry::File(file(&app, RULES)), false);
    draw(&mut t, &mut app);
    assert_eq!(
        cell_bg(&t, screen(&app, app.layout.tiles[f])),
        theme::trail_bg(Kind::Edit, true)
    );
}

#[test]
fn read_and_grep_colors_have_high_saturation() {
    for c in [theme::READ, theme::GREP, theme::EDIT] {
        let (s, _) = theme::saturation_lightness(c);
        assert!(s >= 0.60, "{c:?} saturation {s:.3}");
    }
}

#[test]
fn the_panel_line_names_the_sender_and_the_age() {
    let mark = |kind, lines, source: Option<&str>| trail::Mark {
        kind,
        seq: 1,
        at: Duration::ZERO,
        lines,
        source: source.map(str::to_string),
    };
    let d = trail::describe;
    assert_eq!(
        d(
            &mark(Kind::Edit, Some((40, 58)), Some("claude-code")),
            Duration::from_secs(150)
        ),
        "CC edited lines 40–58 · 2m ago"
    );
    assert_eq!(
        d(
            &mark(Kind::Edit, Some((7, 7)), Some("codex")),
            Duration::from_secs(5)
        ),
        "codex edited line 7 · 5s ago"
    );
    assert_eq!(
        d(&mark(Kind::Edit, None, None), Duration::from_secs(5)),
        "agent edited this file · 5s ago"
    );
    assert_eq!(
        d(
            &mark(Kind::Read, None, Some("claude-code")),
            Duration::from_secs(5)
        ),
        "CC read this file · 5s ago"
    );
    assert_eq!(
        d(
            &mark(Kind::Grep, None, Some("claude-code")),
            Duration::from_secs(60)
        ),
        "CC found this file by search · 1m ago"
    );
    assert_eq!(
        d(&mark(Kind::Read, None, None), Duration::from_secs(7200)),
        "agent read this file · today"
    );
}

#[test]
fn the_seeded_trail_on_the_map_the_tree_the_overview_and_the_panel() {
    let (mut t, mut app) = seeded();
    app.now = Duration::from_secs(125);
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());

    // The panel's trail line, in the edit color.
    let (y, line) = (0..45)
        .map(|y| (y, row(&t, y)))
        .find(|(_, r)| r.contains("CC edited lines 40–58 · 2m ago"))
        .expect("the panel's trail line");
    let x = line
        .find("CC edited")
        .map(|b| line[..b].chars().count())
        .unwrap() as u16;
    assert_eq!(t.backend().buffer()[(x, y)].fg, theme::EDIT);

    // The tree marks each visible trail file with `▌` in its first cell.
    let rows = app.tree_rows();
    let top = app.filetree.top;
    for (k, r) in rows.iter().enumerate().skip(top).take(41) {
        let Entry::File(f) = r.entry else {
            continue;
        };
        let cell = &t.backend().buffer()[(0, (2 + k - top) as u16)];
        match app.trail_mark(f) {
            Some((m, _)) => {
                assert_eq!(cell.symbol(), "▌", "{}", app.tree.files[f].path);
                assert_eq!(cell.fg, theme::trail_color(m.kind));
            }
            None => assert_ne!(cell.symbol(), "▌", "{}", app.tree.files[f].path),
        }
    }

    // The overview colors the cells with trail files, inside the viewport too.
    for p in [RULES, ALLOC, PLANNER, UI, HANDLER, SCORING, TEST] {
        let f = file(&app, p);
        let (x, y) = overview_cell(&app, f);
        let fg = t.backend().buffer()[(x as u16, y as u16)].fg;
        let want = [theme::EDIT, theme::READ, theme::GREP];
        assert!(want.contains(&fg), "{p}: {fg:?}");
    }
    let (x, y) = overview_cell(&app, file(&app, TEST));
    assert_eq!(t.backend().buffer()[(x as u16, y as u16)].fg, theme::EDIT);
}

#[test]
fn fresh_and_older_trail_backgrounds_on_the_map() {
    let (mut t, mut app) = seeded();
    // Select a file with no trail, so no trail cell pulses.
    app.select(
        Entry::File(file(&app, "components/berth/src/quayside/berth/core.clj")),
        true,
    );
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
    let want = [
        (PLANNER, Kind::Read, true),
        (UI, Kind::Read, true),
        (TEST, Kind::Edit, true),
        (RULES, Kind::Read, false),
        (ALLOC, Kind::Read, false),
        (HANDLER, Kind::Grep, false),
        (SCORING, Kind::Grep, false),
    ];
    let v = app.view();
    let mut checked = 0;
    for (p, kind, fresh) in want {
        let f = file(&app, p);
        assert_eq!(
            app.trail_mark(f).map(|(m, fr)| (m.kind, fr)),
            Some((kind, fresh)),
            "{p}"
        );
        let (x, y) = screen(&app, app.layout.tiles[f]);
        let o = app.overview().unwrap();
        if v.in_map(x, y) && !o.contains(x, y) {
            assert_eq!(cell_bg(&t, (x, y)), theme::trail_bg(kind, fresh), "{p}");
            checked += 1;
        }
    }
    assert!(checked >= 3, "only {checked} trail tiles in view");
}
