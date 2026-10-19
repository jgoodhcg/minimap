//! The layout stability metric (unit `009`).

use std::collections::HashMap;

use minimap_core::stability::{Positions, compare, nearest_rank};
use minimap_core::{Layout, LayoutConfig, Tree};

/// Ten files in one directory at `(k, 0)`, with the directory header at the origin.
fn ten() -> Positions {
    let mut p = Positions::default();
    for k in 0..10 {
        let path = format!("d/f{k}");
        p.tiles.insert(path.clone(), (k, 0));
        p.dir.insert(path, "d".to_string());
    }
    p.headers = HashMap::from([("d".to_string(), (0, -1)), (String::new(), (0, -2))]);
    p
}

#[test]
fn three_of_ten_moved_by_known_amounts() {
    let a = ten();
    let mut b = ten();
    // Displacements 1, 5 (a 3-4-5 triangle), and 2.
    b.tiles.insert("d/f0".into(), (0, 1));
    b.tiles.insert("d/f1".into(), (4, 4));
    b.tiles.insert("d/f2".into(), (4, 0));
    // A file only in `b` does not count.
    b.tiles.insert("d/new".into(), (20, 0));
    b.dir.insert("d/new".into(), "d".into());
    let m = compare(&a, &b);
    assert_eq!(m.files, 10);
    assert_eq!(m.moved, 3);
    assert!((m.moved_share() - 0.3).abs() < 1e-9);
    assert_eq!(m.median, 2.0);
    assert_eq!(m.p90, 5.0);
    // The header stayed, so all three moved inside their block.
    assert_eq!(m.reflow, 3);
    assert_eq!((m.dirs, m.dirs_moved), (2, 0));
}

#[test]
fn identical_trees_report_nothing_moved() {
    let paths = ["src/a.clj", "src/b.clj", "src/x/c.clj", "README.md"];
    let tree = Tree::from_paths("r", paths.iter().map(|p| (*p, false)));
    for cfg in [LayoutConfig::CHARS, LayoutConfig::TILES] {
        let p = Positions::of(&tree, &Layout::compute(&tree, &cfg));
        let m = compare(&p, &p.clone());
        assert_eq!((m.files, m.moved, m.reflow), (4, 0, 0));
        assert_eq!((m.median, m.p90), (0.0, 0.0));
        assert_eq!(m.dirs_moved, 0);
        assert_eq!(m.dirs, 3);
    }
}

#[test]
fn a_block_move_is_packing_and_a_grid_insert_is_reflow() {
    let mut a = Positions::default();
    for (path, dir, xy) in [
        ("a/m", "a", (2, 1)),
        ("a/z", "a", (4, 1)),
        ("b/y", "b", (2, 5)),
    ] {
        a.tiles.insert(path.into(), xy);
        a.dir.insert(path.into(), dir.into());
    }
    a.headers = HashMap::from([("a".into(), (0, 0)), ("b".into(), (0, 4))]);
    let mut b = a.clone();
    // An insert in `a` shifts `a/z` inside its grid; block `a` stays.
    b.tiles.insert("a/z".into(), (6, 1));
    // Block `b` moves down 2 rows with its file.
    b.headers.insert("b".into(), (0, 6));
    b.tiles.insert("b/y".into(), (2, 7));
    let m = compare(&a, &b);
    assert_eq!((m.files, m.moved), (3, 2));
    assert_eq!(m.reflow, 1, "a/z moved inside block a");
    assert_eq!(m.moved - m.reflow, 1, "b/y moved with block b");
    assert_eq!((m.dirs, m.dirs_moved), (2, 1));
}

#[test]
fn nearest_rank_percentiles() {
    assert_eq!(nearest_rank(&[], 0.5), 0.0);
    assert_eq!(nearest_rank(&[3.0], 0.9), 3.0);
    let v: Vec<f64> = (1..=10).map(f64::from).collect();
    assert_eq!(nearest_rank(&v, 0.5), 5.0);
    assert_eq!(nearest_rank(&v, 0.9), 9.0);
    assert_eq!(nearest_rank(&v, 1.0), 10.0);
}
