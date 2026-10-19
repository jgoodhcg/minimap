//! Editor handoff (unit 031): the steps of every preset and place, with a runner that records them.

mod common;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ratatui::crossterm::event::KeyCode;

use common::*;
use minimap_core::config::EditorConfig;
use minimap_core::{Content, Details};
use minimap_tui::editor::{
    self, Editor, Herdr, Outcome, Place, PlaceState, Preset, Request, Runner, Settings,
};
use minimap_tui::filetree::Entry;

/// What the fake runner saw: `run`, `spawn`, and `rm` steps, as one line each.
type Log = Rc<RefCell<Vec<String>>>;

/// A runner that records each step and answers from a script: the first rule whose prefix starts the command line.
struct Fake {
    log: Log,
    rules: Vec<(String, Result<String, String>)>,
}

impl Fake {
    fn answer(&self, line: &str) -> Result<String, String> {
        self.rules
            .iter()
            .find(|(prefix, _)| line.starts_with(prefix.as_str()))
            .map_or(Ok(String::new()), |(_, r)| r.clone())
    }
}

impl Runner for Fake {
    fn run(&mut self, argv: &[String]) -> Result<String, String> {
        let line = argv.join(" ");
        self.log.borrow_mut().push(format!("run {line}"));
        self.answer(&line)
    }

    fn spawn(&mut self, argv: &[String]) -> Result<(), String> {
        let line = argv.join(" ");
        self.log.borrow_mut().push(format!("spawn {line}"));
        self.answer(&line).map(|_| ())
    }

    fn remove_file(&mut self, path: &Path) {
        self.log.borrow_mut().push(format!("rm {}", path.display()));
    }
}

const TAB_JSON: &str =
    r#"{"result":{"root_pane":{"pane_id":"w1:p9"},"tab":{"tab_id":"w1:t9"},"type":"tab_created"}}"#;
const SPLIT_JSON: &str =
    r#"{"result":{"pane":{"pane_id":"w1:p7","tab_id":"w1:t1"},"type":"pane_info"}}"#;

fn herdr() -> Herdr {
    Herdr {
        workspace: "w1".to_string(),
        pane: "w1:p1".to_string(),
    }
}

fn req() -> Request {
    Request {
        path: PathBuf::from("/repo/src/a b.clj"),
        line: 42,
        col: 1,
        root: PathBuf::from("/repo"),
    }
}

fn editor(
    preset: Preset,
    place: Place,
    with_herdr: bool,
    rules: &[(&str, Result<&str, &str>)],
) -> (Editor, Log) {
    let log: Log = Rc::default();
    let rules = rules
        .iter()
        .map(|(p, r)| (p.to_string(), r.map(str::to_string).map_err(str::to_string)))
        .collect();
    let e = Editor {
        settings: Ok(Settings { preset, place }),
        herdr: with_herdr.then(herdr),
        runner: Box::new(Fake {
            log: log.clone(),
            rules,
        }),
        state: PlaceState::default(),
    };
    (e, log)
}

fn socket() -> String {
    editor::nvim_socket(Path::new("/repo"))
        .display()
        .to_string()
}

#[test]
fn nvim_first_open_creates_a_focused_tab_and_starts_nvim_on_the_socket() {
    let s = socket();
    let (mut e, log) = editor(
        Preset::Nvim,
        Place::Tab,
        true,
        &[
            ("nvim --server", Err("no server")),
            ("herdr tab create", Ok(TAB_JSON)),
        ],
    );
    assert_eq!(e.open(&req()), Ok(Outcome::Opened));
    assert_eq!(
        log.borrow().as_slice(),
        [
            format!("run nvim --server {s} --remote-expr 1"),
            "run herdr tab create --workspace w1 --cwd /repo --label minimap editor --focus"
                .to_string(),
            format!("rm {s}"),
            format!(
                "run herdr pane run w1:p9 nvim --listen {s} +42 -c 'normal! zt' '/repo/src/a b.clj'"
            ),
            "run herdr tab focus w1:t9".to_string(),
        ]
    );
    assert_eq!(e.state.pane.as_deref(), Some("w1:p9"));
}

#[test]
fn a_later_nvim_open_goes_to_the_socket_and_focuses_the_tab() {
    let s = socket();
    let (mut e, log) = editor(Preset::Nvim, Place::Tab, true, &[]);
    e.state = PlaceState {
        pane: Some("w1:p9".to_string()),
        tab: Some("w1:t9".to_string()),
    };
    assert_eq!(e.open(&req()), Ok(Outcome::Opened));
    let keys = r"<C-\><C-N>:execute 'drop ' .. fnameescape('/repo/src/a b.clj') | call cursor(42, 1) | normal! zt<CR>";
    assert_eq!(
        log.borrow().as_slice(),
        [
            format!("run nvim --server {s} --remote-expr 1"),
            format!("run nvim --server {s} --remote-send {keys}"),
            "run herdr tab focus w1:t9".to_string(),
        ]
    );
}

#[test]
fn nvim_restarts_in_its_pane_or_in_a_new_place() {
    let s = socket();
    // The user quit nvim, and the pane is still there.
    let (mut e, log) = editor(
        Preset::Nvim,
        Place::Tab,
        true,
        &[("nvim --server", Err("gone"))],
    );
    e.state = PlaceState {
        pane: Some("w1:p9".to_string()),
        tab: Some("w1:t9".to_string()),
    };
    e.open(&req()).unwrap();
    assert_eq!(log.borrow()[1], "run herdr pane get w1:p9");
    assert!(log.borrow()[3].starts_with(&format!("run herdr pane run w1:p9 nvim --listen {s}")));
    // The pane is gone too: a new tab.
    let (mut e, log) = editor(
        Preset::Nvim,
        Place::Tab,
        true,
        &[
            ("nvim --server", Err("gone")),
            ("herdr pane get", Err("pane_not_found")),
            ("herdr tab create", Ok(TAB_JSON)),
        ],
    );
    e.state.pane = Some("w1:p9".to_string());
    e.open(&req()).unwrap();
    assert!(log.borrow()[2].starts_with("run herdr tab create"));
}

#[test]
fn splits_open_beside_minimap_and_focus_by_direction() {
    for (place, dir) in [(Place::Right, "right"), (Place::Down, "down")] {
        let (mut e, log) = editor(
            Preset::Nvim,
            place,
            true,
            &[
                ("nvim --server", Err("no server")),
                ("herdr pane split", Ok(SPLIT_JSON)),
            ],
        );
        e.open(&req()).unwrap();
        let log = log.borrow();
        assert_eq!(
            log[1],
            format!("run herdr pane split w1:p1 --direction {dir} --cwd /repo")
        );
        assert!(log[3].starts_with("run herdr pane run w1:p7 nvim --listen"));
        assert_eq!(
            log[4],
            format!("run herdr pane focus --pane w1:p1 --direction {dir}")
        );
        assert_eq!(e.state.tab, None);
    }
}

#[test]
fn vim_starts_in_its_place_and_later_gets_drop_typed_into_it() {
    let (mut e, log) = editor(
        Preset::Vim,
        Place::Tab,
        true,
        &[("herdr tab create", Ok(TAB_JSON))],
    );
    e.open(&req()).unwrap();
    assert_eq!(
        log.borrow()[1],
        "run herdr pane run w1:p9 vim +42 -c 'normal! zt' '/repo/src/a b.clj'"
    );
    let running =
        r#"{"result":{"process_info":{"foreground_processes":[{"name":"vim","pid":1}]}}}"#;
    let (mut e, log) = editor(
        Preset::Vim,
        Place::Tab,
        true,
        &[("herdr pane process-info", Ok(running))],
    );
    e.state = PlaceState {
        pane: Some("w1:p9".to_string()),
        tab: Some("w1:t9".to_string()),
    };
    e.open(&req()).unwrap();
    assert_eq!(
        log.borrow().as_slice(),
        [
            "run herdr pane process-info --pane w1:p9",
            "run herdr pane send-keys w1:p9 Escape",
            "run herdr pane send-text w1:p9 :execute 'drop ' .. fnameescape('/repo/src/a b.clj') | call cursor(42, 1) | normal! zt",
            "run herdr pane send-keys w1:p9 Enter",
            "run herdr tab focus w1:t9",
        ]
    );
    // The pane runs a shell, not vim: vim starts again in it.
    let shell = r#"{"result":{"process_info":{"foreground_processes":[{"name":"zsh","pid":1}]}}}"#;
    let (mut e, log) = editor(
        Preset::Vim,
        Place::Tab,
        true,
        &[("herdr pane process-info", Ok(shell))],
    );
    e.state.pane = Some("w1:p9".to_string());
    e.open(&req()).unwrap();
    assert!(log.borrow()[2].starts_with("run herdr pane run w1:p9 vim +42"));
}

#[test]
fn without_herdr_terminal_editors_take_minimaps_terminal() {
    for (preset, bin) in [(Preset::Nvim, "nvim"), (Preset::Vim, "vim")] {
        let (mut e, log) = editor(preset, Place::Tab, false, &[]);
        let want: Vec<String> = [bin, "+42", "-c", "normal! zt", "/repo/src/a b.clj"]
            .map(String::from)
            .to_vec();
        assert_eq!(e.open(&req()), Ok(Outcome::Suspend(want)));
        assert!(log.borrow().is_empty());
    }
}

#[test]
fn gui_editors_and_custom_commands_run_detached() {
    let cases = [
        (Preset::Vscode, "spawn code --goto /repo/src/a b.clj:42:1"),
        (
            Preset::Emacs,
            "spawn emacsclient -n +42:1 /repo/src/a b.clj",
        ),
        (
            Preset::Custom {
                command: ["zed", "{path}:{line}:{col}", "--root={root}"]
                    .map(String::from)
                    .to_vec(),
                terminal: false,
            },
            "spawn zed /repo/src/a b.clj:42:1 --root=/repo",
        ),
    ];
    for (preset, want) in cases {
        for with_herdr in [true, false] {
            let (mut e, log) = editor(preset.clone(), Place::Tab, with_herdr, &[]);
            assert_eq!(e.open(&req()), Ok(Outcome::Opened));
            assert_eq!(log.borrow().as_slice(), [want]);
        }
    }
    let term = Preset::Custom {
        command: ["hx", "{path}:{line}"].map(String::from).to_vec(),
        terminal: true,
    };
    let (mut e, _) = editor(term, Place::Tab, true, &[]);
    assert_eq!(
        e.open(&req()),
        Ok(Outcome::Suspend(vec![
            "hx".to_string(),
            "/repo/src/a b.clj:42".to_string()
        ]))
    );
}

#[test]
fn a_failed_step_returns_its_error() {
    let (mut e, _) = editor(
        Preset::Vscode,
        Place::Tab,
        false,
        &[("code", Err("code: not found"))],
    );
    assert_eq!(e.open(&req()), Err("code: not found".to_string()));
    let (mut e, _) = editor(
        Preset::Nvim,
        Place::Tab,
        true,
        &[("nvim --server", Err("x")), ("herdr tab create", Ok("{}"))],
    );
    assert_eq!(
        e.open(&req()),
        Err("herdr did not return the new pane's ID.".to_string())
    );
}

#[test]
fn settings_come_from_the_config_then_visual_then_editor() {
    let cfg = |preset: Option<&str>, place: Option<&str>| EditorConfig {
        preset: preset.map(String::from),
        place: place.map(String::from),
        ..EditorConfig::default()
    };
    let preset = |c: &EditorConfig, v: Option<&str>, e: Option<&str>| {
        Settings::resolve(c, v, e).map(|s| s.preset)
    };
    assert_eq!(preset(&cfg(None, None), None, None), Ok(Preset::Nvim));
    assert_eq!(
        preset(&cfg(None, None), Some("code --wait"), Some("vim")),
        Ok(Preset::Vscode)
    );
    assert_eq!(
        preset(&cfg(None, None), None, Some("/usr/bin/vim")),
        Ok(Preset::Vim)
    );
    assert_eq!(
        preset(&cfg(None, None), Some("nano"), Some("emacsclient -t")),
        Ok(Preset::Emacs)
    );
    assert_eq!(
        preset(&cfg(Some("vscode"), None), Some("nvim"), None),
        Ok(Preset::Vscode)
    );
    assert!(
        preset(&cfg(Some("kakoune"), None), None, None)
            .unwrap_err()
            .contains("Unknown [editor] preset")
    );
    assert!(
        preset(&cfg(Some("custom"), None), None, None)
            .unwrap_err()
            .contains("needs a command")
    );
    let place = |p| Settings::resolve(&cfg(None, p), None, None).map(|s| s.place);
    assert_eq!(place(None), Ok(Place::Tab));
    assert_eq!(place(Some("down")), Ok(Place::Down));
    assert!(place(Some("left")).is_err());
}

#[test]
fn quoting_keeps_paths_with_spaces_quotes_and_angle_brackets_intact() {
    assert_eq!(editor::shell_quote("/a/b.clj"), "/a/b.clj");
    assert_eq!(editor::shell_quote("/a b/it's.clj"), r"'/a b/it'\''s.clj'");
    let r = Request {
        path: PathBuf::from("/r/it's <x>.clj"),
        ..req()
    };
    assert_eq!(
        editor::vim_command(&r),
        ":execute 'drop ' .. fnameescape('/r/it''s <x>.clj') | call cursor(42, 1) | normal! zt"
    );
    let send = editor::nvim_send(Path::new("/tmp/s.sock"), &r);
    assert!(send[4].contains("<lt>x>.clj"), "{}", send[4]);
    assert!(send[4].starts_with(r"<C-\><C-N>:execute") && send[4].ends_with("<CR>"));
}

#[test]
fn the_socket_path_is_short_enough_for_a_unix_socket() {
    let long = PathBuf::from(format!("/Users/{}/projects/repo", "x".repeat(200)));
    let s = editor::nvim_socket(&long);
    assert!(s.to_string_lossy().len() < 100, "{}", s.display());
    assert_ne!(
        editor::nvim_socket(Path::new("/a")),
        editor::nvim_socket(Path::new("/b"))
    );
    assert_eq!(
        editor::nvim_socket(Path::new("/a")),
        editor::nvim_socket(Path::new("/a"))
    );
}

#[test]
fn the_nvim_socket_name_did_not_change_when_the_hash_moved_to_the_core() {
    // Unit 010 moved the FNV-1a hash to `minimap_core::events`. The name must stay, so a running nvim is found.
    let s = editor::nvim_socket(Path::new("/repo"));
    let name = s.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("minimap-"), "{name}");
    assert!(name.ends_with("-f7556aa5a4543342.sock"), "{name}");
}

/// Quayside with the berth fixture on disk at its quayside path, and an editor that records its steps.
fn quayside_on_disk() -> (
    ratatui::Terminal<ratatui::backend::TestBackend>,
    minimap_tui::App,
    Log,
    tempfile::TempDir,
) {
    let (mut t, mut app) = quayside(150, 45);
    let dir = tempfile::tempdir().unwrap();
    let path = "components/berth/src/quayside/berth/core.clj";
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/clojure/berth.clj"
    ))
    .unwrap();
    std::fs::create_dir_all(dir.path().join("components/berth/src/quayside/berth")).unwrap();
    std::fs::write(dir.path().join(path), &text).unwrap();
    app.content = Content::Repo(dir.path().to_path_buf());
    app.details = Details::memory(HashMap::from([(path.to_string(), text)]));
    let (e, log) = editor(Preset::Vscode, Place::Tab, false, &[]);
    app.editor = e;
    let f = file(&app, path);
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    (t, app, log, dir)
}

#[test]
fn e_uses_the_viewer_line_then_the_outline_item_then_line_1() {
    let (mut t, mut app, log, dir) = quayside_on_disk();
    let at = |line: u32| {
        format!(
            "spawn code --goto {}:{line}:1",
            dir.path()
                .join("components/berth/src/quayside/berth/core.clj")
                .display()
        )
    };
    keys(&mut app, "e");
    assert_eq!(log.borrow().last(), Some(&at(1)));
    assert_eq!(
        app.flash_text(),
        Some("Opened components/berth/src/quayside/berth/core.clj:1 in VS Code")
    );
    keys(&mut app, "ojje");
    assert_eq!(log.borrow().last(), Some(&at(13)), "the third outline item");
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    draw(&mut t, &mut app);
    keys(&mut app, "30Ge");
    assert_eq!(log.borrow().last(), Some(&at(30)), "the viewer's cursor");
}

#[test]
fn e_on_a_directory_or_without_a_repo_says_why() {
    let (_, mut app, log, _dir) = quayside_on_disk();
    let n = node(&app, "components/berth/src/quayside/berth");
    app.select(Entry::Dir(n), true);
    keys(&mut app, "e");
    assert_eq!(app.flash_text(), Some("Select a file to open it."));
    let (_, mut plain) = quayside(150, 45);
    keys(&mut plain, "e");
    assert_eq!(
        plain.flash_text(),
        Some("No repo on disk to open files from.")
    );
    assert!(log.borrow().is_empty());
}

#[test]
fn a_failed_open_flashes_and_the_app_keeps_running() {
    let (_, mut app, _, _dir) = quayside_on_disk();
    let (e, _) = editor(
        Preset::Vscode,
        Place::Tab,
        false,
        &[("code", Err("code: No such file or directory"))],
    );
    app.editor = e;
    keys(&mut app, "e");
    assert_eq!(app.flash_text(), Some("code: No such file or directory"));
    assert!(!app.quit);
}

#[test]
fn a_terminal_editor_without_herdr_asks_the_main_loop_to_suspend() {
    let (_, mut app, _, dir) = quayside_on_disk();
    let (e, _) = editor(Preset::Nvim, Place::Tab, false, &[]);
    app.editor = e;
    keys(&mut app, "e");
    let path = dir
        .path()
        .join("components/berth/src/quayside/berth/core.clj");
    assert_eq!(
        app.suspend,
        Some(
            [
                "nvim",
                "+1",
                "-c",
                "normal! zt",
                &path.display().to_string()
            ]
            .map(String::from)
            .to_vec()
        )
    );
}
