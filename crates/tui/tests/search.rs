//! The file picker (unit 026): `/`, typing, the arrows, Enter, Esc, `n` / `N`, the list, and the marks on the map.

mod common;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use common::*;
use minimap_core::fuzzy::{Candidate, Pattern, rank};
use minimap_tui::App;
use minimap_tui::filetree::Entry;
use minimap_tui::{icons, theme};

fn ctrl(app: &mut App, c: char) {
    app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

fn results(app: &App) -> Vec<usize> {
    let s = app.search.as_ref().unwrap();
    s.results.iter().map(|r| r.index).collect()
}

/// The visible files ranked directly with the matcher, for comparison.
fn direct(app: &App, q: &str) -> Vec<usize> {
    let c: Vec<(usize, Candidate)> = (0..app.tree.files.len())
        .filter(|&f| app.file_visible(f))
        .map(|f| (f, Candidate::new(&app.tree.files[f].path)))
        .collect();
    let mut r: Vec<usize> = rank(c.iter().map(|(f, c)| (*f, c)), &Pattern::parse(q))
        .into_iter()
        .map(|r| r.index)
        .collect();
    r.truncate(40);
    r
}

fn marked(app: &App) -> Vec<usize> {
    let s = app.search.as_ref().unwrap();
    (0..app.tree.files.len())
        .filter(|&f| s.is_marked(f))
        .collect()
}

#[test]
fn slash_opens_the_picker_with_every_visible_file_and_nothing_highlighted() {
    let (mut t, mut app) = quayside(160, 45);
    let before = (app.selection(), app.cam_x, app.cam_y);
    keys(&mut app, "/");
    assert!(app.picker_open());
    let visible = (0..app.tree.files.len())
        .filter(|&f| app.file_visible(f))
        .count();
    assert_eq!(results(&app).len(), visible);
    let s = app.search.as_ref().unwrap();
    assert_eq!(s.cursor, None);
    assert!(marked(&app).is_empty(), "an empty query marks nothing");
    assert_eq!((app.selection(), app.cam_x, app.cam_y), before);
    draw(&mut t, &mut app);
    assert!(row(&t, 1).starts_with(" SEARCH"));
    let n = minimap_core::text::thousands(visible as u64);
    assert!(row(&t, 1).contains(&format!("{n} / {n}")));
    assert!(row(&t, 43).starts_with(" > ▏"));
    assert!(row(&t, 44).starts_with(" SEARCH  ↑↓ ^p ^n move · ⏎ keep · esc back "));
}

#[test]
fn typing_ranks_and_the_selection_follows_the_best_result() {
    let (_, mut app) = quayside(160, 45);
    keys(&mut app, "/rules");
    let got = results(&app);
    assert_eq!(got[..40], direct(&app, "rules")[..]);
    assert_eq!(app.selection(), Some(Entry::File(got[0])));
    assert!(app.tree.files[got[0]].name.contains("rules"));
    assert_selection_in_view(&app);
    // The tree does not open while the picker previews.
    let s = app.search.as_ref().unwrap();
    assert_eq!(s.cursor, Some(0));

    // Up moves to a worse result, Down back to a better one, and Down stops at the best.
    press(&mut app, KeyCode::Up);
    assert_eq!(app.selection(), Some(Entry::File(got[1])));
    ctrl(&mut app, 'p');
    assert_eq!(app.selection(), Some(Entry::File(got[2])));
    ctrl(&mut app, 'n');
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.selection(), Some(Entry::File(got[0])));
    assert_selection_in_view(&app);
}

#[test]
fn a_query_edits_like_a_prompt() {
    let (_, mut app) = quayside(160, 45);
    keys(&mut app, "/berth rules");
    ctrl(&mut app, 'w');
    assert_eq!(app.search.as_ref().unwrap().query, "berth ");
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.search.as_ref().unwrap().query, "berth");
    assert_eq!(results(&app)[..40], direct(&app, "berth")[..]);
    ctrl(&mut app, 'u');
    assert_eq!(app.search.as_ref().unwrap().query, "");
    // q, ?, j, and . are text in the prompt: no quit, no help, no tree move, no hidden toggle.
    keys(&mut app, "q?j.");
    assert!(!app.quit && !app.help && app.show_hidden);
    assert_eq!(app.search.as_ref().unwrap().query, "q?j.");
    ctrl(&mut app, 'c');
    assert!(app.quit);
}

#[test]
fn enter_keeps_the_result_and_n_and_shift_n_cycle_the_kept_rows() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "/rules");
    press(&mut app, KeyCode::Up);
    let got = results(&app);
    let rows = app.search.as_ref().unwrap().rows;
    press(&mut app, KeyCode::Enter);
    assert!(!app.picker_open());
    assert_eq!(app.selection(), Some(Entry::File(got[1])));
    let rows_now = app.tree_rows();
    assert!(
        app.filetree.cursor_index(&rows_now).is_some(),
        "the tree opens to the kept result"
    );
    let kept: Vec<usize> = got.iter().take(rows).copied().collect();
    assert_eq!(app.search.as_ref().unwrap().kept, kept);
    assert_eq!(marked(&app).len(), kept.len());
    keys(&mut app, "n");
    assert_eq!(app.selection(), Some(Entry::File(kept[2])));
    keys(&mut app, "NNN");
    assert_eq!(
        app.selection(),
        Some(Entry::File(kept[kept.len() - 1])),
        "N wraps"
    );
    draw(&mut t, &mut app);
    let status = row(&t, 44);
    assert!(
        status.starts_with(&format!(
            " /rules  {} of {} · n/N · esc clear ",
            kept.len(),
            got.len()
        )),
        "{status:?}"
    );
    // Esc clears the kept results and their marks.
    press(&mut app, KeyCode::Esc);
    assert!(app.search.is_none());
    keys(&mut app, "n");
    assert_eq!(app.selection(), Some(Entry::File(kept[kept.len() - 1])));
}

#[test]
fn esc_restores_the_selection_and_camera_from_before_the_picker() {
    let (_, mut app) = quayside(160, 45);
    app.pan(30, 10);
    let before = (app.selection(), app.cam_x, app.cam_y);
    keys(&mut app, "/rules");
    press(&mut app, KeyCode::Up);
    assert_ne!(app.selection(), before.0);
    press(&mut app, KeyCode::Esc);
    assert!(app.search.is_none());
    assert_eq!((app.selection(), app.cam_x, app.cam_y), before);
}

#[test]
fn the_marks_are_the_visible_rows_before_and_after_scrolling() {
    let (_, mut app) = quayside(160, 45);
    keys(&mut app, "/core");
    let rows = app.search.as_ref().unwrap().rows;
    let got = results(&app);
    assert!(got.len() > 2 * rows);
    let mut want: Vec<usize> = got[..rows].to_vec();
    want.sort();
    assert_eq!(marked(&app), want);
    press(&mut app, KeyCode::PageUp);
    press(&mut app, KeyCode::Up);
    let s = app.search.as_ref().unwrap();
    assert!(s.scroll > 0);
    let mut want: Vec<usize> = got[s.scroll..s.scroll + rows].to_vec();
    want.sort();
    assert_eq!(marked(&app), want);
}

#[test]
fn matched_characters_are_bold_in_the_search_color() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "/rules");
    draw(&mut t, &mut app);
    let s = app.search.as_ref().unwrap();
    let best = &s.results[0];
    let tf = &app.tree.files[best.index];
    let (y, h) = app.view().tree_rows();
    let ry = (y + h - 1) as u16;
    let b = t.backend().buffer();
    assert_eq!(
        b[(1, ry)].fg,
        icons::type_color(&tf.name, false),
        "the type icon"
    );
    let name_start = tf.path.chars().count() - tf.name.chars().count();
    for (k, ch) in tf.name.chars().enumerate() {
        let cell = &b[(3 + k as u16, ry)];
        assert_eq!(cell.symbol(), ch.to_string());
        let hit = best.positions.contains(&(name_start + k));
        assert_eq!(cell.fg == theme::SEARCH_HI, hit, "{ch} at {k}");
        assert_eq!(cell.modifier.contains(ratatui::style::Modifier::BOLD), hit);
        assert_eq!(cell.bg, theme::CURSOR, "the best row is highlighted");
    }
}

#[test]
fn with_the_tree_hidden_the_picker_shows_the_column_and_esc_hides_it() {
    let (_, mut app) = quayside(160, 45);
    let tiles = app.layout.tiles.clone();
    keys(&mut app, "t");
    assert_eq!(app.view().tw, 0);
    keys(&mut app, "/");
    assert!(app.view().tw > 0);
    keys(&mut app, "rules");
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.view().tw, 0);
    keys(&mut app, "/rules");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.view().tw, 0, "Enter hides it again too");
    assert_eq!(app.layout.tiles, tiles);
}

#[test]
fn hidden_files_are_results_only_while_shown() {
    let (_, mut app) = quayside(160, 45);
    keys(&mut app, "/.yml");
    let shown = results(&app);
    assert!(shown.iter().any(|&f| app.tree.files[f].hidden));
    press(&mut app, KeyCode::Esc);
    keys(&mut app, "./.yml");
    assert!(results(&app).iter().all(|&f| !app.tree.files[f].hidden));
    // A kept search drops results that the toggle hides.
    press(&mut app, KeyCode::Esc);
    keys(&mut app, "./.yml");
    press(&mut app, KeyCode::Enter);
    keys(&mut app, ".");
    let s = app.search.as_ref().unwrap();
    assert!(s.kept.iter().all(|&f| !app.tree.files[f].hidden));
}

#[test]
fn a_click_highlights_a_row_and_the_wheel_moves_the_highlight() {
    let (_, mut app) = quayside(160, 45);
    keys(&mut app, "/core");
    let got = results(&app);
    let (y, h) = app.view().tree_rows();
    // The third row from the bottom.
    click(&mut app, 5, y + h - 3);
    assert_eq!(app.selection(), Some(Entry::File(got[2])));
    mouse(&mut app, MouseEventKind::ScrollUp, 5, y + 3);
    assert_eq!(app.selection(), Some(Entry::File(got[5])));
    mouse(&mut app, MouseEventKind::ScrollDown, 5, y + 3);
    assert_eq!(app.selection(), Some(Entry::File(got[2])));
    assert!(app.picker_open());
}

#[test]
fn search_snapshots() {
    let (mut t, mut app) = quayside(160, 45);
    keys(&mut app, "/");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("picker_empty", t.backend());
    keys(&mut app, "rules");
    draw(&mut t, &mut app);
    insta::assert_snapshot!("picker_rules", t.backend());
    press(&mut app, KeyCode::Enter);
    draw(&mut t, &mut app);
    assert!(row(&t, 44).starts_with(" /rules  "));
    insta::assert_snapshot!("picker_after_enter", t.backend());
}

#[test]
fn a_kept_search_with_no_result_still_shows_in_the_status_bar() {
    let (mut t, mut app) = quayside(160, 45);
    let before = app.selection();
    keys(&mut app, "/zzzz");
    assert!(results(&app).is_empty());
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.selection(), before);
    draw(&mut t, &mut app);
    assert!(
        row(&t, 44).starts_with(" /zzzz  0 of 0 · n/N · esc clear "),
        "{:?}",
        row(&t, 44)
    );
    press(&mut app, KeyCode::Esc);
    draw(&mut t, &mut app);
    assert!(row(&t, 44).starts_with(" 1 ● trail  4 ○ age  5 ○ churn  . ● hidden "));
}
