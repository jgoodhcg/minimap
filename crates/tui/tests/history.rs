//! The git group in the selection panel and the heat layers on the map (unit `008`).

mod common;

use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

use common::*;
use minimap_core::History;
use minimap_core::history::{Heat, Loader};
use minimap_tui::App;
use minimap_tui::app::GitHistory;
use minimap_tui::filetree::Entry;
use minimap_tui::theme;

const CORE: &str = "components/berth/src/quayside/berth/core.clj";

fn berth_core() -> (Terminal<TestBackend>, App) {
    let (mut t, mut app) = quayside_history(150, 45);
    let f = file(&app, CORE);
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    (t, app)
}

fn panel(t: &Terminal<TestBackend>, app: &App) -> Vec<String> {
    let v = app.view();
    (1..v.rows - 1)
        .map(|y| {
            row(t, y as u16)
                .chars()
                .skip((v.cols - v.rw + 2) as usize)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

/// Every drawn tile's screen cell and its background.
fn tile_backgrounds(t: &Terminal<TestBackend>, app: &App) -> Vec<(usize, Color)> {
    let v = app.view();
    let o = app.overview();
    let b = t.backend().buffer();
    let mut out = Vec::new();
    for (i, &xy) in app.layout.tiles.iter().enumerate() {
        let (sx, sy) = screen(app, xy);
        if v.in_map(sx, sy) && !o.is_some_and(|o| o.contains(sx, sy)) && app.file_visible(i) {
            out.push((i, b[(sx as u16, sy as u16)].bg));
        }
    }
    out
}

#[test]
fn panel_with_the_git_group() {
    let (t, app) = berth_core();
    let p = panel(&t, &app);
    let git = p.iter().position(|r| r.starts_with("GIT")).unwrap();
    assert!(p[git].ends_with("50 commits"), "{}", p[git]);
    assert_eq!(p[git + 1], "changed 3y ago · created 4y ago");
    // Descriptions drop the `area:` prefix.
    assert!(
        p[git + 2].trim_start().starts_with("3y  "),
        "{}",
        p[git + 2]
    );
    assert!(p[git + 2..git + 5].iter().all(|r| !r.contains("berth:")));
    // The group ends right above the key hint.
    assert!(p[git + 5].starts_with("⏎ view · o outline"));
    insta::assert_snapshot!(t.backend());
}

#[test]
fn map_with_age_heat() {
    let (mut t, mut app) = berth_core();
    keys(&mut app, "4");
    draw(&mut t, &mut app);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn heat_colors_the_cell_background_by_ramp_step() {
    let (mut t, mut app) = berth_core();
    for (key, heat) in [("4", Heat::Age), ("5", Heat::Churn)] {
        keys(&mut app, key);
        assert_eq!(app.heat, Some(heat));
        draw(&mut t, &mut app);
        let sel = app.selected_file();
        let mut seen = [false; 4];
        for (i, bg) in tile_backgrounds(&t, &app) {
            if Some(i) == sel || app.search.is_some() {
                continue;
            }
            let h = app.history().unwrap();
            let step = heat.step(h, h.file(&app.index.files[i].path).unwrap());
            let want = step.map_or(theme::PANE, theme::heat_bg);
            assert_eq!(bg, want, "{} {:?}", app.index.files[i].path, heat);
            seen[step.unwrap_or(3)] = true;
        }
        // Tiles with heat and tiles past the last step are on screen. (No quayside tile here has 60 commits.)
        assert!(
            seen[3] && seen[..3].iter().filter(|&&s| s).count() >= 2,
            "{heat:?}: {seen:?}"
        );
        // The glyph keeps its type color.
        let (i, _) = tile_backgrounds(&t, &app)[0];
        let (sx, sy) = screen(&app, app.layout.tiles[i]);
        let fg = t.backend().buffer()[(sx as u16, sy as u16)].fg;
        if Some(i) != app.selected_file() {
            assert_eq!(
                fg,
                minimap_tui::icons::type_color(&app.tree.files[i].name, app.index.files[i].binary)
            );
        }
    }
}

#[test]
fn four_and_five_are_exclusive_and_toggle_off() {
    let (_, mut app) = berth_core();
    assert_eq!(app.heat, None);
    keys(&mut app, "4");
    assert_eq!(app.heat, Some(Heat::Age));
    keys(&mut app, "5");
    assert_eq!(app.heat, Some(Heat::Churn));
    keys(&mut app, "5");
    assert_eq!(app.heat, None);
}

#[test]
fn heat_moves_no_tile_and_no_camera_and_leaves_the_overview() {
    let (mut t, mut app) = berth_core();
    let tiles = app.layout.tiles.clone();
    let headers = app.layout.headers.clone();
    let cam = (app.cam_x, app.cam_y);
    let o = app.overview().unwrap();
    let overview = |t: &Terminal<TestBackend>| -> Vec<String> {
        let b = t.backend().buffer();
        (o.y..o.y + o.bh)
            .flat_map(|y| (o.x..o.x + o.bw).map(move |x| (x, y)))
            .map(|(x, y)| {
                let c = &b[(x as u16, y as u16)];
                format!("{}{:?}{:?}", c.symbol(), c.fg, c.bg)
            })
            .collect()
    };
    let before = overview(&t);
    for key in ["4", "5", "4", "4"] {
        keys(&mut app, key);
        draw(&mut t, &mut app);
        assert_eq!(app.layout.tiles, tiles);
        assert_eq!(app.layout.headers, headers);
        assert_eq!((app.cam_x, app.cam_y), cam);
        assert_eq!(overview(&t), before, "the overview shows no layers");
    }
}

#[test]
fn the_status_bar_shows_the_toggles_and_the_active_legend() {
    let (mut t, mut app) = berth_core();
    let status = row(&t, 44);
    assert!(
        status.starts_with(" 1 ● trail  4 ○ age  5 ○ churn  . ● hidden"),
        "{status}"
    );
    keys(&mut app, "4");
    draw(&mut t, &mut app);
    let status = row(&t, 44);
    assert!(
        status.starts_with(" 1 ● trail  4 ● age  5 ○ churn    <2w   <2mo   <1y  . ● hidden"),
        "{status}"
    );
    let b = t.backend().buffer();
    let x = status[..status.find("<2w").unwrap()].chars().count() as u16;
    assert_eq!(b[(x - 2, 44)].bg, theme::heat_bg(0));
    assert_eq!(b[(x - 1, 44)].bg, theme::heat_bg(0));
    assert_eq!(
        b[(13, 44)].fg,
        theme::HEAT,
        "the on toggle has the heat color"
    );
    keys(&mut app, "5");
    draw(&mut t, &mut app);
    assert!(row(&t, 44).starts_with(" 1 ● trail  4 ○ age  5 ● churn    60+   25+   10+"));
}

#[test]
fn the_panel_shows_loading_until_the_walk_finishes() {
    let (mut t, mut app) = quayside_details(150, 45);
    let (tx, loader) = Loader::channel();
    app.git = GitHistory::Loading(loader);
    let f = file(&app, CORE);
    app.select(Entry::File(f), true);
    // A heat layer can turn on while the walk runs.
    keys(&mut app, "4");
    draw(&mut t, &mut app);
    let p = panel(&t, &app);
    let git = p.iter().position(|r| r.starts_with("GIT")).unwrap();
    assert!(p[git].ends_with("loading"));
    assert!(
        app.next_redraw()
            .is_some_and(|d| d <= Duration::from_millis(100))
    );
    assert!(
        tile_backgrounds(&t, &app)
            .iter()
            .all(|&(i, bg)| Some(i) == app.selected_file() || bg == theme::PANE)
    );
    keys(&mut app, "H");
    assert_eq!(app.flash_text(), Some("Git history is still loading."));

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/quayside-details.json"
    );
    let h = History::from_json(&std::fs::read_to_string(path).unwrap(), NOW).unwrap();
    tx.send((Ok(h), Duration::from_millis(5))).unwrap();
    draw(&mut t, &mut app);
    assert!(app.history().is_some());
    let p = panel(&t, &app);
    assert!(
        p.iter()
            .any(|r| r.starts_with("GIT") && r.ends_with("50 commits"))
    );
    assert!(
        tile_backgrounds(&t, &app)
            .iter()
            .any(|&(_, bg)| bg == theme::heat_bg(0))
    );
}

#[test]
fn a_failed_walk_says_so() {
    let (mut t, mut app) = quayside_details(150, 45);
    let (tx, loader) = Loader::channel();
    app.git = GitHistory::Loading(loader);
    tx.send((Err(anyhow::anyhow!("bad object")), Duration::ZERO))
        .unwrap();
    draw(&mut t, &mut app);
    assert!(matches!(app.git, GitHistory::Failed(_)));
    assert!(
        panel(&t, &app)
            .iter()
            .any(|r| r.starts_with("Git history failed: bad object"))
    );
}

#[test]
fn shift_h_copies_the_newest_commit_hash() {
    let (_, mut app) = berth_core();
    let copies = record_copies(&mut app);
    keys(&mut app, "H");
    let f = app.file_history(file(&app, CORE)).unwrap();
    let newest = app
        .history()
        .unwrap()
        .recent(f)
        .next()
        .unwrap()
        .hash
        .clone();
    assert_eq!(copies.borrow().as_slice(), std::slice::from_ref(&newest));
    assert_eq!(
        app.flash_text(),
        Some(format!("Copied commit: {newest}").as_str())
    );
    // A file with a history but no recent commits in the fixture has nothing to copy.
    let readme = file(&app, "README.md");
    app.select(Entry::File(readme), true);
    keys(&mut app, "H");
    assert_eq!(app.flash_text(), Some("No commits for this file."));
}

#[test]
fn a_file_without_commits_says_so() {
    let (mut t, mut app) = quayside_details(150, 45);
    app.git = GitHistory::Ready(History::default());
    let f = file(&app, CORE);
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    let p = panel(&t, &app);
    let git = p.iter().position(|r| r.starts_with("GIT")).unwrap();
    assert!(p[git].ends_with("0 commits"));
    assert_eq!(p[git + 1], "No commits yet.");
}

#[test]
fn heat_needs_a_git_history() {
    let (_, mut app) = quayside(150, 45);
    keys(&mut app, "4");
    assert_eq!(app.heat, None);
    assert_eq!(app.flash_text(), Some("No git history for this repo."));
}

#[test]
fn the_heat_color_is_a_high_chroma_state_color_with_fading_steps() {
    let (s, _) = theme::saturation_lightness(theme::HEAT);
    assert!(s >= 0.6, "heat saturation {s:.3}");
    assert_eq!(
        theme::heat_bg(0),
        theme::state_bg(theme::HEAT),
        "the strongest step is the state mix"
    );
    let lum = |c: Color| match c {
        Color::Rgb(r, g, b) => u32::from(r) + u32::from(g) + u32::from(b),
        _ => 0,
    };
    assert!(lum(theme::heat_bg(0)) > lum(theme::heat_bg(1)));
    assert!(lum(theme::heat_bg(1)) > lum(theme::heat_bg(2)));
    assert!(lum(theme::heat_bg(2)) > lum(theme::PANE));
}
