//! The selection panel in the window's right column (unit `044`): the TUI's panel (`draw_side`) in screen points.
//!
//! [`build`] turns the selection's facts into draw items and the rects of its clickable rows, and [`paint`] draws them,
//! so tests read and click the panel with no window, as `scene` does for the map. Panel text has a screen size, so the zoom never changes it.
//! Input goes through `GuiApp::on_event`; this module has the geometry, the content, and the painting.

use eframe::egui::{Align2, Painter, Pos2, Rect, pos2, vec2};
use minimap_core::details::FileDetails;
use minimap_core::filetree::{Entry, FileTree};
use minimap_core::history::{self, History, ago, ago_phrase, describe};
use minimap_core::outline::OutlineItem;
use minimap_core::shape::{LineShape, MAX_LEN};
use minimap_core::{filetype, palette};

use crate::color::rgb;
use crate::fonts::font;
use crate::scene::Map;
use crate::text::{fit, kilo, thousands, wrap};
use crate::tree::{EDGE_GRAB, ROW_H, TEXT};

/// The panel's width in points: the default and the limits. The TUI's panel is 44 to 54 cells wide.
pub const PANEL_W: f32 = 360.0;
pub const PANEL_MIN: f32 = 240.0;
pub const PANEL_MAX: f32 = 640.0;
/// The edge's width in points at the panel's left. A dragged edge paints 2 points wide over the margin.
const EDGE: f32 = 1.0;
/// The message without a selection.
pub const EMPTY: &str = "Select a file in the tree or on the map.";
/// The shape's line-number gutter, in cells, as the TUI's.
pub const GUTTER: usize = 5;
/// The shape's most cells across, and its rows: 40 % of the free rows, from 4 to 12, as the TUI counts them (unit `045`).
pub const SHAPE_COLS: usize = 40;
pub const SHAPE_ROWS: (usize, usize) = (4, 12);
/// One line is at most this share of a row high, as one braille dot row in the TUI, so a short file does not stretch.
pub const SHAPE_LINE: f32 = 0.25;
/// A shape bar is 1 point shorter than its lines when it is this tall, or taller, as on the cards.
pub const SHAPE_GAP_FROM: f32 = 3.0;
/// A definition tick's width, in points.
pub const TICK_W: f32 = 4.0;
/// The outline's label: a click on an item zooms to its line (unit `046`).
pub const OUTLINE_LABEL: &str = "OUTLINE · click to zoom";
/// The notes' label (unit `054`), and the most note rows before `… K more`.
pub const NOTES_LABEL: &str = "NOTES · click to read";
pub const NOTE_ROWS: usize = 4;
/// The hint rows.
pub const FILE_HINT: &str = "f ⏎ zoom · p hide";
pub const DIR_HINT: &str = "l open · h close · j/k move · p hide";
pub const NONE_HINT: &str = "p hide";

/// The git history of the repo, as the TUI holds it (unit `008`).
pub enum GitHistory {
    /// No history source, as for an index JSON without git fields.
    None,
    /// The walk runs on a background thread.
    Loading(history::Loader),
    Ready(History),
    Failed(String),
}

/// The panel column's parts on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelView {
    /// The whole column, its 1-point edge included.
    pub rect: Rect,
    /// The rows above the hint row.
    pub body: Rect,
    /// The last row: the key hint.
    pub hint: Rect,
    /// The drag zone over the left edge.
    pub edge: Rect,
}

impl PanelView {
    pub fn new(rect: Rect) -> PanelView {
        let hint = Rect::from_min_max(pos2(rect.left(), rect.bottom() - ROW_H), rect.max);
        let body = Rect::from_min_max(rect.min, pos2(rect.right(), hint.top().max(rect.top())));
        let edge = Rect::from_min_max(
            pos2(rect.left() - EDGE_GRAB / 2.0, rect.top()),
            pos2(rect.left() + EDGE_GRAB / 2.0, rect.bottom()),
        );
        PanelView {
            rect,
            body,
            hint,
            edge,
        }
    }
}

/// What the panel shows: the map, the selection, and what is known about it.
pub struct Facts<'a> {
    pub map: &'a Map,
    /// The tree's model, for a directory's file count.
    pub filetree: &'a FileTree,
    pub selection: Option<Entry>,
    /// The selected file's details, once loaded.
    pub details: Option<&'a FileDetails>,
    pub git: &'a GitHistory,
    /// The lines of every indexed file.
    pub total_lines: u64,
    /// Nerd Font icons; without them, rows have no icon column, as in the TUI.
    pub icons: bool,
    /// A monospace glyph's advance as a share of the text size.
    pub char_w: f32,
    /// The picked outline item, by its index in the outline (unit `046`).
    pub picked: Option<usize>,
    /// The first outline item to show, before the panel's limits.
    pub outline_top: usize,
    /// The row under the pointer.
    pub hovered: Option<Hit>,
    /// The selected file's trail line and its color (unit `047`), for example `CC edited lines 40–58 · 2m ago`.
    pub trail: Option<(String, u32)>,
    /// The selected file's notes (unit `054`), oldest first, and the note that the reader shows.
    pub notes: Vec<NoteRow>,
    pub reading: Option<u64>,
}

/// One row of the NOTES section (unit `054`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRow {
    pub id: u64,
    /// The note's place among all notes, as `note 3 of 7` shows it.
    pub ordinal: usize,
    pub lines: Option<(u32, u32)>,
    /// The first line of the note's text.
    pub text: String,
}

/// A clickable row of the panel (unit `046`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// An outline item, by its index in the outline.
    Outline(usize),
    /// A directory's largest file.
    File(usize),
    /// A note of the selected file, by its id (unit `054`).
    Note(u64),
}

/// The outline's rows on the screen, for the wheel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlineView {
    /// The item rows, from the label's bottom to the last row.
    pub rows: Rect,
    /// The first item shown, and the largest first item: the first item of the last full page.
    pub top: usize,
    pub max_top: usize,
}

/// One draw item of the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum Draw {
    /// A text, its left end and vertical center at `at`.
    Text {
        at: Pos2,
        text: String,
        color: u32,
        bold: bool,
    },
    Fill {
        rect: Rect,
        color: u32,
    },
}

/// The panel's draw items, in paint order, and its clickable rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Panel {
    pub draws: Vec<Draw>,
    pub hits: Vec<(Rect, Hit)>,
    pub outline: Option<OutlineView>,
}

impl Panel {
    /// The clickable row under a point.
    pub fn hit(&self, p: Pos2) -> Option<Hit> {
        self.hits
            .iter()
            .find(|(r, _)| r.contains(p))
            .map(|&(_, h)| h)
    }

    /// The rect of a clickable row.
    pub fn rect_of(&self, h: Hit) -> Option<Rect> {
        self.hits.iter().find(|(_, k)| *k == h).map(|&(r, _)| r)
    }

    /// The texts from the top, in paint order.
    pub fn texts(&self) -> Vec<&str> {
        self.draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.as_str()),
                Draw::Fill { .. } => None,
            })
            .collect()
    }

    /// The first text item that is exactly `s`.
    pub fn find(&self, s: &str) -> Option<&Draw> {
        self.draws
            .iter()
            .find(|d| matches!(d, Draw::Text { text, .. } if text == s))
    }
}

/// The panel's text column, as the TUI's `Column`: cells across, rows of [`ROW_H`] down, to the hint row.
struct Column<'a> {
    out: &'a mut Panel,
    /// The x of cell 0, and one cell's width.
    x0: f32,
    cell: f32,
    /// The first cell of text, and the cells of text.
    x: usize,
    w: usize,
    /// The top of the next row, and the bottom of the last row.
    y: f32,
    y1: f32,
}

impl Column<'_> {
    /// A row fits above the hint row.
    fn ok(&self) -> bool {
        self.y + ROW_H <= self.y1 + 1e-3
    }

    /// The whole rows left above the hint row.
    fn rows_left(&self) -> usize {
        ((self.y1 - self.y) / ROW_H + 1e-3).floor().max(0.0) as usize
    }

    fn put(&mut self, cell: usize, s: &str, color: u32, bold: bool) {
        if s.is_empty() {
            return;
        }
        self.out.draws.push(Draw::Text {
            at: pos2(self.x0 + cell as f32 * self.cell, self.y + ROW_H / 2.0),
            text: s.to_string(),
            color,
            bold,
        });
    }

    /// A text right-aligned at the column's right end.
    fn right(&mut self, s: &str, color: u32) {
        let n = s.chars().count();
        if n <= self.w {
            self.put(self.x + self.w - n, s, color, false);
        }
    }

    fn line(&mut self, s: &str, color: u32, bold: bool) {
        if self.ok() {
            let s = fit(s, self.w);
            self.put(self.x, &s, color, bold);
        }
        self.y += ROW_H;
    }

    /// Word-wrapped lines, at most `max` of them.
    fn para(&mut self, s: &str, color: u32, max: usize) {
        for l in wrap(s, self.w).iter().take(max) {
            self.line(l, color, false);
        }
    }

    /// A section label, with an optional faint note right-aligned.
    fn label(&mut self, s: &str, note: &str) {
        if self.ok() {
            let s = fit(s, self.w);
            self.put(self.x, &s, palette::LABEL, true);
            self.right(note, palette::FAINT);
        }
        self.y += ROW_H;
    }

    /// A row with a 1-point rule across the panel.
    fn rule(&mut self, left: f32, right: f32) {
        if self.ok() {
            let y = self.y + ROW_H / 2.0;
            self.out.draws.push(Draw::Fill {
                rect: Rect::from_min_max(pos2(left, y - 0.5), pos2(right, y + 0.5)),
                color: palette::RULE,
            });
        }
        self.y += ROW_H;
    }

    /// A file's type icon at the cell `x` when icons are on. Returns the cell where its name starts.
    fn icon(&mut self, facts: &Facts, f: usize, x: usize) -> usize {
        if !facts.icons {
            return x;
        }
        let glyph = filetype::for_name(&facts.map.tree.files[f].name).glyph;
        self.put(x, &glyph.to_string(), facts.map.colors[f], false);
        x + 2
    }

    /// The next row across the text and its 1-cell margins: the TUI's selected-row fill.
    fn row_rect(&self) -> Rect {
        Rect::from_min_max(
            pos2(self.x0 + (self.x - 1) as f32 * self.cell, self.y),
            pos2(
                self.x0 + (self.x + self.w + 1) as f32 * self.cell,
                self.y + ROW_H,
            ),
        )
    }

    /// Makes the next row clickable, with the picked row's or the hovered row's fill. Returns true for the picked row.
    fn hit_row(&mut self, h: Hit, picked: bool, hovered: bool) -> bool {
        if !self.ok() {
            return false;
        }
        let r = self.row_rect();
        let fill = if picked {
            Some(palette::CURSOR)
        } else if hovered {
            Some(palette::TILE_HI)
        } else {
            None
        };
        if let Some(color) = fill {
            self.out.draws.push(Draw::Fill { rect: r, color });
        }
        self.out.hits.push((r, h));
        picked
    }
}

/// The panel's draw items for a view and the facts (the TUI's `draw_side`).
pub fn build(view: &PanelView, facts: &Facts) -> Panel {
    let mut panel = Panel::default();
    let cell = facts.char_w * TEXT;
    let x0 = view.rect.left() + EDGE;
    let cols = ((view.rect.width() - EDGE) / cell).floor().max(0.0) as usize;
    let mut c = Column {
        out: &mut panel,
        x0,
        cell,
        x: 1,
        w: cols.saturating_sub(2),
        y: view.body.top(),
        y1: view.body.bottom(),
    };
    repo_stats(&mut c, facts);
    c.rule(x0, view.rect.right());
    let hint = match facts.selection {
        Some(Entry::File(f)) => {
            file_info(&mut c, facts, f);
            FILE_HINT
        }
        Some(Entry::Dir(n)) => {
            dir_info(&mut c, facts, n);
            DIR_HINT
        }
        None => {
            c.line(EMPTY, palette::FAINT, false);
            NONE_HINT
        }
    };
    // The hint row is the panel's last row.
    c.y = view.hint.top();
    c.y1 = view.hint.bottom();
    c.line(hint, palette::FAINT, false);
    panel
}

/// The repo name, then `N files · M dirs · K lines` right-aligned on the same row, or on the next row when both do not fit.
fn repo_stats(c: &mut Column, facts: &Facts) {
    let map = facts.map;
    let root = &map.tree.root_name;
    let stats = format!(
        "{} files · {} dirs · {} lines",
        thousands(map.index.files.len() as u64),
        thousands(map.tree.nodes.len() as u64),
        kilo(facts.total_lines)
    );
    if c.ok() {
        let s = fit(root, c.w);
        c.put(c.x, &s, palette::INK, true);
    }
    if root.chars().count() + 1 + stats.chars().count() > c.w {
        c.y += ROW_H;
    }
    if c.ok() {
        let s = fit(&stats, c.w);
        c.right(&s, palette::DIM);
    }
    c.y += ROW_H;
}

/// The rows that the git group takes: its label and summary, then the recent commits. 0 without a history source.
fn git_rows(facts: &Facts, f: usize) -> usize {
    match facts.git {
        GitHistory::None => 0,
        GitHistory::Loading(_) | GitHistory::Failed(_) => 2,
        GitHistory::Ready(h) => {
            2 + h
                .file(&facts.map.index.files[f].path)
                .map_or(0, |fh| h.recent(fh).count())
        }
    }
}

/// Icon and name, directory, namespace, `N lines`, then the outline and the git group (`drawSelection`).
fn file_info(c: &mut Column, facts: &Facts, f: usize) {
    let map = facts.map;
    let tf = &map.tree.files[f];
    let file = &map.index.files[f];
    if c.ok() {
        let x = c.icon(facts, f, c.x);
        let room = (c.x + c.w).saturating_sub(x);
        c.put(x, &fit(&tf.name, room), palette::INK, true);
    }
    c.y += ROW_H;
    let dir = tf.path.rsplit_once('/').map_or(".", |(d, _)| d);
    c.para(&format!("{dir}/"), palette::DIM, 2);
    if let Some(ns) = facts.details.and_then(|d| d.namespace.as_deref()) {
        c.para(ns, palette::INK2, 2);
    }
    let mut size = if file.binary {
        "binary".to_string()
    } else {
        format!("{} lines", thousands(file.lines))
    };
    if file.untracked {
        size.push_str(" · untracked");
    }
    if file.generated {
        size.push_str(" · generated");
    }
    c.line(&size, palette::DIM, false);
    if let Some((line, color)) = &facts.trail {
        c.para(line, *color, 2);
    }
    c.y += ROW_H;
    if !facts.notes.is_empty() {
        notes_list(c, facts);
        c.y += ROW_H;
    }

    let git_rows = git_rows(facts, f);
    let outline: &[OutlineItem] = facts.details.map_or(&[], |d| &d.outline);
    if let Some(shape) = facts.details.and_then(|d| d.shape.as_ref()) {
        let avail = c.rows_left().saturating_sub(git_rows + 1);
        if shape.lines > 0 && !shape.rows.is_empty() && avail > 7 {
            let picked = facts.picked.and_then(|k| outline.get(k)).map(|o| o.line);
            shape_box(c, shape, outline, picked, avail);
        }
    }
    let outline_rows = c.rows_left().saturating_sub(git_rows + 1);
    if !outline.is_empty() && outline_rows >= 2 {
        outline_list(c, facts, outline, outline_rows);
    }
    if git_rows > 0 {
        c.y = (c.y + ROW_H).max(c.y1 - git_rows as f32 * ROW_H);
        git(c, facts, f);
    }
}

/// SHAPE (unit `045`): the whole file as bars, with line numbers at the first, middle, and last rows, and a tick at each definition
/// in the marker column at the right, as the TUI's braille shape. The picked item's line (unit `046`) has `◀` in place of its tick,
/// and its sample bar is white.
fn shape_box(
    c: &mut Column,
    shape: &LineShape,
    outline: &[OutlineItem],
    picked: Option<u32>,
    avail: usize,
) {
    let rows = ((avail as f32 * 0.4).floor() as usize).clamp(SHAPE_ROWS.0, SHAPE_ROWS.1);
    let cols = c.w.saturating_sub(GUTTER + 2).min(SHAPE_COLS);
    if cols < 4 {
        return;
    }
    c.label("SHAPE", "");
    let n = shape.lines as usize;
    let top = c.y;
    let left = c.x0 + (c.x + GUTTER) as f32 * c.cell;
    let width = cols as f32 * c.cell;
    let line_h = (rows as f32 * ROW_H / n as f32).min(SHAPE_LINE * ROW_H);
    let used = ((n as f32 * line_h / ROW_H) - 1e-3).ceil().max(1.0) as usize;
    c.out.draws.push(Draw::Fill {
        rect: Rect::from_min_size(pos2(left, top), vec2(width, used as f32 * ROW_H)),
        color: palette::MINI_BG,
    });
    // Row `r` of `R` samples covers lines `r * n / R` to `(r + 1) * n / R`.
    let samples = shape.rows.len();
    let col = |x: u16| left + f32::from(x.min(MAX_LEN)) / f32::from(MAX_LEN) * width;
    let line_of = |line: u32| (line.max(1) as usize - 1).min(n - 1);
    let picked = picked.map(line_of);
    for (r, row) in shape.rows.iter().enumerate() {
        let a = r * n / samples;
        let b = ((r + 1) * n / samples).max(a + 1);
        let (x0, x1) = (col(row.indent), col(row.len));
        if x1 <= x0 {
            continue;
        }
        let h = (b - a) as f32 * line_h;
        let gap = if h >= SHAPE_GAP_FROM { 1.0 } else { 0.0 };
        let holds_pick = picked.is_some_and(|l| (a..b).contains(&l));
        c.out.draws.push(Draw::Fill {
            rect: Rect::from_min_size(pos2(x0, top + a as f32 * line_h), vec2(x1 - x0, h - gap)),
            color: if holds_pick {
                palette::WHITE
            } else {
                palette::MINI_DOT
            },
        });
    }
    let tick_cell = c.x + GUTTER + cols + 1;
    let tick_x = c.x0 + tick_cell as f32 * c.cell;
    for item in outline {
        let l = line_of(item.line);
        if picked == Some(l) {
            continue;
        }
        c.out.draws.push(Draw::Fill {
            rect: Rect::from_min_size(
                pos2(tick_x, top + l as f32 * line_h),
                vec2(TICK_W, line_h.max(1.0)),
            ),
            color: palette::FAINT,
        });
    }
    // Line numbers at the first, middle, and last rows of the lines: a row's first line, and the line count at the last row.
    let last = used - 1;
    let mid = last / 2;
    for r in 0..used {
        if !(r == 0 || r == last || (r == mid && mid > 0 && mid < last)) {
            continue;
        }
        let line = if r == last {
            n
        } else {
            (r as f32 * ROW_H / line_h).floor() as usize + 1
        };
        let s = line.to_string();
        let x = (c.x + GUTTER - 1).saturating_sub(s.chars().count());
        let y = c.y;
        c.y = top + r as f32 * ROW_H;
        c.put(x, &s, palette::FAINT, false);
        c.y = y;
    }
    if let Some(l) = picked {
        // `◀` centered on the picked line, as the TUI marks the browsed item.
        c.out.draws.push(Draw::Text {
            at: pos2(tick_x, top + (l as f32 + 0.5) * line_h),
            text: "◀".to_string(),
            color: palette::WHITE,
            bold: true,
        });
    }
    c.y = top + used as f32 * ROW_H + ROW_H;
}

/// OUTLINE: `line  name` rows from the scroll's first item, private items faint, and `… K more` when they do not fit.
/// Each row is clickable (unit `046`); the picked row is `CURSOR` with white text, as the TUI's browsed item.
fn outline_list(c: &mut Column, facts: &Facts, outline: &[OutlineItem], rows: usize) {
    let avail = rows.saturating_sub(1).max(1);
    // A list longer than the room keeps its last row for `… K more`.
    let shown = if outline.len() > avail {
        (avail - 1).max(1)
    } else {
        avail
    };
    let max_top = outline.len().saturating_sub(shown);
    let top = facts.outline_top.min(max_top);
    let note = if top > 0 {
        format!("{top} above")
    } else {
        outline.len().to_string()
    };
    c.label(OUTLINE_LABEL, &note);
    let rows_top = c.y;
    let end = outline.len().min(top + shown);
    for (k, item) in outline.iter().enumerate().take(end).skip(top) {
        let hit = Hit::Outline(k);
        let picked = c.hit_row(hit, facts.picked == Some(k), facts.hovered == Some(hit));
        let color = if picked {
            palette::WHITE
        } else if item.private {
            palette::FAINT
        } else {
            palette::DIM
        };
        // A nested item's name indents 2 spaces per depth (unit 032).
        let indent = "  ".repeat(usize::from(item.depth));
        c.line(
            &format!("{:>4}  {indent}{}", item.line, item.name),
            color,
            false,
        );
    }
    c.out.outline = Some(OutlineView {
        rows: Rect::from_min_max(
            pos2(c.x0, rows_top),
            pos2(
                c.x0 + (c.x + c.w + 1) as f32 * c.cell,
                rows_top + shown as f32 * ROW_H,
            ),
        ),
        top,
        max_top,
    });
    if end < outline.len() {
        c.line(
            &format!("  … {} more", outline.len() - end),
            palette::FAINT,
            false,
        );
    }
}

/// NOTES (unit `054`): one row per note of the file, oldest first: the ordinal in `FOCUS`, the lines, and the text's first line.
/// At most [`NOTE_ROWS`] rows, then `… K more`. A click on a row reads its note; the reader's note is `CURSOR` with white text.
fn notes_list(c: &mut Column, facts: &Facts) {
    c.label(NOTES_LABEL, &facts.notes.len().to_string());
    for n in facts.notes.iter().take(NOTE_ROWS) {
        let hit = Hit::Note(n.id);
        let picked = c.hit_row(hit, facts.reading == Some(n.id), facts.hovered == Some(hit));
        if c.ok() {
            let ord = format!("{:>3}", n.ordinal);
            c.put(
                c.x,
                &ord,
                if picked {
                    palette::WHITE
                } else {
                    palette::FOCUS
                },
                true,
            );
            let lines = match n.lines {
                Some((a, b)) if a == b => a.to_string(),
                Some((a, b)) => format!("{a}–{b}"),
                None => "file".to_string(),
            };
            c.put(c.x + 5, &lines, palette::DIM, false);
            let at = c.x + 5 + lines.chars().count().max(9) + 2;
            let room = (c.x + c.w).saturating_sub(at);
            let color = if picked {
                palette::WHITE
            } else {
                palette::INK2
            };
            c.put(at, &fit(&n.text, room), color, false);
        }
        c.y += ROW_H;
    }
    if facts.notes.len() > NOTE_ROWS {
        c.line(
            &format!("  … {} more · n next", facts.notes.len() - NOTE_ROWS),
            palette::FAINT,
            false,
        );
    }
}

/// GIT: `N commits`, `changed X ago · created Y ago`, then the recent commits as age and description.
fn git(c: &mut Column, facts: &Facts, f: usize) {
    match facts.git {
        GitHistory::None => {}
        GitHistory::Loading(_) => {
            c.label("GIT", "loading");
            c.line("Reading the git history…", palette::FAINT, false);
        }
        GitHistory::Failed(e) => {
            c.label("GIT", "");
            c.line(&format!("Git history failed: {e}"), palette::FAINT, false);
        }
        GitHistory::Ready(h) => {
            let Some(fh) = h.file(&facts.map.index.files[f].path) else {
                c.label("GIT", "0 commits");
                c.line("No commits yet.", palette::FAINT, false);
                return;
            };
            let count = if fh.count == 1 {
                "1 commit".to_string()
            } else {
                format!("{} commits", thousands(u64::from(fh.count)))
            };
            c.label("GIT", &count);
            let summary = format!(
                "changed {} · created {}",
                ago_phrase(h.age_days(fh.newest)),
                ago_phrase(h.age_days(fh.oldest))
            );
            c.line(&summary, palette::DIM, false);
            for commit in h.recent(fh) {
                if !c.ok() {
                    break;
                }
                let age = format!("{:>5}", ago(h.age_days(commit.time)));
                c.put(c.x, &age, palette::INK2, false);
                let room = c.w.saturating_sub(7);
                c.put(
                    c.x + 7,
                    &fit(describe(&commit.subject), room),
                    palette::DIM,
                    false,
                );
                c.y += ROW_H;
            }
        }
    }
}

/// Path, counts, lines, then the largest files (`drawDirInfo`).
fn dir_info(c: &mut Column, facts: &Facts, n: usize) {
    let map = facts.map;
    let node = &map.tree.nodes[n];
    // The root has no path: it shows the repo name, as its header does.
    let label = if n == 0 {
        format!("{}/", map.tree.root_name)
    } else {
        format!("{}/", node.path)
    };
    c.line(&label, palette::INK, true);
    let count = facts.filetree.count(n, true);
    c.line(
        &format!(
            "{} files · {} subdirectories",
            thousands(count as u64),
            node.kids.len()
        ),
        palette::DIM,
        false,
    );
    // The files under the directory, in tree order: each node's files, then its children.
    let mut files = Vec::new();
    let mut stack = vec![n];
    while let Some(m) = stack.pop() {
        let nm = &map.tree.nodes[m];
        files.extend(nm.files.iter().copied().filter(|&f| map.visible(f)));
        stack.extend(nm.kids.iter().rev());
    }
    let lines: u64 = files.iter().map(|&f| map.index.files[f].lines).sum();
    c.line(&format!("{} lines", kilo(lines)), palette::DIM, false);
    c.y += ROW_H;
    c.label("LARGEST FILES", "lines");
    let mut text: Vec<usize> = files
        .into_iter()
        .filter(|&f| !map.index.files[f].binary)
        .collect();
    text.sort_by_key(|&f| std::cmp::Reverse(map.index.files[f].lines));
    let skip = if n == 0 { 0 } else { node.path.len() + 1 };
    for f in text {
        if !c.ok() {
            break;
        }
        let tf = &map.tree.files[f];
        let hit = Hit::File(f);
        c.hit_row(hit, false, facts.hovered == Some(hit));
        let x = c.icon(facts, f, c.x);
        let room = (c.x + c.w).saturating_sub(8 + x);
        c.put(x, &fit(&tf.path[skip..], room), palette::INK2, false);
        c.right(&map.index.files[f].lines.to_string(), palette::DIM);
        c.y += ROW_H;
    }
}

/// Paints the column: the background, the edge, and the draw items.
pub fn paint(painter: &Painter, view: &PanelView, panel: &Panel, dragging: bool) {
    let p = painter.with_clip_rect(view.rect);
    p.rect_filled(view.rect, 0.0, rgb(palette::SIDE));
    let (edge_w, edge_color) = if dragging {
        (2.0, palette::DIM)
    } else {
        (EDGE, palette::RULE)
    };
    p.rect_filled(
        Rect::from_min_size(view.rect.min, vec2(edge_w, view.rect.height())),
        0.0,
        rgb(edge_color),
    );
    paint_draws(&p, &panel.draws);
}

/// Paints draw items: fills, and texts at the panel's text size.
pub fn paint_draws(p: &Painter, draws: &[Draw]) {
    for d in draws {
        match d {
            Draw::Fill { rect, color } => {
                p.rect_filled(*rect, 0.0, rgb(*color));
            }
            Draw::Text {
                at,
                text,
                color,
                bold,
            } => {
                p.text(
                    *at,
                    Align2::LEFT_CENTER,
                    text,
                    font(TEXT, *bold),
                    rgb(*color),
                );
            }
        }
    }
}
