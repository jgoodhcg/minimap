//! Real-repo tests. They read `MINIMAP_TARGET_REPO` and `MINIMAP_CONFIG`, and write only under `.private/`.
//!
//! ```sh
//! MINIMAP_TARGET_REPO=<path> MINIMAP_CONFIG=.private/minimap.toml cargo test --workspace --locked -- --ignored
//! ```

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use minimap_core::config::expand_home;
use minimap_core::index::{exclude_set, repo_root};
use minimap_core::{Config, Index, IndexOptions, Layout, LayoutConfig, Tree};

/// Resolves a relative path against the workspace root, because `cargo test` runs in the crate directory.
fn workspace_path(p: &str) -> PathBuf {
    let p = expand_home(p);
    if p.is_absolute() {
        p
    } else {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(p)
    }
}

fn target() -> (PathBuf, Vec<String>) {
    let repo = std::env::var("MINIMAP_TARGET_REPO").expect("set MINIMAP_TARGET_REPO");
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let excludes = Config::load(config.as_deref()).unwrap().excludes_for(&root);
    (root, excludes)
}

/// The target's root and its index options from the config: the excludes and the generated-file rules (unit `041`).
fn target_options() -> (PathBuf, IndexOptions) {
    let repo = std::env::var("MINIMAP_TARGET_REPO").expect("set MINIMAP_TARGET_REPO");
    let root = repo_root(&workspace_path(&repo)).unwrap();
    let config = std::env::var("MINIMAP_CONFIG")
        .ok()
        .map(|c| workspace_path(&c));
    let config = Config::load(config.as_deref()).unwrap();
    let options = IndexOptions {
        excludes: config.excludes_for(&root),
        generated: config.generated_for(&root),
    };
    (root, options)
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn generated_files_on_the_target() {
    // Unit 041: the marks by source, and the CARDS map with and without the cap. It prints numbers only, no paths.
    use minimap_core::generated::{Marks, is_attributes_file, is_lock_file};
    let (root, options) = target_options();
    let index = Index::from_repo(&root, &options).unwrap();
    let by_name = index.files.iter().filter(|f| is_lock_file(&f.path)).count();
    let mut attrs = Marks::new(&[]).unwrap();
    let mut attr_files = 0;
    for f in index.files.iter().filter(|f| is_attributes_file(&f.path)) {
        if let Ok(text) = std::fs::read_to_string(root.join(&f.path)) {
            attrs.add_attributes(&f.path, &text);
            attr_files += 1;
        }
    }
    let with_attrs = index
        .files
        .iter()
        .filter(|f| attrs.is_generated(&f.path))
        .count();
    let generated: Vec<_> = index.files.iter().filter(|f| f.generated).collect();
    let lines: u64 = generated.iter().map(|f| f.lines).sum();
    let all: u64 = index.files.iter().map(|f| f.lines).sum();
    eprintln!(
        "generated files: {by_name} by the built-in names; {with_attrs} with {attr_files} .gitattributes files; \
         {} with {} config rules; {lines} of {all} lines ({:.1} %)",
        generated.len(),
        options.generated.len(),
        lines as f64 * 100.0 / all as f64
    );
    let tree = Tree::from_index(&index);
    let capped = Layout::compute(&tree, &LayoutConfig::CARDS);
    let uncapped = Layout::compute(
        &tree,
        &LayoutConfig {
            gen_lines: 0,
            ..LayoutConfig::CARDS
        },
    );
    let cap_rows = LayoutConfig::CARDS.tile_rows(LayoutConfig::CARDS.gen_lines, false);
    for (label, l) in [("capped", &capped), ("uncapped", &uncapped)] {
        let w = i64::from(l.cfg.tile_size().0);
        let area: i64 = l.tile_h.iter().map(|&h| w * i64::from(h)).sum();
        let tallest_gen = (0..tree.files.len())
            .filter(|&f| tree.files[f].generated)
            .map(|f| l.tile_h[f])
            .max()
            .unwrap_or(0);
        let over = (0..tree.files.len())
            .filter(|&f| tree.files[f].generated && l.tile_h[f] > cap_rows)
            .count();
        eprintln!(
            "{label}: {} x {} cells; fill {:.1} %; the tallest generated card {} points; {over} generated cards over {} points",
            l.w,
            l.h,
            area as f64 * 100.0 / (f64::from(l.w) * f64::from(l.h)),
            tallest_gen * 18,
            cap_rows * 18
        );
    }
    for f in 0..tree.files.len() {
        if tree.files[f].generated {
            assert!(capped.tile_h[f] <= cap_rows, "{}", tree.files[f].path);
        }
    }
    common::assert_no_card_or_header_overlap(&tree, &capped);
    common::assert_containment(&tree, &capped);
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn index_matches_git_ls_files_minus_excludes() {
    let (root, excludes) = target();
    // Tracked, then untracked and not ignored (unit 034). A nested repository shows as `dir/`; the index skips it.
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let mut listed: Vec<String> = out
        .stdout
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty() && !p.ends_with(b"/"))
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect();
    listed.sort();
    listed.dedup();
    let set = exclude_set(&excludes).unwrap();
    let kept: Vec<&String> = listed
        .iter()
        .filter(|p| !set.is_match(p.as_str()))
        .collect();

    let t0 = Instant::now();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "real repo index: {} files ({} untracked), {} excluded, {ms:.0} ms",
        index.files.len(),
        index.files.iter().filter(|f| f.untracked).count(),
        index.excluded
    );

    let paths: Vec<&String> = index.files.iter().map(|f| &f.path).collect();
    assert_eq!(paths, kept);
    assert_eq!(index.excluded, listed.len() - kept.len());
    assert!(
        index.files.iter().all(|f| !set.is_match(&f.path)),
        "an indexed path matches an exclude glob"
    );
}

/// The re-index that the watcher runs (unit 034), warm: the whole working tree again, reading only changed files.
/// A full index from nothing is printed too.
#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn a_warm_reindex_is_fast() {
    let (root, excludes) = target();
    let options = IndexOptions {
        excludes,
        ..Default::default()
    };
    let repo = minimap_core::index::open(&root).unwrap();
    let first = minimap_core::index::scan(&repo, &root, &options, None).unwrap();
    let time = |prev: Option<&minimap_core::index::Scan>| {
        (0..5)
            .map(|_| {
                let t0 = Instant::now();
                let s = minimap_core::index::scan(&repo, &root, &options, prev).unwrap();
                assert_eq!(s.index, first.index);
                t0.elapsed().as_secs_f64() * 1000.0
            })
            .fold(f64::INFINITY, f64::min)
    };
    let full = time(None);
    let rescan = time(Some(&first));
    eprintln!("real repo re-index, warm: {rescan:.0} ms; full index from nothing: {full:.0} ms");
    assert!(rescan < 150.0, "re-index took {rescan:.0} ms");
}

/// A watcher on the target sends nothing while nothing changes (unit 034). It reads the target only.
#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn a_watcher_on_the_target_stays_quiet() {
    let (root, excludes) = target();
    let options = IndexOptions {
        excludes,
        ..Default::default()
    };
    let index = Index::from_repo(&root, &options).unwrap();
    let w = minimap_core::watch::Watcher::spawn(root, options, index).unwrap();
    let t0 = Instant::now();
    while t0.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let (update, error) = w.poll();
        assert_eq!(error, None);
        assert!(
            update.is_none(),
            "an update with no changes: {:?}",
            update.map(|u| u.changed)
        );
    }
    eprintln!(
        "real repo watcher: {} re-indexes in 10 s, no update",
        w.scans()
    );
}

/// The golden files come from `minimap index` and `mockups/export-layout.mjs` (units 003 and 019).
fn assert_golden(golden_file: &str, cfg: &LayoutConfig) {
    let files =
        std::fs::read_to_string(workspace_path(".private/fixtures/target-files.json")).unwrap();
    let golden = std::fs::read_to_string(workspace_path(golden_file)).unwrap();
    let tree = Tree::from_index(&Index::from_json(&files).unwrap());
    common::assert_parity(
        &tree,
        &Layout::compute(&tree, cfg),
        &serde_json::from_str(&golden).unwrap(),
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn layout_matches_the_golden_file() {
    assert_golden(".private/fixtures/target-layout.json", &LayoutConfig::TILES);
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn chars_layout_matches_the_golden_file() {
    // The golden file comes from the v5 mockup, which does not round sizes (unit 027).
    assert_golden(
        ".private/fixtures/target-chars-layout.json",
        &LayoutConfig::CHARS.unrounded(),
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn layout_invariants_on_the_live_index() {
    let (root, excludes) = target();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    let tree = Tree::from_index(&index);
    for cfg in [LayoutConfig::TILES, LayoutConfig::CHARS] {
        let t0 = Instant::now();
        let layout = Layout::compute(&tree, &cfg);
        eprintln!(
            "real repo layout (tw {}): {} tiles, {} headers, {} x {} cells, {:.1} ms",
            cfg.tw,
            layout.tiles.len(),
            layout.headers.len(),
            layout.w,
            layout.h,
            t0.elapsed().as_secs_f64() * 1000.0
        );
        common::assert_no_overlap(&tree, &layout);
        common::assert_containment(&tree, &layout);
        common::assert_deterministic(&index, &cfg);
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn cards_layout_on_the_live_index() {
    let (root, excludes) = target();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    let tree = Tree::from_index(&index);
    let t0 = Instant::now();
    let layout = Layout::compute(&tree, &LayoutConfig::CARDS);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    // One card per index file, in cells of 8 x 18 points at zoom 1.
    assert_eq!(layout.tiles.len(), index.files.len());
    eprintln!(
        "real repo CARDS layout: {} cards, {} headers, {} x {} cells, {} x {} points, {ms:.1} ms",
        layout.tiles.len(),
        layout.headers.len(),
        layout.w,
        layout.h,
        layout.w * 8,
        layout.h * 18
    );
    // Line-scaled cards (unit 039) against one card size: the summed card area, the fill, and the tallest card.
    let one = Layout::compute(
        &tree,
        &LayoutConfig {
            line_pt: 0.0,
            ..LayoutConfig::CARDS
        },
    );
    let area = |l: &Layout| -> i64 {
        let w = i64::from(l.cfg.tile_size().0);
        l.tile_h.iter().map(|&h| w * i64::from(h)).sum()
    };
    let fill = |l: &Layout| area(l) as f64 / (f64::from(l.w) * f64::from(l.h));
    let tallest = (0..layout.tiles.len())
        .max_by_key(|&f| layout.tile_h[f])
        .unwrap();
    let mut heights: Vec<i32> = layout.tile_h.clone();
    heights.sort_unstable();
    eprintln!(
        "line-scaled cards: card area {:.2} times one size's; fill {:.1} % (one size {:.1} %); one size {} x {} cells; \
         median card {} points, tallest {} points ({} lines)",
        area(&layout) as f64 / area(&one) as f64,
        fill(&layout) * 100.0,
        fill(&one) * 100.0,
        one.w,
        one.h,
        heights[heights.len() / 2] * 18,
        layout.tile_h[tallest] * 18,
        tree.files[tallest].lines
    );
    for (f, file) in tree.files.iter().enumerate() {
        assert_eq!(
            layout.tile_h[f],
            LayoutConfig::CARDS.tile_rows(file.lines, file.generated)
        );
    }
    common::assert_no_card_or_header_overlap(&tree, &layout);
    common::assert_no_overlap(&tree, &layout);
    common::assert_containment(&tree, &layout);
    common::assert_deterministic(&index, &LayoutConfig::CARDS);
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn count_name_bands_on_the_target() {
    // Unit 062: names sized by the count of files below each directory, against the width rule of unit 040.
    // It prints numbers only, no paths.
    use minimap_core::layout::{NAME_ADVANCE, NAME_SHARE, Names};
    let (root, options) = target_options();
    let tree = Tree::from_index(&Index::from_repo(&root, &options).unwrap());
    let mut count = vec![0usize; tree.nodes.len()];
    for n in (0..tree.nodes.len()).rev() {
        let node = &tree.nodes[n];
        count[n] = node.files.len() + node.kids.iter().map(|&k| count[k]).sum::<usize>();
    }
    let cards = LayoutConfig::CARDS;
    let by_count = Layout::compute(&tree, &cards);
    let by_width = Layout::compute(
        &tree,
        &LayoutConfig {
            names: Names::Width(NAME_SHARE),
            ..cards
        },
    );
    let by_node = |l: &Layout| {
        let mut v = l.headers.clone();
        v.sort_by_key(|h| h.node);
        v
    };
    let (hc, hw) = (by_node(&by_count), by_node(&by_width));
    for (n, node) in tree.nodes.iter().enumerate() {
        let r = (1..).find(|r| r * r >= count[n].max(1)).unwrap();
        assert_eq!(hc[n].band, cards.quant.round(r) as i32);
        for &k in &node.kids {
            assert!(by_count.name_size[k] <= by_count.name_size[n]);
        }
    }
    // Every card keeps its place in its directory's grid; only blocks move.
    for (f, file) in tree.files.iter().enumerate() {
        let off = |l: &Layout, h: &[minimap_core::layout::Header]| {
            let b = h[file.node];
            (l.tiles[f].0 - b.x, l.tiles[f].1 - b.y - b.band)
        };
        assert_eq!(off(&by_width, &hw), off(&by_count, &hc));
    }
    common::assert_no_card_or_header_overlap(&tree, &by_count);
    common::assert_containment(&tree, &by_count);
    common::assert_deterministic(&Index::from_repo(&root, &options).unwrap(), &cards);
    let stats = |l: &Layout, h: &[minimap_core::layout::Header]| {
        let bands: i64 = h.iter().map(|h| i64::from(h.band) * i64::from(h.w)).sum();
        let larger_kids = tree
            .nodes
            .iter()
            .enumerate()
            .flat_map(|(n, node)| node.kids.iter().map(move |&k| (n, k)))
            .filter(|&(n, k)| l.name_size[k] > l.name_size[n])
            .count();
        let cut = h
            .iter()
            .filter(|h| {
                let chars = tree.nodes[h.node].name.chars().count() + 1;
                chars as f64 * NAME_ADVANCE * l.name_size[h.node] > f64::from(h.w) * 8.0
            })
            .count();
        format!(
            "{} x {} cells, bands {:.1} % of the map, root band {} rows, {larger_kids} names larger than their parent's, {cut} names cut",
            l.w,
            l.h,
            100.0 * bands as f64 / (f64::from(l.w) * f64::from(l.h)),
            h[0].band
        )
    };
    let moved = (0..tree.files.len())
        .filter(|&f| by_width.tiles[f] != by_count.tiles[f])
        .count();
    let changed = (0..tree.nodes.len())
        .filter(|&n| hw[n].band != hc[n].band)
        .count();
    let districts = &tree.nodes[0].kids;
    let kept = districts
        .iter()
        .filter(|&&d| (hw[d].x, hw[d].y - hw[0].band) == (hc[d].x, hc[d].y - hc[0].band))
        .count();
    eprintln!("width bands (unit 040): {}", stats(&by_width, &hw));
    eprintln!("count bands (unit 062): {}", stats(&by_count, &hc));
    eprintln!(
        "{changed} of {} bands changed; {moved} of {} cards moved; {kept} of {} districts kept their place under the root band",
        tree.nodes.len(),
        tree.files.len(),
        districts.len()
    );
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn the_loader_computes_every_file_of_the_target() {
    use minimap_core::details::{Details, Loader, compute_file};
    use minimap_core::syntax::too_long;
    let (root, excludes) = target();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    let paths: Vec<String> = index.files.iter().map(|f| f.path.clone()).collect();
    let t0 = Instant::now();
    let mut loader = Loader::spawn(Details::repo(root.clone()), || {});
    loader.request(paths.clone());
    let mut got = std::collections::HashMap::new();
    while got.len() < paths.len() {
        got.extend(loader.poll());
        assert!(
            t0.elapsed().as_secs() < 300,
            "{} of {} files",
            got.len(),
            paths.len()
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let total = t0.elapsed();
    let workers =
        std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1));
    let (mut shapes, mut outlined, mut generic) = (0, 0, 0);
    for f in &index.files {
        let d = &got[&f.path];
        if f.binary || f.missing {
            continue;
        }
        let text =
            String::from_utf8_lossy(&std::fs::read(root.join(&f.path)).unwrap()).into_owned();
        if !too_long(&text) {
            assert!(d.shape.is_some(), "{} has no shape", f.path);
        }
        shapes += usize::from(d.shape.is_some());
        outlined += usize::from(!d.outline.is_empty());
        generic += usize::from(
            !d.outline.is_empty() && minimap_core::outline::language_for(&f.path).is_none(),
        );
    }
    eprintln!(
        "loader: {} files in {:.0} ms on {workers} workers; {shapes} shapes, {outlined} outlines ({generic} from grammars)",
        paths.len(),
        total.as_secs_f64() * 1000.0
    );
    // The slowest files, timed one by one on this thread.
    let mut times: Vec<(f64, &str, u64)> = index
        .files
        .iter()
        .filter(|f| !f.binary && !f.missing)
        .map(|f| {
            let bytes = std::fs::read(root.join(&f.path)).unwrap();
            let t = Instant::now();
            compute_file(&f.path, &bytes);
            (t.elapsed().as_secs_f64() * 1000.0, f.path.as_str(), f.lines)
        })
        .collect();
    times.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (ms, path, lines) in times.iter().take(5) {
        eprintln!("  {ms:7.1} ms  {lines:>6} lines  {path}");
    }
    let one_by_one: f64 = times.iter().map(|t| t.0).sum();
    eprintln!("  one by one: {one_by_one:.0} ms");
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn fuzzy_matches_telescope_fzf_native_on_the_target() {
    // The golden file comes from `scripts/fzf-golden.py` on `target-files.json` (unit 025).
    let files =
        std::fs::read_to_string(workspace_path(".private/fixtures/target-files.json")).unwrap();
    let golden =
        std::fs::read_to_string(workspace_path(".private/fixtures/target-fzf.json")).unwrap();
    let checked = common::assert_matches_fzf_native(&files, &golden);
    eprintln!("real repo fuzzy: {checked} hits match telescope-fzf-native");
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn fuzzy_ranking_of_the_whole_index_takes_under_a_frame() {
    use minimap_core::fuzzy::{Candidate, Pattern, rank};
    let (root, excludes) = target();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    // The picker prepares the candidates once, then ranks them on each keystroke.
    let t0 = Instant::now();
    let candidates: Vec<Candidate> = index
        .files
        .iter()
        .map(|f| Candidate::new(&f.path))
        .collect();
    eprintln!(
        "real repo fuzzy candidates: {:.1} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    // Queries from a real file name, so no name is written here.
    let name = index
        .files
        .iter()
        .map(|f| f.path.rsplit('/').next().unwrap())
        .find(|n| n.len() >= 8 && n.is_ascii())
        .unwrap()
        .to_string();
    for len in [1, 3, 8] {
        let q = &name[..len];
        let t0 = Instant::now();
        let ranked = rank(candidates.iter().enumerate(), &Pattern::parse(q));
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "real repo fuzzy rank: {len}-char query, {} matches, {ms:.1} ms",
            ranked.len()
        );
        assert!(ms < 16.0, "{len}-char query took {ms:.1} ms");
        let top = &index.files[ranked[0].index].path.to_lowercase();
        let mut chars = top.chars();
        assert!(
            q.to_lowercase().chars().all(|c| chars.any(|t| t == c)),
            "the top result lacks the query"
        );
    }
}

/// Top-level definition heads by a line scan: a line that starts with `(` and a symbol whose name part (after any
/// `alias/`) starts with `def` or `reg-`. This is the `^\((\S+/)?(def|reg-)` check of unit 007.
fn scanned_definitions(text: &str) -> usize {
    text.lines()
        .filter_map(|l| l.strip_prefix('('))
        .map(|l| {
            l.split(|c: char| c.is_whitespace() || "()[]{}\"".contains(c))
                .next()
                .unwrap_or("")
        })
        .filter(|sym| {
            let name = sym.rsplit('/').next().unwrap_or(sym);
            name.starts_with("def") || name.starts_with("reg-")
        })
        .count()
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn clojure_outlines_on_the_target() {
    use minimap_core::FileDetails;
    use minimap_core::outline::{Clojure, language_for};
    let (root, excludes) = target();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    let clojure: Vec<_> = index
        .files
        .iter()
        .filter(|f| !f.binary && !f.missing && language_for(&f.path).is_some())
        .collect();
    let (mut items, mut scanned, mut errors, mut with_ns, mut ns_match) = (0, 0, 0, 0, 0);
    let mut short = 0;
    let mut slowest = (0.0f64, 0u64);
    let mut near_1000 = None;
    for f in &clojure {
        let bytes = std::fs::read(root.join(&f.path)).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        let t0 = Instant::now();
        let d = FileDetails::compute(&f.path, &bytes);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if ms > slowest.0 {
            slowest = (ms, f.lines);
        }
        if near_1000.is_none_or(|(_, n): (f64, u64)| f.lines.abs_diff(1000) < n.abs_diff(1000)) {
            near_1000 = Some((ms, f.lines));
        }
        errors += usize::from(Clojure::has_errors(&text));
        // Lines rise (reader conditional branches may share one) and stay inside the file.
        let lines: Vec<u32> = d.outline.iter().map(|o| o.line).collect();
        assert!(lines.windows(2).all(|w| w[0] <= w[1]), "outline lines rise");
        assert!(
            lines.iter().all(|&l| l >= 1 && u64::from(l) <= f.lines),
            "outline lines are inside the file"
        );
        assert_eq!(d.shape.as_ref().unwrap().lines, f.lines, "shape line count");
        let n = scanned_definitions(&text);
        items += d.outline.len();
        scanned += n;
        short += usize::from(d.outline.len() < n);
        if let Some(ns) = &d.namespace {
            with_ns += 1;
            let stem = f
                .path
                .rsplit('/')
                .next()
                .unwrap()
                .split('.')
                .next()
                .unwrap();
            ns_match += usize::from(ns.rsplit('.').next() == Some(&stem.replace('_', "-")));
        }
    }
    let report = format!(
        "clojure files {}; parse errors {errors}; outline items {items}; scanned heads {scanned}; \
         files with fewer items than scanned heads {short}; namespaces {with_ns}, matching the file name {ns_match}; \
         slowest details {:.1} ms ({} lines); nearest to 1,000 lines {:.1} ms ({} lines)",
        clojure.len(),
        slowest.0,
        slowest.1,
        near_1000.unwrap().0,
        near_1000.unwrap().1,
    );
    eprintln!("real repo outlines: {report}");
    let capture = workspace_path(".private/captures/v007-outlines.txt");
    std::fs::create_dir_all(capture.parent().unwrap()).unwrap();
    std::fs::write(&capture, format!("{report}\n")).unwrap();

    assert!(clojure.len() > 100, "enough Clojure files to check");
    assert!(
        errors * 100 <= clojure.len(),
        "at most 1 % of files have parse errors"
    );
    assert!(
        items >= scanned,
        "the outline has at least the scanned heads"
    );
    assert!(
        ns_match * 100 >= with_ns * 95,
        "95 % of namespaces match their file name"
    );
    assert!(
        near_1000.unwrap().0 < 50.0,
        "details of a 1,000-line file take under 50 ms"
    );
}

/// `git rev-list` in the target repo, one line per commit.
fn rev_list(root: &std::path::Path, args: &[&str]) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-list", "--no-merges", "--full-history"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn git_history_matches_git_rev_list() {
    use minimap_core::history::{History, now};
    let (root, excludes) = target();
    let index = Index::from_repo(
        &root,
        &IndexOptions {
            excludes,
            ..Default::default()
        },
    )
    .unwrap();
    let paths: Vec<String> = index.files.iter().map(|f| f.path.clone()).collect();
    let t0 = Instant::now();
    let h = History::walk(&root, &paths, now()).unwrap();
    let secs = t0.elapsed().as_secs_f64();
    let with_history = paths.iter().filter(|p| h.file(p).is_some()).count();
    let report = format!(
        "git walk {secs:.2} s; {} of {} files have history; {} recent commits kept",
        with_history,
        paths.len(),
        h.commits.len()
    );
    eprintln!("real repo history: {report}");
    let capture = workspace_path(".private/captures/v008-history.txt");
    std::fs::write(&capture, format!("{report}\n")).unwrap();

    // 20 files spread over the index, compared with git.
    let n = paths.len();
    for k in 0..20 {
        let p = &paths[k * n / 20];
        let want = rev_list(&root, &["--format=%H %ct", "HEAD", "--", p]);
        // `--format` puts a `commit <hash>` line before each formatted line.
        let want: Vec<(String, i64)> = want
            .iter()
            .filter(|l| !l.starts_with("commit "))
            .map(|l| {
                let (hash, t) = l.split_once(' ').unwrap();
                (hash.to_string(), t.parse().unwrap())
            })
            .collect();
        let Some(f) = h.file(p) else {
            assert!(
                want.is_empty(),
                "file {k} has git history but no walk history"
            );
            continue;
        };
        assert_eq!(f.count as usize, want.len(), "count of file {k}");
        let times: Vec<i64> = want.iter().map(|w| w.1).collect();
        assert_eq!(f.newest, *times.iter().max().unwrap(), "newest of file {k}");
        assert_eq!(f.oldest, *times.iter().min().unwrap(), "oldest of file {k}");
        // The recent commits are the newest by commit time. Equal times may come in either order.
        let mut sorted = want.clone();
        sorted.sort_by_key(|w| std::cmp::Reverse(w.1));
        let got: Vec<i64> = h.recent(f).map(|c| c.time).collect();
        let expect: Vec<i64> = sorted.iter().take(5).map(|w| w.1).collect();
        assert_eq!(got, expect, "recent commit times of file {k}");
        for c in h.recent(f) {
            assert!(
                want.iter().any(|w| w.0 == c.hash),
                "a recent commit of file {k} is not in git's list"
            );
        }
    }
    assert!(
        with_history * 100 >= paths.len() * 99,
        "almost every file has history"
    );
}

/// Asserts that a diff is a valid edit script: its rows rebuild both texts in order, and equal rows hold equal lines.
fn assert_valid_diff(d: &minimap_core::diff::FileDiff, old: &str, new: &str) {
    use minimap_core::diff::Tag;
    let (old_lines, new_lines): (Vec<&str>, Vec<&str>) =
        (old.lines().collect(), new.lines().collect());
    let olds: Vec<usize> = d.rows.iter().filter_map(|r| r.old).collect();
    let news: Vec<usize> = d.rows.iter().filter_map(|r| r.new).collect();
    assert_eq!(olds, (0..old_lines.len()).collect::<Vec<_>>(), "old rows");
    assert_eq!(news, (0..new_lines.len()).collect::<Vec<_>>(), "new rows");
    for r in &d.rows {
        if let (Tag::Same, Some(o), Some(n)) = (r.tag, r.old, r.new) {
            assert_eq!(old_lines[o], new_lines[n], "an equal row");
        }
    }
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn diff_counts_sit_between_git_minimal_and_git_default_on_recent_commits() {
    // Git's default Myers takes heuristic shortcuts, so its diff is not always minimal (unit 030). Ours must be a valid
    // edit script, never larger than git's default, and never smaller than `--minimal`.
    use minimap_core::diff::{FileDiff, text_at};
    let (root, excludes) = target();
    let excludes = exclude_set(&excludes).unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    // `added removed path` rows of a numstat, text files only.
    let numstat = |extra: &[&str], parent: &str, commit: &str| -> Vec<(usize, usize, String)> {
        let args = [
            &["diff", "--numstat", "--no-renames"],
            extra,
            &[parent, commit],
        ]
        .concat();
        git(&args)
            .lines()
            .filter_map(|row| {
                let mut cols = row.split('\t');
                let a = cols.next()?.parse().ok()?;
                let r = cols.next()?.parse().ok()?;
                Some((a, r, cols.next()?.to_string()))
            })
            .collect()
    };
    let commits = git(&["log", "--no-merges", "-n", "60", "--format=%H %P"]);
    let (mut checked, mut equal) = (0, 0);
    let t0 = Instant::now();
    'commits: for line in commits.lines() {
        let mut ids = line.split_whitespace();
        let (Some(commit), Some(parent)) = (ids.next(), ids.next()) else {
            continue;
        };
        let default = numstat(&[], parent, commit);
        let minimal = numstat(&["--minimal"], parent, commit);
        for ((a, r, path), (ma, mr, _)) in default.iter().zip(&minimal) {
            if excludes.is_match(path) {
                continue;
            }
            // A deleted file has no text at the commit.
            let Some(new) = text_at(&root, commit, path).unwrap() else {
                continue;
            };
            let old = text_at(&root, parent, path).unwrap().unwrap_or_default();
            let d = FileDiff::compute(&old, &new);
            assert_valid_diff(&d, &old, &new);
            let (ours, git_default, git_minimal) = (d.added + d.removed, a + r, ma + mr);
            assert!(
                git_minimal <= ours && ours <= git_default,
                "commit {commit}: ours {} {}, git {a} {r}, minimal {ma} {mr}",
                d.added,
                d.removed
            );
            equal += usize::from((d.added, d.removed) == (*a, *r));
            checked += 1;
            if checked == 40 {
                break 'commits;
            }
        }
    }
    eprintln!(
        "diffs of {checked} files: valid, within git's minimal and default counts; {equal} equal to git's default; {:.0} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(checked, 40);
}

#[test]
#[ignore = "needs MINIMAP_TARGET_REPO"]
fn notes_on_target_files_normalize_and_round_trip() {
    // Unit 052: notes on 20 target files, by absolute and by relative path. It prints counts only, no paths.
    use minimap_core::events::{Event, parse};
    let (root, options) = target_options();
    let canonical = root.canonicalize().unwrap();
    let index = Index::from_repo(&root, &options).unwrap();
    let step = (index.files.len() / 20).max(1);
    let picked: Vec<&str> = index
        .files
        .iter()
        .step_by(step)
        .take(20)
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(picked.len(), 20);
    let mut checked = 0;
    for (k, p) in picked.iter().enumerate() {
        let lines = Some((k as u32 + 1, k as u32 + 3));
        for given in [root.join(p).to_string_lossy().into_owned(), p.to_string()] {
            let e = Event::Note {
                path: given,
                lines,
                text: format!("Stop {k}: «quoted» \"text\"\nwith two lines."),
                source: None,
                session: Some("s".into()),
            };
            let back = parse(&e.to_line()).unwrap().normalized(&root, &canonical);
            let Some(Event::Note { path, lines: l, .. }) = back else {
                panic!("note {k} did not normalize")
            };
            assert_eq!(path, *p);
            assert_eq!(l, lines);
            checked += 1;
        }
    }
    println!("{checked} notes normalized to index paths");
}
