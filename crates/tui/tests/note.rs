//! `minimap note` (unit `052`): the binary sends notes and notes clears to a listener, and reports a window that is not there.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use minimap_core::events::{Event, Listener};

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

fn note(cwd: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_minimap"))
        .arg("note")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    if let Some(s) = stdin {
        pipe.write_all(s.as_bytes()).unwrap();
    }
    drop(pipe);
    child.wait_with_output().unwrap()
}

fn events(l: &Listener, n: usize) -> Vec<Event> {
    let mut got = Vec::new();
    let t0 = Instant::now();
    while got.len() < n && t0.elapsed() < Duration::from_secs(3) {
        got.extend(l.wait(Duration::from_millis(100)));
    }
    got
}

fn ev(path: &str, lines: Option<(u32, u32)>, text: &str) -> Event {
    Event::Note {
        path: path.into(),
        lines,
        text: text.into(),
        source: None,
        session: None,
    }
}

#[test]
fn minimap_note_sends_notes_and_a_clear_in_order() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let l = Listener::spawn(&root).unwrap();
    let src = root.join("src");
    for (args, stdin) in [
        (
            &["app/a.clj", "--lines", "9-4", "Retries start here."][..],
            None,
        ),
        (&["app/b.clj", "--lines", "12"], None),
        (&["app/new.clj", "-"], Some("Line one.\nLine two.\n\n")),
        (&["--clear"], None),
    ] {
        let out = note(&src, args, stdin);
        assert!(out.status.success(), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
    }
    assert_eq!(
        events(&l, 4),
        [
            ev("src/app/a.clj", Some((4, 9)), "Retries start here."),
            ev("src/app/b.clj", Some((12, 12)), ""),
            // A new file's note is fine; stdin loses its trailing blank lines.
            ev("src/app/new.clj", None, "Line one.\nLine two."),
            Event::ClearNotes,
        ]
    );
}

#[test]
fn minimap_note_reports_a_missing_window_and_a_path_outside_the_repo() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let out = note(&root, &["src/a.clj", "t"], None);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("minimap note: no minimap shows ")
            && err.contains("Start minimap-gui there."),
        "{err}"
    );
    let _l = Listener::spawn(&root).unwrap();
    let other = tempfile::tempdir().unwrap();
    let outside = other.path().join("x.clj");
    let out = note(
        &root,
        &["--repo", root.to_str().unwrap(), outside.to_str().unwrap()],
        None,
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("is outside the repo"));
}

#[test]
fn minimap_note_usage_errors_exit_2_and_the_help_has_the_recipe() {
    let repo = git_repo();
    for args in [
        &[][..],
        &["a.clj", "--lines", "0-3"],
        &["a.clj", "--lines", "x"],
        &["--clear", "a.clj"],
    ] {
        let out = note(repo.path(), args, None);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
    }
    let help = note(repo.path(), &["--help"], None);
    let text = String::from_utf8_lossy(&help.stdout);
    for part in [
        "minimap note --clear",
        "--lines 40-58",
        "press n in the window",
        "Exit status",
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
    let top = Command::new(env!("CARGO_BIN_EXE_minimap"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&top.stdout).contains("minimap note --help"));
}
