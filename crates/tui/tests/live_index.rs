//! The live index in the app (unit `034`): in-memory updates on the fake `quayside` repo.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use ratatui::crossterm::event::KeyCode;

use common::*;
use minimap_core::watch::Update;
use minimap_core::{Content, Index, IndexFile};
use minimap_tui::App;
use minimap_tui::filetree::Entry;

/// A directory with 6 files in a grid of 7 slots (unit `027`), so one more file fits.
const USERS: &str = "bases/admin/src/quayside/admin/users";

fn path(name: &str) -> String {
    format!("{USERS}/{name}")
}

/// The app's index with more files and without some, as the watcher would send it.
fn changed_index(app: &App, add: &[IndexFile], remove: &[&str]) -> Index {
    let mut index = app.index.clone();
    index.files.retain(|f| !remove.contains(&f.path.as_str()));
    index.files.extend(add.iter().cloned());
    index
}

fn update(index: Index, changed: &[&str]) -> Update {
    Update {
        index,
        changed: changed.iter().map(|p| p.to_string()).collect(),
        head_moved: false,
    }
}

fn tiles_by_path(app: &App) -> HashMap<String, (i32, i32)> {
    app.tree
        .files
        .iter()
        .zip(&app.layout.tiles)
        .map(|(f, &t)| (f.path.clone(), t))
        .collect()
}

fn new_file(p: &str, lines: u64) -> IndexFile {
    IndexFile {
        lines,
        untracked: true,
        ..IndexFile::new(p)
    }
}

#[test]
fn an_added_file_moves_only_tiles_in_its_directory_and_keeps_the_selection_in_place() {
    let (mut t, mut app) = quayside(150, 45);
    let selected = path("roles.clj");
    app.select(Entry::File(file(&app, &selected)), true);
    draw(&mut t, &mut app);
    let before = tiles_by_path(&app);
    let screen_before = screen(&app, app.layout.tiles[file(&app, &selected)]);

    let added = path("export.clj");
    let index = changed_index(&app, &[new_file(&added, 12)], &[]);
    app.apply_index(update(index, &[]));
    draw(&mut t, &mut app);

    let after = tiles_by_path(&app);
    let mut moved: Vec<&str> = before
        .iter()
        .filter(|(p, xy)| after[p.as_str()] != **xy)
        .map(|(p, _)| p.as_str())
        .collect();
    moved.sort();
    // The files after the new name shift by one slot. No other tile moves.
    assert_eq!(
        moved,
        [path("invite.clj"), path("roles.clj"), path("sessions.clj")]
    );
    assert_eq!(app.selected_path().as_deref(), Some(selected.as_str()));
    let f = file(&app, &selected);
    assert_eq!(screen(&app, app.layout.tiles[f]), screen_before);
    assert_eq!(
        app.flash_text(),
        Some("1 added · 0 removed · 3 tiles moved")
    );
    assert!(
        app.tree_rows()
            .iter()
            .any(|r| r.entry == Entry::File(file(&app, &added)))
    );
}

#[test]
fn an_update_without_a_file_set_change_moves_no_tile_and_flashes_nothing() {
    let (mut t, mut app) = quayside(150, 45);
    let before = tiles_by_path(&app);
    let cam = (app.cam_x, app.cam_y);
    let mut index = app.index.clone();
    let k = index
        .files
        .iter()
        .position(|f| f.path == path("roles.clj"))
        .unwrap();
    let lines = index.files[k].lines + 40;
    index.files[k].lines = lines;
    app.apply_index(update(index, &[path("roles.clj").as_str()]));
    draw(&mut t, &mut app);
    assert_eq!(tiles_by_path(&app), before);
    assert_eq!((app.cam_x, app.cam_y), cam);
    assert_eq!(app.flash_text(), None);
    let f = file(&app, &path("roles.clj"));
    assert_eq!(app.index.files[f].lines, lines);
}

#[test]
fn a_removed_selected_file_selects_its_directory() {
    let (mut t, mut app) = quayside(150, 45);
    let gone = path("roles.clj");
    app.select(Entry::File(file(&app, &gone)), true);
    let index = changed_index(&app, &[], &[&gone]);
    app.apply_index(update(index, &[]));
    draw(&mut t, &mut app);
    assert_eq!(app.selection(), Some(Entry::Dir(node(&app, USERS))));
    assert!(app.tree.files.iter().all(|f| f.path != gone));
    assert!(
        app.flash_text()
            .unwrap()
            .starts_with("0 added · 1 removed · ")
    );
}

#[test]
fn open_directories_stay_open_by_path() {
    let (mut t, mut app) = quayside(150, 45);
    let dir = node(&app, "bases/console/src/quayside/console/views/berth");
    app.select(Entry::Dir(dir), true);
    press(&mut app, KeyCode::Enter);
    assert!(app.filetree.is_open(dir));
    let index = changed_index(&app, &[new_file("aaa-new.md", 3)], &[]);
    app.apply_index(update(index, &[]));
    draw(&mut t, &mut app);
    let dir = node(&app, "bases/console/src/quayside/console/views/berth");
    assert!(app.filetree.is_open(dir));
    assert_eq!(app.selection(), Some(Entry::Dir(dir)));
}

/// The app with file text in memory for one of the `users` files, and the viewer open on it at `line` (0-based).
fn viewing(
    name: &str,
    lines: usize,
    line: usize,
) -> (ratatui::Terminal<ratatui::backend::TestBackend>, App) {
    let (mut t, mut app) = quayside(150, 45);
    let text: Vec<String> = (1..=lines).map(|n| format!("(def v{n} {n})")).collect();
    app.content = Content::memory(HashMap::from([(path(name), text.join("\n"))]));
    let f = file(&app, &path(name));
    app.open_viewer(f, Some(line), false);
    assert!(app.viewer.is_some(), "{:?}", app.flash_text());
    draw(&mut t, &mut app);
    (t, app)
}

#[test]
fn a_viewer_on_a_removed_file_closes_with_a_flash() {
    let (mut t, mut app) = viewing("roles.clj", 30, 5);
    let gone = path("roles.clj");
    let index = changed_index(&app, &[], &[&gone]);
    app.apply_index(update(index, &[]));
    draw(&mut t, &mut app);
    assert!(app.viewer.is_none());
    assert_eq!(
        app.flash_text(),
        Some(format!("{gone} was deleted.").as_str())
    );
}

#[test]
fn a_viewer_on_a_changed_file_reloads_and_keeps_its_line() {
    let (mut t, mut app) = viewing("roles.clj", 50, 19);
    let p = path("roles.clj");
    let text: Vec<String> = (1..=60).map(|n| format!("(def w{n} {n})")).collect();
    app.content = Content::memory(HashMap::from([(p.clone(), text.join("\n"))]));
    let index = app.index.clone();
    app.apply_index(update(index, &[p.as_str()]));
    app.wait_for_colors(Duration::from_secs(5));
    draw(&mut t, &mut app);
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!(vw.cursor_line(), 19);
    assert_eq!(vw.lines.len(), 60);
    assert_eq!(vw.lines[19], "(def w20 20)");
    assert!(vw.diff.is_some(), "a new diff after the reload");

    // A shorter file clamps the cursor to its last line.
    let text: Vec<String> = (1..=8).map(|n| format!("(def u{n} {n})")).collect();
    app.content = Content::memory(HashMap::from([(p.clone(), text.join("\n"))]));
    let index = app.index.clone();
    app.apply_index(update(index, &[p.as_str()]));
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!(vw.cursor_line(), 7);
    assert_eq!(vw.lines[7], "(def u8 8)");
}

#[test]
fn a_viewer_follows_its_file_to_its_new_index() {
    let (mut t, mut app) = viewing("roles.clj", 30, 3);
    let index = changed_index(&app, &[new_file("aaa-first.md", 1)], &[]);
    app.apply_index(update(index, &[]));
    draw(&mut t, &mut app);
    let vw = app.viewer.as_ref().unwrap();
    assert_eq!(vw.file, file(&app, &path("roles.clj")));
    assert_eq!(vw.cursor_line(), 3);
}

#[test]
fn a_kept_search_keeps_its_files_by_path() {
    let (mut t, mut app) = quayside(150, 45);
    keys(&mut app, "/tariff");
    press(&mut app, KeyCode::Enter);
    let kept: Vec<String> = app
        .search
        .as_ref()
        .unwrap()
        .kept
        .iter()
        .map(|&f| app.tree.files[f].path.clone())
        .collect();
    assert!(!kept.is_empty());
    let index = changed_index(&app, &[new_file("aaa-first.md", 1)], &[]);
    app.apply_index(update(index, &[]));
    draw(&mut t, &mut app);
    let after: Vec<String> = app
        .search
        .as_ref()
        .unwrap()
        .kept
        .iter()
        .map(|&f| app.tree.files[f].path.clone())
        .collect();
    assert_eq!(after, kept);
}

#[test]
fn the_panel_notes_an_untracked_file() {
    let (mut t, mut app) = quayside(150, 45);
    let added = path("export.clj");
    let index = changed_index(&app, &[new_file(&added, 12)], &[]);
    app.apply_index(update(index, &[]));
    app.select(Entry::File(file(&app, &added)), true);
    draw(&mut t, &mut app);
    let panel: Vec<String> = (0..45).map(|y| row(&t, y)).collect();
    assert!(
        panel.iter().any(|r| r.contains("12 lines · untracked")),
        "{panel:#?}"
    );
}
