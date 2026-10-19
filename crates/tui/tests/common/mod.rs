//! Helpers shared by the TUI tests.
#![allow(dead_code)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use minimap_core::{Details, History, Index};
use minimap_tui::App;
use minimap_tui::app::GitHistory;
use minimap_tui::filetree::{Entry, Row};

/// The app on the fake `quayside` repo, drawn once at `w` x `h`.
pub fn quayside(w: u16, h: u16) -> (Terminal<TestBackend>, App) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/quayside-files.json"
    );
    app_at(
        Index::from_json(&std::fs::read_to_string(path).unwrap()).unwrap(),
        w,
        h,
    )
}

/// The app on the fake `quayside` repo with the details fixture (shapes, outlines, namespaces of one directory).
pub fn quayside_details(w: u16, h: u16) -> (Terminal<TestBackend>, App) {
    let (mut t, mut app) = quayside(w, h);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/quayside-details.json"
    );
    app.details = Details::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    draw(&mut t, &mut app);
    (t, app)
}

/// The time the history fixture's day ages count back from. Any time works; ages stay the same.
pub const NOW: i64 = 1_790_000_000;

/// [`quayside_details`] with the git history of the fixture: ages and counts for every file, and recent commits for
/// one directory.
pub fn quayside_history(w: u16, h: u16) -> (Terminal<TestBackend>, App) {
    let (mut t, mut app) = quayside_details(w, h);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/quayside-details.json"
    );
    let history = History::from_json(&std::fs::read_to_string(path).unwrap(), NOW).unwrap();
    app.git = GitHistory::Ready(history);
    draw(&mut t, &mut app);
    (t, app)
}

/// A clipboard that records what it copies.
pub fn record_copies(app: &mut App) -> std::rc::Rc<std::cell::RefCell<Vec<String>>> {
    let copies = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = copies.clone();
    app.copy = Box::new(move |text| {
        sink.borrow_mut().push(text.to_string());
        Ok(())
    });
    copies
}

/// The app on an inline index JSON, drawn once at `w` x `h`.
pub fn app_at(index: Index, w: u16, h: u16) -> (Terminal<TestBackend>, App) {
    let mut app = App::new(index);
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    draw(&mut t, &mut app);
    (t, app)
}

pub fn draw(t: &mut Terminal<TestBackend>, app: &mut App) {
    t.draw(|f| minimap_tui::draw(f, app)).unwrap();
}

pub fn press(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

pub fn keys(app: &mut App, s: &str) {
    for c in s.chars() {
        press(app, KeyCode::Char(c));
    }
}

pub fn mouse(app: &mut App, kind: MouseEventKind, col: i32, row: i32) {
    app.on_mouse(MouseEvent {
        kind,
        column: col as u16,
        row: row as u16,
        modifiers: KeyModifiers::NONE,
    });
}

/// A left press and release on one cell.
pub fn click(app: &mut App, col: i32, row: i32) {
    mouse(app, MouseEventKind::Down(MouseButton::Left), col, row);
    mouse(app, MouseEventKind::Up(MouseButton::Left), col, row);
}

/// A left press at `from`, a drag to `to`, and a release there.
pub fn drag(app: &mut App, from: (i32, i32), to: (i32, i32)) {
    mouse(app, MouseEventKind::Down(MouseButton::Left), from.0, from.1);
    mouse(app, MouseEventKind::Drag(MouseButton::Left), to.0, to.1);
    mouse(app, MouseEventKind::Up(MouseButton::Left), to.0, to.1);
}

/// One screen row as text.
pub fn row(t: &Terminal<TestBackend>, y: u16) -> String {
    let b = t.backend().buffer();
    (0..b.area.width)
        .map(|x| b[(x, y)].symbol().to_string())
        .collect()
}

/// A tree row as `  name/` or `  name`, indented by depth.
pub fn label(app: &App, r: &Row) -> String {
    match r.entry {
        Entry::Dir(n) => format!("{}{}/", "  ".repeat(r.depth), app.tree.nodes[n].name),
        Entry::File(f) => format!("{}{}", "  ".repeat(r.depth), app.tree.files[f].name),
    }
}

pub fn node(app: &App, path: &str) -> usize {
    app.tree.nodes.iter().position(|n| n.path == path).unwrap()
}

pub fn file(app: &App, path: &str) -> usize {
    app.tree.files.iter().position(|f| f.path == path).unwrap()
}

/// The screen cell of a layout cell.
pub fn screen(app: &App, (x, y): (i32, i32)) -> (i32, i32) {
    let v = app.view();
    (v.mx + x - app.cam_x, v.my + y - app.cam_y)
}

/// The selected entry's layout rectangle on screen `(x0, y0, x1, y1)`, inclusive, for a file tile or a header row.
pub fn selection_rect(app: &App) -> (i32, i32, i32, i32) {
    match app.selection().unwrap() {
        Entry::File(f) => {
            let (x, y) = screen(app, app.layout.tiles[f]);
            let (tw, th) = app.layout.cfg.tile_size();
            (x, y, x + tw - 1, y + th - 1)
        }
        Entry::Dir(n) => {
            let h = app.header(n);
            let (x, y) = screen(app, (h.x, h.y));
            (x, y, x, y)
        }
    }
}

/// Asserts that the selection lies inside the map region with the scroll margin.
pub fn assert_selection_in_view(app: &App) {
    let v = app.view();
    let m = minimap_tui::app::MARGIN;
    let (x0, y0, x1, y1) = selection_rect(app);
    assert!(
        x0 >= v.mx + m && x1 <= v.mx + v.mw - 1 - m && y0 >= v.my + m && y1 <= v.my + v.mh - 1 - m,
        "selection {:?} at ({x0}, {y0})-({x1}, {y1}) is outside the map region {v:?} with margin {m}",
        app.selected_path()
    );
}
