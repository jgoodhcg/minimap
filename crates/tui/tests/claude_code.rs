//! The Claude Code adapter (unit `011`): payloads map to trail events and go to each path's repo.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use minimap_core::events::{Event, Kind, Listener};
use minimap_tui::claude_code::{SOURCE, map, route};

const RULES: &str = "components/berth/src/quayside/berth/rules.clj";
const ALLOC: &str = "components/berth/src/quayside/berth/allocation.clj";
const TEST: &str = "components/berth/test/quayside/berth/rules_test.clj";

/// A git repo with three quayside files of 30 lines each, at its canonical path.
fn repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(status.success());
    for p in [RULES, ALLOC, TEST] {
        let full = root.join(p);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        let text: Vec<String> = (1..=30).map(|n| format!("(def v{n} {n})")).collect();
        std::fs::write(full, text.join("\n") + "\n").unwrap();
    }
    (dir, root)
}

fn at(root: &Path, p: &str) -> String {
    root.join(p).to_string_lossy().into_owned()
}

fn payload(root: &Path, tool: &str, input: Value, response: Value) -> Value {
    json!({
        "session_id": "s",
        "cwd": root.to_string_lossy(),
        "hook_event_name": "PostToolUse",
        "tool_name": tool,
        "tool_input": input,
        "tool_response": response,
        "tool_use_id": "toolu_1",
    })
}

/// A payload's mapping, with paths as strings.
type Strings = (Kind, Vec<String>, Option<(u32, u32)>);

fn mapped(p: &Value) -> Option<Strings> {
    map(p).map(|m| {
        let paths = m
            .paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        (m.kind, paths, m.lines)
    })
}

#[test]
fn read_write_and_notebook_edit_map_to_their_paths() {
    let (_d, root) = repo();
    let read = payload(
        &root,
        "Read",
        json!({"file_path": at(&root, RULES)}),
        json!({}),
    );
    assert_eq!(
        mapped(&read),
        Some((Kind::Read, vec![at(&root, RULES)], None))
    );
    let write = payload(
        &root,
        "Write",
        json!({"file_path": at(&root, TEST), "content": "x"}),
        json!({}),
    );
    assert_eq!(
        mapped(&write),
        Some((Kind::Edit, vec![at(&root, TEST)], None))
    );
    let nb = payload(
        &root,
        "NotebookEdit",
        json!({"notebook_path": at(&root, "docs/a.ipynb"), "new_source": "x"}),
        json!({}),
    );
    assert_eq!(
        mapped(&nb),
        Some((Kind::Edit, vec![at(&root, "docs/a.ipynb")], None))
    );
}

#[test]
fn glob_bash_and_unknown_tools_send_nothing() {
    let (_d, root) = repo();
    for tool in ["Glob", "Bash", "WebFetch", "Task"] {
        let p = payload(
            &root,
            tool,
            json!({"pattern": "**/*.clj", "file_path": at(&root, RULES)}),
            json!({"filenames": [at(&root, RULES)]}),
        );
        assert_eq!(mapped(&p), None, "{tool}");
        assert!(route(&p).is_empty(), "{tool}");
    }
}

#[test]
fn an_edit_takes_its_lines_from_the_structured_patch() {
    let (_d, root) = repo();
    let p = payload(
        &root,
        "Edit",
        json!({"file_path": at(&root, RULES), "old_string": "(def v12 12)", "new_string": "(def v12 :x)\n(def w 1)"}),
        json!({
            "filePath": at(&root, RULES),
            "structuredPatch": [{
                "oldStart": 9, "oldLines": 7, "newStart": 9, "newLines": 8,
                "lines": [" (def v9 9)", " (def v10 10)", " (def v11 11)", "-(def v12 12)", "+(def v12 :x)", "+(def w 1)",
                          " (def v13 13)", " (def v14 14)", " (def v15 15)"]
            }]
        }),
    );
    assert_eq!(mapped(&p).unwrap().2, Some((12, 13)));
}

#[test]
fn an_edit_without_a_patch_takes_the_lines_of_its_new_text() {
    let (_d, root) = repo();
    let p = payload(
        &root,
        "Edit",
        json!({"file_path": at(&root, RULES), "old_string": "x", "new_string": "(def v20 20)\n(def v21 21)\n"}),
        json!({"filePath": at(&root, RULES)}),
    );
    assert_eq!(mapped(&p).unwrap().2, Some((20, 21)));
    // New text that is not in the file gives no lines.
    let p = payload(
        &root,
        "Edit",
        json!({"file_path": at(&root, RULES), "old_string": "x", "new_string": "not there"}),
        json!({}),
    );
    assert_eq!(mapped(&p), Some((Kind::Edit, vec![at(&root, RULES)], None)));
}

#[test]
fn grep_content_rows_give_each_path_once() {
    let (_d, root) = repo();
    let content = format!(
        "{r}:3:(def v3 3)\n{r}:4:(def v4 4)\n{a}-2-(def v2 2)\n--\n{a}:5:(def v5 5)\n",
        r = at(&root, RULES),
        a = at(&root, ALLOC)
    );
    let p = payload(
        &root,
        "Grep",
        json!({"pattern": "def", "output_mode": "content", "-n": true}),
        json!({"mode": "content", "content": content, "numLines": 5}),
    );
    assert_eq!(
        mapped(&p),
        Some((Kind::Grep, vec![at(&root, RULES), at(&root, ALLOC)], None))
    );
}

#[test]
fn grep_relative_paths_resolve_against_the_search_path_then_the_session_directory() {
    let (_d, root) = repo();
    let berth = root.join("components/berth/src/quayside/berth");
    let p = payload(
        &root,
        "Grep",
        json!({"pattern": "def", "path": berth.to_string_lossy(), "output_mode": "files_with_matches"}),
        json!({"mode": "files_with_matches", "filenames": ["rules.clj", "allocation.clj"], "numFiles": 2}),
    );
    assert_eq!(mapped(&p).unwrap().1, [at(&root, RULES), at(&root, ALLOC)]);
    let p = payload(
        &root,
        "Grep",
        json!({"pattern": "def", "output_mode": "count"}),
        json!({"mode": "count", "content": format!("{RULES}:30\n{TEST}:30\n")}),
    );
    assert_eq!(mapped(&p).unwrap().1, [at(&root, RULES), at(&root, TEST)]);
}

#[test]
fn grep_without_matched_paths_sends_nothing() {
    let (_d, root) = repo();
    let p = payload(
        &root,
        "Grep",
        json!({"pattern": "zzz"}),
        json!({"mode": "files_with_matches", "filenames": [], "numFiles": 0}),
    );
    assert_eq!(mapped(&p), None);
}

#[test]
fn paths_in_two_repos_go_to_both_and_a_path_in_no_repo_drops_out() {
    let (_d1, one) = repo();
    let (_d2, two) = repo();
    let outside = tempfile::tempdir().unwrap();
    let outside = outside.path().canonicalize().unwrap().join("x.clj");
    let content = format!(
        "{}:1:a\n{}:1:a\n{}:1:a\n",
        at(&one, RULES),
        at(&two, ALLOC),
        outside.display()
    );
    std::fs::write(&outside, "a\n").unwrap();
    let p = payload(
        &one,
        "Grep",
        json!({"pattern": "a", "output_mode": "content"}),
        json!({"mode": "content", "content": content}),
    );
    let mut got = route(&p);
    got.sort_by(|a, b| a.root.cmp(&b.root));
    let mut want = vec![
        (one.clone(), vec![at(&one, RULES)]),
        (two.clone(), vec![at(&two, ALLOC)]),
    ];
    want.sort();
    assert_eq!(got.len(), 2, "{got:?}");
    for (g, (root, paths)) in got.iter().zip(want) {
        assert_eq!(g.root, root);
        assert_eq!(
            g.event,
            Event::Trail {
                kind: Kind::Grep,
                paths,
                lines: None,
                source: Some(SOURCE.to_string()),
                session: Some("s".into()),
            }
        );
    }
}

fn hook(stdin: &str, log: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_minimap"));
    cmd.args(["hook", "claude-code"]);
    if let Some(dir) = log {
        cmd.arg("--log").arg(dir);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn assert_silent(out: &Output) {
    assert!(out.status.success(), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn malformed_stdin_an_empty_object_and_an_unknown_tool_exit_0_silently() {
    let (_d, root) = repo();
    let unknown = payload(&root, "Frobnicate", json!({}), json!({})).to_string();
    for stdin in ["", "not json", "{}", "[1, 2]", &unknown] {
        assert_silent(&hook(stdin, None));
    }
}

#[test]
fn the_hook_puts_the_event_in_the_listeners_channel_and_logs_the_payload() {
    let (_d, root) = repo();
    let l = Listener::spawn(&root).unwrap();
    let logs = tempfile::tempdir().unwrap();
    let p = payload(
        &root,
        "Read",
        json!({"file_path": at(&root, RULES)}),
        json!({}),
    );
    assert_silent(&hook(&p.to_string(), Some(logs.path())));
    assert_eq!(
        l.wait(Duration::from_secs(2)),
        Some(Event::Trail {
            kind: Kind::Read,
            paths: vec![RULES.into()],
            lines: None,
            source: Some(SOURCE.into()),
            session: Some("s".into()),
        })
    );
    let logged = std::fs::read_to_string(logs.path().join("Read-toolu_1.json")).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&logged).unwrap(), p);
}

#[test]
fn the_hook_without_a_listener_exits_0_fast() {
    let (_d, root) = repo();
    let p = payload(
        &root,
        "Edit",
        json!({"file_path": at(&root, RULES), "new_string": "(def v3 3)"}),
        json!({}),
    )
    .to_string();
    // The first run of a new binary can wait for the system's code check.
    let _ = hook(&p, None);
    let t0 = Instant::now();
    let out = hook(&p, None);
    let took = t0.elapsed();
    assert_silent(&out);
    assert!(took < Duration::from_millis(50), "took {took:?}");
}

#[test]
fn routed_events_carry_the_payloads_session() {
    // Unit 050: the payload's `session_id`, or no session without one.
    let (_d, root) = repo();
    let mut p = payload(
        &root,
        "Read",
        json!({"file_path": at(&root, RULES)}),
        json!({}),
    );
    let session = |p: &Value| match &route(p)[0].event {
        Event::Trail { session, .. } => session.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(session(&p), Some("s".to_string()));
    p.as_object_mut().unwrap().remove("session_id");
    assert_eq!(session(&p), None);
}
