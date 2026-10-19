//! The shape, the outline, and the namespace in the selection panel (unit `007`).

mod common;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::KeyCode;

use common::*;
use minimap_tui::App;
use minimap_tui::filetree::Entry;
use minimap_tui::theme;

const CORE: &str = "components/berth/src/quayside/berth/core.clj";

/// The app at 150 x 45 with `berth/core.clj` selected.
fn berth_core() -> (Terminal<TestBackend>, App) {
    let (mut t, mut app) = quayside_details(150, 45);
    let f = file(&app, CORE);
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    (t, app)
}

/// The panel's text rows, from the rule under the repo stats to the key hint.
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

fn panel_row(t: &Terminal<TestBackend>, app: &App, starts: &str) -> Option<usize> {
    panel(t, app).iter().position(|r| r.starts_with(starts))
}

#[test]
fn panel_shape_and_outline() {
    let (t, _) = berth_core();
    insta::assert_snapshot!(t.backend());
}

#[test]
fn panel_outline_browse() {
    let (mut t, mut app) = berth_core();
    keys(&mut app, "ojjj");
    draw(&mut t, &mut app);
    assert_eq!(app.outline_browse(), Some(3));
    insta::assert_snapshot!(t.backend());
}

#[test]
fn the_panel_shows_the_namespace_under_the_directory() {
    let (t, app) = berth_core();
    let p = panel(&t, &app);
    let dir = panel_row(&t, &app, "components/berth/src/quayside/berth/").unwrap();
    assert_eq!(p[dir + 1], "quayside.berth.core");
    assert_eq!(p[dir + 2], "547 lines");
    assert!(panel_row(&t, &app, "SHAPE").is_some());
    let outline = panel_row(&t, &app, "OUTLINE · o to browse").unwrap();
    assert!(p[outline].ends_with("43"), "item count: {}", p[outline]);
    assert!(p[outline + 1].starts_with("   9  allocate-berth-window"));
}

#[test]
fn shape_rows_have_line_numbers_and_definition_markers() {
    let (t, app) = berth_core();
    let v = app.view();
    let x0 = (v.cols - v.rw + 2) as usize;
    let shape = panel_row(&t, &app, "SHAPE").unwrap() + 1;
    let rows: Vec<String> = (shape..shape + 12)
        .map(|y| row(&t, (y + 1) as u16).chars().skip(x0).collect())
        .collect();
    assert!(
        rows[0].trim_start().starts_with("1 "),
        "first row number: {:?}",
        rows[0]
    );
    let last_numbered = rows
        .iter()
        .rposition(|r| r.trim_start().starts_with("547"))
        .unwrap();
    assert!(last_numbered > 0);
    // Braille in the grid, and `·` markers right of it.
    assert!(
        rows.iter()
            .any(|r| r.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)))
    );
    assert!(rows.iter().filter(|r| r.contains('·')).count() >= 4);
}

#[test]
fn o_browses_and_j_k_move_within_the_outline() {
    let (mut t, mut app) = berth_core();
    press(&mut app, KeyCode::Char('o'));
    assert_eq!(app.outline_browse(), Some(0));
    keys(&mut app, "k");
    assert_eq!(app.outline_browse(), Some(0), "k stops at the first item");
    for _ in 0..60 {
        press(&mut app, KeyCode::Down);
    }
    assert_eq!(
        app.outline_browse(),
        Some(42),
        "Down stops at the last item"
    );
    let sel = app.selection();
    // The tree does not move while the outline is browsed.
    keys(&mut app, "jjhl");
    assert_eq!(app.selection(), sel);
    draw(&mut t, &mut app);
    let status = row(&t, 44);
    assert!(
        status.starts_with(" OUTLINE  j/k move · ⏎ view at line · e edit · esc back"),
        "{status}"
    );
    // The highlighted last item scrolled into view, with a white `◀` at its shape row.
    let p = panel(&t, &app);
    let last = app.file_details(file(&app, CORE)).unwrap().outline[42].clone();
    let want = format!("{:>4}  {}", last.line, last.name);
    assert!(
        p.iter().any(|r| r.starts_with(&want)),
        "{want:?} is visible"
    );
    let hi = (1..44u16)
        .find(|&y| {
            let b = t.backend().buffer();
            (0..150u16).any(|x| b[(x, y)].symbol() == "◀" && b[(x, y)].fg == theme::WHITE)
        })
        .expect("a ◀ marker");
    assert!(hi > 1);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.outline_browse(), None);
    keys(&mut app, "oo");
    assert_eq!(app.outline_browse(), None, "o leaves too");
}

#[test]
fn the_highlighted_item_stays_visible_while_scrolling() {
    let (mut t, mut app) = berth_core();
    press(&mut app, KeyCode::Char('o'));
    let rows: Vec<String> = app
        .file_details(file(&app, CORE))
        .unwrap()
        .outline
        .iter()
        .map(|o| format!("{:>4}  {}", o.line, o.name))
        .collect();
    for (k, want) in rows.iter().enumerate() {
        draw(&mut t, &mut app);
        assert_eq!(app.outline_browse(), Some(k));
        let p = panel(&t, &app);
        assert!(
            p.iter().any(|r| r.starts_with(want)),
            "item {k} {want:?} is visible"
        );
        keys(&mut app, "j");
    }
}

#[test]
fn enter_in_the_outline_views_the_file_or_says_why_not() {
    // The fixture has no file text, so the viewer cannot open (unit 029), and browsing goes on.
    let (_, mut app) = berth_core();
    keys(&mut app, "o");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.flash_text(),
        Some("No file contents in --from-json mode.")
    );
    assert_eq!(app.outline_browse(), Some(0));
    assert!(app.viewer.is_none());
}

#[test]
fn a_selection_change_leaves_the_outline() {
    let (mut t, mut app) = berth_core();
    keys(&mut app, "o");
    let other = file(&app, "components/berth/src/quayside/berth/db.clj");
    app.select(Entry::File(other), true);
    draw(&mut t, &mut app);
    assert_eq!(app.outline_browse(), None);
    assert!(!row(&t, 44).starts_with(" OUTLINE"));
}

#[test]
fn o_explains_when_it_cannot_browse() {
    let (mut t, mut app) = quayside_details(150, 45);
    // The start file has no details in the fixture.
    keys(&mut app, "o");
    assert_eq!(app.flash_text(), Some("This file has no outline."));
    assert_eq!(app.outline_browse(), None);
    let f = file(&app, CORE);
    app.select(Entry::File(f), true);
    keys(&mut app, "p");
    draw(&mut t, &mut app);
    keys(&mut app, "o");
    assert_eq!(
        app.flash_text(),
        Some("The outline is in the selection panel. Press p to show it.")
    );
    keys(&mut app, "h");
    keys(&mut app, "o");
    assert_eq!(
        app.flash_text(),
        Some("Select a file to browse its outline.")
    );
}

#[test]
fn shift_y_copies_the_namespace() {
    let (_, mut app) = berth_core();
    let copies = record_copies(&mut app);
    keys(&mut app, "Y");
    assert_eq!(copies.borrow().as_slice(), ["quayside.berth.core"]);
    assert_eq!(
        app.flash_text(),
        Some("Copied namespace: quayside.berth.core")
    );
    let readme = file(&app, "README.md");
    app.select(Entry::File(readme), true);
    keys(&mut app, "Y");
    assert_eq!(copies.borrow().len(), 1);
    assert_eq!(
        app.flash_text(),
        Some("This file has no namespace. Press y to copy its path.")
    );
}

#[test]
fn private_items_are_faint() {
    let (t, app) = berth_core();
    let v = app.view();
    let x0 = (v.cols - v.rw + 2) as u16;
    let b = t.backend().buffer();
    let at = |starts: &str| {
        let y = (1..v.rows as u16 - 1)
            .find(|&y| {
                row(&t, y)
                    .chars()
                    .skip(x0 as usize)
                    .collect::<String>()
                    .starts_with(starts)
            })
            .unwrap();
        b[(x0 + 6, y)].fg
    };
    assert_eq!(
        at("  18  release-berth-report"),
        theme::FAINT,
        "defn- is faint"
    );
    assert_eq!(at("  35  validate-berth"), theme::DIM, "defn is dim");
}
