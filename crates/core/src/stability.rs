//! Layout stability: how far tiles and directory headers move between two layouts of one repo (unit `009`).
//!
//! A tile that moves while its directory's header stays is a reflow: an insert or a delete in its own grid moved it.
//! A tile whose header moved too is a packing move: its block moved.

use std::collections::HashMap;

use crate::layout::{Layout, Tree};

/// The positions of one layout, keyed by path, so layouts of different commits compare.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Positions {
    /// The top-left cell of each file's tile.
    pub tiles: HashMap<String, (i32, i32)>,
    /// The header cell of each directory block, keyed by the directory path. The root is `""`.
    pub headers: HashMap<String, (i32, i32)>,
    /// The directory of each file: the path of its block.
    pub dir: HashMap<String, String>,
}

impl Positions {
    pub fn of(tree: &Tree, layout: &Layout) -> Positions {
        let mut p = Positions::default();
        for (f, &xy) in tree.files.iter().zip(&layout.tiles) {
            p.tiles.insert(f.path.clone(), xy);
            p.dir
                .insert(f.path.clone(), tree.nodes[f.node].path.clone());
        }
        for h in &layout.headers {
            p.headers
                .insert(tree.nodes[h.node].path.clone(), (h.x, h.y));
        }
        p
    }
}

/// The moves between two layouts, over the files and directories present in both.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Moves {
    /// Files present in both layouts.
    pub files: usize,
    /// Shared files whose tile moved.
    pub moved: usize,
    /// Moved files whose directory header did not move.
    pub reflow: usize,
    /// The median and 90th-percentile displacement of the moved files, in cells (nearest rank). 0 when none moved.
    pub median: f64,
    pub p90: f64,
    /// Directories present in both layouts, and those whose header moved.
    pub dirs: usize,
    pub dirs_moved: usize,
}

impl Moves {
    /// The share of shared files that moved, from 0 to 1.
    pub fn moved_share(&self) -> f64 {
        share(self.moved, self.files)
    }

    /// The share of shared files that moved only inside their block.
    pub fn reflow_share(&self) -> f64 {
        share(self.reflow, self.files)
    }

    /// The share of shared directories whose header moved.
    pub fn dirs_moved_share(&self) -> f64 {
        share(self.dirs_moved, self.dirs)
    }
}

fn share(n: usize, of: usize) -> f64 {
    if of == 0 { 0.0 } else { n as f64 / of as f64 }
}

/// Compares two layouts of the same repo.
pub fn compare(a: &Positions, b: &Positions) -> Moves {
    let mut m = Moves::default();
    let mut dist = Vec::new();
    for (path, &(ax, ay)) in &a.tiles {
        let Some(&(bx, by)) = b.tiles.get(path) else {
            continue;
        };
        m.files += 1;
        if (ax, ay) == (bx, by) {
            continue;
        }
        m.moved += 1;
        dist.push(f64::from(bx - ax).hypot(f64::from(by - ay)));
        let header = |p: &Positions| p.dir.get(path).and_then(|d| p.headers.get(d)).copied();
        if header(a).is_some() && header(a) == header(b) {
            m.reflow += 1;
        }
    }
    dist.sort_by(f64::total_cmp);
    m.median = nearest_rank(&dist, 0.5);
    m.p90 = nearest_rank(&dist, 0.9);
    for (dir, xy) in &a.headers {
        if let Some(other) = b.headers.get(dir) {
            m.dirs += 1;
            if other != xy {
                m.dirs_moved += 1;
            }
        }
    }
    m
}

/// The nearest-rank percentile of sorted values: the smallest value with at least `q` of the values at or below it.
pub fn nearest_rank(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (q * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank.min(sorted.len()) - 1]
}
