use std::fs;
use std::path::Path;
use std::process::Command;

use minimap_core::{Config, Index, IndexFile, IndexOptions};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

fn write(dir: &Path, path: &str, bytes: &[u8]) {
    let full = dir.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, bytes).unwrap();
}

/// A repo with tracked, untracked, gitignored, binary, and deleted files.
fn fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "empty.txt", b"");
    write(d, "trailing.txt", b"a\nb\n");
    write(d, "no-trailing.txt", b"a\nb");
    write(d, "blob.bin", b"PNG\x00\x01\x02\n");
    write(d, "src/app/core.clj", b"(ns app.core)\n");
    write(d, "web/public/icons/a.svg", b"<svg/>\n");
    write(d, "web/public/icons/deep/b.svg", b"<svg/>\n");
    write(d, "gone.txt", b"x\n");
    git(d, &["add", "."]);
    fs::remove_file(d.join("gone.txt")).unwrap();
    write(d, "untracked.txt", b"new\n");
    write(d, ".gitignore", b"build/\n*.log\n");
    write(d, "build/out.txt", b"built\n");
    write(d, "src/app/debug.log", b"log\n");
    write(d, "src/app/new.clj", b"(ns app.new)\n");
    dir
}

fn find<'a>(index: &'a Index, path: &str) -> &'a IndexFile {
    index
        .files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("{path} is not in the index"))
}

/// The paths that git lists for `git ls-files --cached --others --exclude-standard`.
fn git_ls_files(dir: &Path) -> Vec<String> {
    let out = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(out.status.success());
    let mut paths: Vec<String> = out
        .stdout
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

#[test]
fn lists_tracked_and_untracked_files_that_git_does_not_ignore() {
    let repo = fixture_repo();
    let index = Index::from_repo(repo.path(), &IndexOptions::default()).unwrap();
    let paths: Vec<&str> = index.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            ".gitignore",
            "blob.bin",
            "empty.txt",
            "gone.txt",
            "no-trailing.txt",
            "src/app/core.clj",
            "src/app/new.clj",
            "trailing.txt",
            "untracked.txt",
            "web/public/icons/a.svg",
            "web/public/icons/deep/b.svg",
        ]
    );
    let untracked: Vec<&str> = index
        .files
        .iter()
        .filter(|f| f.untracked)
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(
        untracked,
        [".gitignore", "src/app/new.clj", "untracked.txt"]
    );
    assert_eq!(find(&index, "untracked.txt").lines, 1);
    assert_eq!(index.excluded, 0);
    assert_eq!(
        index.root,
        repo.path()
            .canonicalize()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
}

#[test]
fn discovers_the_root_from_a_subdirectory() {
    let repo = fixture_repo();
    let index = Index::from_repo(&repo.path().join("src/app"), &IndexOptions::default()).unwrap();
    assert_eq!(index.files.len(), 11);
}

#[test]
fn the_index_equals_git_ls_files_cached_others_exclude_standard() {
    let repo = fixture_repo();
    let index = Index::from_repo(repo.path(), &IndexOptions::default()).unwrap();
    let paths: Vec<String> = index.files.iter().map(|f| f.path.clone()).collect();
    assert_eq!(paths, git_ls_files(repo.path()));
}

#[test]
fn a_staged_new_file_is_listed_without_untracked() {
    let repo = fixture_repo();
    git(repo.path(), &["add", "src/app/new.clj"]);
    let index = Index::from_repo(repo.path(), &IndexOptions::default()).unwrap();
    assert!(!find(&index, "src/app/new.clj").untracked);
    assert!(find(&index, "untracked.txt").untracked);
}

#[test]
fn config_excludes_drop_untracked_files_too() {
    let repo = fixture_repo();
    let options = IndexOptions {
        excludes: vec!["src/app/new.clj".into()],
        ..Default::default()
    };
    let index = Index::from_repo(repo.path(), &options).unwrap();
    assert_eq!(index.excluded, 1);
    assert!(index.files.iter().all(|f| f.path != "src/app/new.clj"));
}

#[test]
fn a_rescan_reads_only_changed_files() {
    let repo = fixture_repo();
    let root = repo.path().canonicalize().unwrap();
    let git = minimap_core::index::open(&root).unwrap();
    let options = IndexOptions::default();
    let first = minimap_core::index::scan(&git, &root, &options, None).unwrap();
    // A stamp that matches keeps the old count, so a faked count shows that the file was not read again.
    let mut prev = first.clone();
    for f in &mut prev.index.files {
        if f.path == "trailing.txt" {
            f.lines = 99;
        }
    }
    let again = minimap_core::index::scan(&git, &root, &options, Some(&prev)).unwrap();
    assert_eq!(find(&again.index, "trailing.txt").lines, 99);
    write(&root, "untracked.txt", b"one\ntwo\nthree\n");
    let changed = minimap_core::index::scan(&git, &root, &options, Some(&first)).unwrap();
    assert_eq!(find(&changed.index, "untracked.txt").lines, 3);
}

#[test]
fn excludes_drop_matching_paths() {
    let repo = fixture_repo();
    let options = IndexOptions {
        excludes: vec!["web/public/icons/**".into(), "*.bin".into()],
        ..Default::default()
    };
    let index = Index::from_repo(repo.path(), &options).unwrap();
    assert_eq!(index.excluded, 3);
    assert!(
        index
            .files
            .iter()
            .all(|f| !f.path.starts_with("web/") && f.path != "blob.bin")
    );
}

#[test]
fn star_does_not_cross_a_slash() {
    let repo = fixture_repo();
    let options = IndexOptions {
        excludes: vec!["web/public/icons/*.svg".into()],
        ..Default::default()
    };
    let index = Index::from_repo(repo.path(), &options).unwrap();
    assert_eq!(index.excluded, 1);
    find(&index, "web/public/icons/deep/b.svg");
}

#[test]
fn line_counts() {
    let repo = fixture_repo();
    let index = Index::from_repo(repo.path(), &IndexOptions::default()).unwrap();
    assert_eq!(find(&index, "empty.txt").lines, 0);
    assert_eq!(find(&index, "trailing.txt").lines, 2);
    assert_eq!(find(&index, "no-trailing.txt").lines, 2);
}

#[test]
fn nul_byte_means_binary() {
    let repo = fixture_repo();
    let index = Index::from_repo(repo.path(), &IndexOptions::default()).unwrap();
    let blob = find(&index, "blob.bin");
    assert!(blob.binary);
    assert_eq!(blob.lines, 0);
    assert!(!find(&index, "trailing.txt").binary);
}

#[test]
fn deleted_tracked_file_is_missing() {
    let repo = fixture_repo();
    let index = Index::from_repo(repo.path(), &IndexOptions::default()).unwrap();
    let gone = find(&index, "gone.txt");
    assert!(gone.missing);
    assert_eq!(gone.lines, 0);
    assert!(!find(&index, "trailing.txt").missing);
}

#[test]
fn not_a_repo_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    // A temp dir can sit inside a repo on some machines, so only check the error when discovery fails.
    if gix_free_dir(dir.path()) {
        assert!(Index::from_repo(dir.path(), &IndexOptions::default()).is_err());
    }
}

fn gix_free_dir(dir: &Path) -> bool {
    !Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success()
}

#[test]
fn json_round_trips_known_fields_and_keeps_ig() {
    let text = r#"{"root":"quayside","files":[
        {"p":"README.md","n":77,"m":"AEAA","o":[[1,"h1","Readme"]],"a1":548,"a0":1601,"c":6,"r":["4eaf257ea"],"sz":9},
        {"p":"logo.png","n":0,"b":1},
        {"p":"gone.txt","x":1},
        {"p":".claude/settings.local.json","n":12,"ig":1,"ns":"x"},
        {"p":"scratch.md","n":3,"u":1}
    ],"commits":{"4eaf257ea":[3,"feat: x"]}}"#;
    let index = Index::from_json(text).unwrap();
    assert_eq!(index.root, "quayside");
    assert_eq!(index.files.len(), 5);
    assert_eq!(
        index.files[0],
        IndexFile {
            lines: 77,
            ..IndexFile::new("README.md")
        }
    );
    assert_eq!(
        index.files[1],
        IndexFile {
            binary: true,
            ..IndexFile::new("logo.png")
        }
    );
    assert_eq!(
        index.files[2],
        IndexFile {
            missing: true,
            ..IndexFile::new("gone.txt")
        }
    );
    assert_eq!(
        index.files[3],
        IndexFile {
            lines: 12,
            ignored: true,
            ..IndexFile::new(".claude/settings.local.json")
        }
    );
    assert_eq!(
        index.files[4],
        IndexFile {
            lines: 3,
            untracked: true,
            ..IndexFile::new("scratch.md")
        }
    );

    let json = index.to_json();
    assert_eq!(
        json,
        r#"{"root":"quayside","files":[{"p":"README.md","n":77},{"p":"logo.png","n":0,"b":1},{"p":"gone.txt","n":0,"x":1},{"p":".claude/settings.local.json","n":12,"ig":1},{"p":"scratch.md","n":3,"u":1}],"commits":{}}"#
    );
    assert_eq!(Index::from_json(&json).unwrap(), index);
}

#[test]
fn config_excludes_apply_to_their_repo() {
    let repo = fixture_repo();
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("config.toml");
    // The temp dir path can differ from its canonical path (macOS /var -> /private/var).
    let key = repo.path().display().to_string().replace('\\', "\\\\");
    fs::write(&cfg_path, format!("[repos.\"{key}\"]\nexclude = [\"web/public/icons/**\"]\n\n[repos.\"/elsewhere\"]\nexclude = [\"*\"]\n")).unwrap();

    let config = Config::load(Some(&cfg_path)).unwrap();
    let excludes = config.excludes_for(&repo.path().canonicalize().unwrap());
    assert_eq!(excludes, ["web/public/icons/**"]);
    let index = Index::from_repo(
        repo.path(),
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(index.excluded, 2);
    assert_eq!(index.files.len(), 9);
}

#[test]
fn a_repo_key_covers_its_worktrees() {
    // Unit 056.
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main");
    fs::create_dir(&main).unwrap();
    git(&main, &["init", "-q"]);
    fs::write(main.join("a.txt"), "a\n").unwrap();
    git(&main, &["add", "."]);
    git(&main, &["commit", "-q", "-m", "init"]);
    let wt = dir.path().join("wt");
    git(&main, &["worktree", "add", "-q", wt.to_str().unwrap()]);
    let main_key = main.canonicalize().unwrap();
    let wt_key = wt.canonicalize().unwrap();

    let config = Config::parse(&format!(
        "[repos.\"{}\"]\nexclude = [\"vendor/**\"]\ngenerated = [\"*.lock\"]\n\n[repos.\"{}\"]\nexclude = [\"wt-only/**\"]\n\n[repos.\"/elsewhere\"]\nexclude = [\"*\"]\n",
        main_key.display(),
        wt_key.display()
    ))
    .unwrap();
    assert_eq!(config.excludes_for(&wt), ["vendor/**", "wt-only/**"]);
    assert_eq!(config.generated_for(&wt), ["*.lock"]);
    assert_eq!(config.excludes_for(&main), ["vendor/**"]);
    assert_eq!(
        config.excludes_for(dir.path()),
        Vec::<String>::new(),
        "a directory outside a repo matches by path only"
    );
}

#[test]
fn explicit_config_must_exist() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Config::load(Some(&dir.path().join("nope.toml"))).is_err());
}

#[test]
fn generated_files_come_from_names_gitattributes_and_the_config() {
    // Unit 041.
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(
        d,
        ".gitattributes",
        b"*.gen.ts linguist-generated\nCargo.lock -linguist-generated\n",
    );
    write(d, "web/.gitattributes", b"api/*.json linguist-generated\n");
    write(d, "package-lock.json", b"{}\n");
    write(d, "Cargo.lock", b"x\n");
    write(d, "src/a.gen.ts", b"x\n");
    write(d, "src/a.ts", b"x\n");
    write(d, "web/api/x.json", b"{}\n");
    write(d, "web/b.json", b"{}\n");
    write(d, "schema/s.json", b"{}\n");
    git(d, &["add", "."]);
    // An untracked lock file is generated too.
    write(d, "web/yarn.lock", b"x\n");
    let options = IndexOptions {
        generated: vec!["schema/".into()],
        ..Default::default()
    };
    let index = Index::from_repo(d, &options).unwrap();
    let generated: Vec<&str> = index
        .files
        .iter()
        .filter(|f| f.generated)
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(
        generated,
        [
            "package-lock.json",
            "schema/s.json",
            "src/a.gen.ts",
            "web/api/x.json",
            "web/yarn.lock"
        ]
    );
    assert!(find(&index, "web/yarn.lock").untracked);
    // The JSON keeps the flag.
    let back = Index::from_json(&index.to_json()).unwrap();
    assert_eq!(back.files, index.files);
    // A bad rule stops the index and names the rule.
    let bad = IndexOptions {
        generated: vec!["a[".into()],
        ..Default::default()
    };
    let err = Index::from_repo(d, &bad).unwrap_err();
    assert!(format!("{err:#}").contains("a["), "{err:#}");
}

#[test]
fn json_marks_built_in_names_and_keeps_g() {
    // `g` is there only when a file differs from the built-in list: 1 for another generated file, 0 for an unmarked lock file.
    let text = r#"{"root":"r","files":[{"p":"a.json","n":3,"g":1},{"p":"web/yarn.lock","n":5},{"p":"Cargo.lock","n":4,"g":0},{"p":"b.json","n":2}],"commits":{}}"#;
    let index = Index::from_json(text).unwrap();
    let flags: Vec<bool> = index.files.iter().map(|f| f.generated).collect();
    assert_eq!(flags, [true, true, false, false]);
    assert_eq!(index.to_json(), text);
}

#[test]
fn config_generated_rules_are_global_then_per_repo() {
    let repo = tempfile::tempdir().unwrap();
    let key = repo.path().display().to_string().replace('\\', "\\\\");
    let config = Config::parse(&format!(
        "[map]\ngenerated = [\"*.min.js\"]\n\n[repos.\"{key}\"]\nexclude = [\"x/**\"]\ngenerated = [\"schema/\", \"!yarn.lock\"]\n\n[repos.\"/elsewhere\"]\ngenerated = [\"*\"]\n"
    ))
    .unwrap();
    let root = repo.path().canonicalize().unwrap();
    assert_eq!(
        config.generated_for(&root),
        ["*.min.js", "schema/", "!yarn.lock"]
    );
    assert_eq!(config.excludes_for(&root), ["x/**"]);
    assert_eq!(
        Config::parse("").unwrap().generated_for(&root),
        Vec::<String>::new()
    );
}
