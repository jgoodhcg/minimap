//! File tree tests on the fake `quayside` repo (units 017 and 018).

mod common;

use common::*;
use minimap_tui::App;
use minimap_tui::filetree::Entry;

#[test]
fn rows_are_directories_then_files_in_grid_order() {
    let (_, mut app) = quayside(160, 45);
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, false);
    }
    let rows = app.tree_rows();
    let labels: Vec<String> = rows.iter().map(|r| label(&app, r)).collect();
    let dirs: Vec<String> = app.tree.nodes[0]
        .kids
        .iter()
        .map(|&k| format!("{}/", app.tree.nodes[k].name))
        .collect();
    assert_eq!(labels[..dirs.len()], dirs[..]);
    assert!(
        rows[..dirs.len()]
            .iter()
            .all(|r| matches!(r.entry, Entry::Dir(_)) && r.depth == 0)
    );
    // Root files follow in grid order, so the dotfiles come last.
    let files: Vec<&str> = labels[dirs.len()..].iter().map(String::as_str).collect();
    assert_eq!(files.first(), Some(&"AGENTS.md"));
    assert_eq!(
        &files[files.len() - 3..],
        [".editorconfig", ".gitignore", ".tool-versions"]
    );

    // An open directory lists its child directories one level deeper, then its files.
    app.filetree.set_open(node(&app, "components"), true);
    app.filetree.set_open(node(&app, "components/vessel"), true);
    let rows = app.tree_rows();
    let vessel = rows
        .iter()
        .position(|r| r.entry == Entry::Dir(node(&app, "components/vessel")))
        .unwrap();
    let after: Vec<String> = rows[vessel + 1..vessel + 5]
        .iter()
        .map(|r| label(&app, r))
        .collect();
    assert_eq!(
        after,
        [
            "    resources/vessel/",
            "    src/quayside/vessel/",
            "    test/quayside/vessel/",
            "    deps.edn"
        ]
    );
}

#[test]
fn counts_include_subdirectories() {
    let (_, app) = quayside(160, 45);
    for (n, d) in app.tree.nodes.iter().enumerate().skip(1) {
        let want = app
            .tree
            .files
            .iter()
            .filter(|f| !f.ignored && f.path.starts_with(&format!("{}/", d.path)))
            .count();
        assert_eq!(app.filetree.count(n, true), want, "{}", d.path);
    }
    // The ignored `bases/api/resources/api/config.local.edn` does not count.
    assert_eq!(app.filetree.count(node(&app, "bases"), true), 558);
}

#[test]
fn opening_and_closing_everything_never_changes_the_layout() {
    let (mut t, mut app) = quayside(160, 45);
    let tiles = app.layout.tiles.clone();
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, true);
    }
    draw(&mut t, &mut app);
    for n in 0..app.tree.nodes.len() {
        app.filetree.set_open(n, false);
    }
    draw(&mut t, &mut app);
    assert_eq!(app.layout.tiles, tiles);
}

#[test]
fn hidden_rows_disappear_while_hidden() {
    let (_, mut app) = quayside(160, 45);
    let hidden_rows = |app: &App| {
        app.tree_rows()
            .iter()
            .filter(|r| match r.entry {
                Entry::Dir(n) => app.tree.nodes[n].hidden,
                Entry::File(f) => app.tree.files[f].hidden,
            })
            .count()
    };
    assert!(hidden_rows(&app) > 0);
    keys(&mut app, ".");
    assert_eq!(hidden_rows(&app), 0);
    let vscode = node(&app, ".vscode");
    assert_eq!(
        app.filetree.count(node(&app, "bases"), false),
        app.filetree.count(node(&app, "bases"), true)
    );
    assert!(
        app.tree_rows()
            .iter()
            .all(|r| r.entry != Entry::Dir(vscode))
    );
}

#[test]
fn the_tree_scrolls_to_the_selection() {
    let (_, mut app) = quayside(160, 20);
    let h = app.filetree.height;
    assert_eq!(h, 16);
    for _ in 0..40 {
        keys(&mut app, "j");
        let i = app.filetree.cursor_index(&app.tree_rows()).unwrap();
        assert!(
            i >= app.filetree.top && i < app.filetree.top + h,
            "row {i} is off screen"
        );
    }
    // A selection far down the tree, made on the map, scrolls the tree to it.
    app.select(
        Entry::File(file(
            &app,
            "components/berth/test/quayside/berth/rules_test.clj",
        )),
        true,
    );
    let i = app.filetree.cursor_index(&app.tree_rows()).unwrap();
    assert!(i >= app.filetree.top && i < app.filetree.top + h);
}
