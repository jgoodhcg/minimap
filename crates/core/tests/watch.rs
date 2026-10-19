//! The live index (unit `034`): a watcher on a scripted temporary repo.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use minimap_core::watch::{Update, Watcher};
use minimap_core::{Index, IndexOptions};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

fn write(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, text).unwrap();
}

/// A committed repo with two files and an ignored `target/`, and a watcher on it.
fn watched() -> (tempfile::TempDir, std::path::PathBuf, Watcher) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    git(&root, &["init", "-q"]);
    write(&root, "src/a.clj", "(ns a)\n");
    write(&root, "src/b.clj", "(ns b)\n");
    write(&root, ".gitignore", "target/\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "init"]);
    let index = Index::from_repo(&root, &IndexOptions::default()).unwrap();
    let w = Watcher::spawn(root.clone(), IndexOptions::default(), index).unwrap();
    // FSEvents can still deliver the setup's writes just after the stream starts. They find no change.
    std::thread::sleep(Duration::from_millis(400));
    let _ = w.poll();
    (dir, root, w)
}

/// The updates that arrive within `wait`.
fn updates(w: &Watcher, wait: Duration) -> Vec<Update> {
    let t0 = Instant::now();
    let mut out = Vec::new();
    while t0.elapsed() < wait {
        std::thread::sleep(Duration::from_millis(20));
        let (u, e) = w.poll();
        assert_eq!(e, None);
        out.extend(u);
    }
    out
}

/// Waits up to 1 s for the first update, then 600 ms more for any other.
fn one_update(w: &Watcher) -> (Update, Duration) {
    let t0 = Instant::now();
    let first = loop {
        assert!(
            t0.elapsed() < Duration::from_secs(1),
            "no update within 1 s"
        );
        std::thread::sleep(Duration::from_millis(10));
        if let (Some(u), _) = w.poll() {
            break u;
        }
    };
    let took = t0.elapsed();
    let rest = updates(w, Duration::from_millis(600));
    assert!(rest.is_empty(), "a second update: {:?}", rest[0].changed);
    (first, took)
}

fn paths(u: &Update) -> Vec<&str> {
    u.index.files.iter().map(|f| f.path.as_str()).collect()
}

#[test]
fn an_added_file_gives_one_update() {
    let (_dir, root, w) = watched();
    write(&root, "src/c.clj", "(ns c)\n(def x 1)\n");
    let (u, took) = one_update(&w);
    eprintln!("added: {took:?}");
    assert_eq!(
        paths(&u),
        [".gitignore", "src/a.clj", "src/b.clj", "src/c.clj"]
    );
    let c = u
        .index
        .files
        .iter()
        .find(|f| f.path == "src/c.clj")
        .unwrap();
    assert!(c.untracked);
    assert_eq!(c.lines, 2);
    assert!(u.changed.is_empty());
    assert!(!u.head_moved);
}

#[test]
fn a_removed_file_gives_one_update() {
    let (_dir, root, w) = watched();
    write(&root, "scratch.txt", "x\n");
    one_update(&w);
    fs::remove_file(root.join("scratch.txt")).unwrap();
    let (u, _) = one_update(&w);
    assert_eq!(paths(&u), [".gitignore", "src/a.clj", "src/b.clj"]);
}

#[test]
fn an_edited_file_gives_one_update_with_its_path() {
    let (_dir, root, w) = watched();
    write(&root, "src/a.clj", "(ns a)\n(def y 2)\n(def z 3)\n");
    let (u, _) = one_update(&w);
    assert_eq!(u.changed, ["src/a.clj"]);
    let a = u
        .index
        .files
        .iter()
        .find(|f| f.path == "src/a.clj")
        .unwrap();
    assert_eq!(a.lines, 3);
    assert!(!a.untracked);
}

#[test]
fn a_rename_gives_one_update() {
    let (_dir, root, w) = watched();
    write(&root, "src/old.clj", "(ns old)\n");
    one_update(&w);
    fs::rename(root.join("src/old.clj"), root.join("src/new.clj")).unwrap();
    let (u, _) = one_update(&w);
    assert_eq!(
        paths(&u),
        [".gitignore", "src/a.clj", "src/b.clj", "src/new.clj"]
    );
}

#[test]
fn writes_under_an_ignored_directory_give_no_update() {
    let (_dir, root, w) = watched();
    let scans = w.scans();
    for k in 0..20 {
        write(&root, &format!("target/debug/out-{k}.o"), "bytes\n");
    }
    let got = updates(&w, Duration::from_secs(1));
    assert!(got.is_empty(), "an update for ignored writes");
    assert_eq!(w.scans(), scans, "an ignored write started a re-index");
}

#[test]
fn a_config_excluded_path_starts_no_reindex() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    git(&root, &["init", "-q"]);
    write(&root, "a.txt", "a\n");
    let options = IndexOptions {
        excludes: vec!["public/**".into()],
        ..Default::default()
    };
    let index = Index::from_repo(&root, &options).unwrap();
    let w = Watcher::spawn(root.clone(), options, index).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    let _ = w.poll();
    let scans = w.scans();
    write(&root, "public/icon.svg", "<svg/>\n");
    assert!(updates(&w, Duration::from_secs(1)).is_empty());
    assert_eq!(w.scans(), scans);
}

#[test]
fn a_commit_sets_head_moved() {
    let (_dir, root, w) = watched();
    write(&root, "src/b.clj", "(ns b)\n(def b 1)\n");
    git(&root, &["commit", "-qam", "change b"]);
    let got = updates(&w, Duration::from_secs(1));
    assert!(got.iter().any(|u| u.head_moved), "no update moved HEAD");
    let last = got.last().unwrap();
    assert!(last.index.files.iter().all(|f| !f.untracked && !f.missing));
}

#[test]
fn a_status_refresh_sends_nothing() {
    let (_dir, root, w) = watched();
    // `git status` can rewrite the git index with fresh stat data; the file set does not change.
    git(&root, &["status", "--short"]);
    git(&root, &["update-index", "--refresh"]);
    assert!(updates(&w, Duration::from_secs(1)).is_empty());
}

#[test]
fn a_watcher_with_a_wake_runs_it_after_the_update_is_queued() {
    // Unit 048: the window repaints on the wake, so the update must be ready to poll when it runs.
    use std::sync::mpsc;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    git(&root, &["init", "-q"]);
    write(&root, "src/a.clj", "(ns a)\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "init"]);
    let index = Index::from_repo(&root, &IndexOptions::default()).unwrap();
    let (tx, rx) = mpsc::channel();
    let w = Watcher::spawn_with(root.clone(), IndexOptions::default(), index, move || {
        let _ = tx.send(());
    })
    .unwrap();
    std::thread::sleep(Duration::from_millis(400));
    let _ = w.poll();
    while rx.try_recv().is_ok() {}
    write(&root, "src/new.clj", "(ns new)\n");
    let t0 = Instant::now();
    loop {
        rx.recv_timeout(Duration::from_secs(5)).expect("a wake");
        let (update, _) = w.poll();
        if let Some(u) = update
            && u.index.files.iter().any(|f| f.path == "src/new.clj")
        {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(5),
            "no update with the new file"
        );
    }
}
