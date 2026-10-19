//! The file tree in the window's left column (unit `042`): the TUI's rows from `minimap_core::filetree`, in screen points.
//!
//! Tree text has a screen size, so the zoom never changes it. The tree scrolls by whole rows, as the TUI's does.
//! Input goes through `GuiApp::on_event`; this module has the geometry, the hit tests, and the painting.

use eframe::egui::{Align2, Painter, Pos2, Rect, pos2, vec2};
use minimap_core::filetree::{Entry, FileTree, Row};
use minimap_core::{filetype, palette};

use crate::color::rgb;
use crate::fonts::font;
use crate::scene::Map;
use crate::text::{fit, thousands};

/// The tree's width in points: the default, the limits, and the step of `[` and `]`.
pub const TREE_W: f32 = 280.0;
pub const TREE_MIN: f32 = 160.0;
pub const TREE_MAX: f32 = 640.0;
pub const TREE_STEP: f32 = 40.0;
/// In a narrow window the tree narrows, down to [`TREE_MIN`], so the map keeps this width.
pub const MAP_MIN: f32 = 240.0;
/// A row's height and its text size, the bars' size.
pub const ROW_H: f32 = 20.0;
pub const TEXT: f32 = 13.0;
/// The zone over the right edge that a drag resizes from, centered on the edge.
pub const EDGE_GRAB: f32 = 6.0;
/// Rows per wheel notch.
pub const WHEEL_ROWS: usize = 3;
/// The tree's key hint, as the TUI's (`j/k · l/h · t hide · [ ] width`), with the zoom to the selection.
pub const HINT: &str = "j/k · l/h · ⏎ zoom · t hide · [ ] width";

/// The tree column's parts on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeView {
    /// The whole column, its 1-point edge included.
    pub rect: Rect,
    /// The first row: `FILES` and the file count.
    pub head: Rect,
    /// The rows between the head and the hint.
    pub rows: Rect,
    /// The last row: the key hint.
    pub hint: Rect,
    /// The drag zone over the right edge.
    pub edge: Rect,
}

impl TreeView {
    pub fn new(rect: Rect) -> TreeView {
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_H));
        let hint = Rect::from_min_max(pos2(rect.left(), rect.bottom() - ROW_H), rect.max);
        let rows = Rect::from_min_max(
            pos2(rect.left(), head.bottom()),
            pos2(rect.right(), hint.top().max(head.bottom())),
        );
        let edge = Rect::from_min_max(
            pos2(rect.right() - EDGE_GRAB / 2.0, rect.top()),
            pos2(rect.right() + EDGE_GRAB / 2.0, rect.bottom()),
        );
        TreeView {
            rect,
            head,
            rows,
            hint,
            edge,
        }
    }

    /// The whole rows that fit between the head and the hint.
    pub fn visible_rows(&self) -> usize {
        (self.rows.height() / ROW_H + 1e-3).floor().max(0.0) as usize
    }

    /// The rect of the `k`th row on the screen, from the top.
    pub fn row_rect(&self, k: usize) -> Rect {
        Rect::from_min_size(
            pos2(self.rows.left(), self.rows.top() + k as f32 * ROW_H),
            vec2(self.rows.width(), ROW_H),
        )
    }

    /// The row on the screen under a point, from the top. Points on the edge's drag zone are not rows.
    pub fn row_at(&self, p: Pos2) -> Option<usize> {
        if !self.rows.contains(p) || self.edge.contains(p) {
            return None;
        }
        let k = ((p.y - self.rows.top()) / ROW_H).floor() as usize;
        (k < self.visible_rows()).then_some(k)
    }
}

/// The tree row under a screen point, by the model's scroll.
pub fn entry_at(view: &TreeView, ft: &FileTree, rows: &[Row], p: Pos2) -> Option<Entry> {
    let k = view.row_at(p)?;
    rows.get(ft.top + k).map(|r| r.entry)
}

/// What the tree paints, apart from the model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Nerd Font icons; without them, rows have no icon column, as in the TUI.
    pub icons: bool,
    /// A monospace glyph's advance as a share of the text size.
    pub char_w: f32,
    pub selected: Option<Entry>,
    pub hovered: Option<Entry>,
    /// The edge is being dragged.
    pub dragging: bool,
    /// Layer 1, the trail, is on (unit `047`).
    pub trail: bool,
    /// Layer 2, the notes, is on (unit `053`).
    pub notes: bool,
}

/// Paints the column: the background, the edge, the head, the visible rows, and the hint (the TUI's `draw_tree`).
pub fn paint(painter: &Painter, view: &TreeView, map: &Map, ft: &FileTree, look: &Look) {
    let p = painter.with_clip_rect(view.rect);
    p.rect_filled(view.rect, 0.0, rgb(palette::SIDE));
    let (edge_w, edge_color) = if look.dragging {
        (2.0, palette::DIM)
    } else {
        (1.0, palette::RULE)
    };
    p.rect_filled(
        Rect::from_min_max(
            pos2(view.rect.right() - edge_w, view.rect.top()),
            view.rect.max,
        ),
        0.0,
        rgb(edge_color),
    );
    let cell = look.char_w * TEXT;
    // Text columns, as the TUI counts cells: column 0 is a margin, and the edge takes the last cell.
    let cols = ((view.rect.width() - edge_w) / cell).floor().max(0.0) as usize;
    let x = |c: usize| view.rect.left() + c as f32 * cell;
    let text = |r: Rect, c: usize, s: &str, color: u32, bold: bool| {
        p.text(
            pos2(x(c), r.center().y),
            Align2::LEFT_CENTER,
            s,
            font(TEXT, bold),
            rgb(color),
        );
    };
    let right = |r: Rect, s: &str, color: u32| {
        let n = s.chars().count();
        if cols > n + 1 {
            text(r, cols - 1 - n, s, color, false);
        }
    };
    text(
        view.head,
        1,
        &fit("FILES", cols.saturating_sub(2)),
        palette::LABEL,
        true,
    );
    right(
        view.head,
        &thousands(ft.count(0, true) as u64),
        palette::FAINT,
    );

    let tree = &map.tree;
    let rows = ft.rows(tree, true);
    for (k, row) in rows
        .iter()
        .enumerate()
        .skip(ft.top)
        .take(view.visible_rows())
    {
        let r = view.row_rect(k - ft.top);
        let sel = look.selected == Some(row.entry);
        if sel {
            p.rect_filled(r, 0.0, rgb(palette::CURSOR));
        } else if look.hovered == Some(row.entry) {
            p.rect_filled(r, 0.0, rgb(palette::TILE_HI));
        }
        let ix = 1 + 2 * row.depth;
        let name_x = if look.icons { ix + 4 } else { ix + 2 };
        match row.entry {
            Entry::Dir(n) => {
                let open = ft.is_open(n);
                let top_level = row.depth == 0;
                let color = if top_level {
                    palette::INK
                } else {
                    palette::INK2
                };
                let fg = if sel { palette::WHITE } else { color };
                let arrow = if open { "▾" } else { "▸" };
                text(
                    r,
                    ix,
                    arrow,
                    if sel { palette::WHITE } else { palette::DIM },
                    false,
                );
                if look.icons {
                    let glyph = if open {
                        filetype::FOLDER_OPEN
                    } else {
                        filetype::FOLDER_CLOSED
                    };
                    text(r, ix + 2, &glyph.to_string(), fg, false);
                }
                let count = thousands(ft.count(n, true) as u64);
                let count_x = cols.saturating_sub(1 + count.chars().count());
                let room = count_x.saturating_sub(1 + name_x);
                let name = fit(&format!("{}/", tree.nodes[n].name), room);
                text(r, name_x, &name, fg, top_level || sel);
                if count_x > name_x {
                    text(
                        r,
                        count_x,
                        &count,
                        if sel { palette::INK2 } else { palette::DIM },
                        false,
                    );
                }
            }
            Entry::File(f) => {
                let name = &tree.files[f].name;
                // A trail file has `▌` in its kind's color in the first cell, as in the TUI (unit `047`). A noted file's is `FOCUS` (unit `053`).
                if map.noted[f].is_some() && look.notes {
                    text(r, 0, "▌", palette::FOCUS, false);
                } else if let Some((k, _)) = map.tint[f].filter(|_| look.trail) {
                    text(r, 0, "▌", crate::scene::trail_rgb(k), false);
                }
                if look.icons {
                    let glyph = filetype::for_name(name).glyph;
                    text(r, ix + 2, &glyph.to_string(), map.colors[f], false);
                }
                let room = cols.saturating_sub(1 + name_x);
                let fg = if sel { palette::WHITE } else { palette::INK2 };
                text(r, name_x, &fit(name, room), fg, sel);
            }
        }
    }
    text(
        view.hint,
        1,
        &fit(HINT, cols.saturating_sub(2)),
        palette::FAINT,
        false,
    );
}
