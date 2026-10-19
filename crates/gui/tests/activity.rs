//! The agent history (unit `051`): the rows, a click that selects and shows the file with no zoom change, the press that keeps its row, `r`, the wheel, and a headless frame.

mod common;

use eframe::egui::{
    self, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, epaint, vec2,
};
use minimap_core::events::{Event as AgentEvent, Kind as TrailKind};
use minimap_core::palette;
use minimap_gui::GuiApp;
use minimap_gui::activity::{
    ACTIVITY_H, EMPTY, Filter, LABEL, Order, Record, SWATCH, StripHit, fit_left, session_rgb,
    short_session, shown,
};
use minimap_gui::camera::Camera;
use minimap_gui::color::SESSION;
use minimap_gui::panel::Draw;
use minimap_gui::scene::Entry;

fn key(k: Key) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
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

fn click(app: &mut GuiApp, at: Pos2, now: f64) {
    app.on_event(&press(at, true), Some(at), now);
    app.on_event(&press(at, false), Some(at), now);
}

/// The quayside map in the capture window, at card width 200 on the root's first file.
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

/// A file deep in the tree, inside closed directories.
fn deep_file(app: &GuiApp) -> usize {
    (0..app.map.tree.files.len())
        .filter(|&f| app.map.visible(f))
        .max_by_key(|&f| app.map.tree.files[f].path.matches('/').count())
        .unwrap()
}

fn path(app: &GuiApp, f: usize) -> String {
    app.map.tree.files[f].path.clone()
}

fn event(
    kind: TrailKind,
    paths: Vec<String>,
    lines: Option<(u32, u32)>,
    session: Option<&str>,
) -> AgentEvent {
    AgentEvent::Trail {
        kind,
        paths,
        lines,
        source: Some("claude-code".into()),
        session: session.map(str::to_string),
    }
}

/// The rects of the strip's clickable rows, from the top; the filter chips are left out (unit `053`).
fn row_hits(app: &GuiApp) -> Vec<Rect> {
    app.strip()
        .unwrap()
        .hits
        .iter()
        .filter(|(_, h)| matches!(h, StripHit::Row(_)))
        .map(|&(r, _)| r)
        .collect()
}

/// The texts of the strip's rows, by row: each row's texts joined with ` | `.
fn rows(app: &GuiApp) -> Vec<String> {
    let strip = app.strip().unwrap();
    let mut by_y: Vec<(f32, Vec<String>)> = Vec::new();
    for d in &strip.draws {
        if let minimap_gui::panel::Draw::Text { at, text, .. } = d {
            match by_y.iter_mut().find(|(y, _)| (y - at.y).abs() < 0.5) {
                Some((_, v)) => v.push(text.clone()),
                None => by_y.push((at.y, vec![text.clone()])),
            }
        }
    }
    by_y.into_iter().map(|(_, v)| v.join(" | ")).collect()
}

#[test]
fn rows_list_each_path_newest_first_with_kind_time_and_session() {
    let mut app = app();
    let view = app.activity_view.expect("the strip shows at startup");
    assert_eq!(view.rect.height(), ACTIVITY_H);
    assert_eq!(view.rect.top(), app.region.bottom());
    let r = rows(&app);
    assert!(r[0].starts_with(LABEL), "{r:?}");
    assert_eq!(r[1], EMPTY);
    let fs: Vec<usize> = app.map.tree.nodes[0]
        .files
        .iter()
        .copied()
        .take(3)
        .collect();
    let session = "0f9e8d7c-1111-2222-3333-444455556666";
    app.record_event(
        &event(
            TrailKind::Read,
            vec![path(&app, fs[0])],
            None,
            Some(session),
        ),
        1.0,
    );
    app.record_event(
        &event(
            TrailKind::Grep,
            fs.iter().map(|&f| path(&app, f)).collect(),
            None,
            None,
        ),
        2.0,
    );
    app.record_event(
        &event(
            TrailKind::Edit,
            vec![path(&app, fs[1])],
            Some((40, 58)),
            Some(session),
        ),
        3.0,
    );
    assert_eq!(app.activity.len(), 5, "a grep of 3 paths gives 3 rows");
    let r = rows(&app);
    assert!(
        r[0].contains("all 5") && r[0].ends_with("1 session | newest first"),
        "{r:?}"
    );
    let clock = |s: &str| {
        let c: Vec<char> = s.chars().collect();
        c.len() == 8
            && c[2] == ':'
            && c[5] == ':'
            && c.iter().filter(|c| c.is_ascii_digit()).count() == 6
    };
    let want = [
        ("edit", path(&app, fs[1]), "0f9e8d7c"),
        ("search", path(&app, fs[2]), "—"),
        ("search", path(&app, fs[1]), "—"),
        ("search", path(&app, fs[0]), "—"),
        ("read", path(&app, fs[0]), "0f9e8d7c"),
    ];
    for (k, (kind, p, s)) in want.iter().enumerate() {
        let parts: Vec<&str> = r[k + 1].split(" | ").collect();
        assert!(clock(parts[0]), "{parts:?}");
        assert_eq!(parts[1], *kind, "{parts:?}");
        assert!(parts.contains(s), "{parts:?}");
        assert!(parts.contains(&p.as_str()), "{parts:?}");
    }
    assert!(r[1].contains("40–58"), "{:?}", r[1]);
    // `x` clears the trail and the glows, and keeps the history.
    app.on_event(&key(Key::X), None, 4.0);
    assert!(app.trail.is_empty());
    assert_eq!(app.activity.len(), 5);
    assert_eq!(short_session(None), "—");
    assert_eq!(fit_left("src/quayside/berth.clj", 10), "…berth.clj");
}

#[test]
fn a_click_selects_the_file_opens_the_tree_and_shows_it_without_a_zoom() {
    let mut app = app();
    let f = deep_file(&app);
    assert!(
        !app.tree_rows().iter().any(|r| r.entry == Entry::File(f)),
        "inside closed directories"
    );
    app.record_event(
        &event(TrailKind::Read, vec![path(&app, f)], None, None),
        1.0,
    );
    app.record_event(
        &event(TrailKind::Read, vec!["gone/deleted.clj".into()], None, None),
        1.1,
    );
    let hits = row_hits(&app);
    assert_eq!(hits.len(), 1, "the unknown path's row does not click");
    let row = hits[0];
    // The unknown path's row is the newest, the first row; a click on it does nothing.
    let first = app.activity_view.unwrap().rows.min + vec2(40.0, 10.0);
    click(&mut app, first, 2.0);
    assert_eq!(app.selection, None);
    let before = app.motion.target();
    click(&mut app, row.center(), 3.0);
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert!(
        app.tree_rows().iter().any(|r| r.entry == Entry::File(f)),
        "the tree opened to the file"
    );
    // As a tree click: the map moves only as far as needed to show the card, and the zoom stays.
    assert_eq!(app.motion.target().zoom, before.zoom, "no zoom change");
    assert_eq!(
        app.motion.target(),
        app.camera_showing(before, app.map.card(f)),
        "the map shows the card"
    );
    assert_ne!(app.motion.target(), before, "the deep file was off screen");
    // The panel shows the file.
    let name = app.map.tree.files[f].name.clone();
    assert!(
        app.panel()
            .unwrap()
            .texts()
            .iter()
            .any(|t| t.contains(&name))
    );
    // The row of the selected file is marked. The active filter chip in the head row is `CURSOR` too (unit `053`).
    let rows_top = app.activity_view.unwrap().rows.top();
    let fills = app
        .strip()
        .unwrap()
        .draws
        .iter()
        .filter(|d| matches!(d, minimap_gui::panel::Draw::Fill { color, rect } if *color == minimap_core::palette::CURSOR && rect.top() >= rows_top))
        .count();
    assert_eq!(fills, 1);
    // A second click within the double-click time zooms to the card, as on a tree row.
    click(&mut app, row.center(), 3.2);
    assert_eq!(app.motion.target(), app.camera_for(Entry::File(f)));
}

#[test]
fn a_press_keeps_its_row_when_a_new_event_arrives() {
    let mut app = app();
    let fs: Vec<usize> = app.map.tree.nodes[0]
        .files
        .iter()
        .copied()
        .take(2)
        .collect();
    app.record_event(
        &event(TrailKind::Read, vec![path(&app, fs[0])], None, None),
        1.0,
    );
    let at = row_hits(&app)[0].center();
    app.on_event(&press(at, true), Some(at), 2.0);
    // A new event takes the first row; the pressed row moves down.
    app.record_event(
        &event(TrailKind::Read, vec![path(&app, fs[1])], None, None),
        2.1,
    );
    app.on_event(&press(at, false), Some(at), 2.2);
    assert_eq!(app.selection, Some(Entry::File(fs[0])));
}

#[test]
fn r_hides_and_shows_the_strip_and_the_map_keeps_its_place() {
    let mut app = app();
    let f = app.map.tree.nodes[0].files[0];
    let place = |app: &GuiApp| {
        let r = app.cam().rect_to_screen(app.region, app.map.card(f));
        [r.left(), r.top(), r.right(), r.bottom()].map(|v| (v * 100.0).round() / 100.0)
    };
    let (before, region) = (place(&app), app.region);
    app.on_event(&key(Key::R), None, 1.0);
    assert!(app.activity_view.is_none());
    assert_eq!(app.region.bottom(), region.bottom() + ACTIVITY_H);
    assert_eq!(place(&app), before, "the card keeps its screen place");
    app.on_event(&key(Key::R), None, 2.0);
    assert!(app.activity_view.is_some());
    assert_eq!(app.region, region);
    assert_eq!(place(&app), before);
}

#[test]
fn the_wheel_scrolls_the_rows_and_never_moves_the_map() {
    let mut app = app();
    let f = app.map.tree.nodes[0].files[0];
    for k in 0..20 {
        app.record_event(
            &event(TrailKind::Read, vec![path(&app, f)], None, None),
            1.0 + k as f64,
        );
    }
    let cam = app.cam();
    let at = app.activity_view.unwrap().rows.center();
    for unit in [MouseWheelUnit::Line, MouseWheelUnit::Point] {
        let wheel = Event::MouseWheel {
            unit,
            delta: vec2(0.0, -3.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        };
        app.on_event(&wheel, Some(at), 30.0);
    }
    assert_eq!(
        app.cam(),
        cam,
        "the wheel over the strip does not zoom or pan"
    );
    assert_eq!(app.activity_top, 9, "3 notches of 3 rows, then 3 points");
    assert!(
        rows(&app)[0].contains("9 newer above · "),
        "{:?}",
        rows(&app)[0]
    );
    // A new event keeps the scrolled rows in place.
    app.record_event(
        &event(TrailKind::Read, vec![path(&app, f)], None, None),
        40.0,
    );
    assert_eq!(app.activity_top, 10);
    // The scroll stops at the last full page.
    for _ in 0..10 {
        let wheel = Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: vec2(0.0, -3.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        };
        app.on_event(&wheel, Some(at), 41.0);
    }
    assert_eq!(
        app.activity_top,
        21 - app.activity_view.unwrap().visible_rows()
    );
}

#[test]
fn a_headless_frame_draws_the_strip_and_a_row_hovers_its_card() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let mut app = app();
    let f = deep_file(&app);
    app.record_event(
        &event(
            TrailKind::Edit,
            vec![path(&app, f)],
            Some((3, 4)),
            Some("abcdef0123"),
        ),
        0.5,
    );
    let row = row_hits(&app)[0];
    let frame = |app: &mut GuiApp, time: f64, pointer: Option<Pos2>| -> Vec<String> {
        let mut events = Vec::new();
        if let Some(p) = pointer {
            events.push(Event::PointerMoved(p));
        }
        let input = egui::RawInput {
            screen_rect: Some(common::screen()),
            time: Some(time),
            max_texture_side: Some(8192),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
        out.textures_delta.clear();
        let texts = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                epaint::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
        texts
    };
    let texts = frame(&mut app, 1.0, None);
    assert!(texts.iter().any(|t| t == LABEL), "{texts:?}");
    assert!(texts.iter().any(|t| t == "abcdef01"));
    assert!(texts.iter().any(|t| t == "edit"));
    frame(&mut app, 1.1, Some(row.center()));
    assert_eq!(app.hover, Some(Entry::File(f)), "the row hovers its card");
    let empty = Rect::NOTHING;
    assert!(!empty.is_positive());
}

fn note(path: &str, text: &str, session: Option<&str>) -> AgentEvent {
    AgentEvent::Note {
        path: path.into(),
        lines: None,
        text: text.into(),
        source: None,
        session: session.map(str::to_string),
    }
}

fn seqs(rows: Vec<&Record>) -> Vec<u64> {
    rows.iter().map(|r| r.seq).collect()
}

#[test]
fn the_sort_chip_shows_the_oldest_rows_first() {
    // Unit 057: `shown` takes the order, for every filter; the chip at the head row's right toggles it.
    let mut app = app();
    let fs: Vec<usize> = app.map.tree.nodes[0]
        .files
        .iter()
        .copied()
        .take(3)
        .collect();
    let ps: Vec<String> = fs.iter().map(|&f| path(&app, f)).collect();
    app.record_event(
        &event(TrailKind::Read, vec![ps[0].clone()], None, None),
        1.0,
    );
    app.record_event(&note(&ps[1], "One.", None), 1.1);
    app.record_event(
        &event(TrailKind::Edit, vec![ps[2].clone()], None, None),
        1.2,
    );
    app.record_event(&note(&ps[0], "Two.", None), 1.3);
    assert_eq!(
        seqs(shown(&app.activity, Filter::All, Order::OldestFirst)),
        [0, 1, 2, 3]
    );
    assert_eq!(
        seqs(shown(&app.activity, Filter::All, Order::NewestFirst)),
        [3, 2, 1, 0]
    );
    assert_eq!(
        seqs(shown(&app.activity, Filter::Notes, Order::OldestFirst)),
        [1, 3]
    );
    assert_eq!(
        seqs(shown(&app.activity, Filter::Notes, Order::NewestFirst)),
        [3, 1]
    );
    // The chip reads `newest first` at the head row's right, and a click toggles it.
    let r = rows(&app);
    assert!(r[0].ends_with("newest first"), "{r:?}");
    assert!(r[1].contains("Two."), "{r:?}");
    let strip = app.strip().unwrap();
    let chip = strip.rect_of(StripHit::Order).unwrap();
    let view = app.activity_view.unwrap();
    assert!(chip.right() <= view.rect.right() + 1e-3);
    click(&mut app, chip.center(), 2.0);
    assert_eq!(app.activity_order, Order::OldestFirst);
    let r = rows(&app);
    assert!(r[0].ends_with("oldest first"), "{r:?}");
    assert!(r[1].contains("read") && r[4].contains("Two."), "{r:?}");
    // The notes filter sorts too.
    app.set_filter(Filter::Notes);
    let r = rows(&app);
    assert!(r[1].contains("One.") && r[2].contains("Two."), "{r:?}");
    // A hovered chip draws as a hovered filter chip.
    app.activity_hover = Some(StripHit::Order);
    let strip = app.strip().unwrap();
    assert!(strip.draws.iter().any(
        |d| matches!(d, Draw::Fill { rect, color } if *rect == chip && *color == palette::TILE_HI)
    ));
    click(&mut app, chip.center(), 3.0);
    assert_eq!(app.activity_order, Order::NewestFirst);
}

#[test]
fn a_scrolled_strip_keeps_its_rows_in_either_order() {
    // Unit 057: oldest first, new rows append at the end; an order change keeps the top row when it can.
    let mut app = app();
    let f = app.map.tree.nodes[0].files[0];
    let p = path(&app, f);
    for k in 0..30 {
        app.record_event(
            &event(TrailKind::Read, vec![p.clone()], None, None),
            k as f64,
        );
    }
    let vis = app.activity_view.unwrap().visible_rows();
    // Newest first, scrolled 10 rows: the top row is row 19. Oldest first, it is the 20th row, at index 19.
    app.activity_top = 10;
    let top = |app: &GuiApp| {
        shown(&app.activity, app.activity_filter, app.activity_order)[app.activity_top].seq
    };
    assert_eq!(top(&app), 19);
    app.set_order(Order::OldestFirst);
    assert_eq!((app.activity_top, top(&app)), (19, 19));
    assert!(
        rows(&app)[0].contains("19 older above · "),
        "{:?}",
        rows(&app)[0]
    );
    // A new row appends at the end and does not move the scrolled strip.
    app.record_event(&event(TrailKind::Read, vec![p.clone()], None, None), 40.0);
    assert_eq!((app.activity_top, top(&app)), (19, 19));
    // Back to newest first, the top row stays the top row.
    app.set_order(Order::NewestFirst);
    assert_eq!(top(&app), 19);
    assert_eq!(app.activity_top, 30 - 19);
    // A top row that cannot be the top in the new order, near its end: the strip shows the new order's first rows.
    app.activity_top = 1;
    app.set_order(Order::OldestFirst);
    assert!(30 - 1 > 31 - vis);
    assert_eq!(app.activity_top, 0);
    // An unscrolled strip shows the first rows of the new order.
    app.set_order(Order::NewestFirst);
    assert_eq!(app.activity_top, 0);
    assert_eq!(top(&app), 30);
}

#[test]
fn each_session_has_a_color_and_the_head_row_shows_a_swatch_per_session() {
    // Unit 057: the session text says who; the kind's color and the glow say what happened.
    let mut app = app();
    let f = app.map.tree.nodes[0].files[0];
    let p = path(&app, f);
    let ids: Vec<String> = (0..9)
        .map(|k| format!("{k}{k}{k}{k}{k}{k}{k}{k}-1111-2222"))
        .collect();
    let swatches = |app: &GuiApp| -> Vec<u32> {
        app.strip()
            .unwrap()
            .draws
            .iter()
            .filter_map(|d| match d {
                Draw::Fill { rect, color } if (rect.width() - SWATCH).abs() < 1e-3 => Some(*color),
                _ => None,
            })
            .collect()
    };
    let color_of = |app: &GuiApp, text: &str| -> Option<u32> {
        app.strip().unwrap().draws.iter().find_map(|d| match d {
            Draw::Text { text: t, color, .. } if t == text => Some(*color),
            _ => None,
        })
    };
    app.record_event(
        &event(TrailKind::Read, vec![p.clone()], None, Some(&ids[0])),
        1.0,
    );
    assert!(swatches(&app).is_empty(), "one session has no swatch");
    assert_eq!(color_of(&app, "00000000"), Some(SESSION[0]));
    app.record_event(
        &event(TrailKind::Edit, vec![p.clone()], None, Some(&ids[1])),
        1.1,
    );
    assert_eq!(swatches(&app), [SESSION[0], SESSION[1]]);
    assert_ne!(SESSION[0], SESSION[1], "two sessions get two colors");
    assert_eq!(color_of(&app, "11111111"), Some(SESSION[1]));
    // The edit's kind and its glow keep the trail's edit color.
    assert_eq!(color_of(&app, "edit"), Some(palette::EDIT));
    assert_eq!(app.glows.last().unwrap().color, palette::EDIT);
    // A row without a session is `DIM`.
    app.record_event(&event(TrailKind::Read, vec![p.clone()], None, None), 1.2);
    assert_eq!(color_of(&app, "—"), Some(palette::DIM));
    for (k, id) in ids.iter().enumerate().skip(2) {
        app.record_event(
            &event(TrailKind::Read, vec![p.clone()], None, Some(id)),
            2.0 + k as f64,
        );
    }
    // The ninth session repeats the first color; the head row shows eight swatches and `…`.
    assert_eq!(session_rgb(&app.sessions, Some(&ids[8])), SESSION[0]);
    assert_eq!(color_of(&app, "88888888"), Some(SESSION[0]));
    assert_eq!(swatches(&app), SESSION);
    let head = &rows(&app)[0];
    assert!(head.contains("9 sessions | … | newest first"), "{head}");
    assert_eq!(session_rgb(&app.sessions, None), palette::DIM);
}

#[test]
fn the_session_palette_reads_on_side_and_stays_off_the_state_colors() {
    let hue = |c: u32| {
        let (r, g, b) = palette::rgb(c);
        let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let h = if max == r {
            ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        h * 60.0
    };
    let apart = |a: f32, b: f32| (a - b).abs().min(360.0 - (a - b).abs());
    let lum = |c: u32| {
        let (r, g, b) = palette::rgb(c);
        let ch = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
    };
    for &s in &SESSION {
        for other in [
            palette::FOCUS,
            palette::READ,
            palette::GREP,
            palette::EDIT,
            palette::CURSOR,
        ] {
            assert!(
                apart(hue(s), hue(other)) >= 20.0,
                "{s:06x} near {other:06x}"
            );
        }
        let contrast = (lum(s) + 0.05) / (lum(palette::SIDE) + 0.05);
        assert!(contrast >= 4.5, "{s:06x} on SIDE: {contrast:.1}");
    }
    for (k, &a) in SESSION.iter().enumerate() {
        for &b in &SESSION[k + 1..] {
            assert!(apart(hue(a), hue(b)) >= 20.0, "{a:06x} near {b:06x}");
        }
    }
}
