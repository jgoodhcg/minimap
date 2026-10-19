//! Outlines and namespaces (unit `007`).

use minimap_core::outline::{Clojure, Language, OutlineItem, language_for};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/clojure/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn items(source: &str) -> Vec<(u32, String, String, bool)> {
    Clojure
        .outline(source)
        .into_iter()
        .map(
            |OutlineItem {
                 line,
                 kind,
                 name,
                 private,
                 ..
             }| (line, kind, name, private),
        )
        .collect()
}

fn item(line: u32, kind: &str, name: &str, private: bool) -> (u32, String, String, bool) {
    (line, kind.to_string(), name.to_string(), private)
}

#[test]
fn clojure_top_level_definitions() {
    let got = items(&fixture("berth.clj"));
    let want = vec![
        item(6, "def", "max-length", false),
        item(8, "defn", "get-berth", false),
        item(13, "defn-", "score", true),
        item(16, "defn", "rank", true),
        item(19, "def", "cache", true),
        item(21, "defmacro", "with-berth", false),
        item(24, "defmulti", "allocate", false),
        item(26, "defmethod", "allocate :greedy", false),
        item(29, "defmethod", "allocate [:fair :v2]", false),
        item(34, "defprotocol", "Scorer", false),
        item(37, "defrecord", "Berth", false),
        item(39, "s/def", "::berth-id", false),
        item(41, "rf/reg-sub", ":berth/list", false),
    ];
    assert_eq!(got, want);
}

#[test]
fn clojure_namespace_skips_metadata_and_docstrings() {
    assert_eq!(
        Clojure.namespace(&fixture("berth.clj")).as_deref(),
        Some("quayside.berth.core")
    );
    assert_eq!(
        Clojure.namespace(&fixture("clock.cljc")).as_deref(),
        Some("quayside.units.clock")
    );
    assert_eq!(Clojure.namespace("(defn a [] 1)"), None);
}

#[test]
fn reader_conditionals_count_every_branch() {
    let got = items(&fixture("clock.cljc"));
    let want = vec![
        item(3, "defn", "now", false),
        item(4, "defn", "now", false),
        item(6, "def", "zone", false),
        item(6, "defn-", "offset", true),
        item(8, "defn", "elapsed", false),
    ];
    assert_eq!(got, want);
}

#[test]
fn forms_without_a_name_are_skipped() {
    assert!(items("(defn)\n(def \"str\" 1)\n(foo bar)\n").is_empty());
}

#[test]
fn languages_register_by_extension() {
    for p in ["a/b.clj", "b.cljs", "c/d.cljc", "bb/e.bb"] {
        assert!(language_for(p).is_some(), "{p}");
    }
    for p in ["a.ts", "Makefile", "clj", "a.clj.md", "dir.clj/readme"] {
        assert!(language_for(p).is_none(), "{p}");
    }
}
