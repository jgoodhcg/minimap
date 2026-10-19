---
title: "Generated files"
status: done
description: "Mark lock files and other generated files, and cap their card height in the CARDS layout, so a long lock file no longer stretches its district"
created: 2026-10-02
updated: 2026-10-02
tags: [milestone-g3, gui, core]
priority: high
---

# Generated files

## Intent

Decided with the user on 2026-10-02, after the user's first G2 reaction ("This is really nice. I enjoy zooming around a lot.").
Very long generated files, such as `package-lock.json`, render too large: a card is as tall as its lines (unit `039`),
and a district's file grid is as tall as its tallest card, so one lock file leaves a tall empty band beside it.
A generated file's card gets a capped height. The cap is a rule on generated files, not on `.json` or `.yml`:
a hand-written config JSON keeps its true height.

## Specification

Marks (`minimap-core`, a new module `generated`). A file is generated when one of these marks it:

1. A built-in list of exact file names, not patterns: `package-lock.json`, `npm-shrinkwrap.json`, `yarn.lock`, `pnpm-lock.yaml`,
   `Cargo.lock`, `Gemfile.lock`, `poetry.lock`, `Pipfile.lock`, `composer.lock`, `go.sum`, `flake.lock`, `mix.lock`, `pubspec.lock`,
   `Podfile.lock`, `deno.lock`, `uv.lock`. The name is the part after the last `/`. A glob such as `*-lock.json` would catch files that are not lock files.
2. `.gitattributes` with `linguist-generated`. Read every `.gitattributes` file that the index lists, before the excludes, with the existing `globset`.
   Read only that attribute:
   - `linguist-generated`, or `linguist-generated=<value>` with a value other than `false`, marks the file, as GitHub Linguist reads it.
   - `-linguist-generated` or `linguist-generated=false` unmarks it, also a built-in name.
   - `!linguist-generated` returns it to unspecified, so the built-in list decides.
   - Patterns follow git's attribute rules. A pattern is relative to the directory of its `.gitattributes` file.
     A pattern with no `/` matches that name at any depth below the directory. A pattern with a `/` matches from the directory. `**` crosses directories.
     A pattern that matches a directory does not match the files inside it. Git forbids negative patterns (`!`), so skip them. Skip `[attr]` macro lines and comments.
   - A later line overrides an earlier line, and a file in a deeper directory overrides a file above it.
   - Skip a line that does not parse. A bad `.gitattributes` never stops the index.
3. A config option, so the user can mark files without a change to the repo. It mirrors `exclude`:
   `[repos."<path>"] generated = [...]` for one repo, and a global `[map] generated = [...]` for every repo.
   The global list makes sense for names that the built-in list does not have, such as `*.min.js`. The global rules come first, so a repo rule overrides them.
   The rules are gitignore rules:
   - A pattern with no `/` matches that file name in any directory. A pattern with a `/` (a leading one too, but not only a trailing one) matches from the repo root.
   - `*` does not cross `/`; `**` does. A trailing `/` matches directories only.
   - A rule that matches a directory marks every file under it.
   - A leading `!` unmarks a file, also a built-in name. The last rule that matches wins.
   - Build it on `globset`, as `index::exclude_set` does. A bad rule stops startup with an error that names the rule, as a bad exclude does.

The order is the built-in list, then `.gitattributes`, then the config. Each one that has an opinion about a file overrides the earlier ones.

Index:

- `IndexFile::generated`, in the JSON as `"g": 1`. `IndexOptions::generated` holds the config rules. `scan` sets the flag on every re-index.
- `Index::from_json` marks the built-in names and honors `"g": 1`, so the `quayside` fixture's `package-lock.json` is generated with no fixture change.
  A dump loses a config unmark of a built-in name; dumps are a test and fake-repo input only.
- Both front ends pass `Config::generated_for(root)` (the global rules, then the repo's) to the index. The TUI does not use the flag, and its output does not change.

Layout (`minimap-core`), `CARDS` only:

- `TreeFile::generated`. `Tree::from_index` copies the index's flag. `Tree::from_files` gives `false`, and a caller can set it.
- `LayoutConfig::gen_lines`: a generated file's tile counts at most this many lines. 0 gives no cap.
  `CARDS` uses `GEN_LINES` 300 (tune it at the review; record the value). `CHARS` and `TILES` use 0.
- `LayoutConfig::tile_lines(lines, generated)` and `LayoutConfig::tile_rows(lines, generated)`. A generated card is as tall as a card of 300 lines:
  `step(300)` is 363 lines, so the body is 21 rows (378 points), and the card is 396 points.
- The cap is in the layout, so a lock file that grows past the cap moves no card.

Cards (`minimap-gui`):

- The shape draws the first lines at the normal scale, as many as fit the body above the cut mark, so one line keeps one height on every card.
- A cut mark (`Kind::Cut`) along the body's bottom edge shows that the card is cut short. It shows when the body cannot hold every line, which only a capped card can have.
  It is a dashed line, `CUT_PT` 2 world points high (at least 1 screen point), in `DIM`. It shows from the icon level up, also with the shape toggled off.
  Tune its look at the review.
- The name row's line count stays the real count.
- The loading rect stops above the cut mark, as the shape does.

Docs: `docs/design.md` `Stability principle`, `Layout` (a `gen_lines` row and a generated-files paragraph), and `GUI (prototype)` (the shape and the cut mark).
The config doc comment in `crates/core/src/config.rs`.

## Validation

- [x] Core tests for the marks:
  - Built-in names match exactly: `package-lock.json` and `web/yarn.lock` match; `my-package-lock.json` and `yarn.lock.bak` do not.
  - `.gitattributes`: set, `=true`, another value, `-`, `=false`, `!`; comments, `[attr]` lines, and negative patterns skip;
    a directory pattern does not mark the files inside; a subdirectory's file is relative to its directory; a later line wins; a deeper file wins.
  - Config rules: no `/` matches at any depth; a `/` anchors at the root; a leading `/`; `**`; a trailing `/` marks the files under a directory only;
    a matched directory marks its files; `!` unmarks a built-in name; the last match wins; a bad glob is an error that names the rule.
  - The order: a config rule overrides `.gitattributes`, which overrides the built-in list.
- [x] Index test on a temporary git repo: a root and a subdirectory `.gitattributes`, an untracked lock file, and a config rule. The JSON round trip keeps `"g": 1`.
- [x] Config test: `[map] generated` and `[repos."<path>"] generated` give the global rules, then the repo's.
- [x] Core layout tests on `quayside`:
  - `package-lock.json` (thousands of lines) is generated, and its card is 22 rows. A hand-written file over 300 lines keeps its true height.
  - A generated file that grows from 400 to 40,000 lines moves no card. A file that grows within its step still moves no card.
  - The `CARDS` insta snapshot changes. The `CHARS` and `TILES` snapshots and the parity tests do not.
- [x] GUI scene tests: the generated card's shape bars lie inside its body and above the cut mark; the cut mark shows from the icon level, also with the shape off;
  a 300-line file that is not generated has no cut mark; a line has one height on a capped card and on another card; the count shows the real line count.
- [x] No TUI snapshot changes: `cargo test -p minimap-tui --test render` writes no `.snap.new`.
- [x] Real-repo test (read only): the generated files on the real repo by source, the map size, the fill, and the tallest generated card, with and without the cap. Print them.
- [x] The stability report (`--cards`) on the real repo, with a capped row, under `.private/`. Record the change against `.private/stability-2026-10-01-bands.md`.
- [x] The headless frame probe on the real repo, in a release build: p95 under 8 ms, and the font atlas does not grow on the second sweep.
- [x] GUI visual check: at the review, after the user agreed (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-02)

- Marks: `minimap_core::generated` (`LOCK_FILES`, `is_lock_file`, `Rules::gitignore`, `Rules::gitattributes`, `Marks`). Each `.gitattributes` file becomes one
  ordered rule list; `Marks::is_generated` takes the built-in list, then the last `.gitattributes` opinion (deeper files later), then the last config rule.
  - `-linguist-generated` and `=false` unmark a built-in name too, as Linguist lets a repo unmark its own detection. A value other than `false` marks, as Linguist reads it.
  - C-style quoted patterns parse. Lines with a bad glob, negative patterns, `[attr]` macros, and directory patterns (`build/`) are skipped.
- Config: `[map] generated` holds the global rules, and `[repos."<path>"] generated` follows them (`Config::generated_for`).
  The global list makes sense: the built-in list has exact names only, and a user who wants `*.min.js` or a codegen suffix on every repo writes it once.
  `[map]` is a new table; `[keys.map]` is unrelated.
- Rules: a rule gives the globs `G/**` (files under a matched directory) and, without a trailing `/`, `G`; a pattern with no `/` gets a `**/` prefix.
  Deviation from gitignore: a `!` rule can unmark a file under a marked directory (`vendor/`, then `!vendor/keep.js`). Gitignore cannot re-include under an excluded directory,
  because git stops its walk there; the marks decide per file, so the later rule wins.
- Index: `IndexFile::generated`, and `IndexOptions::generated` from both front ends. `scan` reads every listed `.gitattributes` before the excludes, on each re-index.
  Deviation: the JSON is exact, not lossy. `IndexFile` serializes through `FileJson`, and `g` is there only when the file differs from the built-in list:
  `1` for another generated file, `0` for a lock-file name that a mark unmarked. So a dump keeps every mark, and `quayside-files.json` needed no change.
- Layout: `TreeFile::generated`, `LayoutConfig::gen_lines` (`GEN_LINES` 300 for `CARDS`, 0 for `CHARS` and `TILES`), `tile_lines`, and `tile_rows(lines, generated)`.
  `GEN_LINES` is untuned: tune it at the review.
- quayside: the `package-lock.json` card shrinks to 22 rows, and the map gets much shorter. A generated file from 301 to 40,000 lines moved no card.
- Real repo (2026-10-02, read only): every generated file came from the built-in names; it has no `.gitattributes`, and the config has no rules.
  Generated files hold a tenth of its lines. Without the cap, several generated cards were far taller than 396 points.
  With the cap, the map got much shorter and its fill rose.
- Stability report (`.private/stability-2026-10-02-generated.md`, 50 samples, 12 months to 2026-10-02). The window moved by a day since
  `.private/stability-2026-10-01-bands.md`. The cap halves the summed moves in the same window. Name order moved more than `CARDS` with the cap,
  and rounded widths moved less, for a larger and sparser map. One card size still moves least.

  The example takes `--generated <rule>` and reads each sampled commit's `.gitattributes` blobs, so its marks match the index's.
- Cards: `Kind::Cut`, `scene::shown_lines`, `CUT_PT` 2, `CUT_ROOM` 6, and `CUT_DASHES` 12. A capped card shows 372 of its lines at zoom 1 (the 378-point body less the room).
  A shape now never draws past its card's body, also for a file that grew after the index (it overflowed before). The loading rect stops above the mark.
- Tests that changed with the layout: `select.rs` picks its sample card in the first grid row of a directory below the root (the old pick moved under the root's band),
  and the 50- and 300-line shape test uses files whose bodies hold those lines.
- Headless frame probe: committed as `headless_frame_probe_along_the_capture_sweep` in `crates/gui/tests/real_repo.rs`, with no target name or path.
  It runs only in a release build and selects the first 27-line file and the longest hand-written file. On the real repo, with the window's atlas limit of 8,192 texels:
  p95 under 3 ms in both passes, at 1 and 2 pixels per point, and the atlas did not grow on the second sweep.
  For the review: with egui's default limit of 2,048 texels (no window), at 2 pixels per point the atlas grew on the second sweep.
  Large directory names rasterize at twice their size, and egui clears an atlas that is over 80 % full. The window's GPU reported 8,192 in the G2 captures, so the window does not see it.
- Real-repo tests: all pass. `fuzzy_ranking_of_the_whole_index_takes_under_a_frame` failed twice under parallel load in a debug build (the loader test computes every file at the same time);
  it took about half of its 16 ms alone and passed with `--test-threads=1`.
- GUI visual check on the real repo, 2026-10-02 at 17:30, release build, at the exit review after the user agreed (no window opened during the stint):
  three captures, with a short file, the longest hand-written file, and the largest lock file as `--select`. The window ran at 1 pixel per point.
  At card width 200 the lock card is 396 world points high, with its first lines at the line height of its neighbors and the dashed mark along its bottom.
  `frames.txt`: p95 under 5 ms in both passes for each capture. The atlas was the same size after both sweeps.
  PNGs: `.private/captures/g3/`. The user tried the build before the captures: "Looks good."

## Scope

- No change to `CHARS`, `TILES`, or the TUI's map and output.
- No `.git/info/attributes` and no `core.attributesFile`: the config option covers local marks.
- No other generated-file heuristics (no content sniffing, no `@generated` headers).
- No new crate and no new crate feature: gix's attributes feature would be a new feature.
- No other work on empty space: the name bands and the shelf holes wait for the G2 review.

## Context

- Decided with the user on 2026-10-02: the three marks, the cap of 300 lines, the cut mark, and the real line count.
- On a real repo, lock files by the built-in list are common, and many are over 300 lines.
- Layout: `crates/core/src/layout.rs` (`TreeFile`, `LayoutConfig::tile_rows`, `flow`). Index: `crates/core/src/index.rs` (`exclude_set`, `scan`).
  Config: `crates/core/src/config.rs` (`RepoConfig`, `excludes_for`). Shape: `crates/gui/src/scene.rs` (`shape`, `CardGeometry`).
  Stability: `crates/core/examples/stability.rs` (`--cards`).
- Unit `039` (line-scaled cards) and unit `040` (name bands): `roadmap/archived/`.
