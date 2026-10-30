mod common;

use std::fs;
use std::time::Instant;

use minimap_core::layout::{NAME_CAP_FILES, NAME_SHARE, Names, Order, Quant, Round};
use minimap_core::{Index, Layout, LayoutConfig, Tree};

const TILES: LayoutConfig = LayoutConfig::TILES;
const CHARS: LayoutConfig = LayoutConfig::CHARS;

fn fixture(name: &str) -> String {
    fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn quayside() -> Index {
    Index::from_json(&fixture("quayside-files.json")).unwrap()
}

fn golden(name: &str) -> serde_json::Value {
    serde_json::from_str(&fixture(name)).unwrap()
}

#[test]
fn tiles_match_the_v3_mockup_tile_for_tile() {
    let index = quayside();
    assert_eq!(index.files.len(), 2585);
    let tree = Tree::from_index(&index);
    let layout = Layout::compute(&tree, &TILES);
    common::assert_parity(&tree, &layout, &golden("quayside-layout.json"));
}

#[test]
fn chars_match_the_v5_mockup_tile_for_tile() {
    // The mockup does not round sizes (unit 027), so the parity check runs without rounding.
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CHARS.unrounded());
    assert_eq!((layout.tiles.len(), layout.headers.len()), (2585, 468));
    assert_eq!((layout.w, layout.h), (290, 192));
    common::assert_parity(&tree, &layout, &golden("quayside-chars-layout.json"));
}

#[test]
fn shuffled_input_gives_the_same_layout() {
    common::assert_deterministic(&quayside(), &TILES);
    common::assert_deterministic(&quayside(), &CHARS);
}

#[test]
fn tiles_do_not_overlap() {
    let tree = Tree::from_index(&quayside());
    common::assert_no_overlap(&tree, &Layout::compute(&tree, &TILES));
    common::assert_no_overlap(&tree, &Layout::compute(&tree, &CHARS));
}

#[test]
fn tiles_lie_inside_their_directory() {
    let tree = Tree::from_index(&quayside());
    common::assert_containment(&tree, &Layout::compute(&tree, &TILES));
    common::assert_containment(&tree, &Layout::compute(&tree, &CHARS));
}

#[test]
fn the_drawn_tile_size_follows_the_pitch() {
    assert_eq!(TILES.tile_size(), (21, 2));
    assert_eq!(CHARS.tile_size(), (1, 1));
}

#[test]
fn a_chars_block_is_as_wide_as_its_name() {
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CHARS);
    for h in &layout.headers {
        let name = &tree.nodes[h.node].name;
        let min = (name.chars().count() as i32 + 1).min(28);
        assert!(h.w >= min, "{name:?} is {} wide, below {min}", h.w);
    }
    // TILES has no label width: a block is only as wide as its tiles.
    let tree = Tree::from_paths("r", [("a-long-directory-name/x.clj", false)]);
    let layout = Layout::compute(&tree, &TILES);
    assert_eq!(layout.headers[1].w, 2 + 22);
}

#[test]
fn a_small_directory_with_a_long_name_spreads_its_files_on_one_row() {
    let tree = Tree::from_paths(
        "r",
        [
            ("a-long-directory-name/x.clj", false),
            ("a-long-directory-name/y.clj", false),
        ],
    );
    let layout = Layout::compute(&tree, &CHARS);
    let (x, y) = (layout.tiles[0], layout.tiles[1]);
    assert_eq!(x.1, y.1, "both files are on one row");
    assert_eq!(y.0, x.0 + CHARS.tw);
    // `a-long-directory-name/` is 22 cells: the block is that wide.
    let h = layout.headers[1];
    assert_eq!((h.w, h.h), (22, 2));
    // Without the name rule, ceil(sqrt(2 * 1.1)) = 2 columns too; with 5 files the name spreads them wider.
    let five: Vec<String> = (0..5)
        .map(|i| format!("a-long-directory-name/{i}.clj"))
        .collect();
    let tree = Tree::from_paths("r", five.iter().map(|p| (p.as_str(), false)));
    let layout = Layout::compute(&tree, &CHARS);
    assert!(
        layout.tiles.iter().all(|t| t.1 == layout.tiles[0].1),
        "5 files fit under a 22-cell name: {:?}",
        layout.tiles
    );
}

#[test]
fn a_chain_of_single_directories_compacts_into_one_header() {
    let tree = Tree::from_paths("r", [("a/b/c/x.clj", false), ("a/b/c/y.clj", false)]);
    let layout = Layout::compute(&tree, &TILES);
    let headers: Vec<(&str, &str, usize)> = layout
        .headers
        .iter()
        .map(|h| {
            let n = &tree.nodes[h.node];
            (n.path.as_str(), n.name.as_str(), h.depth)
        })
        .collect();
    assert_eq!(headers, [("", "r", 0), ("a/b/c", "a/b/c", 1)]);
    assert_eq!(tree.nodes[0].kids, [1]);
}

#[test]
fn a_directory_with_files_stops_compaction() {
    let tree = Tree::from_paths("r", [("a/top.clj", false), ("a/b/c/x.clj", false)]);
    let names: Vec<&str> = tree.nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["r", "a", "b/c"]);
}

#[test]
fn an_ignored_file_sorts_after_the_others() {
    let tree = Tree::from_paths(
        "r",
        [("d/a.edn", true), ("d/z.clj", false), ("d/m.clj", false)],
    );
    let layout = Layout::compute(&tree, &TILES);
    let names: Vec<&str> = tree.nodes[1]
        .files
        .iter()
        .map(|&f| tree.files[f].name.as_str())
        .collect();
    assert_eq!(names, ["m.clj", "z.clj", "a.edn"]);
    // Three files take two columns: m and z on the first row, a.edn first on the second row.
    let (m, z, a) = (layout.tiles[2], layout.tiles[1], layout.tiles[0]);
    assert_eq!(z, (m.0 + TILES.tw, m.1));
    assert_eq!(a, (m.0, m.1 + TILES.th));
}

#[test]
fn an_empty_tree_has_only_the_root_header() {
    let tree = Tree::from_paths("r", []);
    let layout = Layout::compute(&tree, &TILES);
    assert_eq!((layout.w, layout.h, layout.headers.len()), (TILES.tw, 1, 1));
    // CHARS: the root block is as wide as `r/`.
    let layout = Layout::compute(&tree, &CHARS);
    assert_eq!((layout.w, layout.h), (2, 1));
}

#[test]
#[ignore = "timing; run with --release"]
fn compute_timing() {
    let tree = Tree::from_index(&quayside());
    for cfg in [TILES, CHARS, LayoutConfig::CARDS] {
        let t0 = Instant::now();
        let runs = 50;
        for _ in 0..runs {
            std::hint::black_box(Layout::compute(&tree, &cfg));
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / runs as f64;
        eprintln!("Layout::compute on 2,585 files (tw {}): {ms:.2} ms", cfg.tw);
        assert!(ms < 50.0);
    }
}

#[test]
fn hidden_files_sort_last_before_ignored_files() {
    let tree = Tree::from_paths(
        "r",
        [
            ("d/.env", false),
            ("d/b.clj", false),
            ("d/a.clj", false),
            ("d/z.edn", true),
            ("d/.x", true),
        ],
    );
    let names: Vec<&str> = tree.nodes[1]
        .files
        .iter()
        .map(|&f| tree.files[f].name.as_str())
        .collect();
    assert_eq!(names, ["a.clj", "b.clj", ".env", "z.edn", ".x"]);
    assert!(tree.files[0].hidden && !tree.files[1].hidden);
}

#[test]
fn a_dot_directory_and_its_files_are_hidden() {
    let tree = Tree::from_paths(
        "r",
        [
            (".github/workflows/ci.yml", false),
            ("src/a.clj", false),
            (".github/workflows/.x", false),
        ],
    );
    // Inside a dot-directory every file is hidden, so the files sort by name alone.
    let names: Vec<&str> = tree.nodes[1]
        .files
        .iter()
        .map(|&f| tree.files[f].name.as_str())
        .collect();
    assert_eq!(names, [".x", "ci.yml"]);
    let hidden: Vec<(&str, bool)> = tree
        .nodes
        .iter()
        .map(|n| (n.path.as_str(), n.hidden))
        .collect();
    assert_eq!(
        hidden,
        [("", false), (".github/workflows", true), ("src", false)]
    );
    assert!(tree.files[0].hidden);
    assert!(minimap_core::layout::is_hidden("a/.b/c.txt"));
    assert!(!minimap_core::layout::is_hidden("a/b.c/d"));
}

/// The rounding variants of unit 027, and the adopted `CHARS`.
fn variants() -> Vec<(&'static str, LayoutConfig)> {
    let base = CHARS.unrounded();
    vec![
        ("CHARS", CHARS),
        (
            "Step(4)",
            LayoutConfig {
                quant: Quant::Step(4),
                ..base
            },
        ),
        (
            "Growth(1.25)",
            LayoutConfig {
                quant: Quant::Growth(1.25),
                ..base
            },
        ),
        (
            "Name",
            LayoutConfig {
                order: Order::Name,
                ..base
            },
        ),
    ]
}

#[test]
fn quant_rounds_up_to_steps() {
    let step: Vec<usize> = (0..=9).map(|x| Quant::Step(4).round(x)).collect();
    assert_eq!(step, [0, 4, 4, 4, 4, 8, 8, 8, 8, 12]);
    let mut values: Vec<usize> = (1..=24).map(|x| Quant::Growth(1.25).round(x)).collect();
    values.dedup();
    assert_eq!(values, [1, 2, 3, 4, 5, 7, 9, 12, 15, 19, 24]);
    assert_eq!(Quant::Growth(1.25).round(25), 30);
    assert_eq!(Quant::None.round(13), 13);
}

#[test]
fn every_variant_keeps_the_layout_invariants() {
    let index = quayside();
    let tree = Tree::from_index(&index);
    for (name, cfg) in variants() {
        let layout = Layout::compute(&tree, &cfg);
        common::assert_no_overlap(&tree, &layout);
        common::assert_containment(&tree, &layout);
        common::assert_deterministic(&index, &cfg);
        assert_eq!(layout.tiles.len(), 2585, "{name}");
    }
}

/// The quayside paths plus one more file in the directory `dir`. It sorts after the directory's other visible files.
fn with_file(index: &Index, dir: &str) -> Tree {
    let extra = if dir.is_empty() {
        "zzzz-new.clj".to_string()
    } else {
        format!("{dir}/zzzz-new.clj")
    };
    let paths = index
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.ignored))
        .chain([(extra.as_str(), false)]);
    Tree::from_paths(&index.root, paths)
}

/// The paths of `node` and its descendants.
fn subtree(tree: &Tree, node: usize) -> Vec<String> {
    let mut out = vec![];
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        out.push(tree.nodes[n].path.clone());
        stack.extend(&tree.nodes[n].kids);
    }
    out
}

/// Tiles and headers that differ between two layouts, outside the blocks with the given paths.
fn moved_outside(a: &common::ByPath, b: &common::ByPath, inside: &[String]) -> Vec<String> {
    let within = |p: &str| {
        inside
            .iter()
            .any(|d| d.is_empty() || p == d || p.starts_with(&format!("{d}/")))
    };
    let mut out: Vec<String> =
        a.0.iter()
            .filter(|(p, xy)| !within(p) && b.0.get(*p) != Some(*xy))
            .map(|(p, _)| p.clone())
            .collect();
    for h in a.1.difference(&b.1) {
        if !within(&h.0) {
            out.push(format!("{}/", h.0));
        }
    }
    out.sort();
    out
}

#[test]
fn a_file_added_where_the_grid_has_room_moves_nothing_outside_its_directory() {
    let index = quayside();
    let tree = Tree::from_index(&index);
    for (name, cfg) in variants() {
        if cfg.quant == Quant::None {
            continue;
        }
        let before = common::by_path(&tree, &Layout::compute(&tree, &cfg));
        let mut checked = 0;
        for node in &tree.nodes {
            let nf = node.files.len();
            if nf == 0 || cfg.quant.round(nf + 1) != cfg.quant.round(nf) {
                continue;
            }
            let t = with_file(&index, &node.path);
            let after = common::by_path(&t, &Layout::compute(&t, &cfg));
            let moved = moved_outside(&before, &after, std::slice::from_ref(&node.path));
            assert!(
                moved.is_empty(),
                "{name}: a file in {:?} moved {moved:?}",
                node.path
            );
            // The block itself keeps its place and size.
            let header = |b: &common::ByPath| b.1.iter().find(|h| h.0 == node.path).cloned();
            assert_eq!(header(&before), header(&after), "{name}: {:?}", node.path);
            checked += 1;
        }
        assert!(checked > 50, "{name}: only {checked} directories had room");
    }
}

/// For each directory with files, adds one file and finds the nearest block, from the directory up, that kept its size.
/// Returns, per addition, the depth of that block (0 when only the root absorbed it) and the tiles that moved outside it.
fn absorbed(index: &Index, cfg: &LayoutConfig) -> Vec<(usize, Vec<String>)> {
    let tree = Tree::from_index(index);
    let l0 = Layout::compute(&tree, cfg);
    let before = common::by_path(&tree, &l0);
    let size = |b: &common::ByPath, p: &str| b.1.iter().find(|h| h.0 == p).map(|h| (h.3, h.4));
    let mut parent = vec![usize::MAX; tree.nodes.len()];
    for (n, node) in tree.nodes.iter().enumerate() {
        for &k in &node.kids {
            parent[k] = n;
        }
    }
    let mut out = vec![];
    for (n, node) in tree.nodes.iter().enumerate() {
        if node.files.is_empty() || n == 0 {
            continue;
        }
        let t = with_file(index, &node.path);
        let after = common::by_path(&t, &Layout::compute(&t, cfg));
        let mut a = n;
        while a != 0 && size(&before, &tree.nodes[a].path) != size(&after, &tree.nodes[a].path) {
            a = parent[a];
        }
        let moved = if a == 0 {
            vec![]
        } else {
            moved_outside(&before, &after, &subtree(&tree, a))
        };
        out.push((tree.nodes[a].depth, moved));
    }
    out
}

#[test]
fn a_block_that_keeps_its_size_moves_nothing_outside_it() {
    // Locality: packing depends only on block sizes, so growth stops at the first block that keeps its size.
    let index = quayside();
    for (name, cfg) in [("unrounded", CHARS.unrounded()), ("CHARS", CHARS)] {
        for (depth, moved) in absorbed(&index, &cfg) {
            assert!(
                moved.is_empty(),
                "{name}: depth {depth} block let {moved:?} move"
            );
        }
    }
}

#[test]
fn rounding_moves_fewer_tiles_when_a_file_is_added() {
    // One more file in each quayside directory in turn: the mean count of tiles that move outside that directory.
    let index = quayside();
    let tree = Tree::from_index(&index);
    let mean = |cfg: &LayoutConfig| {
        let before = common::by_path(&tree, &Layout::compute(&tree, cfg));
        let dirs: Vec<&str> = tree
            .nodes
            .iter()
            .filter(|n| !n.files.is_empty())
            .map(|n| n.path.as_str())
            .collect();
        let moved: usize = dirs
            .iter()
            .map(|d| {
                let t = with_file(&index, d);
                let after = common::by_path(&t, &Layout::compute(&t, cfg));
                moved_outside(&before, &after, &[d.to_string()])
                    .iter()
                    .filter(|p| !p.ends_with('/'))
                    .count()
            })
            .sum();
        moved as f64 / dirs.len() as f64
    };
    let (base, rounded) = (mean(&CHARS.unrounded()), mean(&CHARS));
    eprintln!("tiles moved per added file: unrounded {base:.1}, CHARS {rounded:.1}");
    // Measured on 2026-10-19: 43.8 and 24.4.
    assert!(
        rounded < base * 0.7,
        "unrounded {base:.1}, CHARS {rounded:.1}"
    );
}

#[test]
fn name_order_keeps_siblings_in_name_order_when_one_grows() {
    let cfg = LayoutConfig {
        order: Order::Name,
        round: Round::ALL,
        ..CHARS.unrounded()
    };
    let order = |extra: usize| {
        let mut paths = vec!["a/1.clj".to_string()];
        paths.extend((0..10).map(|i| format!("b/{i}.clj")));
        paths.extend((0..3).map(|i| format!("c/{i}.clj")));
        paths.extend((0..extra).map(|i| format!("a/x{i}.clj")));
        let tree = Tree::from_paths("r", paths.iter().map(|p| (p.as_str(), false)));
        let layout = Layout::compute(&tree, &cfg);
        layout
            .headers
            .iter()
            .skip(1)
            .map(|h| tree.nodes[h.node].name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(order(0), ["a", "b", "c"]);
    assert_eq!(order(40), ["a", "b", "c"]);
    // Tallest-first puts the biggest block first.
    let tree = Tree::from_paths(
        "r",
        [
            ("a/1.clj", false),
            ("b/1.clj", false),
            ("b/2.clj", false),
            ("b/3.clj", false),
            ("b/4.clj", false),
            ("b/5.clj", false),
            ("b/6.clj", false),
            ("b/7.clj", false),
            ("b/8.clj", false),
            ("b/9.clj", false),
        ],
    );
    let layout = Layout::compute(&tree, &CHARS.unrounded());
    assert_eq!(tree.nodes[layout.headers[1].node].name, "b");
}

const CARDS: LayoutConfig = LayoutConfig::CARDS;

#[test]
fn a_card_is_21_cells_wide_and_as_tall_as_its_rounded_lines() {
    assert_eq!(CARDS.tile_size().0, 21);
    assert!(CARDS.line_scaled() && !CHARS.line_scaled() && !TILES.line_scaled());
    // The body is `ceil(step(lines) * LINE_PT / PY)` rows under a name row of 1 row, with LINE_PT 1 and PY 18.
    assert_eq!(minimap_core::layout::LINE_PT, 1.0);
    for (lines, step, body) in [
        (0, 0, 0),
        (1, 1, 1),
        (18, 19, 2),
        (53, 60, 4),
        (100, 118, 7),
        (300, 363, 21),
        (7329, 8279, 460),
    ] {
        assert_eq!(CARDS.quant.round(lines as usize), step, "{lines}");
        assert_eq!(CARDS.body_rows(lines), body, "{lines} lines");
        assert_eq!(CARDS.tile_rows(lines, false), 1 + body, "{lines} lines");
    }
    // Every quayside card follows the formula, and a file with 0 lines is only its name row.
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CARDS);
    for (f, file) in tree.files.iter().enumerate() {
        let lines = CARDS.tile_lines(file.lines, file.generated);
        let step = CARDS.quant.round(lines as usize) as f64;
        let body = (step * CARDS.line_pt / CARDS.py as f64).ceil() as i32;
        assert_eq!(layout.tile_h[f], 1 + body, "{}", file.path);
    }
    assert!(tree.files.iter().any(|f| f.lines == 0));
    // One-size layouts keep one tile height.
    let chars = Layout::compute(&tree, &CHARS);
    assert!(chars.tile_h.iter().all(|&h| h == 1));
}

#[test]
fn cards_and_headers_do_not_overlap() {
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CARDS);
    common::assert_no_card_or_header_overlap(&tree, &layout);
    common::assert_no_overlap(&tree, &layout);
    common::assert_containment(&tree, &layout);
}

#[test]
fn cards_give_the_same_layout_on_every_run() {
    let tree = Tree::from_index(&quayside());
    let a = common::by_path(&tree, &Layout::compute(&tree, &CARDS));
    let b = common::by_path(&tree, &Layout::compute(&tree, &CARDS));
    assert_eq!(a, b);
    common::assert_deterministic(&quayside(), &CARDS);
}

/// The quayside tree with one file's line count changed. The generated flags stay.
fn with_lines(index: &Index, path: &str, lines: u64) -> Tree {
    let mut index = index.clone();
    for f in index.files.iter_mut().filter(|f| f.path == path) {
        f.lines = lines;
    }
    Tree::from_index(&index)
}

#[test]
fn a_generated_card_is_as_tall_as_a_card_of_300_lines() {
    // Unit 041: quayside's package-lock.json (12,487 lines) is generated by its name, and a hand-written file keeps its true height.
    assert_eq!(CARDS.gen_lines, 300);
    assert_eq!((CHARS.gen_lines, TILES.gen_lines), (0, 0));
    let index = quayside();
    let tree = Tree::from_index(&index);
    let layout = Layout::compute(&tree, &CARDS);
    let lock = tree
        .files
        .iter()
        .position(|f| f.path == "package-lock.json")
        .unwrap();
    assert!(tree.files[lock].generated);
    assert_eq!(tree.files[lock].lines, 12_487);
    assert_eq!(layout.tile_h[lock], 1 + 21);
    assert_eq!(CARDS.tile_rows(300, true), CARDS.tile_rows(300, false));
    assert_eq!(CARDS.tile_rows(12_487, true), 22);
    assert_eq!(CARDS.tile_rows(12_487, false), 720);
    assert_eq!(CARDS.tile_rows(120, true), CARDS.tile_rows(120, false));
    // Every other file is hand-written, and the long ones keep their heights.
    let long: Vec<_> = (0..tree.files.len())
        .filter(|&f| f != lock && tree.files[f].lines > 300)
        .collect();
    assert!(long.len() > 20, "{}", long.len());
    for f in long {
        assert!(!tree.files[f].generated, "{}", tree.files[f].path);
        assert_eq!(
            layout.tile_h[f],
            CARDS.tile_rows(tree.files[f].lines, false)
        );
        assert!(layout.tile_h[f] >= layout.tile_h[lock]);
    }
    // The root's grid was as tall as the lock file; with the cap the map is much shorter.
    let uncapped = Layout::compute(
        &tree,
        &LayoutConfig {
            gen_lines: 0,
            ..CARDS
        },
    );
    eprintln!(
        "quayside: {} x {} cells with the cap, {} x {} without",
        layout.w, layout.h, uncapped.w, uncapped.h
    );
    assert!(layout.h < uncapped.h);
}

#[test]
fn a_generated_file_that_grows_past_the_cap_moves_no_card() {
    let index = quayside();
    let tree = Tree::from_index(&index);
    let before = common::by_path(&tree, &Layout::compute(&tree, &CARDS));
    for lines in [301, 400, 4_000, 40_000] {
        let t = with_lines(&index, "package-lock.json", lines);
        let after = common::by_path(&t, &Layout::compute(&t, &CARDS));
        assert_eq!(before, after, "package-lock.json at {lines} lines");
    }
    // A generated file under the cap is line-scaled like any file: past its step, it grows.
    let t = with_lines(&index, "package-lock.json", 100);
    let small = Layout::compute(&t, &CARDS);
    let lock = t
        .files
        .iter()
        .position(|f| f.path == "package-lock.json")
        .unwrap();
    assert_eq!(small.tile_h[lock], CARDS.tile_rows(100, false));
}

#[test]
fn a_file_that_grows_within_its_step_moves_no_card() {
    let index = quayside();
    let tree = Tree::from_index(&index);
    let before = common::by_path(&tree, &Layout::compute(&tree, &CARDS));
    let mut checked = 0;
    for f in index.files.iter().step_by(23) {
        let step = CARDS.quant.round(f.lines as usize) as u64;
        if f.lines == 0 || step == f.lines {
            continue;
        }
        let t = with_lines(&index, &f.path, step);
        let after = common::by_path(&t, &Layout::compute(&t, &CARDS));
        assert_eq!(before, after, "{} grew from {} to {step}", f.path, f.lines);
        checked += 1;
    }
    assert!(checked > 50, "{checked}");
}

#[test]
fn a_file_that_grows_past_its_step_moves_nothing_outside_its_rounded_block_while_its_size_holds() {
    // A block at depth 1 or 2 rounds its height. Its width is not rounded, so the check needs both to hold.
    let index = quayside();
    let tree = Tree::from_index(&index);
    let before = common::by_path(&tree, &Layout::compute(&tree, &CARDS));
    let mut parent = vec![usize::MAX; tree.nodes.len()];
    for (n, node) in tree.nodes.iter().enumerate() {
        for &k in &node.kids {
            parent[k] = n;
        }
    }
    let size = |b: &common::ByPath, p: &str| b.1.iter().find(|h| h.0 == p).map(|h| (h.3, h.4));
    let (mut held, mut moved_block, mut width_only) = (0, 0, 0);
    for (f, file) in tree.files.iter().enumerate().step_by(11) {
        let mut a = file.node;
        while tree.nodes[a].depth > 2 {
            a = parent[a];
        }
        if tree.nodes[a].depth == 0 {
            continue;
        }
        let grown = CARDS.quant.round(file.lines as usize) as u64 + 1;
        let t = with_lines(&index, &tree.files[f].path, grown);
        let after = common::by_path(&t, &Layout::compute(&t, &CARDS));
        let path = &tree.nodes[a].path;
        let (b, c) = (size(&before, path).unwrap(), size(&after, path).unwrap());
        if b != c {
            moved_block += 1;
            width_only += usize::from(b.1 == c.1);
            continue;
        }
        let moved = moved_outside(&before, &after, &subtree(&tree, a));
        assert!(
            moved.is_empty(),
            "{} grew past its step and moved {moved:?}",
            file.path
        );
        held += 1;
    }
    eprintln!(
        "a file past its step: its rounded block held {held} times and changed {moved_block} times ({width_only} by width only)"
    );
    assert!(held > 100, "{held}");
}

#[test]
fn files_flow_down_columns_in_name_order() {
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CARDS);
    let mut multi = 0;
    for node in &tree.nodes {
        // Flow order is column by column, top to bottom.
        let mut at: Vec<(i32, i32)> = node.files.iter().map(|&f| layout.tiles[f]).collect();
        let flow = at.clone();
        at.sort();
        assert_eq!(at, flow, "{:?} is not in column order", node.path);
        // Hidden files come after the others, so hiding them leaves every other card in place.
        let hidden: Vec<bool> = node.files.iter().map(|&f| tree.files[f].hidden).collect();
        assert!(hidden.windows(2).all(|w| w[0] <= w[1]), "{:?}", node.path);
        let cols: std::collections::HashSet<i32> = flow.iter().map(|t| t.0).collect();
        multi += usize::from(cols.len() > 1);
    }
    assert!(multi > 50, "{multi} directories use more than one column");
    // Inside a column, each card starts one gap row below the card above it.
    for node in &tree.nodes {
        for w in node.files.windows(2) {
            let (a, b) = (layout.tiles[w[0]], layout.tiles[w[1]]);
            if a.0 == b.0 {
                assert_eq!(b.1, a.1 + layout.tile_h[w[0]] + 1, "{:?}", node.path);
            }
        }
    }
}

#[test]
fn the_card_layout_takes_no_hidden_or_selection_input() {
    // The layout is a function of the tree: the same tree gives the same rects, whatever a front end hides or selects.
    let index = quayside();
    let tree = Tree::from_index(&index);
    let a = Layout::compute(&tree, &CARDS);
    let mut index2 = index.clone();
    for f in &mut index2.files {
        f.lines = f.lines.max(1);
    }
    let b = Layout::compute(&Tree::from_index(&index2), &CARDS);
    // Only line counts change card heights.
    assert_ne!(a.tile_h, b.tile_h);
    let a2 = Layout::compute(&tree, &CARDS);
    assert_eq!((a.tiles, a.tile_h), (a2.tiles, a2.tile_h));
}

/// Per node: the count of files below it, at every depth (unit 062), summed from the tree.
fn files_below(tree: &Tree) -> Vec<usize> {
    let mut count = vec![0; tree.nodes.len()];
    // Nodes are in preorder, so each child comes after its parent.
    for n in (0..tree.nodes.len()).rev() {
        let node = &tree.nodes[n];
        count[n] = node.files.len() + node.kids.iter().map(|&k| count[k]).sum::<usize>();
    }
    count
}

/// The band rows of unit 062 for a count: the least `r` with `r * r >= count`, rounded up to a step.
fn count_band(count: usize) -> i32 {
    let n = count.clamp(1, NAME_CAP_FILES);
    let r = (1..).find(|r| r * r >= n).unwrap();
    CARDS.quant.round(r) as i32
}

#[test]
fn a_block_band_holds_its_name_sized_by_the_count_of_files_below() {
    use minimap_core::layout::{BAND_K, NAME_MIN};
    assert_eq!(BAND_K * NAME_MIN, 1.0);
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CARDS);
    let count = files_below(&tree);
    assert_eq!(count[0], 2585);
    let mut sized = 0;
    for h in &layout.headers {
        let path = &tree.nodes[h.node].path;
        let size = NAME_MIN * 18.0 * (count[h.node] as f64).sqrt();
        assert!((layout.name_size[h.node] - size).abs() < 1e-9, "{path:?}");
        assert_eq!(h.band, count_band(count[h.node]), "{path:?}");
        // The name's line fits its band.
        assert!(BAND_K * size <= f64::from(h.band) * 18.0 + 1e-9, "{path:?}");
        sized += usize::from(count[h.node] > 1);
    }
    eprintln!(
        "{sized} of {} names are larger than the least size",
        layout.headers.len()
    );
    assert!(sized > 300, "{sized}");
    // The root gets the largest name: 14.4 x sqrt(2,585) points, in a band of 60 rows.
    assert!((layout.name_size[0] - 14.4 * 2585f64.sqrt()).abs() < 1e-9);
    assert_eq!(layout.headers[0].band, 60);
    assert!(layout.name_size.iter().all(|&s| s <= layout.name_size[0]));
    // One-size layouts keep one header row.
    let chars = Layout::compute(&tree, &CHARS);
    assert!(chars.headers.iter().all(|h| h.band == 1));
    assert!(chars.name_size.iter().all(|&s| s == 0.0));
}

#[test]
fn a_larger_count_has_a_name_at_least_as_large_and_no_child_outgrows_its_parent() {
    // The size and the band follow the count alone: the block's width and the name's length do not change them.
    let mut last = (0.0, 0);
    for files in 0..=12_000 {
        let got = CARDS.name_band(22, 4, files);
        assert!(got.0 >= last.0 && got.1 >= last.1, "{files} files");
        assert_eq!(got, CARDS.name_band(3000, 40, files), "{files} files");
        last = got;
    }
    // One file, or none, gets the least size in one row. A square count fills its rows exactly.
    assert_eq!(CARDS.name_band(22, 4, 0), (14.4, 1));
    assert_eq!(CARDS.name_band(22, 4, 1), (14.4, 1));
    for (files, rows) in [
        (2, 2),
        (4, 2),
        (5, 3),
        (9, 3),
        (10, 4),
        (16, 4),
        (17, 5),
        (25, 5),
        (26, 7),
        (49, 7),
        (50, 9),
    ] {
        assert_eq!(CARDS.name_band(22, 4, files).1, rows, "{files} files");
    }
    assert!((CARDS.name_band(22, 4, 100).0 - 144.0).abs() < 1e-9);
    // The cap: from 10,000 files the size is 1,440 points, 100 times the least, in a band of 100 rows rounded up to 118.
    let cap = CARDS.name_band(22, 4, NAME_CAP_FILES);
    assert!((cap.0 - 1440.0).abs() < 1e-9);
    assert_eq!(cap.1, 118);
    assert_eq!(CARDS.name_band(22, 4, 1_000_000), cap);
    // On quayside no child's name is larger than its parent's. With the width rule of unit 040, some were.
    let tree = Tree::from_index(&quayside());
    let larger_kids = |names: Names| {
        let layout = Layout::compute(&tree, &LayoutConfig { names, ..CARDS });
        tree.nodes
            .iter()
            .enumerate()
            .flat_map(|(n, node)| node.kids.iter().map(move |&k| (n, k)))
            .filter(|&(n, k)| layout.name_size[k] > layout.name_size[n])
            .count()
    };
    assert_eq!(larger_kids(Names::Count), 0);
    assert_eq!(larger_kids(Names::Width(NAME_SHARE)), 34);
}

#[test]
fn a_block_keeps_its_band_while_its_count_stays_inside_its_step() {
    // The band is a function of the count of files below the block. Its width no longer changes it (unit 040 did).
    let index = quayside();
    let tree = Tree::from_index(&index);
    let a = Layout::compute(&tree, &CARDS);
    let count = files_below(&tree);
    let header = |l: &Layout| {
        let mut v = vec![None; tree.nodes.len()];
        for h in &l.headers {
            v[h.node] = Some(*h);
        }
        v.into_iter().map(Option::unwrap).collect::<Vec<_>>()
    };
    let before = header(&a);
    let (mut held, mut held_new_width, mut passed) = (0, 0, 0);
    // Add a file to every 7th directory that has files of its own, so the tree keeps its nodes.
    for (n, node) in tree.nodes.iter().enumerate().step_by(7) {
        if node.files.is_empty() {
            continue;
        }
        let mut grown = index.clone();
        let mut f = grown.files[node.files[0]].clone();
        f.path = format!("{}/zz_added.clj", node.path);
        grown.files.push(f);
        let t = Tree::from_index(&grown);
        assert_eq!(t.nodes.len(), tree.nodes.len());
        let after = header(&Layout::compute(&t, &CARDS));
        for (m, cm) in files_below(&t).into_iter().enumerate() {
            let (hb, ha) = (before[m], after[m]);
            if count_band(cm) == count_band(count[m]) {
                assert_eq!(
                    ha.band, hb.band,
                    "{:?} after a file in {:?}",
                    tree.nodes[m].path, node.path
                );
                held += 1;
                held_new_width += usize::from(ha.w != hb.w);
            } else {
                assert_ne!(ha.band, hb.band);
                passed += 1;
            }
        }
        let _ = n;
    }
    eprintln!(
        "blocks that kept their band: {held} ({held_new_width} of them with a new width); blocks whose count passed a step: {passed}"
    );
    assert!(
        held > 10_000 && held_new_width > 10 && passed > 10,
        "{held} {held_new_width} {passed}"
    );
    // Without name bands every block has one header row.
    let flat = Layout::compute(
        &tree,
        &LayoutConfig {
            names: Names::None,
            ..CARDS
        },
    );
    assert!(flat.headers.iter().all(|h| h.band == 1));
}

#[test]
fn count_bands_move_blocks_and_never_a_card_inside_its_grid() {
    // Unit 062 changes the bands from the width rule of unit 040 to the count rule. This proves which tiles move on quayside.
    let tree = Tree::from_index(&quayside());
    let width = Layout::compute(
        &tree,
        &LayoutConfig {
            names: Names::Width(NAME_SHARE),
            ..CARDS
        },
    );
    let count = Layout::compute(&tree, &CARDS);
    let by_node = |l: &Layout| {
        let mut v = l.headers.clone();
        v.sort_by_key(|h| h.node);
        v
    };
    let (hw, hc) = (by_node(&width), by_node(&count));
    // 1. Every card keeps its place in its directory's file grid: its offset from the block's left and from the band's bottom.
    for (f, file) in tree.files.iter().enumerate() {
        let off = |l: &Layout, h: &[minimap_core::layout::Header]| {
            let b = h[file.node];
            (l.tiles[f].0 - b.x, l.tiles[f].1 - b.y - b.band)
        };
        assert_eq!(off(&width, &hw), off(&count, &hc), "{}", file.path);
    }
    // 2. A block whose band and whose descendants' bands held keeps its size and moves as a whole:
    //    its cards and child blocks keep their offsets from its top-left.
    let mut held = vec![false; tree.nodes.len()];
    for n in (0..tree.nodes.len()).rev() {
        held[n] = hw[n].band == hc[n].band && tree.nodes[n].kids.iter().all(|&k| held[k]);
    }
    let mut whole = 0;
    for n in (0..tree.nodes.len()).filter(|&n| held[n]) {
        assert_eq!(
            (hw[n].w, hw[n].h),
            (hc[n].w, hc[n].h),
            "{:?}",
            tree.nodes[n].path
        );
        for m in subtree_nodes(&tree, n) {
            let rel = |h: &[minimap_core::layout::Header]| (h[m].x - h[n].x, h[m].y - h[n].y);
            assert_eq!(
                rel(&hw),
                rel(&hc),
                "{:?} in {:?}",
                tree.nodes[m].path,
                tree.nodes[n].path
            );
            for &f in &tree.nodes[m].files {
                let rel = |l: &Layout, h: &[minimap_core::layout::Header]| {
                    (l.tiles[f].0 - h[n].x, l.tiles[f].1 - h[n].y)
                };
                assert_eq!(rel(&width, &hw), rel(&count, &hc), "{}", tree.files[f].path);
            }
        }
        whole += 1;
    }
    // 3. The counts: the root's band goes from 118 to 60 rows, so every card moves on the map.
    let changed = (0..tree.nodes.len()).filter(|&n| hw[n].band != hc[n].band);
    let (grew, shrank) = changed.fold((0, 0), |(g, s), n| {
        if hc[n].band > hw[n].band {
            (g + 1, s)
        } else {
            (g, s + 1)
        }
    });
    let resized = (0..tree.nodes.len())
        .filter(|&n| (hw[n].w, hw[n].h) != (hc[n].w, hc[n].h))
        .count();
    let moved = (0..tree.files.len())
        .filter(|&f| width.tiles[f] != count.tiles[f])
        .count();
    let lift = hc[0].band - hw[0].band;
    let lifted = (0..tree.files.len())
        .filter(|&f| {
            (
                count.tiles[f].0 - width.tiles[f].0,
                count.tiles[f].1 - width.tiles[f].1,
            ) == (0, lift)
        })
        .count();
    eprintln!(
        "bands: {grew} grew, {shrank} shrank of {}; blocks resized: {resized}; blocks that moved as a whole: {whole}; \
         cards moved: {moved} of {}, {lifted} of them only by the root band's {lift} rows; map {} x {} cells, was {} x {}",
        tree.nodes.len(),
        tree.files.len(),
        count.w,
        count.h,
        width.w,
        width.h
    );
    // Districts that kept their place under the root's band.
    let districts: Vec<usize> = tree.nodes[0].kids.clone();
    let kept = districts
        .iter()
        .filter(|&&d| (hw[d].x, hw[d].y - hw[0].band) == (hc[d].x, hc[d].y - hc[0].band))
        .count();
    eprintln!(
        "districts that kept their place under the root's band: {kept} of {}",
        districts.len()
    );
    assert_eq!((hw[0].band, hc[0].band), (118, 60));
    assert_eq!((grew, shrank, resized, whole), (228, 90, 275, 119));
    assert_eq!((moved, lifted, kept), (tree.files.len(), 15, 1));
}

/// The node and every node below it.
fn subtree_nodes(tree: &Tree, node: usize) -> Vec<usize> {
    let mut out = vec![];
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        out.push(n);
        stack.extend(&tree.nodes[n].kids);
    }
    out
}

#[test]
fn the_cards_layout_of_quayside() {
    let tree = Tree::from_index(&quayside());
    let layout = Layout::compute(&tree, &CARDS);
    // x, y, and height in cells, then the line count.
    let first: Vec<String> = (0..20)
        .map(|i| {
            let (x, y) = layout.tiles[i];
            let f = &tree.files[i];
            format!(
                "{x:>5} {y:>5} {:>4}  {} ({})",
                layout.tile_h[i], f.path, f.lines
            )
        })
        .collect();
    let rows: i64 = layout.tile_h.iter().map(|&h| i64::from(h)).sum();
    // The first headers: x, y, w, h, and band in cells, then the name size in points.
    let heads: Vec<String> = layout.headers[..8]
        .iter()
        .map(|h| {
            format!(
                "{:>5} {:>5} {:>5} {:>5} {:>4} {:>7.1}  {}/",
                h.x, h.y, h.w, h.h, h.band, layout.name_size[h.node], tree.nodes[h.node].name
            )
        })
        .collect();
    insta::assert_snapshot!(format!(
        "{} x {} cells, {} cards, {} headers, {} card rows, tallest {}\n{}\n{}",
        layout.w,
        layout.h,
        layout.tiles.len(),
        layout.headers.len(),
        rows,
        layout.tile_h.iter().max().unwrap(),
        first.join("\n"),
        heads.join("\n")
    ));
}
