//! App state and input: the one selection, the camera, the overview, the file tree, the selection pulse,
//! the file picker, the outline browser, git history and heat, the file viewer, the flash message, the help overlay,
//! the live index, and the agent's trail.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use ratatui::style::Color;

use minimap_core::events::{Event as AgentEvent, Listener};
use minimap_core::fuzzy::{self, Candidate, Pattern, Ranked};
use minimap_core::history::{FileHistory, Heat, Loader};
use minimap_core::layout::Header;
use minimap_core::outline::{OutlineItem, language_for};
use minimap_core::watch::{Update, Watcher};
use minimap_core::{Content, Details, FileDetails, History, Index, Layout, LayoutConfig, Tree};

use crate::editor::{self, Editor, Outcome};
use crate::filetree::{self, Entry, FileTree, Row, Step};
use crate::keymap::{Command, Key, Keymap, Lookup, Mode};
use crate::search::Search;
use crate::syntax::{self, Colored, Done, Job, Worker};
use crate::text::{kilo, thousands};
use crate::trail::{Mark, Trail};
use crate::viewer::{self, Action, Viewer};

/// Pan step for `a`/`d`, in columns. Shift multiplies it by 4.
pub const PAN_X: i32 = 11;
/// Pan step for `w`/`s`, in rows.
pub const PAN_Y: i32 = 2;
/// File tree width at startup, the step of `[` and `]`, and its limits.
pub const TREE_W: i32 = 34;
pub const TREE_STEP: i32 = 4;
pub const TREE_MIN: i32 = 16;
pub const TREE_MAX: i32 = 80;
/// The selection panel needs this many terminal columns.
pub const PANEL_MIN_COLS: i32 = 100;
/// The tree narrows so the map keeps this many columns beside the tree's edge column.
const MAP_MIN: i32 = 30;
/// The camera shows the selection with this margin, in cells.
pub const MARGIN: i32 = 2;
/// How long a flash message stays in the top bar.
pub const FLASH: Duration = Duration::from_millis(3200);
/// One cycle of the selection pulse.
pub const PULSE_PERIOD: Duration = Duration::from_secs(2);
/// The redraw interval while the selection pulses.
pub const PULSE_FRAME: Duration = Duration::from_millis(80);
/// How often the main loop checks for the git history while it loads.
pub const HISTORY_POLL: Duration = Duration::from_millis(100);
/// How often the main loop checks the live inputs: the watcher (unit `034`) and the event socket (unit `010`).
pub const LIVE_POLL: Duration = Duration::from_millis(100);
/// How often the panel's trail age redraws.
pub const TRAIL_TICK: Duration = Duration::from_secs(1);
/// How often the main loop checks for syntax colors while a file is highlighted.
pub const SYNTAX_POLL: Duration = Duration::from_millis(40);
/// Two clicks on one tile within this time are a double-click.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// The colored files kept in memory.
pub const COLORED_KEEP: usize = 8;

/// The git history of the repo (unit 008).
pub enum GitHistory {
    /// No history source, as for an index JSON without git fields.
    None,
    /// The walk runs on a background thread.
    Loading(Loader),
    Ready(History),
    Failed(String),
}

/// Screen regions for a terminal size (`docs/design.md` `Screen regions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub cols: i32,
    pub rows: i32,
    /// File tree width, 0 while it is hidden. The tree's right edge is the divider column `tw`.
    pub tw: i32,
    /// Selection panel width: 0 while it is hidden or below 100 columns, 44 below 160, else 54.
    pub rw: i32,
    /// The map region, below the top bar: columns `mx..mx + mw`, rows `my..my + mh`.
    pub mx: i32,
    pub my: i32,
    pub mw: i32,
    pub mh: i32,
}

impl View {
    pub fn new(cols: i32, rows: i32, tree_on: bool, tree_w: i32, panel_on: bool) -> View {
        let rw = if !panel_on || cols < PANEL_MIN_COLS {
            0
        } else if cols < 160 {
            44
        } else {
            54
        };
        let tw = if tree_on {
            tree_w.min(cols - rw - 1 - MAP_MIN).max(TREE_MIN)
        } else {
            0
        };
        let mx = if tw > 0 { tw + 1 } else { 0 };
        View {
            cols,
            rows,
            tw,
            rw,
            mx,
            my: 1,
            mw: (cols - rw - mx).max(0),
            mh: (rows - 2).max(0),
        }
    }

    /// The tree rows' rectangle `(y, h)`: below the `FILES` row, above the key hint row.
    pub fn tree_rows(&self) -> (i32, i32) {
        (2, (self.rows - 4).max(0))
    }

    pub fn in_map(&self, col: i32, row: i32) -> bool {
        col >= self.mx && col < self.mx + self.mw && row >= self.my && row < self.my + self.mh
    }
}

/// The overview box in the map region's top-left corner (`ovRect` in the v5 mockup).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overview {
    /// Overview cells per layout cell.
    pub s: f64,
    /// The box's top-left screen cell, and its size with the 1-cell margin.
    pub x: i32,
    pub y: i32,
    pub bw: i32,
    pub bh: i32,
    /// The braille area inside the margin, in cells.
    pub w: i32,
    pub h: i32,
}

impl Overview {
    pub fn contains(&self, col: i32, row: i32) -> bool {
        col >= self.x && col < self.x + self.bw && row >= self.y && row < self.y + self.bh
    }

    /// The viewport rectangle `(x0, y0, x1, y1)` in braille cells, inclusive, for a camera and a map region size.
    pub fn viewport(&self, cam: (i32, i32), mw: i32, mh: i32) -> (i32, i32, i32, i32) {
        let cell = |a: i32| f64::from(a) * self.s;
        let x0 = (cell(cam.0).floor() as i32).max(0);
        let y0 = (cell(cam.1).floor() as i32).max(0);
        let x1 = ((cell(cam.0 + mw).ceil() as i32) - 1)
            .max(x0)
            .min(self.w - 1);
        let y1 = ((cell(cam.1 + mh).ceil() as i32) - 1)
            .max(y0)
            .min(self.h - 1);
        (x0, y0, x1, y1)
    }
}

enum Drag {
    /// Panning the map from a start cell and camera. `moved` stays false until the pointer leaves the start cell.
    Map {
        col: i32,
        row: i32,
        cam_x: i32,
        cam_y: i32,
        moved: bool,
    },
    /// Resizing the tree by its right edge.
    TreeEdge,
    /// Moving the view by the overview.
    Overview,
}

/// Copies text to the clipboard, or returns why it could not.
pub type CopyFn = Box<dyn FnMut(&str) -> Result<(), String>>;

pub struct App {
    pub index: Index,
    pub tree: Tree,
    pub layout: Layout,
    pub total_lines: u64,
    /// The layout cell at the map region's top-left corner.
    pub cam_x: i32,
    pub cam_y: i32,
    pub help: bool,
    pub quit: bool,
    /// Draw Nerd Font file-type icons (`--no-icons` turns them off).
    pub icons: bool,
    /// Draw hidden files and dot-directories. `.` toggles it. Their slots stay reserved either way.
    pub show_hidden: bool,
    /// The file tree. Its cursor is the selection.
    pub filetree: FileTree,
    /// Show the file tree (`t`).
    pub tree_on: bool,
    /// The tree width that `[`, `]`, and the edge drag set. [`View::new`] can draw it narrower.
    pub tree_w: i32,
    /// Show the selection panel (`p`). Below [`PANEL_MIN_COLS`] it is hidden anyway.
    pub panel_on: bool,
    /// Show the overview (`v`).
    pub overview_on: bool,
    /// The file or directory header under the mouse on the map.
    pub hover: Option<Entry>,
    /// The time since start. The main loop sets it, and tests set it directly.
    pub now: Duration,
    /// The selection pulses (`m` toggles it). Off, it holds the middle of the pulse.
    pub pulse: bool,
    /// A top-bar message and the time it ends.
    pub flash: Option<(String, Duration)>,
    /// The file picker, from `/` until Esc (unit 026).
    pub search: Option<Search>,
    /// File details for the selection panel (unit 007). The binary sets the source.
    pub details: Details,
    /// Outline browse mode (`o`): the file and its highlighted outline item. It ends when the selection changes.
    outline: Option<(usize, usize)>,
    /// The first outline row in the panel. The panel scrolls it while drawing, to keep the highlighted item in view.
    pub outline_top: Cell<usize>,
    /// The git history (unit 008). The binary starts the walk.
    pub git: GitHistory,
    /// A walk that restarted after `HEAD` moved (unit `034`). The shown history stays until it ends.
    history_reload: Option<Loader>,
    /// The working-tree watcher (unit `034`). The binary starts it in repo mode.
    pub watcher: Option<Watcher>,
    /// The agent's trail (unit `010`), and layer 1 that shows it.
    pub trail: Trail,
    pub trail_on: bool,
    /// The event socket (unit `010`). The binary starts it in repo mode.
    pub listener: Option<Listener>,
    /// The heat layer on the map: `4` age, `5` churn, or none.
    pub heat: Option<Heat>,
    /// File text for the viewer (unit 029). The binary sets the source.
    pub content: Content,
    /// The file viewer, in place of the map, from Enter until Esc.
    pub viewer: Option<Viewer>,
    /// Where each viewed file's viewer closed: `(cursor, top)`, for this session.
    viewed: HashMap<usize, (usize, usize)>,
    /// A `.tmTheme` for the viewer, from the config. None uses the built-in theme.
    pub theme_path: Option<PathBuf>,
    /// The highlighting thread, started at the first view.
    syntax: Option<Worker>,
    /// The colored files, oldest first, at most [`COLORED_KEEP`].
    colored: Vec<Arc<Colored>>,
    /// Paths sent to the highlighting thread and not back yet.
    highlighting: HashSet<String>,
    /// Outlines from the grammars' definition scopes, by path, for files that `minimap-core` does not outline (unit 032).
    outlines: HashMap<String, Arc<Vec<OutlineItem>>>,
    /// Paths whose outline-only job has not come back yet.
    outlining: HashSet<String>,
    /// The theme's text color for plain text, once known.
    pub text_fg: Option<Color>,
    /// The last click on the map, for double-clicks: its time and cell.
    last_click: Option<(Duration, i32, i32)>,
    /// The editor for `e` (unit 031). The binary sets it up.
    pub editor: Editor,
    /// A command for the main loop to run in minimap's terminal: a terminal editor without herdr.
    pub suspend: Option<Vec<String>>,
    /// Each file prepared once for the fuzzy matcher.
    candidates: Vec<Candidate>,
    /// The file indices in file tree order: each directory's subdirectories, then its files.
    tree_order: Vec<usize>,
    /// The clipboard for `y`. The default has none; the binary installs the system clipboard.
    pub copy: CopyFn,
    /// The keys (unit `033`): the defaults, with the config's changes when the binary sets them.
    pub keymap: Arc<Keymap>,
    /// The keys of a sequence typed so far in the map, outline, or help, and the mode they belong to.
    pending: (Mode, Vec<Key>),
    /// The header index of each node.
    header_of: Vec<usize>,
    view: View,
    drag: Option<Drag>,
    /// A selection made before the first draw scrolls the map at the first resize.
    show_pending: bool,
}

/// The state that follows from an index: the tree, the layout, and the lookups on them.
struct Built {
    tree: Tree,
    layout: Layout,
    total_lines: u64,
    candidates: Vec<Candidate>,
    tree_order: Vec<usize>,
    header_of: Vec<usize>,
}

impl Built {
    fn new(index: &Index) -> Built {
        let tree = Tree::from_index(index);
        let layout = Layout::compute(&tree, &LayoutConfig::CHARS);
        let total_lines = index.files.iter().map(|f| f.lines).sum();
        let candidates = tree.files.iter().map(|f| Candidate::new(&f.path)).collect();
        let tree_order = tree_order(&tree, 0);
        let mut header_of = vec![0; tree.nodes.len()];
        for (h, header) in layout.headers.iter().enumerate() {
            header_of[header.node] = h;
        }
        Built {
            tree,
            layout,
            total_lines,
            candidates,
            tree_order,
            header_of,
        }
    }
}

impl App {
    pub fn new(index: Index) -> App {
        let Built {
            tree,
            layout,
            total_lines,
            candidates,
            tree_order,
            header_of,
        } = Built::new(&index);
        let filetree = FileTree::new(&tree);
        let mut app = App {
            index,
            tree,
            layout,
            total_lines,
            cam_x: 0,
            cam_y: 0,
            help: false,
            quit: false,
            icons: true,
            show_hidden: true,
            filetree,
            tree_on: true,
            tree_w: TREE_W,
            panel_on: true,
            overview_on: true,
            hover: None,
            now: Duration::ZERO,
            pulse: true,
            flash: None,
            search: None,
            details: Details::default(),
            outline: None,
            outline_top: Cell::new(0),
            git: GitHistory::None,
            history_reload: None,
            watcher: None,
            trail: Trail::default(),
            trail_on: true,
            listener: None,
            heat: None,
            content: Content::None,
            viewer: None,
            viewed: HashMap::new(),
            theme_path: None,
            syntax: None,
            colored: Vec::new(),
            highlighting: HashSet::new(),
            outlines: HashMap::new(),
            outlining: HashSet::new(),
            text_fg: None,
            last_click: None,
            editor: Editor::none(),
            suspend: None,
            candidates,
            tree_order,
            copy: Box::new(|_| Err("no clipboard".to_string())),
            keymap: Keymap::shared_default(),
            pending: (Mode::Map, Vec::new()),
            header_of,
            view: View::new(0, 0, true, TREE_W, true),
            drag: None,
            show_pending: false,
        };
        // Start on the first visible file that is not at the repo root, in index order.
        let first = (0..app.tree.files.len()).find(|&f| {
            let tf = &app.tree.files[f];
            !tf.ignored && !tf.hidden && tf.node != 0
        });
        match first {
            Some(f) => app.select(Entry::File(f), true),
            None => app.filetree.cursor = app.tree_rows().first().map(|r| r.entry),
        }
        app
    }

    pub fn view(&self) -> View {
        self.view
    }

    /// Records the terminal size, clamps the camera to it, and sizes the tree.
    pub fn resize(&mut self, area: Rect) {
        self.view = View::new(
            i32::from(area.width),
            i32::from(area.height),
            self.left_on(),
            self.tree_w,
            self.panel_on,
        );
        if let Some(s) = &mut self.search {
            s.set_rows(self.view.tree_rows().1 as usize);
        }
        self.clamp();
        if let Some(vw) = &mut self.viewer {
            vw.set_height(self.view.mh.max(1) as usize);
        }
        self.filetree.height = self.view.tree_rows().1 as usize;
        let rows = self.tree_rows();
        self.filetree.clamp_top(rows.len());
        if self.show_pending {
            self.show_pending = false;
            self.filetree.keep_visible(&rows);
            self.show_selection();
        }
    }

    /// `N files · M dirs · K lines` for the top of the selection panel.
    pub fn header_stats(&self) -> String {
        format!(
            "{} files · {} dirs · {} lines",
            thousands(self.index.files.len() as u64),
            thousands(self.tree.nodes.len() as u64),
            kilo(self.total_lines)
        )
    }

    pub fn tree_rows(&self) -> Vec<Row> {
        self.filetree.rows(&self.tree, self.show_hidden)
    }

    pub fn selection(&self) -> Option<Entry> {
        self.filetree.cursor
    }

    pub fn selected_file(&self) -> Option<usize> {
        match self.selection() {
            Some(Entry::File(f)) => Some(f),
            _ => None,
        }
    }

    pub fn selected_dir(&self) -> Option<usize> {
        match self.selection() {
            Some(Entry::Dir(n)) => Some(n),
            _ => None,
        }
    }

    pub fn header(&self, node: usize) -> Header {
        self.layout.headers[self.header_of[node]]
    }

    /// A file that is drawn: not ignored, and not hidden while hidden files are hidden.
    pub fn file_visible(&self, f: usize) -> bool {
        let tf = &self.tree.files[f];
        !tf.ignored && (self.show_hidden || !tf.hidden)
    }

    /// Selects an entry. The tree opens down to it and scrolls to it. With `scroll`, the map scrolls to it too.
    pub fn select(&mut self, e: Entry, scroll: bool) {
        if self.filetree.cursor != Some(e) {
            self.outline_top.set(0);
        }
        self.outline = None;
        self.filetree.cursor = Some(e);
        self.filetree.reveal(&self.tree, e);
        let rows = self.tree_rows();
        self.filetree.keep_visible(&rows);
        if scroll {
            self.show_selection();
        }
    }

    /// Scrolls the map only as far as needed to show the selection with a [`MARGIN`].
    /// For a directory, the target is its header and as much of the block as fits.
    pub fn show_selection(&mut self) {
        let v = self.view;
        if v.mw == 0 || v.mh == 0 {
            // Before the first draw there is no view yet.
            self.show_pending = true;
            return;
        }
        match self.selection() {
            Some(Entry::File(f)) => {
                let (x, y) = self.layout.tiles[f];
                let (tw, th) = self.layout.cfg.tile_size();
                self.ensure_visible(x, y, x + tw - 1, y + th - 1);
            }
            Some(Entry::Dir(n)) => {
                let h = self.header(n);
                let w = h.w.min(v.mw - 8).max(1);
                let hh = h.h.min(v.mh - 4).max(1);
                self.ensure_visible(h.x, h.y, h.x + w - 1, h.y + hh - 1);
            }
            None => {}
        }
    }

    /// Moves the camera the least distance that puts the layout rectangle (inclusive corners) inside the map region
    /// with a [`MARGIN`]. The top-left corner wins when the rectangle does not fit (`ensureVisible` in the v5 mockup).
    /// A rectangle that would start under the overview moves right of it.
    fn ensure_visible(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (v, m) = (self.view, MARGIN);
        if x0 - self.cam_x < m {
            self.cam_x = x0 - m;
        } else if x1 - self.cam_x > v.mw - 1 - m {
            self.cam_x = x1 - (v.mw - 1 - m);
        }
        if y0 - self.cam_y < m {
            self.cam_y = y0 - m;
        } else if y1 - self.cam_y > v.mh - 1 - m {
            self.cam_y = y1 - (v.mh - 1 - m);
        }
        if let Some(o) = self.overview()
            && x0 - self.cam_x < o.bw + 1
            && y0 - self.cam_y < o.bh
        {
            self.cam_x = x0 - (o.bw + 2);
        }
        self.clamp();
    }

    /// The overview: at most 26 x 9 cells and 30 % of the map region, with the map's aspect, inside a 1-cell margin.
    /// `None` while `v` hides it.
    pub fn overview(&self) -> Option<Overview> {
        if !self.overview_on {
            return None;
        }
        let v = self.view;
        let (lw, lh) = (f64::from(self.layout.w), f64::from(self.layout.h));
        let max_w = (f64::from(v.mw) * 0.3).floor().clamp(4.0, 26.0);
        let max_h = (f64::from(v.mh) * 0.3).floor().clamp(2.0, 9.0);
        let mut s = max_w / lw;
        if lh * s > max_h {
            s = max_h / lh;
        }
        let w = ((lw * s).ceil() as i32).max(1);
        let h = ((lh * s).ceil() as i32).max(1);
        Some(Overview {
            s,
            x: v.mx,
            y: v.my,
            bw: w + 2,
            bh: h + 2,
            w,
            h,
        })
    }

    /// Keeps the camera within 2 columns and 1 row of the layout edges (`docs/design.md` `Camera`).
    /// On the left it can go past the overview, so every tile can pan out from under it.
    pub fn clamp(&mut self) {
        let v = self.view;
        let left = self.overview().map_or(2, |o| o.bw + 2);
        self.cam_x = self.cam_x.min(self.layout.w - v.mw + 2).max(-left);
        self.cam_y = self.cam_y.min(self.layout.h - v.mh + 1).max(-1);
    }

    /// Centers the view on the layout point under an overview cell, then clamps the camera.
    fn overview_jump(&mut self, col: i32, row: i32) {
        let v = self.view;
        let Some(o) = self.overview() else {
            return;
        };
        // `Math.round`: halves round up.
        let round = |a: f64| (a + 0.5).floor() as i32;
        let at =
            |c: i32, c0: i32, m: i32| round((f64::from(c - c0) + 0.5) / o.s - f64::from(m) / 2.0);
        self.cam_x = at(col, o.x + 1, v.mw);
        self.cam_y = at(row, o.y + 1, v.mh);
        self.clamp();
    }

    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.cam_x += dx;
        self.cam_y += dy;
        self.clamp();
    }

    /// Shows or hides the tree, or sets its width. The map content keeps its place on screen,
    /// and the camera moves only when the selection would go out of view.
    fn set_tree(&mut self, on: bool, w: i32) {
        self.tree_on = on;
        self.tree_w = w.clamp(TREE_MIN, TREE_MAX);
        self.relayout();
    }

    /// `p`: shows or hides the selection panel. It needs [`PANEL_MIN_COLS`] columns.
    fn toggle_panel(&mut self) {
        if self.view.cols < PANEL_MIN_COLS {
            self.flash(format!("The panel needs {PANEL_MIN_COLS} columns"));
            return;
        }
        self.panel_on = !self.panel_on;
        self.relayout();
    }

    /// `v`: shows or hides the overview. The camera moves only as the clamp requires.
    fn toggle_overview(&mut self) {
        self.overview_on = !self.overview_on;
        self.clamp();
    }

    /// Recomputes the view after a column change, keeping the map content in place on screen.
    fn relayout(&mut self) {
        let old = self.view;
        self.view = View::new(
            old.cols,
            old.rows,
            self.left_on(),
            self.tree_w,
            self.panel_on,
        );
        self.cam_x += self.view.mx - old.mx;
        self.clamp();
        if self.view.mw != old.mw && self.view.mw > 0 {
            self.show_selection();
        }
    }

    /// The flash message while it lasts.
    pub fn flash_text(&self) -> Option<&str> {
        match &self.flash {
            Some((m, until)) if self.now < *until => Some(m),
            _ => None,
        }
    }

    pub fn flash(&mut self, msg: String) {
        self.flash = Some((msg, self.now + FLASH));
    }

    /// The pulse level from 0 to 1: a sine over [`PULSE_PERIOD`], or 0.5 while the pulse is off.
    pub fn pulse_level(&self) -> f64 {
        if !self.pulse {
            return 0.5;
        }
        let phase = self.now.as_secs_f64() / PULSE_PERIOD.as_secs_f64();
        0.5 + 0.5 * (phase * std::f64::consts::TAU).sin()
    }

    /// How long the main loop can wait for input before the screen needs a redraw. `None`: until the next input.
    pub fn next_redraw(&self) -> Option<Duration> {
        let flash = match &self.flash {
            Some((_, until)) if self.now < *until => Some(*until - self.now),
            _ => None,
        };
        let pulse = (self.pulse && self.selection().is_some()).then_some(PULSE_FRAME);
        let git = (matches!(self.git, GitHistory::Loading(_)) || self.history_reload.is_some())
            .then_some(HISTORY_POLL);
        let colors =
            (!self.highlighting.is_empty() || !self.outlining.is_empty()).then_some(SYNTAX_POLL);
        // The panel's trail age counts seconds and minutes.
        let trail_age = self
            .selected_file()
            .and_then(|f| self.trail_mark(f))
            .map(|_| TRAIL_TICK);
        [flash, pulse, git, colors, trail_age]
            .into_iter()
            .flatten()
            .min()
    }

    /// Starts the git history walk for the indexed files on a background thread.
    pub fn start_history(&mut self, root: PathBuf, now: i64) {
        let paths = self.index.files.iter().map(|f| f.path.clone()).collect();
        self.git = GitHistory::Loading(Loader::spawn(root, paths, now));
    }

    /// Takes the git history when the walk has finished. [`crate::draw`] calls it before each frame.
    /// A walk that restarted after `HEAD` moved replaces the shown history when it ends.
    pub fn poll_history(&mut self) {
        let done = |r: Result<History, anyhow::Error>| match r {
            Ok(h) => GitHistory::Ready(h),
            Err(e) => GitHistory::Failed(format!("{e:#}")),
        };
        if let Some((result, _took)) = self.history_reload.as_ref().and_then(Loader::poll) {
            self.history_reload = None;
            self.git = done(result);
        }
        let GitHistory::Loading(loader) = &self.git else {
            return;
        };
        if let Some((result, _took)) = loader.poll() {
            self.git = done(result);
        }
    }

    /// Restarts the git history walk after `HEAD` moved (unit `034`). A walk that is still loading starts over;
    /// a loaded history stays on screen until the new walk ends.
    pub fn restart_history(&mut self, now: i64) {
        let Some(root) = self.content.root().map(|r| r.to_path_buf()) else {
            return;
        };
        let paths = self.index.files.iter().map(|f| f.path.clone()).collect();
        let loader = Loader::spawn(root, paths, now);
        match self.git {
            GitHistory::Ready(_) => self.history_reload = Some(loader),
            _ => self.git = GitHistory::Loading(loader),
        }
    }

    /// The git history once it has loaded.
    pub fn history(&self) -> Option<&History> {
        match &self.git {
            GitHistory::Ready(h) => Some(h),
            _ => None,
        }
    }

    /// A file's git history, once loaded. `None` also for a file without commits.
    pub fn file_history(&self, f: usize) -> Option<&FileHistory> {
        self.history()?.file(&self.index.files[f].path)
    }

    /// The heat ramp step of a file while a heat layer is on, 0 for the strongest.
    pub fn heat_step(&self, f: usize) -> Option<usize> {
        let (heat, h) = (self.heat?, self.history()?);
        heat.step(h, h.file(&self.index.files[f].path)?)
    }

    /// `4` / `5`: turns a heat layer on, or off when it is on. The layers are exclusive. No tile moves.
    /// A layer can turn on while the history loads, but not without a history.
    fn toggle_heat(&mut self, heat: Heat) {
        let on = self.heat != Some(heat);
        match &self.git {
            GitHistory::None if on => self.flash("No git history for this repo.".to_string()),
            GitHistory::Failed(_) if on => self.flash("Git history failed to load.".to_string()),
            _ => self.heat = on.then_some(heat),
        }
    }

    /// `H`: copies the selected file's newest commit hash.
    fn copy_commit(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        let msg = match &self.git {
            GitHistory::None => "No git history for this repo.",
            GitHistory::Loading(_) => "Git history is still loading.",
            GitHistory::Failed(_) => "Git history failed to load.",
            GitHistory::Ready(h) => {
                let hash = h
                    .file(&self.index.files[f].path)
                    .and_then(|fh| h.recent(fh).next())
                    .map(|c| c.hash.clone());
                match hash {
                    Some(hash) => return self.copy_text("commit", &hash),
                    None => "No commits for this file.",
                }
            }
        };
        self.flash(msg.to_string());
    }

    /// The repo-relative path of the selection.
    pub fn selected_path(&self) -> Option<String> {
        match self.selection()? {
            Entry::File(f) => Some(self.tree.files[f].path.clone()),
            Entry::Dir(n) => Some(self.tree.nodes[n].path.clone()),
        }
    }

    fn copy_selection(&mut self) {
        let Some(path) = self.selected_path() else {
            return;
        };
        self.copy_text("path", &path);
    }

    /// Copies text to the clipboard and flashes what happened.
    fn copy_text(&mut self, what: &str, text: &str) {
        let msg = match (self.copy)(text) {
            Ok(()) => format!("Copied {what}: {text}"),
            Err(_) => format!("Copy failed: {text}"),
        };
        self.flash(msg);
    }

    /// Loads the selected file's details. [`crate::draw`] calls it before each frame.
    /// A file that `minimap-core` cannot outline gets an outline job on the highlighting thread, once (unit 032).
    pub fn load_details(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        self.details.load(&self.index.files[f]);
        let file = &self.index.files[f];
        let path = &file.path;
        if file.binary
            || file.missing
            || language_for(path).is_some()
            || self.outlines.contains_key(path)
            || self.outlining.contains(path)
            || self.highlighting.contains(path)
        {
            return;
        }
        let path = path.clone();
        let text = match self.content.text(&path) {
            Ok(Some(t)) if !syntax::too_long(&t) => t,
            // No text, or too long: an empty outline, so the file is not read again.
            _ => {
                self.outlines.insert(path, Arc::new(Vec::new()));
                return;
            }
        };
        let theme = self.theme_path.clone();
        let worker = self.syntax.get_or_insert_with(|| Worker::spawn(theme));
        worker.request(Job {
            path: path.clone(),
            text: Arc::new(text),
            color: false,
            head: Content::None,
            outline_only: true,
        });
        self.outlining.insert(path);
    }

    /// A file's outline: `minimap-core`'s for the languages it outlines (Clojure), else the grammar's (unit 032).
    pub fn outline(&self, f: usize) -> &[OutlineItem] {
        let path = &self.index.files[f].path;
        if language_for(path).is_some() {
            return self.file_details(f).map_or(&[], |d| &d.outline);
        }
        self.outlines.get(path).map_or(&[], |o| o.as_slice())
    }

    /// A file's details, once loaded.
    pub fn file_details(&self, f: usize) -> Option<&FileDetails> {
        self.details.get(&self.index.files[f].path)
    }

    /// `Y`: copies the selected file's namespace.
    fn copy_namespace(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        self.load_details();
        match self.file_details(f).and_then(|d| d.namespace.clone()) {
            Some(ns) => self.copy_text("namespace", &ns),
            None => self.flash("This file has no namespace. Press y to copy its path.".to_string()),
        }
    }

    /// The highlighted outline item while the selected file's outline is browsed.
    pub fn outline_browse(&self) -> Option<usize> {
        match self.outline {
            Some((f, k)) if self.selected_file() == Some(f) => Some(k),
            _ => None,
        }
    }

    /// `o`: browses the selected file's outline in the selection panel.
    fn open_outline(&mut self) {
        let Some(f) = self.selected_file() else {
            self.flash("Select a file to browse its outline.".to_string());
            return;
        };
        if self.view.rw == 0 {
            let msg = if self.view.cols < PANEL_MIN_COLS {
                format!(
                    "The outline is in the selection panel, which needs {PANEL_MIN_COLS} columns."
                )
            } else {
                "The outline is in the selection panel. Press p to show it.".to_string()
            };
            self.flash(msg);
            return;
        }
        self.load_details();
        if self.outline(f).is_empty() {
            let loading = self.outlining.contains(&self.index.files[f].path);
            let msg = if loading {
                "The outline is still loading."
            } else {
                "This file has no outline."
            };
            self.flash(msg.to_string());
            return;
        }
        self.outline = Some((f, 0));
    }

    /// An action while the outline is browsed: move, leave, view the file at the item, or open the editor there.
    fn outline_command(&mut self, cmd: Command) {
        let Some((f, k)) = self.outline else {
            return;
        };
        let n = self.outline(f).len();
        match cmd {
            Command::Close => self.outline = None,
            Command::Down => self.outline = Some((f, (k + 1).min(n.max(1) - 1))),
            Command::Up => self.outline = Some((f, k.saturating_sub(1))),
            Command::Open => {
                let line = self.outline(f).get(k).map(|o| o.line.max(1) as usize - 1);
                // A viewer that opens ends browse mode; one that cannot flashes why, and browsing goes on.
                self.open_viewer(f, line, true);
            }
            Command::OpenEditor => {
                let line = self.outline(f).get(k).map_or(1, |o| o.line as usize);
                self.open_editor(f, line);
            }
            Command::Quit => self.quit = true,
            _ => {}
        }
    }

    /// Opens the viewer on a file (unit 029). With `line`, the cursor goes there, at the top of the view with `at_top`.
    /// Without it, the viewer opens where this file's viewer last closed, else at line 1.
    pub fn open_viewer(&mut self, f: usize, line: Option<usize>, at_top: bool) {
        let file = &self.index.files[f];
        if file.binary {
            return self.flash("Binary file: nothing to view.".to_string());
        }
        if file.missing {
            return self.flash("The file is not in the working tree.".to_string());
        }
        if self.content.is_none() {
            return self.flash("No file contents in --from-json mode.".to_string());
        }
        let path = self.tree.files[f].path.clone();
        let text = match self.content.text(&path) {
            Ok(Some(t)) => t,
            Ok(None) => return self.flash(format!("No text for {path}.")),
            Err(e) => return self.flash(format!("{e:#}")),
        };
        self.store_view();
        let mut vw = Viewer::new(f, &path, &text);
        vw.keys = self.keymap.clone();
        vw.set_height(self.view.mh.max(1) as usize);
        match (line, self.viewed.get(&f)) {
            (Some(l), _) => {
                vw.go(l);
                if at_top {
                    vw.to_top();
                }
            }
            (None, Some(&(cursor, top))) => {
                vw.top = top;
                vw.go(cursor);
            }
            (None, None) => {}
        }
        vw.plain = syntax::too_long(&text);
        // Colors and the diff against HEAD come from the background thread (units 029 and 030).
        // The cache is used only while the text is the same, so a file changed on disk gets a new diff.
        let cached = self
            .colored_for(&path)
            .filter(|c| c.text.as_deref() == Some(&text))
            .map(|c| (c.diff.clone(), c.head_text.clone()));
        match cached {
            Some((Some(d), old)) => vw.set_diff(d, old),
            Some((None, _)) => {}
            None if !self.highlighting.contains(&path) => {
                // Stale colors of an older text would not line up: plain text shows until the new ones arrive.
                self.colored.retain(|c| c.path != path);
                let theme = self.theme_path.clone();
                let worker = self.syntax.get_or_insert_with(|| Worker::spawn(theme));
                worker.request(Job {
                    path: path.clone(),
                    text: Arc::new(text),
                    color: !vw.plain,
                    head: self.content.clone(),
                    outline_only: false,
                });
                self.highlighting.insert(path);
            }
            None => {}
        }
        if self.filetree.cursor != Some(Entry::File(f)) {
            self.select(Entry::File(f), true);
        }
        self.outline = None;
        self.viewer = Some(vw);
    }

    /// Remembers where the open viewer is, for the next view of its file.
    fn store_view(&mut self) {
        if let Some(vw) = &self.viewer {
            self.viewed.insert(vw.file, (vw.cursor, vw.top));
        }
    }

    /// Closes the viewer. The map shows as it was: the camera does not change.
    pub fn close_viewer(&mut self) {
        self.store_view();
        self.viewer = None;
    }

    /// A file's syntax colors, once the highlighting thread has sent them.
    pub fn colored_for(&self, path: &str) -> Option<&Colored> {
        self.colored
            .iter()
            .rev()
            .find(|c| c.path == path)
            .map(|c| c.as_ref())
    }

    /// Takes the finished syntax colors. [`crate::draw`] calls it before each frame.
    pub fn poll_syntax(&mut self) {
        let Some(worker) = &self.syntax else {
            return;
        };
        for done in worker.poll() {
            match done {
                Done::Foreground(fg) => self.text_fg = Some(fg),
                Done::ThemeError(e) => self.flash(e),
                Done::Outline(path, outline) => {
                    self.outlining.remove(&path);
                    self.outlines.insert(path, outline);
                }
                Done::Colored(c) => {
                    self.highlighting.remove(&c.path);
                    // Colors of a text that the viewer no longer shows came from a job before a reload (unit `034`).
                    // The newer job follows.
                    let stale = self.viewer.as_ref().is_some_and(|vw| {
                        vw.path == c.path && c.text.as_deref().is_some_and(|t| !vw.shows(t))
                    });
                    if stale {
                        continue;
                    }
                    if !c.lines.is_empty() {
                        self.outlines.insert(c.path.clone(), c.outline.clone());
                    }
                    if let (Some(vw), Some(d)) = (self.viewer.as_mut(), &c.diff)
                        && vw.path == c.path
                    {
                        vw.set_diff(d.clone(), c.head_text.clone());
                    }
                    self.colored.retain(|old| old.path != c.path);
                    self.colored.push(c);
                    if self.colored.len() > COLORED_KEEP {
                        self.colored.remove(0);
                    }
                }
            }
        }
    }

    /// Waits until no file is being highlighted or outlined, for tests.
    pub fn wait_for_colors(&mut self, timeout: Duration) {
        let t0 = std::time::Instant::now();
        while !(self.highlighting.is_empty() && self.outlining.is_empty()) && t0.elapsed() < timeout
        {
            std::thread::sleep(Duration::from_millis(5));
            self.poll_syntax();
        }
    }

    /// The viewer on a file: its first and last working-tree lines in view and its cursor line, 1-based.
    pub fn viewing(&self, f: usize) -> Option<(u32, u32, u32)> {
        let vw = self.viewer.as_ref().filter(|vw| vw.file == f)?;
        let last = (vw.top + vw.height).min(vw.len()) - 1;
        let line = |row: usize| vw.line_of_row(row) as u32 + 1;
        Some((line(vw.top), line(last), line(vw.cursor)))
    }

    /// A file's changed working-tree lines, 1-based, once its diff is known.
    pub fn changed_lines(&self, f: usize) -> Vec<u32> {
        self.colored_for(&self.tree.files[f].path)
            .and_then(|c| c.diff.as_ref())
            .map(|d| d.changed_lines().iter().map(|&l| l as u32 + 1).collect())
            .unwrap_or_default()
    }

    /// `e`: opens a file at a 1-based line in the editor (unit 031).
    pub fn open_editor(&mut self, f: usize, line: usize) {
        let file = &self.index.files[f];
        if file.binary {
            return self.flash("Binary file: open it outside minimap.".to_string());
        }
        if file.missing {
            return self.flash("The file is not in the working tree.".to_string());
        }
        let Some(root) = self.content.root().map(|r| r.to_path_buf()) else {
            return self.flash("No repo on disk to open files from.".to_string());
        };
        let path = self.tree.files[f].path.clone();
        let req = editor::Request {
            path: root.join(&path),
            line: line.max(1),
            col: 1,
            root,
        };
        let name = self.editor.name().to_string();
        match self.editor.open(&req) {
            Ok(Outcome::Opened) => self.flash(format!("Opened {path}:{} in {name}", req.line)),
            Ok(Outcome::Suspend(cmd)) => self.suspend = Some(cmd),
            Err(e) => self.flash(e),
        }
    }

    /// Takes the watcher's update, if one arrived, and applies it. Returns true when the screen needs a redraw.
    pub fn poll_watch(&mut self) -> bool {
        let Some(w) = &self.watcher else {
            return false;
        };
        let (update, error) = w.poll();
        let redraw = update.is_some() || error.is_some();
        if let Some(e) = error {
            self.flash(format!("Re-index failed: {e}"));
        }
        if let Some(u) = update {
            self.apply_index(u);
        }
        redraw
    }

    /// True while a live input runs, so the main loop checks it every [`LIVE_POLL`].
    pub fn live(&self) -> bool {
        self.watcher.is_some() || self.listener.is_some()
    }

    /// Checks the watcher and the event socket. Returns true when the screen needs a redraw.
    pub fn poll_live(&mut self) -> bool {
        let index = self.poll_watch();
        let events = self.poll_events();
        index || events
    }

    /// Takes the events that arrived on the socket. Returns true when any did.
    pub fn poll_events(&mut self) -> bool {
        let Some(l) = &self.listener else {
            return false;
        };
        let events = l.poll();
        for e in &events {
            self.record_event(e);
        }
        !events.is_empty()
    }

    /// Records an agent event on the trail (unit `010`). No tile moves, and the camera does not change.
    pub fn record_event(&mut self, event: &AgentEvent) {
        self.trail.record(event, self.now);
    }

    /// A file's trail mark and whether it is fresh, while layer 1 is on.
    pub fn trail_mark(&self, f: usize) -> Option<(&Mark, bool)> {
        if !self.trail_on {
            return None;
        }
        let m = self.trail.get(&self.tree.files[f].path)?;
        Some((m, self.trail.is_fresh(m)))
    }

    /// Replaces the index after a re-index (unit `034`). The tree, the layout, and their lookups are rebuilt.
    /// The selection, the open directories, the viewer, the viewer places, outline browse, and the search
    /// keep their files by path. The selected entry keeps its place on the screen.
    pub fn apply_index(&mut self, update: Update) {
        let Update {
            index,
            changed,
            head_moved,
        } = update;
        let built = Built::new(&index);
        let file_of: HashMap<&str, usize> = built
            .tree
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.as_str(), i))
            .collect();
        let node_of: HashMap<&str, usize> = built
            .tree
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.path.as_str(), i))
            .collect();
        // Old file and node indices to new ones, by path.
        let file_map: Vec<Option<usize>> = (self.tree.files.iter())
            .map(|f| file_of.get(f.path.as_str()).copied())
            .collect();
        let node_map: Vec<Option<usize>> = (self.tree.nodes.iter())
            .map(|n| node_of.get(n.path.as_str()).copied())
            .collect();
        let file = |f: usize| file_map.get(f).copied().flatten();
        let entry = |e: Entry| match e {
            Entry::File(f) => file(f).map(Entry::File),
            Entry::Dir(n) => node_map.get(n).copied().flatten().map(Entry::Dir),
        };
        let kept = file_map.iter().flatten().count();
        let added = built.tree.files.len() - kept;
        let removed = self.tree.files.len() - kept;
        let moved = file_map
            .iter()
            .enumerate()
            .filter(|&(f, nf)| nf.is_some_and(|nf| built.layout.tiles[nf] != self.layout.tiles[f]))
            .count();

        // The selection by path; a removed entry gives way to its nearest remaining directory.
        let place = |app: &App, e: Entry| match e {
            Entry::File(f) => app.layout.tiles[f],
            Entry::Dir(n) => {
                let h = app.header(n);
                (h.x, h.y)
            }
        };
        let old_sel = self.selection();
        let kept_sel = old_sel.and_then(entry);
        let new_sel = kept_sel.or_else(|| {
            let path = match old_sel? {
                Entry::File(f) => &self.tree.files[f].path,
                Entry::Dir(n) => &self.tree.nodes[n].path,
            };
            let mut dir = path.as_str();
            while let Some((up, _)) = dir.rsplit_once('/') {
                if let Some(&n) = node_of.get(up) {
                    return Some(Entry::Dir(n));
                }
                dir = up;
            }
            None
        });
        let old_place = old_sel.map(|e| place(self, e));
        let open: Vec<usize> = (0..self.tree.nodes.len())
            .filter(|&n| self.filetree.is_open(n))
            .filter_map(|n| node_map[n])
            .collect();
        let hover = self.hover.and_then(entry);
        let outline = self.outline.and_then(|(f, k)| file(f).map(|nf| (nf, k)));
        let viewed: HashMap<usize, (usize, usize)> = self
            .viewed
            .iter()
            .filter_map(|(&f, &at)| file(f).map(|nf| (nf, at)))
            .collect();
        let viewer_file = self.viewer.as_ref().map(|vw| file(vw.file));
        drop(file_of);
        drop(node_of);

        let top = self.filetree.top;
        self.index = index;
        self.tree = built.tree;
        self.layout = built.layout;
        self.total_lines = built.total_lines;
        self.candidates = built.candidates;
        self.tree_order = built.tree_order;
        self.header_of = built.header_of;
        self.filetree = FileTree::new(&self.tree);
        self.filetree.height = self.view.tree_rows().1 as usize;
        self.filetree.top = top;
        for n in open {
            self.filetree.set_open(n, true);
        }
        self.hover = hover;
        self.outline = outline;
        self.viewed = viewed;

        match new_sel {
            Some(e) => {
                self.filetree.cursor = Some(e);
                self.filetree.reveal(&self.tree, e);
            }
            None => self.filetree.cursor = self.tree_rows().first().map(|r| r.entry),
        }
        let rows = self.tree_rows();
        self.filetree.clamp_top(rows.len());
        self.filetree.keep_visible(&rows);
        // The selected entry keeps its screen place: the camera moves as far as its tile moved.
        if let (Some(e), Some((x0, y0))) = (kept_sel, old_place) {
            let (x1, y1) = place(self, e);
            self.cam_x += x1 - x0;
            self.cam_y += y1 - y0;
        }
        self.clamp();

        if let Some(mut s) = self.search.take() {
            let results = self.rank_files(&Pattern::parse(&s.query));
            let total = (0..self.tree.files.len())
                .filter(|&f| self.file_visible(f))
                .count();
            s.remap(file, entry, results, total);
            self.search = Some(s);
        }

        for path in &changed {
            self.details.forget(path);
            self.outlines.remove(path);
            self.colored.retain(|c| &c.path != path);
        }
        let mut deleted = None;
        match viewer_file {
            Some(None) => {
                deleted = self.viewer.take().map(|vw| vw.path);
            }
            Some(Some(nf)) => {
                if let Some(vw) = &mut self.viewer {
                    vw.file = nf;
                }
                let path = self.tree.files[nf].path.clone();
                if self.index.files[nf].missing {
                    self.viewer = None;
                    deleted = Some(path);
                } else if changed.contains(&path) || head_moved {
                    self.reload_viewer(changed.contains(&path));
                }
            }
            None => {}
        }
        if head_moved {
            self.restart_history(minimap_core::history::now());
        }
        match deleted {
            Some(path) => self.flash(format!("{path} was deleted.")),
            None if added + removed > 0 => self.flash(format!(
                "{added} added · {removed} removed · {moved} tiles moved"
            )),
            None => {}
        }
    }

    /// Reads the viewer's file again when its text changed, and asks the background thread for new colors and a new
    /// diff against `HEAD`. A file that cannot be read closes the viewer.
    fn reload_viewer(&mut self, text_changed: bool) {
        let Some(path) = self.viewer.as_ref().map(|vw| vw.path.clone()) else {
            return;
        };
        let text = match self.content.text(&path) {
            Ok(Some(t)) => t,
            Ok(None) => return,
            Err(e) => {
                self.viewer = None;
                return self.flash(format!("{e:#}"));
            }
        };
        let Some(vw) = &mut self.viewer else {
            return;
        };
        if text_changed {
            vw.reload(&text);
            vw.plain = syntax::too_long(&text);
        }
        let color = !vw.plain;
        let theme = self.theme_path.clone();
        let worker = self.syntax.get_or_insert_with(|| Worker::spawn(theme));
        worker.request(Job {
            path: path.clone(),
            text: Arc::new(text),
            color,
            head: self.content.clone(),
            outline_only: false,
        });
        self.highlighting.insert(path);
    }

    /// Keys while the viewer is open.
    fn on_viewer_key(&mut self, key: KeyEvent) {
        let Some(file) = self.viewer.as_ref().map(|vw| vw.file) else {
            return;
        };
        let outline: Vec<u32> = self.outline(file).iter().map(|o| o.line).collect();
        let Some(vw) = &mut self.viewer else {
            return;
        };
        match vw.key(key, &outline) {
            Action::None => {}
            Action::Close => self.close_viewer(),
            Action::Help => self.help = true,
            Action::Copy(loc) => self.copy_text("location", &loc),
            Action::Flash(m) => self.flash(m),
            Action::Editor => {
                let (f, line) = (vw.file, vw.cursor_line() + 1);
                self.open_editor(f, line);
            }
            Action::ToggleTree => self.set_tree(!self.tree_on, self.tree_w),
            Action::TogglePanel => self.toggle_panel(),
        }
    }

    /// The left column shows the tree, or the picker while it is open.
    fn left_on(&self) -> bool {
        self.tree_on || self.search.as_ref().is_some_and(|s| s.active)
    }

    fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        let visible: Vec<bool> = (0..self.tree.files.len())
            .map(|f| self.file_visible(f))
            .collect();
        if let Some(s) = &mut self.search {
            s.retain_kept(|f| visible[f]);
        }
        let rows = self.tree_rows();
        if self.filetree.cursor_index(&rows).is_none() {
            // The toggle hid the selection: it moves to the first visible row.
            if let Some(r) = rows.first() {
                self.select(r.entry, true);
            }
        }
        let rows = self.tree_rows();
        self.filetree.keep_visible(&rows);
    }

    /// `j` / `k`: the next or previous tree row.
    fn tree_move(&mut self, delta: isize) {
        let rows = self.tree_rows();
        if rows.is_empty() {
            return;
        }
        let j = match self.filetree.cursor_index(&rows) {
            Some(i) => (i as isize + delta).clamp(0, rows.len() as isize - 1) as usize,
            None => 0,
        };
        self.select(rows[j].entry, true);
    }

    /// `g` / `G`: the first or last tree row.
    fn tree_end(&mut self, last: bool) {
        let rows = self.tree_rows();
        let row = if last { rows.last() } else { rows.first() };
        if let Some(r) = row {
            self.select(r.entry, true);
        }
    }

    /// `l`: open a closed directory; on an open directory, go to its first child (`FileTree::step_in`, shared with the GUI).
    fn tree_in(&mut self) {
        let rows = self.tree_rows();
        let step = self.filetree.step_in(&self.tree, &rows, self.selection());
        self.tree_step(step);
    }

    /// `h`: close an open directory; else go to the parent directory (`FileTree::step_out`).
    fn tree_out(&mut self) {
        let step = self.filetree.step_out(&self.tree, self.selection());
        self.tree_step(step);
    }

    fn tree_step(&mut self, step: Option<Step>) {
        match step {
            Some(Step::Open(n)) => self.set_open(n, true),
            Some(Step::Close(n)) => self.set_open(n, false),
            Some(Step::Select(e)) => self.select(e, true),
            None => {}
        }
    }

    /// Opens or closes a tree directory. The layout does not change.
    fn set_open(&mut self, node: usize, open: bool) {
        self.filetree.set_open(node, open);
        let rows = self.tree_rows();
        self.filetree.keep_visible(&rows);
    }

    /// Arrows: the nearest visible file in that direction on the map (`mockups/v3-template.html:629-641`),
    /// measured in the layout's tile pitches. With a directory selected, the first file row in the tree.
    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let Some(cur) = self.selected_file() else {
            if let Some(r) = self
                .tree_rows()
                .iter()
                .find(|r| matches!(r.entry, Entry::File(_)))
            {
                self.select(r.entry, true);
            }
            return;
        };
        let (cx, cy) = self.layout.tiles[cur];
        let (pw, ph) = (f64::from(self.layout.cfg.tw), f64::from(self.layout.cfg.th));
        let mut best = None;
        let mut bs = f64::INFINITY;
        for (i, &(x, y)) in self.layout.tiles.iter().enumerate() {
            if i == cur || !self.file_visible(i) {
                continue;
            }
            let ddx = f64::from(x - cx) / pw;
            let ddy = f64::from(y - cy) / ph;
            let along = if dx != 0 {
                ddx * f64::from(dx)
            } else {
                ddy * f64::from(dy)
            };
            if along <= 0.01 {
                continue;
            }
            let across = if dx != 0 { ddy.abs() } else { ddx.abs() };
            let score = if dx != 0 {
                along + across * 10.0
            } else {
                along * 10.0 + across
            };
            if score < bs {
                bs = score;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            self.select(Entry::File(i), true);
        }
    }

    /// `/`: opens the picker with every visible file, in tree order, and nothing highlighted.
    fn open_search(&mut self) {
        let origin = (self.selection(), self.cam_x, self.cam_y);
        let mut s = Search::open(self.rank_files(&Pattern::default()), origin);
        s.set_rows(self.view.tree_rows().1 as usize);
        self.search = Some(s);
        self.relayout();
    }

    /// The visible files ranked for a pattern. An empty pattern keeps tree order.
    fn rank_files(&self, pattern: &Pattern) -> Vec<Ranked> {
        let files = self
            .tree_order
            .iter()
            .filter(|&&f| self.file_visible(f))
            .map(|&f| (f, &self.candidates[f]));
        fuzzy::rank(files, pattern)
    }

    /// Re-ranks after a query change and previews the best result.
    fn update_search(&mut self) {
        let Some(query) = self.search.as_ref().map(|s| s.query.clone()) else {
            return;
        };
        let results = self.rank_files(&Pattern::parse(&query));
        if let Some(s) = &mut self.search {
            s.set_results(results);
        }
        self.preview();
    }

    /// Moves the highlight and previews it.
    fn search_move(&mut self, delta: isize) {
        if let Some(s) = &mut self.search {
            s.move_by(delta);
        }
        self.preview();
    }

    /// The highlighted result becomes the selection and the map shows it.
    /// The tree does not open to it: the picker covers the tree until Enter.
    fn preview(&mut self) {
        if let Some(f) = self.search.as_ref().and_then(Search::current) {
            self.filetree.cursor = Some(Entry::File(f));
            self.show_selection();
        }
    }

    /// Enter: keeps the highlighted result as the selection, and the visible rows for `n` and `N`.
    fn keep_search(&mut self) {
        let Some(s) = &mut self.search else {
            return;
        };
        let current = s.current();
        let origin = s.origin.and_then(|(e, _, _)| e);
        if s.query.trim().is_empty() {
            self.search = None;
        } else {
            s.keep();
        }
        self.relayout();
        // Without a highlighted result, the selection goes back to where it was.
        if let Some(e) = current.map(Entry::File).or(origin) {
            self.select(e, true);
        }
    }

    /// Esc in the picker: restores the selection and the camera from before `/`.
    fn cancel_search(&mut self) {
        let Some(s) = self.search.take() else {
            return;
        };
        self.relayout();
        if let Some((selection, cam_x, cam_y)) = s.origin {
            match selection {
                Some(e) => self.select(e, false),
                None => self.filetree.cursor = None,
            }
            (self.cam_x, self.cam_y) = (cam_x, cam_y);
            self.clamp();
        }
    }

    /// `n` / `N` after Enter: the next or previous kept result, wrapping.
    fn next_kept(&mut self, delta: isize) {
        if let Some(f) = self.search.as_mut().and_then(|s| s.next_kept(delta)) {
            self.select(Entry::File(f), true);
        }
    }

    /// Keys while the picker is open: text edits the query, the arrows and Ctrl+P/N move, Enter keeps, Esc goes back.
    fn on_search_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let Some(s) = &mut self.search else {
            return;
        };
        let page = s.rows as isize;
        match key.code {
            KeyCode::Esc => self.cancel_search(),
            KeyCode::Enter => self.keep_search(),
            KeyCode::Up => self.search_move(1),
            KeyCode::Down => self.search_move(-1),
            KeyCode::Char('p') if ctrl => self.search_move(1),
            KeyCode::Char('n') if ctrl => self.search_move(-1),
            KeyCode::PageUp => self.search_move(page),
            KeyCode::PageDown => self.search_move(-page),
            KeyCode::Char('u') if ctrl => {
                s.query.clear();
                self.update_search();
            }
            KeyCode::Char('w') if ctrl => {
                let kept = s
                    .query
                    .trim_end()
                    .trim_end_matches(|c: char| c != ' ')
                    .len();
                s.query.truncate(kept);
                self.update_search();
            }
            KeyCode::Backspace => {
                s.query.pop();
                self.update_search();
            }
            KeyCode::Char(c) if !ctrl && !alt => {
                s.query.push(c);
                self.update_search();
            }
            _ => {}
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        let searching = self.search.as_ref().is_some_and(|s| s.active);
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
        } else if searching {
            self.on_search_key(key);
        } else if self.viewer.is_some() && !self.help {
            self.on_viewer_key(key);
        } else if self.help {
            self.on_help_key(key);
        } else if self.outline_browse().is_some() {
            if let Some(cmd) = self.resolve(Mode::Outline, key) {
                self.outline_command(cmd);
            }
        } else if let Some(cmd) = self.resolve(Mode::Map, key) {
            self.map_command(cmd);
        }
    }

    /// Adds a key to the typed sequence of a mode and looks the sequence up (unit `033`).
    /// Returns the action once a binding completes. A key that completes nothing ends the sequence.
    fn resolve(&mut self, mode: Mode, key: KeyEvent) -> Option<Command> {
        if self.pending.0 != mode {
            self.pending = (mode, Vec::new());
        }
        self.pending.1.push(Key::from_event(&key));
        match self.keymap.table(mode).lookup(&self.pending.1) {
            Lookup::Run(cmd) => {
                self.pending.1.clear();
                Some(cmd)
            }
            Lookup::Wait => None,
            Lookup::None => {
                self.pending.1.clear();
                None
            }
        }
    }

    /// Keys while the help shows: Esc and the help keys close it, the quit keys quit, and other keys do nothing.
    fn on_help_key(&mut self, key: KeyEvent) {
        let k = Key::from_event(&key);
        let viewer_help =
            self.viewer.is_some() && self.keymap.viewer.lookup(&[k]) == Lookup::Run(Command::Help);
        match self.resolve(Mode::Map, key) {
            Some(Command::Quit) => self.quit = true,
            Some(Command::Help) => self.help = false,
            _ if k == Key::ESC || viewer_help => {
                self.pending.1.clear();
                self.help = false;
            }
            _ => {}
        }
    }

    /// An action in the map and the file tree.
    fn map_command(&mut self, cmd: Command) {
        let page = self.filetree.height.max(1) as isize;
        let pan = |dx: i32, dy: i32, k: i32| (dx * k, dy * k);
        let step = match cmd {
            Command::PanUp => Some(pan(0, -PAN_Y, 1)),
            Command::PanDown => Some(pan(0, PAN_Y, 1)),
            Command::PanLeft => Some(pan(-PAN_X, 0, 1)),
            Command::PanRight => Some(pan(PAN_X, 0, 1)),
            Command::PanUpFar => Some(pan(0, -PAN_Y, 4)),
            Command::PanDownFar => Some(pan(0, PAN_Y, 4)),
            Command::PanLeftFar => Some(pan(-PAN_X, 0, 4)),
            Command::PanRightFar => Some(pan(PAN_X, 0, 4)),
            _ => None,
        };
        if let Some((dx, dy)) = step {
            return self.pan(dx, dy);
        }
        match cmd {
            Command::Quit => self.quit = true,
            Command::Help => self.help = true,
            Command::Search => self.open_search(),
            Command::NextResult => self.next_kept(1),
            Command::PreviousResult => self.next_kept(-1),
            Command::ClearSearch => self.search = None,
            Command::ToggleHidden => self.toggle_hidden(),
            Command::ToggleTree => self.set_tree(!self.tree_on, self.tree_w),
            Command::NarrowTree => self.set_tree(self.tree_on, self.tree_w - TREE_STEP),
            Command::WidenTree => self.set_tree(self.tree_on, self.tree_w + TREE_STEP),
            Command::CopyPath => self.copy_selection(),
            Command::CopyNamespace => self.copy_namespace(),
            Command::CopyCommit => self.copy_commit(),
            Command::ToggleTrail => self.trail_on = !self.trail_on,
            Command::ClearTrail => {
                self.trail.clear();
                self.flash("Cleared the agent's trail.".to_string());
            }
            Command::ToggleAgeHeat => self.toggle_heat(Heat::Age),
            Command::ToggleChurnHeat => self.toggle_heat(Heat::Churn),
            Command::BrowseOutline => self.open_outline(),
            Command::OpenEditor => match self.selection() {
                Some(Entry::File(f)) => self.open_editor(f, 1),
                _ => self.flash("Select a file to open it.".to_string()),
            },
            Command::TogglePulse => self.pulse = !self.pulse,
            Command::TogglePanel => self.toggle_panel(),
            Command::ToggleOverview => self.toggle_overview(),
            Command::FirstRow => self.tree_end(false),
            Command::LastRow => self.tree_end(true),
            Command::Down => self.tree_move(1),
            Command::Up => self.tree_move(-1),
            Command::PageDown => self.tree_move(page),
            Command::PageUp => self.tree_move(-page),
            Command::TreeIn => self.tree_in(),
            Command::TreeOut => self.tree_out(),
            Command::Open => match self.selection() {
                Some(Entry::Dir(n)) => self.set_open(n, !self.filetree.is_open(n)),
                Some(Entry::File(f)) => self.open_viewer(f, None, false),
                None => {}
            },
            Command::SelectLeft => self.move_cursor(-1, 0),
            Command::SelectRight => self.move_cursor(1, 0),
            Command::SelectUp => self.move_cursor(0, -1),
            Command::SelectDown => self.move_cursor(0, 1),
            _ => {}
        }
    }

    pub fn picker_open(&self) -> bool {
        self.search.as_ref().is_some_and(|s| s.active)
    }

    /// The picker list row under a screen cell, counted from the bottom (the best result's row is 0).
    pub fn picker_row_at(&self, col: i32, row: i32) -> Option<usize> {
        let (y, h) = self.view.tree_rows();
        (self.picker_open() && self.over_tree(col, row)).then(|| (y + h - 1 - row) as usize)
    }

    /// The tree row index under a screen cell. None while the picker covers the tree.
    pub fn tree_row_at(&self, col: i32, row: i32) -> Option<usize> {
        let v = self.view;
        let (y, h) = v.tree_rows();
        if self.picker_open() || v.tw == 0 || col < 0 || col >= v.tw || row < y || row >= y + h {
            return None;
        }
        let k = self.filetree.top + (row - y) as usize;
        (k < self.tree_rows().len()).then_some(k)
    }

    /// The visible file tile or directory header text under a screen cell of the map region.
    /// The root header does not count: the root has no tree row. The overview covers what is under it.
    pub fn hit(&self, col: i32, row: i32) -> Option<Entry> {
        let v = self.view;
        if self.viewer.is_some() || !v.in_map(col, row) || self.in_overview(col, row) {
            return None;
        }
        let (lx, ly) = (col - v.mx + self.cam_x, row - v.my + self.cam_y);
        let (tw, th) = self.layout.cfg.tile_size();
        let file = self.layout.tiles.iter().enumerate().find(|&(i, &(x, y))| {
            lx >= x && lx < x + tw && ly >= y && ly < y + th && self.file_visible(i)
        });
        if let Some((i, _)) = file {
            return Some(Entry::File(i));
        }
        self.layout
            .headers
            .iter()
            .find(|h| {
                h.node != 0
                    && ly == h.y
                    && lx >= h.x
                    && lx < h.x + crate::ui::header_text_width(self, h)
                    && (self.show_hidden || !self.tree.nodes[h.node].hidden)
            })
            .map(|h| Entry::Dir(h.node))
    }

    /// The overview is under a screen cell. While the viewer covers the map, there is no overview to click.
    fn in_overview(&self, col: i32, row: i32) -> bool {
        self.viewer.is_none() && self.overview().is_some_and(|o| o.contains(col, row))
    }

    /// True while the tree edge is being dragged.
    pub fn dragging_tree_edge(&self) -> bool {
        matches!(self.drag, Some(Drag::TreeEdge))
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        let v = self.view;
        let (col, row) = (i32::from(m.column), i32::from(m.row));
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if v.tw > 0 && col == v.tw && row >= 1 && row < v.rows - 1 {
                    self.drag = Some(Drag::TreeEdge);
                } else if v.in_map(col, row) && self.in_overview(col, row) {
                    self.drag = Some(Drag::Overview);
                    self.overview_jump(col, row);
                } else if let Some(from_bottom) = self.picker_row_at(col, row) {
                    if let Some(s) = &mut self.search {
                        s.pick_row(from_bottom);
                    }
                    self.preview();
                } else if let Some(k) = self.tree_row_at(col, row) {
                    let e = self.tree_rows()[k].entry;
                    self.select(e, true);
                    match e {
                        Entry::Dir(n) => {
                            self.set_open(n, !self.filetree.is_open(n));
                            self.close_viewer();
                        }
                        Entry::File(f) if self.viewer.is_some() => self.open_viewer(f, None, false),
                        Entry::File(_) => {}
                    }
                } else if let Some(vw) = self.viewer.as_mut().filter(|_| v.in_map(col, row)) {
                    // A click on a line moves the viewer's cursor there.
                    let line = vw.top + (row - v.my) as usize;
                    if line < vw.len() {
                        vw.go(line);
                    }
                } else if v.in_map(col, row) {
                    self.drag = Some(Drag::Map {
                        col,
                        row,
                        cam_x: self.cam_x,
                        cam_y: self.cam_y,
                        moved: false,
                    });
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => match &mut self.drag {
                Some(Drag::TreeEdge) => self.set_tree(true, col),
                Some(Drag::Overview) => self.overview_jump(col, row),
                Some(Drag::Map {
                    col: c0,
                    row: r0,
                    cam_x,
                    cam_y,
                    moved,
                }) => {
                    *moved |= col != *c0 || row != *r0;
                    if *moved {
                        self.cam_x = *cam_x - (col - *c0);
                        self.cam_y = *cam_y - (row - *r0);
                        self.clamp();
                    }
                }
                None => {}
            },
            MouseEventKind::Up(_) => {
                // A press and release without a drag is a click: it selects without scrolling the map.
                if let Some(Drag::Map {
                    col,
                    row,
                    moved: false,
                    ..
                }) = self.drag.take()
                    && let Some(e) = self.hit(col, row)
                {
                    self.select(e, false);
                    // A second click on the same tile soon after is a double-click: it views the file.
                    let double = self.last_click.is_some_and(|(t, c, r)| {
                        (c, r) == (col, row) && self.now.saturating_sub(t) <= DOUBLE_CLICK
                    });
                    self.last_click = Some((self.now, col, row));
                    if let (true, Entry::File(f)) = (double, e) {
                        self.last_click = None;
                        self.open_viewer(f, None, false);
                    }
                }
            }
            MouseEventKind::Moved => self.hover = self.hit(col, row),
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                if self.viewer.is_some() && v.in_map(col, row) =>
            {
                if let Some(vw) = &mut self.viewer {
                    let down = matches!(m.kind, MouseEventKind::ScrollDown);
                    vw.scroll(viewer::WHEEL_LINES, down);
                }
            }
            MouseEventKind::ScrollUp if self.picker_open() && self.over_tree(col, row) => {
                self.search_move(filetree::WHEEL_ROWS as isize);
            }
            MouseEventKind::ScrollDown if self.picker_open() && self.over_tree(col, row) => {
                self.search_move(-(filetree::WHEEL_ROWS as isize));
            }
            MouseEventKind::ScrollDown if self.over_tree(col, row) => {
                let len = self.tree_rows().len();
                self.filetree.scroll(len, filetree::WHEEL_ROWS as isize);
            }
            MouseEventKind::ScrollUp if self.over_tree(col, row) => {
                let len = self.tree_rows().len();
                self.filetree.scroll(len, -(filetree::WHEEL_ROWS as isize));
            }
            // The wheel does not pan the map (M1 review).
            _ => {}
        }
    }

    fn over_tree(&self, col: i32, row: i32) -> bool {
        let v = self.view;
        let (y, h) = v.tree_rows();
        v.tw > 0 && col >= 0 && col < v.tw && row >= y && row < y + h
    }
}

/// The files under a node in file tree order: each subdirectory's files first, then the node's own files.
fn tree_order(tree: &Tree, node: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(tree.files.len());
    let mut stack = vec![(node, false)];
    while let Some((n, kids_done)) = stack.pop() {
        if kids_done {
            out.extend(&tree.nodes[n].files);
        } else {
            stack.push((n, true));
            stack.extend(tree.nodes[n].kids.iter().rev().map(|&k| (k, false)));
        }
    }
    out
}
