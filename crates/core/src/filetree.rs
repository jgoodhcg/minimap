//! The file tree's row model (units 017 and 018), shared by the TUI's left column and the GUI's (unit `042`).
//!
//! Rows come from the compacted layout tree, so a chain shows as one row, as on the map.
//! The cursor is the one selection of the app. Opening and closing rows never changes the layout.
//! Scrolling counts whole rows: `top` is the first row on screen, and `height` the rows on screen.

use crate::layout::Tree;

/// A file or a directory: a tree row, the selection, a hover, or a hit on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Entry {
    /// A directory block, by node index.
    Dir(usize),
    /// A file, by file index.
    File(usize),
}

/// What the tree keys `l` and `h` do to an entry (unit `043`), so the TUI and the GUI decide alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Open this directory node.
    Open(usize),
    /// Close this directory node.
    Close(usize),
    /// Select this entry.
    Select(Entry),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub entry: Entry,
    /// 0 for the root's children.
    pub depth: usize,
}

pub struct FileTree {
    /// Open state per node.
    open: Vec<bool>,
    /// The parent node of each node. The root has none.
    parent: Vec<Option<usize>>,
    /// Files per node including subdirectories: all files, and files that are not hidden.
    count_all: Vec<usize>,
    count_shown: Vec<usize>,
    /// The cursor entry. `None` until the first row exists.
    pub cursor: Option<Entry>,
    /// The first row on screen.
    pub top: usize,
    /// The number of rows on screen at the last draw.
    pub height: usize,
}

impl FileTree {
    pub fn new(tree: &Tree) -> FileTree {
        let n = tree.nodes.len();
        let mut parent = vec![None; n];
        for (p, node) in tree.nodes.iter().enumerate() {
            for &k in &node.kids {
                parent[k] = Some(p);
            }
        }
        let (mut count_all, mut count_shown) = (vec![0; n], vec![0; n]);
        // Nodes are in preorder, so a reverse pass sees each child before its parent.
        for id in (0..n).rev() {
            let node = &tree.nodes[id];
            let files = node.files.iter().filter(|&&f| !tree.files[f].ignored);
            count_all[id] = files.clone().count();
            count_shown[id] = files.filter(|&&f| !tree.files[f].hidden).count();
            for &k in &node.kids {
                count_all[id] += count_all[k];
                count_shown[id] += count_shown[k];
            }
        }
        let mut t = FileTree {
            open: vec![false; n],
            parent,
            count_all,
            count_shown,
            cursor: None,
            top: 0,
            height: 0,
        };
        t.cursor = t.rows(tree, true).first().map(|r| r.entry);
        t
    }

    pub fn is_open(&self, node: usize) -> bool {
        self.open[node]
    }

    /// The file count of a directory row, including subdirectories.
    pub fn count(&self, node: usize, show_hidden: bool) -> usize {
        if show_hidden {
            self.count_all[node]
        } else {
            self.count_shown[node]
        }
    }

    /// The visible rows: under each open directory, its child directories, then its files in grid order.
    pub fn rows(&self, tree: &Tree, show_hidden: bool) -> Vec<Row> {
        let mut out = Vec::new();
        self.push_children(tree, 0, 0, show_hidden, &mut out);
        out
    }

    fn push_children(
        &self,
        tree: &Tree,
        node: usize,
        depth: usize,
        show_hidden: bool,
        out: &mut Vec<Row>,
    ) {
        for &k in &tree.nodes[node].kids {
            if tree.nodes[k].hidden && !show_hidden {
                continue;
            }
            out.push(Row {
                entry: Entry::Dir(k),
                depth,
            });
            if self.open[k] {
                self.push_children(tree, k, depth + 1, show_hidden, out);
            }
        }
        for &f in &tree.nodes[node].files {
            let file = &tree.files[f];
            if file.ignored || (file.hidden && !show_hidden) {
                continue;
            }
            out.push(Row {
                entry: Entry::File(f),
                depth,
            });
        }
    }

    /// The row index of the cursor, if its entry is visible.
    pub fn cursor_index(&self, rows: &[Row]) -> Option<usize> {
        let c = self.cursor?;
        rows.iter().position(|r| r.entry == c)
    }

    /// The parent directory row of an entry. Root children have none.
    pub fn parent_of(&self, tree: &Tree, e: Entry) -> Option<Entry> {
        let p = match e {
            Entry::Dir(n) => self.parent[n]?,
            Entry::File(f) => tree.files[f].node,
        };
        (p != 0).then_some(Entry::Dir(p))
    }

    /// Opens every directory above an entry, so its row is visible.
    pub fn reveal(&mut self, tree: &Tree, e: Entry) {
        let mut p = self.parent_of(tree, e);
        while let Some(Entry::Dir(n)) = p {
            self.open[n] = true;
            p = self.parent_of(tree, Entry::Dir(n));
        }
    }

    /// Moves the cursor by `delta` rows, clamped, and scrolls to keep it on screen.
    pub fn move_by(&mut self, rows: &[Row], delta: isize) {
        if rows.is_empty() {
            return;
        }
        let i = self.cursor_index(rows).unwrap_or(0) as isize;
        let j = (i + delta).clamp(0, rows.len() as isize - 1) as usize;
        self.cursor = Some(rows[j].entry);
        self.keep_visible(rows);
    }

    /// `l` on an entry: open a closed directory; on an open directory, go to its first child row. Nothing for a file.
    pub fn step_in(&self, tree: &Tree, rows: &[Row], e: Option<Entry>) -> Option<Step> {
        let Some(Entry::Dir(n)) = e else {
            return None;
        };
        if !self.open[n] {
            return Some(Step::Open(n));
        }
        let i = rows
            .iter()
            .position(|r| r.entry == Entry::Dir(n))
            .unwrap_or(0);
        rows.get(i + 1)
            .filter(|r| self.parent_of(tree, r.entry) == Some(Entry::Dir(n)))
            .map(|r| Step::Select(r.entry))
    }

    /// `h` on an entry: close an open directory; else go up to the parent directory. Nothing for a root child.
    pub fn step_out(&self, tree: &Tree, e: Option<Entry>) -> Option<Step> {
        match e? {
            Entry::Dir(n) if self.open[n] => Some(Step::Close(n)),
            e => self.parent_of(tree, e).map(Step::Select),
        }
    }

    pub fn set_open(&mut self, node: usize, open: bool) {
        self.open[node] = open;
    }

    /// Scrolls so the cursor row is on screen. Before the first draw the height is unknown, and it does nothing.
    pub fn keep_visible(&mut self, rows: &[Row]) {
        let Some(i) = self.cursor_index(rows) else {
            return;
        };
        if self.height == 0 {
            return;
        }
        let h = self.height;
        if i < self.top {
            self.top = i;
        } else if i >= self.top + h {
            self.top = i + 1 - h;
        }
        self.clamp_top(rows.len());
    }

    /// Scrolls by `delta` rows without moving the cursor.
    pub fn scroll(&mut self, len: usize, delta: isize) {
        self.top = (self.top as isize + delta).max(0) as usize;
        self.clamp_top(len);
    }

    pub fn clamp_top(&mut self, len: usize) {
        self.top = self.top.min(len.saturating_sub(self.height.max(1)));
    }
}
