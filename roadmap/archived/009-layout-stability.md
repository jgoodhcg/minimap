---
title: "Layout stability measurement"
status: done
description: "Measure how many tiles move between historical commits of the real target repo, and report whether the packing needs to change"
created: 2026-09-25
updated: 2026-09-25
tags: [core, layout, research]
priority: medium
---

# Layout stability measurement

## Intent

Tallest-first packing sorts by size, so a growing directory can move its siblings.
If tiles move often, spatial memory breaks and the core premise fails.
Measure it before the user invests memory in the current layout.

## Specification

- A Cargo example, `cargo run --release -p minimap-core --example stability -- <repo> [--exclude <glob>]... [--months 12] [--samples 50]`.
- Take `--samples` commits evenly spaced along the first-parent history of the last `--months` months (`gix` `rev_walk` with `first_parent_only`).
- For each sample, list the files in that commit's tree with `gix`, apply the excludes, and compute the layout.
  No new `gix` features are needed: `rev_walk` and tree traversal have no feature gate in `gix` 0.87.
- The metric is a module, `minimap_core::stability`, so its unit tests sit in `crates/core/tests/`. The example only reads git and prints.
- For each consecutive pair of samples, over the files present in both:
  - the fraction of files whose tile position changed;
  - the median and 90th-percentile displacement in cells, as the Euclidean distance of `(dx, dy)`, over the moved files;
  - the fraction of files that moved only inside their block: the tile moved, and the header of its directory did not (grid reflow from an insert or a delete);
  - the fraction of directory headers that moved, over the directories present in both.
- Also report the same numbers for the pair (oldest sample, newest sample).
- Report both layouts: `CHARS` (unit `019`, the map) and `TILES`.
- Print a Markdown table to stdout. Save the real-repo report to `.private/stability-YYYY-MM-DD.md`. The report contains numbers only, no paths.
- End the report with a recommendation: keep the packing, or name one stability-preserving alternative to prototype.
  Reflow and packing moves need different fixes, so the recommendation names which one dominates. Do not implement the alternative in this unit.

## Validation

- [x] Unit test of the metric: two position maps with 3 of 10 shared files moved by known amounts report 30 % moved and the expected median and p90.
- [x] Unit test: identical trees report 0 moved.
- [x] Unit test: a file moved with its block counts as a packing move; a file moved inside a block that stayed counts as reflow.
- [x] Run on the real target repo with the exclusions from `.private/targets.md`. The report exists under `.private/`.
- [x] All `AGENTS.md` validation levels 1–3 pass.

## Scope

- Measurement and a recommendation only. No layout changes.

## Context

- Measure the `CHARS` layout from unit `019`, which the map uses, as well as `TILES`.
- `docs/design.md` `Stability principle`.
- Layout code: unit `003` (`crates/core/src/layout.rs`).
- The target repo's first-parent commit count for the last 12 months is in `.private/targets.md`.
