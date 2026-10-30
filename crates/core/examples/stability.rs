//! Measures how many tiles move between historical commits of a repo (unit `009`).
//!
//! ```sh
//! cargo run --release -p minimap-core --example stability -- <repo> [--exclude <glob>]... [--generated <rule>]... [--months 12] [--skip-months 0] [--samples 50] [--variants | --cards]
//! ```
//!
//! It prints a Markdown report with numbers only, no paths.
//! With `--variants`, it compares the unrounded `CHARS` packing with the rounding and order variants of unit `027`,
//! and applies the adoption rule.
//! With `--cards`, it compares `CARDS` with one card size and the line-scaled `CARDS` of unit `039`,
//! with the name bands by block width (unit `040`) and by the count of files below each directory (unit `062`).
//! It reads each file's line count at each sampled commit from its blob, once per blob ID.
//! It marks generated files at each commit (unit `041`): the built-in names, the commit's `.gitattributes` files, and the `--generated` rules.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use minimap_core::generated::{Marks, is_attributes_file};
use minimap_core::index::{count_lines, exclude_set, is_binary, repo_root};
use minimap_core::layout::{NAME_SHARE, Names, Order, Quant, Round};
use minimap_core::stability::{Moves, Positions, compare};
use minimap_core::{Layout, LayoutConfig, Tree};

struct Args {
    repo: PathBuf,
    excludes: Vec<String>,
    /// Generated-file rules, as the config's (unit `041`).
    generated: Vec<String>,
    months: u32,
    /// Leave out the newest months, so a window ends in the past.
    skip: u32,
    samples: usize,
    variants: bool,
    cards: bool,
}

fn args() -> Result<Args> {
    let mut a = Args {
        repo: PathBuf::new(),
        excludes: vec![],
        generated: vec![],
        months: 12,
        skip: 0,
        samples: 50,
        variants: false,
        cards: false,
    };
    let mut it = std::env::args().skip(1);
    let mut repo = None;
    while let Some(arg) = it.next() {
        let mut value = || it.next().with_context(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--exclude" => a.excludes.push(value()?),
            "--generated" => a.generated.push(value()?),
            "--months" => a.months = value()?.parse()?,
            "--skip-months" => a.skip = value()?.parse()?,
            "--samples" => a.samples = value()?.parse()?,
            "--variants" => a.variants = true,
            "--cards" => a.cards = true,
            s if s.starts_with("--") => bail!("unknown flag {s}"),
            _ => repo = Some(PathBuf::from(arg)),
        }
    }
    a.repo = repo.context(
        "usage: stability <repo> [--exclude <glob>]... [--generated <rule>]... [--months 12] [--skip-months 0] [--samples 50] [--variants | --cards]",
    )?;
    if a.samples < 2 {
        bail!("--samples must be at least 2");
    }
    if a.variants && a.cards {
        bail!("--variants and --cards do not go together");
    }
    Ok(a)
}

/// One sampled commit: its time, and per configuration the layout positions of its files, the map area and size,
/// and the summed drawn tile area, all in cells.
struct Sample {
    time: i64,
    files: usize,
    pos: Vec<Positions>,
    area: Vec<i64>,
    size: Vec<(i32, i32)>,
    tiles: Vec<i64>,
}

fn configs(variants: bool, cards: bool) -> Vec<(String, LayoutConfig)> {
    if cards {
        // Unit 039 compared one card size with line-scaled cards; unit 040 added the name bands by width; unit 041 the generated cap.
        // Unit 062 sizes the names by the count of files below each directory.
        let widths = LayoutConfig {
            names: Names::Width(NAME_SHARE),
            ..LayoutConfig::CARDS
        };
        let uncapped = LayoutConfig {
            gen_lines: 0,
            ..widths
        };
        let flat = LayoutConfig {
            names: Names::None,
            ..uncapped
        };
        let rounded_w = Round {
            w: true,
            ..LayoutConfig::CARDS.round
        };
        return vec![
            (
                "CARDS, one card size".to_string(),
                LayoutConfig {
                    line_pt: 0.0,
                    ..flat
                },
            ),
            ("CARDS, line-scaled".to_string(), flat),
            ("CARDS, line-scaled, width bands".to_string(), uncapped),
            (
                "CARDS, line-scaled, width bands, generated cap".to_string(),
                widths,
            ),
            (
                "CARDS, line-scaled, count bands, generated cap".to_string(),
                LayoutConfig::CARDS,
            ),
            // Measured for the G2 review, not adopted: knobs that the layout already has.
            (
                "line-scaled, count bands, name order".to_string(),
                LayoutConfig {
                    order: Order::Name,
                    ..LayoutConfig::CARDS
                },
            ),
            (
                "line-scaled, count bands, widths rounded".to_string(),
                LayoutConfig {
                    round: rounded_w,
                    ..LayoutConfig::CARDS
                },
            ),
        ];
    }
    if !variants {
        return vec![
            ("CHARS".to_string(), LayoutConfig::CHARS),
            ("TILES".to_string(), LayoutConfig::TILES),
        ];
    }
    // The variants of unit 027, against the unrounded `CHARS` packing of the v5 mockup.
    let base = LayoutConfig::CHARS.unrounded();
    let with = |quant, order, round| LayoutConfig {
        quant,
        order,
        round,
        ..base
    };
    let r = |cap, w, h, max_depth| Round {
        cap,
        w,
        h,
        max_depth,
    };
    let g = Quant::Growth(1.25);
    let t = Order::Tallest;
    vec![
        ("baseline".to_string(), base),
        ("Step(4)".to_string(), with(Quant::Step(4), t, Round::ALL)),
        ("Growth(1.25)".to_string(), with(g, t, Round::ALL)),
        (
            "Name".to_string(),
            with(Quant::None, Order::Name, Round::ALL),
        ),
        (
            "Name + Growth(1.25)".to_string(),
            with(g, Order::Name, Round::ALL),
        ),
        (
            "Growth(1.25) capacity".to_string(),
            with(g, t, r(true, false, false, 0)),
        ),
        (
            "Growth(1.25) capacity + height".to_string(),
            with(g, t, r(true, false, true, 0)),
        ),
        (
            "Growth(1.25) capacity + height, depth ≤ 1".to_string(),
            with(g, t, r(true, false, true, 1)),
        ),
        (
            "Growth(1.25) capacity + height, depth ≤ 2".to_string(),
            with(g, t, r(true, false, true, 2)),
        ),
        (
            "Growth(1.2) capacity + height, depth ≤ 2".to_string(),
            with(Quant::Growth(1.2), t, r(true, false, true, 2)),
        ),
        (
            "Step(4) capacity + height, depth ≤ 2".to_string(),
            with(Quant::Step(4), t, r(true, false, true, 2)),
        ),
    ]
}

fn main() -> Result<()> {
    let a = args()?;
    let root = repo_root(&a.repo)?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut repo = gix::open(&root)?;
    repo.object_cache_size_if_unset(64 * 1024 * 1024);
    let excludes = exclude_set(&a.excludes)?;
    // A bad rule stops the report before the walk.
    Marks::new(&a.generated)?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let end = now - i64::from(a.skip) * 2_629_746;
    let cutoff = end - i64::from(a.months) * 2_629_746;
    let head = repo.head_commit()?;
    // First-parent history since the cutoff, oldest first. The whole chain is walked and filtered,
    // so a commit with a skewed clock does not end the walk early.
    // `gix` leaves `Info::commit_time` unset on a first-parent walk, so the time comes from the commit.
    let mut ids = Vec::new();
    for info in repo.rev_walk([head.id]).first_parent_only().all()? {
        let id = info?.id;
        let t = repo.find_commit(id)?.time()?.seconds;
        if t >= cutoff && t <= end {
            ids.push(id);
        }
    }
    ids.reverse();
    if ids.len() < 2 {
        bail!(
            "only {} commits in the {}-month window",
            ids.len(),
            a.months
        );
    }
    let n = a.samples.min(ids.len());
    let picks: Vec<usize> = (0..n).map(|k| k * (ids.len() - 1) / (n - 1)).collect();

    let configs = configs(a.variants, a.cards);
    let mut samples = Vec::with_capacity(n);
    // Line counts by blob ID, as the index counts them: 0 for binary blobs (`--cards` only).
    let mut lines_of: HashMap<gix::ObjectId, u64> = HashMap::new();
    // `.gitattributes` texts by blob ID (`--cards` only).
    let mut text_of: HashMap<gix::ObjectId, String> = HashMap::new();
    let mut generated_files = Vec::with_capacity(n);
    for &k in &picks {
        let commit = repo.find_commit(ids[k])?;
        let time = commit.time()?.seconds;
        let entries = commit.tree()?.traverse().breadthfirst.files()?;
        let mut files: Vec<(String, u64)> = Vec::new();
        let mut marks = Marks::new(&a.generated)?;
        for e in entries.iter().filter(|e| e.mode.is_no_tree()) {
            let path = e.filepath.to_string();
            // The attributes apply to every file, so an excluded `.gitattributes` still counts, as in the index.
            if a.cards && is_attributes_file(&path) {
                let text = text_of.entry(e.oid).or_insert_with(|| {
                    repo.find_object(e.oid).map_or(String::new(), |o| {
                        String::from_utf8_lossy(&o.data).into_owned()
                    })
                });
                marks.add_attributes(&path, text);
            }
            if excludes.is_match(&path) {
                continue;
            }
            let lines = if a.cards {
                *lines_of.entry(e.oid).or_insert_with(|| {
                    repo.find_object(e.oid).map_or(0, |o| {
                        if is_binary(&o.data) {
                            0
                        } else {
                            count_lines(&o.data)
                        }
                    })
                })
            } else {
                0
            };
            files.push((path, lines));
        }
        let mut tree = Tree::from_files(&name, files.iter().map(|(p, l)| (p.as_str(), false, *l)));
        if a.cards {
            for f in &mut tree.files {
                f.generated = marks.is_generated(&f.path);
            }
        }
        generated_files.push(tree.files.iter().filter(|f| f.generated).count());
        let mut s = Sample {
            time,
            files: files.len(),
            pos: Vec::new(),
            area: Vec::new(),
            size: Vec::new(),
            tiles: Vec::new(),
        };
        for (_, cfg) in &configs {
            let layout = Layout::compute(&tree, cfg);
            s.pos.push(Positions::of(&tree, &layout));
            s.area.push(i64::from(layout.w) * i64::from(layout.h));
            s.size.push((layout.w, layout.h));
            let tw = i64::from(cfg.tile_size().0);
            s.tiles
                .push(layout.tile_h.iter().map(|&h| tw * i64::from(h)).sum());
        }
        samples.push(s);
    }

    print_report(&a, &configs, ids.len(), &samples);
    if a.variants {
        print_variants(&configs, &samples);
    }
    if a.cards {
        print_cards(&configs, &samples, lines_of.len());
        println!();
        println!(
            "Generated files (unit `041`): {} at the oldest sample, {} at the newest; {} `.gitattributes` blobs read; {} `--generated` rules.",
            generated_files[0],
            generated_files[generated_files.len() - 1],
            text_of.len(),
            a.generated.len(),
        );
    }
    Ok(())
}

fn date(secs: i64) -> String {
    // Civil date from days since the epoch (Howard Hinnant's algorithm).
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn pct(x: f64) -> String {
    format!("{:.1} %", x * 100.0)
}

fn row(label: &str, m: &Moves) -> String {
    format!(
        "| {label} | {} | {} | {} | {:.1} | {:.1} | {} |",
        m.files,
        pct(m.moved_share()),
        pct(m.reflow_share()),
        m.median,
        m.p90,
        pct(m.dirs_moved_share()),
    )
}

/// The median of a list of shares, for the summary line.
fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(f64::total_cmp);
    minimap_core::stability::nearest_rank(&xs, 0.5)
}

fn print_report(a: &Args, configs: &[(String, LayoutConfig)], commits: usize, samples: &[Sample]) {
    let (first, last) = (&samples[0], &samples[samples.len() - 1]);
    println!("# Layout stability");
    println!();
    println!(
        "{} samples of {} first-parent commits in a {}-month window, {} to {}. Files: {} to {}. Excludes: {}.",
        samples.len(),
        commits,
        a.months,
        date(first.time),
        date(last.time),
        first.files,
        last.files,
        a.excludes.len(),
    );
    println!();
    println!(
        "Columns: files in both samples; files whose tile moved; files that moved only inside their block (reflow);"
    );
    println!(
        "median and p90 displacement of the moved files in cells; directories whose header moved."
    );
    for (c, (label, _)) in configs.iter().enumerate() {
        println!();
        println!("## `{label}`");
        println!();
        println!("| Pair | Files | Moved | Reflow | Median | p90 | Dirs moved |");
        println!("|---|---|---|---|---|---|---|");
        let mut moved = Vec::new();
        let mut reflow = Vec::new();
        let mut dirs = Vec::new();
        let mut changed = 0;
        for (k, w) in samples.windows(2).enumerate() {
            let m = compare(&w[0].pos[c], &w[1].pos[c]);
            moved.push(m.moved_share());
            reflow.push(m.reflow_share());
            dirs.push(m.dirs_moved_share());
            changed += usize::from(m.moved > 0);
            println!("{}", row(&format!("{} → {}", k, k + 1), &m));
        }
        let m = compare(&first.pos[c], &last.pos[c]);
        println!("{}", row("oldest → newest", &m));
        println!();
        println!(
            "Consecutive pairs: {} of {} moved any tile. Median moved {}, median reflow {}, median dirs moved {}.",
            changed,
            samples.len() - 1,
            pct(median(moved)),
            pct(median(reflow)),
            pct(median(dirs)),
        );
    }
}

/// The unit `027` numbers of one configuration over the samples.
struct Summary {
    /// The sum over consecutive pairs of the moved share: whole-map moves in the sampled period.
    summed: f64,
    /// Consecutive pairs that moved any tile.
    pairs_moved: usize,
    /// The moved share from the oldest to the newest sample.
    oldest_newest: f64,
    /// The map area of the newest sample, in cells.
    area: i64,
}

fn summarize(samples: &[Sample], c: usize) -> Summary {
    let (first, last) = (&samples[0], &samples[samples.len() - 1]);
    let mut s = Summary {
        summed: 0.0,
        pairs_moved: 0,
        oldest_newest: compare(&first.pos[c], &last.pos[c]).moved_share(),
        area: last.area[c],
    };
    for w in samples.windows(2) {
        let m = compare(&w[0].pos[c], &w[1].pos[c]);
        s.summed += m.moved_share();
        s.pairs_moved += usize::from(m.moved > 0);
    }
    s
}

/// The comparison table and the adoption rule of unit `027`: the lowest summed moves wins
/// when they are at most half the baseline's and the area is at most 1.3 times the baseline's.
fn print_variants(configs: &[(String, LayoutConfig)], samples: &[Sample]) {
    let sums: Vec<Summary> = (0..configs.len()).map(|c| summarize(samples, c)).collect();
    let base = &sums[0];
    println!();
    println!("## Variants (unit `027`)");
    println!();
    println!("| Variant | Summed moves | Pairs moved | Oldest → newest | Area | Area ratio |");
    println!("|---|---|---|---|---|---|");
    for ((label, _), s) in configs.iter().zip(&sums) {
        println!(
            "| {label} | {:.2} | {} of {} | {} | {} | {:.2} |",
            s.summed,
            s.pairs_moved,
            samples.len() - 1,
            pct(s.oldest_newest),
            s.area,
            s.area as f64 / base.area as f64,
        );
    }
    let pick = (1..configs.len())
        .filter(|&c| {
            sums[c].summed <= base.summed / 2.0 && sums[c].area as f64 <= base.area as f64 * 1.3
        })
        .min_by(|&x, &y| {
            sums[x]
                .summed
                .total_cmp(&sums[y].summed)
                .then(sums[x].area.cmp(&sums[y].area))
        });
    println!();
    match pick {
        Some(c) => println!(
            "Adoption rule: adopt **{}** (summed moves {:.2} against {:.2}, area ratio {:.2}).",
            configs[c].0,
            sums[c].summed,
            base.summed,
            sums[c].area as f64 / base.area as f64
        ),
        None => println!(
            "Adoption rule: no variant halves the summed moves ({:.2}) within 1.3 times the area. Keep the packing.",
            base.summed
        ),
    }
}

/// The comparison of unit `039`: one card size against line-scaled cards. Not a gate: the numbers go to the G2 review.
fn print_cards(configs: &[(String, LayoutConfig)], samples: &[Sample], blobs: usize) {
    let last = &samples[samples.len() - 1];
    println!();
    println!("## Cards (unit `039`)");
    println!();
    println!(
        "Line counts come from {blobs} blobs. Summed moves: the moved share summed over consecutive pairs. \
         Fill: the drawn card area over the map area, at the newest sample."
    );
    println!();
    println!(
        "| Layout | Summed moves | Pairs moved | Oldest → newest | Map size (cells) | Map area | Card area | Fill |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for (c, (label, _)) in configs.iter().enumerate() {
        let s = summarize(samples, c);
        println!(
            "| {label} | {:.2} | {} of {} | {} | {} x {} | {} | {} | {} |",
            s.summed,
            s.pairs_moved,
            samples.len() - 1,
            pct(s.oldest_newest),
            last.size[c].0,
            last.size[c].1,
            last.area[c],
            last.tiles[c],
            pct(last.tiles[c] as f64 / last.area[c] as f64),
        );
    }
}
