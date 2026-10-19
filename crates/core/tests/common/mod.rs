//! Layout checks shared by the fixture tests and the real-repo tests.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use minimap_core::fuzzy::Pattern;
use minimap_core::{Index, Layout, LayoutConfig, Tree};

/// Asserts that the layout equals a golden file from `mockups/export-layout.mjs`.
pub fn assert_parity(tree: &Tree, layout: &Layout, golden: &serde_json::Value) {
    assert_eq!(
        layout.w,
        golden["w"].as_i64().unwrap() as i32,
        "layout width"
    );
    assert_eq!(
        layout.h,
        golden["h"].as_i64().unwrap() as i32,
        "layout height"
    );
    let tiles = golden["tiles"].as_array().unwrap();
    assert_eq!(layout.tiles.len(), tiles.len(), "tile count");
    for (i, t) in tiles.iter().enumerate() {
        let want = (t[0].as_i64().unwrap() as i32, t[1].as_i64().unwrap() as i32);
        assert_eq!(layout.tiles[i], want, "tile of {}", tree.files[i].path);
    }
    let headers = golden["headers"].as_array().unwrap();
    assert_eq!(layout.headers.len(), headers.len(), "header count");
    for (h, g) in layout.headers.iter().zip(headers) {
        let path = &tree.nodes[h.node].path;
        let got = (path.as_str(), h.x, h.y, h.w, h.h, h.depth);
        let want = (
            g["path"].as_str().unwrap(),
            g["x"].as_i64().unwrap() as i32,
            g["y"].as_i64().unwrap() as i32,
            g["w"].as_i64().unwrap() as i32,
            g["h"].as_i64().unwrap() as i32,
            g["depth"].as_u64().unwrap() as usize,
        );
        assert_eq!(got, want, "header of {path:?}");
    }
}

/// Asserts that no two drawn tile rectangles (the width of [`LayoutConfig::tile_size`], the height of [`Layout::tile_h`]) share a cell.
pub fn assert_no_overlap(tree: &Tree, layout: &Layout) {
    let tw = layout.cfg.tile_size().0;
    let mut seen: HashMap<(i32, i32), usize> = HashMap::new();
    for (i, &(x, y)) in layout.tiles.iter().enumerate() {
        for dy in 0..layout.tile_h[i] {
            for dx in 0..tw {
                if let Some(j) = seen.insert((x + dx, y + dy), i) {
                    panic!("{} overlaps {}", tree.files[i].path, tree.files[j].path);
                }
            }
        }
    }
}

/// Asserts that no card and no header row shares a cell with another.
pub fn assert_no_card_or_header_overlap(tree: &Tree, layout: &Layout) {
    let tw = layout.cfg.tile_size().0;
    let mut seen: HashMap<(i32, i32), String> = HashMap::new();
    let mut mark = |x0: i32, y0: i32, w: i32, h: i32, what: String| {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                if let Some(other) = seen.insert((x, y), what.clone()) {
                    panic!("{what} overlaps {other} at ({x}, {y})");
                }
            }
        }
    };
    for (i, &(x, y)) in layout.tiles.iter().enumerate() {
        mark(
            x,
            y,
            tw,
            layout.tile_h[i],
            format!("card {}", tree.files[i].path),
        );
    }
    for h in &layout.headers {
        let path = &tree.nodes[h.node].path;
        mark(h.x, h.y, h.w, h.band, format!("header {path:?}"));
    }
}

/// Asserts that each tile lies inside its directory's block, below the header band,
/// and that each block lies inside its parent's block, below the parent's band.
pub fn assert_containment(tree: &Tree, layout: &Layout) {
    let tw = layout.cfg.tile_size().0;
    let rect: HashMap<usize, _> = layout.headers.iter().map(|h| (h.node, *h)).collect();
    assert_eq!(rect.len(), tree.nodes.len(), "one header per node");
    for (i, &(x, y)) in layout.tiles.iter().enumerate() {
        let th = layout.tile_h[i];
        let h = rect[&tree.files[i].node];
        let inside = x >= h.x && x + tw <= h.x + h.w && y >= h.y + h.band && y + th <= h.y + h.h;
        assert!(
            inside,
            "{} at ({x}, {y}) is outside its block {h:?}",
            tree.files[i].path
        );
    }
    for (n, node) in tree.nodes.iter().enumerate() {
        let p = rect[&n];
        for k in &node.kids {
            let c = rect[k];
            let inside = c.x >= p.x
                && c.x + c.w <= p.x + p.w
                && c.y >= p.y + p.band
                && c.y + c.h <= p.y + p.h;
            assert!(
                inside,
                "block {:?} is outside its parent {:?}",
                tree.nodes[*k].path, node.path
            );
        }
    }
    assert_eq!(
        (layout.w, layout.h),
        (layout.headers[0].w, layout.headers[0].h)
    );
}

/// Tile positions and header rectangles keyed by path, so layouts of reordered inputs compare.
pub type ByPath = (
    HashMap<String, (i32, i32)>,
    HashSet<(String, i32, i32, i32, i32)>,
);

pub fn by_path(tree: &Tree, layout: &Layout) -> ByPath {
    let tiles = tree
        .files
        .iter()
        .zip(&layout.tiles)
        .map(|(f, &t)| (f.path.clone(), t))
        .collect();
    let headers = layout
        .headers
        .iter()
        .map(|h| (tree.nodes[h.node].path.clone(), h.x, h.y, h.w, h.h))
        .collect();
    (tiles, headers)
}

/// Asserts that shuffled input orders give the same layout per path.
pub fn assert_deterministic(index: &Index, cfg: &LayoutConfig) {
    let tree = Tree::from_index(index);
    let want = by_path(&tree, &Layout::compute(&tree, cfg));
    let mut files = index.files.clone();
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    for round in 0..3 {
        // Fisher-Yates with a fixed LCG, so the test is reproducible.
        for i in (1..files.len()).rev() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            files.swap(i, (seed >> 33) as usize % (i + 1));
        }
        let shuffled = Index {
            files: files.clone(),
            ..index.clone()
        };
        let t = Tree::from_index(&shuffled);
        assert_eq!(
            by_path(&t, &Layout::compute(&t, cfg)),
            want,
            "shuffle round {round}"
        );
    }
}

/// Asserts that every query in a golden file from `scripts/fzf-golden.py` gives the C library's matches:
/// the same files with the same scores, and the same positions for the best files.
pub fn assert_matches_fzf_native(files: &str, golden: &str) -> usize {
    let index = Index::from_json(files).unwrap();
    let golden: serde_json::Value = serde_json::from_str(golden).unwrap();
    let mut checked = 0;
    for g in golden["queries"].as_array().unwrap() {
        let q = g["query"].as_str().unwrap();
        let pattern = Pattern::parse(q);
        let mut want: Vec<(usize, i32)> = Vec::new();
        for h in g["hits"].as_array().unwrap() {
            want.push((
                h[0].as_u64().unwrap() as usize,
                h[1].as_i64().unwrap() as i32,
            ));
        }
        let got: Vec<(usize, i32)> = index
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.path.is_ascii())
            .filter_map(|(i, f)| pattern.score(&f.path).map(|m| (i, m.score)))
            .collect();
        assert_eq!(got.len(), want.len(), "match count for {q:?}");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g, w, "score for {q:?} on {}", index.files[w.0].path);
        }
        for t in g["top"].as_array().unwrap() {
            let i = t[0].as_u64().unwrap() as usize;
            let want: Vec<usize> = t[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_u64().unwrap() as usize)
                .collect();
            let got = pattern.score(&index.files[i].path).unwrap().positions;
            assert_eq!(got, want, "positions for {q:?} on {}", index.files[i].path);
        }
        checked += want.len();
    }
    checked
}
