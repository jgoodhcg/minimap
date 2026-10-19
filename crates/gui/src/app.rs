//! The window's state and input: the map, the camera, the toggles, the selection and hover, the details loader,
//! the file tree (unit `042`), the selection panel and the git history (unit `044`), the agent's trail (unit `047`),
//! the live index (unit `048`), the glows of agent events (unit `049`), the agent history (unit `051`), the agent's notes
//! and the history filter (unit `053`), the note reader (unit `054`), and the bars.
//!
//! Input goes through [`GuiApp::on_event`] and [`GuiApp::on_drag`], so tests drive the app with no window.
//! [`GuiApp::frame`] is one whole frame, so a headless `egui::Context::run_ui` drives it too.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, CursorIcon, Event, Id, Key, MouseWheelUnit, PointerButton, Pos2, Rect, Sense,
    Vec2, pos2, vec2,
};
use minimap_core::content::Content;
use minimap_core::details::{FileDetails, Loader};
use minimap_core::events::{Event as AgentEvent, Listener};
use minimap_core::filetree::{FileTree, Row, Step};
use minimap_core::layout::Breadcrumb;
use minimap_core::notes::Notes;
use minimap_core::palette;
use minimap_core::trail::{self, Trail};
use minimap_core::watch::{Update, Watcher};

use crate::activity::{
    self, ACTIVITY_H, ACTIVITY_MAX, Act, ActivityView, Filter, Order, REGION_MIN_H, Record,
    StripHit,
};
use crate::camera::{
    Bounds, Camera, KEY_EASE, KEY_ZOOM, Motion, PAN_FAR, PAN_STEP, SCROLL_ZOOM_POINTS, WHEEL_ZOOM,
};
use crate::capture::Capture;
use crate::color::rgb;
use crate::fonts::font;
use crate::paint;
use crate::panel::{
    self, Facts, GitHistory, Hit, NoteRow, PANEL_MAX, PANEL_MIN, PANEL_W, PanelView,
};
use crate::reader::{self, CODE_ROW, Colorer, ReaderHit, ReaderView, Selection, Spot, Text};
use crate::scene::{self, Entry, Glow, Item, Level, Map, Options};
use crate::text::thousands;
use crate::tree::{
    self, MAP_MIN, ROW_H, TREE_MAX, TREE_MIN, TREE_STEP, TREE_W, TreeView, WHEEL_ROWS,
};

/// The top bar and the status bar are this tall, in points.
pub const BAR_H: f32 = 24.0;
/// The bars' text size.
pub const BAR_TEXT: f32 = 13.0;
/// A press that moves more than this, in points, before its release is a pan, not a click.
pub const CLICK_SLOP: f32 = 4.0;
/// Two clicks on one entry within this time, in seconds, are a double-click.
pub const DOUBLE_CLICK: f64 = 0.4;
/// Zoom to a card: it is this wide, in points, and centered horizontally. Zooms to an entry ease over [`ZOOM_TO_EASE`].
/// A card that fits the region's height is centered in it; a taller card has its top [`ZOOM_TO_TOP`] points below the region's top.
pub const ZOOM_TO_WIDTH: f32 = 420.0;
pub const ZOOM_TO_TOP: f32 = 24.0;
pub const ZOOM_TO_EASE: f64 = 0.240;
/// A tree selection moves the map only as far as needed to show the entry with this margin, in points (unit `042`).
pub const SHOW_MARGIN: f32 = 24.0;
/// A flash in the top bar shows this long, in seconds (unit `048`).
pub const FLASH_SECS: f64 = 4.0;
/// The flashes of `n` and `N` (unit `054`).
pub const NO_NOTES: &str = "No notes yet. An agent sends them with minimap note.";
pub const LAST_NOTE: &str = "Last note.";
pub const FIRST_NOTE: &str = "First note.";
/// The status bar's key hint on the map, and while the reader shows.
pub const MAP_HINT: &str = "click select · double-click f ⏎ zoom to · esc clear · + - zoom · wasd pan · 0 fit · j k h l tree · t tree · p panel · r history · n N notes";
pub const READER_HINT: &str =
    "n N notes · z lines / whole file · j k g G scroll · f map · esc close · drag select · ⌘c copy";

/// What a primary-button press began on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PressOn {
    Map,
    /// A tree row, or the tree's empty space.
    Tree(Option<Entry>),
    /// The tree's right edge, with the tree's width when the press began.
    Edge(f32),
    /// The selection panel (unit `044`): a clickable row (unit `046`), or the rest of the panel.
    Panel(Option<Hit>),
    /// The panel's left edge, with the panel's width when the press began.
    PanelEdge(f32),
    /// The agent history (unit `051`): a row with a card, by its number, a filter chip (unit `053`), or the rest of the strip.
    Activity(Option<StripHit>),
    /// The note reader (unit `054`): a mode chip, or the rest of the reader.
    Reader(Option<ReaderHit>),
    /// The reader's body, at the text spot under the press (unit `057`): a drag selects from it, and a click clears the selection.
    ReaderBody(Spot),
}

/// The note that the reader shows (unit `054`), its file's text, and its scroll.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub note: u64,
    pub path: String,
    pub text: Text,
    /// The first row shown, and the scroll that has not made a whole row yet.
    pub top: usize,
    scroll: f32,
    /// The text's syntax colors, once the colorer has them (unit `055`).
    pub colors: Option<Arc<Vec<Vec<minimap_core::highlight::Span>>>>,
}

/// A primary-button press.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    at: Pos2,
    last: Pos2,
    panning: bool,
    on: PressOn,
}

/// The window's regions: the bars, the tree column and the panel column while they show, and the map region.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub top: Rect,
    pub tree: Option<Rect>,
    pub panel: Option<Rect>,
    /// The agent history strip below the map region, while it shows (unit `051`).
    pub activity: Option<Rect>,
    pub region: Rect,
    pub status: Rect,
}

pub struct GuiApp {
    pub map: Map,
    pub bounds: Bounds,
    pub motion: Motion,
    pub opts: Options,
    /// The map region of the last frame.
    pub region: Rect,
    /// The first frame fits the map.
    started: bool,
    char_w_measured: bool,
    pub capture: Option<Capture>,
    /// Computes shapes (and outlines, for a later selection panel) in the background. Without it, shapes show as loading.
    pub loader: Option<Loader>,
    /// The cards whose details were last requested, nearest to the center first.
    requested: Vec<usize>,
    /// The status bar's toggle entries, for clicks.
    toggle_rects: Vec<(Rect, Toggle)>,
    pub selection: Option<Entry>,
    pub hover: Option<Entry>,
    press: Option<Press>,
    /// The last click's time and entry, for double-clicks.
    last_click: Option<(f64, Entry)>,
    /// `--select`: the first frame zooms to the selection, not the whole map.
    zoom_to_selection: bool,
    /// The file tree's model (unit `042`): open directories, the cursor, and the scroll.
    pub filetree: FileTree,
    /// The tree shows, and its width in points, before a narrow window narrows it.
    pub tree_on: bool,
    pub tree_w: f32,
    /// The tree column of the last layout, while the tree shows.
    pub tree_view: Option<TreeView>,
    /// The window's rect at the last layout, so a tree change lays out at once.
    screen: Option<Rect>,
    /// Scroll over the tree, in rows, that has not made a whole row yet.
    tree_scroll: f32,
    /// The picked outline item of the selected file (unit `046`), by its index in the outline.
    pub pick: Option<usize>,
    /// The outline's first item, and the scroll over it that has not made a whole row yet.
    pub outline_top: usize,
    outline_scroll: f32,
    /// The panel row under the pointer.
    pub panel_hover: Option<Hit>,
    /// The last click on a largest-file row: its time, its point, and the file. The click selects the file, so the panel
    /// changes under the pointer, and a second click there completes a double-click.
    file_click: Option<(f64, Pos2, usize)>,
    /// The agent's trail (unit `047`), and the listener on the repo's event socket that feeds it.
    pub trail: Trail,
    pub listener: Option<Listener>,
    /// A message at the top bar's right while nothing is hovered, such as another minimap that has the trail.
    pub notice: Option<String>,
    /// The time of the last frame or event, in egui seconds.
    pub now: f64,
    /// The live index (unit `048`): the watcher on the working tree, and the repo root for a new history walk.
    pub watcher: Option<Watcher>,
    pub root: Option<std::path::PathBuf>,
    /// A message at the top bar's right until a time, such as `2 added · 0 removed`.
    pub flash: Option<(String, f64)>,
    /// A history walk after `HEAD` moved; the shown history stays until it ends.
    history_reload: Option<minimap_core::history::Loader>,
    /// The selection panel shows (unit `044`), and its width in points, before a narrow window narrows it.
    pub panel_on: bool,
    pub panel_w: f32,
    /// The panel column of the last layout, while the panel shows.
    pub panel_view: Option<PanelView>,
    /// The git history for the panel.
    pub git: GitHistory,
    /// The lines of every indexed file, for the panel's first row.
    pub total_lines: u64,
    /// The running glows of agent events (unit `049`), by path: a new event on a path restarts its glow.
    pub glows: Vec<Glow>,
    /// The agent history (unit `051`): one row per path of each agent event this window received, oldest first,
    /// the next row's number, and the distinct sessions.
    pub activity: Vec<Record>,
    activity_seq: u64,
    pub sessions: Vec<String>,
    /// The strip shows, the newest rows scrolled past, and the scroll that has not made a whole row yet.
    pub activity_on: bool,
    pub activity_top: usize,
    activity_scroll: f32,
    /// The strip of the last layout, while it shows.
    pub activity_view: Option<ActivityView>,
    /// The strip's row or chip under the pointer.
    pub activity_hover: Option<StripHit>,
    /// The strip's filter (unit `053`), and its order (unit `057`).
    pub activity_filter: Filter,
    pub activity_order: Order,
    /// The agent's notes (unit `053`), in arrival order.
    pub notes: Notes,
    /// The note reader (unit `054`), while it shows; its mode, `whole file` or `lines`; the last note read, for `n` and `N`;
    /// the chip under the pointer; and the source of file text.
    pub reader: Option<Reading>,
    pub reader_whole: bool,
    pub note_cursor: Option<u64>,
    pub reader_hover: Option<ReaderHit>,
    /// The reader's text selection (unit `057`), and the text that the next frame copies to the clipboard.
    pub reader_sel: Option<Selection>,
    pub copied: Option<String>,
    pub content: Content,
    /// The reader's highlight thread (unit `055`). Without it, the reader's text is plain.
    pub colorer: Option<Colorer>,
}

/// A toggle of the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Trail,
    Notes,
    Names,
    Shape,
}

impl GuiApp {
    pub fn new(map: Map) -> GuiApp {
        let bounds = Bounds { map: map.world() };
        let mut filetree = FileTree::new(&map.tree);
        // The tree's cursor is the selection, and the GUI starts with none.
        filetree.cursor = None;
        let total_lines = map.index.files.iter().map(|f| f.lines).sum();
        GuiApp {
            map,
            bounds,
            motion: Motion::new(Camera {
                center: Pos2::ZERO,
                zoom: 1.0,
            }),
            opts: Options::default(),
            region: Rect::NOTHING,
            started: false,
            char_w_measured: false,
            capture: None,
            loader: None,
            requested: Vec::new(),
            toggle_rects: Vec::new(),
            selection: None,
            hover: None,
            press: None,
            last_click: None,
            zoom_to_selection: false,
            filetree,
            tree_on: true,
            tree_w: TREE_W,
            tree_view: None,
            screen: None,
            tree_scroll: 0.0,
            panel_on: true,
            panel_w: PANEL_W,
            panel_view: None,
            git: GitHistory::None,
            total_lines,
            pick: None,
            outline_top: 0,
            outline_scroll: 0.0,
            panel_hover: None,
            file_click: None,
            trail: Trail::default(),
            listener: None,
            notice: None,
            now: 0.0,
            watcher: None,
            root: None,
            flash: None,
            history_reload: None,
            glows: Vec::new(),
            activity: Vec::new(),
            activity_seq: 0,
            sessions: Vec::new(),
            activity_on: true,
            activity_top: 0,
            activity_scroll: 0.0,
            activity_view: None,
            activity_hover: None,
            activity_filter: Filter::All,
            activity_order: Order::NewestFirst,
            notes: Notes::default(),
            reader: None,
            reader_whole: false,
            note_cursor: None,
            reader_hover: None,
            reader_sel: None,
            copied: None,
            content: Content::None,
            colorer: None,
        }
    }

    /// Selects an entry at startup (`--select`). The first frame shows it.
    pub fn select_at_start(&mut self, e: Entry) {
        self.select(Some(e), false, 0.0);
        self.zoom_to_selection = true;
    }

    /// The one selection of the tree and the map (unit `042`): the tree opens down to the entry and scrolls to its row.
    /// With `follow`, the map moves only as far as needed to show it ([`GuiApp::show`]).
    pub fn select(&mut self, e: Option<Entry>, follow: bool, now: f64) {
        // Another selection clears the pick, and its outline starts at the top (unit `046`).
        if e != self.selection {
            self.pick = None;
            self.outline_top = 0;
            self.outline_scroll = 0.0;
        }
        // Another selection closes the note reader (unit `054`).
        if let Some(r) = &self.reader
            && e != self.map.entry(&r.path)
        {
            self.close_reader();
        }
        self.selection = e;
        self.filetree.cursor = e;
        let Some(e) = e else {
            return;
        };
        self.filetree.reveal(&self.map.tree, e);
        let rows = self.tree_rows();
        self.filetree.keep_visible(&rows);
        if follow {
            self.show(e, now);
        }
    }

    /// The tree's visible rows. The GUI has no hidden-files toggle, so hidden files show.
    pub fn tree_rows(&self) -> Vec<Row> {
        self.filetree.rows(&self.map.tree, true)
    }

    /// Opens or closes a tree directory. The layout does not change.
    pub fn set_open(&mut self, node: usize, open: bool) {
        self.filetree.set_open(node, open);
        let rows = self.tree_rows();
        self.filetree.keep_visible(&rows);
    }

    /// The camera at `cam`'s zoom that shows a world rect with [`SHOW_MARGIN`], moved as little as possible.
    /// A rect larger than the region less its margins shows its top and left edges.
    pub fn camera_showing(&self, cam: Camera, r: Rect) -> Camera {
        let s = cam.rect_to_screen(self.region, r);
        let avail = self
            .region
            .shrink(SHOW_MARGIN.min(self.region.width().min(self.region.height()) / 4.0));
        let need = |lo: f32, hi: f32, a: f32, b: f32| {
            if hi - lo > b - a || lo < a {
                lo - a
            } else if hi > b {
                hi - b
            } else {
                0.0
            }
        };
        let d = vec2(
            need(s.left(), s.right(), avail.left(), avail.right()),
            need(s.top(), s.bottom(), avail.top(), avail.bottom()),
        );
        if d == Vec2::ZERO {
            return cam;
        }
        self.bounds.clamp(
            self.region,
            Camera {
                center: cam.center + d / cam.zoom,
                zoom: cam.zoom,
            },
        )
    }

    /// Moves the map, eased, only as far as needed to show an entry: its card, or its block's top. The zoom does not change.
    pub fn show(&mut self, e: Entry, now: f64) {
        let from = self.motion.target();
        let to = self.camera_showing(from, self.map.rect_of(e));
        if to != from {
            self.motion.ease_to(to, now, KEY_EASE);
        }
    }

    /// The window's regions for a screen rect: the tree takes its width at the left and the panel its width at the right.
    /// In a narrow window the panel narrows first, down to [`PANEL_MIN`], then the tree, down to [`TREE_MIN`], so the map keeps [`MAP_MIN`].
    pub fn screen_layout(&self, screen: Rect) -> Screen {
        let top = Rect::from_min_size(screen.min, vec2(screen.width(), BAR_H));
        let status = Rect::from_min_max(pos2(screen.left(), screen.bottom() - BAR_H), screen.max);
        let mid = Rect::from_min_max(
            pos2(screen.left(), top.bottom()),
            pos2(screen.right(), status.top()),
        );
        let room = screen.width() - MAP_MIN;
        let tree_set = if self.tree_on { self.tree_w } else { 0.0 };
        let panel_w = self
            .panel_on
            .then(|| self.panel_w.min(room - tree_set).max(PANEL_MIN));
        let tree = self.tree_on.then(|| {
            let w = self.tree_w.min(room - panel_w.unwrap_or(0.0)).max(TREE_MIN);
            Rect::from_min_size(mid.min, vec2(w, mid.height()))
        });
        let panel = panel_w.map(|w| Rect::from_min_max(pos2(mid.right() - w, mid.top()), mid.max));
        let (left, right) = (
            tree.map_or(mid.left(), |t| t.right()),
            panel.map_or(mid.right(), |p| p.left()),
        );
        // The agent history takes the bottom of the middle column; the map region keeps `REGION_MIN_H`, and a strip under two rows hides.
        let strip_h = if self.activity_on {
            ACTIVITY_H.min(mid.height() - REGION_MIN_H)
        } else {
            0.0
        };
        let activity = (strip_h >= 2.0 * ROW_H).then(|| {
            Rect::from_min_max(
                pos2(left, mid.bottom() - strip_h),
                pos2(right, mid.bottom()),
            )
        });
        let region = Rect::from_min_max(
            pos2(left, mid.top()),
            pos2(right, activity.map_or(mid.bottom(), |a| a.top())),
        );
        Screen {
            top,
            tree,
            panel,
            activity,
            region,
            status,
        }
    }

    /// Lays out the window: the tree column and the map region. The map content keeps its place on the screen.
    pub fn set_screen(&mut self, screen: Rect) {
        self.screen = Some(screen);
        let s = self.screen_layout(screen);
        self.tree_view = s.tree.map(TreeView::new);
        self.panel_view = s.panel.map(PanelView::new);
        self.activity_view = s.activity.map(ActivityView::new);
        if let Some(v) = &self.activity_view {
            self.activity_top = self
                .activity_top
                .min(activity::max_top(self.activity_rows(), v));
        }
        let height = self.tree_view.map_or(0, |v| v.visible_rows());
        if height != self.filetree.height {
            let shown = self.filetree.height == 0;
            self.filetree.height = height;
            let rows = self.tree_rows();
            self.filetree.clamp_top(rows.len());
            // A tree that shows again scrolls to the selection, which may have moved while it was hidden.
            if shown {
                self.filetree.keep_visible(&rows);
            }
        }
        self.set_region(s.region);
    }

    /// Shows or hides the tree, and sets its width within its limits. The map content keeps its place on the screen.
    pub fn set_tree(&mut self, on: bool, w: f32) {
        self.tree_on = on;
        self.tree_w = w.clamp(TREE_MIN, TREE_MAX);
        if let Some(s) = self.screen {
            self.set_screen(s);
        }
    }

    /// The tree's width on the screen, or its set width while it is hidden.
    pub fn tree_width(&self) -> f32 {
        self.tree_view.map_or(self.tree_w, |v| v.rect.width())
    }

    /// Shows or hides the selection panel, and sets its width within its limits (unit `044`). The map content keeps its place on the screen.
    pub fn set_panel(&mut self, on: bool, w: f32) {
        self.panel_on = on;
        self.panel_w = w.clamp(PANEL_MIN, PANEL_MAX);
        if let Some(s) = self.screen {
            self.set_screen(s);
        }
    }

    /// Shows or hides the agent history (unit `051`). The map content keeps its place on the screen.
    pub fn set_activity(&mut self, on: bool) {
        self.activity_on = on;
        if let Some(s) = self.screen {
            self.set_screen(s);
        }
    }

    /// The agent history's draw items, while it shows.
    pub fn strip(&self) -> Option<activity::Strip> {
        let view = self.activity_view?;
        let facts = activity::Facts {
            map: &self.map,
            records: &self.activity,
            top: self.activity_top,
            sessions: &self.sessions,
            selection: self.selection,
            hovered: self.activity_hover,
            filter: self.activity_filter,
            order: self.activity_order,
            char_w: self.opts.char_w,
            notice: self.notice.as_deref().filter(|_| self.listener.is_none()),
        };
        Some(activity::build(&view, &facts))
    }

    /// The agent history's rows that its filter shows (unit `053`).
    pub fn activity_rows(&self) -> usize {
        self.activity
            .iter()
            .filter(|r| self.activity_filter.matches(r.kind))
            .count()
    }

    /// A click on a filter chip (unit `053`): the strip shows that kind's rows, from its first row.
    pub fn set_filter(&mut self, f: Filter) {
        self.activity_filter = f;
        self.activity_top = 0;
        self.activity_scroll = 0.0;
    }

    /// A click on the sort chip (unit `057`): the strip's rows in the other order. A scrolled strip keeps its top row
    /// when that row can be the top row in the new order; else the strip shows the new order's first rows.
    pub fn set_order(&mut self, order: Order) {
        if order == self.activity_order {
            return;
        }
        let top_row = (self.activity_top > 0)
            .then(|| {
                activity::shown(&self.activity, self.activity_filter, self.activity_order)
                    .get(self.activity_top)
                    .map(|r| r.seq)
            })
            .flatten();
        self.activity_order = order;
        self.activity_scroll = 0.0;
        let rows = activity::shown(&self.activity, self.activity_filter, order);
        let max = self
            .activity_view
            .map_or(0, |v| activity::max_top(rows.len(), &v));
        self.activity_top = top_row
            .and_then(|seq| rows.iter().position(|r| r.seq == seq))
            .filter(|&k| k <= max)
            .unwrap_or(0);
    }

    /// A click on an agent history row (unit `051`), as a click on its file's tree row: the file becomes the one selection,
    /// so the tree opens to it and the panel shows it, and the map moves only as far as needed to show it, with no zoom change
    /// (the user's try-out, 2026-10-07). A double-click zooms to it. A row whose path has no card does nothing.
    /// A note row (unit `057`) selects its file first, so the user sees it on the map; a click on it while its file is the selection
    /// opens the note in the reader, so a double-click opens it too. A note row without a card opens the note at once (unit `054`).
    pub fn click_activity(&mut self, seq: u64, now: f64) {
        let Some(r) = self.activity.iter().find(|r| r.seq == seq) else {
            return;
        };
        let file = activity::file_of(&self.map, &r.path).map(Entry::File);
        // A note row works while its note is there; after a notes clear it clicks as a trail row.
        if let Some(id) = r.note.filter(|&id| self.notes.get(id).is_some())
            && (file.is_none() || file == self.selection)
        {
            self.open_note(id, now);
            self.last_click = None;
            return;
        }
        let Some(e) = file else {
            return;
        };
        self.click_row(e, now);
    }

    /// Scrolls the agent history by `rows` toward older rows, within its limits, keeping the part of a row that is left.
    fn scroll_activity(&mut self, rows: f32) {
        let Some(v) = self.activity_view else {
            return;
        };
        self.activity_scroll += rows;
        let whole = self.activity_scroll.trunc();
        self.activity_scroll -= whole;
        let max = activity::max_top(self.activity_rows(), &v);
        self.activity_top =
            (self.activity_top as isize + whole as isize).clamp(0, max as isize) as usize;
    }

    /// The panel's width on the screen, or its set width while it is hidden.
    pub fn panel_width(&self) -> f32 {
        self.panel_view.map_or(self.panel_w, |v| v.rect.width())
    }

    /// A file's details, once the loader has them.
    pub fn file_details(&self, f: usize) -> Option<&FileDetails> {
        self.loader.as_ref()?.get(&self.map.tree.files[f].path)
    }

    /// Takes the git history when the walk has finished. [`GuiApp::frame`] calls it before each frame.
    pub fn poll_history(&mut self) {
        if let Some((result, _took)) = self.history_reload.as_ref().and_then(|l| l.poll()) {
            self.history_reload = None;
            self.git = match result {
                Ok(h) => GitHistory::Ready(h),
                Err(e) => GitHistory::Failed(format!("{e:#}")),
            };
        }
        let GitHistory::Loading(loader) = &self.git else {
            return;
        };
        if let Some((result, _took)) = loader.poll() {
            self.git = match result {
                Ok(h) => GitHistory::Ready(h),
                Err(e) => GitHistory::Failed(format!("{e:#}")),
            };
        }
    }

    /// The panel still waits for something: the git history, or the selected file's details. The capture waits for it.
    pub fn panel_loading(&self) -> bool {
        let details = match self.selection {
            Some(Entry::File(f)) => self.loader.is_some() && self.file_details(f).is_none(),
            _ => false,
        };
        self.panel_view.is_some() && (details || matches!(self.git, GitHistory::Loading(_)))
    }

    /// The panel's draw items for the selection.
    pub fn panel(&self) -> Option<panel::Panel> {
        let view = self.panel_view?;
        let details = match self.selection {
            Some(Entry::File(f)) => self.file_details(f),
            _ => None,
        };
        let facts = Facts {
            map: &self.map,
            filetree: &self.filetree,
            selection: self.selection,
            details,
            git: &self.git,
            total_lines: self.total_lines,
            icons: self.opts.icons,
            char_w: self.opts.char_w,
            picked: self.pick,
            outline_top: self.outline_top,
            hovered: self.panel_hover,
            trail: self.trail_line(),
            notes: self.note_rows(),
            reading: self.reader.as_ref().map(|r| r.note),
        };
        Some(panel::build(&view, &facts))
    }

    /// The selected file's notes, for the panel's NOTES section (unit `054`).
    pub fn note_rows(&self) -> Vec<NoteRow> {
        let Some(Entry::File(f)) = self.selection else {
            return Vec::new();
        };
        self.notes
            .of_path(&self.map.tree.files[f].path)
            .map(|n| NoteRow {
                id: n.id,
                ordinal: self.notes.ordinal(n.id).unwrap_or(0),
                lines: n.lines,
                text: activity::first_line(&n.text),
            })
            .collect()
    }

    /// The reader's facts, while a note shows (unit `054`).
    fn reader_facts(&self) -> Option<reader::Facts<'_>> {
        let r = self.reader.as_ref()?;
        Some(reader::Facts {
            notes: &self.notes,
            note: r.note,
            whole: self.reader_whole,
            text: &r.text,
            top: r.top,
            char_w: self.opts.char_w,
            hovered: self.reader_hover,
            colors: r.colors.as_ref().map(|c| c.as_slice()),
            selection: self.reader_sel,
        })
    }

    /// The reader's page, while a note shows (unit `054`).
    pub fn reader_page(&self) -> Option<reader::Page> {
        let facts = self.reader_facts()?;
        Some(reader::build(&ReaderView::new(self.region), &facts))
    }

    /// The reader's text spot under a point, for a selection end (unit `057`).
    pub fn reader_spot(&self, p: Pos2) -> Option<Spot> {
        let facts = self.reader_facts()?;
        let view = ReaderView::new(self.region);
        let page = reader::build(&view, &facts);
        reader::spot_at(&view, &facts, &page, p)
    }

    /// The reader's selected text (unit `057`), or `None` without a selection.
    pub fn selected_text(&self) -> Option<String> {
        let sel = self.reader_sel.filter(|s| !s.is_empty())?;
        let facts = self.reader_facts()?;
        Some(reader::selected_text(
            &ReaderView::new(self.region),
            &facts,
            sel,
        ))
    }

    /// Opens a note in the reader (unit `054`). Its file becomes the one selection, as on a tree click: the tree opens to it,
    /// the panel shows it, and the map behind the reader moves only as far as needed to show the card. The reader reads the file
    /// unless it shows that file already, and starts at the mode's entry scroll.
    pub fn open_note(&mut self, id: u64, now: f64) {
        let Some(path) = self.notes.get(id).map(|n| n.path.clone()) else {
            return;
        };
        if let Some(f) = activity::file_of(&self.map, &path) {
            self.select(Some(Entry::File(f)), true, now);
        }
        let (text, colors) = match self.reader.take() {
            Some(r) if r.path == path => (r.text, r.colors),
            _ => {
                let text = Text::load(&self.content, &path);
                self.request_colors(&path, &text);
                (text, None)
            }
        };
        self.reader = Some(Reading {
            note: id,
            path,
            text,
            top: 0,
            scroll: 0.0,
            colors,
        });
        self.note_cursor = Some(id);
        self.reader_hover = None;
        self.reader_sel = None;
        self.reset_reader_top();
    }

    /// Asks the colorer for a text's syntax colors (unit `055`), when the text has lines and is not too long for them.
    fn request_colors(&self, path: &str, text: &Text) {
        if let (Some(c), Text::Lines(lines)) = (&self.colorer, text)
            && reader::colorable(lines)
        {
            c.request(path, lines.clone());
        }
    }

    /// Takes the colorer's results (unit `055`): the colors of the reader's text, or a theme error as a flash. Returns true when any arrived.
    pub fn poll_colors(&mut self, now: f64) -> bool {
        let done = self.colorer.as_ref().map(Colorer::poll).unwrap_or_default();
        for d in &done {
            match d {
                reader::Done::Colors { lines, spans, .. } => {
                    if let Some(r) = &mut self.reader
                        && matches!(&r.text, Text::Lines(l) if Arc::ptr_eq(l, lines))
                    {
                        r.colors = Some(spans.clone());
                    }
                }
                reader::Done::ThemeError(e) => {
                    self.flash = Some((e.clone(), now + FLASH_SECS));
                }
            }
        }
        !done.is_empty()
    }

    /// Closes the note reader. The map shows again.
    pub fn close_reader(&mut self) {
        self.reader = None;
        self.reader_hover = None;
        self.reader_sel = None;
    }

    /// The reader's entry scroll: the first row in `lines` mode; in `whole file` mode, the current note's box a quarter of the body below its top.
    fn reset_reader_top(&mut self) {
        let top = match self.reader_page() {
            Some(p) if self.reader_whole => reader::entry_top(&p, &ReaderView::new(self.region)),
            _ => 0,
        };
        if let Some(r) = &mut self.reader {
            r.top = top;
            r.scroll = 0.0;
        }
    }

    /// `n` and `N` (unit `054`): the next or previous note after the last one read, in the reader. At an end, or without notes, a flash.
    pub fn step_note(&mut self, forward: bool, now: f64) {
        let next = if forward {
            self.notes.next(self.note_cursor)
        } else {
            self.notes.previous(self.note_cursor)
        };
        match next.map(|n| n.id) {
            Some(id) => self.open_note(id, now),
            None => {
                let msg = if self.notes.is_empty() {
                    NO_NOTES
                } else if forward {
                    LAST_NOTE
                } else {
                    FIRST_NOTE
                };
                self.flash = Some((msg.to_string(), now + FLASH_SECS));
            }
        }
    }

    /// `z` and the mode chips (unit `054`): `whole file` or `lines` mode, at its entry scroll. The mode stays for the next note.
    pub fn set_reader_whole(&mut self, whole: bool) {
        if self.reader_whole != whole {
            self.reader_whole = whole;
            // The rows change, so the selection goes (unit `057`).
            self.reader_sel = None;
            self.reset_reader_top();
        }
    }

    /// Scrolls the reader by `rows`, within its limits, keeping the part of a row that is left.
    pub fn scroll_reader(&mut self, rows: f32) {
        let Some(max) = self.reader_page().map(|p| p.max_top) else {
            return;
        };
        let Some(r) = &mut self.reader else {
            return;
        };
        r.scroll += rows;
        let whole = r.scroll.trunc();
        r.scroll -= whole;
        r.top = (r.top.min(max) as isize + whole as isize).clamp(0, max as isize) as usize;
    }

    /// `g` / `G`, Home / End in the reader: its first or last row.
    fn reader_end(&mut self, last: bool) {
        let Some(max) = self.reader_page().map(|p| p.max_top) else {
            return;
        };
        if let Some(r) = &mut self.reader {
            r.top = if last { max } else { 0 };
            r.scroll = 0.0;
        }
    }

    /// `f` and Enter in the reader: it closes, and the map zooms to the note's first line on its card, or to the card.
    pub fn reader_to_map(&mut self, now: f64) {
        let Some(r) = self.reader.take() else {
            return;
        };
        self.reader_hover = None;
        self.reader_sel = None;
        let Some(note) = self.notes.get(r.note) else {
            return;
        };
        let Some(f) = activity::file_of(&self.map, &note.path) else {
            return;
        };
        let to = match note.lines {
            Some((a, _)) => self.line_camera(f, a),
            None => self.camera_for(Entry::File(f)),
        };
        self.motion.ease_to(to, now, ZOOM_TO_EASE);
    }

    /// A key while the reader shows (unit `054`). Returns false for a key that works as on the map.
    fn reader_key(&mut self, key: Key, shift: bool, now: f64) -> bool {
        let page = ReaderView::new(self.region)
            .visible_rows()
            .saturating_sub(2)
            .max(1) as f32;
        match key {
            // Esc clears a selection first (unit `057`).
            Key::Escape if self.reader_sel.is_some() => self.reader_sel = None,
            Key::Escape => self.close_reader(),
            Key::N => self.step_note(!shift, now),
            Key::Z => self.set_reader_whole(!self.reader_whole),
            Key::J | Key::ArrowDown => self.scroll_reader(1.0),
            Key::K | Key::ArrowUp => self.scroll_reader(-1.0),
            Key::PageDown => self.scroll_reader(page),
            Key::PageUp => self.scroll_reader(-page),
            Key::G => self.reader_end(shift),
            Key::Home => self.reader_end(false),
            Key::End => self.reader_end(true),
            Key::F | Key::Enter => self.reader_to_map(now),
            Key::T
            | Key::P
            | Key::R
            | Key::Num1
            | Key::Num2
            | Key::Num7
            | Key::Num8
            | Key::X
            | Key::OpenBracket
            | Key::CloseBracket => return false,
            // The map keys do nothing while the reader shows.
            _ => {}
        }
        true
    }

    /// Scrolls the outline by `rows` from its first row on the screen, within its limits, keeping the part of a row that is left.
    fn scroll_outline(&mut self, rows: f32, top: usize, max_top: usize) {
        self.outline_scroll += rows;
        let whole = self.outline_scroll.trunc();
        self.outline_scroll -= whole;
        self.outline_top = (top as isize + whole as isize).clamp(0, max_top as isize) as usize;
    }

    /// Takes the watcher's update, if one arrived, and applies it (unit `048`). Returns true when one did.
    pub fn poll_watch(&mut self, now: f64) -> bool {
        let Some(w) = &self.watcher else {
            return false;
        };
        let (update, error) = w.poll();
        if let Some(e) = error {
            self.flash = Some((format!("Re-index failed: {e}"), now + FLASH_SECS));
        }
        match update {
            Some(u) => {
                self.apply_index(u, now);
                true
            }
            None => false,
        }
    }

    /// Replaces the index after a re-index (unit `048`, the TUI's `apply_index`). The map is rebuilt; the selection, the hover,
    /// and the open directories keep their entries by path; the selected entry keeps its place on the screen.
    pub fn apply_index(&mut self, update: Update, now: f64) {
        let Update {
            index,
            changed,
            head_moved,
        } = update;
        let map = Map::new(index);
        let path_of = |m: &Map, e: Entry| match e {
            Entry::File(f) => m.tree.files[f].path.clone(),
            Entry::Dir(n) => m.tree.nodes[n].path.clone(),
        };
        let carry = |e: Entry| map.entry(&path_of(&self.map, e));
        let kept = self
            .map
            .tree
            .files
            .iter()
            .filter(|f| map.entry(&f.path).is_some())
            .count();
        let (added, removed) = (
            map.tree.files.len() - kept,
            self.map.tree.files.len() - kept,
        );
        let old_sel = self.selection;
        let new_sel = old_sel.and_then(carry);
        let old_place = old_sel.map(|e| self.map.rect_of(e).min);
        let sel_changed = matches!(old_sel, Some(Entry::File(f)) if changed.contains(&self.map.tree.files[f].path));
        let hover = self.hover.and_then(carry);
        let open: Vec<String> = (0..self.map.tree.nodes.len())
            .filter(|&n| self.filetree.is_open(n))
            .map(|n| self.map.tree.nodes[n].path.clone())
            .collect();
        let (height, top) = (self.filetree.height, self.filetree.top);

        self.map = map;
        self.bounds = Bounds {
            map: self.map.world(),
        };
        self.total_lines = self.map.index.files.iter().map(|f| f.lines).sum();
        self.filetree = FileTree::new(&self.map.tree);
        self.filetree.cursor = None;
        self.filetree.height = height;
        self.filetree.top = top;
        for p in &open {
            if let Some(Entry::Dir(n)) = self.map.entry(p) {
                self.filetree.set_open(n, true);
            }
        }
        // Entries of the old map are stale: no press, click, or row hover carries over.
        self.press = None;
        self.last_click = None;
        self.file_click = None;
        self.panel_hover = None;
        self.hover = hover;
        self.selection = new_sel;
        self.filetree.cursor = new_sel;
        if new_sel.is_none() || sel_changed {
            self.pick = None;
            self.outline_top = 0;
            self.outline_scroll = 0.0;
        }
        if let Some(e) = new_sel {
            self.filetree.reveal(&self.map.tree, e);
        }
        let rows = self.tree_rows();
        self.filetree.clamp_top(rows.len());
        self.filetree.keep_visible(&rows);
        // The selected entry keeps its screen place: the camera moves as far as its card or block moved. Then the limits apply.
        let by = match (new_sel, old_place) {
            (Some(e), Some(p)) => self.map.rect_of(e).min - p,
            _ => Vec2::ZERO,
        };
        self.motion.shift(&self.bounds, self.region, by);
        // Changed files compute their details again.
        if let Some(loader) = &mut self.loader {
            for p in &changed {
                loader.forget(p);
            }
        }
        self.requested.clear();
        self.update_tint();
        self.update_notes();
        // The reader reads its file again when it changed, and keeps its scroll (unit `054`). The selection goes (unit `057`).
        if let Some(r) = &mut self.reader
            && changed.contains(&r.path)
        {
            r.text = Text::load(&self.content, &r.path);
            r.colors = None;
            self.reader_sel = None;
            let (path, text) = (r.path.clone(), r.text.clone());
            self.request_colors(&path, &text);
        }
        if head_moved && let Some(root) = self.root.clone() {
            let paths = self
                .map
                .index
                .files
                .iter()
                .map(|f| f.path.clone())
                .collect();
            let loader =
                minimap_core::history::Loader::spawn(root, paths, minimap_core::history::now());
            match self.git {
                GitHistory::Ready(_) => self.history_reload = Some(loader),
                _ => self.git = GitHistory::Loading(loader),
            }
        }
        if added + removed > 0 {
            self.flash = Some((
                format!("{added} added · {removed} removed"),
                now + FLASH_SECS,
            ));
        }
    }

    /// Takes the listener's events and records them on the trail (unit `047`). Returns true when any arrived.
    pub fn poll_events(&mut self, now: f64) -> bool {
        let events = self
            .listener
            .as_ref()
            .map(Listener::poll)
            .unwrap_or_default();
        for e in &events {
            self.record_event(e, now);
        }
        !events.is_empty()
    }

    /// Records one agent event at an egui time. Each of its paths starts or restarts a glow (unit `049`).
    /// A note (unit `053`) adds a note, a history row with its text, and a glow in `FOCUS`. No card moves, and the camera does not change.
    pub fn record_event(&mut self, e: &AgentEvent, now: f64) {
        self.now = self.now.max(now);
        self.trail
            .record(e, std::time::Duration::from_secs_f64(now.max(0.0)));
        let note = self.notes.record(e);
        let (kind, paths, lines, session, text) = match e {
            AgentEvent::Trail {
                kind,
                paths,
                lines,
                session,
                ..
            } => (
                Act::from(*kind),
                paths.clone(),
                *lines,
                session,
                String::new(),
            ),
            AgentEvent::Note {
                path,
                lines,
                text,
                session,
                ..
            } => (
                Act::Note,
                vec![path.clone()],
                *lines,
                session,
                activity::first_line(text),
            ),
            AgentEvent::Clear => {
                self.glows.retain(|g| g.note);
                self.update_tint();
                return;
            }
            AgentEvent::ClearNotes => {
                self.glows.retain(|g| !g.note);
                self.update_notes();
                self.close_reader();
                self.note_cursor = None;
                return;
            }
        };
        // The agent history (unit `051`): one row per path, at the local time. A scrolled strip keeps its rows in place.
        let clock = minimap_core::history::local_clock();
        for p in &paths {
            self.activity.push(Record {
                seq: self.activity_seq,
                clock: clock.clone(),
                kind,
                path: p.clone(),
                lines,
                session: session.clone(),
                note,
                text: text.clone(),
            });
            self.activity_seq += 1;
        }
        // A scrolled strip keeps its rows in place (unit `057`). Newest first, new rows enter above them, so its top counts them.
        // Oldest first, new rows append at the end, and the oldest rows that drop past `ACTIVITY_MAX` leave its top.
        let mut dropped = 0;
        if self.activity.len() > ACTIVITY_MAX {
            let n = self.activity.len() - ACTIVITY_MAX;
            let filter = self.activity_filter;
            dropped = self
                .activity
                .drain(..n)
                .filter(|r| filter.matches(r.kind))
                .count();
        }
        if let Some(s) = session
            && !self.sessions.contains(s)
        {
            self.sessions.push(s.clone());
        }
        if self.activity_top > 0 {
            match self.activity_order {
                Order::NewestFirst if self.activity_filter.matches(kind) => {
                    self.activity_top += paths.len();
                }
                Order::NewestFirst => {}
                Order::OldestFirst => self.activity_top = self.activity_top.saturating_sub(dropped),
            }
        }
        for p in &paths {
            let glow = Glow {
                path: p.clone(),
                color: kind.rgb(),
                note: kind == Act::Note,
                at: now,
            };
            match self.glows.iter_mut().find(|g| &g.path == p) {
                Some(g) => *g = glow,
                None => self.glows.push(glow),
            }
        }
        self.update_tint();
        self.update_notes();
    }

    /// Sets each card's note ordinal from the notes (unit `053`): the place of its file's first note.
    pub fn update_notes(&mut self) {
        let mut noted = vec![None; self.map.tree.files.len()];
        for (k, n) in self.notes.all().iter().enumerate() {
            if let Some(Entry::File(f)) = self.map.entry(&n.path)
                && noted[f].is_none()
            {
                noted[f] = Some(k + 1);
            }
        }
        self.map.noted = noted;
    }

    /// A glow shows while its layer is on: layer 2 for a note's glow, else layer 1.
    fn glow_on(&self, g: &Glow) -> bool {
        if g.note {
            self.opts.toggles.notes
        } else {
            self.opts.toggles.trail
        }
    }

    /// `x`: clears the trail and its glows, as in the TUI. The notes and their glows stay (unit `053`).
    pub fn clear_trail(&mut self) {
        self.trail.clear();
        self.glows.retain(|g| g.note);
        self.update_tint();
    }

    /// A glow runs while its layer is on (units `049`, `053`): the window repaints each frame until it ends.
    pub fn glowing(&self) -> bool {
        self.glows
            .iter()
            .any(|g| self.glow_on(g) && self.now - g.at < scene::GLOW_SECS)
    }

    /// Sets each card's trail tint from the trail: its kind and whether it is fresh.
    pub fn update_tint(&mut self) {
        let trail = &self.trail;
        self.map.tint = self
            .map
            .tree
            .files
            .iter()
            .map(|f| {
                let m = trail.get(&f.path)?;
                Some((m.kind, trail.is_fresh(m)))
            })
            .collect();
    }

    /// The selected file's trail line and its color while layer 1 is on, for example `CC read this file · 5s ago`.
    pub fn trail_line(&self) -> Option<(String, u32)> {
        let Some(Entry::File(f)) = self.selection else {
            return None;
        };
        if !self.opts.toggles.trail {
            return None;
        }
        let m = self.trail.get(&self.map.tree.files[f].path)?;
        let now = std::time::Duration::from_secs_f64(self.now.max(0.0));
        let age = now.saturating_sub(m.at);
        Some((trail::describe(m, age), scene::trail_rgb(m.kind)))
    }

    /// The selected file's outline, once its details are loaded.
    pub fn outline(&self) -> &[minimap_core::outline::OutlineItem] {
        match self.selection {
            Some(Entry::File(f)) => self.file_details(f).map_or(&[], |d| &d.outline),
            _ => &[],
        }
    }

    /// The picked item's file and line, for the card's mark.
    pub fn picked_line(&self) -> Option<(usize, u32)> {
        let Some(Entry::File(f)) = self.selection else {
            return None;
        };
        Some((f, self.outline().get(self.pick?)?.line))
    }

    /// A click on the panel (unit `046`): an outline row picks its item, and a largest-file row selects its file, as a tree click does.
    /// A press that began on another row clicks nothing. A second click where a largest-file click was zooms to that file.
    fn click_panel(&mut self, began: Option<Hit>, at: Pos2, now: f64) {
        if let Some((t, p, f)) = self.file_click.take()
            && now - t <= DOUBLE_CLICK
            && p.distance(at) <= CLICK_SLOP
        {
            self.zoom_to(Entry::File(f), now);
            return;
        }
        let Some(h) = began.filter(|&h| self.panel().and_then(|p| p.hit(at)) == Some(h)) else {
            return;
        };
        match h {
            Hit::Outline(k) => self.pick_item(k, now),
            Hit::File(f) => {
                self.click_row(Entry::File(f), now);
                self.file_click = Some((now, at, f));
            }
            Hit::Note(id) => self.open_note(id, now),
        }
    }

    /// A click on an outline item (unit `046`): it picks the item, and the map zooms to its line on the card.
    pub fn pick_item(&mut self, k: usize, now: f64) {
        let Some(Entry::File(f)) = self.selection else {
            return;
        };
        let Some(line) = self.outline().get(k).map(|o| o.line) else {
            return;
        };
        self.close_reader();
        self.pick = Some(k);
        let to = self.line_camera(f, line);
        self.motion.ease_to(to, now, ZOOM_TO_EASE);
    }

    /// The camera that shows a card's line at [`ZOOM_TO_WIDTH`]: the card centered when it fits the region's height,
    /// else the line [`ZOOM_TO_TOP`] points below the region's top. A line past a capped card's shown lines is its last shown line.
    pub fn line_camera(&self, f: usize, line: u32) -> Camera {
        let zoom = ZOOM_TO_WIDTH / self.map.card_w();
        let card = self.map.card(f);
        if card.height() * zoom <= self.region.height() {
            return self.card_camera(f, zoom);
        }
        let y = scene::line_top(&self.map, f, line);
        self.bounds.clamp(
            self.region,
            Camera {
                center: pos2(
                    card.center().x,
                    y + (self.region.height() / 2.0 - ZOOM_TO_TOP) / zoom,
                ),
                zoom,
            },
        )
    }

    /// Scrolls the tree by `rows`, keeping the part of a row that is left for the next scroll.
    fn scroll_tree(&mut self, rows: f32) {
        self.tree_scroll += rows;
        let whole = self.tree_scroll.trunc();
        self.tree_scroll -= whole;
        let len = self.tree_rows().len();
        self.filetree.scroll(len, whole as isize);
    }

    /// `j` / `k` (unit `043`): the next or previous tree row, and the map follows. With no selection, the first row.
    pub fn tree_move(&mut self, delta: isize, now: f64) {
        let rows = self.tree_rows();
        if rows.is_empty() {
            return;
        }
        let at = self
            .selection
            .and_then(|e| rows.iter().position(|r| r.entry == e));
        let j = match at {
            Some(i) => (i as isize + delta).clamp(0, rows.len() as isize - 1) as usize,
            None => 0,
        };
        self.select(Some(rows[j].entry), true, now);
    }

    /// `g` / `G`, Home / End: the first or last tree row.
    pub fn tree_end(&mut self, last: bool, now: f64) {
        let rows = self.tree_rows();
        let row = if last { rows.last() } else { rows.first() };
        if let Some(r) = row {
            self.select(Some(r.entry), true, now);
        }
    }

    /// `l` and `h`: the shared decisions of `minimap_core::filetree`, as in the TUI.
    pub fn tree_step(&mut self, step: Option<Step>, now: f64) {
        match step {
            Some(Step::Open(n)) => self.set_open(n, true),
            Some(Step::Close(n)) => self.set_open(n, false),
            Some(Step::Select(e)) => self.select(Some(e), true, now),
            None => {}
        }
    }

    /// A click on a tree row: it selects the row, and the map follows. A directory row also opens or closes,
    /// except on the click that completes a double-click, which zooms to the entry.
    fn click_row(&mut self, e: Entry, now: f64) {
        self.close_reader();
        let double =
            matches!(self.last_click, Some((t, prev)) if prev == e && now - t <= DOUBLE_CLICK);
        self.select(Some(e), true, now);
        if double {
            self.zoom_to(e, now);
            self.last_click = None;
            return;
        }
        if let Entry::Dir(n) = e {
            let open = self.filetree.is_open(n);
            self.set_open(n, !open);
        }
        self.last_click = Some((now, e));
    }

    /// The camera that shows an entry: a card at [`ZOOM_TO_WIDTH`], or a directory's block fitted.
    pub fn camera_for(&self, e: Entry) -> Camera {
        match e {
            Entry::File(f) => self.card_camera(f, ZOOM_TO_WIDTH / self.map.card_w()),
            Entry::Dir(_) => self.bounds.fit_rect(self.region, self.map.rect_of(e)),
        }
    }

    /// The camera at a zoom that shows a card, centered horizontally: centered in the region when it fits its height,
    /// else with its top [`ZOOM_TO_TOP`] points below the region's top.
    pub fn card_camera(&self, f: usize, zoom: f32) -> Camera {
        let card = self.map.card(f);
        let y = if card.height() * zoom <= self.region.height() {
            card.center().y
        } else {
            card.top() + (self.region.height() / 2.0 - ZOOM_TO_TOP) / zoom
        };
        self.bounds.clamp(
            self.region,
            Camera {
                center: pos2(card.center().x, y),
                zoom,
            },
        )
    }

    /// Zooms to an entry, eased.
    pub fn zoom_to(&mut self, e: Entry, now: f64) {
        let to = self.camera_for(e);
        self.motion.ease_to(to, now, ZOOM_TO_EASE);
    }

    /// The draw items of the current frame.
    pub fn items(&self) -> Vec<Item<'_>> {
        let opts = Options {
            selected: self.selection,
            hovered: self.hover,
            mark: self.picked_line(),
            ..self.opts
        };
        let mut items = scene::scene(
            &self.map,
            &self.motion.cam,
            self.region,
            &opts,
            &self.loader,
        );
        // The note marks draw above the map (unit `053`), with layer 2, and the glows above them (unit `049`).
        if self.opts.toggles.notes && !self.notes.is_empty() {
            items.extend(scene::note_marks(
                &self.map,
                &self.motion.cam,
                self.region,
                &opts,
            ));
        }
        let glows: Vec<Glow> = self
            .glows
            .iter()
            .filter(|g| self.glow_on(g))
            .cloned()
            .collect();
        if !glows.is_empty() {
            items.extend(scene::glows(
                &self.map,
                &self.motion.cam,
                self.region,
                &glows,
                self.now,
            ));
        }
        items
    }

    /// The entry under a screen point. The reader covers the map (unit `054`).
    pub fn hit(&self, p: Pos2) -> Option<Entry> {
        if !self.region.contains(p) || self.reader.is_some() {
            return None;
        }
        scene::hit(&self.map, &self.items(), p, &self.motion.cam, self.region)
    }

    /// A click: it selects what it hits, and a second click on the same entry zooms to it.
    /// A click on empty map keeps the selection.
    fn click(&mut self, p: Pos2, now: f64) {
        let Some(e) = self.hit(p) else {
            self.last_click = None;
            return;
        };
        self.select(Some(e), false, now);
        match self.last_click {
            Some((t, prev)) if prev == e && now - t <= DOUBLE_CLICK => {
                self.zoom_to(e, now);
                self.last_click = None;
            }
            _ => self.last_click = Some((now, e)),
        }
    }

    /// The breadcrumb of the selection.
    pub fn breadcrumb(&self) -> Breadcrumb<'_> {
        let t = &self.map.tree;
        match self.selection {
            Some(Entry::File(f)) => t.file_crumb(f),
            Some(Entry::Dir(n)) => t.dir_crumb(n),
            None => t.root_crumb(),
        }
    }

    /// The hovered entry's path, as the TUI's top bar shows it.
    pub fn hover_text(&self) -> String {
        match self.hover {
            Some(Entry::File(f)) => self.map.tree.files[f].path.clone(),
            Some(Entry::Dir(n)) => self.map.tree.dir_label(n),
            None => String::new(),
        }
    }

    /// Turns a toggle on or off.
    pub fn toggle(&mut self, t: Toggle) {
        let g = &mut self.opts.toggles;
        match t {
            Toggle::Trail => g.trail = !g.trail,
            Toggle::Notes => g.notes = !g.notes,
            Toggle::Names => g.names = !g.names,
            Toggle::Shape => g.shape = !g.shape,
        }
    }

    /// The visible cards at the shape level or above, nearest to the center first. Empty below the shape level.
    pub fn detail_cards(&self) -> Vec<usize> {
        if scene::level(&self.map, &self.motion.cam) < Level::Shape {
            return Vec::new();
        }
        scene::visible_cards(&self.map, &self.motion.cam, self.region)
    }

    /// Takes finished details, and requests the visible cards' details when that set changed.
    /// The selected file comes first at every zoom, for the panel (unit `044`).
    pub fn update_details(&mut self) {
        if self.loader.is_none() {
            return;
        }
        let mut cards = self.detail_cards();
        if let Some(Entry::File(f)) = self.selection {
            cards.retain(|&c| c != f);
            cards.insert(0, f);
        }
        let changed = cards != self.requested;
        let paths: Vec<String> = if changed {
            cards
                .iter()
                .map(|&f| self.map.tree.files[f].path.clone())
                .collect()
        } else {
            Vec::new()
        };
        let Some(loader) = &mut self.loader else {
            return;
        };
        loader.poll();
        if changed {
            loader.request(paths);
            self.requested = cards;
        }
    }

    /// The visible cards at the shape level or above whose details have not arrived.
    pub fn details_missing(&self) -> usize {
        let Some(loader) = &self.loader else {
            return 0;
        };
        self.detail_cards()
            .iter()
            .filter(|&&f| loader.get(&self.map.tree.files[f].path).is_none())
            .count()
    }

    pub fn cam(&self) -> Camera {
        self.motion.cam
    }

    /// Sets the map region. The first region fits the map; a later one keeps the camera inside its limits.
    pub fn set_region(&mut self, region: Rect) {
        if !self.started {
            self.started = true;
            self.region = region;
            let cam = match self.selection.filter(|_| self.zoom_to_selection) {
                Some(e) => self.camera_for(e),
                None => self.bounds.fit(region),
            };
            self.motion.jump(cam);
            return;
        }
        if region != self.region {
            // A new width cuts and wraps the reader's rows anew, so its selection goes (unit `057`).
            if region.width() != self.region.width() {
                self.reader_sel = None;
            }
            // Each world point keeps its screen point: the camera moves as far as the region's center moved (unit `042`).
            let d = region.center() - self.region.center();
            self.region = region;
            self.motion.keep_on_screen(&self.bounds, region, d);
        }
    }

    /// The screen width of a card.
    pub fn card_width(&self) -> f32 {
        self.map.card_w() * self.motion.cam.zoom
    }

    fn zoom_now(&mut self, at: Pos2, factor: f32) {
        let cam = self
            .bounds
            .zoom_around(self.region, self.motion.cam, at, factor);
        self.motion.jump(cam);
    }

    fn ease(&mut self, to: Camera, now: f64) {
        self.motion.ease_to(to, now, KEY_EASE);
    }

    /// Zooms around the map region's center, eased, from where the camera is going.
    pub fn key_zoom(&mut self, factor: f32, now: f64) {
        let to = self.bounds.zoom_around(
            self.region,
            self.motion.target(),
            self.region.center(),
            factor,
        );
        self.ease(to, now);
    }

    /// Fits the whole map, eased.
    pub fn fit(&mut self, now: f64) {
        let to = self.bounds.fit(self.region);
        self.ease(to, now);
    }

    /// Pans by a share of the view: `dx`, `dy` in view widths and heights.
    pub fn pan_view(&mut self, dx: f32, dy: f32) {
        let view = self.motion.cam.view(self.region).size();
        self.motion
            .shift(&self.bounds, self.region, vec2(dx * view.x, dy * view.y));
    }

    /// A drag with the primary button: the content follows the pointer.
    pub fn on_drag(&mut self, delta: Vec2) {
        let by = -delta / self.motion.cam.zoom;
        self.motion.shift(&self.bounds, self.region, by);
    }

    /// One input event. `pointer` is the pointer's screen position, if known; `now` is egui's time in seconds.
    pub fn on_event(&mut self, ev: &Event, pointer: Option<Pos2>, now: f64) {
        let at = pointer
            .filter(|p| self.region.contains(*p))
            .unwrap_or(self.region.center());
        match ev {
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                ..
            } => {
                let view = self.tree_view;
                let on = if view.is_some_and(|v| v.edge.contains(*pos)) {
                    Some(PressOn::Edge(self.tree_width()))
                } else if self.panel_view.is_some_and(|v| v.edge.contains(*pos)) {
                    Some(PressOn::PanelEdge(self.panel_width()))
                } else if self.panel_view.is_some_and(|v| v.rect.contains(*pos)) {
                    Some(PressOn::Panel(self.panel().and_then(|p| p.hit(*pos))))
                } else if self.activity_view.is_some_and(|v| v.rect.contains(*pos)) {
                    Some(PressOn::Activity(self.strip().and_then(|s| s.hit(*pos))))
                } else if self.reader.is_some() && self.region.contains(*pos) {
                    let hit = self.reader_page().and_then(|p| p.hit(*pos));
                    let body = ReaderView::new(self.region).body.contains(*pos);
                    match self.reader_spot(*pos).filter(|_| hit.is_none() && body) {
                        Some(spot) => Some(PressOn::ReaderBody(spot)),
                        None => Some(PressOn::Reader(hit)),
                    }
                } else if let Some(v) = view.filter(|v| v.rect.contains(*pos)) {
                    let rows = self.tree_rows();
                    Some(PressOn::Tree(tree::entry_at(
                        &v,
                        &self.filetree,
                        &rows,
                        *pos,
                    )))
                } else if self.region.contains(*pos) {
                    Some(PressOn::Map)
                } else {
                    None
                };
                if let Some(on) = on {
                    self.press = Some(Press {
                        at: *pos,
                        last: *pos,
                        panning: false,
                        on,
                    });
                }
            }
            Event::PointerMoved(pos) => {
                if let Some(Press {
                    on: PressOn::Edge(from),
                    at,
                    ..
                }) = self.press
                {
                    self.set_tree(true, from + (pos.x - at.x));
                    return;
                }
                if let Some(Press {
                    on: PressOn::PanelEdge(from),
                    at,
                    ..
                }) = self.press
                {
                    // The panel's edge is at its left, so a drag to the left widens it.
                    self.set_panel(true, from - (pos.x - at.x));
                    return;
                }
                // A drag over the reader's body selects from the press's spot to the pointer's (unit `057`).
                if let Some(mut p) = self.press
                    && let PressOn::ReaderBody(anchor) = p.on
                {
                    if !p.panning && pos.distance(p.at) > CLICK_SLOP {
                        p.panning = true;
                        self.press = Some(p);
                    }
                    if p.panning
                        && let Some(head) = self.reader_spot(*pos)
                    {
                        self.reader_sel = Some(Selection { anchor, head });
                    }
                    return;
                }
                if let Some(mut p) = self.press.filter(|p| p.on == PressOn::Map) {
                    if !p.panning && pos.distance(p.at) > CLICK_SLOP {
                        // The content catches up with the pointer from where the press started.
                        p.panning = true;
                        p.last = p.at;
                    }
                    if p.panning {
                        self.on_drag(*pos - p.last);
                        p.last = *pos;
                    }
                    self.press = Some(p);
                }
            }
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                ..
            } => {
                if let Some(p) = self.press.take()
                    && !p.panning
                    && pos.distance(p.at) <= CLICK_SLOP
                {
                    match p.on {
                        PressOn::Map => self.click(p.at, now),
                        PressOn::Tree(Some(e)) => self.click_row(e, now),
                        PressOn::Panel(h) => self.click_panel(h, *pos, now),
                        // The pressed row's event, though a new event moved the rows since the press (unit `051`).
                        PressOn::Activity(Some(StripHit::Row(seq))) => {
                            self.click_activity(seq, now)
                        }
                        PressOn::Activity(Some(StripHit::Filter(f))) => self.set_filter(f),
                        PressOn::Activity(Some(StripHit::Order)) => {
                            self.set_order(self.activity_order.flip())
                        }
                        PressOn::Reader(Some(ReaderHit::Mode(whole))) => {
                            if self.reader_page().and_then(|p| p.hit(*pos))
                                == Some(ReaderHit::Mode(whole))
                            {
                                self.set_reader_whole(whole);
                            }
                        }
                        PressOn::ReaderBody(_) => self.reader_sel = None,
                        PressOn::Tree(None)
                        | PressOn::Edge(_)
                        | PressOn::PanelEdge(_)
                        | PressOn::Activity(None)
                        | PressOn::Reader(None) => {}
                    }
                }
            }
            Event::PointerGone => self.press = None,
            // Cmd+C (Ctrl+C on other platforms) copies the reader's selection; the frame hands it to the clipboard (unit `057`).
            Event::Copy => {
                if let Some(t) = self.selected_text().filter(|t| !t.is_empty()) {
                    self.copied = Some(t);
                }
            }
            // The reader covers the map: a pinch zooms nothing (unit `054`).
            Event::Zoom(_) if self.reader.is_some() => {}
            Event::Zoom(f) => self.zoom_now(at, *f),
            Event::MouseWheel {
                unit,
                delta,
                modifiers,
                ..
            } => {
                // The wheel over the tree scrolls it, by rows (unit `042`).
                if pointer.is_some_and(|p| self.tree_view.is_some_and(|v| v.rect.contains(p))) {
                    let rows = match unit {
                        MouseWheelUnit::Line => -delta.y * WHEEL_ROWS as f32,
                        MouseWheelUnit::Page => -delta.y * self.filetree.height.max(1) as f32,
                        MouseWheelUnit::Point => -delta.y / ROW_H,
                    };
                    self.scroll_tree(rows);
                    return;
                }
                // The wheel over the panel never moves the map (unit `044`). Over the outline's rows, it scrolls them (unit `046`).
                if let Some(p) =
                    pointer.filter(|p| self.panel_view.is_some_and(|v| v.rect.contains(*p)))
                {
                    if let Some(o) = self
                        .panel()
                        .and_then(|b| b.outline)
                        .filter(|o| o.rows.contains(p))
                    {
                        let rows = match unit {
                            MouseWheelUnit::Line => -delta.y * WHEEL_ROWS as f32,
                            MouseWheelUnit::Page => -delta.y * (o.rows.height() / ROW_H).max(1.0),
                            MouseWheelUnit::Point => -delta.y / ROW_H,
                        };
                        self.scroll_outline(rows, o.top, o.max_top);
                    }
                    return;
                }
                // The wheel over the agent history scrolls its rows (unit `051`), and never moves the map.
                if let Some(v) = self
                    .activity_view
                    .filter(|v| pointer.is_some_and(|p| v.rect.contains(p)))
                {
                    let rows = match unit {
                        MouseWheelUnit::Line => -delta.y * WHEEL_ROWS as f32,
                        MouseWheelUnit::Page => -delta.y * v.visible_rows().max(1) as f32,
                        MouseWheelUnit::Point => -delta.y / ROW_H,
                    };
                    self.scroll_activity(rows);
                    return;
                }
                if pointer.is_some_and(|p| !self.region.contains(p)) {
                    return;
                }
                // The wheel over the reader scrolls its rows (unit `054`).
                if self.reader.is_some() {
                    let body = ReaderView::new(self.region).visible_rows().max(1) as f32;
                    let rows = match unit {
                        MouseWheelUnit::Line => -delta.y * WHEEL_ROWS as f32,
                        MouseWheelUnit::Page => -delta.y * body,
                        MouseWheelUnit::Point => -delta.y / CODE_ROW,
                    };
                    self.scroll_reader(rows);
                    return;
                }
                match unit {
                    // A wheel zooms by notches; some mice send points, and their scroll pans.
                    MouseWheelUnit::Line | MouseWheelUnit::Page => {
                        self.zoom_now(at, WHEEL_ZOOM.powf(delta.y));
                    }
                    MouseWheelUnit::Point if modifiers.command => {
                        self.zoom_now(at, (delta.y / SCROLL_ZOOM_POINTS).exp());
                    }
                    MouseWheelUnit::Point => {
                        let cam = self.bounds.pan(self.region, self.motion.cam, *delta);
                        self.motion.jump(cam);
                    }
                }
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                repeat,
                ..
            } if !modifiers.command => {
                if self.reader.is_some() && self.reader_key(*key, modifiers.shift, now) {
                    return;
                }
                let step = if modifiers.shift { PAN_FAR } else { PAN_STEP };
                match key {
                    Key::Plus | Key::Equals => self.key_zoom(KEY_ZOOM, now),
                    Key::Minus => self.key_zoom(1.0 / KEY_ZOOM, now),
                    Key::Num0 => self.fit(now),
                    Key::Escape => self.select(None, false, now),
                    Key::T if !repeat => self.set_tree(!self.tree_on, self.tree_w),
                    Key::P if !repeat => self.set_panel(!self.panel_on, self.panel_w),
                    Key::R if !repeat => self.set_activity(!self.activity_on),
                    // The tree keys (unit `043`). They work while the tree is hidden: the tree cursor is the selection.
                    Key::J => self.tree_move(1, now),
                    Key::K => self.tree_move(-1, now),
                    Key::L => {
                        let rows = self.tree_rows();
                        let step = self.filetree.step_in(&self.map.tree, &rows, self.selection);
                        self.tree_step(step, now);
                    }
                    Key::H => {
                        let step = self.filetree.step_out(&self.map.tree, self.selection);
                        self.tree_step(step, now);
                    }
                    Key::G => self.tree_end(modifiers.shift, now),
                    Key::Home => self.tree_end(false, now),
                    Key::End => self.tree_end(true, now),
                    Key::OpenBracket => self.set_tree(self.tree_on, self.tree_width() - TREE_STEP),
                    Key::CloseBracket => self.set_tree(self.tree_on, self.tree_width() + TREE_STEP),
                    Key::F | Key::Enter => {
                        if let Some(e) = self.selection {
                            self.zoom_to(e, now);
                        }
                    }
                    Key::Num1 => self.toggle(Toggle::Trail),
                    Key::Num2 => self.toggle(Toggle::Notes),
                    Key::N => self.step_note(!modifiers.shift, now),
                    Key::X => self.clear_trail(),
                    Key::Num7 => self.toggle(Toggle::Names),
                    Key::Num8 => self.toggle(Toggle::Shape),
                    Key::W => self.pan_view(0.0, -step),
                    Key::S => self.pan_view(0.0, step),
                    Key::A => self.pan_view(-step, 0.0),
                    Key::D => self.pan_view(step, 0.0),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// The top bar: the breadcrumb of the selection with its last part bright, the counts, and the hovered path at the right.
    fn top_bar(&self, painter: &egui::Painter, bar: Rect) {
        painter.rect_filled(bar, 0.0, rgb(palette::BAR));
        let y = bar.center().y;
        let glyph = self.opts.char_w * BAR_TEXT;
        let mut right = self.hover_text();
        if right.is_empty()
            && let Some((f, _)) = self.flash.as_ref().filter(|(_, until)| self.now < *until)
        {
            right = f.clone();
        }
        if right.is_empty()
            && let Some(n) = &self.notice
        {
            right = n.clone();
        }
        let right_w = right.chars().count() as f32 * glyph;
        if !right.is_empty() {
            painter.text(
                pos2(bar.right() - 10.0, y),
                Align2::RIGHT_CENTER,
                &right,
                font(BAR_TEXT, false),
                rgb(palette::INK2),
            );
        }
        let room = ((bar.width() - 20.0 - right_w - 3.0 * glyph) / glyph).max(0.0) as usize;
        let bc = self.breadcrumb();
        let (crumb, bright_at) = crumb_line(&bc, room);
        let left = pos2(bar.left() + 10.0, y);
        let r = painter.text(
            left,
            Align2::LEFT_CENTER,
            &crumb,
            font(BAR_TEXT, false),
            rgb(palette::DIM),
        );
        if let Some(k) = bright_at {
            let bright: String = bc.bright().chars().take(room.saturating_sub(k)).collect();
            painter.text(
                pos2(left.x + k as f32 * glyph, y),
                Align2::LEFT_CENTER,
                bright,
                font(BAR_TEXT, true),
                rgb(palette::INK),
            );
        }
        let stats = format!(
            "{} files · {} dirs",
            thousands(self.map.index.files.len() as u64),
            thousands(self.map.tree.nodes.len() as u64)
        );
        let stats_w = stats.chars().count() as f32 * glyph;
        let x = r.right() + 2.0 * glyph;
        if x + stats_w < bar.right() - 10.0 - right_w - 2.0 * glyph {
            painter.text(
                pos2(x, y),
                Align2::LEFT_CENTER,
                stats,
                font(BAR_TEXT, false),
                rgb(palette::FAINT),
            );
        }
    }

    /// The status bar: the zoom, the toggles, and a key hint. Returns the toggles' rects.
    fn status_bar(&self, painter: &egui::Painter, bar: Rect) -> Vec<(Rect, Toggle)> {
        painter.rect_filled(bar, 0.0, rgb(palette::BAR));
        let y = bar.center().y;
        let zoom = format!(
            "zoom {:.3} · card {:.0} pt",
            self.motion.cam.zoom,
            self.card_width()
        );
        let r = painter.text(
            pos2(bar.left() + 10.0, y),
            Align2::LEFT_CENTER,
            zoom,
            font(BAR_TEXT, false),
            rgb(palette::DIM),
        );
        let t = self.opts.toggles;
        let mut x = r.right() + 24.0;
        let mut rects = Vec::new();
        for (on, label, which) in [
            (t.trail, "1 trail", Toggle::Trail),
            (t.notes, "2 notes", Toggle::Notes),
            (t.names, "7 names", Toggle::Names),
            (t.shape, "8 shape", Toggle::Shape),
        ] {
            let color = if on { palette::INK } else { palette::FAINT };
            let r = painter.text(
                pos2(x, y),
                Align2::LEFT_CENTER,
                label,
                font(BAR_TEXT, false),
                rgb(color),
            );
            rects.push((r.expand2(vec2(4.0, (BAR_H - r.height()) / 2.0)), which));
            x = r.right() + 16.0;
        }
        let hint = if self.reader.is_some() {
            READER_HINT
        } else {
            MAP_HINT
        };
        painter.text(
            pos2(bar.right() - 10.0, y),
            Align2::RIGHT_CENTER,
            hint,
            font(BAR_TEXT, false),
            rgb(palette::FAINT),
        );
        rects
    }
}

impl GuiApp {
    /// One frame in a ui that covers the window: the layout, the input, the capture step, the details, and the painting.
    /// `cpu_usage` is the last frame's CPU time, for the capture.
    pub fn frame(&mut self, ui: &mut egui::Ui, cpu_usage: Option<f32>) {
        let ctx = ui.ctx().clone();
        if !self.char_w_measured {
            self.char_w_measured = true;
            self.opts.char_w = ctx.fonts_mut(|f| f.glyph_width(&font(100.0, false), 'M') / 100.0);
        }
        self.opts.pixels_per_point = ctx.pixels_per_point();
        let screen = ui.max_rect();
        self.set_screen(screen);
        self.poll_history();

        let (events, pointer, now) =
            ctx.input(|i| (i.events.clone(), i.pointer.latest_pos(), i.time));
        self.now = now;
        self.poll_events(now);
        self.poll_watch(now);
        self.poll_colors(now);
        // A glow animates until it ends; then the window repaints only on input again (unit `049`).
        self.glows.retain(|g| now - g.at < scene::GLOW_SECS);
        if self.glowing() {
            ctx.request_repaint();
        }
        if let Some((_, until)) = self.flash
            && now < until
        {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(until - now));
        }
        // The panel's trail age counts seconds.
        if self.trail_line().is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        if self.capture.is_none() {
            for ev in &events {
                self.on_event(ev, pointer, now);
            }
            if let Some(t) = self.copied.take() {
                ctx.copy_text(t);
            }
            for (k, (r, which)) in self.toggle_rects.clone().into_iter().enumerate() {
                if ui
                    .interact(r, Id::new(("toggle", k)), Sense::click())
                    .clicked()
                {
                    self.toggle(which);
                }
            }
        }
        if let Some(mut cap) = self.capture.take() {
            cap.step(self, &ctx, &events, cpu_usage);
            self.capture = Some(cap);
        }
        if self.motion.tick(now) {
            ctx.request_repaint();
        }
        self.update_details();

        // The input can change the tree, so the regions come from the layout after it.
        let s = self.screen_layout(screen);
        let region = self.region;
        let on_edge = pointer.is_some_and(|p| {
            self.tree_view.is_some_and(|v| v.edge.contains(p))
                || self.panel_view.is_some_and(|v| v.edge.contains(p))
        });
        let dragging = matches!(
            self.press,
            Some(Press {
                on: PressOn::Edge(_),
                ..
            })
        );
        let panel_dragging = matches!(
            self.press,
            Some(Press {
                on: PressOn::PanelEdge(_),
                ..
            })
        );
        if (on_edge && self.press.is_none()) || dragging || panel_dragging {
            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        let painter = ui.painter();
        painter.rect_filled(region, 0.0, rgb(palette::PANE));
        let rows = self.tree_rows();
        // The hover comes from this frame's items, a tree row, or a panel row. Hover and selection change no geometry, so only a new hover redraws.
        // A largest-file row hovers its card, as a tree row does (unit `046`).
        let over = pointer.filter(|_| self.press.is_none() && self.capture.is_none());
        let mut built = self.panel();
        let panel_hover = over
            .filter(|p| !on_edge && self.panel_view.is_some_and(|v| v.rect.contains(*p)))
            .and_then(|p| built.as_ref()?.hit(p));
        if panel_hover != self.panel_hover {
            self.panel_hover = panel_hover;
            built = self.panel();
        }
        // The agent history's row under the pointer hovers its file's card (unit `051`).
        let mut strip = self.strip();
        let activity_hover = over
            .filter(|p| self.activity_view.is_some_and(|v| v.rect.contains(*p)))
            .and_then(|p| strip.as_ref()?.hit(p));
        if activity_hover != self.activity_hover {
            self.activity_hover = activity_hover;
            strip = self.strip();
        }
        // The reader covers the map (unit `054`): the map draws no items, and the pointer over the reader hovers a chip.
        let reading = self.reader.is_some();
        let mut page = self.reader_page();
        let reader_hover = over
            .filter(|p| reading && region.contains(*p))
            .and_then(|p| page.as_ref()?.hit(p));
        if reader_hover != self.reader_hover {
            self.reader_hover = reader_hover;
            page = self.reader_page();
        }
        let mut items = if reading { Vec::new() } else { self.items() };
        let hover = match over {
            Some(p) => {
                if region.contains(p) && reading {
                    None
                } else if region.contains(p) && !on_edge {
                    scene::hit(&self.map, &items, p, &self.motion.cam, region)
                } else if let Some(v) = self.tree_view.filter(|v| v.rect.contains(p)) {
                    tree::entry_at(&v, &self.filetree, &rows, p)
                } else if let Some(Hit::File(f)) = panel_hover {
                    Some(Entry::File(f))
                } else if let Some(StripHit::Row(seq)) = activity_hover {
                    self.activity
                        .iter()
                        .find(|r| r.seq == seq)
                        .and_then(|r| activity::file_of(&self.map, &r.path))
                        .map(Entry::File)
                } else {
                    None
                }
            }
            None => None,
        };
        if hover != self.hover {
            self.hover = hover;
            items = if reading { Vec::new() } else { self.items() };
        }
        // The bars, the tree, and the panel lay out their text first, so no glyph enters the font atlas after the map's text meshes read its size.
        self.top_bar(painter, s.top);
        let toggle_rects = self.status_bar(painter, s.status);
        if let Some(v) = &self.tree_view {
            let look = tree::Look {
                icons: self.opts.icons,
                char_w: self.opts.char_w,
                selected: self.selection,
                hovered: self.hover,
                dragging,
                trail: self.opts.toggles.trail,
                notes: self.opts.toggles.notes,
            };
            tree::paint(painter, v, &self.map, &self.filetree, &look);
        }
        if let (Some(v), Some(p)) = (&self.panel_view, &built) {
            panel::paint(painter, v, p, panel_dragging);
        }
        if let (Some(v), Some(s)) = (&self.activity_view, &strip) {
            activity::paint(painter, v, s);
        }
        match &page {
            Some(p) => reader::paint(painter, &ReaderView::new(region), p),
            None => paint::items(&painter.with_clip_rect(region), &items),
        }
        drop(items);
        self.toggle_rects = toggle_rects;
    }
}

impl eframe::App for GuiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        rgb(palette::PANE).to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.frame(ui, frame.info().cpu_usage);
    }
}

/// The breadcrumb line for a room in characters, cut at the left with `…` as the TUI cuts it,
/// and the character offset of its bright part.
pub fn crumb_line(bc: &Breadcrumb, room: usize) -> (String, Option<usize>) {
    let mut chars: Vec<char> = bc.text().chars().collect();
    if chars.len() > room {
        let cut = (chars.len() + 1).saturating_sub(room).min(chars.len());
        chars = ['…']
            .into_iter()
            .chain(chars[cut..].iter().copied())
            .collect();
    }
    chars.truncate(room);
    let bright: Vec<char> = bc.bright().chars().collect();
    let at = (!bright.is_empty() && bright.len() <= chars.len())
        .then(|| {
            (0..=chars.len() - bright.len())
                .rev()
                .find(|&i| chars[i..i + bright.len()] == bright[..])
        })
        .flatten();
    (chars.into_iter().collect(), at)
}
