//! The file viewer (unit 029): motions, find, syntax colors, and the viewer in the map column.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use common::*;
use minimap_core::{Content, Details};
use minimap_tui::App;
use minimap_tui::filetree::Entry;
use minimap_tui::syntax::Highlighter;
use minimap_tui::viewer::{self, Action, Sign, Viewer};

/// 100 lines: `line N`, with lines 10, 20, … 90 blank.
fn hundred() -> Viewer {
    let text: Vec<String> = (1..=100)
        .map(|n| {
            if n % 10 == 0 && n < 100 {
                String::new()
            } else {
                format!("line {n}")
            }
        })
        .collect();
    let mut v = Viewer::new(0, "src/a.clj", &text.join("\n"));
    v.set_height(20);
    v
}

/// Outline lines, 1-based.
const OUTLINE: [u32; 5] = [3, 25, 40, 71, 96];

fn key(v: &mut Viewer, code: KeyCode) -> Action {
    v.key(KeyEvent::new(code, KeyModifiers::NONE), &OUTLINE)
}

fn ctrl(v: &mut Viewer, c: char) -> Action {
    v.key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        &OUTLINE,
    )
}

fn typed(v: &mut Viewer, s: &str) {
    for c in s.chars() {
        key(v, KeyCode::Char(c));
    }
}

#[test]
fn j_and_k_move_the_cursor_with_counts_and_stop_at_the_ends() {
    let mut v = hundred();
    typed(&mut v, "j");
    assert_eq!(v.cursor, 1);
    typed(&mut v, "5j");
    assert_eq!(v.cursor, 6);
    typed(&mut v, "k");
    assert_eq!(v.cursor, 5);
    typed(&mut v, "20k");
    assert_eq!((v.cursor, v.top), (0, 0));
    typed(&mut v, "25j");
    // The view scrolls only as far as needed: line 26 is the last row of 20.
    assert_eq!((v.cursor, v.top), (25, 6));
    typed(&mut v, "500j");
    assert_eq!((v.cursor, v.top), (99, 80));
    key(&mut v, KeyCode::Down);
    assert_eq!(v.cursor, 99);
    key(&mut v, KeyCode::Up);
    assert_eq!(v.cursor, 98);
}

#[test]
fn ctrl_keys_scroll_by_half_and_whole_pages() {
    let mut v = hundred();
    ctrl(&mut v, 'd');
    assert_eq!((v.cursor, v.top), (10, 10));
    ctrl(&mut v, 'f');
    assert_eq!((v.cursor, v.top), (30, 30));
    ctrl(&mut v, 'u');
    assert_eq!((v.cursor, v.top), (20, 20));
    ctrl(&mut v, 'b');
    assert_eq!((v.cursor, v.top), (0, 0));
    ctrl(&mut v, 'b');
    assert_eq!((v.cursor, v.top), (0, 0));
    // Near the end the view stops with the last line at the bottom; the cursor goes on to the last line.
    for _ in 0..10 {
        ctrl(&mut v, 'f');
    }
    assert_eq!((v.cursor, v.top), (99, 80));
}

#[test]
fn gg_g_and_counts_go_to_lines() {
    let mut v = hundred();
    typed(&mut v, "G");
    assert_eq!(v.cursor, 99);
    typed(&mut v, "gg");
    assert_eq!(v.cursor, 0);
    typed(&mut v, "12G");
    assert_eq!(v.cursor, 11);
    typed(&mut v, "40gg");
    assert_eq!(v.cursor, 39);
    typed(&mut v, "999G");
    assert_eq!(v.cursor, 99);
}

#[test]
fn z_keys_put_the_cursor_line_at_the_middle_top_or_bottom() {
    let mut v = hundred();
    typed(&mut v, "50G");
    typed(&mut v, "zt");
    assert_eq!(v.top, 49);
    typed(&mut v, "zz");
    assert_eq!(v.top, 39);
    typed(&mut v, "zb");
    assert_eq!(v.top, 30);
    assert_eq!(v.cursor, 49);
}

#[test]
fn brackets_jump_between_definitions_and_braces_between_blank_lines() {
    let mut v = hundred();
    typed(&mut v, "]]");
    assert_eq!(v.cursor, 2);
    typed(&mut v, "]]");
    assert_eq!(v.cursor, 24);
    typed(&mut v, "2]]");
    assert_eq!(v.cursor, 70);
    typed(&mut v, "[[");
    assert_eq!(v.cursor, 39);
    typed(&mut v, "G]]");
    assert_eq!(v.cursor, 99, "no definition after the last line");
    typed(&mut v, "gg");
    typed(&mut v, "}");
    assert_eq!(v.cursor, 9);
    typed(&mut v, "}}");
    assert_eq!(v.cursor, 29);
    typed(&mut v, "{");
    assert_eq!(v.cursor, 19);
    typed(&mut v, "G}");
    assert_eq!(v.cursor, 99);
    typed(&mut v, "gg{");
    assert_eq!(v.cursor, 0);
    let mut plain = hundred();
    let no_outline = plain.key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE), &[]);
    assert_eq!(no_outline, Action::None);
    let a = plain.key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE), &[]);
    assert_eq!(a, Action::Flash("This file has no outline.".to_string()));
}

#[test]
fn h_l_and_0_scroll_sideways() {
    let mut v = Viewer::new(0, "a.txt", &format!("{}\nshort", "x".repeat(40)));
    v.set_height(5);
    typed(&mut v, "l");
    assert_eq!(v.left, viewer::HSCROLL);
    typed(&mut v, "10l");
    assert_eq!(v.left, 39, "stops at the widest line's last column");
    typed(&mut v, "h");
    assert_eq!(v.left, 31);
    typed(&mut v, "0");
    assert_eq!(v.left, 0);
}

#[test]
fn find_uses_smart_case_and_n_wraps() {
    let text = "(defn Alpha [])\n(def beta 1)\n(defn gamma [] alpha)\n(def ALPHA 2)";
    let mut v = Viewer::new(0, "a.clj", text);
    v.set_height(10);
    typed(&mut v, "/alpha");
    assert_eq!(v.prompt.as_deref(), Some("alpha"));
    assert_eq!(v.match_count("alpha"), 3, "lowercase ignores case");
    assert_eq!(
        v.match_count("Alpha"),
        1,
        "an uppercase letter matches case"
    );
    key(&mut v, KeyCode::Enter);
    // At or after the cursor: line 1 itself matches.
    assert_eq!((v.cursor, v.prompt.as_deref()), (0, None));
    typed(&mut v, "n");
    assert_eq!(v.cursor, 2);
    typed(&mut v, "n");
    assert_eq!(v.cursor, 3);
    typed(&mut v, "n");
    assert_eq!(v.cursor, 0, "n wraps to the top");
    typed(&mut v, "N");
    assert_eq!(v.cursor, 3, "N wraps to the bottom");
    assert_eq!(v.matches_in(2), vec![(15, 20)]);
    typed(&mut v, "/zeta");
    let a = key(&mut v, KeyCode::Enter);
    assert_eq!(a, Action::Flash("Not found: zeta".to_string()));
    // Esc closes the prompt and keeps the last query.
    typed(&mut v, "/be");
    key(&mut v, KeyCode::Esc);
    assert_eq!(
        (v.prompt.as_deref(), v.query.as_deref()),
        (None, Some("zeta"))
    );
    let mut fresh = Viewer::new(0, "a.clj", text);
    assert_eq!(
        key(&mut fresh, KeyCode::Char('n')),
        Action::Flash("No search yet. Press / to find.".to_string())
    );
}

#[test]
fn other_keys_ask_the_app() {
    let mut v = hundred();
    typed(&mut v, "3j");
    assert_eq!(
        key(&mut v, KeyCode::Char('y')),
        Action::Copy("src/a.clj:4".to_string())
    );
    assert_eq!(key(&mut v, KeyCode::Char('e')), Action::Editor);
    assert_eq!(key(&mut v, KeyCode::Char('t')), Action::ToggleTree);
    assert_eq!(key(&mut v, KeyCode::Char('p')), Action::TogglePanel);
    assert_eq!(key(&mut v, KeyCode::Char('?')), Action::Help);
    assert_eq!(key(&mut v, KeyCode::Char('q')), Action::Close);
    assert_eq!(key(&mut v, KeyCode::Esc), Action::Close);
}

#[test]
fn tabs_run_to_the_next_stop_and_wide_chars_take_two_cells() {
    assert_eq!(viewer::display_width("\tab"), 6);
    assert_eq!(viewer::display_width("ab\tc"), 5);
    assert_eq!(viewer::char_width('\t', 3), 1);
    assert_eq!(viewer::char_width('漢', 0), 2);
    assert_eq!(viewer::display_width("a漢b"), 4);
}

#[test]
fn main_file_types_have_a_syntax() {
    let (h, err) = Highlighter::new(None);
    assert_eq!(err, None);
    for path in [
        "a.clj",
        "a.cljs",
        "a.cljc",
        "a.sql",
        "a.ts",
        "a.js",
        "a.py",
        "README.md",
        "a.json",
        "a.yml",
        "a.toml",
        "a.sh",
        "a.rs",
        "Makefile",
    ] {
        let s = h.syntax(path, "");
        assert_ne!(s.name, "Plain Text", "{path}");
    }
    assert_eq!(h.syntax("notes.unknownext", "").name, "Plain Text");
    assert_eq!(
        h.syntax("run", "#!/bin/bash").name,
        "Bourne Again Shell (bash)"
    );
}

#[test]
fn the_built_in_theme_colors_keywords_and_strings_apart() {
    let (h, _) = Highlighter::new(None);
    assert_eq!(
        h.foreground(),
        Some(ratatui::style::Color::Rgb(0xc0, 0xca, 0xf5))
    );
    let c = h.highlight("a.clj", "(def x :kw \"str\")\n");
    assert_eq!(c.syntax, "Clojure");
    let fg = |t: &str| c.lines[0].iter().find(|s| s.text.contains(t)).map(|s| s.fg);
    assert!(
        fg("kw").is_some() && fg("str").is_some(),
        "{:?}",
        c.lines[0]
    );
    assert_ne!(fg("kw"), fg("str"));
    let (_, err) = Highlighter::new(Some(std::path::Path::new("/no/such.tmTheme")));
    assert!(
        err.unwrap()
            .starts_with("Cannot load the theme /no/such.tmTheme")
    );
}

/// The quayside path that holds the berth fixture in the viewer tests.
const BERTH: &str = "components/berth/src/quayside/berth/core.clj";

/// Quayside at `w` x `h` with the berth fixture as the text and the details of one file.
fn quayside_viewer(w: u16, h: u16) -> (ratatui::Terminal<ratatui::backend::TestBackend>, App) {
    let (mut t, mut app) = quayside(w, h);
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/clojure/berth.clj"
    ))
    .unwrap();
    let files = HashMap::from([(BERTH.to_string(), text)]);
    app.content = Content::memory(files.clone());
    app.details = Details::memory(files);
    let f = file(&app, BERTH);
    app.index.files[f].lines = 51;
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    (t, app)
}

fn open(t: &mut ratatui::Terminal<ratatui::backend::TestBackend>, app: &mut App) {
    press(app, KeyCode::Enter);
    app.wait_for_colors(Duration::from_secs(10));
    draw(t, app);
}

#[test]
fn enter_views_the_file_and_esc_returns_to_the_same_map() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    let tiles = app.layout.tiles.clone();
    let cam = (app.cam_x, app.cam_y);
    open(&mut t, &mut app);
    assert!(app.viewer.is_some());
    assert!(row(&t, 44).starts_with(" VIEW  1 / 51 "), "{}", row(&t, 44));
    keys(&mut app, "20j");
    press(&mut app, KeyCode::Esc);
    draw(&mut t, &mut app);
    assert!(app.viewer.is_none());
    assert_eq!(app.layout.tiles, tiles);
    assert_eq!((app.cam_x, app.cam_y), cam);
    // The viewer opens again where it closed.
    open(&mut t, &mut app);
    assert_eq!(app.viewer.as_ref().unwrap().cursor, 20);
}

#[test]
fn the_viewer_needs_text() {
    let (_, mut app) = quayside(150, 45);
    press(&mut app, KeyCode::Enter);
    assert!(app.viewer.is_none());
    assert_eq!(
        app.flash_text(),
        Some("No file contents in --from-json mode.")
    );
}

#[test]
fn y_copies_the_location_and_e_asks_the_editor() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    let copies = record_copies(&mut app);
    open(&mut t, &mut app);
    keys(&mut app, "7Gy");
    assert_eq!(copies.borrow().as_slice(), [format!("{BERTH}:7")]);
    // Text in memory has no repo on disk for an editor (unit 031 tests the editor itself).
    keys(&mut app, "e");
    assert_eq!(
        app.flash_text(),
        Some("No repo on disk to open files from.")
    );
}

#[test]
fn enter_in_outline_browse_views_the_item_at_the_top() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    keys(&mut app, "o");
    keys(&mut app, "jj");
    press(&mut app, KeyCode::Enter);
    draw(&mut t, &mut app);
    let vw = app.viewer.as_ref().unwrap();
    // The third outline item, `score`, is on line 13.
    assert_eq!((vw.cursor, vw.top), (12, 12));
}

#[test]
fn the_wheel_scrolls_and_a_click_moves_the_cursor() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    open(&mut t, &mut app);
    let v = app.view();
    mouse(&mut app, MouseEventKind::ScrollDown, v.mx + 5, v.my + 5);
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!((vw.top, vw.cursor), (3, 3));
    click(&mut app, v.mx + 5, v.my + 4);
    assert_eq!(app.viewer.as_ref().unwrap().cursor, 7);
}

#[test]
fn a_double_click_on_a_tile_views_the_file() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    let f = file(&app, BERTH);
    let (sx, sy) = screen(&app, app.layout.tiles[f]);
    click(&mut app, sx, sy);
    assert!(app.viewer.is_none(), "one click selects");
    app.now += Duration::from_millis(200);
    click(&mut app, sx, sy);
    assert!(app.viewer.is_some(), "a second click views");
    draw(&mut t, &mut app);
}

#[test]
fn viewer_snapshots() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    open(&mut t, &mut app);
    insta::assert_snapshot!("viewer_top", t.backend());
    keys(&mut app, "]]]]");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("viewer_after_two_definitions", t.backend());
    keys(&mut app, "gg/berth");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("viewer_find_prompt", t.backend());
    press(&mut app, KeyCode::Enter);
    keys(&mut app, "t");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("viewer_found_without_tree", t.backend());
}

/// The berth fixture with uncommitted edits: line 7 changes, the `score` fn (lines 13-14) goes,
/// and two lines arrive at the end. `HEAD` holds the fixture as committed.
fn quayside_diff(w: u16, h: u16) -> (ratatui::Terminal<ratatui::backend::TestBackend>, App) {
    let (t, mut app) = quayside_viewer(w, h);
    let head = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/clojure/berth.clj"
    ))
    .unwrap();
    let mut lines: Vec<&str> = head.lines().collect();
    lines[5] = "(def max-length 450)";
    lines.drain(12..15);
    let mut edited = lines.join("\n");
    edited.push_str("\n\n(def berths-per-crane 3)\n");
    app.content = Content::Memory {
        files: HashMap::from([(BERTH.to_string(), edited.clone())]),
        head: HashMap::from([(BERTH.to_string(), head)]),
    };
    app.details = Details::memory(HashMap::from([(BERTH.to_string(), edited)]));
    (t, app)
}

#[test]
fn the_status_bar_counts_changes_and_equals_switches_to_diff_mode() {
    let (mut t, mut app) = quayside_diff(150, 45);
    open(&mut t, &mut app);
    let vw = app.viewer.as_ref().unwrap();
    let d = vw.diff.as_ref().expect("the diff arrives with the colors");
    assert_eq!((d.added, d.removed), (3, 4));
    assert!(
        row(&t, 44).starts_with(" VIEW  1 / 50 +3 −4 · = diff"),
        "{}",
        row(&t, 44)
    );
    keys(&mut app, "8G=");
    draw(&mut t, &mut app);
    let vw = app.viewer.as_ref().unwrap();
    assert!(vw.diff_mode);
    // Line 8 keeps the cursor; in diff mode it is row 9, below the removed line 6 and the added line 6.
    assert_eq!((vw.cursor_line(), vw.cursor), (7, 8));
    assert!(
        row(&t, 44).starts_with(" DIFF  8 / 50 +3 −4"),
        "{}",
        row(&t, 44)
    );
    keys(&mut app, "=");
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!((vw.diff_mode, vw.cursor), (false, 7));
}

#[test]
fn hunk_motions_stop_at_the_ends() {
    let (mut t, mut app) = quayside_diff(150, 45);
    open(&mut t, &mut app);
    // File mode: hunks start at lines 6, 13, and 49 (1-based).
    keys(&mut app, "]c");
    assert_eq!(app.viewer.as_ref().unwrap().cursor_line(), 5);
    keys(&mut app, "]c");
    assert_eq!(app.viewer.as_ref().unwrap().cursor_line(), 12);
    keys(&mut app, "]c]c]c");
    assert_eq!(
        app.viewer.as_ref().unwrap().cursor_line(),
        48,
        "stops at the last hunk"
    );
    keys(&mut app, "[c");
    assert_eq!(app.viewer.as_ref().unwrap().cursor_line(), 12);
    keys(&mut app, "gg[c");
    assert_eq!(app.viewer.as_ref().unwrap().cursor_line(), 0);
    // Diff mode: the hunk rows.
    keys(&mut app, "=]c");
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!((vw.cursor, vw.sign(vw.cursor)), (5, Sign::Minus));
}

#[test]
fn y_on_a_removed_row_copies_the_next_line() {
    let (mut t, mut app) = quayside_diff(150, 45);
    let copies = record_copies(&mut app);
    open(&mut t, &mut app);
    keys(&mut app, "=]c]c");
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!(vw.sign(vw.cursor), Sign::Minus);
    keys(&mut app, "y");
    // The removed `score` fn sat above working-tree line 13.
    assert_eq!(copies.borrow().as_slice(), [format!("{BERTH}:13")]);
}

#[test]
fn a_clean_file_has_no_diff() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    open(&mut t, &mut app);
    keys(&mut app, "=");
    assert_eq!(
        app.flash_text(),
        Some("No uncommitted changes in this file.")
    );
    keys(&mut app, "]c");
    assert_eq!(
        app.flash_text(),
        Some("No uncommitted changes in this file.")
    );
    assert!(!app.viewer.as_ref().unwrap().diff_mode);
}

#[test]
fn a_file_that_head_does_not_have_is_all_added() {
    let (mut t, mut app) = quayside_viewer(150, 45);
    let Content::Memory { files, .. } = app.content.clone() else {
        unreachable!()
    };
    app.content = Content::Memory {
        files,
        head: HashMap::new(),
    };
    open(&mut t, &mut app);
    let vw = app.viewer.as_ref().unwrap();
    let d = vw.diff.as_ref().unwrap();
    assert_eq!((d.added, d.removed), (51, 0));
    assert_eq!(vw.sign(0), Sign::Added);
}

#[test]
fn diff_snapshots() {
    let (mut t, mut app) = quayside_diff(150, 45);
    open(&mut t, &mut app);
    insta::assert_snapshot!("diff_file_mode_signs", t.backend());
    keys(&mut app, "=");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("diff_mode", t.backend());
    keys(&mut app, "]c]c");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("diff_after_two_hunks", t.backend());
}

#[test]
fn a_file_changed_between_views_gets_a_new_diff() {
    // The agent edits a file while the user reads it: the next view diffs the new text.
    let (mut t, mut app) = quayside_viewer(150, 45);
    open(&mut t, &mut app);
    assert!(
        app.viewer
            .as_ref()
            .unwrap()
            .diff
            .as_ref()
            .unwrap()
            .is_empty()
    );
    press(&mut app, KeyCode::Esc);
    let (_, edited) = quayside_diff(150, 45);
    app.content = edited.content.clone();
    open(&mut t, &mut app);
    let d = app.viewer.as_ref().unwrap().diff.clone().unwrap();
    assert_eq!((d.added, d.removed), (3, 4));
}
