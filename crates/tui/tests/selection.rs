//! The one selection (unit 018): tree keys, arrows, scrolling, the tree toggle and width, the clipboard, and the mouse.

mod common;

use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};

use common::*;
use minimap_core::{Index, Layout};
use minimap_tui::App;
use minimap_tui::app::{FLASH, MARGIN, TREE_MAX, TREE_MIN, TREE_W};
use minimap_tui::filetree::Entry;

/// `a/` holds two files and `a/b/`, then `c/`, then a root file.
fn small() -> (ratatui::Terminal<ratatui::backend::TestBackend>, App) {
    let index = Index::from_json(
        r#"{"root":"tiny","files":[
            {"p":"top.md","n":3},{"p":"a/x.clj","n":10},{"p":"a/y.clj","n":20},
            {"p":"a/b/z.clj","n":30},{"p":"c/w.clj","n":40}]}"#,
    )
    .unwrap();
    app_at(index, 160, 45)
}

fn labels(app: &App) -> Vec<String> {
    app.tree_rows().iter().map(|r| label(app, r)).collect()
}

fn selected(app: &App) -> String {
    app.selected_path().unwrap()
}

#[test]
fn startup_selects_the_first_file_below_the_root_and_opens_the_tree_to_it() {
    let (_, app) = small();
    assert_eq!(selected(&app), "a/x.clj", "top.md is at the root");
    assert_eq!(
        labels(&app),
        ["a/", "  b/", "  x.clj", "  y.clj", "c/", "top.md"]
    );
    assert_selection_in_view(&app);

    let (_, app) = quayside(160, 45);
    assert_eq!(selected(&app), "components/vessel/deps.edn");
    assert!(app.filetree.is_open(node(&app, "components/vessel")));
    assert_selection_in_view(&app);
}

#[test]
fn j_k_l_h_and_enter_move_through_the_tree() {
    let (_, mut app) = small();
    keys(&mut app, "k");
    assert_eq!(selected(&app), "a/b");
    keys(&mut app, "kk");
    assert_eq!(selected(&app), "a", "k stops at the first row");

    // l on a closed directory opens it; on an open directory it goes to the first child.
    keys(&mut app, "j");
    keys(&mut app, "l");
    assert!(app.filetree.is_open(node(&app, "a/b")));
    assert_eq!(selected(&app), "a/b");
    keys(&mut app, "l");
    assert_eq!(selected(&app), "a/b/z.clj");
    keys(&mut app, "l");
    assert_eq!(selected(&app), "a/b/z.clj", "l on a file does nothing");

    // h on a file goes to its directory; on an open directory it closes it; on a closed one it goes up.
    keys(&mut app, "h");
    assert_eq!(selected(&app), "a/b");
    keys(&mut app, "h");
    assert!(!app.filetree.is_open(node(&app, "a/b")));
    assert_eq!(selected(&app), "a/b");
    keys(&mut app, "h");
    assert_eq!(selected(&app), "a");
    keys(&mut app, "h");
    assert!(!app.filetree.is_open(node(&app, "a")));
    keys(&mut app, "h");
    assert_eq!(
        selected(&app),
        "a",
        "a top-level directory has no parent row"
    );

    // Enter opens and closes a directory, and does nothing on a file.
    press(&mut app, KeyCode::Enter);
    assert!(app.filetree.is_open(node(&app, "a")));
    press(&mut app, KeyCode::Enter);
    assert!(!app.filetree.is_open(node(&app, "a")));
    keys(&mut app, "jjj");
    assert_eq!(selected(&app), "top.md");
    press(&mut app, KeyCode::Enter);
    assert_eq!(selected(&app), "top.md");
    keys(&mut app, "j");
    assert_eq!(selected(&app), "top.md", "j stops at the last row");
}

#[test]
fn arrows_pick_the_nearest_file_in_that_direction_and_open_the_tree_to_it() {
    let (_, mut app) = small();
    let pos = |app: &App, p: &str| app.layout.tiles[file(app, p)];
    // x and y share a grid row in a/, and b/z sits below them.
    assert_eq!(pos(&app, "a/y.clj").1, pos(&app, "a/x.clj").1);
    assert!(pos(&app, "a/b/z.clj").1 > pos(&app, "a/x.clj").1);

    press(&mut app, KeyCode::Right);
    assert_eq!(selected(&app), "a/y.clj");
    press(&mut app, KeyCode::Left);
    assert_eq!(selected(&app), "a/x.clj");
    assert!(!app.filetree.is_open(node(&app, "a/b")));
    press(&mut app, KeyCode::Down);
    assert_eq!(selected(&app), "a/b/z.clj");
    assert!(
        app.filetree.is_open(node(&app, "a/b")),
        "the tree opens to the selection"
    );
    let rows = app.tree_rows();
    assert!(app.filetree.cursor_index(&rows).is_some());
    press(&mut app, KeyCode::Up);
    assert_ne!(selected(&app), "a/b/z.clj");

    // With a directory selected, an arrow selects the first file row in the tree.
    app.select(Entry::Dir(node(&app, "c")), true);
    press(&mut app, KeyCode::Right);
    let first_file = app
        .tree_rows()
        .iter()
        .find_map(|r| match r.entry {
            Entry::File(f) => Some(app.tree.files[f].path.clone()),
            Entry::Dir(_) => None,
        })
        .unwrap();
    assert_eq!(selected(&app), first_file);
}

#[test]
fn every_selection_change_scrolls_the_map_to_show_it_with_a_margin() {
    let (_, mut app) = quayside(160, 45);
    for _ in 0..60 {
        keys(&mut app, "j");
        assert_selection_in_view(&app);
    }
    for code in [KeyCode::Right, KeyCode::Down, KeyCode::Left, KeyCode::Up] {
        for _ in 0..15 {
            press(&mut app, code);
            assert_selection_in_view(&app);
        }
    }
    // A far jump by the tree, then a directory.
    app.select(
        Entry::File(file(
            &app,
            "bases/console/src/quayside/console/views/berth/planner.cljs",
        )),
        true,
    );
    assert_selection_in_view(&app);
    keys(&mut app, "h");
    assert!(app.selected_dir().is_some());
    assert_selection_in_view(&app);
}

#[test]
fn a_small_move_scrolls_only_as_far_as_needed() {
    let (_, mut app) = quayside(160, 45);
    let (v, o, m) = (app.view(), app.overview().unwrap(), MARGIN);
    let cam = (app.cam_x, app.cam_y);
    let clear_x = |sx: i32| sx >= v.mx + m && sx <= v.mx + v.mw - 1 - m;
    let clear_of_overview = |sx: i32, sy: i32| sx - v.mx > o.bw || sy - v.my >= o.bh;
    // A file in view, inside the margin, and clear of the overview: no scroll.
    let inside = (0..app.tree.files.len())
        .find(|&f| {
            let (sx, sy) = screen(&app, app.layout.tiles[f]);
            app.file_visible(f)
                && clear_x(sx)
                && sy >= v.my + m
                && sy <= v.my + v.mh - 1 - m
                && clear_of_overview(sx, sy)
        })
        .unwrap();
    app.select(Entry::File(inside), true);
    assert_eq!((app.cam_x, app.cam_y), cam);
    // A file below the view: the camera moves down only to the margin, and not sideways.
    let below = (0..app.tree.files.len())
        .find(|&f| {
            let (sx, sy) = screen(&app, app.layout.tiles[f]);
            app.file_visible(f) && clear_x(sx) && sx - v.mx > o.bw && sy > v.my + v.mh + 3
        })
        .unwrap();
    let (_, y) = app.layout.tiles[below];
    app.select(Entry::File(below), true);
    assert_eq!((app.cam_x, app.cam_y), (cam.0, y - (v.mh - 1 - m)));
}

#[test]
fn tree_toggle_width_drag_and_selection_never_move_a_tile() {
    let (mut t, mut app) = quayside(160, 45);
    let tiles = app.layout.tiles.clone();
    let headers = app.layout.headers.clone();
    keys(&mut app, "t");
    assert!(!app.tree_on && app.view().tw == 0);
    draw(&mut t, &mut app);
    keys(&mut app, "t");
    assert_eq!(app.view().tw, TREE_W);
    keys(&mut app, "]]");
    assert_eq!(app.tree_w, TREE_W + 8);
    keys(&mut app, "[[[[[[[[[[");
    assert_eq!(app.tree_w, TREE_MIN);
    let edge = app.view().tw;
    drag(&mut app, (edge, 10), (60, 12));
    assert_eq!(app.tree_w, 60);
    draw(&mut t, &mut app);
    keys(&mut app, "jjjlljj");
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    keys(&mut app, ".");
    draw(&mut t, &mut app);
    keys(&mut app, ".");
    assert_eq!(app.layout.tiles, tiles);
    assert_eq!(app.layout.headers, headers);
    let fresh = Layout::compute(&app.tree, &app.layout.cfg);
    assert_eq!(fresh.tiles, tiles);
}

#[test]
fn hiding_the_tree_widens_the_map_and_keeps_the_map_in_place() {
    let (mut t, mut app) = quayside(160, 45);
    // A file in the middle of the layout, so the camera is not at a clamp limit.
    let mid = (0..app.tree.files.len())
        .min_by_key(|&f| {
            let (x, y) = app.layout.tiles[f];
            (x - app.layout.w / 2).abs() + (y - app.layout.h / 2).abs()
        })
        .unwrap();
    app.select(Entry::File(mid), true);
    let before = app.view();
    let at = screen(&app, app.layout.tiles[mid]);
    keys(&mut app, "t");
    let after = app.view();
    assert_eq!(after.mx, 0);
    assert_eq!(after.mw, before.mw + before.tw + 1);
    assert_eq!(
        screen(&app, app.layout.tiles[mid]),
        at,
        "the selected tile keeps its screen cell"
    );
    keys(&mut app, "t");
    assert_eq!(screen(&app, app.layout.tiles[mid]), at);
    keys(&mut app, "t");
    draw(&mut t, &mut app);
    assert!(!row(&t, 1).contains("FILES"));
    // j and k still move the selection while the tree is hidden.
    app.select(
        Entry::File(file(
            &app,
            "components/event-bus/src/quayside/event_bus/interface.clj",
        )),
        true,
    );
    keys(&mut app, "k");
    assert_eq!(
        selected(&app),
        "components/event-bus/src/quayside/event_bus/bus.clj"
    );
}

#[test]
fn a_narrow_terminal_narrows_the_tree_so_the_map_keeps_30_columns() {
    let (_, app) = quayside(60, 20);
    let v = app.view();
    assert_eq!((v.rw, v.tw, v.mw), (0, 29, 30));
    let (_, app) = quayside(40, 20);
    assert_eq!(app.view().tw, TREE_MIN, "the tree keeps its minimum width");
    let (_, mut app) = quayside(160, 45);
    for _ in 0..20 {
        keys(&mut app, "]");
    }
    assert_eq!(app.tree_w, TREE_MAX);
    assert_eq!(app.view().tw, 160 - 54 - 1 - 30);
}

#[test]
fn y_copies_the_path_and_flashes_it_for_3_2_seconds() {
    let (mut t, mut app) = quayside(160, 45);
    let copied = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink = copied.clone();
    app.copy = Box::new(move |s| {
        sink.borrow_mut().push(s.to_string());
        Ok(())
    });
    app.now = Duration::from_secs(10);
    // Without the pulse, only the flash asks for a redraw.
    app.pulse = false;
    keys(&mut app, "y");
    assert_eq!(*copied.borrow(), ["components/vessel/deps.edn"]);
    draw(&mut t, &mut app);
    assert!(row(&t, 0).contains("Copied path: components/vessel/deps.edn"));
    assert_eq!(app.next_redraw(), Some(FLASH));
    app.now += FLASH;
    draw(&mut t, &mut app);
    assert!(!row(&t, 0).contains("Copied"));
    assert_eq!(app.next_redraw(), None);

    // A directory copies its path. A clipboard error flashes the path with `Copy failed`.
    keys(&mut app, "h");
    app.copy = Box::new(|_| Err("no display".to_string()));
    keys(&mut app, "y");
    draw(&mut t, &mut app);
    assert!(
        row(&t, 0).contains("Copy failed: components/vessel"),
        "{}",
        row(&t, 0)
    );
}

#[test]
fn the_hidden_toggle_moves_a_hidden_selection_to_the_first_row() {
    let (_, mut app) = quayside(160, 45);
    app.select(Entry::File(file(&app, ".github/workflows/check.yml")), true);
    keys(&mut app, ".");
    let rows = app.tree_rows();
    assert_eq!(app.selection(), Some(rows[0].entry));
    assert_selection_in_view(&app);
    // A visible selection stays.
    keys(&mut app, ".j.");
    assert_eq!(app.selection(), Some(app.tree_rows()[1].entry));
}

#[test]
fn clicks_select_tiles_headers_and_tree_rows() {
    let (mut t, mut app) = quayside(160, 45);
    let v = app.view();
    // At the lower camera limits nothing is under the overview.
    app.pan(-10_000, -10_000);
    // A tile click selects the file without scrolling the map.
    let f = file(&app, "components/event-bus/src/quayside/event_bus/bus.clj");
    let (sx, sy) = screen(&app, app.layout.tiles[f]);
    assert!(v.in_map(sx, sy));
    let cam = (app.cam_x, app.cam_y);
    click(&mut app, sx + 1, sy);
    assert_ne!(
        app.selection(),
        Some(Entry::File(f)),
        "the gap after a tile is empty"
    );
    click(&mut app, sx, sy);
    assert_eq!(app.selection(), Some(Entry::File(f)));
    assert_eq!((app.cam_x, app.cam_y), cam);

    // A header click selects the directory; the rule after the text does not.
    let n = node(&app, "components");
    let h = app.header(n);
    let (hx, hy) = screen(&app, (h.x, h.y));
    click(&mut app, hx + 2, hy);
    assert_eq!(app.selection(), Some(Entry::Dir(n)));
    click(&mut app, hx + 20, hy);
    assert_eq!(
        app.selection(),
        Some(Entry::Dir(n)),
        "a click on the rule changes nothing"
    );
    // The root header does not select: the root has no tree row.
    keys(&mut app, "wwwwwwww");
    let (rx, ry) = screen(&app, (0, 0));
    click(&mut app, rx + 1, ry);
    assert_eq!(app.selection(), Some(Entry::Dir(n)));

    // A tree row click selects the row and toggles a directory: the first closed directory row in view.
    draw(&mut t, &mut app);
    let rows = app.tree_rows();
    let in_view =
        |app: &App| app.filetree.top..rows.len().min(app.filetree.top + app.filetree.height);
    let d = in_view(&app)
        .find_map(|k| match rows[k].entry {
            Entry::Dir(d) if !app.filetree.is_open(d) => Some((k, d)),
            _ => None,
        })
        .unwrap();
    let (ty, _) = v.tree_rows();
    let row_y = ty + (d.0 - app.filetree.top) as i32;
    click(&mut app, 5, row_y);
    assert_eq!(app.selection(), Some(Entry::Dir(d.1)));
    assert!(app.filetree.is_open(d.1));
    assert_selection_in_view(&app);
    click(&mut app, 5, row_y);
    assert!(!app.filetree.is_open(d.1));
    // A file row in view.
    let rows = app.tree_rows();
    let k = (app.filetree.top..rows.len().min(app.filetree.top + app.filetree.height))
        .find(|&k| matches!(rows[k].entry, Entry::File(_)))
        .unwrap();
    let file_y = ty + (k - app.filetree.top) as i32;
    click(&mut app, 5, file_y);
    assert_eq!(app.selection(), Some(rows[k].entry));
}

#[test]
fn hover_shows_the_path_in_the_top_bar() {
    let (mut t, mut app) = quayside(160, 45);
    app.pan(-10_000, -10_000);
    let f = file(&app, "components/event-bus/src/quayside/event_bus/bus.clj");
    let (sx, sy) = screen(&app, app.layout.tiles[f]);
    mouse(&mut app, MouseEventKind::Moved, sx, sy);
    assert_eq!(app.hover, Some(Entry::File(f)));
    draw(&mut t, &mut app);
    assert!(
        row(&t, 0)
            .trim_end()
            .ends_with("components/event-bus/src/quayside/event_bus/bus.clj")
    );
    assert_eq!(
        t.backend().buffer()[(sx as u16, sy as u16)].bg,
        minimap_tui::theme::TILE_HI
    );
    // A directory header shows its path with a slash.
    let h = app.header(node(&app, "components"));
    let (hx, hy) = screen(&app, (h.x, h.y));
    mouse(&mut app, MouseEventKind::Moved, hx + 1, hy);
    draw(&mut t, &mut app);
    assert!(row(&t, 0).trim_end().ends_with(" components/"));
    mouse(&mut app, MouseEventKind::Moved, 2, 2);
    assert_eq!(app.hover, None);
}

#[test]
fn the_wheel_scrolls_the_tree_and_never_the_map() {
    let (_, mut app) = quayside(160, 30);
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    app.filetree.top = 0;
    let cam = (app.cam_x, app.cam_y);
    let sel = app.selection();
    mouse(&mut app, MouseEventKind::ScrollDown, 5, 5);
    assert_eq!(app.filetree.top, 3);
    mouse(&mut app, MouseEventKind::ScrollUp, 5, 5);
    assert_eq!(app.filetree.top, 0);
    let v = app.view();
    for kind in [
        MouseEventKind::ScrollDown,
        MouseEventKind::ScrollUp,
        MouseEventKind::ScrollLeft,
        MouseEventKind::ScrollRight,
    ] {
        mouse(&mut app, kind, v.mx + 10, v.my + 5);
    }
    assert_eq!((app.cam_x, app.cam_y), cam, "the wheel moved the map");
    assert_eq!(app.selection(), sel, "the wheel moved the selection");
}

#[test]
fn dragging_the_map_pans_it_and_does_not_select() {
    let (_, mut app) = quayside(160, 45);
    app.pan(100, 20);
    let (x, y) = (app.cam_x, app.cam_y);
    let sel = app.selection();
    let v = app.view();
    let start = (v.mx + 30, v.my + 20);
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        start.0,
        start.1,
    );
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        start.0 - 10,
        start.1 - 4,
    );
    assert_eq!((app.cam_x, app.cam_y), (x + 10, y + 4));
    mouse(
        &mut app,
        MouseEventKind::Up(MouseButton::Left),
        start.0 - 10,
        start.1 - 4,
    );
    assert_eq!(app.selection(), sel, "a drag is not a click");
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 0, 0);
    assert_eq!(
        (app.cam_x, app.cam_y),
        (x + 10, y + 4),
        "no drag after release"
    );

    // A drag that starts in the selection panel or on the top bar does not pan.
    for from in [(v.cols - 10, 10), (v.mx + 10, 0)] {
        drag(&mut app, from, (from.0 - 20, from.1 + 5));
        assert_eq!((app.cam_x, app.cam_y), (x + 10, y + 4));
    }
}

#[test]
fn dragging_the_tree_edge_sets_its_width_within_the_limits() {
    let (mut t, mut app) = quayside(160, 45);
    let edge = app.view().tw;
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), edge, 20);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 50, 21);
    assert_eq!(app.view().tw, 50);
    assert!(app.dragging_tree_edge());
    draw(&mut t, &mut app);
    assert_eq!(t.backend().buffer()[(50, 10)].symbol(), "┃");
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 3, 21);
    assert_eq!(app.tree_w, TREE_MIN);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 150, 21);
    assert_eq!(app.tree_w, TREE_MAX);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 150, 21);
    assert!(!app.dragging_tree_edge());
    draw(&mut t, &mut app);
    assert_eq!(
        t.backend().buffer()[(app.view().tw as u16, 10)].symbol(),
        "│"
    );
}
