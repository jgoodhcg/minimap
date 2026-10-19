//! The agent's notes in the window (unit `053`): the note model and its history rows, the marks at the block level and from the icon level,
//! layer 2, the glow color, the filter chips, and a headless frame.

mod common;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, epaint};
use minimap_core::events::{Event as AgentEvent, Kind as TrailKind};
use minimap_core::palette;
use minimap_gui::GuiApp;
use minimap_gui::activity::{Act, Filter, LABEL, StripHit};
use minimap_gui::camera::Camera;
use minimap_gui::color::rgb;
use minimap_gui::scene::{Body, Entry, Item, Kind, NOTE_MIN, NOTE_MIX};

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

/// The quayside map in the capture window, at the fit view.
fn fit_app() -> GuiApp {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    app
}

/// The quayside map at card width 200, centered on a file's card.
fn near_app(f: usize) -> GuiApp {
    let mut app = fit_app();
    let cam = Camera {
        center: app.map.card(f).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    app
}

fn path(app: &GuiApp, f: usize) -> String {
    app.map.tree.files[f].path.clone()
}

fn note(path: &str, lines: Option<(u32, u32)>, text: &str) -> AgentEvent {
    AgentEvent::Note {
        path: path.into(),
        lines,
        text: text.into(),
        source: Some("claude-code".into()),
        session: Some("0f9e8d7c-aaaa".into()),
    }
}

fn of_kind<'a>(items: &'a [Item<'a>], kind: Kind) -> Vec<&'a Item<'a>> {
    items.iter().filter(|i| i.kind == kind).collect()
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

fn root_files(app: &GuiApp, n: usize) -> Vec<usize> {
    app.map.tree.nodes[0]
        .files
        .iter()
        .copied()
        .take(n)
        .collect()
}

fn card_rects(app: &GuiApp) -> Vec<Rect> {
    (0..app.map.tree.files.len())
        .map(|f| app.map.card(f))
        .collect()
}

#[test]
fn a_note_adds_a_note_a_focus_glow_and_a_history_row_with_its_text() {
    let mut app = fit_app();
    let fs = root_files(&app, 2);
    let (a, b) = (path(&app, fs[0]), path(&app, fs[1]));
    app.record_event(
        &note(&a, Some((40, 58)), "\n  Retries start here.\nSecond line."),
        1.0,
    );
    app.record_event(&note(&b, None, ""), 1.5);
    assert_eq!(app.notes.len(), 2);
    assert_eq!(app.map.noted[fs[0]], Some(1));
    assert_eq!(app.map.noted[fs[1]], Some(2));
    assert!(app.trail.is_empty(), "a note is not on the trail");
    let g = app.glows.iter().find(|g| g.path == a).unwrap();
    assert_eq!((g.color, g.note), (palette::FOCUS, true));
    let r = &app.activity[0];
    assert_eq!(
        (r.kind, r.note, r.lines, r.text.as_str()),
        (Act::Note, Some(1), Some((40, 58)), "Retries start here.")
    );
    let shown = rows(&app);
    let parts: Vec<&str> = shown[2].split(" | ").collect();
    assert_eq!(parts[1], "note", "{parts:?}");
    assert!(parts.contains(&"40–58"), "{parts:?}");
    assert!(parts.contains(&"Retries start here."), "{parts:?}");
    assert!(parts.contains(&"0f9e8d7c"), "{parts:?}");
    // A clear of the notes empties them and their marks, and the history keeps its rows.
    app.record_event(&AgentEvent::ClearNotes, 2.0);
    assert!(app.notes.is_empty());
    assert!(app.map.noted.iter().all(Option::is_none));
    assert!(app.glows.iter().all(|g| !g.note));
    assert_eq!(app.activity.len(), 2);
    // A new note after the clear is number 1 on the map.
    app.record_event(&note(&b, None, "again"), 3.0);
    assert_eq!(app.map.noted[fs[1]], Some(1));
}

#[test]
fn the_fit_view_marks_each_noted_card_and_no_card_moves() {
    let mut app = fit_app();
    let before = card_rects(&app);
    let fs: Vec<usize> = (0..app.map.tree.files.len())
        .filter(|&f| app.map.visible(f))
        .step_by(97)
        .take(12)
        .collect();
    for (k, &f) in fs.iter().enumerate() {
        let p = path(&app, f);
        app.record_event(&note(&p, None, "t"), k as f64);
    }
    // Every file twice: the mark counts files, not notes.
    let p = path(&app, fs[0]);
    app.record_event(&note(&p, Some((1, 2)), "again"), 20.0);
    let items = app.items();
    let marks = of_kind(&items, Kind::NoteMark);
    assert_eq!(marks.len(), fs.len());
    for m in &marks {
        assert!(m.rect.width() >= NOTE_MIN - 1e-3 && m.rect.height() >= NOTE_MIN - 1e-3);
        assert_eq!(
            m.color,
            minimap_gui::color::mix(
                palette::FOCUS,
                palette::PANE,
                minimap_gui::scene::TRAIL_BLOCK_FRESH_MIX
            )
        );
    }
    // The marks draw after every card and header, and before the glows.
    let first_mark = items.iter().position(|i| i.kind == Kind::NoteMark).unwrap();
    assert!(
        items[..first_mark]
            .iter()
            .all(|i| !matches!(i.kind, Kind::Glow | Kind::NoteMark))
    );
    assert!(items[first_mark..].iter().all(|i| matches!(
        i.kind,
        Kind::NoteMark | Kind::Glow | Kind::GlowCore | Kind::Ripple | Kind::GlowEdge
    )));
    // A mark does not count for hits: the point hits the card under it, or nothing.
    let m = marks[0];
    let hit = app.hit(m.rect.center());
    assert!(hit.is_none() || matches!(hit, Some(Entry::File(_))));
    // `2` hides the marks and the note glows; the history keeps its rows. No card moves at any step.
    app.now = 20.5;
    assert!(app.glowing());
    app.on_event(&key(Key::Num2), None, 21.0);
    assert!(!app.opts.toggles.notes);
    let items = app.items();
    assert!(of_kind(&items, Kind::NoteMark).is_empty());
    assert!(of_kind(&items, Kind::Glow).is_empty());
    assert!(!app.glowing());
    assert_eq!(app.activity.len(), fs.len() + 1);
    assert_eq!(card_rects(&app), before);
    app.on_event(&key(Key::Num2), None, 22.0);
    assert!(app.opts.toggles.notes);
    app.record_event(&AgentEvent::ClearNotes, 23.0);
    assert_eq!(card_rects(&app), before);
}

#[test]
fn from_the_icon_level_a_noted_card_has_the_note_fill_and_a_numbered_badge() {
    let mut app = fit_app();
    let fs = root_files(&app, 3);
    let mut app2 = near_app(fs[2]);
    std::mem::swap(&mut app, &mut app2);
    let (a, c) = (path(&app, fs[0]), path(&app, fs[2]));
    app.record_event(&note(&a, None, "first"), 1.0);
    // A trail event on the noted card: the note's fill wins over the trail's tint.
    app.record_event(&AgentEvent::trail(TrailKind::Edit, vec![c.clone()]), 1.5);
    app.record_event(&note(&c, Some((2, 3)), "second"), 2.0);
    app.now = 10.0;
    let items = app.items();
    let card = items
        .iter()
        .find(|i| i.kind == Kind::Card && i.path == c)
        .unwrap();
    assert_eq!(
        card.color,
        minimap_gui::color::mix(palette::FOCUS, palette::TILE, NOTE_MIX)
    );
    let badge = items
        .iter()
        .find(|i| i.kind == Kind::NoteBadge && i.path == c)
        .unwrap();
    let screen_card = app
        .motion
        .cam
        .rect_to_screen(app.region, app.map.card(fs[2]));
    assert!((badge.rect.center() - screen_card.right_top()).length() < 1e-3);
    assert_eq!(badge.color, rgb(palette::FOCUS));
    let number = items
        .iter()
        .find(|i| i.kind == Kind::NoteNumber && i.path == c)
        .unwrap();
    assert_eq!(
        number.body,
        Body::Str("2".into()),
        "the file's first note is note 2"
    );
    assert!(number.bold);
    assert!(badge.rect.contains(number.rect.center()));
    // The hover wins over the note's fill.
    app.hover = Some(Entry::File(fs[2]));
    let items = app.items();
    let card = items
        .iter()
        .find(|i| i.kind == Kind::Card && i.path == c)
        .unwrap();
    assert_eq!(card.color, rgb(palette::TILE_HI));
    // `x` clears the trail; the note glows and marks stay.
    app.hover = None;
    app.now = 2.5;
    app.on_event(&key(Key::X), None, 2.5);
    assert!(app.trail.is_empty());
    assert!(app.glows.iter().any(|g| g.note));
    assert!(!of_kind(&app.items(), Kind::NoteBadge).is_empty());
}

#[test]
fn the_filter_chips_count_rows_and_show_one_kind() {
    let mut app = fit_app();
    let fs = root_files(&app, 3);
    let ps: Vec<String> = fs.iter().map(|&f| path(&app, f)).collect();
    app.record_event(
        &AgentEvent::trail(TrailKind::Read, vec![ps[0].clone()]),
        1.0,
    );
    app.record_event(&AgentEvent::trail(TrailKind::Grep, ps.clone()), 1.1);
    app.record_event(&note(&ps[1], Some((3, 4)), "Look here."), 1.2);
    app.record_event(
        &AgentEvent::trail(TrailKind::Edit, vec![ps[2].clone()]),
        1.3,
    );
    app.record_event(&note(&ps[2], None, "And here."), 1.4);
    let head = &rows(&app)[0];
    for chip in ["all 7", "notes 2", "edits 1", "reads 1", "searches 3"] {
        assert!(head.contains(chip), "{chip}: {head}");
    }
    assert!(head.starts_with(LABEL));
    let strip = app.strip().unwrap();
    let chip = strip.rect_of(StripHit::Filter(Filter::Notes)).unwrap();
    click(&mut app, chip.center(), 2.0);
    assert_eq!(app.activity_filter, Filter::Notes);
    let shown = rows(&app);
    assert_eq!(shown.len(), 3, "the head and 2 note rows: {shown:?}");
    assert!(shown[1].contains("And here.") && shown[2].contains("Look here."));
    // A note row's first click selects its file, as any row's does, and does not read it (unit `057`).
    let row = app
        .strip()
        .unwrap()
        .hits
        .iter()
        .find(|(_, h)| matches!(h, StripHit::Row(_)))
        .unwrap()
        .0;
    click(&mut app, row.center(), 3.0);
    assert_eq!(app.selection, Some(Entry::File(fs[2])));
    assert!(app.reader.is_none());
    // The empty texts per filter.
    let mut empty = fit_app();
    for (f, text) in [
        (
            Filter::Notes,
            "No notes yet. An agent sends them with minimap note.",
        ),
        (Filter::Edits, "No edits yet."),
        (Filter::Reads, "No reads yet."),
        (Filter::Searches, "No searches yet."),
    ] {
        empty.set_filter(f);
        assert_eq!(rows(&empty)[1], text);
    }
    empty.record_event(
        &AgentEvent::trail(TrailKind::Read, vec![ps[0].clone()]),
        1.0,
    );
    empty.set_filter(Filter::Notes);
    assert_eq!(
        rows(&empty)[1],
        "No notes yet. An agent sends them with minimap note."
    );
}

#[test]
fn a_filtered_strip_scrolls_and_a_chip_press_survives_a_new_event() {
    let mut app = fit_app();
    let f = root_files(&app, 1)[0];
    let p = path(&app, f);
    for k in 0..30 {
        app.record_event(&note(&p, Some((k + 1, k + 1)), &format!("n{k}")), k as f64);
        app.record_event(
            &AgentEvent::trail(TrailKind::Read, vec![p.clone()]),
            k as f64,
        );
    }
    app.set_filter(Filter::Notes);
    assert_eq!(app.activity_rows(), 30);
    app.activity_top = 5;
    // A new read does not shift the filtered rows; a new note does.
    app.record_event(&AgentEvent::trail(TrailKind::Read, vec![p.clone()]), 40.0);
    assert_eq!(app.activity_top, 5);
    app.record_event(&note(&p, None, "n30"), 41.0);
    assert_eq!(app.activity_top, 6);
    // A press on a chip, a new event, then the release: the chip sets the filter, from the newest row.
    let chip = app
        .strip()
        .unwrap()
        .rect_of(StripHit::Filter(Filter::Reads))
        .unwrap();
    app.on_event(&press(chip.center(), true), Some(chip.center()), 42.0);
    app.record_event(&note(&p, None, "n31"), 42.1);
    app.on_event(&press(chip.center(), false), Some(chip.center()), 42.2);
    assert_eq!(app.activity_filter, Filter::Reads);
    assert_eq!(app.activity_top, 0);
}

#[test]
fn a_narrow_strip_drops_chips_from_the_right() {
    let mut app = fit_app();
    // A narrow window: the strip is the map region's width, between the tree and the panel.
    app.set_tree(true, 160.0);
    app.set_screen(Rect::from_min_max(Pos2::ZERO, egui::pos2(900.0, 700.0)));
    let strip = app.strip().unwrap();
    let chips: Vec<Filter> = strip
        .hits
        .iter()
        .filter_map(|(_, h)| match h {
            StripHit::Filter(f) => Some(*f),
            _ => None,
        })
        .collect();
    assert!(
        !chips.is_empty() && chips.len() < Filter::ALL.len(),
        "{chips:?}"
    );
    assert_eq!(chips[..], Filter::ALL[..chips.len()]);
    let view = app.activity_view.unwrap();
    for (r, _) in &strip.hits {
        assert!(r.right() <= view.rect.right() + 1e-3);
    }
}

#[test]
fn a_headless_frame_draws_the_badges_number() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let f = fit_app().map.tree.nodes[0].files[0];
    let mut app = near_app(f);
    let p = path(&app, f);
    app.record_event(&note(&p, Some((1, 1)), "Start here."), 0.5);
    let input = egui::RawInput {
        screen_rect: Some(common::screen()),
        time: Some(10.0),
        max_texture_side: Some(8192),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
    out.textures_delta.clear();
    let texts: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            epaint::Shape::Text(t) => Some(t.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
    assert!(
        texts.iter().any(|t| t == "2 notes"),
        "the status bar's toggle"
    );
    assert!(
        texts.iter().any(|t| t == minimap_gui::app::MAP_HINT),
        "the status bar's hint"
    );
    assert!(texts.iter().any(|t| t == "Start here."), "the history row");
    let items = app.items();
    assert!(
        items
            .iter()
            .any(|i| i.kind == Kind::NoteNumber && i.body == Body::Str("1".into()))
    );
}

#[test]
fn the_map_hint_shows_both_note_keys() {
    // Unit 057: `N` shows where `n` shows; the reader's hint ends with the selection keys.
    assert!(minimap_gui::app::MAP_HINT.contains("n N notes"));
    assert!(minimap_gui::app::READER_HINT.ends_with("drag select · ⌘c copy"));
}
