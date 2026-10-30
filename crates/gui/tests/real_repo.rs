//! Real-repo tests for the GUI. They read `MINIMAP_TARGET_REPO` and `MINIMAP_CONFIG`, and write nothing.
//!
//! ```sh
//! MINIMAP_TARGET_REPO=<path> MINIMAP_CONFIG=.private/minimap.toml cargo test --release -p minimap-gui -- --ignored
//! ```

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use minimap_core::config::expand_home;
use minimap_core::details::{Details, Loader};
use minimap_core::index::repo_root;
use minimap_core::{Config, Index, IndexOptions};
use minimap_gui::camera::Camera;
use minimap_gui::scene::{self, Map};

fn workspace_path(p: &str) -> PathBuf {
    let p = expand_home(p);
    if p.is_absolute() {
        p
    } else {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(p)
    }
}

fn target() -> (PathBuf, Index) {
    let repo = std::env::var("MINIMAP_TARGET_REPO").expect("set MINIMAP_TARGET_REPO");
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let excludes = Config::load(config.as_deref()).unwrap().excludes_for(&root);
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    (root, index)
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn visible_cards_at_width_200_get_details_within_1_s() {
    let (root, index) = target();
    let map = Map::new(index);
    let region = common::region();
    // The card nearest the map center: line-scaled cards (unit 039) leave the center itself empty on the target.
    let mid = (map.world() / 2.0).to_pos2();
    let f = (0..map.tree.files.len())
        .min_by(|&a, &b| {
            let d = |f: usize| map.card(f).distance_sq_to_pos(mid);
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    let cam = Camera {
        center: map.card(f).center(),
        zoom: 200.0 / map.card_w(),
    };
    let cards = scene::visible_cards(&map, &cam, region);
    assert!(!cards.is_empty(), "the map center shows cards");
    let paths: Vec<String> = cards
        .iter()
        .map(|&f| map.tree.files[f].path.clone())
        .collect();
    let t0 = Instant::now();
    let mut loader = Loader::spawn(Details::repo(root), || {});
    loader.request(paths.clone());
    let mut got = 0;
    while got < paths.len() {
        got += loader.poll().len();
        assert!(
            t0.elapsed() < Duration::from_secs(10),
            "{got} of {}",
            paths.len()
        );
        std::thread::sleep(Duration::from_micros(200));
    }
    let took = t0.elapsed();
    let lines: u64 = cards.iter().map(|&f| map.index.files[f].lines).sum();
    eprintln!(
        "{} visible cards at width 200 ({lines} lines): details in {:.0} ms",
        cards.len(),
        took.as_secs_f64() * 1000.0
    );
    assert!(took < Duration::from_secs(1), "{took:?}");
    // The scene with every visible shape: one bar per line in view (unit 039).
    let t0 = Instant::now();
    let items = scene::scene(&map, &cam, region, &scene::Options::default(), &loader);
    let bars: usize = items
        .iter()
        .map(|i| match &i.body {
            scene::Body::Bars(b) => b.len(),
            _ => 0,
        })
        .sum();
    eprintln!(
        "scene at width 200: {} items, {bars} shape bars, {:.2} ms",
        items.len(),
        t0.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn directory_names_at_the_fit_view_and_at_card_width_33() {
    // Units 040 and 062: names sized by the count of files below their directories, drawn as text from 7 points and as bars from 2.
    let (_, index) = target();
    let map = Map::new(index);
    let region = common::region();
    let b = minimap_gui::camera::Bounds { map: map.world() };
    let mid = (map.world() / 2.0).to_pos2();
    let f = (0..map.tree.files.len())
        .min_by(|&a, &c| {
            let d = |f: usize| map.card(f).distance_sq_to_pos(mid);
            d(a).total_cmp(&d(c))
        })
        .unwrap();
    let w33 = b.clamp(
        region,
        Camera {
            center: map.card(f).center(),
            zoom: 33.0 / map.card_w(),
        },
    );
    for (label, cam) in [("fit view", b.fit(region)), ("card width 33", w33)] {
        let items = scene::scene(
            &map,
            &cam,
            region,
            &scene::Options::default(),
            &scene::NoDetails,
        );
        let names: Vec<_> = items
            .iter()
            .filter(|i| i.kind == scene::Kind::Header)
            .collect();
        let text = names
            .iter()
            .filter(|i| matches!(i.body, scene::Body::Str(_)))
            .count();
        let bars = names
            .iter()
            .filter(|i| i.body == scene::Body::Greek)
            .count();
        let districts = |body: fn(&scene::Body) -> bool| {
            names
                .iter()
                .filter(|i| body(&i.body))
                .filter(|i| matches!(map.entry(i.path), Some(scene::Entry::Dir(n)) if map.tree.nodes[n].depth == 1))
                .count()
        };
        let in_view = map
            .layout
            .headers
            .iter()
            .enumerate()
            .filter(|(h, _)| cam.rect_to_screen(region, map.band(*h)).intersects(region))
            .count();
        eprintln!(
            "{label} (zoom {:.4}): {in_view} bands in view; {text} names as text ({} districts), {bars} as bars ({} districts)",
            cam.zoom,
            districts(|b| matches!(b, scene::Body::Str(_))),
            districts(|b| *b == scene::Body::Greek),
        );
        assert!(text > 0, "{label}");
    }
}

/// One file selection for the probe: the first file by path with this many lines, or the longest hand-written file.
fn probe_file(map: &Map, lines: Option<u64>) -> usize {
    let files = &map.index.files;
    match lines {
        Some(n) => (0..files.len())
            .filter(|&f| files[f].lines == n && !files[f].binary)
            .min_by(|&a, &b| files[a].path.cmp(&files[b].path))
            .expect("a file with that line count"),
        None => (0..files.len())
            .filter(|&f| !files[f].generated)
            .max_by_key(|&f| files[f].lines)
            .unwrap(),
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn headless_frame_probe_along_the_capture_sweep() {
    // The frame CPU time with no window (unit 041; the G2 probe): each whole frame (`GuiApp::frame`: the layout, the details,
    // the bars, the file tree of unit 042, the selection panel of unit 044 with the git history, and the map) and its tessellation,
    // along the capture's zoom sweep from the fit view to card width 400, twice. Run it in a release build.
    use eframe::egui::{self, Rect, pos2, vec2};
    use minimap_gui::camera::between;
    if cfg!(debug_assertions) {
        eprintln!("the frame probe runs only in a release build: cargo test --release");
        return;
    }
    let (root, index) = target();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let font = Config::load(config.as_deref()).unwrap().gui_font();
    let probe_map = Map::new(index.clone());
    // A short file and the longest hand-written file, as the G2 captures selected them.
    let picks = [
        ("27-line file", probe_file(&probe_map, Some(27))),
        ("longest file", probe_file(&probe_map, None)),
    ];
    let paths: Vec<String> = probe_map
        .tree
        .files
        .iter()
        .map(|f| f.path.clone())
        .collect();
    let mut loader = Loader::spawn(Details::repo(root.clone()), || {});
    loader.request(paths.clone());
    let t0 = Instant::now();
    let mut got = 0;
    while got < paths.len() && t0.elapsed() < Duration::from_secs(120) {
        got += loader.poll().len();
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!(
        "details of {got} of {} files in {:.1} s",
        paths.len(),
        t0.elapsed().as_secs_f64()
    );
    // The panel's git group: the history, walked once.
    let t0 = Instant::now();
    let history = minimap_core::History::walk(&root, &paths, minimap_core::history::now()).unwrap();
    eprintln!("git history in {:.2} s", t0.elapsed().as_secs_f64());
    let mut loader = Some(loader);
    for (label, f) in picks {
        for ppp in [1.0, 2.0] {
            let ctx = egui::Context::default();
            let faces = minimap_gui::fonts::find(font.clone()).unwrap();
            minimap_gui::fonts::install(&ctx, faces.as_ref()).unwrap();
            ctx.set_pixels_per_point(ppp);
            let mut app = minimap_gui::GuiApp::new(Map::new(index.clone()));
            app.select_at_start(scene::Entry::File(f));
            app.loader = loader.take();
            app.git = minimap_gui::panel::GitHistory::Ready(history.clone());
            // A trail of 50 marks (unit 047): every 50th file, read, then the selection, edited.
            for p in paths.iter().step_by(paths.len() / 50).take(50) {
                app.record_event(
                    &minimap_core::events::Event::trail(
                        minimap_core::events::Kind::Read,
                        vec![p.clone()],
                    ),
                    0.0,
                );
            }
            let sel = app.map.tree.files[f].path.clone();
            app.record_event(
                &minimap_core::events::Event::trail(minimap_core::events::Kind::Edit, vec![sel]),
                0.0,
            );
            // 30 notes (unit 053): marks at the block level and badges near the selection, and the strip filtered to them.
            for (k, p) in paths
                .iter()
                .skip(7)
                .step_by(paths.len() / 30)
                .take(30)
                .enumerate()
            {
                app.record_event(
                    &minimap_core::events::Event::Note {
                        path: p.clone(),
                        lines: Some((1, 3)),
                        text: format!("Stop {k}: this file matters for the review."),
                        source: None,
                        session: None,
                    },
                    0.0,
                );
            }
            app.set_filter(minimap_gui::activity::Filter::Notes);
            // The window's wgpu texture limit: its atlas was 8,192 texels wide in the G2 captures. egui's default without a window is 2,048.
            let input = || egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 1000.0))),
                max_texture_side: Some(8192),
                ..Default::default()
            };
            // Two frames first, so the context has its pixels per point and the app its layout.
            for _ in 0..2 {
                let mut out = ctx.run_ui(input(), |ui| app.frame(ui, None));
                out.textures_delta.clear();
            }
            assert!(app.panel_view.is_some(), "the frames include the panel");
            // The agent history below the map (unit 051), with a row per event above.
            assert!(
                app.strip().is_some_and(|s| !s.hits.is_empty()),
                "the frames include the agent history"
            );
            // The frames include the mark of a picked outline item, when the file has one (unit 046).
            if !app.outline().is_empty() {
                app.pick = Some(0);
            }
            let region = app.region;
            let from = app.bounds.fit(region);
            let to = app.card_camera(f, 400.0 / app.map.card_w());
            let mut atlas = Vec::new();
            for pass in 0..2 {
                // 50 running glows (unit 049), restarted at each pass, so every frame of the sweep draws them:
                // halos at the block level, rings near the selection, and edge markers for the cards off screen.
                for p in paths.iter().step_by(paths.len() / 50).take(50) {
                    app.record_event(
                        &minimap_core::events::Event::trail(
                            minimap_core::events::Kind::Read,
                            vec![p.clone()],
                        ),
                        app.now,
                    );
                }
                assert!(app.glowing(), "the sweep draws running glows");
                let mut times = Vec::new();
                for k in 0..120 {
                    let cam = app
                        .bounds
                        .clamp(region, between(from, to, k as f32 / 119.0));
                    app.motion.jump(cam);
                    let t1 = Instant::now();
                    let mut out = ctx.run_ui(input(), |ui| app.frame(ui, None));
                    out.textures_delta.clear();
                    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
                    std::hint::black_box(prims.len());
                    times.push(t1.elapsed().as_secs_f64() * 1000.0);
                }
                times.sort_by(f64::total_cmp);
                let (size, fill) = ctx.fonts(|f| (f.font_image_size(), f.font_atlas_fill_ratio()));
                let p95 = times[(119.0 * 0.95f64).round() as usize];
                eprintln!(
                    "{label}, {ppp} px/pt, pass {}: p50 {:.2} ms, p95 {p95:.2} ms, max {:.2} ms; atlas {} x {}, {:.1} %",
                    pass + 1,
                    times[60],
                    times[119],
                    size[0],
                    size[1],
                    fill * 100.0
                );
                assert!(p95 < 8.0, "{label} pass {}: p95 {p95:.2} ms", pass + 1);
                atlas.push(size);
            }
            assert_eq!(
                atlas[0], atlas[1],
                "{label}: the atlas grew on the second sweep"
            );
            loader = app.loader.take();
        }
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_file_tree_on_the_target() {
    // Unit 042: the tree's rows at startup and with every directory open, and the time to build them and to paint the tree.
    use eframe::egui;
    let (_, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    let start = app.tree_rows().len();
    for n in 1..app.map.tree.nodes.len() {
        app.set_open(n, true);
    }
    let t0 = Instant::now();
    let rows = app.tree_rows();
    let build = t0.elapsed().as_secs_f64() * 1000.0;
    let dirs = rows
        .iter()
        .filter(|r| matches!(r.entry, scene::Entry::Dir(_)))
        .count();
    let deepest = rows.iter().map(|r| r.depth).max().unwrap_or(0);
    assert_eq!(rows.len() - dirs, app.filetree.count(0, true));
    // The tree's paint, with every directory open, scrolled to the middle.
    app.filetree.top = rows.len() / 2;
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut times = Vec::new();
    for _ in 0..20 {
        let t1 = Instant::now();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(common::screen()),
                ..Default::default()
            },
            |ui| {
                let look = minimap_gui::tree::Look {
                    icons: true,
                    char_w: 0.6,
                    selected: None,
                    hovered: None,
                    dragging: false,
                    trail: true,
                    notes: true,
                };
                let v = app.tree_view.unwrap();
                minimap_gui::tree::paint(ui.painter(), &v, &app.map, &app.filetree, &look);
            },
        );
        out.textures_delta.clear();
        times.push(t1.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "tree: {start} rows at startup; {} rows with every directory open ({dirs} directories, depth up to {deepest}), built in {build:.2} ms; \
         paint p50 {:.2} ms",
        rows.len(),
        times[10]
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_selection_panel_on_the_target() {
    // Unit 044: the panel's rows for the longest hand-written file and for the root directory, the history walk,
    // and the time to build the panel's items. It prints counts only.
    let (root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    let f = probe_file(&app.map, None);
    let path = app.map.tree.files[f].path.clone();
    let mut loader = Loader::spawn(Details::repo(root.clone()), || {});
    loader.request([path.clone()]);
    let t0 = Instant::now();
    while loader.poll().is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(1));
    }
    app.loader = Some(loader);
    let paths: Vec<String> = app.map.index.files.iter().map(|f| f.path.clone()).collect();
    let t0 = Instant::now();
    let walk = minimap_core::history::Loader::spawn(root, paths, minimap_core::history::now());
    let history = loop {
        if let Some((h, took)) = walk.poll() {
            eprintln!(
                "history walk: {:.2} s on its thread, {:.2} s to the poll",
                took.as_secs_f64(),
                t0.elapsed().as_secs_f64()
            );
            break h.unwrap();
        }
        assert!(t0.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(5));
    };
    let fh = history.file(&path).cloned();
    app.git = minimap_gui::panel::GitHistory::Ready(history);
    let time = |app: &minimap_gui::GuiApp| {
        let mut times: Vec<f64> = (0..50)
            .map(|_| {
                let t1 = Instant::now();
                std::hint::black_box(app.panel());
                t1.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        times[25]
    };
    app.select(Some(scene::Entry::File(f)), false, 0.0);
    let p = app.panel().unwrap();
    let outline = app.file_details(f).map_or(0, |d| d.outline.len());
    let rows = p
        .texts()
        .iter()
        .filter(|t| {
            t.len() > 6
                && t.chars()
                    .take(4)
                    .collect::<String>()
                    .trim()
                    .parse::<u32>()
                    .is_ok()
        })
        .count();
    assert!(p.texts().contains(&"GIT"), "the git group shows");
    // The shape (unit 045): its box, bars, and definition ticks.
    let fills = |color: u32| {
        p.draws
            .iter()
            .filter_map(|d| match d {
                minimap_gui::panel::Draw::Fill { rect, color: c } if *c == color => Some(*rect),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let shape_box = fills(minimap_core::palette::MINI_BG);
    assert_eq!(shape_box.len(), 1, "the shape shows");
    let ticks = fills(minimap_core::palette::FAINT)
        .iter()
        .filter(|r| r.left() > shape_box[0].right())
        .count();
    assert_eq!(ticks, outline);
    eprintln!(
        "longest file's shape: a box of {:.0} x {:.0} points, {} bars, {ticks} ticks",
        shape_box[0].width(),
        shape_box[0].height(),
        fills(minimap_core::palette::MINI_DOT).len()
    );
    eprintln!(
        "longest file ({} lines): {outline} outline items, {rows} rows of them shown; {} commits; {} texts; built in {:.3} ms (p50)",
        app.map.index.files[f].lines,
        fh.map_or(0, |h| h.count),
        p.texts().len(),
        time(&app)
    );
    app.select(Some(scene::Entry::Dir(0)), false, 0.0);
    let p = app.panel().unwrap();
    assert!(p.texts().contains(&"LARGEST FILES"));
    eprintln!(
        "root directory: {} texts; built in {:.3} ms (p50)",
        p.texts().len(),
        time(&app)
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn an_outline_pick_on_the_target() {
    // Unit 046: a pick of the longest hand-written file's last outline item puts its line 24 points below the region's top.
    use minimap_gui::app::{ZOOM_TO_TOP, ZOOM_TO_WIDTH};
    let (root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    let f = probe_file(&app.map, None);
    let mut loader = Loader::spawn(Details::repo(root), || {});
    loader.request([app.map.tree.files[f].path.clone()]);
    let t0 = Instant::now();
    while loader.poll().is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(1));
    }
    app.loader = Some(loader);
    app.select(Some(scene::Entry::File(f)), false, 0.0);
    let k = app.outline().len() - 1;
    let line = app.outline()[k].line;
    let r = app
        .panel()
        .unwrap()
        .rect_of(minimap_gui::panel::Hit::Outline(k))
        .expect("the last item's row");
    for (at, pressed) in [(r.center(), true), (r.center(), false)] {
        let ev = eframe::egui::Event::PointerButton {
            pos: at,
            button: eframe::egui::PointerButton::Primary,
            pressed,
            modifiers: eframe::egui::Modifiers::NONE,
        };
        app.on_event(&ev, Some(at), 1.0);
    }
    assert_eq!(app.pick, Some(k));
    app.motion.tick(20.0);
    let cam = app.cam();
    let y = cam
        .to_screen(
            app.region,
            eframe::egui::pos2(0.0, scene::line_top(&app.map, f, line)),
        )
        .y;
    eprintln!(
        "pick of item {} of {} (line {line} of {}): zoom {:.3}, the line at {:.1} points below the region's top",
        k + 1,
        k + 1,
        app.map.index.files[f].lines,
        cam.zoom,
        y - app.region.top()
    );
    assert_eq!(cam.zoom, ZOOM_TO_WIDTH / app.map.card_w());
    assert!((y - (app.region.top() + ZOOM_TO_TOP)).abs() < 0.5, "{y}");
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn a_hook_payload_reaches_the_windows_trail_on_the_target() {
    // Unit 047: a Read payload piped into the installed `minimap hook claude-code` reaches the window's listener, and its card is tinted.
    // It needs `minimap` on PATH (unit 011) and no other minimap on the target.
    use std::io::Write;
    use std::process::{Command, Stdio};
    let (root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.listener = Some(
        minimap_core::events::Listener::spawn(&root)
            .expect("no other minimap listens on the target"),
    );
    let f = probe_file(&app.map, None);
    let abs = root.join(&app.map.tree.files[f].path);
    let payload = format!(
        r#"{{"hook_event_name":"PostToolUse","tool_name":"Read","tool_input":{{"file_path":{}}},"cwd":{}}}"#,
        serde_json_string(&abs.to_string_lossy()),
        serde_json_string(&root.to_string_lossy())
    );
    let t0 = Instant::now();
    let mut hook = Command::new("minimap")
        .args(["hook", "claude-code"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("minimap on PATH");
    hook.stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    assert!(hook.wait().unwrap().success());
    while !app.poll_events(1.0) {
        assert!(t0.elapsed() < Duration::from_secs(2), "no event within 2 s");
        std::thread::sleep(Duration::from_millis(2));
    }
    eprintln!(
        "hook to window trail: {:.1} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(
        app.map.tint[f],
        Some((minimap_core::events::Kind::Read, true))
    );
}

/// A JSON string literal.
fn serde_json_string(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_live_index_on_the_target() {
    // Unit 048: the watcher starts on the target, an idle wait gives no update, and a re-index rebuilds the map in the window's thread.
    let (root, index) = target();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let excludes = Config::load(config.as_deref()).unwrap().excludes_for(&root);
    let options = IndexOptions {
        excludes,
        ..Default::default()
    };
    let w = minimap_core::watch::Watcher::spawn(root.clone(), options, index.clone()).unwrap();
    std::thread::sleep(Duration::from_secs(3));
    let (update, error) = w.poll();
    eprintln!(
        "idle 3 s: {} update, {} error",
        if update.is_some() { "an" } else { "no" },
        if error.is_some() { "an" } else { "no" }
    );
    assert!(error.is_none());
    let mut app = minimap_gui::GuiApp::new(Map::new(index.clone()));
    app.set_screen(common::screen());
    let f = probe_file(&app.map, None);
    app.select(Some(scene::Entry::File(f)), false, 0.0);
    let mut times = Vec::new();
    for _ in 0..5 {
        let u = minimap_core::watch::Update {
            index: index.clone(),
            changed: vec![app.map.tree.files[f].path.clone()],
            head_moved: false,
        };
        let t0 = Instant::now();
        app.apply_index(u, 1.0);
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "apply_index on the target: median {:.1} ms, max {:.1} ms",
        times[2], times[4]
    );
    assert_eq!(app.selection, Some(scene::Entry::File(f)));
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn notes_on_the_target_map() {
    // Unit 053: 30 notes on target files. At the fit view, one block mark per visible noted card; at card width 200 near one, its badge.
    // No card moves. It prints counts only, no paths.
    use minimap_core::events::Event;
    use minimap_gui::scene::Kind;
    let (_root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    let before: Vec<_> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    let noted: Vec<usize> = (0..app.map.tree.files.len())
        .filter(|&f| app.map.visible(f))
        .step_by(app.map.tree.files.len() / 30)
        .take(30)
        .collect();
    for (k, &f) in noted.iter().enumerate() {
        let path = app.map.tree.files[f].path.clone();
        app.record_event(
            &Event::Note {
                path,
                lines: None,
                text: format!("stop {k}"),
                source: None,
                session: None,
            },
            k as f64,
        );
    }
    let items = app.items();
    let marks = items.iter().filter(|i| i.kind == Kind::NoteMark).count();
    let in_region = noted
        .iter()
        .filter(|&&f| {
            app.motion
                .cam
                .rect_to_screen(app.region, app.map.card(f))
                .expand(minimap_gui::scene::NOTE_MIN)
                .intersects(app.region)
        })
        .count();
    assert_eq!(marks, in_region);
    assert!(
        marks >= 25,
        "most noted cards show at the fit view: {marks}"
    );
    let f = noted[10];
    let cam = Camera {
        center: app.map.card(f).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    let items = app.items();
    let path = app.map.tree.files[f].path.as_str();
    assert!(
        items
            .iter()
            .any(|i| i.kind == Kind::NoteNumber && i.path == path),
        "the badge has its number at card width 200"
    );
    let after: Vec<_> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    assert_eq!(before, after);
    println!("{marks} note marks at the fit view");
}

/// The 20 largest hand-written text files of the target, and a 1-line file.
fn reader_picks(map: &Map) -> Vec<usize> {
    let files = &map.index.files;
    let mut text: Vec<usize> = (0..files.len())
        .filter(|&f| !files[f].binary && !files[f].generated && map.visible(f))
        .collect();
    text.sort_by_key(|&f| std::cmp::Reverse(files[f].lines));
    let mut picks: Vec<usize> = text.iter().copied().take(20).collect();
    picks.extend(text.iter().copied().find(|&f| files[f].lines == 1));
    picks
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_reader_on_the_target() {
    // Unit 054: notes on the 20 largest target text files and a 1-line file, read in both modes. It prints counts and times only, no paths.
    use eframe::egui::{Key, Modifiers};
    use minimap_core::events::Event;
    use minimap_gui::reader::{Row, Text};
    let (root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    app.content = minimap_core::Content::Repo(root);
    let picks = reader_picks(&app.map);
    assert_eq!(picks.len(), 21, "the target has a 1-line file");
    for (k, &f) in picks.iter().enumerate() {
        let lines = app.map.index.files[f].lines as u32;
        app.record_event(
            &Event::Note {
                path: app.map.tree.files[f].path.clone(),
                lines: Some((lines / 2 + 1, (lines / 2 + 4).min(lines.max(1)))),
                text: format!("Stop {k}: read the middle of this file."),
                source: None,
                session: None,
            },
            k as f64,
        );
    }
    let key = |k: Key| eframe::egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    for (k, &f) in picks.iter().enumerate() {
        app.on_event(&key(Key::N), None, 100.0 + k as f64);
        let r = app.reader.as_ref().unwrap();
        assert_eq!(r.note, k as u64 + 1);
        let Text::Lines(lines) = &r.text else {
            panic!("note {k}: {:?}", r.text)
        };
        assert_eq!(lines.len() as u64, app.map.index.files[f].lines, "note {k}");
        for whole in [false, true] {
            app.set_reader_whole(whole);
            let page = app.reader_page().unwrap();
            assert_eq!(page.rows[page.current], Row::Label { note: k as u64 + 1 });
            let want = format!("NOTE {} of 21", k + 1);
            assert!(page.texts().contains(&want.as_str()), "note {k}");
        }
        app.set_reader_whole(false);
    }
    // The build time of the largest file in `whole file` mode.
    app.on_event(&key(Key::G), None, 200.0);
    app.step_note(false, 200.0);
    while app.reader.as_ref().unwrap().note != 1 {
        app.step_note(false, 200.0);
    }
    app.set_reader_whole(true);
    let mut times = Vec::new();
    for _ in 0..50 {
        let t0 = Instant::now();
        std::hint::black_box(app.reader_page());
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let p95 = times[47];
    println!(
        "largest file, {} lines: reader build p50 {:.2} ms, p95 {p95:.2} ms",
        app.map.index.files[picks[0]].lines, times[25]
    );
    if !cfg!(debug_assertions) {
        assert!(p95 < 4.0, "p95 {p95:.2} ms");
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn reader_frame_probe_on_the_target() {
    // Unit 054: whole frames with the reader on the largest target file in `whole file` mode, scrolling 120 frames, twice. Release only.
    use eframe::egui::{self, Rect, pos2, vec2};
    use minimap_core::events::Event;
    if cfg!(debug_assertions) {
        eprintln!("the frame probe runs only in a release build: cargo test --release");
        return;
    }
    let (root, index) = target();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let font = Config::load(config.as_deref()).unwrap().gui_font();
    let ctx = egui::Context::default();
    let faces = minimap_gui::fonts::find(font).unwrap();
    minimap_gui::fonts::install(&ctx, faces.as_ref()).unwrap();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.content = minimap_core::Content::Repo(root);
    let f = reader_picks(&app.map)[0];
    let lines = app.map.index.files[f].lines as u32;
    app.record_event(
        &Event::Note {
            path: app.map.tree.files[f].path.clone(),
            lines: Some((lines / 2, lines / 2 + 20)),
            text: "A long note. ".repeat(40),
            source: None,
            session: None,
        },
        0.0,
    );
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 1000.0))),
        max_texture_side: Some(8192),
        ..Default::default()
    };
    for _ in 0..2 {
        let mut out = ctx.run_ui(input(), |ui| app.frame(ui, None));
        out.textures_delta.clear();
    }
    // The frames draw the syntax colors (unit 055): wait for them first.
    app.colorer = Some(minimap_gui::reader::Colorer::spawn(None, || {}));
    app.open_note(1, 0.0);
    app.set_reader_whole(true);
    let t0 = Instant::now();
    while app.reader.as_ref().unwrap().colors.is_none() {
        app.poll_colors(0.0);
        assert!(t0.elapsed() < Duration::from_secs(60), "no colors");
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!("colors in {:.2} s", t0.elapsed().as_secs_f64());
    let mut atlas = Vec::new();
    for pass in 0..2 {
        let start = app.reader.as_ref().unwrap().top;
        let mut times = Vec::new();
        for k in 0..120 {
            app.reader.as_mut().unwrap().top = start.saturating_sub(60) + k;
            let t1 = Instant::now();
            let mut out = ctx.run_ui(input(), |ui| app.frame(ui, None));
            out.textures_delta.clear();
            let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
            std::hint::black_box(prims.len());
            times.push(t1.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        let p95 = times[(119.0 * 0.95f64).round() as usize];
        let size = ctx.fonts(|f| f.font_image_size());
        eprintln!(
            "reader, {lines} lines, pass {}: p50 {:.2} ms, p95 {p95:.2} ms, max {:.2} ms; atlas {} x {}",
            pass + 1,
            times[60],
            times[119],
            size[0],
            size[1]
        );
        assert!(p95 < 8.0, "pass {}: p95 {p95:.2} ms", pass + 1);
        atlas.push(size);
    }
    assert_eq!(atlas[0], atlas[1], "the atlas grew on the second pass");
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn reader_colors_on_the_target() {
    // Unit 055: the reader colors the target's largest Clojure and ClojureScript files. It writes the times to `.private/`, without paths.
    use minimap_core::events::Event;
    let (root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    app.content = minimap_core::Content::Repo(root);
    app.colorer = Some(minimap_gui::reader::Colorer::spawn(None, || {}));
    let mut report = String::from("# Reader colors on the target (unit 055)\n\n");
    for ext in [".clj", ".cljs"] {
        let files = &app.map.index.files;
        let f = (0..files.len())
            .filter(|&f| files[f].path.ends_with(ext) && !files[f].generated)
            .max_by_key(|&f| files[f].lines)
            .unwrap();
        let lines = files[f].lines;
        app.record_event(
            &Event::Note {
                path: files[f].path.clone(),
                lines: Some((1, 3)),
                text: "Colors.".into(),
                source: None,
                session: None,
            },
            0.0,
        );
        let id = app.notes.all().last().unwrap().id;
        let t0 = Instant::now();
        app.open_note(id, 0.0);
        while app.reader.as_ref().unwrap().colors.is_none() {
            app.poll_colors(0.0);
            assert!(t0.elapsed() < Duration::from_secs(60), "{ext}: no colors");
            std::thread::sleep(Duration::from_millis(2));
        }
        let took = t0.elapsed().as_secs_f64() * 1000.0;
        let colors = app.reader.as_ref().unwrap().colors.clone().unwrap();
        assert_eq!(colors.len() as u64, lines, "{ext}: one span list per line");
        let page = app.reader_page().unwrap();
        let distinct: std::collections::HashSet<u32> = page
            .draws
            .iter()
            .filter_map(|d| match d {
                minimap_gui::panel::Draw::Text { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert!(distinct.len() >= 5, "{ext}: {} colors", distinct.len());
        let line = format!(
            "- largest `{ext}` file: {lines} lines, colors in {took:.0} ms ({})\n",
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        );
        print!("{line}");
        report.push_str(&line);
    }
    let out = workspace_path(".private/reader-colors-055.md");
    std::fs::write(&out, report).unwrap();
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn usage_fixes_on_the_target() {
    // Unit 057 on the 20 largest target text files and a 1-line file: a note row click selects and a second click reads,
    // the history sorts oldest first at 5,000 rows, sessions get colors, and a selection of every line copies the file's text.
    // It prints counts and times only, no paths.
    use minimap_core::events::{Event, Kind};
    use minimap_gui::activity::{self, Filter, Order};
    use minimap_gui::reader::{Row, Selection, Spot};
    use minimap_gui::scene::Entry;
    let (root, index) = target();
    let mut app = minimap_gui::GuiApp::new(Map::new(index));
    app.set_screen(common::screen());
    app.content = minimap_core::Content::Repo(root.clone());
    let picks = reader_picks(&app.map);
    let sessions = ["aaaa1111-x", "bbbb2222-x", "cccc3333-x"];
    for (k, &f) in picks.iter().enumerate() {
        let path = app.map.tree.files[f].path.clone();
        app.record_event(
            &Event::Trail {
                kind: Kind::Read,
                paths: vec![path.clone()],
                lines: None,
                source: None,
                session: Some(sessions[k % 3].into()),
            },
            k as f64,
        );
        app.record_event(
            &Event::Note {
                path,
                lines: None,
                text: format!("Stop {k}: the whole file."),
                source: None,
                session: Some(sessions[k % 3].into()),
            },
            k as f64 + 0.5,
        );
    }
    // Session colors: three sessions, three swatches, and each row's session text in its color.
    let strip = app.strip().unwrap();
    let swatches = strip
        .draws
        .iter()
        .filter(|d| matches!(d, minimap_gui::panel::Draw::Fill { rect, .. } if (rect.width() - activity::SWATCH).abs() < 1e-3))
        .count();
    assert_eq!(swatches, 3);
    for (k, s) in sessions.iter().enumerate() {
        assert_eq!(
            activity::session_rgb(&app.sessions, Some(s)),
            minimap_gui::color::SESSION[k]
        );
    }
    // A note row: the first click selects its file and does not read; the second reads.
    let mut clicks = 0;
    let note_rows: Vec<(u64, u64, String)> = app
        .activity
        .iter()
        .filter_map(|r| Some((r.seq, r.note?, r.path.clone())))
        .collect();
    for (k, (seq, id, path)) in note_rows.iter().enumerate() {
        let now = 100.0 + 2.0 * k as f64;
        app.click_activity(*seq, now);
        let f = activity::file_of(&app.map, path).expect("a target file has a card");
        assert_eq!(app.selection, Some(Entry::File(f)));
        assert!(
            app.reader.is_none(),
            "note {id}: the first click does not read"
        );
        app.click_activity(*seq, now + 1.0);
        assert_eq!(app.reader.as_ref().map(|r| r.note), Some(*id));
        clicks += 1;
    }
    // A selection of every line copies the file's text: whole lines, with their tabs and their parts past the screen.
    app.set_reader_whole(true);
    let (mut tabbed, mut cut, mut copy_ms) = (0, 0, Vec::new());
    let cols =
        (app.region.width() / (app.opts.char_w * 13.0)) as usize - minimap_gui::reader::GUTTER - 2;
    for &(_, id, ref path) in &note_rows {
        app.open_note(id, 200.0);
        let page = app.reader_page().unwrap();
        let first = page
            .rows
            .iter()
            .position(|r| matches!(r, Row::Line { .. }))
            .unwrap();
        let last = page.rows.len() - 1;
        let Row::Line { n, .. } = page.rows[last] else {
            panic!("the last row is a line")
        };
        let disk = std::fs::read_to_string(root.join(path)).unwrap();
        let want: Vec<&str> = disk.lines().collect();
        assert_eq!(n + 1, want.len());
        app.reader_sel = Some(Selection {
            anchor: Spot { row: first, col: 0 },
            head: Spot {
                row: last,
                col: minimap_gui::reader::width(want[n]),
            },
        });
        let t0 = Instant::now();
        let got = app.selected_text().unwrap();
        copy_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
        assert!(
            got == want.join("\n"),
            "note {id}: the copy differs from the file"
        );
        tabbed += usize::from(want.iter().any(|l| l.contains('\t')));
        cut += want
            .iter()
            .filter(|l| minimap_gui::reader::width(l) > cols)
            .count();
    }
    // The history oldest first, at its 5,000-row cap: arrival order, and a scrolled strip keeps its top row as the oldest rows drop.
    let p = app.map.tree.files[picks[0]].path.clone();
    for k in 0..5100 {
        app.record_event(
            &Event::trail(Kind::Read, vec![p.clone()]),
            300.0 + k as f64 * 1e-3,
        );
    }
    app.set_filter(Filter::All);
    app.set_order(Order::OldestFirst);
    let shown = activity::shown(&app.activity, Filter::All, Order::OldestFirst);
    assert!(shown.windows(2).all(|w| w[0].seq < w[1].seq));
    app.activity_top = 100;
    let top = shown[100].seq;
    app.record_event(&Event::trail(Kind::Read, vec![p.clone()]), 400.0);
    let shown = activity::shown(&app.activity, Filter::All, Order::OldestFirst);
    assert_eq!(shown[app.activity_top].seq, top, "the top row stays");
    copy_ms.sort_by(f64::total_cmp);
    println!(
        "{clicks} note rows click-select then read; {} files copy exactly ({tabbed} with tabs, {cut} lines past the screen); copy max {:.2} ms",
        note_rows.len(),
        copy_ms.last().unwrap()
    );
}
