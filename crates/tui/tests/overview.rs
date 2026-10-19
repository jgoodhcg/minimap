//! The overview in the map's top-left corner (unit 020).

mod common;

use ratatui::crossterm::event::{MouseButton, MouseEventKind};

use common::*;
use minimap_tui::app::Overview;
use minimap_tui::filetree::Entry;
use minimap_tui::{App, theme};

/// The viewport cells in the drawn overview: edge and inside, by background.
fn drawn_viewport(
    t: &ratatui::Terminal<ratatui::backend::TestBackend>,
    o: &Overview,
) -> Vec<(i32, i32, bool)> {
    let buf = t.backend().buffer();
    let mut out = Vec::new();
    for cy in 0..o.h {
        for cx in 0..o.w {
            let bg = buf[((o.x + 1 + cx) as u16, (o.y + 1 + cy) as u16)].bg;
            if bg == theme::VIEW_EDGE {
                out.push((cx, cy, true));
            } else if bg == theme::VIEW_IN {
                out.push((cx, cy, false));
            } else {
                assert_eq!(bg, theme::MINI_BG, "cell ({cx}, {cy})");
            }
        }
    }
    out
}

#[test]
fn the_overview_keeps_the_map_aspect_within_its_limits() {
    let (_, app) = quayside(160, 45);
    let o = app.overview().unwrap();
    // The map region is 71 x 43: at most 21 x 9 cells. For the 292 x 224 map the height limits it.
    assert_eq!((app.view().mw, app.view().mh), (71, 43));
    assert_eq!((app.layout.w, app.layout.h), (292, 224));
    assert_eq!((o.w, o.h, o.bw, o.bh), (12, 9, 14, 11));
    assert!((o.s - 9.0 / 224.0).abs() < 1e-12);
    assert_eq!((o.x, o.y), (app.view().mx, app.view().my));
    // A wide map region: the 26-cell cap.
    let (_, app) = quayside(400, 100);
    let o = app.overview().unwrap();
    assert!(o.w <= 26 && o.h <= 9, "{o:?}");
}

#[test]
fn the_viewport_box_covers_the_cells_of_the_camera() {
    let (mut t, mut app) = quayside(160, 45);
    let o = app.overview().unwrap();
    app.cam_x = 0;
    app.cam_y = 0;
    draw(&mut t, &mut app);
    // 71 x 43 cells at 9/224 per cell: columns 0-2, rows 0-1. Every cell is on the edge.
    let want: Vec<(i32, i32, bool)> = (0..2)
        .flat_map(|cy| (0..3).map(move |cx| (cx, cy, true)))
        .collect();
    assert_eq!(drawn_viewport(&t, &o), want);

    app.cam_x = 150;
    app.cam_y = 118;
    draw(&mut t, &mut app);
    // Columns 6-8 and rows 4-6: the middle cell is inside, the rest is the edge.
    let want: Vec<(i32, i32, bool)> = (4..7)
        .flat_map(|cy| (6..9).map(move |cx| (cx, cy, !(cx == 7 && cy == 5))))
        .collect();
    assert_eq!(drawn_viewport(&t, &o), want);
    assert_eq!(o.viewport((150, 118), 71, 43), (6, 4, 8, 6));
}

#[test]
fn a_click_in_the_overview_centers_the_view_there() {
    let (_, mut app) = quayside(160, 45);
    let o = app.overview().unwrap();
    // Cell (6, 4) is the layout point ((6 + 0.5) / s, (4 + 0.5) / s) = (161.8, 112.0).
    click(&mut app, o.x + 1 + 6, o.y + 1 + 4);
    assert_eq!((app.cam_x, app.cam_y), (126, 90));
    // A corner cell clamps the camera.
    click(&mut app, o.x + 1, o.y + 1);
    assert_eq!((app.cam_x, app.cam_y), (-(o.bw + 2), -1));
    click(&mut app, o.x + o.w, o.y + o.h);
    assert_eq!(
        (app.cam_x, app.cam_y),
        (
            app.layout.w - app.view().mw + 2,
            app.layout.h - app.view().mh + 1
        )
    );
    // The click moves the view only: the selection stays.
    assert_eq!(
        app.selected_path().as_deref(),
        Some("components/vessel/deps.edn")
    );
}

#[test]
fn a_drag_across_the_overview_moves_the_camera_with_it() {
    let (_, mut app) = quayside(160, 45);
    let o = app.overview().unwrap();
    let (x0, y0) = (o.x + 1, o.y + 1);
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        x0 + 1,
        y0 + 1,
    );
    assert_eq!((app.cam_x, app.cam_y), (2, 16));
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        x0 + 6,
        y0 + 4,
    );
    assert_eq!((app.cam_x, app.cam_y), (126, 90));
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        x0 + 8,
        y0 + 5,
    );
    assert_eq!((app.cam_x, app.cam_y), (176, 115));
    mouse(
        &mut app,
        MouseEventKind::Up(MouseButton::Left),
        x0 + 8,
        y0 + 5,
    );
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        x0 + 1,
        y0 + 1,
    );
    assert_eq!((app.cam_x, app.cam_y), (176, 115), "no drag after release");
}

#[test]
fn at_the_lower_camera_limits_no_tile_is_under_the_overview() {
    let (_, mut app) = quayside(160, 45);
    app.pan(-10_000, -10_000);
    let o = app.overview().unwrap();
    assert_eq!((app.cam_x, app.cam_y), (-16, -1));
    for (i, &p) in app.layout.tiles.iter().enumerate() {
        let (sx, sy) = screen(&app, p);
        assert!(
            !o.contains(sx, sy),
            "{} is under the overview",
            app.tree.files[i].path
        );
    }
    // A mouse over the overview hits nothing on the map below it.
    app.cam_x = 0;
    app.cam_y = 0;
    assert_eq!(app.hit(o.x + 2, o.y + 2), None);
}

#[test]
fn a_selection_under_the_overview_moves_out_from_under_it() {
    let (_, mut app) = quayside(160, 45);
    app.cam_x = 0;
    app.cam_y = 0;
    let o = app.overview().unwrap();
    // A root file near the top-left corner lies under the overview at this camera.
    let f = file(&app, "README.md");
    assert!(o.contains(
        screen(&app, app.layout.tiles[f]).0,
        screen(&app, app.layout.tiles[f]).1
    ));
    app.select(Entry::File(f), true);
    let (sx, sy) = screen(&app, app.layout.tiles[f]);
    assert!(
        !o.contains(sx, sy),
        "({sx}, {sy}) is under the overview {o:?}"
    );
    assert!(app.view().in_map(sx, sy));
    assert_eq!(sx, o.x + o.bw + 2, "the file sits right of the overview");
}

#[test]
fn every_selection_stays_clear_of_the_overview() {
    let (_, mut app) = quayside(160, 45);
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    let rows = app.tree_rows();
    app.select(rows[0].entry, true);
    for k in 0..rows.len() {
        let o = app.overview().unwrap();
        let (x0, y0, _, _) = selection_rect(&app);
        assert!(
            !o.contains(x0, y0),
            "row {k} ({:?}) is under the overview",
            app.selected_path()
        );
        assert!(app.view().in_map(x0, y0), "row {k} is outside the map");
        keys(&mut app, "j");
    }
}

#[test]
fn overview_snapshots() {
    let (mut t, mut app) = quayside(160, 45);
    insta::assert_snapshot!("overview_initial", t.backend());
    app.pan(10_000, 10_000);
    draw(&mut t, &mut app);
    insta::assert_snapshot!("overview_far_right_and_bottom", t.backend());
    let o = app.overview().unwrap();
    let vp = drawn_viewport(&t, &o);
    assert!(
        vp.iter().any(|&(cx, cy, _)| cx == o.w - 1 && cy == o.h - 1),
        "the viewport reaches the corner"
    );
}

#[test]
fn the_overview_dots_count_the_visible_files() {
    let (mut t, mut app) = quayside(160, 45);
    let dots = |t: &ratatui::Terminal<ratatui::backend::TestBackend>, app: &App| -> u32 {
        let (o, buf) = (app.overview().unwrap(), t.backend().buffer());
        let mut n = 0;
        for cy in 0..o.h {
            for cx in 0..o.w {
                let ch = buf[((o.x + 1 + cx) as u16, (o.y + 1 + cy) as u16)]
                    .symbol()
                    .chars()
                    .next()
                    .unwrap();
                if ('\u{2800}'..='\u{28ff}').contains(&ch) {
                    n += (ch as u32 - 0x2800).count_ones();
                }
            }
        }
        n
    };
    let all = dots(&t, &app);
    assert!(all > 0);
    keys(&mut app, ".");
    draw(&mut t, &mut app);
    assert!(dots(&t, &app) <= all, "hiding files removes dots");
}
