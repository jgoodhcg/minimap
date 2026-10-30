//! The "Districts" layout: directory blocks packed into shelves of stacked columns.
//!
//! This is a port of `size`, `plan`, and `place` in `mockups/v3-template.html:101-168`,
//! with the constants as data, as `makeLayout` in `mockups/v5-template.html` has them.
//! It keeps the JavaScript operation order and uses `f64` for the area math, so the results match the mockups exactly.
//! The layout is a function of the file tree and a [`LayoutConfig`] only. It takes no selection, layer, or view input.
//! `CHARS` and `TILES` give every tile one size. `CARDS` makes each card as tall as its file's rounded line count (unit `039`),
//! so for `CARDS` the tree's line counts are an input too. `CARDS` also sizes each directory name by the count of files below it
//! (unit `062`), and gives the block a header band that holds the name (unit `040`). It caps a generated file's card at the height of
//! [`GEN_LINES`] lines (unit `041`), so for `CARDS` the tree's generated flags are an input too.

use std::collections::HashMap;

use crate::index::Index;

/// How block sizes and file-grid capacities round up (unit `027`), so small growth moves no tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quant {
    /// No rounding: the v3 and v5 mockups' packing.
    None,
    /// Round up to the next multiple of `n`.
    Step(usize),
    /// Round up to the next value of `1, 2, 3, …`, where each value is `max(prev + 1, ceil(prev * r))`.
    Growth(f64),
}

impl Quant {
    /// Rounds a count or a size up. 0 stays 0.
    pub fn round(self, x: usize) -> usize {
        if x == 0 {
            return 0;
        }
        match self {
            Quant::None => x,
            Quant::Step(n) => x.div_ceil(n.max(1)) * n.max(1),
            Quant::Growth(r) => {
                let mut v = 1;
                while v < x {
                    v = (v + 1).max((v as f64 * r).ceil() as usize);
                }
                v
            }
        }
    }

    fn round_i32(self, x: i32) -> i32 {
        self.round(x.max(0) as usize) as i32
    }
}

/// The order of child blocks before they pack into shelves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Tallest first, then by name (`mockups/v3-template.html:119-143`).
    Tallest,
    /// By name only, so a height change never reorders siblings.
    Name,
}

/// What sets a directory's name size and its block's header band ([`LayoutConfig::name_band`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Names {
    /// Every block has [`LayoutConfig::head`] header rows, and names have no size.
    None,
    /// The count of files below the directory, at every depth (unit `062`), so a parent's name is never smaller than its child's.
    Count,
    /// The share of the block's width that the name spans (unit `040`), so a short name draws larger.
    /// The stability report and the move tests compare with it.
    Width(f64),
}

/// The layout constants (`docs/design.md` `Layout`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConfig {
    /// Tile pitch in columns.
    pub tw: i32,
    /// Tile pitch in rows.
    pub th: i32,
    /// Indent of a child block's content (depth > 0).
    pub ind: i32,
    /// Gap between columns in a shelf.
    pub gx: i32,
    /// Gap between shelves, and between a file grid and its shelves.
    pub gy: i32,
    /// Gap between blocks stacked in one column.
    pub vg: i32,
    /// Header rows per block.
    pub head: i32,
    /// Cell aspect, used only for the packing area math.
    pub px: i64,
    pub py: i64,
    /// File columns start at `ceil(sqrt(nf * fc_k))` for a directory with `nf` files.
    pub fc_k: f64,
    /// With a value above 0, a block is at least `min(name chars + 1, label_w)` wide,
    /// and its files spread across that width.
    pub label_w: i32,
    /// Root packing aspect: the target width is `sqrt(area * root_k)`. Below the root it is 1.5.
    pub root_k: f64,
    /// Rounding of file-grid capacities and of the sizes of blocks below the root.
    pub quant: Quant,
    /// The order of child blocks in a shelf packing.
    pub order: Order,
    /// What [`LayoutConfig::quant`] rounds: file-grid capacities, block widths, and block heights.
    pub round: Round,
    /// Line-scaled tiles (unit `039`): one line is this many points high at zoom 1, and a tile is a name row
    /// and a body of `ceil(quant(lines) * line_pt / py)` rows. Files flow down columns. 0 gives every tile one size.
    pub line_pt: f64,
    /// Name bands: what sets a directory's name size, in points at zoom 1, and its block's header band that holds the name.
    pub names: Names,
    /// Generated files (unit `041`): a generated file's line-scaled tile counts at most this many lines. 0 gives no cap.
    pub gen_lines: u64,
}

/// The quantities that [`LayoutConfig::quant`] rounds up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Round {
    /// The file-grid capacity of every directory.
    pub cap: bool,
    /// The width and height of blocks below the root, at depth `max_depth` or less (0: every depth).
    pub w: bool,
    pub h: bool,
    pub max_depth: usize,
}

impl Round {
    /// Everything, at every depth.
    pub const ALL: Round = Round {
        cap: true,
        w: true,
        h: true,
        max_depth: 0,
    };
}

impl LayoutConfig {
    /// The v3 tile map: 21 x 2 tiles with names (`mockups/v3-template.html`).
    pub const TILES: LayoutConfig = LayoutConfig {
        tw: 22,
        th: 3,
        ind: 2,
        gx: 3,
        gy: 1,
        vg: 1,
        head: 1,
        px: 8,
        py: 18,
        fc_k: 0.7,
        label_w: 0,
        root_k: 3.2,
        quant: Quant::None,
        order: Order::Tallest,
        round: Round::ALL,
        line_pt: 0.0,
        names: Names::None,
        gen_lines: 0,
    };

    /// The v5 map: one character per file, and names only for directories (`mockups/v5-template.html` `CHARS`),
    /// with file-grid capacities and the heights of blocks at depth 1 and 2 rounded up (unit `027`).
    pub const CHARS: LayoutConfig = LayoutConfig {
        tw: 2,
        th: 1,
        ind: 2,
        gx: 2,
        gy: 1,
        vg: 1,
        head: 1,
        px: 8,
        py: 18,
        fc_k: 1.1,
        label_w: 28,
        root_k: 1.0,
        // Unit 027: with this rounding, small growth moves fewer tiles, for some more area.
        quant: Quant::Growth(1.25),
        order: Order::Tallest,
        round: Round {
            cap: true,
            w: false,
            h: true,
            max_depth: 2,
        },
        line_pt: 0.0,
        names: Names::None,
        gen_lines: 0,
    };

    /// The GUI's card map (unit `035`): the `TILES` pitch, and the `CHARS` rounding.
    /// A card is 21 cells wide. Since unit `039` it is a name row and a body as tall as its file's lines,
    /// with the line count rounded up to a step; `th` 7 keeps a gap of one row between cards.
    /// Names come from the cards and the headers, so blocks keep the `CHARS` label width.
    /// `root_k` 3.2 puts every district in one shelf, so the map is wider than tall and fits a window better:
    /// 1.6 made a portrait map with narrower cards at the fit view.
    pub const CARDS: LayoutConfig = LayoutConfig {
        tw: 22,
        th: 7,
        ind: 2,
        gx: 3,
        gy: 1,
        vg: 1,
        head: 1,
        px: 8,
        py: 18,
        fc_k: 0.7,
        label_w: 28,
        root_k: 3.2,
        quant: Quant::Growth(1.25),
        order: Order::Tallest,
        round: Round {
            cap: true,
            w: false,
            h: true,
            max_depth: 2,
        },
        line_pt: LINE_PT,
        names: Names::Count,
        gen_lines: GEN_LINES,
    };

    /// The same constants without rounding and with tallest-first order: the packing of the mockups.
    pub fn unrounded(&self) -> LayoutConfig {
        LayoutConfig {
            quant: Quant::None,
            order: Order::Tallest,
            ..*self
        }
    }

    /// The drawn tile size in cells: the pitch minus the gap, and at least one row.
    /// It is 21 x 2 for [`TILES`](Self::TILES), 1 x 1 for [`CHARS`](Self::CHARS), and 21 x 6 for [`CARDS`](Self::CARDS).
    /// Line-scaled cards keep the width; [`LayoutConfig::tile_rows`] gives their heights.
    pub fn tile_size(&self) -> (i32, i32) {
        (self.tw - 1, (self.th - 1).max(1))
    }

    /// Tiles have heights from their line counts ([`LayoutConfig::line_pt`] above 0).
    pub fn line_scaled(&self) -> bool {
        self.line_pt > 0.0
    }

    /// The body rows of a line-scaled tile: `ceil(quant(lines) * line_pt / py)`. 0 for a file with 0 lines.
    pub fn body_rows(&self, lines: u64) -> i32 {
        let step = self.quant.round(lines as usize) as f64;
        (step * self.line_pt / self.py as f64).ceil() as i32
    }

    /// The lines that a file's line-scaled tile counts: its line count, at most [`LayoutConfig::gen_lines`] for a generated file.
    pub fn tile_lines(&self, lines: u64, generated: bool) -> u64 {
        if generated && self.gen_lines > 0 {
            lines.min(self.gen_lines)
        } else {
            lines
        }
    }

    /// The drawn height of a file's tile in cells: the name row and the body for line-scaled tiles,
    /// with a generated file's lines capped ([`LayoutConfig::tile_lines`]), else [`LayoutConfig::tile_size`]'s height.
    pub fn tile_rows(&self, lines: u64, generated: bool) -> i32 {
        if self.line_scaled() {
            NAME_ROWS + self.body_rows(self.tile_lines(lines, generated))
        } else {
            self.tile_size().1
        }
    }

    /// The header band of a block `w` cells wide whose header text (with its `/`) has `chars` characters,
    /// with `files` files below it at every depth: the name size in points at zoom 1, and the band's rows.
    /// Without name bands, the size is 0 and the band is `head`. The least size is [`NAME_MIN`] of the header row.
    ///
    /// - [`Names::Count`] (unit `062`): the least size times the square root of `files`, which counts at most [`NAME_CAP_FILES`].
    ///   The band is `ceil(sqrt(files))` header rows, so it holds `BAND_K` times the size.
    /// - [`Names::Width`] (unit `040`): the largest size at which the name spans the share of the block's width, at least the least size.
    ///   The band is `ceil(BAND_K * size / py)` rows.
    ///
    /// The band is at least `head` rows and rounds up by `quant`.
    pub fn name_band(&self, w: i32, chars: usize, files: usize) -> (f64, i32) {
        let least = NAME_MIN * f64::from(self.head) * self.py as f64;
        let (size, rows) = match self.names {
            Names::None => return (0.0, self.head),
            Names::Count => {
                // `BAND_K * NAME_MIN` is 1. Integer math keeps the band exact when the count is a square.
                let n = files.clamp(1, NAME_CAP_FILES);
                let head = self.head.max(1) as usize;
                (least * (n as f64).sqrt(), ceil_sqrt(n * head * head) as i32)
            }
            Names::Width(share) => {
                let width = f64::from(w) * self.px as f64;
                let fit = share * width / (chars.max(1) as f64 * NAME_ADVANCE);
                let size = fit.max(least);
                (size, (BAND_K * size / self.py as f64).ceil() as i32)
            }
        };
        (size, self.quant.round_i32(rows.max(self.head)))
    }

    /// The rows between two tiles in a column: the pitch minus the drawn height of one-size tiles.
    fn row_gap(&self) -> i32 {
        self.th - self.tile_size().1
    }

    /// The minimum block width for a directory name: `min(name chars + 1, label_w)`, or 0 without labels.
    fn label(&self, name: &str) -> i32 {
        if self.label_w > 0 {
            (name.chars().count() as i32 + 1).min(self.label_w)
        } else {
            0
        }
    }
}

/// One line is this many points high at zoom 1 on line-scaled cards (unit `039`).
pub const LINE_PT: f64 = 1.0;
/// A line-scaled tile's name row, in cells.
pub const NAME_ROWS: i32 = 1;
/// A directory name spans this share of its block's width with [`Names::Width`] (unit `040`). `CARDS` used it until unit `062`.
pub const NAME_SHARE: f64 = 0.30;
/// A monospace glyph's advance as a share of its size, for the name sizes. FiraCode's is 0.6, as egui's default monospace is.
pub const NAME_ADVANCE: f64 = 0.6;
/// The least name size, as a share of the header row: 14.4 points for a row of 18.
pub const NAME_MIN: f64 = 0.8;
/// A header band is this many times its name size, before it rounds up to whole rows and a step.
pub const BAND_K: f64 = 1.25;
/// A directory name stops growing at this many files below it (unit `062`): 1,440 points at zoom 1, 100 times the least size.
pub const NAME_CAP_FILES: usize = 10_000;
/// A generated file's card on `CARDS` is as tall as a card of this many lines (unit `041`): 363 lines after the rounding, 21 body rows.
pub const GEN_LINES: u64 = 300;

/// One file in the tree.
#[derive(Debug, Clone)]
pub struct TreeFile {
    pub path: String,
    /// The part after the last `/`.
    pub name: String,
    pub ignored: bool,
    /// The line count from the index: 0 for binary and missing files. Only line-scaled layouts read it.
    pub lines: u64,
    /// A tool writes the file (unit `041`, [`crate::generated`]). Line-scaled layouts cap its height.
    pub generated: bool,
    /// A path segment starts with `.` (see [`is_hidden`]).
    pub hidden: bool,
    /// The node that holds the file.
    pub node: usize,
}

/// One directory block.
#[derive(Debug, Clone)]
pub struct Node {
    /// The repo-relative directory path. The root is `""`.
    pub path: String,
    /// The header name. A compacted chain has a name such as `src/quayside`.
    pub name: String,
    pub depth: usize,
    /// A path segment starts with `.`, so every file in the block is hidden too.
    pub hidden: bool,
    /// File indices, sorted by (`ignored`, `hidden`, name).
    pub files: Vec<usize>,
    /// Child node indices, sorted by name.
    pub kids: Vec<usize>,
}

/// The directory tree after compaction. `nodes[0]` is the root; nodes are in preorder.
#[derive(Debug, Clone)]
pub struct Tree {
    pub root_name: String,
    pub nodes: Vec<Node>,
    /// The files, indexed like the input.
    pub files: Vec<TreeFile>,
}

struct RawNode {
    path: String,
    name: String,
    files: Vec<usize>,
    kids: Vec<usize>,
}

impl Tree {
    /// Builds the tree from `(path, ignored)` pairs. Every file has 0 lines.
    pub fn from_paths<'a>(
        root_name: &str,
        files: impl IntoIterator<Item = (&'a str, bool)>,
    ) -> Tree {
        Tree::from_files(root_name, files.into_iter().map(|(p, i)| (p, i, 0)))
    }

    /// Builds the tree from `(path, ignored, lines)` triples. No file is generated; a caller can set [`TreeFile::generated`].
    pub fn from_files<'a>(
        root_name: &str,
        files: impl IntoIterator<Item = (&'a str, bool, u64)>,
    ) -> Tree {
        let mut raw = vec![RawNode {
            path: String::new(),
            name: root_name.to_string(),
            files: vec![],
            kids: vec![],
        }];
        let mut by_path: HashMap<String, usize> = HashMap::from([(String::new(), 0)]);
        let mut out: Vec<TreeFile> = Vec::new();

        for (i, (path, ignored, lines)) in files.into_iter().enumerate() {
            let (dir, name) = match path.rfind('/') {
                Some(s) => (&path[..s], &path[s + 1..]),
                None => ("", path),
            };
            let node = ensure(&mut raw, &mut by_path, dir);
            raw[node].files.push(i);
            out.push(TreeFile {
                path: path.to_string(),
                name: name.to_string(),
                ignored,
                lines,
                generated: false,
                hidden: is_hidden(path),
                node: usize::MAX,
            });
        }

        // Hidden files take the last slots before ignored files, so hiding them moves no tile.
        for n in &mut raw {
            n.files.sort_by(|&a, &b| {
                fn key(f: &TreeFile) -> (bool, bool, &str) {
                    (f.ignored, f.hidden, f.name.as_str())
                }
                key(&out[a]).cmp(&key(&out[b]))
            });
        }
        let names: Vec<String> = raw.iter().map(|n| n.name.clone()).collect();
        for n in &mut raw {
            n.kids.sort_by(|&a, &b| names[a].cmp(&names[b]));
        }

        compact(&mut raw, 0);

        let mut nodes = Vec::new();
        emit(&raw, 0, 0, &mut nodes, &mut out);
        Tree {
            root_name: root_name.to_string(),
            nodes,
            files: out,
        }
    }

    /// Builds the tree from an index, with its line counts and generated flags.
    pub fn from_index(index: &Index) -> Tree {
        let mut tree = Tree::from_files(
            &index.root,
            index
                .files
                .iter()
                .map(|f| (f.path.as_str(), f.ignored, f.lines)),
        );
        for (t, f) in tree.files.iter_mut().zip(&index.files) {
            t.generated = f.generated;
        }
        tree
    }
}

/// The breadcrumb of a selection (`docs/design.md` `Screen regions`): the repo name, the directory names, and the file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breadcrumb<'a> {
    /// The repo name, then each directory name of the path.
    pub parts: Vec<&'a str>,
    /// The file name, for a file.
    pub last: Option<&'a str>,
}

impl Breadcrumb<'_> {
    /// `quayside › server › src › main.clj`.
    pub fn text(&self) -> String {
        let mut s = self.parts.join(" › ");
        if let Some(name) = self.last {
            s.push_str(" › ");
            s.push_str(name);
        }
        s
    }

    /// The part that shows bright: the file name, else the last directory name.
    pub fn bright(&self) -> &str {
        self.last
            .unwrap_or_else(|| self.parts.last().copied().unwrap_or(""))
    }
}

impl Tree {
    /// The breadcrumb with nothing selected: the repo name.
    pub fn root_crumb(&self) -> Breadcrumb<'_> {
        Breadcrumb {
            parts: vec![self.root_name.as_str()],
            last: None,
        }
    }

    /// The breadcrumb of a file.
    pub fn file_crumb(&self, f: usize) -> Breadcrumb<'_> {
        let mut c = self.root_crumb();
        let tf = &self.files[f];
        if let Some((dir, _)) = tf.path.rsplit_once('/') {
            c.parts.extend(dir.split('/'));
        }
        c.last = Some(tf.name.as_str());
        c
    }

    /// The breadcrumb of a directory node. The root's is the repo name and an empty part, as the TUI shows it.
    pub fn dir_crumb(&self, n: usize) -> Breadcrumb<'_> {
        let mut c = self.root_crumb();
        c.parts.extend(self.nodes[n].path.split('/'));
        c
    }

    /// The hover text of a directory node: its path and `/`.
    pub fn dir_label(&self, n: usize) -> String {
        format!("{}/", self.nodes[n].path)
    }
}

/// A path is hidden when any of its segments starts with `.`, for example `.github/workflows/ci.yml`.
pub fn is_hidden(path: &str) -> bool {
    path.split('/').any(|seg| seg.starts_with('.'))
}

fn ensure(raw: &mut Vec<RawNode>, by_path: &mut HashMap<String, usize>, p: &str) -> usize {
    if let Some(&n) = by_path.get(p) {
        return n;
    }
    let (parent_path, name) = match p.rfind('/') {
        Some(k) => (&p[..k], &p[k + 1..]),
        None => ("", p),
    };
    let parent = ensure(raw, by_path, parent_path);
    let n = raw.len();
    raw.push(RawNode {
        path: p.to_string(),
        name: name.to_string(),
        files: vec![],
        kids: vec![],
    });
    raw[parent].kids.push(n);
    by_path.insert(p.to_string(), n);
    n
}

/// Merges each chain of directories with no files and one child into one node (`:83-86`).
fn compact(raw: &mut [RawNode], n: usize) {
    for k in raw[n].kids.clone() {
        compact(raw, k);
    }
    let kids = raw[n].kids.clone();
    let mut merged = Vec::with_capacity(kids.len());
    for mut k in kids {
        while raw[k].files.is_empty() && raw[k].kids.len() == 1 {
            let c = raw[k].kids[0];
            raw[c].name = format!("{}/{}", raw[k].name, raw[c].name);
            k = c;
        }
        merged.push(k);
    }
    raw[n].kids = merged;
}

/// Writes the compacted tree in preorder with depths.
fn emit(
    raw: &[RawNode],
    r: usize,
    depth: usize,
    nodes: &mut Vec<Node>,
    files: &mut [TreeFile],
) -> usize {
    let id = nodes.len();
    let src = &raw[r];
    for &f in &src.files {
        files[f].node = id;
    }
    nodes.push(Node {
        path: src.path.clone(),
        name: src.name.clone(),
        depth,
        hidden: is_hidden(&src.path),
        files: src.files.clone(),
        kids: vec![],
    });
    let kids: Vec<usize> = src
        .kids
        .iter()
        .map(|&k| emit(raw, k, depth + 1, nodes, files))
        .collect();
    nodes[id].kids = kids;
    id
}

/// One directory block on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The header band's rows at the block's top: `head`, or the name band (units `040` and `062`).
    pub band: i32,
    pub depth: usize,
    pub node: usize,
}

/// Tile and header positions in cells.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The constants that made this layout.
    pub cfg: LayoutConfig,
    /// The top-left cell of each file's tile, indexed like the input files.
    pub tiles: Vec<(i32, i32)>,
    /// The drawn height of each file's tile in cells ([`LayoutConfig::tile_rows`]), indexed like `tiles`.
    pub tile_h: Vec<i32>,
    /// One header per directory block, in placement order (preorder by shelf).
    pub headers: Vec<Header>,
    /// Per node: the name size in points at zoom 1 (units `040` and `062`), or 0 without name bands.
    pub name_size: Vec<f64>,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, Default)]
struct Size {
    w: i32,
    h: i32,
}

/// A shelf: columns of stacked child nodes.
type Shelf = Vec<Vec<usize>>;

struct Plan {
    cfg: LayoutConfig,
    /// File columns per node.
    fc: Vec<usize>,
    /// File-grid slots per node: the file count, rounded up by [`LayoutConfig::quant`].
    cap: Vec<usize>,
    /// The file grid's height in rows per node, with the gap row under its last tiles. 0 without files.
    grid_h: Vec<i32>,
    /// Per file: its column in its node's grid, and its row offset from the grid's top.
    slot: Vec<(i32, i32)>,
    /// Per node: the count of files below it, at every depth (unit `062`).
    files: Vec<usize>,
    /// Per node: the header band's rows and the name size.
    band: Vec<i32>,
    name_size: Vec<f64>,
    /// Per node: shelves, each a list of columns, each a list of child nodes.
    shelves: Vec<Vec<Shelf>>,
    size: Vec<Size>,
}

impl Layout {
    pub fn compute(tree: &Tree, cfg: &LayoutConfig) -> Layout {
        let n = tree.nodes.len();
        let mut p = Plan {
            cfg: *cfg,
            fc: vec![0; n],
            cap: vec![0; n],
            grid_h: vec![0; n],
            slot: vec![(0, 0); tree.files.len()],
            files: vec![0; n],
            band: vec![0; n],
            name_size: vec![0.0; n],
            shelves: vec![vec![]; n],
            size: vec![Size::default(); n],
        };
        plan(tree, &mut p, 0);
        let mut layout = Layout {
            cfg: *cfg,
            tiles: vec![(0, 0); tree.files.len()],
            tile_h: tree
                .files
                .iter()
                .map(|f| cfg.tile_rows(f.lines, f.generated))
                .collect(),
            headers: Vec::with_capacity(n),
            name_size: Vec::new(),
            w: 0,
            h: 0,
        };
        place(tree, &p, 0, 0, 0, &mut layout);
        layout.name_size = p.name_size;
        layout.w = p.size[0].w;
        layout.h = p.size[0].h;
        layout
    }
}

fn ceil_div(a: usize, b: usize) -> usize {
    a.div_ceil(b)
}

/// The least `r` with `r * r >= x`.
fn ceil_sqrt(x: usize) -> usize {
    let r = x.isqrt();
    if r * r < x { r + 1 } else { r }
}

/// `size` (`:103-118`). It needs the node's plan and the sizes of its children.
/// It also sets the node's header band, from the count of files below the node (unit `062`), which `plan` sums bottom-up.
fn size(tree: &Tree, p: &mut Plan, n: usize) -> Size {
    let c = &p.cfg;
    let node = &tree.nodes[n];
    let nf = node.files.len();
    let ind = if node.depth > 0 { c.ind } else { 0 };
    let fc = p.fc[n];
    let fw = if nf > 0 { fc as i32 * c.tw } else { 0 };
    let fh = p.grid_h[n];
    let (mut sw, mut sh) = (0, 0);
    for (si, shelf) in p.shelves[n].iter().enumerate() {
        let (mut w, mut h) = (0, 0);
        for (j, col) in shelf.iter().enumerate() {
            let (mut cw, mut chh) = (0, 0);
            for (q, &k) in col.iter().enumerate() {
                let s = p.size[k];
                cw = cw.max(s.w);
                chh += if q > 0 { c.vg } else { 0 } + s.h;
            }
            w += if j > 0 { c.gx } else { 0 } + cw;
            h = h.max(chh);
        }
        sw = sw.max(w);
        sh += if si > 0 { c.gy } else { 0 } + h;
    }
    let gap = if nf > 0 && !p.shelves[n].is_empty() {
        c.gy
    } else {
        0
    };
    let w = c.tw.max(c.label(&node.name)).max(ind + fw.max(sw));
    // The header text is the name and `/`.
    let (name_size, band) = c.name_band(w, node.name.chars().count() + 1, p.files[n]);
    p.band[n] = band;
    p.name_size[n] = name_size;
    let s = Size {
        w,
        h: band + fh + gap + sh,
    };
    let r = c.round;
    if node.depth == 0 || (r.max_depth > 0 && node.depth > r.max_depth) {
        return s;
    }
    Size {
        w: if r.w { c.quant.round_i32(s.w) } else { s.w },
        h: if r.h { c.quant.round_i32(s.h) } else { s.h },
    }
}

/// A child block in pixel units for the packing math.
struct Item {
    k: usize,
    w: i64,
    h: i64,
}

fn col_w(c: &[&Item]) -> i64 {
    c.iter().map(|s| s.w).max().unwrap_or(i64::MIN)
}

fn col_h(c: &[&Item], vg: i64) -> i64 {
    c.iter()
        .enumerate()
        .map(|(q, s)| if q > 0 { vg } else { 0 } + s.h)
        .sum()
}

fn shelf_w(sh: &[Vec<&Item>], gx: i64) -> i64 {
    sh.iter()
        .enumerate()
        .map(|(j, c)| if j > 0 { gx } else { 0 } + col_w(c))
        .sum()
}

fn shelf_h(sh: &[Vec<&Item>], vg: i64) -> i64 {
    sh.iter().map(|c| col_h(c, vg)).max().unwrap_or(i64::MIN)
}

/// `plan` (`:119-143`): packs the children of `n` into shelves, tallest first.
fn plan(tree: &Tree, p: &mut Plan, n: usize) {
    for &k in &tree.nodes[n].kids {
        plan(tree, p, k);
    }
    let c = p.cfg;
    let node = &tree.nodes[n];
    let nf = node.files.len();
    p.files[n] = nf + node.kids.iter().map(|&k| p.files[k]).sum::<usize>();
    let ind = if node.depth > 0 { c.ind } else { 0 };
    let cap = if c.round.cap { c.quant.round(nf) } else { nf };
    let mut fc = if nf > 0 {
        (cap as f64 * c.fc_k).sqrt().ceil() as usize
    } else {
        0
    };
    if nf > 0 && c.label_w > 0 {
        // Small directories spread their files under the name.
        let under_name = (c.label(&node.name) - ind).max(0) as usize;
        fc = cap.min(fc.max(ceil_div(under_name, c.tw as usize)));
    }
    p.fc[n] = fc;
    p.cap[n] = cap;
    p.grid_h[n] = if nf == 0 {
        0
    } else if c.line_scaled() {
        flow(tree, p, n, fc)
    } else {
        for (i, &f) in node.files.iter().enumerate() {
            p.slot[f] = ((i % fc) as i32, (i / fc) as i32 * c.th);
        }
        ceil_div(cap, fc) as i32 * c.th
    };
    let (gx, vg) = (c.gx as i64 * c.px, c.vg as i64 * c.py);

    let mut ks: Vec<Item> = node
        .kids
        .iter()
        .map(|&k| Item {
            k,
            w: p.size[k].w as i64 * c.px,
            h: p.size[k].h as i64 * c.py,
        })
        .collect();
    let by_name = |a: &Item, b: &Item| {
        tree.nodes[a.k]
            .name
            .as_bytes()
            .cmp(tree.nodes[b.k].name.as_bytes())
    };
    match c.order {
        Order::Tallest => ks.sort_by(|a, b| b.h.cmp(&a.h).then_with(|| by_name(a, b))),
        Order::Name => ks.sort_by(by_name),
    }

    let fwpx = if nf > 0 {
        fc as i64 * c.tw as i64 * c.px
    } else {
        0
    };
    let mut area: i64 = ks.iter().map(|s| s.w * s.h).sum();
    if nf > 0 {
        area += fwpx * p.grid_h[n] as i64 * c.py;
    }
    let k = if node.depth > 0 { 1.5 } else { c.root_k };
    let mut target = ((area as f64) * k).sqrt();
    target = target.max(fwpx as f64);
    for s in &ks {
        target = target.max(s.w as f64);
    }

    let mut shelves: Vec<Vec<Vec<&Item>>> = Vec::new();
    for s in &ks {
        let Some(shelf) = shelves.last_mut() else {
            shelves.push(vec![vec![s]]);
            continue;
        };
        let h = shelf_h(shelf, vg);
        let last = shelf.len() - 1;
        let sw = shelf_w(shelf, gx);
        let fits = |j: usize, c: &[&Item]| {
            col_h(c, vg) + vg + s.h <= h
                && (s.w <= col_w(c) || (j == last && ((sw - col_w(c) + s.w) as f64) <= target))
        };
        if let Some(j) = (0..shelf.len()).find(|&j| fits(j, &shelf[j])) {
            shelf[j].push(s);
            continue;
        }
        if ((sw + gx + s.w) as f64) <= target {
            shelf.push(vec![s]);
            continue;
        }
        shelves.push(vec![vec![s]]);
    }
    p.shelves[n] = shelves
        .iter()
        .map(|sh| sh.iter().map(|c| c.iter().map(|s| s.k).collect()).collect())
        .collect();
    p.size[n] = size(tree, p, n);
}

/// The columns that line-scaled tiles of these pitches take when they flow down columns of height `h`.
fn columns(pitch: &[i32], h: i32) -> usize {
    let (mut cols, mut y) = (1, 0);
    for &p in pitch {
        if y > 0 && y + p > h {
            cols += 1;
            y = 0;
        }
        y += p;
    }
    cols
}

/// Flows a node's line-scaled tiles in name order down `fc` columns, and returns the column height (unit `039`):
/// the least height at which they fit, at least the tallest tile's pitch, rounded up by [`LayoutConfig::quant`].
/// A tile keeps its height until its line count passes a step, and the rounded column height absorbs small changes.
fn flow(tree: &Tree, p: &mut Plan, n: usize, fc: usize) -> i32 {
    let c = p.cfg;
    let files = &tree.nodes[n].files;
    let gap = c.row_gap();
    let pitch: Vec<i32> = files
        .iter()
        .map(|&f| {
            let tf = &tree.files[f];
            c.tile_rows(tf.lines, tf.generated) + gap
        })
        .collect();
    let (mut lo, mut hi) = (
        pitch.iter().copied().max().unwrap_or(0),
        pitch.iter().sum::<i32>(),
    );
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if columns(&pitch, mid) <= fc {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    let h = c.quant.round_i32(lo);
    let (mut col, mut y) = (0, 0);
    for (&f, &pt) in files.iter().zip(&pitch) {
        if y > 0 && y + pt > h {
            col += 1;
            y = 0;
        }
        p.slot[f] = (col, y);
        y += pt;
    }
    h
}

/// `place` (`:149-165`).
fn place(tree: &Tree, p: &Plan, n: usize, x: i32, y: i32, out: &mut Layout) {
    let c = &p.cfg;
    let node = &tree.nodes[n];
    let s = p.size[n];
    let ind = if node.depth > 0 { c.ind } else { 0 };
    out.headers.push(Header {
        x,
        y,
        w: s.w,
        h: s.h,
        band: p.band[n],
        depth: node.depth,
        node: n,
    });
    let mut cy = y + p.band[n];
    for &f in &node.files {
        let (col, dy) = p.slot[f];
        out.tiles[f] = (x + ind + col * c.tw, cy + dy);
    }
    if !node.files.is_empty() {
        cy += p.grid_h[n] + if p.shelves[n].is_empty() { 0 } else { c.gy };
    }
    for (si, shelf) in p.shelves[n].iter().enumerate() {
        if si > 0 {
            cy += c.gy;
        }
        let mut cx = x + ind;
        let mut sh = 0;
        for col in shelf {
            let mut yy = cy;
            let mut cw = 0;
            for (q, &k) in col.iter().enumerate() {
                if q > 0 {
                    yy += c.vg;
                }
                place(tree, p, k, cx, yy, out);
                yy += p.size[k].h;
                cw = cw.max(p.size[k].w);
            }
            cx += cw + c.gx;
            sh = sh.max(yy - cy);
        }
        cy += sh;
    }
}
