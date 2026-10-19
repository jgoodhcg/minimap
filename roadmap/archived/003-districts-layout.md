---
title: "Districts layout"
status: done
description: "Port the v3 Districts layout to minimap-core and prove it matches the mockup tile for tile"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1, core, layout]
priority: high
---

# Districts layout

## Intent

The layout is the product.
Spatial memory depends on it being deterministic and stable.
The v3 mockup layout was tuned by eye on the fake `quayside` repo, so the Rust port must reproduce it exactly before anyone changes it.

## Specification

In `minimap-core`, module `layout`:

- `Tree::from_paths(root_name, files)` where each file has a path and an `ignored` flag.
  - Build directory nodes. Sort child directories by name and files by (`ignored`, name).
  - Compact chains: a directory with no files and one child merges with that child, and the header shows `a/b/`.
  - Record the depth of each node.
- `Layout::compute(&Tree) -> Layout` with:
  - `tiles`: one `(x, y)` in cells per file, indexed like the input files.
  - `headers`: one `{x, y, w, h, depth, node}` per directory block.
  - `w`, `h`: the whole layout size in cells.
- Port `size`, `plan`, and `place` from `mockups/v3-template.html:101-168` with the constants in `docs/design.md` `Layout`.
  Use `f64` for the area and `sqrt` math, and the same operation order, so results match the JavaScript.
  Compare names by byte order. This matches the JavaScript order for ASCII paths.
- The layout reads only the tree. It takes no selection, layer, or view input.

Parity fixtures (fake data, safe to commit):

- `mockups/export-layout.mjs`: a Node script. It reads `mockups/v3-template.html`, takes the script text from `/* ---------- tree ---------- */` to `/* ---------- colors ---------- */`, and evaluates it with `DATA` from `mockups/fake.json` and a stub `$` that returns `{}`.
  It writes:
  - `crates/core/tests/fixtures/quayside-files.json`: `{"root", "files": [{"p", "ig"?}]}` in `fake.json` order.
  - `crates/core/tests/fixtures/quayside-layout.json`: `{"w", "h", "tiles": [[x, y], …], "headers": [{"path", "x", "y", "w", "h", "depth"}, …]}`.
- Generate `mockups/fake.json` first with `python3 mockups/fakegen.py mockups/fake.json`.
- The script also takes `<input.json> <output-dir>` arguments. Run it on `.private/fixtures/target-files.json` (from unit `002`) with output in `.private/fixtures/`, so the real repo gets its own golden layout.
- Document the regeneration command at the top of the script.

## Validation

- [x] Parity test: every tile position, every header rectangle, and `w`/`h` equal the golden file for every quayside file.
- [x] Real-repo test (`AGENTS.md` `Real-repo tests`): parity with `.private/fixtures/target-layout.json`, no overlap, containment, and determinism on the real repo.
- [x] Determinism test: shuffle the input file order; the layout per path does not change.
- [x] No-overlap test: no two tile rectangles (`TW - 1` × `TH - 1`) overlap.
- [x] Containment test: each tile lies inside its directory's header rectangle.
- [x] Compaction test: `a/b/c/x.clj` and `a/b/c/y.clj` produce one header for `a/b/c/` under the root.
- [x] Ignored test: an ignored file sorts after the non-ignored files of its directory.
- [x] Timing: `Layout::compute` on the fixture takes under 50 ms in a release build (a `#[ignore]` test or a small example is enough). Record the number.
- [x] All `AGENTS.md` validation levels 1–3 pass.

## Scope

- Do not improve the packing. Stability changes wait for unit `009`.
- No rendering.

## Context

- `docs/design.md` `Layout` and `Stability principle`.
- Mockup layout: `mockups/v3-template.html:71-168`.
- Node is installed (`node` v26 via nvm). Python 3 is installed.
