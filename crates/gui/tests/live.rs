//! The live index in the window (unit `048`): a re-index rebuilds the map and keeps the selection, the tree, and the trail by path.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use minimap_core::details::{Details, Loader};
use minimap_core::events::{Event as AgentEvent, Kind as TrailKind};
use minimap_core::watch::Update;
use minimap_core::{Index, IndexOptions};
use minimap_gui::GuiApp;
use minimap_gui::camera::Camera;
use minimap_gui::scene::{Entry, Map};

/// A git repo in the temp directory, removed on drop. The GUI has no tempfile dependency.
struct Repo(PathBuf);

impl Repo {
    fn new(name: &str) -> Repo {
        let dir =
            std::env::temp_dir().join(format!("minimap-gui-live-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.canonicalize().unwrap();
        let ok = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let r = Repo(root);
        for (i, d) in ["src/app", "src/lib", "test"].iter().enumerate() {
            for k in 0..6 {
                r.write(&format!("{d}/f{k}.clj"), &"(x)\n".repeat(10 + 7 * k + i));
            }
        }
        r
    }

    fn root(&self) -> &Path {
        &self.0
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.0.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn index(&self) -> Index {
        Index::from_repo(&self.0, &IndexOptions::default()).unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app(repo: &Repo) -> GuiApp {
    let mut app = GuiApp::new(Map::new(repo.index()));
    app.set_screen(common::screen());
    app
}

fn update(repo: &Repo, changed: &[&str]) -> Update {
    Update {
        index: repo.index(),
        changed: changed.iter().map(|s| s.to_string()).collect(),
        head_moved: false,
    }
}

fn file(app: &GuiApp, path: &str) -> usize {
    match app.map.entry(path) {
        Some(Entry::File(f)) => f,
        e => panic!("{path}: {e:?}"),
    }
}

#[test]
fn a_new_file_gets_a_card_and_a_tree_row_and_the_top_bar_flashes() {
    let repo = Repo::new("new");
    let mut app = app(&repo);
    let n = app.map.tree.files.len();
    assert!(app.map.entry("src/app/new.clj").is_none());
    repo.write("src/app/new.clj", "(ns new)\n");
    app.apply_index(update(&repo, &["src/app/new.clj"]), 1.0);
    assert_eq!(app.map.tree.files.len(), n + 1);
    let f = file(&app, "src/app/new.clj");
    assert!(app.map.card(f).width() > 0.0);
    app.select(Some(Entry::File(f)), false, 1.0);
    assert!(app.tree_rows().iter().any(|r| r.entry == Entry::File(f)));
    assert_eq!(
        app.flash.as_ref().map(|f| f.0.as_str()),
        Some("1 added · 0 removed")
    );
}

#[test]
fn the_selection_keeps_its_place_on_the_screen_and_the_tree_its_open_directories() {
    let repo = Repo::new("keep");
    let mut app = app(&repo);
    let f = file(&app, "test/f3.clj");
    app.select(Some(Entry::File(f)), false, 0.0);
    let Some(Entry::Dir(lib)) = app.map.entry("src/lib") else {
        panic!()
    };
    app.set_open(lib, true);
    let cam = Camera {
        center: app.map.card(f).center(),
        zoom: 200.0 / app.map.card_w(),
    };
    app.motion.jump(app.bounds.clamp(app.region, cam));
    let before = app.cam().rect_to_screen(app.region, app.map.card(f));
    let world_before = app.map.card(f);
    // A long new file in an earlier directory moves the selected card in the world.
    repo.write("src/app/zz.clj", &"(y)\n".repeat(900));
    app.apply_index(update(&repo, &["src/app/zz.clj"]), 1.0);
    let f = file(&app, "test/f3.clj");
    assert_eq!(app.selection, Some(Entry::File(f)));
    assert_ne!(
        app.map.card(f).min,
        world_before.min,
        "the card moved in the world"
    );
    let after = app.cam().rect_to_screen(app.region, app.map.card(f));
    assert!(
        before.min.distance(after.min) < 0.01,
        "{before:?} to {after:?}"
    );
    let Some(Entry::Dir(lib)) = app.map.entry("src/lib") else {
        panic!()
    };
    assert!(app.filetree.is_open(lib), "src/lib stays open");
}

#[test]
fn a_deleted_selected_file_clears_the_selection() {
    let repo = Repo::new("gone");
    let mut app = app(&repo);
    let f = file(&app, "src/lib/f2.clj");
    app.select(Some(Entry::File(f)), false, 0.0);
    std::fs::remove_file(repo.root().join("src/lib/f2.clj")).unwrap();
    app.apply_index(update(&repo, &[]), 1.0);
    assert_eq!(app.selection, None);
    assert_eq!(app.filetree.cursor, None);
    assert_eq!(
        app.flash.as_ref().map(|f| f.0.as_str()),
        Some("0 added · 1 removed")
    );
}

#[test]
fn an_edited_file_computes_its_details_again() {
    let repo = Repo::new("edit");
    let mut app = app(&repo);
    let mut loader = Loader::with_workers(Details::repo(repo.root().to_path_buf()), || {}, 1);
    loader.request(["src/app/f1.clj".to_string()]);
    let t0 = Instant::now();
    while loader.poll().is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(2));
    }
    let lines = |l: &Loader| {
        l.get("src/app/f1.clj")
            .and_then(|d| d.shape.as_ref())
            .map(|s| s.lines)
    };
    assert_eq!(lines(&loader), Some(17));
    app.loader = Some(loader);
    let f = file(&app, "src/app/f1.clj");
    app.select(Some(Entry::File(f)), false, 0.0);
    app.pick = Some(0);
    repo.write("src/app/f1.clj", &"(z)\n".repeat(40));
    app.apply_index(update(&repo, &["src/app/f1.clj"]), 1.0);
    assert!(
        app.loader.as_ref().unwrap().get("src/app/f1.clj").is_none(),
        "forgotten"
    );
    assert_eq!(
        app.pick, None,
        "the selected file changed, so its pick cleared"
    );
    // The selected file comes first in the next request, and its new shape arrives.
    app.update_details();
    let t0 = Instant::now();
    loop {
        app.update_details();
        if lines(app.loader.as_ref().unwrap()) == Some(40) {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(5), "no new details");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn the_trail_keeps_its_marks_by_path_so_a_new_file_shows_its_edit() {
    let repo = Repo::new("trail");
    let mut app = app(&repo);
    // The agent writes a file: the event can arrive before the re-index.
    app.record_event(
        &AgentEvent::trail(TrailKind::Edit, vec!["src/lib/made.clj".to_string()]),
        1.0,
    );
    repo.write("src/lib/made.clj", "(ns made)\n");
    app.apply_index(update(&repo, &["src/lib/made.clj"]), 1.2);
    let f = file(&app, "src/lib/made.clj");
    assert_eq!(app.map.tint[f], Some((TrailKind::Edit, true)));
}
