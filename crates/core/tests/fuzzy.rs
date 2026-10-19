//! The fuzzy matcher (unit 025), against hand-checked fzf behavior.

mod common;

use minimap_core::fuzzy::{Candidate, Pattern, rank};

fn score(query: &str, path: &str) -> Option<i32> {
    Pattern::parse(query).score(path).map(|m| m.score)
}

fn ranked(query: &str, paths: &[&str]) -> Vec<String> {
    let pattern = Pattern::parse(query);
    let candidates: Vec<Candidate> = paths.iter().map(|p| Candidate::new(p)).collect();
    rank(candidates.iter().enumerate(), &pattern)
        .into_iter()
        .map(|r| paths[r.index].to_string())
        .collect()
}

#[test]
fn a_whole_word_at_a_path_segment_ranks_above_scattered_letters() {
    let paths = [
        "resources/ui/layouts/emails/s.txt",
        "components/berth/src/quayside/berth/rules.clj",
        "src/rustler.clj",
    ];
    assert_eq!(
        ranked("rules", &paths)[0],
        "components/berth/src/quayside/berth/rules.clj"
    );
}

#[test]
fn segment_starts_beat_mid_word_matches() {
    // b and c both start a segment in `berth/core.clj`; in `abc.clj` they sit inside a word.
    assert_eq!(score("bc", "berth/core.clj"), Some(49));
    assert_eq!(score("bc", "abc.clj"), Some(36));
    // A camel-case hump counts as a boundary too.
    assert_eq!(score("bc", "BerthCore.clj"), Some(49));
}

#[test]
fn consecutive_characters_beat_a_gapped_match() {
    // `core.clj`: 32 + 24 + 24 + 24. `c_o_r_e.clj`: each letter follows `_`, but the gaps cost more.
    // The values are telescope-fzf-native's (`scripts/fzf-golden.py`).
    assert_eq!(score("core", "core.clj"), Some(104));
    assert_eq!(score("core", "c_o_r_e.clj"), Some(95));
}

#[test]
fn positions_are_the_char_indices_of_the_best_alignment() {
    let m = Pattern::parse("rules")
        .score("quayside/berth/rules.clj")
        .unwrap();
    assert_eq!(m.positions, [15, 16, 17, 18, 19], "not the r in berth");
    let m = Pattern::parse("bc").score("berth/core.clj").unwrap();
    assert_eq!(m.positions, [0, 6]);
    // Non-ASCII text counts chars, not bytes.
    let m = Pattern::parse("né").score("docs/année.md").unwrap();
    assert_eq!(m.positions, [7, 8], "n n é: the n next to é");
}

#[test]
fn a_one_char_query_takes_the_first_boundary() {
    let m = Pattern::parse("c").score("abc/core.clj").unwrap();
    assert_eq!(m.positions, [4]);
    assert_eq!(m.score, 16 + 8 * 2);
}

#[test]
fn every_term_must_match() {
    let paths = ["berth/rules.clj", "berth/core.clj", "tariff/rules.clj"];
    assert_eq!(ranked("berth rules", &paths), ["berth/rules.clj"]);
    // Positions from both terms, merged.
    let m = Pattern::parse("berth rules")
        .score("berth/rules.clj")
        .unwrap();
    assert_eq!(m.positions, (0..11).filter(|&i| i != 5).collect::<Vec<_>>());
}

#[test]
fn extended_syntax() {
    // 'exact: a substring, not a subsequence.
    assert!(score("'rule", "berth/rules.clj").is_some());
    assert!(score("'rule", "r_u_l_e.clj").is_none());
    assert!(score("rule", "r_u_l_e.clj").is_some());
    // ^prefix and suffix$ anchor to the path.
    assert!(score("^server", "server/a.clj").is_some());
    assert!(score("^server", "a/server.clj").is_none());
    assert!(score(".clj$", "a.clj").is_some());
    assert!(score(".clj$", "a.cljs").is_none());
    assert!(score("^a.clj$", "a.clj").is_some());
    assert!(score("^a.clj$", "a.clj.bak").is_none());
    // !term: the path must not contain it.
    let paths = ["berth/rules.clj", "berth/rules_test.clj"];
    assert_eq!(ranked("berth !test", &paths), ["berth/rules.clj"]);
    assert_eq!(ranked("!^berth", &paths), Vec::<String>::new());
    // A lone operator is no term.
    assert!(Pattern::parse("! ' ^").is_empty());
}

#[test]
fn smart_case() {
    assert!(score("rules", "Rules.md").is_some());
    assert!(score("Rules", "Rules.md").is_some());
    assert!(score("Rules", "rules.md").is_none());
    // Per term: `Readme` is case-sensitive, `docs` is not.
    assert!(score("DOCS Readme", "docs/Readme.md").is_none());
    assert!(score("docs Readme", "DOCS/Readme.md").is_some());
}

#[test]
fn an_empty_query_matches_everything_in_input_order() {
    let paths = ["b/long/path.clj", "a.clj", "c.md"];
    assert!(Pattern::parse("").is_empty());
    assert_eq!(score("", "anything"), Some(0));
    assert_eq!(ranked("", &paths), paths);
    assert_eq!(ranked("   ", &paths), paths);
}

#[test]
fn ties_prefer_a_file_name_match_then_a_shorter_path_then_input_order() {
    // The same score: `core` at a segment start in both.
    assert_eq!(score("core", "core/a.clj"), score("core", "a/core.clj"));
    assert_eq!(
        ranked("core", &["core/a.clj", "a/core.clj"]),
        ["a/core.clj", "core/a.clj"]
    );
    assert_eq!(
        ranked("core", &["xy/core.clj", "x/core.clj"]),
        ["x/core.clj", "xy/core.clj"]
    );
    assert_eq!(
        ranked("core", &["b/core.clj", "a/core.clj"]),
        ["b/core.clj", "a/core.clj"]
    );
}

#[test]
fn a_pattern_that_is_not_a_subsequence_does_not_match() {
    assert!(score("xyz", "berth/rules.clj").is_none());
    assert!(score("seluR", "Rules.md").is_none());
    assert!(score("rulesx", "rules").is_none());
}

#[test]
fn matches_telescope_fzf_native_on_quayside() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let files = std::fs::read_to_string(format!("{dir}/quayside-files.json")).unwrap();
    let golden = std::fs::read_to_string(format!("{dir}/quayside-fzf.json")).unwrap();
    assert!(common::assert_matches_fzf_native(&files, &golden) > 9_000);
}
