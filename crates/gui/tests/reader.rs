//! The note reader (unit `054`): its rows in `lines` and `whole file` mode, the note box and its frame, the messages,
//! `n` `N` `z` and the chips, Esc, `f`, the clicks that open and close it, the scroll, the panel's NOTES section, and a headless frame.

mod common;

use std::collections::HashMap;

use eframe::egui::{
    self, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, epaint, vec2,
};
use minimap_core::Content;
use minimap_core::events::{Event as AgentEvent, Kind as TrailKind};
use minimap_core::notes::Notes;
use minimap_gui::GuiApp;
use minimap_gui::activity::StripHit;
use minimap_gui::app::{FIRST_NOTE, LAST_NOTE, NO_NOTES};
use minimap_gui::camera::Camera;
use minimap_gui::panel::Hit;
use minimap_gui::reader::{
    self, CODE_ROW, CONTEXT, Facts, NO_SOURCE, ReaderHit, ReaderView, Row, Text, expand, note_lines,
};
use minimap_gui::scene::Entry;

fn key(k: Key) -> Event {
    key_with(k, Modifiers::NONE)
}

fn key_with(k: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
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

fn note(path: &str, lines: Option<(u32, u32)>, text: &str) -> AgentEvent {
    AgentEvent::Note {
        path: path.into(),
        lines,
        text: text.into(),
        source: None,
        session: None,
    }
}

/// A text of `n` lines: `line 1`, `line 2`, …
fn numbered(n: usize) -> String {
    (1..=n).map(|k| format!("line {k}\n")).collect()
}

/// The quayside map in the capture window, with text for three root files, at card width 200 on the first.
fn app() -> (GuiApp, Vec<usize>) {
    let mut app = GuiApp::new(common::quayside());
    app.set_screen(common::screen());
    let fs: Vec<usize> = app.map.tree.nodes[0]
        .files
        .iter()
        .copied()
        .take(3)
        .collect();
    let mut files = HashMap::new();
    files.insert(app.map.tree.files[fs[0]].path.clone(), numbered(300));
    files.insert(app.map.tree.files[fs[1]].path.clone(), numbered(5));
    files.insert(
        app.map.tree.files[fs[2]].path.clone(),
        "a\u{0}b".to_string(),
    );
    app.content = Content::memory(files);
    let cam = Camera {
        center: app.map.card(fs[0]).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    (app, fs)
}

fn path(app: &GuiApp, f: usize) -> String {
    app.map.tree.files[f].path.clone()
}

fn facts<'a>(notes: &'a Notes, note: u64, whole: bool, text: &'a Text) -> Facts<'a> {
    Facts {
        notes,
        note,
        whole,
        text,
        top: 0,
        char_w: 0.6,
        hovered: None,
        colors: None,
        selection: None,
    }
}

fn view() -> ReaderView {
    ReaderView::new(common::region())
}

#[test]
fn lines_mode_shows_the_box_on_the_noted_lines_with_context() {
    let mut notes = Notes::default();
    notes.record(&note("a.clj", Some((12, 10)), "Retries start here."));
    let text = Text::of("a.clj", &numbered(30));
    let page = reader::build(&view(), &facts(&notes, 1, false, &text));
    let lines: Vec<Row> = (7..9)
        .map(|n| Row::Line { n, context: true })
        .chain([Row::Label { note: 1 }, Row::Note { note: 1, k: 0 }])
        .chain((9..12).map(|n| Row::Line { n, context: false }))
        .chain((12..14).map(|n| Row::Line { n, context: true }))
        .collect();
    assert_eq!(page.rows, lines);
    assert_eq!(page.current, CONTEXT);
    // The frame runs from the box's label row to the last noted line.
    let body = view().body;
    let (id, frame) = page.frames[0];
    assert_eq!(id, 1);
    assert!((frame.top() - (body.top() + 2.0 * CODE_ROW)).abs() < 1e-3);
    assert!((frame.bottom() - (body.top() + 7.0 * CODE_ROW)).abs() < 1e-3);
    let texts = page.texts();
    for t in [
        "NOTE 1 of 1",
        "a.clj",
        ":10–12",
        "note 1 of 1",
        "lines 10–12",
        "Retries start here.",
        "   10",
        "line 10",
        "lines",
        "whole file",
    ] {
        assert!(texts.contains(&t), "{t}: {texts:?}");
    }
    // A context line is `DIM`, and a noted line's number is `FOCUS`.
    let color_of = |s: &str| {
        page.draws.iter().find_map(|d| match d {
            minimap_gui::panel::Draw::Text { text, color, .. } if text == s => Some(*color),
            _ => None,
        })
    };
    assert_eq!(color_of("line 8"), Some(minimap_core::palette::DIM));
    assert_eq!(color_of("   10"), Some(minimap_core::palette::FOCUS));
    assert_eq!(color_of("    8"), Some(minimap_core::palette::FAINT));
}

#[test]
fn whole_file_mode_shows_every_line_with_each_note_above_its_first_line() {
    let mut notes = Notes::default();
    notes.record(&note("a.clj", Some((200, 210)), "Second stop."));
    notes.record(&note("b.clj", Some((1, 1)), "Elsewhere."));
    notes.record(&note("a.clj", Some((5, 6)), "First stop."));
    let text = Text::of("a.clj", &numbered(300));
    let v = view();
    let page = reader::build(&v, &facts(&notes, 1, true, &text));
    // 300 lines and two boxes of 2 rows each.
    assert_eq!(page.rows.len(), 304);
    assert_eq!(page.rows[4], Row::Label { note: 3 });
    assert_eq!(
        page.rows[4 + 2],
        Row::Line {
            n: 4,
            context: false
        }
    );
    assert_eq!(
        page.current,
        199 + 2,
        "note 1's box sits above line 200, after note 3's box"
    );
    assert_eq!(page.rows[page.current], Row::Label { note: 1 });
    // On entry the current box is a quarter of the body below its top.
    let top = reader::entry_top(&page, &v);
    assert_eq!(top, page.current - v.visible_rows() / 4);
    let mut f = facts(&notes, 1, true, &text);
    f.top = top;
    let page = reader::build(&v, &f);
    assert_eq!(page.top, top);
    let frame = page.frames.iter().find(|(id, _)| *id == 1).unwrap().1;
    let quarter = v.body.top() + (v.visible_rows() / 4) as f32 * CODE_ROW;
    assert!((frame.top() - quarter).abs() < 1e-3);
    assert!(page.texts().contains(&"Second stop."));
    // The other note of the file shows too, with a `FAINT` frame; the note on another file does not.
    let page0 = reader::build(&v, &facts(&notes, 1, true, &text));
    assert!(page0.frames.iter().any(|(id, _)| *id == 3));
    assert!(!page0.frames.iter().any(|(id, _)| *id == 2));
}

#[test]
fn a_note_without_lines_and_lines_past_the_end() {
    let mut notes = Notes::default();
    notes.record(&note("a.clj", None, "The whole file."));
    notes.record(&note("a.clj", Some((40, 50)), "Past the end."));
    let text = Text::of("a.clj", &numbered(5));
    for whole in [false, true] {
        let page = reader::build(&view(), &facts(&notes, 1, whole, &text));
        assert_eq!(page.rows[0], Row::Label { note: 1 }, "whole {whole}");
        assert_eq!(page.current, 0);
        assert!(page.texts().contains(&"whole file"));
    }
    let page = reader::build(&view(), &facts(&notes, 2, false, &text));
    // Lines 40–50 of 5 lines: the last line, with 2 lines of context above.
    assert_eq!(
        page.rows,
        [
            Row::Line {
                n: 2,
                context: true
            },
            Row::Line {
                n: 3,
                context: true
            },
            Row::Label { note: 2 },
            Row::Note { note: 2, k: 0 },
            Row::Line {
                n: 4,
                context: false
            },
        ]
    );
}

#[test]
fn a_long_note_wraps_and_tabs_expand() {
    let text = "First paragraph with enough words to wrap twice in a narrow box.\n\nSecond.\n\n";
    let lines = note_lines(text, 24);
    assert_eq!(
        lines,
        [
            "First paragraph with",
            "enough words to wrap",
            "twice in a narrow box.",
            "",
            "Second."
        ]
    );
    assert_eq!(expand("\tx", 80), "    x");
    assert_eq!(expand("ab\tc", 80), "ab  c");
    assert_eq!(expand("abcdef", 3), "abc");
    assert_eq!(expand("a\tb", 2), "a ");
    let mut notes = Notes::default();
    notes.record(&note("a.clj", Some((1, 1)), &"word ".repeat(400)));
    let t = Text::of("a.clj", "x\n");
    let page = reader::build(&view(), &facts(&notes, 1, false, &t));
    let note_rows = page
        .rows
        .iter()
        .filter(|r| matches!(r, Row::Note { .. }))
        .count();
    assert!(note_rows > 5, "{note_rows}");
}

#[test]
fn messages_take_the_place_of_the_text() {
    assert_eq!(
        Text::load(&Content::None, "a.clj"),
        Text::Message(NO_SOURCE.into())
    );
    assert_eq!(
        Text::of("a.png", "PNG\u{0}\u{1}"),
        Text::Message("a.png is a binary file.".into())
    );
    // A scratch directory: the GUI crate has no temp-dir crate.
    let dir = std::env::temp_dir().join(format!("minimap-reader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let Text::Message(m) = Text::load(&Content::Repo(dir.clone()), "gone.clj") else {
        panic!()
    };
    assert!(m.starts_with("Cannot read gone.clj"), "{m}");
    std::fs::write(dir.join("big.txt"), vec![b'x'; 3 * 1024 * 1024]).unwrap();
    assert_eq!(
        Text::load(&Content::Repo(dir.clone()), "big.txt"),
        Text::Message("big.txt is too large for the reader (3.0 MiB).".into())
    );
    std::fs::remove_dir_all(&dir).unwrap();
    // The box shows above the message.
    let mut notes = Notes::default();
    notes.record(&note("gone.clj", Some((3, 4)), "Deleted in this change."));
    let t = Text::Message("Cannot read gone.clj.".into());
    let page = reader::build(&view(), &facts(&notes, 1, false, &t));
    assert_eq!(
        page.rows,
        [
            Row::Label { note: 1 },
            Row::Note { note: 1, k: 0 },
            Row::Message
        ]
    );
    assert!(page.texts().contains(&"Cannot read gone.clj."));
}

#[test]
fn n_and_shift_n_step_through_the_notes_and_flash_at_the_ends() {
    let (mut app, fs) = app();
    app.on_event(&key(Key::N), None, 1.0);
    assert!(app.reader.is_none());
    assert_eq!(app.flash.as_ref().unwrap().0, NO_NOTES);
    let (a, b) = (path(&app, fs[0]), path(&app, fs[1]));
    app.record_event(&note(&a, Some((100, 104)), "One."), 1.0);
    app.record_event(&note(&b, Some((2, 3)), "Two."), 1.1);
    app.record_event(&note(&a, Some((10, 10)), "Three."), 1.2);
    // From the map, `N` opens the last note.
    app.on_event(&key_with(Key::N, Modifiers::SHIFT), None, 2.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 3);
    app.on_event(&key(Key::Escape), None, 2.1);
    assert!(app.reader.is_none());
    // `n` after the last note read: the end.
    app.on_event(&key(Key::N), None, 2.2);
    assert!(app.reader.is_none());
    assert_eq!(app.flash.as_ref().unwrap().0, LAST_NOTE);
    let mut fresh = self::app().0;
    for e in [
        note(&a, Some((100, 104)), "One."),
        note(&b, Some((2, 3)), "Two."),
    ] {
        fresh.record_event(&e, 1.0);
    }
    fresh.on_event(&key(Key::N), None, 3.0);
    assert_eq!(fresh.reader.as_ref().unwrap().note, 1);
    assert_eq!(fresh.selection, Some(Entry::File(fs[0])));
    fresh.on_event(&key(Key::N), None, 3.1);
    let r = fresh.reader.as_ref().unwrap();
    assert_eq!((r.note, r.path.as_str()), (2, b.as_str()));
    assert_eq!(
        fresh.selection,
        Some(Entry::File(fs[1])),
        "the tree and panel follow"
    );
    fresh.on_event(&key_with(Key::N, Modifiers::SHIFT), None, 3.2);
    fresh.on_event(&key_with(Key::N, Modifiers::SHIFT), None, 3.3);
    assert_eq!(
        fresh.reader.as_ref().unwrap().note,
        1,
        "the reader stays at the first note"
    );
    assert_eq!(fresh.flash.as_ref().unwrap().0, FIRST_NOTE);
}

#[test]
fn z_and_the_chips_switch_the_mode_and_it_stays_for_the_next_note() {
    let (mut app, fs) = app();
    let a = path(&app, fs[0]);
    app.record_event(&note(&a, Some((200, 204)), "One."), 1.0);
    app.record_event(&note(&a, Some((20, 22)), "Two."), 1.1);
    app.on_event(&key(Key::N), None, 2.0);
    assert!(!app.reader_whole);
    assert_eq!(app.reader.as_ref().unwrap().top, 0);
    app.on_event(&key(Key::Z), None, 2.1);
    assert!(app.reader_whole);
    let page = app.reader_page().unwrap();
    let v = ReaderView::new(app.region);
    assert_eq!(
        app.reader.as_ref().unwrap().top,
        reader::entry_top(&page, &v)
    );
    assert!(page.top > 0);
    // The mode stays for the next note.
    app.on_event(&key(Key::N), None, 2.2);
    assert!(app.reader_whole);
    assert_eq!(app.reader_page().unwrap().rows.len(), 300 + 4);
    // The `lines` chip switches back.
    let chip = app
        .reader_page()
        .unwrap()
        .rect_of(ReaderHit::Mode(false))
        .unwrap();
    click(&mut app, chip.center(), 2.3);
    assert!(!app.reader_whole);
    assert_eq!(app.reader.as_ref().unwrap().top, 0);
    assert!(app.reader.is_some(), "a chip click keeps the reader");
}

#[test]
fn esc_returns_to_the_map_at_the_notes_card_with_no_zoom_change() {
    let (mut app, fs) = app();
    let b = path(&app, fs[1]);
    let zoom = app.motion.target().zoom;
    app.record_event(&note(&b, Some((2, 3)), "Two."), 1.0);
    app.on_event(&key(Key::N), None, 2.0);
    // The map keys and a pinch do nothing while the reader shows.
    let target = app.motion.target();
    for k in [
        Key::W,
        Key::D,
        Key::Plus,
        Key::Minus,
        Key::Num0,
        Key::H,
        Key::L,
    ] {
        app.on_event(&key(k), None, 2.1);
    }
    app.on_event(&Event::Zoom(2.0), None, 2.1);
    assert_eq!(app.motion.target(), target);
    assert!(app.reader.is_some());
    assert_eq!(
        app.hit(app.region.center()),
        None,
        "the reader covers the map"
    );
    app.on_event(&key(Key::Escape), None, 2.2);
    assert!(app.reader.is_none());
    assert_eq!(
        app.selection,
        Some(Entry::File(fs[1])),
        "Esc keeps the selection"
    );
    assert_eq!(app.motion.target().zoom, zoom);
    let shown = app
        .motion
        .target()
        .rect_to_screen(app.region, app.map.card(fs[1]));
    assert!(
        app.region.intersects(shown),
        "the map shows the note's card"
    );
}

#[test]
fn f_closes_the_reader_and_zooms_to_the_notes_line() {
    let (mut app, fs) = app();
    let a = path(&app, fs[0]);
    app.record_event(&note(&a, Some((100, 104)), "One."), 1.0);
    app.on_event(&key(Key::N), None, 2.0);
    app.on_event(&key(Key::F), None, 2.1);
    assert!(app.reader.is_none());
    assert_eq!(app.motion.target(), app.line_camera(fs[0], 100));
}

#[test]
fn clicks_open_and_close_the_reader() {
    let (mut app, fs) = app();
    let (a, b) = (path(&app, fs[0]), path(&app, fs[1]));
    app.record_event(&AgentEvent::trail(TrailKind::Read, vec![a.clone()]), 0.5);
    app.record_event(&note(&a, Some((100, 104)), "One."), 1.0);
    app.record_event(&note(&b, Some((2, 3)), "Two."), 1.1);
    // A history note row selects its file, and a second click opens its note (unit `057`).
    let strip = app.strip().unwrap();
    let rows: Vec<_> = strip
        .hits
        .iter()
        .filter_map(|(r, h)| match h {
            StripHit::Row(_) => Some(*r),
            _ => None,
        })
        .collect();
    click(&mut app, rows[1].center(), 1.5);
    assert!(app.reader.is_none());
    click(&mut app, rows[1].center(), 2.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 1);
    assert_eq!(app.note_cursor, Some(1));
    // The panel lists the file's notes; a click on one reads it.
    let panel = app.panel().unwrap();
    assert!(panel.texts().iter().any(|t| t.starts_with("NOTES")));
    let row = panel.rect_of(Hit::Note(1)).unwrap();
    assert!(panel.texts().contains(&"One."));
    app.close_reader();
    app.on_event(&press(row.center(), true), Some(row.center()), 3.0);
    app.on_event(&press(row.center(), false), Some(row.center()), 3.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 1);
    // A history row that is not a note closes the reader and selects its file.
    click(&mut app, rows[2].center(), 4.0);
    assert!(app.reader.is_none());
    assert_eq!(app.selection, Some(Entry::File(fs[0])));
    // A tree click closes the reader.
    app.on_event(&key(Key::N), None, 5.0);
    assert!(app.reader.is_some());
    let tree = app.tree_view.unwrap();
    let at = tree.rows.min + vec2(40.0, 10.0);
    click(&mut app, at, 6.0);
    assert!(app.reader.is_none());
    // A notes clear closes the reader. `N` from the map opens the note before the last one read.
    app.on_event(&key_with(Key::N, Modifiers::SHIFT), None, 7.0);
    assert!(app.reader.is_some());
    app.record_event(&AgentEvent::ClearNotes, 7.1);
    assert!(app.reader.is_none());
    assert_eq!(app.note_cursor, None);
}

#[test]
fn the_wheel_and_the_keys_scroll_within_the_limits() {
    let (mut app, fs) = app();
    let a = path(&app, fs[0]);
    app.record_event(&note(&a, Some((10, 12)), "One."), 1.0);
    app.on_event(&key(Key::N), None, 2.0);
    app.on_event(&key(Key::Z), None, 2.1);
    let max = app.reader_page().unwrap().max_top;
    assert!(max > 100);
    app.on_event(&key_with(Key::G, Modifiers::SHIFT), None, 2.2);
    assert_eq!(app.reader.as_ref().unwrap().top, max);
    app.on_event(&key(Key::J), None, 2.3);
    assert_eq!(
        app.reader.as_ref().unwrap().top,
        max,
        "the last row stops it"
    );
    app.on_event(&key(Key::K), None, 2.4);
    assert_eq!(app.reader.as_ref().unwrap().top, max - 1);
    app.on_event(&key(Key::G), None, 2.5);
    assert_eq!(app.reader.as_ref().unwrap().top, 0);
    app.on_event(&key(Key::K), None, 2.6);
    assert_eq!(
        app.reader.as_ref().unwrap().top,
        0,
        "the first row stops it"
    );
    let at = app.region.center();
    let cam = app.cam();
    let wheel = Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, -2.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    };
    app.on_event(&wheel, Some(at), 3.0);
    assert_eq!(app.reader.as_ref().unwrap().top, 6, "2 notches of 3 rows");
    assert_eq!(app.cam(), cam, "the wheel does not zoom the map");
    app.on_event(&key(Key::PageDown), None, 3.1);
    let page = ReaderView::new(app.region).visible_rows() - 2;
    assert_eq!(app.reader.as_ref().unwrap().top, 6 + page);
    // `t` works as on the map, and the reader stays.
    app.on_event(&key(Key::T), None, 3.2);
    assert!(!app.tree_on);
    assert!(app.reader.is_some());
}

#[test]
fn a_binary_file_shows_its_message_under_the_box() {
    let (mut app, fs) = app();
    let c = path(&app, fs[2]);
    app.record_event(&note(&c, Some((1, 1)), "Binary."), 1.0);
    app.on_event(&key(Key::N), None, 2.0);
    let page = app.reader_page().unwrap();
    assert!(
        page.texts()
            .contains(&format!("{c} is a binary file.").as_str())
    );
}

#[test]
fn a_headless_frame_draws_the_reader() {
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let (mut app, fs) = app();
    let a = path(&app, fs[0]);
    app.record_event(&note(&a, Some((10, 12)), "Retries start here."), 0.5);
    app.on_event(&key(Key::N), None, 1.0);
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
    for t in [
        "NOTE 1 of 1",
        "note 1 of 1",
        "Retries start here.",
        "line 10",
        minimap_gui::app::READER_HINT,
    ] {
        assert!(texts.iter().any(|x| x == t), "{t}: {texts:?}");
    }
    // The panel's NOTES section shows the note.
    assert!(texts.iter().any(|x| x.starts_with("NOTES")));
}

#[test]
fn the_colorer_colors_the_reader_after_plain_text() {
    // Unit 055: the reader shows plain text until the thread answers, then one text per colored span.
    use std::time::{Duration, Instant};
    let (mut app, _) = app();
    // A Clojure path, so the grammar colors it; it has no card, and the reader still reads it.
    let a = "src/dock.clj".to_string();
    let mut files = HashMap::new();
    files.insert(
        a.clone(),
        "(ns quayside.berth\n  (:require [clojure.string :as str]))\n\n(defn dock\n  \"Docks a ship.\"\n  [ship]\n  (str/upper-case ship))\n".to_string(),
    );
    app.content = Content::memory(files);
    app.colorer = Some(reader::Colorer::spawn(None, || {}));
    app.record_event(&note(&a, Some((4, 7)), "The dock."), 1.0);
    app.on_event(&key(Key::N), None, 2.0);
    let plain = app.reader_page().unwrap();
    assert!(plain.texts().contains(&"(defn dock"), "plain text first");
    let t0 = Instant::now();
    while app.reader.as_ref().unwrap().colors.is_none() {
        app.poll_colors(3.0);
        assert!(t0.elapsed() < Duration::from_secs(10), "no colors");
        std::thread::sleep(Duration::from_millis(5));
    }
    let colors = app.reader.as_ref().unwrap().colors.clone().unwrap();
    assert_eq!(colors.len(), 7, "one span list per line");
    let page = app.reader_page().unwrap();
    let texts = page.texts();
    assert!(!texts.contains(&"(defn dock"), "the line draws as spans");
    assert!(texts.contains(&"defn"), "{texts:?}");
    let distinct: std::collections::HashSet<u32> = page
        .draws
        .iter()
        .filter_map(|d| match d {
            minimap_gui::panel::Draw::Text { at, color, .. } if at.x > 60.0 => Some(*color),
            _ => None,
        })
        .collect();
    assert!(distinct.len() >= 3, "{distinct:?}");
    // A span after a tab starts at the tab's next stop.
    let (t, next) = reader::expand_from("\tx", 2, 80);
    assert_eq!((t.as_str(), next), ("  x", 5));
}

#[test]
fn a_text_too_long_for_colors_stays_plain() {
    let many: Vec<String> = (0..minimap_core::syntax::MAX_LINES + 1)
        .map(|k| k.to_string())
        .collect();
    assert!(!reader::colorable(&many));
    assert!(reader::colorable(&many[..100]));
    let wide = vec!["x".repeat(minimap_core::syntax::MAX_BYTES)];
    assert!(!reader::colorable(&wide));
    // A colorer with a theme that does not load reports it once, and colors with the built-in theme.
    let c = reader::Colorer::spawn(Some("/no/such/theme.tmTheme".into()), || {});
    match c.wait(std::time::Duration::from_secs(10)) {
        Some(reader::Done::ThemeError(e)) => assert!(e.starts_with("Cannot load the theme"), "{e}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_note_on_a_deleted_file_opens_from_its_history_row() {
    // A pull request can delete a file: its note has no card, and its row still opens the reader with a message.
    let (mut app, _) = app();
    app.record_event(
        &note("gone/deleted.clj", Some((3, 4)), "Removed here."),
        1.0,
    );
    let row = app
        .strip()
        .unwrap()
        .hits
        .iter()
        .find_map(|(r, h)| matches!(h, StripHit::Row(_)).then_some(*r))
        .expect("the note row clicks");
    click(&mut app, row.center(), 2.0);
    let page = app.reader_page().expect("the reader shows the note");
    assert!(page.texts().contains(&"Removed here."));
    assert!(page.rows.contains(&Row::Message));
    assert_eq!(app.selection, None, "no card to select");
}

#[test]
fn a_note_row_click_selects_its_file_first_and_a_second_click_reads_it() {
    // Unit 057: the user sees the file on the map before the reader covers it.
    let (mut app, fs) = app();
    let (a, b) = (path(&app, fs[0]), path(&app, fs[1]));
    app.record_event(&note(&a, Some((100, 104)), "One."), 1.0);
    app.record_event(&note(&b, Some((2, 3)), "Two."), 1.1);
    let rows: Vec<_> = app
        .strip()
        .unwrap()
        .hits
        .iter()
        .filter_map(|(r, h)| matches!(h, StripHit::Row(_)).then_some(*r))
        .collect();
    // The rows, newest first: note 2 on `b`, then note 1 on `a`.
    let before = app.motion.target();
    click(&mut app, rows[1].center(), 2.0);
    assert!(app.reader.is_none(), "the first click does not read");
    assert_eq!(app.selection, Some(Entry::File(fs[0])));
    assert_eq!(
        app.motion.target(),
        app.camera_showing(before, app.map.card(fs[0])),
        "the map shows the card, as on a trail row click"
    );
    assert_eq!(app.motion.target().zoom, before.zoom, "no zoom change");
    // A second click, after the double-click time, reads the note.
    click(&mut app, rows[1].center(), 3.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 1);
    assert_eq!(app.selection, Some(Entry::File(fs[0])));
    // A double-click on another note row selects its file and reads the note, in place of the zoom.
    let zoom = app.motion.target().zoom;
    click(&mut app, rows[0].center(), 4.0);
    assert!(app.reader.is_none(), "another file's row closes the reader");
    assert_eq!(app.selection, Some(Entry::File(fs[1])));
    click(&mut app, rows[0].center(), 4.2);
    assert_eq!(app.reader.as_ref().unwrap().note, 2);
    assert_eq!(
        app.motion.target().zoom,
        zoom,
        "a double-click does not zoom"
    );
    assert_ne!(app.motion.target(), app.camera_for(Entry::File(fs[1])));
    // The panel's NOTES row reads its note on one click: its file is the selection.
    app.close_reader();
    let panel = app.panel().unwrap();
    let row = panel.rect_of(Hit::Note(2)).unwrap();
    click(&mut app, row.center(), 6.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 2);
    // `n` and `N` read the note at once.
    app.close_reader();
    app.on_event(&key_with(Key::N, Modifiers::SHIFT), None, 7.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 1);
    // A note row on a path without a card reads on the first click.
    app.close_reader();
    app.record_event(&note("gone/deleted.clj", None, "Removed."), 8.0);
    // The new row is the top row; the rows below move down by one.
    let rows: Vec<_> = app
        .strip()
        .unwrap()
        .hits
        .iter()
        .filter_map(|(r, h)| matches!(h, StripHit::Row(_)).then_some(*r))
        .collect();
    click(&mut app, rows[0].center(), 9.0);
    assert_eq!(app.reader.as_ref().unwrap().note, 3);
    // After a notes clear, a note row clicks as a trail row: it selects, and a double-click zooms.
    app.record_event(&AgentEvent::ClearNotes, 10.0);
    click(&mut app, rows[2].center(), 11.0);
    click(&mut app, rows[2].center(), 11.1);
    assert!(app.reader.is_none());
    assert_eq!(app.motion.target(), app.camera_for(Entry::File(fs[0])));
}

/// A text for the selection tests: a tab, and a line longer than the reader's width.
fn select_text() -> String {
    let long = format!("\t(str/upper-case ship) ;; {}", "x".repeat(400));
    let more: String = (0..100).map(|k| format!("(comment {k})\n")).collect();
    format!("(ns quayside.dock)\n(defn dock [ship]\n{long}\n(dock :queen-mary)\n{more}")
}

/// The reader on `fs[1]` with [`select_text`], in `whole file` mode, with a note without lines: its box is rows 0 and 1, and file line `n` is row `n + 2`.
fn select_app() -> (GuiApp, Vec<usize>) {
    let (mut app, fs) = app();
    let b = path(&app, fs[1]);
    let mut files = HashMap::new();
    files.insert(b.clone(), select_text());
    app.content = Content::memory(files);
    app.record_event(&note(&b, None, "Read the dock."), 1.0);
    app.on_event(&key(Key::N), None, 2.0);
    app.on_event(&key(Key::Z), None, 2.1);
    (app, fs)
}

/// The screen point of a row's text column, a fifth of a cell right of the column's left edge.
fn spot(app: &GuiApp, row: usize, col: usize, start: usize) -> Pos2 {
    let v = ReaderView::new(app.region);
    let cell = app.opts.char_w * 13.0;
    let top = app.reader_page().unwrap().top;
    egui::pos2(
        v.rect.left() + (start + col) as f32 * cell + 0.2 * cell,
        v.body.top() + (row - top) as f32 * CODE_ROW + CODE_ROW / 2.0,
    )
}

fn drag(app: &mut GuiApp, from: Pos2, to: Pos2, now: f64) {
    app.on_event(&press(from, true), Some(from), now);
    app.on_event(&Event::PointerMoved(from + vec2(6.0, 0.0)), Some(from), now);
    app.on_event(&Event::PointerMoved(to), Some(to), now);
    app.on_event(&press(to, false), Some(to), now);
}

#[test]
fn a_drag_selects_by_row_and_column_and_the_copy_has_the_full_lines() {
    // Unit 057: from row 3 column 4 to row 5 column 2, as in a terminal.
    use minimap_core::palette;
    use minimap_gui::reader::{GUTTER, SELECT_MIX, Selection, Spot};
    let (mut app, _) = select_app();
    let page = app.reader_page().unwrap();
    assert_eq!(
        page.rows[3],
        Row::Line {
            n: 1,
            context: false
        }
    );
    let start = GUTTER + 1;
    let (from, to) = (spot(&app, 3, 4, start), spot(&app, 5, 2, start));
    drag(&mut app, from, to, 3.0);
    assert_eq!(
        app.reader_sel,
        Some(Selection {
            anchor: Spot { row: 3, col: 4 },
            head: Spot { row: 5, col: 2 },
        })
    );
    assert!(app.reader.is_some(), "a drag keeps the reader");
    // The fills: row 3 from column 4, row 4 whole, row 5 to column 2, as `CURSOR` at 25 % over the pane.
    let v = ReaderView::new(app.region);
    let cell = app.opts.char_w * 13.0;
    let x = |c: usize| v.rect.left() + c as f32 * cell;
    let color = minimap_gui::color::mix_hex(palette::CURSOR, palette::PANE, SELECT_MIX);
    let page = app.reader_page().unwrap();
    let fills: Vec<eframe::egui::Rect> = page
        .draws
        .iter()
        .filter_map(|d| match d {
            minimap_gui::panel::Draw::Fill { rect, color: c } if *c == color => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 3, "{fills:?}");
    let right = x(((v.rect.width() / cell).floor() as usize) - 1);
    let row_top = |r: usize| v.body.top() + r as f32 * CODE_ROW;
    let want = [
        (x(start + 4), right, row_top(3)),
        (x(start), right, row_top(4)),
        (x(start), x(start + 2), row_top(5)),
    ];
    for (r, (l, rt, t)) in fills.iter().zip(want) {
        assert!(
            (r.left() - l).abs() < 1e-3
                && (r.right() - rt).abs() < 1e-3
                && (r.top() - t).abs() < 1e-3,
            "{r:?}"
        );
    }
    // The selection draws below the text.
    let first_sel = page
        .draws
        .iter()
        .position(|d| matches!(d, minimap_gui::panel::Draw::Fill { color: c, .. } if *c == color))
        .unwrap();
    let first_code = page.draws.iter().position(|d| matches!(d, minimap_gui::panel::Draw::Text { text, .. } if text == "(defn dock [ship]")).unwrap();
    assert!(first_sel < first_code);
    // The copy: the full lines, with the tab and the part past the screen, and no gutter.
    let text = select_text();
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        !page.texts().contains(&lines[2]),
        "the long line is cut on the screen"
    );
    app.on_event(&Event::Copy, None, 3.1);
    let want = format!("n dock [ship]\n{}\n(d", lines[2]);
    assert_eq!(app.copied.as_deref(), Some(want.as_str()));
    // The wheel and the keys scroll and keep the selection; Esc clears it, and a second Esc closes the reader.
    app.on_event(&key(Key::J), None, 3.2);
    assert_eq!(app.reader.as_ref().unwrap().top, 1);
    assert!(app.reader_sel.is_some());
    app.on_event(&key(Key::Escape), None, 3.3);
    assert!(app.reader_sel.is_none() && app.reader.is_some());
    app.on_event(&key(Key::Escape), None, 3.4);
    assert!(app.reader.is_none());
}

#[test]
fn a_click_clears_the_selection_and_a_drag_from_the_gutter_starts_at_column_0() {
    let (mut app, _) = select_app();
    let start = minimap_gui::reader::GUTTER + 1;
    // A drag from the gutter, upward: the ends sort.
    let gutter = spot(&app, 2, 0, 1);
    let from = spot(&app, 3, 10, start);
    drag(&mut app, from, gutter, 3.0);
    app.copied = None;
    app.on_event(&Event::Copy, None, 3.1);
    assert_eq!(
        app.copied.as_deref(),
        Some("(ns quayside.dock)\n(defn dock")
    );
    // A click without a drag clears the selection, and Cmd+C copies nothing.
    let at = spot(&app, 4, 3, start);
    click(&mut app, at, 4.0);
    assert!(app.reader_sel.is_none());
    app.copied = None;
    app.on_event(&Event::Copy, None, 4.1);
    assert_eq!(app.copied, None);
    // A mode chip works during a selection, and the new mode's rows clear it.
    let (from, to) = (spot(&app, 3, 0, start), spot(&app, 4, 2, start));
    drag(&mut app, from, to, 5.0);
    assert!(app.reader_sel.is_some());
    let chip = app
        .reader_page()
        .unwrap()
        .rect_of(ReaderHit::Mode(false))
        .unwrap();
    click(&mut app, chip.center(), 5.1);
    assert!(!app.reader_whole);
    assert!(app.reader_sel.is_none());
    // A drag below the rows ends at the last shown row's end; above them, at the first shown row's start.
    let page = app.reader_page().unwrap();
    let last = page.top + ReaderView::new(app.region).visible_rows() - 1;
    assert!(last + 1 < page.rows.len(), "rows below the body");
    let below = egui::pos2(app.region.center().x, app.region.bottom() - 1.0);
    let above = egui::pos2(app.region.center().x, app.region.top() + 2.0);
    let from = spot(&app, 2, 0, start);
    drag(&mut app, from, below, 6.0);
    let sel = app.reader_sel.unwrap();
    assert_eq!(sel.head.row, last);
    let from = spot(&app, 2, 3, start);
    drag(&mut app, from, above, 7.0);
    assert_eq!(
        app.reader_sel.unwrap().head,
        minimap_gui::reader::Spot { row: 0, col: 0 }
    );
    // A width change clears it: the rows cut and wrap anew.
    app.on_event(&key(Key::T), None, 8.0);
    assert!(app.reader_sel.is_none());
}

#[test]
fn note_box_rows_copy_as_the_notes_text_lines() {
    // Unit 057: wrapped rows give the note's lines back, with their own spaces.
    use minimap_gui::reader::{Selection, Spot, box_rows, selected_text};
    let text = "First  paragraph with enough words to wrap twice in a narrow box.\n\n  Second.";
    for w in [8, 13, 24, 80] {
        for (k, line) in text.lines().enumerate() {
            let rows: Vec<String> = box_rows(text, w)
                .into_iter()
                .filter(|r| r.line == k)
                .map(|r| r.text)
                .collect();
            let want = if line.trim().is_empty() {
                vec![String::new()]
            } else {
                minimap_core::text::wrap(line, w)
            };
            assert_eq!(rows, want, "width {w}, line {k}");
        }
    }
    let mut notes = Notes::default();
    notes.record(&note("a.clj", Some((1, 1)), text));
    let t = Text::of("a.clj", "x\n");
    // A narrow view: the box wraps at 24 characters.
    let cell = 0.6 * 13.0;
    let v = ReaderView::new(eframe::egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        vec2(28.5 * cell, 600.0),
    ));
    let page = reader::build(&v, &facts(&notes, 1, false, &t));
    let note_rows: Vec<usize> = (0..page.rows.len())
        .filter(|&k| matches!(page.rows[k], Row::Note { .. }))
        .collect();
    assert_eq!(note_lines(text, 24).len(), note_rows.len());
    assert_eq!(note_rows.len(), 5, "{:?}", note_lines(text, 24));
    let sel = |a: (usize, usize), b: (usize, usize)| Selection {
        anchor: Spot { row: a.0, col: a.1 },
        head: Spot { row: b.0, col: b.1 },
    };
    let f = facts(&notes, 1, false, &t);
    // From the label row to the code row's start: the whole note, with its double space, blank line, and indent.
    let last = page.rows.len() - 1;
    assert_eq!(
        selected_text(&v, &f, sel((0, 0), (last, 0))),
        format!("{text}\n")
    );
    // From the middle of the first row to the middle of the second: one text line, the spaces at the wrap kept.
    assert_eq!(
        selected_text(&v, &f, sel((1, 6), (2, 6))),
        "paragraph with enough"
    );
    // A start at a line's first column takes its indent; an end at a line's last column takes the line's end.
    assert_eq!(selected_text(&v, &f, sel((5, 0), (5, 7))), "  Second.");
}

#[test]
fn a_headless_frame_hands_the_copy_to_the_clipboard() {
    // Unit 057: Cmd+C arrives as `Event::Copy`, and the frame copies through `egui::Context::copy_text`.
    let ctx = egui::Context::default();
    minimap_gui::fonts::install(&ctx, None).unwrap();
    let (mut app, _) = select_app();
    let start = minimap_gui::reader::GUTTER + 1;
    let (from, to) = (spot(&app, 2, 0, start), spot(&app, 3, 5, start));
    drag(&mut app, from, to, 3.0);
    let input = egui::RawInput {
        screen_rect: Some(common::screen()),
        time: Some(10.0),
        max_texture_side: Some(8192),
        events: vec![Event::Copy],
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.frame(ui, None));
    out.textures_delta.clear();
    let copied: Vec<&String> = out
        .platform_output
        .commands
        .iter()
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(copied, [&"(ns quayside.dock)\n(defn".to_string()]);
    assert_eq!(app.copied, None, "the frame took it");
    let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
}
