//! The scene: what the map region shows for a camera, as a flat list of draw items in paint order.
//!
//! The painter reads only these items, so tests check the scene without a window or a GPU.
//! Only cards and headers that intersect the map region give items.

use std::collections::HashMap;

use eframe::egui::{Color32, Pos2, Rect, Vec2, pos2, vec2};
use minimap_core::details::{Details, FileDetails, Loader};
use minimap_core::events::Kind as TrailKind;
use minimap_core::shape::{MAX_LEN, MEDIAN_ABOVE, ShapeRow, summarize};
use minimap_core::{Index, Layout, LayoutConfig, Tree, filetype, palette};

use crate::camera::{CELL, Camera};
use crate::color::{alpha, mix, rgb, with_alpha};
use crate::text::fit;

/// Text in the world (unit `040`): every text and icon has a size in world points, and its screen size is that size times the zoom.
/// A text this large on the screen, or larger, draws as text.
pub const TEXT_FROM: f32 = 7.0;
/// From this size up to [`TEXT_FROM`], a text draws as a bar (greeking); below it, not at all.
pub const GREEK_FROM: f32 = 2.0;
/// A greeked bar is this share of its text size high, in the text color at this opacity.
pub const GREEK_H: f32 = 0.5;
pub const GREEK_ALPHA: f32 = 0.35;
/// Glyph sizes snap down to `LADDER_BASE * LADDER_STEP^n` for a whole `n` of 0 or more, with no top end,
/// so the glyph atlas holds a bounded set of sizes. The painter scales each text from its ladder size to its exact size.
pub const LADDER_BASE: f32 = 7.0;
pub const LADDER_STEP: f32 = 1.1;
/// A directory name larger than this on the screen fades out over the next [`FADE_SPAN`] of it, so a name never fills the window.
/// 200, not 96: a block fitted in the window gives its name about `776 / chars` points, so a 4-character name is 194 points (unit `040`).
pub const FADE_FROM: f32 = 200.0;
pub const FADE_SPAN: f32 = 0.5;

/// The glyph size for a screen size: the largest ladder size at or below it. `None` below the ladder.
pub fn ladder(size: f32) -> Option<f32> {
    if size < LADDER_BASE {
        return None;
    }
    let n = ((size / LADDER_BASE).ln() / LADDER_STEP.ln() + 1e-4)
        .floor()
        .max(0.0) as i32;
    Some(ladder_size(n))
}

/// The ladder's `n`th size.
pub fn ladder_size(n: i32) -> f32 {
    (f64::from(LADDER_BASE) * f64::from(LADDER_STEP).powi(n)) as f32
}

/// A directory name's opacity at a screen size: 1 up to [`FADE_FROM`], then down to 0 over [`FADE_SPAN`] of it.
pub fn fade(size: f32) -> f32 {
    (1.0 - (size - FADE_FROM) / (FADE_SPAN * FADE_FROM)).clamp(0.0, 1.0)
}

/// The card widths on the screen, in points, where each detail level starts. Below [`ICON_FROM`], a card is a block.
/// Unit `040` moved the name and the shape down from 60 and 110: names greek below a card width of 84, and a shape at 60
/// shows a card's code where it was an empty box.
pub const ICON_FROM: f32 = 20.0;
pub const NAME_FROM: f32 = 40.0;
pub const SHAPE_FROM: f32 = 60.0;
/// A block's type color share over the pane.
pub const BLOCK_MIX: f32 = 0.55;
/// Card text in world points on a card 168 points wide (unit `040`), in proportion to the card width:
/// the name row (one layout row), the icon, and the name and the line count.
pub const NAME_ROW_PT: f32 = 18.0;
pub const ICON_PT: f32 = 16.0;
pub const NAME_PT: f32 = 14.0;
/// Padding inside a card, as a share of its width.
pub const PAD: f32 = 0.06;
/// A card's corner radius, as a share of its width.
pub const RADIUS: f32 = 0.04;
/// A text row is this many times its text size.
pub const LINE: f32 = 1.25;
/// The text columns that the shape spans across the body. Longer lines clamp at its edge.
pub const SHAPE_CHARS: f32 = 120.0;
/// The shape's type color share over the card, and the loading rect's share of `FAINT`.
pub const SHAPE_MIX: f32 = 0.70;
pub const LOADING_MIX: f32 = 0.20;
/// A shape bar is 1 point shorter than its row when the row is this tall on the screen, or taller.
pub const BAR_GAP_FROM: f32 = 3.0;
/// The selected card's ring, in points.
pub const RING: f32 = 2.0;
/// The cut mark (unit `041`): a dashed line along the bottom of a card whose body cannot hold every line, which only a capped
/// generated card has. Its dashes are this many world points high (at least 1 screen point); the shape leaves this much of the body
/// above the bottom to it; it has this many dashes across the body.
pub const CUT_PT: f32 = 2.0;
pub const CUT_ROOM: f32 = 6.0;
pub const CUT_DASHES: usize = 12;
/// The picked outline line's mark (unit `046`): at least this tall on the screen, in points, across the middle half of the card's left padding.
pub const MARK_MIN: f32 = 2.0;
/// The trail (unit `047`): a trail card's fill is its kind's color at these shares over the card, while fresh and after;
/// at the block level, at these shares over the pane, so a trail card shows at the fit view.
pub const TRAIL_FRESH_MIX: f32 = 0.45;
pub const TRAIL_OLD_MIX: f32 = 0.20;
pub const TRAIL_BLOCK_FRESH_MIX: f32 = 0.90;
pub const TRAIL_BLOCK_OLD_MIX: f32 = 0.60;

/// The glow (unit `049`): each agent event lights its file's card at full strength for [`GLOW_HOLD`] seconds,
/// then fades out over [`GLOW_FADE`], so the user sees where the agent works at every zoom.
pub const GLOW_HOLD: f64 = 1.0;
pub const GLOW_FADE: f64 = 3.0;
pub const GLOW_SECS: f64 = GLOW_HOLD + GLOW_FADE;
/// The glow box at the block level: the card grown about its center to at least this size on the screen, in points,
/// so a glow shows at the fit view, where a card is about 4 points wide. From the icon level, the box is the card.
pub const GLOW_MIN: f32 = 36.0;
/// The halo: the kind's color at this opacity at full strength, fading to nothing this far out from the glow box, in points.
pub const GLOW_ALPHA: f32 = 0.85;
pub const GLOW_REACH: f32 = 32.0;
/// The core: `WHITE` at this share over the kind's color. At the block level it fills the card, grown to at least
/// [`GLOW_CORE_MIN`] points; from the icon level it is a ring [`GLOW_RING`] points wide around the card.
pub const GLOW_CORE_MIX: f32 = 0.75;
pub const GLOW_CORE_MIN: f32 = 8.0;
pub const GLOW_RING: f32 = 3.0;
/// The ripple: for [`RIPPLE_SECS`] after the event, a ring [`RIPPLE_W`] points wide that grows up to [`RIPPLE_GROW`] points
/// out from the glow box while it fades.
pub const RIPPLE_SECS: f64 = 0.9;
pub const RIPPLE_GROW: f32 = 48.0;
pub const RIPPLE_W: f32 = 2.0;
/// The edge marker of a glow whose card is outside the map region: a dot of this radius, this far inside the region's edge.
pub const EDGE_DOT: f32 = 7.0;
pub const EDGE_INSET: f32 = 14.0;

/// One running glow (unit `049`): a path, its newest event's color, and that event's time in egui seconds.
/// A note's glow (unit `053`) is in `FOCUS` and shows with layer 2; another glow shows with layer 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Glow {
    pub path: String,
    pub color: u32,
    pub note: bool,
    pub at: f64,
}

/// A noted card's mark at the block level (unit `053`): the card grown about its center to at least this many points,
/// in `FOCUS` at [`TRAIL_BLOCK_FRESH_MIX`] over the pane.
pub const NOTE_MIN: f32 = 8.0;
/// A noted card's fill from the icon level: `FOCUS` at this share over the card.
pub const NOTE_MIX: f32 = 0.30;
/// The note badge on a card's top-right corner: as wide as the name row, with its number at this world size.
pub const BADGE_NUMBER_PT: f32 = 12.0;

/// A glow's strength at an age in seconds: 1 for [`GLOW_HOLD`], then a smoothstep down to 0 over [`GLOW_FADE`].
pub fn glow_strength(age: f64) -> f32 {
    if age <= GLOW_HOLD {
        return 1.0;
    }
    let u = ((age - GLOW_HOLD) / GLOW_FADE).clamp(0.0, 1.0);
    (1.0 - u * u * (3.0 - 2.0 * u)) as f32
}

/// A trail kind's state color (`docs/design.md` `Color`).
pub fn trail_rgb(kind: TrailKind) -> u32 {
    match kind {
        TrailKind::Read => palette::READ,
        TrailKind::Grep => palette::GREP,
        TrailKind::Edit => palette::EDIT,
    }
}

/// What fits on a card at its screen width. Every card has one width, so one level holds for a whole frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Block,
    Icon,
    Name,
    Shape,
}

impl Level {
    pub fn for_width(w: f32) -> Level {
        if w < ICON_FROM {
            Level::Block
        } else if w < NAME_FROM {
            Level::Icon
        } else if w < SHAPE_FROM {
            Level::Name
        } else {
            Level::Shape
        }
    }
}

/// A file or a directory node: the selection, the hover, or a hit. The file tree's entry (unit `042`).
pub use minimap_core::filetree::Entry;

/// What the user wants to see: `1` trail (unit `047`), `2` notes (unit `053`), `7` names, `8` shape. The zoom decides what fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggles {
    pub trail: bool,
    pub notes: bool,
    pub names: bool,
    pub shape: bool,
}

impl Default for Toggles {
    fn default() -> Toggles {
        Toggles {
            trail: true,
            notes: true,
            names: true,
            shape: true,
        }
    }
}

/// Where the scene finds a file's details. `None` while they load.
pub trait Lookup {
    fn details(&self, path: &str) -> Option<&FileDetails>;
}

impl Lookup for Loader {
    fn details(&self, path: &str) -> Option<&FileDetails> {
        self.get(path)
    }
}

impl Lookup for Details {
    fn details(&self, path: &str) -> Option<&FileDetails> {
        self.get(path)
    }
}

impl<T: Lookup> Lookup for Option<T> {
    fn details(&self, path: &str) -> Option<&FileDetails> {
        self.as_ref().and_then(|t| t.details(path))
    }
}

impl Lookup for std::collections::HashMap<String, FileDetails> {
    fn details(&self, path: &str) -> Option<&FileDetails> {
        self.get(path)
    }
}

/// No details source: shapes show as loading.
pub struct NoDetails;

impl Lookup for NoDetails {
    fn details(&self, _: &str) -> Option<&FileDetails> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A card below [`ICON_FROM`]: a filled rect in the type color.
    Block,
    /// The card's fill.
    Card,
    /// The file-type icon, or a square without icons.
    Icon,
    /// The rule at the bottom of a header band.
    Rule,
    /// A directory name, in its block's header band.
    Header,
    /// The file name in the card's name row.
    Name,
    /// The line count, right-aligned in the name row.
    Count,
    /// The shape's bars.
    Shape,
    /// The shape's place while the file's details load.
    Loading,
    /// The dashed line at the bottom of a card that is cut short (unit `041`).
    Cut,
    /// The picked outline item's line, in the card's left padding (unit `046`).
    Mark,
    /// The selected file's ring.
    Ring,
    /// The selected directory's block.
    Selected,
    /// A glow's halo (unit `049`), drawn above the map.
    Glow,
    /// A glow's near-white core: a fill at the block level, else a ring around the card.
    GlowCore,
    /// The ring that grows out from a new glow.
    Ripple,
    /// The dot at the region's edge for a glow whose card is off screen.
    GlowEdge,
    /// A noted card's mark at the block level (unit `053`).
    NoteMark,
    /// A noted card's badge from the icon level, and its number.
    NoteBadge,
    NoteNumber,
}

/// What an item draws in its rect.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// The rect, filled.
    Fill,
    Glyph(char),
    /// No icon font, or `--no-icons`: a square in the type color.
    Square,
    Str(String),
    /// Filled rects, in screen points.
    Bars(Vec<Rect>),
    /// The rect's outline, this many points wide, outside the rect.
    Stroke(f32),
    /// A text too small to read (greeking): the rect, filled and rounded.
    Greek,
    /// A glow's halo (unit `049`): the color from `inner` fading out to nothing at the item's rect, and inside `inner` when `filled`.
    Halo {
        inner: Rect,
        filled: bool,
    },
    /// A filled circle in the rect.
    Dot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item<'a> {
    pub kind: Kind,
    /// The file's path, or the directory's path (`""` for the root).
    pub path: &'a str,
    /// The screen rect, in points.
    pub rect: Rect,
    pub level: Level,
    pub color: Color32,
    pub body: Body,
    /// The text or icon size on the screen, in points: its world size times the zoom. 0 for shapes.
    pub size: f32,
    /// The size its glyphs rasterize at: [`ladder`] of `size`. 0 for shapes and greeked text.
    pub glyph: f32,
    pub bold: bool,
    /// The corner radius, in points.
    pub radius: f32,
    /// From 0 to 1: a directory name fades as it grows past [`FADE_FROM`].
    pub opacity: f32,
}

impl<'a> Item<'a> {
    fn shape(kind: Kind, path: &'a str, rect: Rect, level: Level, color: Color32) -> Item<'a> {
        Item {
            kind,
            path,
            rect,
            level,
            color,
            body: Body::Fill,
            size: 0.0,
            glyph: 0.0,
            bold: false,
            radius: 0.0,
            opacity: 1.0,
        }
    }

    /// A text of `n` characters at a screen size, its line centered on `cy` from `x`:
    /// text from [`TEXT_FROM`], a greeked bar from [`GREEK_FROM`], else nothing.
    #[allow(clippy::too_many_arguments)]
    fn text(
        kind: Kind,
        path: &'a str,
        text: String,
        x: f32,
        cy: f32,
        size: f32,
        char_w: f32,
        level: Level,
        color: Color32,
    ) -> Option<Item<'a>> {
        let n = text.chars().count();
        let w = n as f32 * char_w * size;
        if n == 0 || size < GREEK_FROM {
            return None;
        }
        let Some(glyph) = ladder(size).filter(|_| size >= TEXT_FROM) else {
            let h = GREEK_H * size;
            return Some(Item {
                body: Body::Greek,
                size,
                radius: h / 2.0,
                ..Item::shape(
                    kind,
                    path,
                    Rect::from_min_size(pos2(x, cy - h / 2.0), vec2(w, h)),
                    level,
                    color.gamma_multiply(GREEK_ALPHA),
                )
            });
        };
        let line = LINE * size;
        Some(Item {
            body: Body::Str(text),
            size,
            glyph,
            ..Item::shape(
                kind,
                path,
                Rect::from_min_size(pos2(x, cy - line / 2.0), vec2(w, line)),
                level,
                color,
            )
        })
    }
}

/// The map: the index, its tree, and the `CARDS` layout, built once.
pub struct Map {
    pub index: Index,
    pub tree: Tree,
    pub layout: Layout,
    /// Per file: the type color.
    pub colors: Vec<u32>,
    /// Per file: the trail's kind and whether it is fresh (unit `047`). The app sets it when events arrive.
    pub tint: Vec<Option<(TrailKind, bool)>>,
    /// Per file: the ordinal of its first note (unit `053`). The app sets it when notes arrive.
    pub noted: Vec<Option<usize>>,
    /// Files and directory nodes by path, and each node's header.
    file_of: HashMap<String, usize>,
    node_of: HashMap<String, usize>,
    header_of: Vec<usize>,
}

impl Map {
    pub fn new(index: Index) -> Map {
        Map::with_config(index, &LayoutConfig::CARDS)
    }

    pub fn with_config(index: Index, cfg: &LayoutConfig) -> Map {
        let tree = Tree::from_index(&index);
        let layout = Layout::compute(&tree, cfg);
        let colors = tree
            .files
            .iter()
            .zip(&index.files)
            .map(|(t, f)| filetype::type_rgb(&t.name, f.binary))
            .collect();
        let file_of = tree
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.clone(), i))
            .collect();
        let node_of = tree
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.path.clone(), i))
            .collect();
        let mut header_of = vec![0; tree.nodes.len()];
        for (h, hd) in layout.headers.iter().enumerate() {
            header_of[hd.node] = h;
        }
        let tint = vec![None; tree.files.len()];
        let noted = vec![None; tree.files.len()];
        Map {
            index,
            tree,
            layout,
            colors,
            tint,
            noted,
            file_of,
            node_of,
            header_of,
        }
    }

    /// The file or directory at a repo-relative path. A directory may end with `/`.
    pub fn entry(&self, path: &str) -> Option<Entry> {
        if let Some(&f) = self.file_of.get(path) {
            return Some(Entry::File(f));
        }
        self.node_of
            .get(path.trim_end_matches('/'))
            .map(|&n| Entry::Dir(n))
    }

    /// A directory node's header index.
    pub fn header_of(&self, node: usize) -> usize {
        self.header_of[node]
    }

    /// An entry's world rect: the card, or the directory's block.
    pub fn rect_of(&self, e: Entry) -> Rect {
        match e {
            Entry::File(f) => self.card(f),
            Entry::Dir(n) => self.block(self.header_of(n)),
        }
    }

    /// The map's size in world points.
    pub fn world(&self) -> Vec2 {
        vec2(self.layout.w as f32, self.layout.h as f32) * CELL
    }

    /// The card width in world points: 168 for `CARDS`. Every card has this width.
    pub fn card_w(&self) -> f32 {
        self.layout.cfg.tile_size().0 as f32 * CELL.x
    }

    /// One line's height on a card in world points (`LayoutConfig::line_pt`).
    pub fn line_pt(&self) -> f32 {
        self.layout.cfg.line_pt as f32
    }

    /// A file's card in world points: the card width, and the height of its rounded line count (unit `039`).
    pub fn card(&self, f: usize) -> Rect {
        let (x, y) = self.layout.tiles[f];
        Rect::from_min_size(
            (vec2(x as f32, y as f32) * CELL).to_pos2(),
            vec2(self.card_w(), self.layout.tile_h[f] as f32 * CELL.y),
        )
    }

    /// A header's whole block in world points.
    pub fn block(&self, h: usize) -> Rect {
        let h = &self.layout.headers[h];
        Rect::from_min_size(
            (vec2(h.x as f32, h.y as f32) * CELL).to_pos2(),
            vec2(h.w as f32, h.h as f32) * CELL,
        )
    }

    /// A header's band in world points: the top of its block, which holds its name (unit `040`).
    pub fn band(&self, h: usize) -> Rect {
        let b = self.block(h);
        let rows = self.layout.headers[h].band;
        Rect::from_min_size(b.min, vec2(b.width(), rows as f32 * CELL.y))
    }

    /// A directory's name size in world points.
    pub fn name_size(&self, h: usize) -> f32 {
        self.layout.name_size[self.layout.headers[h].node] as f32
    }

    /// Ignored files keep empty slots.
    pub fn visible(&self, f: usize) -> bool {
        !self.tree.files[f].ignored
    }

    /// The header text: the repo name for the root, else the directory's own name (`src/quayside` for a chain), and `/`.
    pub fn header_name(&self, h: usize) -> String {
        let h = &self.layout.headers[h];
        let name = if h.depth == 0 {
            &self.tree.root_name
        } else {
            &self.tree.nodes[h.node].name
        };
        format!("{name}/")
    }
}

/// How the scene draws, apart from the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Nerd Font icons; off draws squares.
    pub icons: bool,
    /// A monospace glyph's advance as a share of the text size.
    pub char_w: f32,
    pub toggles: Toggles,
    pub selected: Option<Entry>,
    pub hovered: Option<Entry>,
    /// The picked outline item's file and 1-based line (unit `046`).
    pub mark: Option<(usize, u32)>,
    /// Screen pixels per point. A shape draws at most one sample per pixel row.
    pub pixels_per_point: f32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            icons: true,
            char_w: 0.6,
            toggles: Toggles::default(),
            selected: None,
            hovered: None,
            mark: None,
            pixels_per_point: 1.0,
        }
    }
}

/// The detail level of every card at a camera.
pub fn level(map: &Map, cam: &Camera) -> Level {
    Level::for_width(map.card_w() * cam.zoom)
}

/// The visible files whose cards intersect the map region, nearest to its center first.
pub fn visible_cards(map: &Map, cam: &Camera, region: Rect) -> Vec<usize> {
    let mut out: Vec<(f32, usize)> = (0..map.tree.files.len())
        .filter(|&f| map.visible(f))
        .filter_map(|f| {
            let r = cam.rect_to_screen(region, map.card(f));
            r.intersects(region)
                .then(|| (r.center().distance_sq(region.center()), f))
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    out.into_iter().map(|(_, f)| f).collect()
}

/// The draw items for a camera and a map region, in paint order: the selected block, cards, then headers.
pub fn scene<'a>(
    map: &'a Map,
    cam: &Camera,
    region: Rect,
    opts: &Options,
    details: &dyn Lookup,
) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let level = level(map, cam);
    if let Some(Entry::Dir(n)) = opts.selected {
        let block = cam.rect_to_screen(region, map.block(map.header_of(n)));
        let path = map.tree.nodes[n].path.as_str();
        items.push(Item::shape(
            Kind::Selected,
            path,
            block,
            level,
            rgb(palette::BLOCK_BG),
        ));
    }
    for f in 0..map.tree.files.len() {
        if !map.visible(f) {
            continue;
        }
        let rect = cam.rect_to_screen(region, map.card(f));
        if !rect.intersects(region) {
            continue;
        }
        card(
            &mut items, map, f, rect, cam.zoom, region, level, opts, details,
        );
    }
    for h in 0..map.layout.headers.len() {
        header(&mut items, map, h, cam, region, level, opts);
    }
    items.retain(|i| i.rect.intersects(region));
    items
}

#[allow(clippy::too_many_arguments)]
fn card<'a>(
    items: &mut Vec<Item<'a>>,
    map: &'a Map,
    f: usize,
    rect: Rect,
    zoom: f32,
    region: Rect,
    level: Level,
    opts: &Options,
    details: &dyn Lookup,
) {
    let path = map.tree.files[f].path.as_str();
    let type_rgb = map.colors[f];
    let selected = opts.selected == Some(Entry::File(f));
    let ring = |radius: f32| Item {
        body: Body::Stroke(RING),
        radius,
        ..Item::shape(Kind::Ring, path, rect, level, rgb(palette::SEL_HI))
    };
    let tint = map.tint[f].filter(|_| opts.toggles.trail);
    if level == Level::Block {
        let color = match tint {
            Some((k, fresh)) => mix(
                trail_rgb(k),
                palette::PANE,
                if fresh {
                    TRAIL_BLOCK_FRESH_MIX
                } else {
                    TRAIL_BLOCK_OLD_MIX
                },
            ),
            None => mix(type_rgb, palette::PANE, BLOCK_MIX),
        };
        items.push(Item::shape(Kind::Block, path, rect, level, color));
        if selected {
            items.push(ring(0.0));
        }
        return;
    }
    let w = rect.width();
    // The hover wins over a note (unit `053`), and a note over the trail, as in the TUI's priority.
    let noted = map.noted[f].is_some() && opts.toggles.notes;
    let fill = match tint {
        _ if opts.hovered == Some(Entry::File(f)) => rgb(palette::TILE_HI),
        _ if noted => mix(palette::FOCUS, palette::TILE, NOTE_MIX),
        Some((k, fresh)) => mix(
            trail_rgb(k),
            palette::TILE,
            if fresh {
                TRAIL_FRESH_MIX
            } else {
                TRAIL_OLD_MIX
            },
        ),
        None => rgb(palette::TILE),
    };
    items.push(Item {
        radius: RADIUS * w,
        ..Item::shape(Kind::Card, path, rect, level, fill)
    });
    if selected {
        items.push(ring(RADIUS * w));
    }
    let g = CardGeometry::new(rect);
    let icon_rgb = if selected { palette::WHITE } else { type_rgb };
    // An icon too small to read is a square in its type color, as without icons.
    if g.icon_size >= GREEK_FROM {
        let glyph = ladder(g.icon_size).filter(|_| opts.icons && g.icon_size >= TEXT_FROM);
        let body = match glyph {
            Some(_) => Body::Glyph(filetype::for_name(&map.tree.files[f].name).glyph),
            None => Body::Square,
        };
        items.push(Item {
            body,
            size: g.icon_size,
            glyph: glyph.unwrap_or(0.0),
            ..Item::shape(Kind::Icon, path, g.icon, level, rgb(icon_rgb))
        });
    }
    let file = &map.index.files[f];
    let line_h = map.line_pt() * zoom;
    let shown = shown_lines(&g, line_h, zoom, file.lines as usize);
    text_and_shape(
        items, map, f, &g, line_h, shown.0, region, level, opts, details,
    );
    if shown.1 {
        items.push(cut_mark(path, &g, zoom, level));
    }
    if let Some((_, line)) = opts.mark.filter(|&(m, _)| m == f)
        && level >= Level::Shape
        && opts.toggles.shape
    {
        let l = shown_line(line, shown.0.min(file.lines as usize));
        let cy = g.body.top() + (l as f32 + 0.5) * line_h;
        let h = line_h.max(MARK_MIN);
        let r = Rect::from_min_max(
            pos2(rect.left() + g.pad / 4.0, cy - h / 2.0),
            pos2(g.body.left() - g.pad / 4.0, cy + h / 2.0),
        );
        items.push(Item::shape(Kind::Mark, path, r, level, rgb(palette::WHITE)));
    }
}

/// A 1-based line as a 0-based line that a card shows: a line past the shown lines is the last shown line (unit `046`).
fn shown_line(line: u32, shown: usize) -> usize {
    (line.max(1) as usize - 1).min(shown.saturating_sub(1))
}

/// The world y of a line's top on a file's card, as the shape draws it: a line past a capped card's shown lines is its last shown line.
pub fn line_top(map: &Map, f: usize, line: u32) -> f32 {
    let g = CardGeometry::new(map.card(f));
    let lines = map.index.files[f].lines as usize;
    let shown = shown_lines(&g, map.line_pt(), 1.0, lines).0.min(lines);
    g.body.top() + shown_line(line, shown) as f32 * map.line_pt()
}

/// The lines that a card's shape shows, and whether the card is cut short (unit `041`): every line when the body holds them,
/// else the lines above the cut mark's room. Without line-scaled cards, every line, and no cut.
pub fn shown_lines(g: &CardGeometry, line_h: f32, zoom: f32, lines: usize) -> (usize, bool) {
    if line_h <= 0.0 {
        return (lines, false);
    }
    let fit = |h: f32| (h / line_h + 1e-3).floor().max(0.0) as usize;
    if lines <= fit(g.body.height()) {
        return (fit(g.body.height()), false);
    }
    let room = (CUT_ROOM * zoom).max(cut_h(zoom));
    (fit(g.body.height() - room), true)
}

/// The cut mark's height on the screen.
fn cut_h(zoom: f32) -> f32 {
    (CUT_PT * zoom).max(1.0)
}

/// The cut mark: [`CUT_DASHES`] dashes in `DIM` along the body's bottom, with gaps as wide as the dashes.
fn cut_mark<'a>(path: &'a str, g: &CardGeometry, zoom: f32, level: Level) -> Item<'a> {
    let h = cut_h(zoom);
    let rect = Rect::from_min_max(
        pos2(g.body.left(), g.body.bottom() - h),
        g.body.right_bottom(),
    );
    let dash = rect.width() / (2 * CUT_DASHES - 1) as f32;
    let bars = (0..CUT_DASHES)
        .map(|k| {
            Rect::from_min_size(
                pos2(rect.left() + 2.0 * k as f32 * dash, rect.top()),
                vec2(dash, h),
            )
        })
        .collect();
    Item {
        body: Body::Bars(bars),
        ..Item::shape(Kind::Cut, path, rect, level, rgb(palette::DIM))
    }
}

/// The name, the line count, and the shape of a card, by its level and the toggles. The shape shows at most `shown` lines.
#[allow(clippy::too_many_arguments)]
fn text_and_shape<'a>(
    items: &mut Vec<Item<'a>>,
    map: &'a Map,
    f: usize,
    g: &CardGeometry,
    line_h: f32,
    shown: usize,
    region: Rect,
    level: Level,
    opts: &Options,
    details: &dyn Lookup,
) {
    if level < Level::Name {
        return;
    }
    let path = map.tree.files[f].path.as_str();
    let type_rgb = map.colors[f];
    let t = opts.toggles;
    let file = &map.index.files[f];
    let advance = opts.char_w * g.name_size;
    let cy = g.name_row.center().y;
    let mut name_right = g.name_row.right();
    if level >= Level::Shape && t.shape {
        let count = file.lines.to_string();
        let cw = count.chars().count() as f32 * advance;
        name_right = g.name_row.right() - cw - advance;
        items.extend(Item::text(
            Kind::Count,
            path,
            count,
            g.name_row.right() - cw,
            cy,
            g.name_size,
            opts.char_w,
            level,
            rgb(palette::DIM),
        ));
    }
    if t.names {
        let name = &map.tree.files[f].name;
        let room = ((name_right - g.name_x) / advance + 1e-3).floor().max(0.0) as usize;
        items.extend(Item::text(
            Kind::Name,
            path,
            fit(name, room),
            g.name_x,
            cy,
            g.name_size,
            opts.char_w,
            level,
            rgb(palette::INK),
        ));
    }
    if level < Level::Shape || !t.shape {
        return;
    }
    shape(
        items,
        path,
        g,
        line_h,
        shown,
        region,
        type_rgb,
        level,
        file,
        details.details(path),
        opts.pixels_per_point,
    );
}

/// Lines per shape sample at a line height on the screen: at most one sample per pixel row.
pub fn lines_per_sample(line_h: f32, pixels_per_point: f32) -> usize {
    let px = line_h * pixels_per_point.max(1e-3);
    if px >= 1.0 {
        1
    } else {
        (1.0 / px).ceil() as usize
    }
}

/// The shape: one bar per line, or per run of lines when a line is under a pixel row on the screen (unit `039`).
/// Only the rows in the map region give bars, and only the first `shown` lines (unit `041`).
#[allow(clippy::too_many_arguments)]
fn shape<'a>(
    items: &mut Vec<Item<'a>>,
    path: &'a str,
    g: &CardGeometry,
    line_h: f32,
    shown: usize,
    region: Rect,
    type_rgb: u32,
    level: Level,
    file: &minimap_core::IndexFile,
    d: Option<&FileDetails>,
    pixels_per_point: f32,
) {
    let top = g.body.top();
    let Some(d) = d else {
        // Loading: a rect as tall as the shape will be, from the index's line count.
        if !file.binary && !file.missing && file.lines > 0 {
            let lines = (file.lines as usize).min(shown);
            let r = Rect::from_min_size(g.body.min, vec2(g.body.width(), lines as f32 * line_h));
            items.push(Item::shape(
                Kind::Loading,
                path,
                r,
                level,
                mix(palette::FAINT, palette::TILE, LOADING_MIX),
            ));
        }
        return;
    };
    // A binary or empty file has an empty body.
    let Some(s) = d
        .shape
        .as_ref()
        .filter(|s| s.lines > 0 && !s.rows.is_empty())
    else {
        return;
    };
    let n = s.lines as usize;
    // The lines that draw: a capped card shows its first lines at the normal scale.
    let limit = n.min(shown);
    let per = g.body.width() / SHAPE_CHARS;
    let bar = |row: ShapeRow, a: usize, b: usize| -> Option<Rect> {
        let x0 = f32::from(row.indent).min(SHAPE_CHARS);
        let x1 = f32::from(row.len.min(MAX_LEN)).min(SHAPE_CHARS);
        let h = (b - a) as f32 * line_h;
        let gap = if h >= BAR_GAP_FROM { 1.0 } else { 0.0 };
        (x1 > x0).then(|| {
            Rect::from_min_size(
                pos2(g.body.left() + x0 * per, top + a as f32 * line_h),
                vec2((x1 - x0) * per, h - gap),
            )
        })
    };
    // The lines in the region, rounded out to whole samples, so the samples do not shift while the map pans.
    let first = ((region.top() - top) / line_h).floor().max(0.0) as usize;
    let last = (((region.bottom() - top) / line_h).ceil().max(0.0) as usize).min(limit);
    let mut bars = Vec::new();
    if d.measured.len() == n {
        let k = lines_per_sample(line_h, pixels_per_point);
        let mut a = first / k * k;
        while a < last {
            let b = (a + k).min(limit);
            let row = match &d.measured[a..b] {
                [one] => *one,
                run if run.len() <= MEDIAN_ABOVE => {
                    let text = run.iter().filter(|l| l.len > 0);
                    ShapeRow {
                        indent: text.clone().map(|l| l.indent).min().unwrap_or(0),
                        len: text.map(|l| l.len).max().unwrap_or(0),
                    }
                }
                run => summarize(run, MEDIAN_ABOVE),
            };
            bars.extend(bar(row, a, b));
            a = b;
        }
    } else {
        // Only the sampled shape (`--from-json`): each row draws as many lines high as it covers.
        let rows = s.rows.len();
        for (r, &row) in s.rows.iter().enumerate() {
            let (a, b) = (r * n / rows, ((r + 1) * n / rows).max(r * n / rows + 1));
            if b > first && a < last {
                bars.extend(bar(row, a, b.min(limit)));
            }
        }
    }
    let rect = Rect::from_min_size(g.body.min, vec2(g.body.width(), limit as f32 * line_h));
    items.push(Item {
        body: Body::Bars(bars),
        ..Item::shape(
            Kind::Shape,
            path,
            rect,
            level,
            mix(type_rgb, palette::TILE, SHAPE_MIX),
        )
    });
}

/// The places inside a card, as shares of its width, so a zoom or a toggle never moves an element inside it,
/// and the name row is the same on every card. Every size is in proportion to the card width (unit `040`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardGeometry {
    pub pad: f32,
    /// The name row: the card's first layout row, with the icon, the name, and the line count.
    pub name_row: Rect,
    /// The icon's square, at the left of the name row.
    pub icon: Rect,
    pub icon_size: f32,
    /// Where the name starts, after the icon.
    pub name_x: f32,
    pub name_size: f32,
    /// Below the name row, to the card's bottom, inside the side padding. The shape starts at its top.
    pub body: Rect,
}

impl CardGeometry {
    pub fn new(card: Rect) -> CardGeometry {
        let w = card.width();
        // Screen points per world point: a card is 168 world points wide.
        let k = w / (CARD_CELLS * CELL.x);
        let pad = PAD * w;
        let row = NAME_ROW_PT * k;
        let icon_size = ICON_PT * k;
        let name_row = Rect::from_min_size(card.min + vec2(pad, 0.0), vec2(w - 2.0 * pad, row));
        let icon = Rect::from_center_size(
            pos2(name_row.left() + icon_size / 2.0, name_row.center().y),
            Vec2::splat(icon_size),
        );
        let body = Rect::from_min_max(
            pos2(card.left() + pad, card.top() + row),
            pos2(card.right() - pad, card.bottom().max(card.top() + row)),
        );
        CardGeometry {
            pad,
            name_row,
            icon,
            icon_size,
            name_x: icon.right(),
            name_size: NAME_PT * k,
            body,
        }
    }
}

/// A card's width in layout cells: `CARDS` `tile_size().0`.
pub const CARD_CELLS: f32 = 21.0;

/// A header's name in its band, and the 1-point rule at the band's bottom (unit `040`).
/// The name's world size comes from the layout, and a name past [`FADE_FROM`] on the screen fades.
fn header<'a>(
    items: &mut Vec<Item<'a>>,
    map: &'a Map,
    h: usize,
    cam: &Camera,
    region: Rect,
    level: Level,
    opts: &Options,
) {
    let hd = &map.layout.headers[h];
    let block = cam.rect_to_screen(region, map.block(h));
    if !block.intersects(region) {
        return;
    }
    let path = map.tree.nodes[hd.node].path.as_str();
    let band = cam.rect_to_screen(region, map.band(h));
    let (fg, bold) = match hd.depth {
        _ if opts.selected == Some(Entry::Dir(hd.node)) => (palette::WHITE, hd.depth <= 2),
        0 => (palette::FOCUS, true),
        1 | 2 => (palette::INK, true),
        _ => (palette::INK2, false),
    };
    let size = map.name_size(h) * cam.zoom;
    let opacity = fade(size);
    if opacity > 0.0 && band.intersects(region) {
        let room = (band.width() / (opts.char_w * size) + 1e-3)
            .floor()
            .max(0.0) as usize;
        if let Some(item) = Item::text(
            Kind::Header,
            path,
            fit(&map.header_name(h), room),
            band.left(),
            band.center().y,
            size,
            opts.char_w,
            level,
            rgb(fg),
        ) {
            let item = Item {
                bold,
                opacity,
                ..item
            };
            items.push(item);
        }
    }
    // The root has no rule, as in the TUI.
    if hd.depth == 0 {
        return;
    }
    let cell = CELL.x * cam.zoom;
    let (from, to) = (band.left(), band.right() - cell);
    if to > from {
        let y = band.bottom() - 0.5;
        let rect = Rect::from_min_max(pos2(from, y - 0.5), pos2(to, y + 0.5));
        items.push(Item::shape(
            Kind::Rule,
            path,
            rect,
            level,
            rgb(palette::RULE),
        ));
    }
}

/// A rect grown about its center to at least `min` points in each direction.
fn grow_to(r: Rect, min: f32) -> Rect {
    Rect::from_center_size(r.center(), r.size().max(Vec2::splat(min)))
}

/// The glows' draw items (unit `049`), to draw after the cards and headers: for each running glow whose file has a card,
/// the halo, the core, and the ripple while it grows; for a card outside the map region, a dot at the region's edge.
/// A glow never moves a card, and [`hit`] ignores it.
pub fn glows<'a>(
    map: &'a Map,
    cam: &Camera,
    region: Rect,
    glows: &[Glow],
    now: f64,
) -> Vec<Item<'a>> {
    let level = level(map, cam);
    let mut items = Vec::new();
    for g in glows {
        let age = (now - g.at).max(0.0);
        let s = glow_strength(age);
        if s <= 0.0 {
            continue;
        }
        let Some(Entry::File(f)) = map.entry(&g.path) else {
            continue;
        };
        if !map.visible(f) {
            continue;
        }
        let path = map.tree.files[f].path.as_str();
        let kind_rgb = g.color;
        let card = cam.rect_to_screen(region, map.card(f));
        if !card.intersects(region) {
            // The edge marker: on the line from the region's center to the card's center, at the inset edge.
            let c = region.center();
            let d = card.center() - c;
            let half = region.shrink(EDGE_INSET).size() / 2.0;
            let t = (half.x / d.x.abs().max(1e-3)).min(half.y / d.y.abs().max(1e-3));
            let at = c + d * t;
            for (r, color) in [
                (EDGE_DOT, alpha(kind_rgb, s)),
                (EDGE_DOT * 0.4, alpha(palette::WHITE, s)),
            ] {
                items.push(Item {
                    body: Body::Dot,
                    ..Item::shape(
                        Kind::GlowEdge,
                        path,
                        Rect::from_center_size(at, Vec2::splat(2.0 * r)),
                        level,
                        color,
                    )
                });
            }
            continue;
        }
        let block = level == Level::Block;
        let inner = if block { grow_to(card, GLOW_MIN) } else { card };
        items.push(Item {
            body: Body::Halo {
                inner,
                filled: block,
            },
            ..Item::shape(
                Kind::Glow,
                path,
                inner.expand(GLOW_REACH),
                level,
                alpha(kind_rgb, GLOW_ALPHA * s),
            )
        });
        let core = with_alpha(mix(palette::WHITE, kind_rgb, GLOW_CORE_MIX), s);
        if block {
            items.push(Item::shape(
                Kind::GlowCore,
                path,
                grow_to(card, GLOW_CORE_MIN),
                level,
                core,
            ));
        } else {
            items.push(Item {
                body: Body::Stroke(GLOW_RING),
                radius: RADIUS * card.width(),
                ..Item::shape(Kind::GlowCore, path, card, level, core)
            });
        }
        if age < RIPPLE_SECS {
            let u = (age / RIPPLE_SECS) as f32;
            let grow = RIPPLE_GROW * (1.0 - (1.0 - u) * (1.0 - u));
            items.push(Item {
                body: Body::Stroke(RIPPLE_W),
                ..Item::shape(
                    Kind::Ripple,
                    path,
                    inner.expand(grow),
                    level,
                    alpha(kind_rgb, 1.0 - u),
                )
            });
        }
    }
    items.retain(|i| i.rect.intersects(region));
    items
}

/// The note marks' draw items (unit `053`), to draw after the cards and headers and before the glows.
/// At the block level, a noted card's mark is the card grown to at least [`NOTE_MIN`] points, so it shows at the fit view.
/// From the icon level, a badge sits centered on the card's top-right corner: a `FOCUS` circle as wide as the name row
/// (a pill for a longer number), with the ordinal of the file's first note, as text in the world.
pub fn note_marks<'a>(map: &'a Map, cam: &Camera, region: Rect, opts: &Options) -> Vec<Item<'a>> {
    let level = level(map, cam);
    let mut items = Vec::new();
    for (f, ordinal) in map.noted.iter().enumerate() {
        let Some(ordinal) = *ordinal else {
            continue;
        };
        if !map.visible(f) {
            continue;
        }
        let path = map.tree.files[f].path.as_str();
        let card = cam.rect_to_screen(region, map.card(f));
        if level == Level::Block {
            items.push(Item::shape(
                Kind::NoteMark,
                path,
                grow_to(card, NOTE_MIN),
                level,
                mix(palette::FOCUS, palette::PANE, TRAIL_BLOCK_FRESH_MIX),
            ));
            continue;
        }
        let g = CardGeometry::new(card);
        let d = g.name_row.height();
        let size = BADGE_NUMBER_PT * d / NAME_ROW_PT;
        let number = ordinal.to_string();
        let text_w = number.chars().count() as f32 * opts.char_w * size;
        let w = d.max(text_w + 0.4 * d);
        let badge = Rect::from_center_size(card.right_top(), vec2(w, d));
        items.push(Item {
            radius: d / 2.0,
            ..Item::shape(Kind::NoteBadge, path, badge, level, rgb(palette::FOCUS))
        });
        if let Some(item) = Item::text(
            Kind::NoteNumber,
            path,
            number,
            badge.center().x - text_w / 2.0,
            badge.center().y,
            size,
            opts.char_w,
            level,
            rgb(palette::BAR),
        ) {
            items.push(Item { bold: true, ..item });
        }
    }
    items.retain(|i| i.rect.intersects(region));
    items
}

/// The file or directory under a screen point: from the drawn items, topmost first, then from the header bands.
/// A card and anything on it hit its file; a header name, its rule, or anywhere in its band hits its directory (unit `040`).
/// Gaps between cards, and the selected directory's block fill, hit nothing.
pub fn hit(map: &Map, items: &[Item], p: Pos2, cam: &Camera, region: Rect) -> Option<Entry> {
    let on_item = items.iter().rev().find_map(|i| {
        // The selected block's fill, the glows (unit `049`), and the note marks (unit `053`) hit nothing.
        let passive = matches!(
            i.kind,
            Kind::Selected
                | Kind::Glow
                | Kind::GlowCore
                | Kind::Ripple
                | Kind::GlowEdge
                | Kind::NoteMark
                | Kind::NoteBadge
                | Kind::NoteNumber
        );
        if passive || !i.rect.contains(p) {
            return None;
        }
        match i.kind {
            Kind::Header | Kind::Rule => map.node_of.get(i.path).map(|&n| Entry::Dir(n)),
            _ => map.file_of.get(i.path).map(|&f| Entry::File(f)),
        }
    });
    on_item.or_else(|| {
        // Bands of nested blocks never overlap: a child block lies below its parent's band.
        let at = cam.to_world(region, p);
        (0..map.layout.headers.len())
            .filter(|&h| map.band(h).contains(at))
            .max_by_key(|&h| map.layout.headers[h].depth)
            .map(|h| Entry::Dir(map.layout.headers[h].node))
    })
}
