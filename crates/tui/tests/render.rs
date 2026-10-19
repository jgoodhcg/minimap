//! Snapshot tests of the screen on the fake `quayside` repo (Ratatui `TestBackend` + `insta`).

mod common;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use common::*;
use minimap_core::{Index, Tree};
use minimap_tui::filetree::Entry;
use minimap_tui::{App, icons, theme};

#[test]
fn file_selected() {
    let (t, app) = quayside(160, 45);
    assert_eq!(
        app.selected_path().as_deref(),
        Some("components/vessel/deps.edn")
    );
    insta::assert_snapshot!(t.backend());
}

#[test]
fn directory_selected() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "h");
    assert_eq!(
        app.selection(),
        Some(Entry::Dir(node(&app, "components/vessel")))
    );
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn tree_hidden() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "t");
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn panel_44_wide() {
    let (t, app) = quayside(110, 35);
    assert_eq!(app.view().rw, 44);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn no_panel() {
    let (t, app) = quayside(90, 30);
    assert_eq!(app.view().rw, 0);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn after_panning() {
    let (mut t, mut app) = quayside(160, 45);
    let (x, y) = (app.cam_x, app.cam_y);
    keys(&mut app, "dds");
    assert_eq!((app.cam_x, app.cam_y), (x + 22, y + 2));
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn help_overlay() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "?");
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
    keys(&mut app, "j");
    assert_eq!(
        app.selected_path().as_deref(),
        Some("components/vessel/deps.edn"),
        "keys wait while the help shows"
    );
    keys(&mut app, "?");
    assert!(!app.help);
}

#[test]
fn no_icons() {
    let (mut t, mut app) = quayside(160, 45);
    app.icons = false;
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn the_panel_shows_the_repo_stats_above_the_selection() {
    let stats = "2,585 files · 468 dirs · 312k lines";
    let (t, app) = quayside(160, 45);
    let x0 = (160 - app.view().rw + 2) as usize;
    let line = |t: &ratatui::Terminal<TestBackend>, y: u16, x0: usize| {
        row(t, y).chars().skip(x0).collect::<String>()
    };
    // One row when both fit in the 50-cell panel: the name left, the stats right-aligned.
    let r = line(&t, 2, x0);
    assert!(
        r.starts_with("quayside ") && r.trim_end().ends_with(stats),
        "{r:?}"
    );
    assert!(line(&t, 3, x0 - 1).starts_with("──────"));
    // The map starts right below the top bar, and no map row shows the stats.
    assert_eq!(app.view().my, 1);
    assert!((1..44).all(|y| {
        !row(&t, y)
            .chars()
            .take(160 - 54)
            .collect::<String>()
            .contains("files ·")
    }));
    assert!(row(&t, 1).starts_with(" FILES"));

    // At 110 columns the panel is 44 wide: the stats take the next row.
    let (t, app) = quayside(110, 35);
    let x0 = (110 - app.view().rw + 2) as usize;
    assert_eq!(line(&t, 2, x0).trim_end(), "quayside");
    assert!(line(&t, 3, x0).trim_end().ends_with(stats));
    assert!(line(&t, 4, x0 - 1).starts_with("──────"));
}

#[test]
fn the_top_bar_shows_the_selection_breadcrumb() {
    let (mut t, mut app) = quayside(160, 45);
    assert!(row(&t, 0).starts_with(" quayside › components › vessel › deps.edn "));
    let b = t.backend().buffer();
    let name = &b[(34, 0)];
    assert_eq!((name.symbol(), name.fg), ("d", theme::INK));
    keys(&mut app, "h");
    draw(&mut t, &mut app);
    assert!(row(&t, 0).starts_with(" quayside › components › vessel "));
}

#[test]
fn the_selection_panel_describes_the_file() {
    let (t, app) = quayside(160, 45);
    let x0 = (160 - app.view().rw + 2) as usize;
    let line = |y: u16| row(&t, y).chars().skip(x0).collect::<String>();
    // The type icon, then the name.
    assert_eq!(
        line(4).chars().skip(2).collect::<String>().trim_end(),
        "deps.edn"
    );
    let b = t.backend().buffer();
    assert_eq!(b[(x0 as u16, 4)].fg, icons::type_color("deps.edn", false));
    assert_eq!(b[(x0 as u16 + 2, 4)].fg, theme::INK);
    assert_eq!(line(5).trim_end(), "components/vessel/");
    assert_eq!(line(6).trim_end(), "38 lines", "no district name");
    assert_eq!(line(42).trim_end(), "⏎ view · o outline · y path · ? keys");
}

#[test]
fn header_and_tile_colors() {
    let (mut t, mut app) = quayside(160, 45);
    // At the lower camera limits nothing is under the overview.
    app.pan(-10_000, -10_000);
    draw(&mut t, &mut app);
    let buf = t.backend().buffer();

    // Every depth-1 header on screen is ink and bold: headers are neutral.
    let v = app.view();
    let mut seen = 0;
    for h in app.layout.headers.iter().filter(|h| h.depth == 1) {
        let (sx, sy) = screen(&app, (h.x, h.y));
        if v.in_map(sx, sy) {
            let cell = &buf[(sx as u16, sy as u16)];
            assert_eq!(cell.fg, theme::INK, "{}", app.tree.nodes[h.node].path);
            assert!(cell.modifier.contains(ratatui::style::Modifier::BOLD));
            seen += 1;
        }
    }
    assert!(seen > 0);

    // A tile is one cell: the file-type icon in its type color, on the pane. The next cell is the gap.
    let i = file(&app, "components/event-bus/src/quayside/event_bus/bus.clj");
    let (sx, sy) = screen(&app, app.layout.tiles[i]);
    let (sx, sy) = (sx as u16, sy as u16);
    let cell = &buf[(sx, sy)];
    assert_eq!(cell.symbol(), icons::for_name("a.clj").glyph.to_string());
    assert_eq!(cell.fg, icons::type_color("a.clj", false));
    assert_eq!(cell.bg, theme::PANE);
    assert_eq!(buf[(sx + 1, sy)].symbol(), " ");

    // The selected tile has the pulse background and a white bold icon. The selection here keeps the camera.
    let f = file(&app, "components/ui-kit/deps.edn");
    app.select(Entry::File(f), false);
    draw(&mut t, &mut app);
    let buf = t.backend().buffer();
    let (sx, sy) = screen(&app, app.layout.tiles[f]);
    let sel = &buf[(sx as u16, sy as u16)];
    assert_eq!(
        (sel.bg, sel.fg),
        (theme::selection_bg(app.pulse_level()), theme::WHITE)
    );
    assert!(sel.modifier.contains(ratatui::style::Modifier::BOLD));

    // A deeper header shows its own name, and a compacted chain shows the chain.
    let h = app.header(node(&app, "components/ui-kit/src/quayside/ui_kit"));
    let (hx, hy) = screen(&app, (h.x, h.y));
    let text: String = (0..21)
        .map(|k| buf[((hx + k) as u16, hy as u16)].symbol().to_string())
        .collect();
    assert_eq!(text, "src/quayside/ui_kit/ ");
}

/// The v5 district palette and the root-files color, removed in unit 023.
const FORMER_DISTRICT_COLORS: [u32; 11] = [
    0x7f98b3, 0x8fa98f, 0xb5a47c, 0xb08878, 0x9d8bb0, 0x7aa5a5, 0xa9a97a, 0x8e93a8, 0xa58f9e,
    0x8aa0b8, 0x6b7680,
];

#[test]
fn no_cell_uses_a_district_color() {
    use ratatui::style::Color;
    let (mut t, mut app) = quayside(160, 45);
    let former: Vec<Color> = FORMER_DISTRICT_COLORS
        .iter()
        .map(|&h| Color::Rgb((h >> 16) as u8, (h >> 8) as u8, h as u8))
        .collect();
    // The start view, a directory selected, and the whole map at the lower limits.
    for step in ["", "h", "wwwwwwwwaaaaa"] {
        keys(&mut app, step);
        draw(&mut t, &mut app);
        let buf = t.backend().buffer();
        for cell in buf.content() {
            assert!(
                !former.contains(&cell.fg) && !former.contains(&cell.bg),
                "{cell:?} after {step:?}"
            );
        }
    }
}

#[test]
fn tree_names_are_neutral_and_tree_icons_have_their_type_color() {
    let (mut t, mut app) = quayside(160, 45);
    // The last row is a root file, so the tree shows the root's files.
    keys(&mut app, "G");
    draw(&mut t, &mut app);
    let buf = t.backend().buffer();
    let (y, _) = app.view().tree_rows();
    let rows = app.tree_rows();
    let mut seen = 0;
    for (k, r) in rows
        .iter()
        .enumerate()
        .skip(app.filetree.top)
        .take(app.filetree.height)
    {
        let ry = (y + (k - app.filetree.top) as i32) as u16;
        let ix = 1 + 2 * r.depth as u16;
        match r.entry {
            Entry::File(f) if app.selection() != Some(r.entry) => {
                let name = &app.tree.files[f].name;
                let binary = app.index.files[f].binary;
                assert_eq!(
                    buf[(ix + 2, ry)].fg,
                    icons::type_color(name, binary),
                    "{name} icon"
                );
                assert_eq!(buf[(ix + 4, ry)].fg, theme::INK2, "{name} name");
                seen += 1;
            }
            Entry::Dir(_) if r.depth == 0 => assert_eq!(buf[(ix + 4, ry)].fg, theme::INK),
            _ => {}
        }
    }
    assert!(seen > 3);
}
#[test]
fn camera_clamps_at_the_layout_edges() {
    let (_, mut app) = quayside(160, 45);
    let v = app.view();
    app.pan(-10_000, -10_000);
    // Left, the camera goes past the overview and its 2-cell gap.
    assert_eq!(
        (app.cam_x, app.cam_y),
        (-(app.overview().unwrap().bw + 2), -1)
    );
    app.pan(10_000, 10_000);
    assert_eq!(
        (app.cam_x, app.cam_y),
        (app.layout.w - v.mw + 2, app.layout.h - v.mh + 1)
    );
    // Shift pans 4x: 'A' moves 44 columns left.
    let x = app.cam_x;
    keys(&mut app, "A");
    assert_eq!(app.cam_x, x - 44);
}

#[test]
fn a_layout_smaller_than_the_view_stays_at_the_top_left() {
    let index = Index::from_json(r#"{"root":"tiny","files":[{"p":"a.clj","n":3}]}"#).unwrap();
    let (_, mut app) = app_at(index, 160, 45);
    app.pan(50, 50);
    assert_eq!(
        (app.cam_x, app.cam_y),
        (-(app.overview().unwrap().bw + 2), -1)
    );
    assert_eq!(Tree::from_index(&app.index).nodes.len(), 1);
    assert_eq!(
        app.selected_path().as_deref(),
        Some("a.clj"),
        "a repo with only root files starts on the first row"
    );
}

#[test]
fn resize_clamps_the_camera() {
    let (_, mut app) = quayside(160, 45);
    app.pan(10_000, 10_000);
    let before = app.cam_x;
    let mut wide = Terminal::new(TestBackend::new(240, 45)).unwrap();
    draw(&mut wide, &mut app);
    assert_eq!(app.cam_x, app.layout.w - app.view().mw + 2);
    assert!(app.cam_x < before);
}

/// Every cell of every drawn non-hidden tile outside the overview, in screen coordinates.
fn visible_tile_cells(app: &App, t: &Terminal<TestBackend>) -> Vec<((i32, i32), String)> {
    let v = app.view();
    let buf = t.backend().buffer();
    let (tw, th) = app.layout.cfg.tile_size();
    let o = app.overview().unwrap();
    let mut out = Vec::new();
    for (i, &p) in app.layout.tiles.iter().enumerate() {
        let f = &app.tree.files[i];
        if f.hidden || f.ignored {
            continue;
        }
        let (x, y) = screen(app, p);
        for dy in 0..th {
            for dx in 0..tw {
                let (sx, sy) = (x + dx, y + dy);
                if v.in_map(sx, sy) && !o.contains(sx, sy) {
                    let cell = &buf[(sx as u16, sy as u16)];
                    out.push((
                        (sx, sy),
                        format!("{}{:?}{:?}", cell.symbol(), cell.fg, cell.bg),
                    ));
                }
            }
        }
    }
    out
}

#[test]
fn hiding_hidden_files_moves_no_visible_tile() {
    let (mut t, mut app) = quayside(160, 45);
    let shown = visible_tile_cells(&app, &t);
    assert!(!shown.is_empty());
    keys(&mut app, ".");
    assert!(!app.show_hidden);
    draw(&mut t, &mut app);
    assert_eq!(visible_tile_cells(&app, &t), shown);
    insta::assert_snapshot!(t.backend());
    keys(&mut app, ".");
    assert!(app.show_hidden);
}

#[test]
fn the_selection_pulses_between_the_30_and_70_percent_mixes() {
    use ratatui::style::Color;
    use std::time::Duration;
    let (mut t, mut app) = quayside(160, 45);
    let f = file(&app, "components/vessel/deps.edn");
    let bg_at = |t: &mut Terminal<TestBackend>, app: &mut App, ms: u64| {
        app.now = Duration::from_millis(ms);
        draw(t, app);
        let (sx, sy) = screen(app, app.layout.tiles[f]);
        t.backend().buffer()[(sx as u16, sy as u16)].bg
    };
    let rgb = |c: Color| match c {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => panic!("{c:?} is not RGB"),
    };
    let lo = rgb(theme::mix(theme::SEL_HI, theme::PANE, 0.3));
    let hi = rgb(theme::mix(theme::SEL_HI, theme::PANE, 0.7));
    // A quarter period apart: the middle of the pulse, then its peak.
    assert_ne!(bg_at(&mut t, &mut app, 0), bg_at(&mut t, &mut app, 500));
    assert_eq!(rgb(bg_at(&mut t, &mut app, 500)), hi);
    assert_eq!(rgb(bg_at(&mut t, &mut app, 1500)), lo);
    for ms in (0..4000).step_by(80) {
        let c = rgb(bg_at(&mut t, &mut app, ms));
        assert!(
            (0..3).all(|k| lo[k] <= c[k] && c[k] <= hi[k]),
            "{c:?} at {ms} ms"
        );
    }
    assert_eq!(app.next_redraw(), Some(minimap_tui::app::PULSE_FRAME));

    // m holds the pulse at the 50 % mix, and the loop waits for input.
    keys(&mut app, "m");
    let mid = theme::mix(theme::SEL_HI, theme::PANE, 0.5);
    for ms in [0, 250, 500, 1500, 1999] {
        assert_eq!(bg_at(&mut t, &mut app, ms), mid, "{ms} ms");
    }
    assert_eq!(app.next_redraw(), None);
    assert!(
        row(&t, 44).starts_with(" 1 ● trail  4 ○ age  5 ○ churn  . ● hidden  m ○ pulse "),
        "{:?}",
        row(&t, 44)
    );
    keys(&mut app, "m");
    draw(&mut t, &mut app);
    assert!(row(&t, 44).starts_with(" 1 ● trail  4 ○ age  5 ○ churn  . ● hidden  m ● pulse "));
}

#[test]
fn a_selected_directory_pulses_its_header_and_shades_its_block() {
    use std::time::Duration;
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "h");
    let n = node(&app, "components/vessel");
    let h = app.header(n);
    app.now = Duration::from_millis(500);
    draw(&mut t, &mut app);
    let buf = t.backend().buffer();
    let (hx, hy) = screen(&app, (h.x, h.y));
    let peak = theme::selection_bg(1.0);
    assert_eq!(buf[(hx as u16, hy as u16)].bg, peak, "header text");
    assert_eq!(
        buf[((hx + 8) as u16, hy as u16)].bg,
        theme::BLOCK_BG,
        "the rule is on the block"
    );
    // The gap cell after the block's first file is part of the block.
    let (fx, fy) = screen(
        &app,
        app.layout.tiles[file(&app, "components/vessel/deps.edn")],
    );
    assert_eq!(buf[((fx + 1) as u16, fy as u16)].bg, theme::BLOCK_BG);
    // Outside the block, the pane.
    assert_eq!(buf[((hx - 1) as u16, (hy + 1) as u16)].bg, theme::PANE);
}
