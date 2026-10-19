//! Outlines for every language from the grammars' definition scopes (unit 032).

mod common;

use std::collections::HashMap;
use std::time::Duration;

use ratatui::crossterm::event::KeyCode;

use common::*;
use minimap_core::{Content, Details};
use minimap_tui::filetree::Entry;
use minimap_tui::syntax::Highlighter;

const RUST: &str = "use std::fmt;\n\npub struct Tile {\n    pub x: i32,\n}\n\npub enum Order {\n    Tallest,\n}\n\npub trait Draw {\n    fn draw(&self);\n}\n\nimpl Tile {\n    pub fn new() -> Tile {\n        Tile { x: 0 }\n    }\n}\n\nconst MAX: usize = 4;\n\nmacro_rules! twice {\n    ($e:expr) => { $e; $e };\n}\n\nfn main() {\n    let t = Tile::new();\n    println!(\"{}\", t.x);\n}\n";
const TS: &str = "const f = Cypress.expose(\"x\");\n\ninterface Params {\n  a: string;\n}\n\nfunction visit(p: Params): Result {\n  cy.go(p);\n  return new Result();\n}\n\nclass Page extends Base {\n  open(x: Options) {\n    this.load();\n  }\n}\n\ntype Alias = Base;\n\nenum Color { Red }\n\nexport const handler = (y: Y) => y;\n";
const PY: &str = "import os\n\nclass Store:\n    def get(self, key):\n        return os.environ.get(key)\n\ndef main():\n    print(Store().get(\"HOME\"))\n";
const JS: &str = "function parse(spec) {\n  return spec.split(\",\");\n}\n\nclass List {\n  add(x) {\n    this.items.push(x);\n  }\n}\n";
const SQL: &str = "create or replace function f_strip_html(s varchar)\nreturns varchar\nas $$ select regexp_replace(s, '<[^>]*>', '') $$;\n\ncreate table users (id int);\n";
const MD: &str = "# Title\n\nText.\n\n## Part one\n\n### Detail\n\n## Part two\n";

/// `(line, kind, name, depth)` of each item.
fn items(path: &str, text: &str) -> Vec<(u32, String, String, u8)> {
    let (h, _) = Highlighter::new(None);
    h.outline(path, text)
        .into_iter()
        .map(|o| (o.line, o.kind, o.name, o.depth))
        .collect()
}

fn item(line: u32, kind: &str, name: &str, depth: u8) -> (u32, String, String, u8) {
    (line, kind.to_string(), name.to_string(), depth)
}

#[test]
fn rust_items_nest_methods_under_impl_and_trait() {
    assert_eq!(
        items("a.rs", RUST),
        [
            item(3, "struct", "Tile", 0),
            item(7, "enum", "Order", 0),
            item(11, "trait", "Draw", 0),
            item(12, "fn", "draw", 1),
            item(15, "impl", "impl Tile", 0),
            item(16, "fn", "new", 1),
            item(23, "macro", "twice", 0),
            item(27, "fn", "main", 0),
        ]
    );
}

#[test]
fn typescript_leaves_out_calls_and_type_references() {
    // `Cypress.expose(…)`, `cy.go(…)`, `new Result()`, `this.load()`, and the annotated types are not items.
    assert_eq!(
        items("a.ts", TS),
        [
            item(3, "interface", "Params", 0),
            item(7, "fn", "visit", 0),
            item(12, "class", "Page", 0),
            item(13, "fn", "open", 1),
            item(18, "type", "Alias", 0),
            item(20, "enum", "Color", 0),
            item(22, "fn", "handler", 0),
        ]
    );
}

#[test]
fn python_javascript_and_sql_definitions() {
    assert_eq!(
        items("a.py", PY),
        [
            item(3, "class", "Store", 0),
            item(4, "fn", "get", 1),
            item(7, "fn", "main", 0)
        ]
    );
    assert_eq!(
        items("a.js", JS),
        [
            item(1, "fn", "parse", 0),
            item(5, "class", "List", 0),
            item(6, "fn", "add", 1)
        ]
    );
    // The SQL grammar tags every created object as a function; the name and line are right.
    assert_eq!(
        items("a.sql", SQL),
        [item(1, "fn", "f_strip_html", 0), item(5, "fn", "users", 0)]
    );
}

#[test]
fn markdown_headings_nest_by_level() {
    assert_eq!(
        items("a.md", MD),
        [
            item(1, "section", "Title", 0),
            item(5, "section", "Part one", 1),
            item(7, "section", "Detail", 2),
            item(9, "section", "Part two", 1),
        ]
    );
    // Deeper than level 3: left out.
    assert_eq!(
        items("b.md", "# A\n\n#### Deep\n"),
        [item(1, "section", "A", 0)]
    );
}

#[test]
fn plain_text_has_no_outline() {
    assert!(items("notes.txt", "plain\ntext\n").is_empty());
    assert!(items("a.json", "{\"a\": 1}\n").is_empty());
}

const CLJ_PATH: &str = "components/berth/src/quayside/berth/core.clj";

/// Quayside with the Python sample and the berth fixture as text in memory, at quayside paths.
fn quayside_mixed() -> (
    ratatui::Terminal<ratatui::backend::TestBackend>,
    minimap_tui::App,
) {
    let (mut t, mut app) = quayside(150, 45);
    let berth = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/clojure/berth.clj"
    ))
    .unwrap();
    // Quayside has no Rust or TypeScript files; its first Python file holds the Python sample.
    let py_path = app
        .tree
        .files
        .iter()
        .find(|f| f.path.ends_with(".py"))
        .map(|f| f.path.clone())
        .unwrap();
    let files = HashMap::from([
        (py_path.clone(), PY.to_string()),
        (CLJ_PATH.to_string(), berth),
    ]);
    app.content = Content::memory(files.clone());
    app.details = Details::memory(files);
    let f = file(&app, &py_path);
    app.index.files[f].lines = PY.lines().count() as u64;
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    app.wait_for_colors(Duration::from_secs(10));
    draw(&mut t, &mut app);
    (t, app)
}

#[test]
fn a_selected_file_gets_its_grammar_outline_and_brackets_follow_it() {
    let (mut t, mut app) = quayside_mixed();
    let f = app.selected_file().unwrap();
    let names: Vec<&str> = app.outline(f).iter().map(|o| o.name.as_str()).collect();
    assert!(!names.is_empty(), "the outline arrives");
    assert!(
        (0..45).any(|y| row(&t, y).contains("OUTLINE")),
        "the panel shows the outline"
    );
    press(&mut app, KeyCode::Enter);
    app.wait_for_colors(Duration::from_secs(10));
    draw(&mut t, &mut app);
    keys(&mut app, "]]");
    let first = app.outline(f)[0].line as usize - 1;
    assert_eq!(app.viewer.as_ref().unwrap().cursor, first);
    keys(&mut app, "]]");
    let second = app.outline(f)[1].line as usize - 1;
    assert_eq!(app.viewer.as_ref().unwrap().cursor, second);
}

#[test]
fn a_clojure_file_keeps_its_tree_sitter_outline() {
    let (mut t, mut app) = quayside_mixed();
    let f = file(&app, CLJ_PATH);
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    app.wait_for_colors(Duration::from_secs(10));
    let kinds: Vec<&str> = app.outline(f).iter().map(|o| o.kind.as_str()).collect();
    // Unit 007's kinds are the head symbols as written.
    assert!(
        kinds.contains(&"defn") && kinds.contains(&"rf/reg-sub"),
        "{kinds:?}"
    );
}

#[test]
fn generic_outline_snapshot() {
    let (t, _) = quayside_mixed();
    insta::assert_snapshot!("generic_outline_panel", t.backend());
}
