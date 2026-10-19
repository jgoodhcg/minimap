//! Per-file git history and the heat ramps (unit `008`).

use std::path::Path;
use std::process::Command;

use minimap_core::history::{DAY, Heat, History, ago, ago_phrase, describe};

/// Runs git in `dir` with fixed author and committer dates.
fn git_at(dir: &Path, date: &str, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn write(dir: &Path, path: &str, text: &str) {
    std::fs::write(dir.join(path), text).unwrap();
}

/// 2026-01-01 00:00 UTC.
const JAN1: i64 = 1_767_225_600;

/// A repo with a merged side branch:
///
/// - Jan 1: add a, b, and gone (`feat(#1): Add a and b`)
/// - Jan 10: change a (`fix!: Fix a`)
/// - side branch, Jan 15: change b (`Change b on the side`)
/// - main, Jan 20: change a, delete gone (`chore: Touch a`)
/// - Jan 25: merge side with `--no-ff` (`Merge branch 'side'`)
/// - Feb 1: add c, change a (`docs(x): Add c`)
fn scripted_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let at = |day: &str| format!("2026-{day}T00:00:00Z");
    git_at(d, &at("01-01"), &["init", "-q"]);
    write(d, "a.txt", "a1\n");
    write(d, "b.txt", "b1\n");
    write(d, "gone.txt", "g\n");
    git_at(d, &at("01-01"), &["add", "."]);
    git_at(d, &at("01-01"), &["commit", "-qm", "feat(#1): Add a and b"]);
    write(d, "a.txt", "a2\n");
    git_at(
        d,
        &at("01-10"),
        &["commit", "-qam", "fix!: Fix a\n\nBody text."],
    );
    git_at(d, &at("01-10"), &["checkout", "-qb", "side"]);
    write(d, "b.txt", "b2\n");
    git_at(d, &at("01-15"), &["commit", "-qam", "Change b on the side"]);
    git_at(d, &at("01-15"), &["checkout", "-q", "main"]);
    write(d, "a.txt", "a3\n");
    git_at(d, &at("01-20"), &["rm", "-q", "gone.txt"]);
    git_at(d, &at("01-20"), &["commit", "-qam", "chore: Touch a"]);
    git_at(
        d,
        &at("01-25"),
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Merge branch 'side'",
            "side",
        ],
    );
    write(d, "c.txt", "c1\n");
    write(d, "a.txt", "a4\n");
    git_at(d, &at("02-01"), &["add", "."]);
    git_at(d, &at("02-01"), &["commit", "-qm", "docs(x): Add c"]);
    dir
}

fn subjects(h: &History, path: &str) -> Vec<String> {
    let f = h.file(path).unwrap();
    h.recent(f).map(|c| c.subject.clone()).collect()
}

#[test]
fn counts_ages_and_recent_subjects() {
    let dir = scripted_repo();
    let paths: Vec<String> = ["a.txt", "b.txt", "c.txt"].map(String::from).to_vec();
    let now = JAN1 + 41 * DAY; // Feb 11
    let h = History::walk(dir.path(), &paths, now).unwrap();

    let a = h.file("a.txt").unwrap();
    assert_eq!(a.count, 4);
    assert_eq!(h.age_days(a.newest), 10, "changed Feb 1");
    assert_eq!(h.age_days(a.oldest), 41, "created Jan 1");
    assert_eq!(
        subjects(&h, "a.txt"),
        [
            "docs(x): Add c",
            "chore: Touch a",
            "fix!: Fix a",
            "feat(#1): Add a and b"
        ]
    );

    // b changed on the merged branch: it counts once, and the merge commit is in no file's list.
    let b = h.file("b.txt").unwrap();
    assert_eq!(b.count, 2);
    assert_eq!(
        subjects(&h, "b.txt"),
        ["Change b on the side", "feat(#1): Add a and b"]
    );
    assert!(h.commits.iter().all(|c| !c.subject.starts_with("Merge")));

    assert_eq!(h.file("c.txt").unwrap().count, 1);
    // Paths outside the index are not recorded.
    assert!(h.file("gone.txt").is_none());
    // Hashes are full object ids.
    let head = Command::new("git")
        .current_dir(dir.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let head = String::from_utf8(head.stdout).unwrap();
    assert_eq!(h.recent(a).next().unwrap().hash, head.trim());
}

#[test]
fn a_file_keeps_its_five_newest_commits() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git_at(d, "2026-01-01T00:00:00Z", &["init", "-q"]);
    for k in 1..=7 {
        write(d, "f.txt", &format!("{k}\n"));
        let date = format!("2026-01-{k:02}T00:00:00Z");
        git_at(d, &date, &["add", "."]);
        git_at(d, &date, &["commit", "-qm", &format!("Change {k}")]);
    }
    let h = History::walk(d, &["f.txt".to_string()], JAN1 + 10 * DAY).unwrap();
    assert_eq!(h.file("f.txt").unwrap().count, 7);
    assert_eq!(
        subjects(&h, "f.txt"),
        ["Change 7", "Change 6", "Change 5", "Change 4", "Change 3"]
    );
}

#[test]
fn a_background_walk_wakes_once_after_its_result_is_ready() {
    // Unit 044: the GUI repaints on the wake, so the result must be ready to poll when it runs.
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, mpsc};
    use std::time::Duration;
    let dir = scripted_repo();
    let paths: Vec<String> = ["a.txt"].map(String::from).to_vec();
    let wakes = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();
    let w = wakes.clone();
    let loader = minimap_core::history::Loader::spawn_with(
        dir.path().to_path_buf(),
        paths,
        JAN1 + 41 * DAY,
        move || {
            w.fetch_add(1, Ordering::SeqCst);
            let _ = tx.send(());
        },
    );
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let (result, _took) = loader.poll().expect("the result is ready at the wake");
    assert_eq!(result.unwrap().file("a.txt").unwrap().count, 4);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
}

#[test]
fn an_unborn_branch_has_no_history() {
    let dir = tempfile::tempdir().unwrap();
    git_at(dir.path(), "2026-01-01T00:00:00Z", &["init", "-q"]);
    let h = History::walk(dir.path(), &["a".to_string()], JAN1).unwrap();
    assert!(h.file("a").is_none());
}

#[test]
fn descriptions_drop_the_type_prefix() {
    assert_eq!(describe("feat(#123): add x"), "add x");
    assert_eq!(describe("fix!: y"), "y");
    assert_eq!(describe("build(deps): Bump z to 1.2"), "Bump z to 1.2");
    assert_eq!(describe("Chore(Scope)!:   spaced"), "spaced");
    assert_eq!(describe("Merge branch z"), "Merge branch z");
    assert_eq!(describe("feat(unclosed: x"), "feat(unclosed: x");
    assert_eq!(
        describe("12: numbers are not a type"),
        "12: numbers are not a type"
    );
    assert_eq!(describe("wip"), "wip");
}

#[test]
fn ages_format_as_the_design_says() {
    let cases = [
        (0, "today"),
        (1, "1d"),
        (13, "13d"),
        (14, "2w"),
        (59, "8w"),
        (60, "2mo"),
        (729, "24mo"),
        (730, "2y"),
        (1000, "3y"),
    ];
    for (d, want) in cases {
        assert_eq!(ago(d), want, "{d} days");
    }
    assert_eq!(ago_phrase(0), "today");
    assert_eq!(ago_phrase(3), "3d ago");
}

#[test]
fn ramp_steps_at_their_edges() {
    let now = JAN1 + 1000 * DAY;
    let json =
        |a1: i64, c: u32| format!(r#"{{"files":[{{"p":"f","a0":999,"a1":{a1},"c":{c},"r":[]}}]}}"#);
    let step = |heat: Heat, a1: i64, c: u32| {
        let h = History::from_json(&json(a1, c), now).unwrap();
        heat.step(&h, h.file("f").unwrap())
    };
    for (days, want) in [
        (0, Some(0)),
        (14, Some(0)),
        (15, Some(1)),
        (60, Some(1)),
        (61, Some(2)),
        (365, Some(2)),
        (366, None),
    ] {
        assert_eq!(step(Heat::Age, days, 1), want, "{days} days");
    }
    for (count, want) in [
        (100, Some(0)),
        (60, Some(0)),
        (59, Some(1)),
        (25, Some(1)),
        (24, Some(2)),
        (10, Some(2)),
        (9, None),
        (1, None),
    ] {
        assert_eq!(step(Heat::Churn, 0, count), want, "{count} commits");
    }
}

#[test]
fn json_history_comes_by_path_with_day_ages() {
    let json = r#"{"root":"r","files":[
        {"p":"a.clj","a0":100,"a1":3,"c":12,"r":["h2","h1","missing"]},
        {"p":"b.md","n":1}
    ],"commits":{"h1":["h1",50,"feat(#1): Old"],"h2":["h2",3,"fix: New"]}}"#;
    let now = JAN1;
    let h = History::from_json(json, now).unwrap();
    let a = h.file("a.clj").unwrap();
    assert_eq!(
        (a.count, h.age_days(a.newest), h.age_days(a.oldest)),
        (12, 3, 100)
    );
    let recent: Vec<(&str, i64)> = h
        .recent(a)
        .map(|c| (c.hash.as_str(), h.age_days(c.time)))
        .collect();
    assert_eq!(recent, [("h2", 3), ("h1", 50)]);
    assert!(h.file("b.md").is_none());
}

#[test]
fn clock_times_of_day_at_an_offset() {
    // Unit 051: the agent history's times. 2026-10-07 15:41:07 UTC is 11:41:07 at UTC-4.
    let t = 1_791_387_667;
    assert_eq!(minimap_core::history::clock(t, 0), "15:41:07");
    assert_eq!(minimap_core::history::clock(t, -4 * 3600), "11:41:07");
    assert_eq!(minimap_core::history::clock(t, 9 * 3600), "00:41:07");
    assert_eq!(minimap_core::history::local_clock().len(), 8);
}
