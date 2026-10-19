//! The event socket (unit `010`): parsing, path normalization, the socket path, and a listener on a temporary repo.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use minimap_core::events::{
    self, Event, Kind, ListenError, Listener, normalize, parse, socket_path, socket_path_in,
};

#[test]
fn parses_each_type() {
    assert_eq!(
        parse(r#"{"v":1,"type":"read","path":"src/a.clj","source":"claude-code"}"#),
        Some(Event::Trail {
            kind: Kind::Read,
            paths: vec!["src/a.clj".into()],
            lines: None,
            source: Some("claude-code".into()),
            session: None,
        })
    );
    assert_eq!(
        parse(r#"{"v":1,"type":"grep","paths":["src/a.clj","src/b.clj"]}"#),
        Some(Event::trail(
            Kind::Grep,
            vec!["src/a.clj".into(), "src/b.clj".into()]
        ))
    );
    assert_eq!(
        parse(r#"{"v":1,"type":"edit","path":"src/a.clj","lines":[40,58],"source":"x"}"#),
        Some(Event::Trail {
            kind: Kind::Edit,
            paths: vec!["src/a.clj".into()],
            lines: Some((40, 58)),
            source: Some("x".into()),
            session: None,
        })
    );
    assert_eq!(
        parse(r#"{"v":1,"type":"clear","layer":"trail"}"#),
        Some(Event::Clear)
    );
    assert_eq!(parse(r#"{"v":1,"type":"clear"}"#), Some(Event::Clear));
    assert_eq!(parse(r#"{"v":1,"type":"clear","layer":"focus"}"#), None);
}

#[test]
fn drops_other_versions_unknown_types_and_missing_paths() {
    assert_eq!(parse(r#"{"v":2,"type":"read","path":"a"}"#), None);
    assert_eq!(parse(r#"{"type":"read","path":"a"}"#), None);
    assert_eq!(parse(r#"{"v":1,"type":"glob","path":"a"}"#), None);
    assert_eq!(parse(r#"{"v":1,"type":"read"}"#), None);
    assert_eq!(parse(r#"{"v":1,"type":"edit","path":""}"#), None);
    assert_eq!(parse(r#"{"v":1,"type":"grep","paths":[]}"#), None);
    assert_eq!(parse("not json"), None);
    assert_eq!(parse(""), None);
}

#[test]
fn edit_lines_are_optional_and_ordered() {
    let lines = |s: &str| match parse(s) {
        Some(Event::Trail { lines, .. }) => lines,
        other => panic!("{other:?}"),
    };
    assert_eq!(lines(r#"{"v":1,"type":"edit","path":"a"}"#), None);
    assert_eq!(
        lines(r#"{"v":1,"type":"edit","path":"a","lines":[9,3]}"#),
        Some((3, 9))
    );
    assert_eq!(
        lines(r#"{"v":1,"type":"edit","path":"a","lines":[1]}"#),
        None
    );
    // Only edits carry lines.
    assert_eq!(
        lines(r#"{"v":1,"type":"read","path":"a","lines":[1,2]}"#),
        None
    );
}

#[test]
fn lines_round_trip() {
    for e in [
        Event::Trail {
            kind: Kind::Edit,
            paths: vec!["src/a.clj".into()],
            lines: Some((40, 58)),
            source: Some("claude-code".into()),
            session: None,
        },
        Event::trail(Kind::Grep, vec!["a".into(), "b".into()]),
        Event::trail(Kind::Read, vec!["/abs/a".into()]),
        Event::Clear,
    ] {
        assert_eq!(parse(&e.to_line()), Some(e.clone()), "{}", e.to_line());
    }
}

#[test]
fn normalizes_paths() {
    let root = Path::new("/work/repo");
    let n = |p: &str| normalize(root, root, p);
    assert_eq!(n("src/a.clj").as_deref(), Some("src/a.clj"));
    assert_eq!(n("./src/./a.clj").as_deref(), Some("src/a.clj"));
    assert_eq!(n("src/x/../a.clj").as_deref(), Some("src/a.clj"));
    assert_eq!(n("/work/repo/src/a.clj").as_deref(), Some("src/a.clj"));
    assert_eq!(n("/work/repo/src/../a.clj").as_deref(), Some("a.clj"));
    assert_eq!(n("/work/repo/../other/a.clj"), None);
    assert_eq!(n("/elsewhere/a.clj"), None);
    assert_eq!(n("../a.clj"), None);
    assert_eq!(n("/work/repo"), None);
    assert_eq!(n("/work/repository/a.clj"), None);
}

#[test]
fn an_absolute_path_matches_the_given_and_the_canonical_root() {
    // On macOS a temp dir under /var is /private/var when canonical.
    let dir = tempfile::tempdir().unwrap();
    let given = dir.path().to_path_buf();
    let canonical = given.canonicalize().unwrap();
    std::fs::create_dir_all(given.join("src")).unwrap();
    let at = |base: &Path, p: &str| base.join(p).to_string_lossy().into_owned();
    for base in [&given, &canonical] {
        assert_eq!(
            normalize(&given, &canonical, &at(base, "src/a.clj")).as_deref(),
            Some("src/a.clj")
        );
    }
    // A root given in its canonical form still matches a path through the link.
    assert_eq!(
        normalize(&canonical, &canonical, &at(&given, "src/new.clj")).as_deref(),
        Some("src/new.clj")
    );
}

#[test]
fn the_socket_path_falls_back_to_tmp_when_too_long() {
    let root = Path::new("/Users/example/projects/quayside");
    let short = socket_path_in(Path::new("/var/folders/ab/T"), "me", root);
    assert_eq!(
        short,
        PathBuf::from("/var/folders/ab/T/minimap-me-5d88662b8abea250-events.sock")
    );
    let long_tmp = PathBuf::from(format!("/var/folders/{}/T", "x".repeat(80)));
    let long = socket_path_in(&long_tmp, "me", root);
    assert_eq!(
        long,
        PathBuf::from("/tmp/minimap-me-5d88662b8abea250-events.sock")
    );
    assert!(long.as_os_str().len() <= events::SOCKET_PATH_MAX);
}

fn git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    dir
}

#[test]
fn a_listener_takes_events_in_order_and_owns_its_socket() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let l = Listener::spawn(&root).unwrap();
    assert_eq!(l.socket(), socket_path(&root));
    let mode = std::os::unix::fs::PermissionsExt::mode(
        &std::fs::metadata(l.socket()).unwrap().permissions(),
    );
    assert_eq!(mode & 0o777, 0o600);

    let abs = root.join("src/b.clj").to_string_lossy().into_owned();
    let sent = [
        Event::trail(Kind::Read, vec!["src/a.clj".into()]),
        Event::trail(Kind::Grep, vec![abs.clone(), "/elsewhere/x.clj".into()]),
        Event::Trail {
            kind: Kind::Edit,
            paths: vec![abs],
            lines: Some((3, 7)),
            source: Some("test".into()),
            session: None,
        },
        Event::Clear,
    ];
    for e in &sent {
        events::send(&root, e).unwrap();
    }
    let mut got = Vec::new();
    let t0 = Instant::now();
    while got.len() < 4 && t0.elapsed() < Duration::from_secs(2) {
        got.extend(l.wait(Duration::from_millis(100)));
    }
    assert_eq!(
        got,
        [
            Event::trail(Kind::Read, vec!["src/a.clj".into()]),
            Event::trail(Kind::Grep, vec!["src/b.clj".into()]),
            Event::Trail {
                kind: Kind::Edit,
                paths: vec!["src/b.clj".into()],
                lines: Some((3, 7)),
                source: Some("test".into()),
                session: None,
            },
            Event::Clear,
        ]
    );

    // A second listener on the repo does not bind.
    assert!(matches!(
        Listener::spawn(&root),
        Err(ListenError::AlreadyListening)
    ));
    let socket = l.socket().to_path_buf();
    drop(l);
    assert!(
        !socket.exists(),
        "dropping the listener removes the socket file"
    );
}

#[test]
fn a_stale_socket_file_is_replaced() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let path = socket_path(&root);
    // A socket that nobody listens on: bound, then its listener dropped without removing the file.
    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
    assert!(path.exists());
    let l = Listener::spawn(&root).unwrap();
    events::send(&root, &Event::Clear).unwrap();
    assert_eq!(l.wait(Duration::from_secs(1)), Some(Event::Clear));
}

#[test]
fn send_without_a_listener_is_an_error_and_fast() {
    let repo = git_repo();
    let t0 = Instant::now();
    assert!(events::send(repo.path(), &Event::Clear).is_err());
    assert!(t0.elapsed() < Duration::from_millis(50));
}

#[test]
fn a_listener_with_a_wake_runs_it_after_each_event_is_queued() {
    // Unit 047: the window repaints on the wake, so the event must be ready to poll when it runs.
    use std::sync::mpsc;
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let (tx, rx) = mpsc::channel();
    let l = Listener::spawn_with(&root, move || {
        let _ = tx.send(());
    })
    .unwrap();
    events::send(&root, &Event::trail(Kind::Read, vec!["src/a.clj".into()])).unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        l.poll(),
        [Event::trail(Kind::Read, vec!["src/a.clj".into()])]
    );
}

#[test]
fn a_session_survives_the_line_and_an_unknown_field_is_ignored() {
    // Unit 050: `session` is optional, and the version stays 1.
    let e = Event::Trail {
        kind: Kind::Edit,
        paths: vec!["src/a.clj".into()],
        lines: Some((3, 4)),
        source: Some("claude-code".into()),
        session: Some("0f9e8d7c-1111-2222-3333-444455556666".into()),
    };
    let line = e.to_line();
    assert!(
        line.contains(r#""session":"0f9e8d7c-1111-2222-3333-444455556666""#),
        "{line}"
    );
    assert!(line.contains(r#""v":1"#), "{line}");
    assert_eq!(parse(&line), Some(e));
    let Some(Event::Trail { session, .. }) = parse(r#"{"v":1,"type":"read","path":"src/a.clj"}"#)
    else {
        panic!()
    };
    assert_eq!(session, None);
    assert_eq!(
        parse(r#"{"v":1,"type":"read","path":"src/a.clj","session":"s1","color":"red"}"#),
        Some(Event::Trail {
            kind: Kind::Read,
            paths: vec!["src/a.clj".into()],
            lines: None,
            source: None,
            session: Some("s1".into()),
        })
    );
}

fn note(path: &str, lines: Option<(u32, u32)>, text: &str) -> Event {
    Event::Note {
        path: path.into(),
        lines,
        text: text.into(),
        source: None,
        session: None,
    }
}

#[test]
fn a_note_parses_with_and_without_its_optional_fields() {
    // Unit 052.
    assert_eq!(
        parse(
            r#"{"v":1,"type":"note","path":"src/a.clj","lines":[58,40],"text":"Retries start here.","source":"claude-code","session":"s1"}"#
        ),
        Some(Event::Note {
            path: "src/a.clj".into(),
            lines: Some((40, 58)),
            text: "Retries start here.".into(),
            source: Some("claude-code".into()),
            session: Some("s1".into()),
        })
    );
    assert_eq!(
        parse(r#"{"v":1,"type":"note","path":"src/a.clj"}"#),
        Some(note("src/a.clj", None, ""))
    );
    // Lines that are not two numbers are left out; the note stays.
    assert_eq!(
        parse(r#"{"v":1,"type":"note","path":"a","lines":[3],"text":"t"}"#),
        Some(note("a", None, "t"))
    );
    assert_eq!(parse(r#"{"v":1,"type":"note","text":"t"}"#), None);
    assert_eq!(parse(r#"{"v":1,"type":"note","path":"","text":"t"}"#), None);
}

#[test]
fn a_long_note_text_is_cut_with_an_ellipsis() {
    let long = "x".repeat(5000);
    let line = format!(r#"{{"v":1,"type":"note","path":"a","text":"{long}"}}"#);
    let Some(Event::Note { text, .. }) = parse(&line) else {
        panic!()
    };
    assert_eq!(text.chars().count(), events::NOTE_TEXT_MAX);
    assert!(text.ends_with('…'));
    let short = "y".repeat(events::NOTE_TEXT_MAX);
    assert_eq!(events::note_text(&short), short);
}

#[test]
fn notes_and_a_notes_clear_round_trip() {
    for e in [
        note(
            "src/a.clj",
            Some((40, 58)),
            "Line one.\nLine two with \"quotes\".",
        ),
        note("src/b.clj", None, ""),
        Event::Note {
            path: "src/c.clj".into(),
            lines: Some((7, 7)),
            text: "t".into(),
            source: Some("x".into()),
            session: Some("s".into()),
        },
        Event::ClearNotes,
    ] {
        assert_eq!(parse(&e.to_line()), Some(e.clone()), "{}", e.to_line());
    }
    assert_eq!(
        parse(r#"{"v":1,"type":"clear","layer":"notes"}"#),
        Some(Event::ClearNotes)
    );
    assert_eq!(parse(r#"{"v":1,"type":"clear","layer":"other"}"#), None);
}

#[test]
fn a_note_outside_the_repo_normalizes_to_none() {
    let root = Path::new("/work/repo");
    let n = |e: Event| e.normalized(root, root);
    assert_eq!(
        n(note("/work/repo/src/a.clj", Some((1, 2)), "t")),
        Some(note("src/a.clj", Some((1, 2)), "t"))
    );
    assert_eq!(n(note("/elsewhere/a.clj", None, "t")), None);
    assert_eq!(n(Event::ClearNotes), Some(Event::ClearNotes));
}

#[test]
fn a_listener_receives_a_note() {
    let repo = git_repo();
    let root = repo.path().canonicalize().unwrap();
    let l = Listener::spawn(&root).unwrap();
    let abs = root.join("src/a.clj").to_string_lossy().into_owned();
    events::send(&root, &note(&abs, Some((3, 9)), "Look here.")).unwrap();
    events::send(&root, &Event::ClearNotes).unwrap();
    assert_eq!(
        l.wait(Duration::from_secs(2)),
        Some(note("src/a.clj", Some((3, 9)), "Look here."))
    );
    assert_eq!(l.wait(Duration::from_secs(2)), Some(Event::ClearNotes));
}
