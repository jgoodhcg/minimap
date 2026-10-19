//! View toggles and tree ends (unit 022): `p` panel, `v` overview, `g` / `G` first and last tree rows.

mod common;

use ratatui::crossterm::event::KeyCode;

use common::*;
use minimap_tui::filetree::Entry;

#[test]
fn toggles_and_tree_ends_never_move_a_tile() {
    let (mut t, mut app) = quayside(160, 45);
    let tiles = app.layout.tiles.clone();
    for k in ["p", "v", "G", "g", "p", "v"] {
        keys(&mut app, k);
        draw(&mut t, &mut app);
    }
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Home);
    draw(&mut t, &mut app);
    assert_eq!(app.layout.tiles, tiles);
}

#[test]
fn hiding_the_panel_widens_the_map_and_keeps_tiles_in_place() {
    let (mut t, mut app) = quayside(160, 45);
    let before = app.view();
    let f = app.selected_file().unwrap();
    let at = screen(&app, app.layout.tiles[f]);
    keys(&mut app, "p");
    let after = app.view();
    assert!(!app.panel_on);
    assert_eq!((after.rw, after.mw), (0, before.mw + before.rw));
    assert_eq!(
        screen(&app, app.layout.tiles[f]),
        at,
        "the selected tile keeps its screen cell"
    );
    draw(&mut t, &mut app);
    assert!(
        !(1..44).any(|y| row(&t, y).contains("files ·")),
        "the stats hide with the panel"
    );
    assert!(row(&t, 44).contains(" p ○ panel "));
    keys(&mut app, "p");
    assert_eq!(app.view(), before);
}

#[test]
fn p_below_100_columns_flashes_and_keeps_the_panel_hidden() {
    let (mut t, mut app) = quayside(90, 30);
    keys(&mut app, "p");
    assert!(app.panel_on, "the setting does not change");
    assert_eq!(app.view().rw, 0);
    draw(&mut t, &mut app);
    assert!(
        row(&t, 0).contains("The panel needs 100 columns"),
        "{:?}",
        row(&t, 0)
    );
}

#[test]
fn with_the_overview_hidden_the_camera_clamps_at_minus_two_and_clicks_hit_the_map() {
    let (mut t, mut app) = quayside(160, 45);
    let o = app.overview().unwrap();
    keys(&mut app, "v");
    assert!(app.overview().is_none());
    app.pan(-10_000, -10_000);
    assert_eq!((app.cam_x, app.cam_y), (-2, -1));
    draw(&mut t, &mut app);
    assert!(row(&t, 44).contains(" v ○ overview "));
    // A tile inside the old overview box: a click selects it.
    let f = (0..app.tree.files.len())
        .find(|&f| {
            let (sx, sy) = screen(&app, app.layout.tiles[f]);
            app.file_visible(f) && o.contains(sx, sy)
        })
        .expect("a tile under the old overview box");
    let (sx, sy) = screen(&app, app.layout.tiles[f]);
    click(&mut app, sx, sy);
    assert_eq!(app.selection(), Some(Entry::File(f)));
    // Selecting it scrolls nothing: the overview is gone.
    assert_eq!((app.cam_x, app.cam_y), (-2, -1));
    // Showing the overview again clamps the camera to its new lower limit.
    keys(&mut app, "v");
    assert_eq!(
        app.cam_x, -2,
        "-2 is inside the wider limit, so the camera stays"
    );
    app.pan(-10_000, 0);
    assert_eq!(app.cam_x, -(o.bw + 2));
}

#[test]
fn g_and_shift_g_select_the_first_and_last_tree_rows() {
    let (_, mut app) = quayside(160, 45);
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    keys(&mut app, "G");
    let rows = app.tree_rows();
    assert_eq!(app.selection(), Some(rows[rows.len() - 1].entry));
    assert_selection_in_view(&app);
    let i = app.filetree.cursor_index(&rows).unwrap();
    assert!(i >= app.filetree.top && i < app.filetree.top + app.filetree.height);
    keys(&mut app, "g");
    assert_eq!(app.selection(), Some(rows[0].entry));
    assert_eq!(app.filetree.top, 0);
    assert_selection_in_view(&app);
    press(&mut app, KeyCode::End);
    assert_eq!(app.selection(), Some(rows[rows.len() - 1].entry));
    press(&mut app, KeyCode::Home);
    assert_eq!(app.selection(), Some(rows[0].entry));
}

#[test]
fn toggle_snapshots() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "p");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("panel_hidden", t.backend());
    keys(&mut app, "pv");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("overview_hidden", t.backend());
}
