//! The keymap (unit 033): key strings, the config merge, remapped keys in the app and the viewer, and `minimap keys`.

mod common;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use common::*;
use minimap_core::{Config, Content, Details};
use minimap_tui::App;
use minimap_tui::app::PAN_X;
use minimap_tui::filetree::Entry;
use minimap_tui::keymap::{Command, Key, Keymap, Lookup, Mode, parse_seq, seq_label};
use minimap_tui::viewer::Viewer;

fn keymap(toml: &str) -> Result<Keymap, Vec<String>> {
    Keymap::from_config(&Config::parse(toml).unwrap().keys)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Key {
    Key::new(code, mods)
}

const NONE: KeyModifiers = KeyModifiers::NONE;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

fn ch(c: char) -> Key {
    key(KeyCode::Char(c), NONE)
}

#[test]
fn key_strings_parse() {
    let one = |s: &str| {
        let seq = parse_seq(s).unwrap_or_else(|e| panic!("{s}: {e}"));
        assert_eq!(seq.len(), 1, "{s}");
        seq[0]
    };
    assert_eq!(one("a"), ch('a'));
    assert_eq!(one("G"), ch('G'));
    assert_eq!(one("shift+g"), ch('G'), "shift+g is G");
    assert_eq!(one("ctrl+d"), key(KeyCode::Char('d'), CTRL));
    assert_eq!(
        one("CTRL+D"),
        key(KeyCode::Char('d'), CTRL),
        "case does not count with ctrl"
    );
    assert_eq!(one("alt+x"), key(KeyCode::Char('x'), KeyModifiers::ALT));
    assert_eq!(one("option+x"), one("alt+x"));
    assert_eq!(one("shift+tab"), key(KeyCode::BackTab, NONE));
    assert_eq!(one("enter"), key(KeyCode::Enter, NONE));
    assert_eq!(one("esc"), Key::ESC);
    assert_eq!(one("escape"), Key::ESC);
    assert_eq!(one("space"), ch(' '));
    assert_eq!(one("f5"), key(KeyCode::F(5), NONE));
    assert_eq!(one("plus"), ch('+'));
    assert_eq!(one("+"), ch('+'));
    assert_eq!(one("/"), ch('/'));
    assert_eq!(
        one("ctrl+shift+left"),
        key(KeyCode::Left, CTRL | KeyModifiers::SHIFT)
    );
    assert_eq!(parse_seq("g g").unwrap(), vec![ch('g'), ch('g')]);
    assert_eq!(parse_seq("] c").unwrap(), vec![ch(']'), ch('c')]);
    assert_eq!(parse_seq("space f").unwrap(), vec![ch(' '), ch('f')]);
    for bad in ["", " ", "ctrl+", "hyper+x", "foo", "shift+/", "f13"] {
        assert!(parse_seq(bad).is_err(), "{bad:?} should not parse");
    }
}

#[test]
fn key_events_match_parsed_keys() {
    // Terminals report Shift with an uppercase letter, and Shift with BackTab.
    let ev = |code, mods| Key::from_event(&KeyEvent::new(code, mods));
    assert_eq!(ev(KeyCode::Char('G'), KeyModifiers::SHIFT), ch('G'));
    assert_eq!(
        ev(KeyCode::BackTab, KeyModifiers::SHIFT),
        parse_seq("shift+tab").unwrap()[0]
    );
    assert_eq!(
        ev(KeyCode::Char('d'), CTRL),
        parse_seq("ctrl+d").unwrap()[0]
    );
    assert_eq!(
        ev(KeyCode::Char('D'), CTRL | KeyModifiers::SHIFT),
        parse_seq("ctrl+d").unwrap()[0]
    );
}

#[test]
fn keys_show_in_their_hint_form() {
    let label = |s: &str| seq_label(&parse_seq(s).unwrap(), false);
    assert_eq!(label("ctrl+d"), "^d");
    assert_eq!(label("enter"), "⏎");
    assert_eq!(seq_label(&parse_seq("enter").unwrap(), true), "enter");
    assert_eq!(label("left"), "←");
    assert_eq!(label("g g"), "gg");
    assert_eq!(label("] c"), "]c");
    assert_eq!(label("space f"), "space f");
    assert_eq!(label("esc"), "esc");
}

#[test]
fn the_default_keymap_is_valid_and_binds_every_action() {
    let k = keymap("").unwrap();
    for mode in Mode::ALL {
        let t = k.table(mode);
        let bound: HashSet<Command> = t.bindings().iter().map(|(_, c)| *c).collect();
        let toml = k.to_toml();
        assert!(
            !toml.contains("# no key:"),
            "every default action has a key:\n{toml}"
        );
        assert!(!bound.is_empty(), "{mode:?}");
    }
    assert_eq!(k.map.lookup(&[ch('q')]), Lookup::Run(Command::Quit));
    assert_eq!(k.viewer.lookup(&[ch('g')]), Lookup::Wait);
    assert_eq!(
        k.viewer.lookup(&[ch('g'), ch('g')]),
        Lookup::Run(Command::FirstLine)
    );
    assert_eq!(k.viewer.lookup(&[ch('g'), ch('x')]), Lookup::None);
}

#[test]
fn the_config_changes_add_and_remove_bindings() {
    let k = keymap(
        r#"
        [keys.map]
        "h" = "pan_left"
        "a" = "none"
        "ctrl+l" = "pan_left"
        "#,
    )
    .unwrap();
    assert_eq!(k.map.lookup(&[ch('h')]), Lookup::Run(Command::PanLeft));
    assert_eq!(k.map.lookup(&[ch('a')]), Lookup::None);
    let keys: Vec<String> = k
        .map
        .keys(Command::PanLeft)
        .map(|s| seq_label(s, false))
        .collect();
    assert_eq!(
        keys,
        ["^l", "h"],
        "the config's keys come first, in key order"
    );
    assert_eq!(
        k.map.label(Command::TreeOut),
        "",
        "h was tree_out's only key"
    );
    // The defaults of other modes stay.
    assert_eq!(
        k.viewer.lookup(&[ch('h')]),
        Lookup::Run(Command::ScrollLeft)
    );
}

#[test]
fn config_errors_name_the_table_and_the_key() {
    let err = |toml: &str| keymap(toml).unwrap_err().join("\n");
    let e = err("[keys.mapp]\n\"h\" = \"pan_left\"");
    assert!(e.contains("[keys.mapp]") && e.contains("not a mode"), "{e}");
    let e = err("[keys.map]\n\"h\" = \"pan_lft\"");
    assert!(
        e.contains("[keys.map] \"h\"") && e.contains("Did you mean pan_left?"),
        "{e}"
    );
    let e = err("[keys.map]\n\"h\" = \"half_page_down\"");
    assert!(e.contains("not an action of the map mode"), "{e}");
    let e = err("[keys.map]\n\"ctrl+\" = \"quit\"");
    assert!(e.contains("[keys.map] \"ctrl+\""), "{e}");
    let e = err("[keys.map]\n\"G\" = \"quit\"\n\"shift+g\" = \"help\"");
    assert!(e.contains("the same key"), "{e}");
    let e = err("[keys.map]\n\"g g\" = \"first_row\"");
    assert!(
        e.contains("\"g\" (first_row) is bound and also starts \"g g\""),
        "{e}"
    );
    let e = err("[keys.viewer]\n\"5\" = \"down\"");
    assert!(
        e.contains("[keys.viewer] \"5\"") && e.contains("count"),
        "{e}"
    );
    assert!(keymap("[keys.map]\n\"g\" = \"none\"\n\"g g\" = \"first_row\"").is_ok());
    assert!(
        keymap("[keys.viewer]\n\"0\" = \"none\"").is_ok(),
        "0 is not a count digit"
    );
}

#[test]
fn minimap_keys_output_parses_back_to_the_same_keymap() {
    for toml in [
        "",
        "[keys.map]\n\"h\" = \"pan_left\"\n\"a\" = \"none\"\n[keys.viewer]\n\"space f\" = \"find\"\n\"\\\"\" = \"help\"",
    ] {
        let k = keymap(toml).unwrap();
        let printed = k.to_toml();
        let again = keymap(&printed).unwrap_or_else(|e| panic!("{e:?}\n{printed}"));
        for mode in Mode::ALL {
            let set =
                |k: &Keymap| -> HashSet<_> { k.table(mode).bindings().iter().cloned().collect() };
            assert_eq!(set(&k), set(&again), "{mode:?}\n{printed}");
        }
    }
    let printed = keymap("[keys.map]\n\"a\" = \"none\"\n\"h\" = \"none\"")
        .unwrap()
        .to_toml();
    assert!(printed.contains("\"a\" = \"none\""), "{printed}");
    assert!(printed.contains("# no key: tree_out"), "{printed}");
}

fn with_keys(app: &mut App, toml: &str) {
    app.keymap = Arc::new(keymap(toml).unwrap());
}

#[test]
fn remapped_keys_run_their_actions_in_the_map() {
    let (_t, mut app) = quayside(150, 45);
    with_keys(&mut app, "[keys.map]\n\"h\" = \"pan_left\"");
    keys(&mut app, "dd");
    let x = app.cam_x;
    keys(&mut app, "h");
    assert_eq!(app.cam_x, x - PAN_X, "h pans left");
    keys(&mut app, "a");
    assert_eq!(app.cam_x, x - 2 * PAN_X, "a still pans left");

    with_keys(&mut app, "[keys.map]\n\"a\" = \"none\"");
    let x = app.cam_x;
    keys(&mut app, "a");
    assert_eq!(app.cam_x, x, "a does nothing");

    // A sequence in the map: the first key waits, the second runs.
    with_keys(
        &mut app,
        "[keys.map]\n\"g\" = \"none\"\n\"g g\" = \"first_row\"",
    );
    keys(&mut app, "jjj");
    let first = app.tree_rows()[0].entry;
    keys(&mut app, "g");
    assert_ne!(app.selection(), Some(first), "g alone waits");
    keys(&mut app, "g");
    assert_eq!(app.selection(), Some(first));

    with_keys(&mut app, "[keys.map]\n\"f1\" = \"help\"\n\"?\" = \"none\"");
    press(&mut app, KeyCode::F(1));
    assert!(app.help);
    press(&mut app, KeyCode::F(1));
    assert!(!app.help, "the help key closes the help");
    keys(&mut app, "?");
    assert!(!app.help, "? no longer opens the help");
}

fn hundred(keys: &str) -> Viewer {
    let text: Vec<String> = (1..=100).map(|n| format!("line {n}")).collect();
    let mut v = Viewer::new(0, "src/a.clj", &text.join("\n"));
    v.set_height(20);
    v.keys = Arc::new(keymap(keys).unwrap());
    v
}

const OUTLINE: [u32; 5] = [3, 25, 40, 71, 96];

fn typed(v: &mut Viewer, s: &str) {
    for c in s.chars() {
        v.key(KeyEvent::new(KeyCode::Char(c), NONE), &OUTLINE);
    }
}

#[test]
fn remapped_keys_run_their_actions_in_the_viewer() {
    let mut v = hundred(
        "[keys.viewer]\n\"J\" = \"half_page_down\"\n\"space f\" = \"find\"\n\"ctrl+n\" = \"down\"",
    );
    typed(&mut v, "J");
    assert_eq!(v.cursor, 10, "J moves half a page");
    typed(&mut v, " ");
    assert!(v.prompt.is_none(), "space waits for f");
    typed(&mut v, "f");
    assert!(v.prompt.is_some(), "space f opens the find prompt");
    v.key(KeyEvent::new(KeyCode::Esc, NONE), &OUTLINE);
    typed(&mut v, "gg5");
    v.key(KeyEvent::new(KeyCode::Char('n'), CTRL), &OUTLINE);
    assert_eq!(v.cursor, 5, "a count applies to a remapped key");
    typed(&mut v, "3]]");
    assert_eq!(
        v.cursor, 70,
        "a count applies to a sequence: the third definition after line 6 is line 71"
    );
}

/// Quayside with the berth fixture, and keys that move help to F1, the search and find to Ctrl+F, and drop `4`.
fn remapped_quayside() -> (ratatui::Terminal<ratatui::backend::TestBackend>, App) {
    let (mut t, mut app) = quayside(150, 45);
    with_keys(
        &mut app,
        r#"
        [keys.map]
        "f1" = "help"
        "?" = "none"
        "ctrl+f" = "search"
        "/" = "none"
        "4" = "none"
        [keys.viewer]
        "ctrl+f" = "find"
        "/" = "none"
        "#,
    );
    let path = "components/berth/src/quayside/berth/core.clj";
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/clojure/berth.clj"
    ))
    .unwrap();
    let files = HashMap::from([(path.to_string(), text)]);
    app.content = Content::memory(files.clone());
    app.details = Details::memory(files);
    let f = file(&app, path);
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    (t, app)
}

#[test]
fn remapped_keys_show_in_the_help_and_the_hints() {
    let (mut t, mut app) = remapped_quayside();
    insta::assert_snapshot!("remapped_map_status", row(&t, 44));
    press(&mut app, KeyCode::F(1));
    draw(&mut t, &mut app);
    insta::assert_snapshot!("remapped_help", t.backend());
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    app.wait_for_colors(Duration::from_secs(10));
    draw(&mut t, &mut app);
    insta::assert_snapshot!("remapped_viewer_status", row(&t, 44));
    app.on_key(KeyEvent::new(KeyCode::Char('f'), CTRL));
    assert!(
        app.viewer.as_ref().unwrap().prompt.is_some(),
        "ctrl+f finds in the viewer"
    );
}

#[test]
fn a_config_with_a_bad_key_fails_with_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    std::fs::write(&config, "[keys.map]\n\"h\" = \"pan_lft\"\n").unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_minimap"))
        .args(["keys", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("[keys.map] \"h\" = \"pan_lft\""), "{err}");
    // The map itself stops before the terminal changes: nothing reaches stdout.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_minimap"))
        .arg("--config")
        .arg(&config)
        .arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "no screen output");
    assert!(String::from_utf8_lossy(&out.stderr).contains("pan_lft"));
    let empty = BTreeMap::new();
    assert!(Keymap::from_config(&empty).is_ok());
}
