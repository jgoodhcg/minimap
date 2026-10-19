//! The selection panel in the window (unit `044`): its content, the git states, the column, and stability.

mod common;

use std::collections::HashMap;

use eframe::egui::{
    self, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, pos2, vec2,
};
use minimap_core::details::{Details, Loader};
use minimap_core::history::{self, History};
use minimap_gui::GuiApp;
use minimap_gui::app::SHOW_MARGIN;
use minimap_gui::camera::Camera;
use minimap_gui::panel::{
    DIR_HINT, Draw, EMPTY, FILE_HINT, GitHistory, NONE_HINT, OUTLINE_LABEL, PANEL_MAX, PANEL_MIN,
    PANEL_W,
};
use minimap_gui::scene::Entry;
use minimap_gui::tree::{ROW_H, TREE_W};

/// 2026-01-01 00:00 UTC: the fixture's ages count back from it.
const NOW: i64 = 1_767_225_600;
/// A Clojure file with a namespace, 17 outline items, and 3 recent commits in `quayside-details.json`.
const BERTH: &str = "components/berth/src/quayside/berth/state.clj";

/// The quayside map in the capture window's screen, with every file's details and the fixture's git history.
fn app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    let text = common::fixture("quayside-details.json");
    let mut loader = Loader::with_workers(Details::from_json(&text).unwrap(), || {}, 0);
    loader.request(app.map.tree.files.iter().map(|f| f.path.clone()));
    loader.poll();
    app.loader = Some(loader);
    app.git = GitHistory::Ready(History::from_json(&text, NOW).unwrap());
    app
}

fn file(app: &GuiApp, path: &str) -> usize {
    match app.map.entry(path) {
        Some(Entry::File(f)) => f,
        e => panic!("{path}: {e:?}"),
    }
}

/// The panel's texts from the top.
fn texts(app: &GuiApp) -> Vec<String> {
    app.panel()
        .unwrap()
        .texts()
        .into_iter()
        .map(String::from)
        .collect()
}

/// The index of each text in order, so the test checks that they come in this order from the top.
fn assert_in_order(texts: &[String], want: &[&str]) {
    let mut from = 0;
    for w in want {
        let at = texts[from..]
            .iter()
            .position(|t| t == w)
            .unwrap_or_else(|| panic!("{w:?} after {:?} in {texts:#?}", texts.get(from)));
        from += at + 1;
    }
}

/// The vertical center of a text item.
fn y_of(app: &GuiApp, s: &str) -> f32 {
    match app.panel().unwrap().find(s) {
        Some(Draw::Text { at, .. }) => at.y,
        _ => panic!("no {s:?}"),
    }
}

fn press(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn key(k: Key) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn the_panel_shows_at_startup_and_the_map_region_ends_at_its_edge() {
    let app = app();
    let v = app.panel_view.unwrap();
    assert!(app.panel_on);
    assert_eq!(v.rect.width(), PANEL_W);
    assert_eq!(v.rect.right(), 1600.0);
    assert_eq!(app.region.right(), 1600.0 - PANEL_W);
    assert_eq!(app.region.left(), TREE_W);
    // The panel runs between the bars, and its last row is the hint. The agent history (unit 051) is below the map region.
    assert_eq!(v.rect.top(), app.region.top());
    assert_eq!(v.rect.bottom(), app.activity_view.unwrap().rect.bottom());
    assert_eq!(v.hint.height(), ROW_H);
}

#[test]
fn a_files_panel_reads_its_facts_outline_and_git_in_the_tuis_order() {
    let mut app = app();
    let f = file(&app, BERTH);
    app.select(Some(Entry::File(f)), false, 0.0);
    // A window 1,200 points high, so the whole outline fits below the shape (unit 045).
    app.set_screen(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1200.0)));
    let t = texts(&app);
    let stats = format!(
        "2,585 files · {} dirs · {} lines",
        minimap_core::text::thousands(app.map.tree.nodes.len() as u64),
        minimap_core::text::kilo(app.total_lines)
    );
    assert_in_order(
        &t,
        &[
            "quayside",
            &stats,
            "state.clj",
            "components/berth/src/quayside/berth/",
            "quayside.berth.state",
            "276 lines",
            OUTLINE_LABEL,
            "17",
            "  10  late?",
            " 223  upsert-lane",
            " 257  daily-berth-counts",
            "GIT",
            "9 commits",
            "changed 6mo ago · created 5y ago",
            "  6mo",
            "Handle missing draft in berth sync",
            "   2y",
            "Split berth fee namespace",
            "   5y",
            "Allow bulk berth event updates",
            FILE_HINT,
        ],
    );
    // The icon comes before the name, in the file's type color.
    let p = app.panel().unwrap();
    let name_at = t.iter().position(|s| s == "state.clj").unwrap();
    let glyph = minimap_core::filetype::for_name("state.clj")
        .glyph
        .to_string();
    assert_eq!(t[name_at - 1], glyph);
    // Private items are faint, public ones dim.
    let color = |s: &str| match p.find(s) {
        Some(Draw::Text { color, .. }) => *color,
        _ => panic!("{s}"),
    };
    assert_eq!(color(" 223  upsert-lane"), minimap_core::palette::FAINT);
    assert_eq!(color("  10  late?"), minimap_core::palette::DIM);
    // The git group ends right above the hint row, and the outline ends at least a row above the group.
    let v = app.panel_view.unwrap();
    let last_commit = y_of(&app, "Allow bulk berth event updates");
    assert!((last_commit - (v.hint.top() - ROW_H / 2.0)).abs() < 1e-3);
    assert!((y_of(&app, FILE_HINT) - v.hint.center().y).abs() < 1e-3);
    assert!(y_of(&app, "GIT") - y_of(&app, " 257  daily-berth-counts") >= 2.0 * ROW_H - 1e-3);
    // Every text is inside the panel, right of its edge.
    for d in &p.draws {
        if let Draw::Text { at, .. } = d {
            assert!(v.rect.contains(*at) && at.x > v.rect.left(), "{d:?}");
        }
    }
}

#[test]
fn a_short_panel_cuts_the_outline_with_a_more_row() {
    let mut app = app();
    let f = file(&app, BERTH);
    app.select(Some(Entry::File(f)), false, 0.0);
    // 600 points less the bars and the hint row: 26 whole rows. The facts take 7, so 19 are free; the git group takes 5 and its gap 1.
    // The shape takes 40 % of the other 13 rows, 5, plus its label and a blank row (unit 045),
    // so the outline has 6: its label, 4 items, and the more row.
    app.set_screen(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 600.0)));
    let t = texts(&app);
    let is_item = |s: &&String| {
        let c: Vec<char> = s.chars().collect();
        c.len() > 6
            && c[..4]
                .iter()
                .collect::<String>()
                .trim()
                .parse::<u32>()
                .is_ok()
            && c[4..6] == [' ', ' ']
    };
    let shown = t.iter().filter(is_item).count();
    assert_eq!(shown, 4, "{t:#?}");
    assert_in_order(
        &t,
        &[
            "SHAPE",
            OUTLINE_LABEL,
            "17",
            "  10  late?",
            "  … 13 more",
            "GIT",
        ],
    );
}

#[test]
fn a_directorys_panel_reads_its_counts_lines_and_largest_files() {
    let mut app = app();
    let Some(Entry::Dir(n)) = app.map.entry("components") else {
        panic!("components");
    };
    app.select(Some(Entry::Dir(n)), false, 0.0);
    let t = texts(&app);
    let count = app.filetree.count(n, true);
    let kids = app.map.tree.nodes[n].kids.len();
    let under: Vec<usize> = (0..app.map.tree.files.len())
        .filter(|&f| app.map.tree.files[f].path.starts_with("components/") && app.map.visible(f))
        .collect();
    let lines: u64 = under.iter().map(|&f| app.map.index.files[f].lines).sum();
    assert_in_order(
        &t,
        &[
            "components/",
            &format!(
                "{} files · {kids} subdirectories",
                minimap_core::text::thousands(count as u64)
            ),
            &format!("{} lines", minimap_core::text::kilo(lines)),
            "LARGEST FILES",
            "lines",
        ],
    );
    // The largest files, most lines first, as paths below the directory with their counts.
    let mut want = under.clone();
    want.retain(|&f| !app.map.index.files[f].binary);
    want.sort_by_key(|&f| std::cmp::Reverse(app.map.index.files[f].lines));
    // 360 points are 46 cells: 44 of text, less the icon's 2 and the count's 8, leave 34 for the path.
    let path =
        |f: usize| minimap_core::text::fit(&app.map.tree.files[f].path["components/".len()..], 34);
    let (first, second) = (&path(want[0]), &path(want[1]));
    assert_in_order(
        &t,
        &[
            "LARGEST FILES",
            first,
            &app.map.index.files[want[0]].lines.to_string(),
            second,
        ],
    );
    assert_eq!(t.last().unwrap(), DIR_HINT);
    // The root shows the repo name.
    app.select(Some(Entry::Dir(0)), false, 0.0);
    assert_eq!(texts(&app)[2], "quayside/");
}

#[test]
fn no_selection_a_binary_and_a_generated_file() {
    let mut app = app();
    let t = texts(&app);
    assert_in_order(&t, &[EMPTY, NONE_HINT]);
    let bin = (0..app.map.index.files.len())
        .find(|&f| app.map.index.files[f].binary)
        .unwrap();
    app.select(Some(Entry::File(bin)), false, 0.0);
    assert!(texts(&app).contains(&"binary".to_string()));
    let lock = file(&app, "package-lock.json");
    app.select(Some(Entry::File(lock)), false, 0.0);
    assert!(texts(&app).contains(&"12,487 lines · generated".to_string()));
}

#[test]
fn the_git_group_while_loading_after_a_failure_without_commits_and_without_a_source() {
    let mut app = app();
    let f = file(&app, BERTH);
    app.select(Some(Entry::File(f)), false, 0.0);
    let (tx, loader) = history::Loader::channel();
    app.git = GitHistory::Loading(loader);
    assert_in_order(
        &texts(&app),
        &["GIT", "loading", "Reading the git history…"],
    );
    assert!(app.panel_loading(), "the capture waits for the walk");
    // The walk's end shows the history.
    tx.send((
        History::from_json(&common::fixture("quayside-details.json"), NOW),
        std::time::Duration::ZERO,
    ))
    .unwrap();
    app.poll_history();
    assert!(matches!(app.git, GitHistory::Ready(_)));
    assert!(!app.panel_loading());
    assert!(texts(&app).contains(&"9 commits".to_string()));
    app.git = GitHistory::Failed("bad object".to_string());
    assert!(texts(&app).contains(&"Git history failed: bad object".to_string()));
    app.git = GitHistory::Ready(History::from_json(r#"{"files": []}"#, NOW).unwrap());
    assert_in_order(&texts(&app), &["GIT", "0 commits", "No commits yet."]);
    app.git = GitHistory::None;
    let t = texts(&app);
    assert!(!t.contains(&"GIT".to_string()), "{t:#?}");
    // Without the group, the whole outline fits.
    assert!(t.contains(&" 257  daily-berth-counts".to_string()));
}

/// The screen points of three world points: the map content's place on the screen.
fn content(app: &GuiApp) -> [Pos2; 3] {
    let w = app.map.world();
    [pos2(0.3, 0.3), pos2(0.5, 0.5), pos2(0.6, 0.4)]
        .map(|k| app.cam().to_screen(app.region, pos2(k.x * w.x, k.y * w.y)))
}

fn assert_in_place(a: [Pos2; 3], b: [Pos2; 3], what: &str) {
    for (p, q) in a.iter().zip(&b) {
        assert!(p.distance(*q) < 1e-2, "{what}: {p:?} to {q:?}");
    }
}

#[test]
fn the_panel_never_moves_a_card_and_keeps_the_map_content_in_place() {
    let mut app = app();
    let cards: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    let cam = Camera {
        center: (app.map.world() / 2.0).to_pos2(),
        zoom: 80.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    let start = content(&app);
    app.on_event(&key(Key::P), None, 1.0);
    assert!(!app.panel_on && app.panel_view.is_none());
    assert_eq!(app.region.right(), 1600.0);
    assert_in_place(start, content(&app), "p hides");
    // A key repeat does nothing.
    let repeat = Event::Key {
        key: Key::P,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: Modifiers::NONE,
    };
    app.on_event(&repeat, None, 1.05);
    assert!(!app.panel_on);
    app.on_event(&key(Key::P), None, 1.1);
    assert_eq!(app.region.right(), 1600.0 - PANEL_W);
    assert_in_place(start, content(&app), "p shows");
    // A drag on the edge sets the width, within its limits; the edge is at the left, so a drag to the left widens it.
    let edge = app.panel_view.unwrap().edge.center();
    app.on_event(&press(edge, true), Some(edge), 2.0);
    for x in [edge.x - 100.0, edge.x - 2000.0, 1600.0, edge.x - 60.0] {
        let p = pos2(x, edge.y);
        app.on_event(&Event::PointerMoved(p), Some(p), 2.1);
        let w = app.panel_width();
        assert!((PANEL_MIN..=PANEL_MAX).contains(&w), "{w}");
        assert_in_place(start, content(&app), "an edge drag");
    }
    let end = pos2(edge.x - 60.0, edge.y);
    app.on_event(&press(end, false), Some(end), 2.2);
    assert_eq!(app.panel_width(), PANEL_W + 60.0);
    assert_eq!(app.region.right(), 1600.0 - PANEL_W - 60.0);
    assert_eq!(app.selection, None, "a drag on the edge selects nothing");
    let after: Vec<Rect> = (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect();
    assert_eq!(cards, after, "no card moved in the world");
}

#[test]
fn a_narrow_window_narrows_the_panel_first_then_the_tree() {
    let mut app = app();
    let widths = |app: &mut GuiApp, w: f32| {
        app.set_screen(Rect::from_min_size(Pos2::ZERO, vec2(w, 600.0)));
        (app.tree_width(), app.panel_width(), app.region.width())
    };
    assert_eq!(widths(&mut app, 1000.0), (280.0, 360.0, 360.0));
    assert_eq!(widths(&mut app, 800.0), (280.0, 280.0, 240.0));
    assert_eq!(widths(&mut app, 700.0), (220.0, 240.0, 240.0));
    // Below both least widths, the map gets what is left.
    assert_eq!(widths(&mut app, 500.0), (160.0, 240.0, 100.0));
    // The set widths stay, so a wide window gives them back.
    assert_eq!(widths(&mut app, 1600.0), (280.0, 360.0, 960.0));
}

#[test]
fn a_press_on_the_panel_selects_nothing_and_its_wheel_never_moves_the_map() {
    let mut app = app();
    let f = file(&app, BERTH);
    app.select(Some(Entry::File(f)), false, 0.0);
    let cam = app.cam();
    let at = app.panel_view.unwrap().body.center();
    app.on_event(&press(at, true), Some(at), 1.0);
    let moved = at + vec2(-200.0, 50.0);
    app.on_event(&Event::PointerMoved(moved), Some(moved), 1.05);
    app.on_event(&press(moved, false), Some(moved), 1.1);
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert_eq!(
        app.cam(),
        cam,
        "a drag that starts on the panel does not pan"
    );
    for unit in [MouseWheelUnit::Line, MouseWheelUnit::Point] {
        let wheel = Event::MouseWheel {
            unit,
            delta: vec2(0.0, -3.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        };
        app.on_event(&wheel, Some(at), 1.2);
    }
    assert_eq!(
        app.cam(),
        cam,
        "the wheel over the panel does not zoom or pan"
    );
}

#[test]
fn the_selected_file_is_requested_first_at_every_zoom() {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    let texts: HashMap<String, String> = app
        .map
        .tree
        .files
        .iter()
        .map(|f| (f.path.clone(), "x\n".to_string()))
        .collect();
    // No workers, so the requests wait in the queue in their order.
    app.loader = Some(Loader::with_workers(Details::memory(texts), || {}, 0));
    let f = file(&app, BERTH);
    app.select(Some(Entry::File(f)), false, 0.0);
    // The fit view is at the block level: only the selection is asked for.
    app.update_details();
    assert_eq!(app.loader.as_ref().unwrap().pending(), [BERTH]);
    // At card width 200 in another district, the selection comes first, then the visible cards nearest to the center.
    let other = file(&app, "bases/api/src/quayside/api/main.clj");
    let cam = Camera {
        center: app.map.card(other).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app.update_details();
    let pending = app.loader.as_ref().unwrap().pending();
    assert_eq!(pending[0], BERTH);
    assert!(pending.len() > 1);
    assert_eq!(pending.iter().filter(|p| *p == BERTH).count(), 1);
}

#[test]
fn a_tree_selection_shows_the_card_inside_the_narrower_region() {
    let mut app = app();
    let f = file(&app, BERTH);
    // Put the card under the panel's place on the screen, at card width 200.
    let zoom = 200.0 / app.map.card_w();
    let card = app.map.card(f);
    let under_panel = pos2(1600.0 - PANEL_W / 2.0, app.region.center().y);
    let cam = Camera {
        center: card.center() - (under_panel - app.region.center()) / zoom,
        zoom,
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app.select(Some(Entry::File(f)), true, 1.0);
    app.motion.tick(20.0);
    let r = app.cam().rect_to_screen(app.region, card);
    let inner = app.region.shrink(SHOW_MARGIN - 1e-2);
    assert!(inner.contains_rect(r), "{r:?} in {inner:?}");
    assert!(r.right() <= app.panel_view.unwrap().rect.left() - SHOW_MARGIN + 0.1);
}

/// One headless frame of the whole window (`GuiApp::frame`), with these events at a time in seconds.
fn frame(ctx: &egui::Context, app: &mut GuiApp, events: Vec<Event>, time: f64) -> egui::FullOutput {
    let input = egui::RawInput {
        screen_rect: Some(common::screen()),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
    // No renderer takes the font atlas's texture deltas.
    out.textures_delta.clear();
    out
}

#[test]
fn headless_frames_paint_the_panel_and_drag_its_edge() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = app();
    let f = file(&app, BERTH);
    app.select(Some(Entry::File(f)), false, 0.0);
    let out = frame(&ctx, &mut app, vec![], 0.0);
    assert_eq!(
        app.region.right(),
        1600.0 - PANEL_W,
        "the frame laid out the panel"
    );
    // The panel's texts paint: the frame has more text than without the panel.
    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    assert!(!prims.is_empty());
    // Over the edge, the cursor is a resize cursor; a drag sets the width.
    let edge = app.panel_view.unwrap().edge.center();
    let out = frame(&ctx, &mut app, vec![Event::PointerMoved(edge)], 1.0);
    assert_eq!(
        out.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    assert_eq!(app.hover, None, "the edge hovers no card");
    let to = edge - vec2(100.0, 0.0);
    let _ = frame(&ctx, &mut app, vec![press(edge, true)], 1.1);
    let out = frame(&ctx, &mut app, vec![Event::PointerMoved(to)], 1.2);
    assert_eq!(
        out.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    let out = frame(&ctx, &mut app, vec![press(to, false)], 1.3);
    assert_eq!(app.panel_width(), PANEL_W + 100.0);
    assert_eq!(app.region.right(), 1600.0 - PANEL_W - 100.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    assert!(!prims.is_empty());
    // `p` in a frame hides the panel.
    let _ = frame(&ctx, &mut app, vec![key(Key::P)], 2.0);
    assert!(app.panel_view.is_none());
    assert_eq!(app.region.right(), 1600.0);
}
