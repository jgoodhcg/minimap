//! `minimap send` (unit `010`): the binary sends events to a listener, and stays silent and fast without one.

use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use minimap_core::events::{Event, Kind, Listener};

fn git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::create_dir_all(dir.path().join("src/app")).unwrap();
    dir
}

fn send(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_minimap"))
        .arg("send")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn assert_silent(out: &Output) {
    assert!(out.status.success(), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn minimap_send_events_arrive_in_order() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let l = Listener::spawn(&root).unwrap();
    // Relative paths resolve against the current directory, here `src/`.
    let src = root.join("src");
    for args in [
        &["--read", "app/a.clj"][..],
        &["--grep", "app/a.clj", "app/b.clj", "/elsewhere/c.clj"],
        &["--edit", "app/b.clj", "--lines", "12-20"],
        &[r#"{"v":1,"type":"read","path":"src/app/c.clj","source":"test"}"#],
        &["--clear"],
    ] {
        assert_silent(&send(&src, args));
    }
    let mut got = Vec::new();
    let t0 = Instant::now();
    while got.len() < 5 && t0.elapsed() < Duration::from_secs(3) {
        got.extend(l.wait(Duration::from_millis(100)));
    }
    assert_eq!(
        got,
        [
            Event::trail(Kind::Read, vec!["src/app/a.clj".into()]),
            Event::trail(
                Kind::Grep,
                vec!["src/app/a.clj".into(), "src/app/b.clj".into()]
            ),
            Event::Trail {
                kind: Kind::Edit,
                paths: vec!["src/app/b.clj".into()],
                lines: Some((12, 20)),
                source: None,
                session: None,
            },
            Event::Trail {
                kind: Kind::Read,
                paths: vec!["src/app/c.clj".into()],
                lines: None,
                source: Some("test".into()),
                session: None,
            },
            Event::Clear,
        ]
    );
}

#[test]
fn minimap_send_with_repo_sends_to_that_repo() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let l = Listener::spawn(&root).unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let a = root.join("src/app/a.clj");
    let out = send(
        elsewhere.path(),
        &[
            "--repo",
            root.to_str().unwrap(),
            "--read",
            a.to_str().unwrap(),
        ],
    );
    assert_silent(&out);
    assert_eq!(
        l.wait(Duration::from_secs(2)),
        Some(Event::trail(Kind::Read, vec!["src/app/a.clj".into()]))
    );
}

#[test]
fn minimap_send_without_a_listener_is_silent_and_fast() {
    let repo = git_repo();
    let no_repo = tempfile::tempdir().unwrap();
    // The first run of a new binary can wait for the system's code check.
    let _ = send(repo.path(), &["--clear"]);
    for (cwd, args) in [
        (repo.path(), &["--read", "src/a.clj"][..]),
        (repo.path(), &["--edit", "../outside.clj", "--lines", "x-y"]),
        (repo.path(), &["not json"]),
        (no_repo.path(), &["--read", "a.clj"]),
    ] {
        let t0 = Instant::now();
        let out = send(cwd, args);
        let took = t0.elapsed();
        assert_silent(&out);
        assert!(took < Duration::from_millis(50), "{args:?} took {took:?}");
    }
}
