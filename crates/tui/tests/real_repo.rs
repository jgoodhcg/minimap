//! Real-repo tests of the map. They read `MINIMAP_TARGET_REPO` and `MINIMAP_CONFIG`, and write only under `.private/`.

use std::path::PathBuf;
use std::time::Instant;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use minimap_core::config::expand_home;
use minimap_core::index::repo_root;
use minimap_core::{Config, Details, Index, IndexOptions};
use minimap_tui::App;
use minimap_tui::filetree::Entry;

/// Resolves a relative path against the workspace root, because `cargo test` runs in the crate directory.
fn workspace_path(p: &str) -> PathBuf {
    let p = expand_home(p);
    if p.is_absolute() {
        p
    } else {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(p)
    }
}

/// The app on the target repo's live index, with file details read from its working tree.
fn target_app() -> App {
    let repo = std::env::var("MINIMAP_TARGET_REPO").expect("set MINIMAP_TARGET_REPO");
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let excludes = Config::load(config.as_deref()).unwrap().excludes_for(&root);
    let mut app = App::new(
        Index::from_repo(
            &root,
            &IndexOptions {
                excludes,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    app.details = Details::repo(root);
    app
}

fn draw(t: &mut Terminal<TestBackend>, app: &mut App) {
    t.draw(|f| minimap_tui::draw(f, app)).unwrap();
}

fn row(t: &Terminal<TestBackend>, y: u16) -> String {
    let buf = t.backend().buffer();
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn map_renders_within_the_frame_budget() {
    let t0 = Instant::now();
    let mut app = target_app();
    let startup = t0.elapsed().as_secs_f64() * 1000.0;

    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    draw(&mut t, &mut app);
    let first: String = t.backend().to_string();
    let capture = workspace_path(".private/captures/real-initial-200x60.txt");
    std::fs::create_dir_all(capture.parent().unwrap()).unwrap();
    std::fs::write(&capture, &first).unwrap();

    // The tree, and the selection panel with the repo stats at its top, show.
    let v = app.view();
    let r1 = row(&t, 1);
    assert!(r1.starts_with(" FILES"), "tree header");
    let stats_row: String = row(&t, 2)
        .chars()
        .skip((v.cols - v.rw + 2) as usize)
        .collect();
    assert!(
        stats_row.starts_with(&app.tree.root_name),
        "panel repo name"
    );
    assert!(stats_row.contains(" files · "), "panel stats");
    assert!(
        row(&t, (v.rows - 3) as u16).contains("y path"),
        "selection panel hint"
    );

    // Pan across the whole map, one frame per step, and time the frames.
    let frames = 200;
    let t1 = Instant::now();
    let mut worst = 0.0f64;
    for k in 0..frames {
        let f0 = Instant::now();
        let (dx, dy) = if k % 20 < 10 {
            (app.layout.w / 20, 0)
        } else {
            (-(app.layout.w / 40), app.layout.h / 20)
        };
        app.pan(dx, dy);
        draw(&mut t, &mut app);
        worst = worst.max(f0.elapsed().as_secs_f64() * 1000.0);
    }
    let mean = t1.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    eprintln!(
        "real repo map: {} files, startup (index + layout) {startup:.0} ms, frame at 200x60 mean {mean:.2} ms, worst {worst:.2} ms, capture {}",
        app.index.files.len(),
        capture.display()
    );
    assert!(worst < 16.0, "a frame took {worst:.2} ms");
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn hiding_hidden_files_moves_no_visible_tile() {
    let mut app = target_app();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    let hidden = app.tree.files.iter().filter(|f| f.hidden).count();
    assert!(hidden > 0, "the target has hidden files");

    // Every map cell that is not in a hidden tile or under the overview must stay the same.
    let cells = |app: &App, t: &Terminal<TestBackend>| -> Vec<String> {
        let (v, buf) = (app.view(), t.backend().buffer());
        let o = app.overview().unwrap();
        let (tw, th) = app.layout.cfg.tile_size();
        let mut out = Vec::new();
        for (i, &(x, y)) in app.layout.tiles.iter().enumerate() {
            if app.tree.files[i].hidden {
                continue;
            }
            for (dx, dy) in (0..tw).flat_map(|dx| (0..th).map(move |dy| (dx, dy))) {
                let (sx, sy) = (v.mx + x - app.cam_x + dx, v.my + y - app.cam_y + dy);
                if v.in_map(sx, sy) && !o.contains(sx, sy) {
                    let c = &buf[(sx as u16, sy as u16)];
                    out.push(format!("{sx},{sy}:{}{:?}{:?}", c.symbol(), c.fg, c.bg));
                }
            }
        }
        out
    };
    // `show_hidden` is set directly, so the selection does not move with it.
    let mut checked = 0;
    for k in 0..12 {
        draw(&mut t, &mut app);
        app.cam_x = (app.layout.w / 12) * k;
        app.cam_y = (app.layout.h / 12) * k;
        app.show_hidden = true;
        draw(&mut t, &mut app);
        let shown = cells(&app, &t);
        app.show_hidden = false;
        draw(&mut t, &mut app);
        assert_eq!(cells(&app, &t), shown, "view {k}");
        checked += shown.len();
    }
    eprintln!(
        "real repo hidden toggle: {hidden} hidden files, {checked} visible tile cells compared"
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn fully_open_tree_lists_every_file_and_directory_once() {
    let mut app = target_app();
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    let rows = app.tree_rows();
    let (mut files, mut dirs) = (
        vec![0u32; app.tree.files.len()],
        vec![0u32; app.tree.nodes.len()],
    );
    for r in &rows {
        match r.entry {
            Entry::File(f) => files[f] += 1,
            Entry::Dir(n) => dirs[n] += 1,
        }
    }
    assert!(
        files.iter().all(|&c| c == 1),
        "a file is missing or repeated"
    );
    assert_eq!(dirs[0], 0, "the root has no row");
    assert!(
        dirs[1..].iter().all(|&c| c == 1),
        "a directory is missing or repeated"
    );
    // Depth follows the tree: each row's parent row is one level up.
    for (i, r) in rows.iter().enumerate() {
        if let Some(p) = app.filetree.parent_of(&app.tree, r.entry) {
            let pi = rows[..i]
                .iter()
                .rposition(|x| x.entry == p)
                .expect("the parent row comes first");
            assert_eq!(rows[pi].depth + 1, r.depth);
        } else {
            assert_eq!(r.depth, 0);
        }
    }
    eprintln!(
        "real repo tree: {} rows fully open ({} files, {} directories)",
        rows.len(),
        files.len(),
        dirs.len() - 1
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn j_reaches_every_tree_row_and_each_selection_is_drawn_in_the_map() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = target_app();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    draw(&mut t, &mut app);
    let rows = app.tree_rows();
    app.select(rows[0].entry, true);
    let t0 = Instant::now();
    for (k, r) in rows.iter().enumerate() {
        assert_eq!(app.selection(), Some(r.entry), "row {k}");
        draw(&mut t, &mut app);
        let v = app.view();
        // The selected tile, or the selected directory's header text, has the pulse background in the map region.
        let (x, y) = match r.entry {
            Entry::File(f) => app.layout.tiles[f],
            Entry::Dir(n) => {
                let h = app.header(n);
                (h.x, h.y)
            }
        };
        let (sx, sy) = (v.mx + x - app.cam_x, v.my + y - app.cam_y);
        assert!(v.in_map(sx, sy), "row {k} is outside the map region");
        let bg = t.backend().buffer()[(sx as u16, sy as u16)].bg;
        assert_eq!(
            bg,
            minimap_tui::theme::selection_bg(app.pulse_level()),
            "row {k} is not drawn as selected"
        );
        let i = app.filetree.cursor_index(&app.tree_rows()).unwrap();
        assert!(i >= app.filetree.top && i < app.filetree.top + app.filetree.height);
        app.on_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    }
    eprintln!(
        "real repo selection: {} rows selected with j and drawn in {:.0} ms",
        rows.len(),
        t0.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_overview_fits_its_limits_and_holds_the_viewport_at_the_four_corners() {
    use minimap_tui::theme;
    let mut app = target_app();
    for (w, h) in [(200u16, 60u16), (120, 40), (300, 90)] {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        draw(&mut t, &mut app);
        let (v, o) = (app.view(), app.overview().unwrap());
        let max_w = ((f64::from(v.mw) * 0.3).floor() as i32).clamp(4, 26);
        let max_h = ((f64::from(v.mh) * 0.3).floor() as i32).clamp(2, 9);
        assert!(o.w <= max_w && o.h <= max_h, "{o:?} at {w} x {h}");
        assert!(
            o.w == max_w || o.h == max_h,
            "the overview uses one of its limits"
        );
        for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            app.pan(dx * 100_000, dy * 100_000);
            draw(&mut t, &mut app);
            let buf = t.backend().buffer();
            // Every viewport cell lies inside the braille area; the box itself has at least one edge cell.
            let mut edge = 0;
            for y in o.y..o.y + o.bh {
                for x in o.x..o.x + o.bw {
                    let bg = buf[(x as u16, y as u16)].bg;
                    let inner = x > o.x && x < o.x + o.bw - 1 && y > o.y && y < o.y + o.bh - 1;
                    if bg == theme::VIEW_EDGE || bg == theme::VIEW_IN {
                        assert!(inner, "viewport cell ({x}, {y}) is in the margin");
                        edge += usize::from(bg == theme::VIEW_EDGE);
                    }
                }
            }
            assert!(edge > 0, "no viewport at corner ({dx}, {dy}) at {w} x {h}");
        }
        eprintln!(
            "real repo overview at {w} x {h}: {} x {} cells, scale {:.4}",
            o.w, o.h, o.s
        );
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_picker_ranks_a_typed_file_name_in_the_top_3_and_redraws_within_a_frame() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = target_app();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    draw(&mut t, &mut app);
    let press = |app: &mut App, code: KeyCode| app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    // Names from the index, spread over it, so no target name enters this file.
    let n = app.tree.files.len();
    let picks: Vec<usize> = (0..12)
        .map(|k| k * n / 12)
        .filter(|&f| app.file_visible(f) && app.tree.files[f].name.is_ascii())
        .collect();
    // Each keystroke previews the best result, which loads its details the first time (unit 007).
    // The first pass includes those loads; the budget applies to the second pass, with the details cached.
    // `a_cljs_file_shows_its_namespace_shape_and_outline_within_50_ms` and the core outline test time the loads.
    let mut worst = [0.0f64; 2];
    for worst in &mut worst {
        for &f in &picks {
            let name = app.tree.files[f].name.clone();
            press(&mut app, KeyCode::Char('/'));
            for c in name.chars().take(10) {
                let t0 = Instant::now();
                press(&mut app, KeyCode::Char(c));
                draw(&mut t, &mut app);
                *worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
            }
            for c in name.chars().skip(10) {
                press(&mut app, KeyCode::Char(c));
            }
            let s = app.search.as_ref().unwrap();
            let top: Vec<&str> = s
                .results
                .iter()
                .take(3)
                .map(|r| app.tree.files[r.index].name.as_str())
                .collect();
            assert!(
                top.contains(&name.as_str()),
                "{} is not in the top 3",
                app.tree.files[f].path
            );
            press(&mut app, KeyCode::Esc);
        }
    }
    eprintln!(
        "real repo picker: {} names in the top 3, worst keystroke with a redraw at 200x60 {:.2} ms \
         ({:.2} ms with first details loads)",
        picks.len(),
        worst[1],
        worst[0]
    );
    assert!(worst[1] < 16.0, "a keystroke took {:.2} ms", worst[1]);
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn shift_g_and_g_select_the_last_and_first_rows_and_draw_them() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = target_app();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    draw(&mut t, &mut app);
    let rows = app.tree_rows();
    for (key, want) in [('G', rows[rows.len() - 1].entry), ('g', rows[0].entry)] {
        app.on_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
        assert_eq!(app.selection(), Some(want), "{key}");
        draw(&mut t, &mut app);
        let v = app.view();
        let (x, y) = match want {
            Entry::File(f) => app.layout.tiles[f],
            Entry::Dir(n) => {
                let h = app.header(n);
                (h.x, h.y)
            }
        };
        let (sx, sy) = (v.mx + x - app.cam_x, v.my + y - app.cam_y);
        assert!(
            v.in_map(sx, sy),
            "{key}: the selection is outside the map region"
        );
        assert_eq!(
            t.backend().buffer()[(sx as u16, sy as u16)].bg,
            minimap_tui::theme::selection_bg(app.pulse_level()),
            "{key}: the selection is not drawn"
        );
    }
    eprintln!(
        "real repo tree ends: G and g reached rows {} and 0",
        rows.len() - 1
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn a_cljs_file_shows_its_namespace_shape_and_outline_within_50_ms() {
    let mut app = target_app();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    draw(&mut t, &mut app);
    // The `.cljs` file nearest to 1,000 lines.
    let f = (0..app.index.files.len())
        .filter(|&f| app.index.files[f].path.ends_with(".cljs") && app.file_visible(f))
        .min_by_key(|&f| app.index.files[f].lines.abs_diff(1000))
        .unwrap();
    let t0 = Instant::now();
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let v = app.view();
    let panel: Vec<String> = (1..v.rows - 1)
        .map(|y| {
            row(&t, y as u16)
                .chars()
                .skip((v.cols - v.rw + 2) as usize)
                .collect()
        })
        .collect();
    let capture = workspace_path(".private/captures/v007-panel-200x60.txt");
    std::fs::write(&capture, t.backend().to_string()).unwrap();
    let d = app.file_details(f).unwrap().clone();
    let ns = d.namespace.as_deref().expect("a namespace");
    assert!(
        panel.iter().any(|r| r.trim_end() == ns),
        "the namespace line"
    );
    assert!(panel.iter().any(|r| r.starts_with("SHAPE")), "the shape");
    assert!(
        panel.iter().any(|r| r.starts_with("OUTLINE · o to browse")),
        "the outline"
    );
    let first = &d.outline[0];
    let want = format!("{:>4}  {}", first.line, first.name);
    assert!(
        panel.iter().any(|r| r.starts_with(&want)),
        "the first outline row"
    );
    eprintln!(
        "real repo panel: {} lines, {} outline items, select and draw {ms:.1} ms, capture {}",
        app.index.files[f].lines,
        d.outline.len(),
        capture.display()
    );
    assert!(ms < 50.0, "select and draw took {ms:.1} ms");
    // Browse to the end of the outline.
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    app.on_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    for _ in 0..d.outline.len() + 2 {
        app.on_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    }
    assert_eq!(app.outline_browse(), Some(d.outline.len() - 1));
    draw(&mut t, &mut app);
    assert!(row(&t, (v.rows - 1) as u16).starts_with(" OUTLINE"));
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn git_history_loads_in_the_background_and_heat_colors_the_map() {
    use minimap_core::history::{Heat, now};
    use minimap_tui::app::GitHistory;
    let mut app = target_app();
    let repo = std::env::var("MINIMAP_TARGET_REPO").unwrap();
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    let t0 = Instant::now();
    app.start_history(root, now());
    // The first frame draws while the walk runs.
    draw(&mut t, &mut app);
    let first = t0.elapsed().as_secs_f64() * 1000.0;
    assert!(
        matches!(app.git, GitHistory::Loading(_)),
        "the walk runs in the background"
    );
    while matches!(app.git, GitHistory::Loading(_)) {
        assert!(t0.elapsed().as_secs() < 30, "the walk takes under 30 s");
        std::thread::sleep(std::time::Duration::from_millis(20));
        draw(&mut t, &mut app);
    }
    let loaded = t0.elapsed().as_secs_f64();
    assert!(app.history().is_some(), "the walk succeeded");
    let v = app.view();
    let panel: Vec<String> = (1..v.rows - 1)
        .map(|y| {
            row(&t, y as u16)
                .chars()
                .skip((v.cols - v.rw + 2) as usize)
                .collect()
        })
        .collect();
    let f = app.selected_file().unwrap();
    let count = app.file_history(f).unwrap().count;
    assert!(
        panel.iter().any(|r| {
            let r = r.trim_end();
            r.starts_with("GIT") && (r.ends_with(" commits") || r.ends_with(" commit"))
        }),
        "the git group"
    );
    assert!(
        panel.iter().any(|r| r.starts_with("changed ")),
        "the git summary"
    );

    // Each heat layer colors some visible tiles, and moves none.
    let tiles = app.layout.tiles.clone();
    let mut colored = Vec::new();
    for (key, heat) in [('4', Heat::Age), ('5', Heat::Churn)] {
        app.on_key(ratatui::crossterm::event::KeyEvent::new(
            ratatui::crossterm::event::KeyCode::Char(key),
            ratatui::crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.heat, Some(heat));
        draw(&mut t, &mut app);
        assert_eq!(app.layout.tiles, tiles);
        let steps = (0..app.index.files.len())
            .filter(|&i| app.heat_step(i).is_some())
            .count();
        colored.push(steps);
        assert!(steps > 0, "{heat:?} colors some files");
    }
    let capture = workspace_path(".private/captures/v008-heat-200x60.txt");
    std::fs::write(&capture, t.backend().to_string()).unwrap();
    eprintln!(
        "real repo git: first frame {first:.0} ms during the walk, history in {loaded:.2} s, selected file {count} commits, \
         age heat on {} files, churn heat on {} files",
        colored[0], colored[1]
    );
}

/// The share of lit dots in the rows a shape uses. A shape that reads as a block fills most of them.
fn fill_share(shape: &minimap_core::shape::LineShape) -> f64 {
    let b = minimap_tui::shape::Braille::new(shape, 40, 12);
    let lit: u32 = b.cells.iter().map(|c| c.count_ones()).sum();
    f64::from(lit) / (b.rows_used * 4 * b.cols * 2) as f64
}

/// The share of a braille shape's dot rows whose right end differs from the row above by at least 2 dots.
fn ragged_share(shape: &minimap_core::shape::LineShape) -> f64 {
    use minimap_tui::shape::{BIT, Braille};
    let b = Braille::new(shape, 40, 12);
    let dots_h = b.rows_used * 4;
    let right: Vec<Option<usize>> = (0..dots_h)
        .map(|dy| {
            (0..b.cols * 2)
                .rev()
                .find(|&dx| b.cells[(dy >> 2) * b.cols + (dx >> 1)] & BIT[dy & 3][dx & 1] != 0)
        })
        .collect();
    let pairs: Vec<(usize, usize)> = right
        .windows(2)
        .filter_map(|w| Some((w[0]?, w[1]?)))
        .collect();
    if pairs.is_empty() {
        return 0.0;
    }
    pairs.iter().filter(|(a, b)| a.abs_diff(*b) >= 2).count() as f64 / pairs.len() as f64
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn long_clojure_shapes_show_more_structure_with_the_median_band() {
    use minimap_core::shape::LineShape;
    let app = target_app();
    let repo = std::env::var("MINIMAP_TARGET_REPO").unwrap();
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let (mut old, mut new) = (vec![], vec![]);
    let (mut old_fill, mut new_fill) = (vec![], vec![]);
    for f in &app.index.files {
        let clojure = [".clj", ".cljs", ".cljc", ".bb"]
            .iter()
            .any(|e| f.path.ends_with(e));
        if !clojure || f.lines <= 500 || f.binary || f.missing {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&f.path)).unwrap();
        let (a, b) = (
            LineShape::from_text_with(&text, usize::MAX),
            LineShape::from_text(&text),
        );
        old.push(ragged_share(&a));
        new.push(ragged_share(&b));
        old_fill.push(fill_share(&a));
        new_fill.push(fill_share(&b));
    }
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[(v.len() - 1) / 2]
    };
    let n = old.len();
    let (old, new) = (median(old), median(new));
    let (old_fill, new_fill) = (median(old_fill), median(new_fill));
    eprintln!(
        "{n} Clojure files over 500 lines, medians: ragged dot rows min-max {old:.3}, median band {new:.3}; \
         lit dots min-max {old_fill:.3}, median band {new_fill:.3}"
    );
    assert!(n > 20);
    assert!(new > old, "min-max {old:.3}, median band {new:.3}");
    assert!(
        new_fill < old_fill,
        "min-max {old_fill:.3}, median band {new_fill:.3}"
    );
}

/// Writes the braille shapes of three target files, before and after the median band, to `.private/captures/`.
/// The paths come from `MINIMAP_SHAPE_FILES` (comma-separated, repo-relative).
#[test]
#[ignore = "needs MINIMAP_TARGET_REPO; captures only with MINIMAP_SHAPE_FILES"]
fn capture_shapes_before_and_after_the_median_band() {
    use minimap_core::shape::LineShape;
    use minimap_tui::shape::Braille;
    let repo = std::env::var("MINIMAP_TARGET_REPO").unwrap();
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let Ok(files) = std::env::var("MINIMAP_SHAPE_FILES") else {
        eprintln!("skipped: set MINIMAP_SHAPE_FILES to capture shapes");
        return;
    };
    let mut out = String::new();
    for path in files.split(',') {
        let text = std::fs::read_to_string(root.join(path)).unwrap();
        let grids = [usize::MAX, minimap_core::shape::MEDIAN_ABOVE]
            .map(|m| Braille::new(&LineShape::from_text_with(&text, m), 40, 12));
        out.push_str(&format!(
            "{} lines: min-max | median band\n",
            text.lines().count()
        ));
        for r in 0..12 {
            for b in &grids {
                let row: String = (0..40).map(|c| b.glyph(c, r)).collect();
                out.push_str(&row);
                out.push_str(" | ");
            }
            out.push('\n');
        }
        out.push('\n');
    }
    let capture = workspace_path(".private/captures/shape-median-band.txt");
    std::fs::write(&capture, &out).unwrap();
    eprintln!("{out}");
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_largest_clojure_file_views_within_a_frame_and_main_types_have_syntaxes() {
    use minimap_core::Content;
    use minimap_tui::syntax::Highlighter;
    let mut app = target_app();
    let repo = std::env::var("MINIMAP_TARGET_REPO").unwrap();
    let root = repo_root(&workspace_path(&repo)).unwrap();
    app.content = Content::Repo(root);
    let clojure = |p: &str| [".clj", ".cljs", ".cljc"].iter().any(|e| p.ends_with(e));
    let f = (0..app.index.files.len())
        .filter(|&f| clojure(&app.index.files[f].path) && !app.index.files[f].binary)
        .max_by_key(|&f| app.index.files[f].lines)
        .unwrap();
    let lines = app.index.files[f].lines;
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    let t0 = Instant::now();
    app.open_viewer(f, None, false);
    draw(&mut t, &mut app);
    let plain_ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert!(app.viewer.is_some());
    app.wait_for_colors(std::time::Duration::from_secs(30));
    let colored = app
        .colored_for(&app.tree.files[f].path)
        .expect("colors arrive");
    let (syntax, took) = (colored.syntax.clone(), colored.took);
    let t1 = Instant::now();
    draw(&mut t, &mut app);
    let colored_ms = t1.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "largest Clojure file: {lines} lines, {syntax}; open and plain draw {plain_ms:.1} ms, \
         highlight {:.0} ms on the thread, colored draw {colored_ms:.1} ms",
        took.as_secs_f64() * 1000.0
    );
    assert!(plain_ms < 16.0, "open and draw took {plain_ms:.1} ms");
    assert!(colored_ms < 16.0, "colored draw took {colored_ms:.1} ms");

    let (h, _) = Highlighter::new(None);
    for ext in [".cljs", ".sql", ".ts", ".js", ".clj", ".py"] {
        let file = app
            .index
            .files
            .iter()
            .find(|x| x.path.ends_with(ext))
            .unwrap();
        let name = &h.syntax(&file.path, "").name;
        eprintln!("{ext}: {name}");
        assert_ne!(name, "Plain Text", "{}", file.path);
    }
}

/// A line that defines something, per file type, for the generic outline's coverage check (unit 032).
fn defines(ext: &str, line: &str) -> bool {
    fn strip<'a>(mut s: &'a str, prefixes: &[&str]) -> &'a str {
        for p in prefixes {
            s = s.strip_prefix(p).unwrap_or(s);
        }
        s
    }
    let t = line.trim_start();
    match ext {
        "rs" => {
            let s = strip(t, &["pub(crate) ", "pub ", "async "]);
            [
                "fn ",
                "struct ",
                "enum ",
                "trait ",
                "impl ",
                "impl<",
                "mod ",
                "macro_rules!",
            ]
            .iter()
            .any(|k| s.starts_with(k))
        }
        "ts" | "js" => {
            // A named definition: an anonymous `function () {` callback names nothing.
            let s = strip(t, &["export ", "default ", "async "]);
            [
                "function ",
                "function* ",
                "class ",
                "interface ",
                "type ",
                "enum ",
            ]
            .iter()
            .any(|k| {
                s.strip_prefix(k)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
            })
        }
        "py" => ["def ", "class ", "async def "]
            .iter()
            .any(|k| t.starts_with(k)),
        "sql" => {
            let l = t.to_lowercase();
            l.starts_with("create function")
                || l.starts_with("create or replace function")
                || l.starts_with("create table")
        }
        "md" => ["# ", "## ", "### "].iter().any(|k| line.starts_with(k)),
        _ => false,
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn grammar_outlines_cover_the_main_non_clojure_types_and_minimaps_rust() {
    use minimap_tui::syntax::Highlighter;
    let (h, _) = Highlighter::new(None);
    let repo = std::env::var("MINIMAP_TARGET_REPO").unwrap();
    let target = repo_root(&workspace_path(&repo)).unwrap();
    let app = target_app();
    let mut sets: Vec<(String, PathBuf, Vec<String>)> = ["ts", "js", "py", "sql", "md"]
        .iter()
        .map(|ext| {
            let paths = app
                .index
                .files
                .iter()
                .filter(|f| !f.binary && !f.missing && f.path.ends_with(&format!(".{ext}")))
                .map(|f| f.path.clone())
                .collect();
            (ext.to_string(), target.clone(), paths)
        })
        .collect();
    // minimap's own Rust files, the self tab's case.
    let own = workspace_path(".");
    let rs = std::process::Command::new("git")
        .arg("-C")
        .arg(&own)
        .args(["ls-files", "*.rs"])
        .output()
        .unwrap();
    sets.push((
        "rs".to_string(),
        own,
        String::from_utf8(rs.stdout)
            .unwrap()
            .lines()
            .map(String::from)
            .collect(),
    ));
    let t0 = Instant::now();
    for (ext, root, paths) in sets {
        let (mut with, mut sizes, mut defining, mut covered) = (0, vec![], 0, 0);
        for p in &paths {
            let Ok(text) = std::fs::read_to_string(root.join(p)) else {
                continue;
            };
            if minimap_tui::syntax::too_long(&text) {
                continue;
            }
            let items = h.outline(p, &text);
            let lines: Vec<&str> = text.lines().collect();
            let mut last = 0;
            for o in &items {
                assert!(
                    o.line as usize >= 1 && o.line as usize <= lines.len(),
                    "{ext}: a line past the end"
                );
                assert!(o.line > last, "{ext}: lines rise");
                last = o.line;
                let l = lines[o.line as usize - 1];
                assert!(
                    !l.contains(&format!(".{}(", o.name)),
                    "{ext}: a call is an item: {} at {p}:{}",
                    o.name,
                    o.line
                );
            }
            if !items.is_empty() {
                with += 1;
                sizes.push(items.len());
            }
            if lines.iter().any(|l| defines(&ext, l)) {
                defining += 1;
                covered += usize::from(!items.is_empty());
                if items.is_empty() && std::env::var("MINIMAP_DEBUG_OUTLINE").is_ok() {
                    let (n, l) = lines
                        .iter()
                        .enumerate()
                        .find(|(_, l)| defines(&ext, l))
                        .unwrap();
                    eprintln!("  uncovered {p}:{}: {}", n + 1, l.trim());
                }
            }
        }
        sizes.sort_unstable();
        let median = sizes.get(sizes.len() / 2).copied().unwrap_or(0);
        let share = |a: usize, b: usize| if b == 0 { 1.0 } else { a as f64 / b as f64 };
        eprintln!(
            ".{ext}: {} files, {with} with an outline (median {median} items); {covered} of {defining} defining files covered ({:.0} %)",
            paths.len(),
            share(covered, defining) * 100.0
        );
        assert!(
            share(covered, defining) >= 0.9,
            ".{ext}: {covered} of {defining} defining files have an outline"
        );
    }
    eprintln!("generic outlines took {:.1} s", t0.elapsed().as_secs_f64());
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_real_config_keymap_prints_and_parses_back() {
    // Unit 033: `minimap keys --config <real config>` prints the keymap, and the output parses back to it.
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_minimap"))
        .arg("keys")
        .args(config.iter().flat_map(|c| ["--config".into(), c.clone()]))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = String::from_utf8(out.stdout).unwrap();
    let parsed = minimap_tui::keymap::Keymap::from_config(&Config::parse(&printed).unwrap().keys)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let config = Config::load(config.as_deref()).unwrap();
    let keymap = minimap_tui::keymap::Keymap::from_config(&config.keys).unwrap();
    for mode in minimap_tui::keymap::Mode::ALL {
        let set = |k: &minimap_tui::keymap::Keymap| -> std::collections::HashSet<_> {
            k.table(mode).bindings().iter().cloned().collect()
        };
        assert_eq!(set(&parsed), set(&keymap), "{mode:?}");
    }
    println!(
        "{} bindings printed",
        printed.lines().filter(|l| l.starts_with('"')).count()
    );
}

/// The trail on the target (unit 010): absolute paths as an agent sends them, drawn without moving a tile, fast.
/// Nothing binds a socket or writes to the target.
#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_trail_draws_on_the_target_without_moving_a_tile() {
    use minimap_core::events::{Event, Kind};
    let repo = std::env::var("MINIMAP_TARGET_REPO").expect("set MINIMAP_TARGET_REPO");
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let mut app = target_app();
    let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
    draw(&mut t, &mut app);
    let tiles = app.layout.tiles.clone();

    // The largest text files: one grep of three, a read, and an edit.
    let mut text: Vec<usize> = (0..app.index.files.len())
        .filter(|&f| !app.index.files[f].binary && app.file_visible(f))
        .collect();
    text.sort_by_key(|&f| std::cmp::Reverse(app.index.files[f].lines));
    let abs = |f: usize| {
        root.join(&app.index.files[f].path)
            .to_string_lossy()
            .into_owned()
    };
    let events = [
        Event::trail(Kind::Grep, vec![abs(text[0]), abs(text[1]), abs(text[2])]),
        Event::trail(Kind::Read, vec![abs(text[3])]),
        Event::Trail {
            kind: Kind::Edit,
            paths: vec![abs(text[0])],
            lines: Some((10, 20)),
            source: Some("claude-code".into()),
            session: None,
        },
    ];
    let canonical = root.canonicalize().unwrap();
    for e in events {
        let e = e
            .normalized(&root, &canonical)
            .expect("the paths are in the repo");
        app.record_event(&e);
    }
    assert_eq!(app.trail.len(), 4);
    app.select(Entry::File(text[0]), true);
    let t0 = Instant::now();
    draw(&mut t, &mut app);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("real repo trail frame: {ms:.1} ms");
    assert_eq!(app.layout.tiles, tiles);
    let screen = t.backend().to_string();
    assert!(
        screen.contains("CC edited lines 10–20"),
        "the panel's trail line"
    );
    std::fs::write(
        workspace_path(".private/captures/real-trail-200x60.txt"),
        &screen,
    )
    .unwrap();
}
