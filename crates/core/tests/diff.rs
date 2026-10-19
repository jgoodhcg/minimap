//! Uncommitted changes against `HEAD` (unit `030`).

use std::path::Path;
use std::process::Command;

use minimap_core::diff::{FileDiff, Tag, text_at};

/// Runs git in `dir` without the user's config, and returns its stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn write(dir: &Path, path: &str, text: &str) {
    std::fs::write(dir.join(path), text).unwrap();
}

/// No line repeats, so the alignment is unique.
const COMMITTED: &str =
    "(ns a)\n;; f\n(defn f [x]\n  (inc x))\n;; g\n(defn g [y]\n  (dec y))\n;; h\n(def h 1)\n";
/// Line 2 of `f` changes, `g` goes, and `k` arrives at the end.
const EDITED: &str = "(ns a)\n;; f\n(defn f [x]\n  (+ x 2))\n;; h\n(def h 1)\n;; k\n(defn k [])\n";

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "a.clj", COMMITTED);
    git(d, &["add", "."]);
    git(d, &["commit", "-qm", "add a"]);
    dir
}

/// `git diff --numstat` counts of one path: added and removed lines.
fn numstat(dir: &Path, args: &[&str]) -> (usize, usize) {
    let out = git(dir, &[&["diff", "--numstat"], args].concat());
    let mut it = out.split_whitespace();
    let n = |s: Option<&str>| s.unwrap().parse().unwrap();
    (n(it.next()), n(it.next()))
}

#[test]
fn text_at_head_is_the_committed_text() {
    let dir = repo();
    write(dir.path(), "a.clj", EDITED);
    assert_eq!(
        text_at(dir.path(), "HEAD", "a.clj").unwrap().as_deref(),
        Some(COMMITTED)
    );
    assert_eq!(text_at(dir.path(), "HEAD", "no.clj").unwrap(), None);
    let id = git(dir.path(), &["rev-parse", "HEAD"]);
    assert_eq!(
        text_at(dir.path(), id.trim(), "a.clj").unwrap().as_deref(),
        Some(COMMITTED)
    );
    assert!(text_at(dir.path(), "not-an-id", "a.clj").is_err());
}

#[test]
fn an_unborn_branch_has_no_head_text() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    assert_eq!(text_at(dir.path(), "HEAD", "a.clj").unwrap(), None);
}

#[test]
fn an_edit_gives_rows_hunks_and_counts() {
    let d = FileDiff::compute(COMMITTED, EDITED);
    let tags: Vec<(Tag, Option<usize>, Option<usize>)> =
        d.rows.iter().map(|r| (r.tag, r.old, r.new)).collect();
    use Tag::*;
    assert_eq!(
        tags,
        [
            (Same, Some(0), Some(0)),
            (Same, Some(1), Some(1)),
            (Same, Some(2), Some(2)),
            (Removed, Some(3), None),
            (Removed, Some(4), None),
            (Removed, Some(5), None),
            (Removed, Some(6), None),
            (Added, None, Some(3)),
            (Same, Some(7), Some(4)),
            (Same, Some(8), Some(5)),
            (Added, None, Some(6)),
            (Added, None, Some(7)),
        ]
    );
    assert_eq!(d.hunks, [(3, 8), (10, 12)]);
    assert_eq!((d.added, d.removed), (3, 4));
    assert!(!d.is_empty());
    // A removed row belongs to the next working-tree line.
    assert_eq!((d.line_of(3), d.line_of(6), d.line_of(8)), (3, 3, 4));
    assert_eq!((d.row_of(3), d.row_of(5), d.row_of(99)), (7, 9, 11));
    assert_eq!(d.added_lines().collect::<Vec<_>>(), [3, 6, 7]);
    assert_eq!(d.removal_marks(), [Some(2)]);
    assert_eq!(d.changed_lines(), [3, 6, 7]);
}

#[test]
fn equal_texts_have_no_changes_and_a_new_file_is_all_added() {
    assert!(FileDiff::compute(COMMITTED, COMMITTED).is_empty());
    let d = FileDiff::compute("", "a\nb\n");
    assert_eq!((d.added, d.removed, d.hunks.clone()), (2, 0, vec![(0, 2)]));
    let top = FileDiff::compute("x\na\n", "a\n");
    assert_eq!(top.removal_marks(), [None], "a removal above line 1");
}

#[test]
fn counts_match_git_diff_numstat() {
    let dir = repo();
    let d = dir.path();
    write(d, "a.clj", EDITED);
    let ours = FileDiff::compute(COMMITTED, EDITED);
    assert_eq!((ours.added, ours.removed), numstat(d, &["--", "a.clj"]));
    // A staged new file is all added against HEAD.
    write(d, "b.clj", "(ns b)\n(def x 1)\n");
    git(d, &["add", "b.clj"]);
    assert_eq!(text_at(d, "HEAD", "b.clj").unwrap(), None);
    let b = FileDiff::compute("", "(ns b)\n(def x 1)\n");
    assert_eq!((b.added, b.removed), numstat(d, &["HEAD", "--", "b.clj"]));
}
