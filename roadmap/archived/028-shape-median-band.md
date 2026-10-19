---
title: "Shape median band"
status: done
description: "Draw long files' shapes from the median indent and length of each row, so a long file no longer reads as a solid block"
created: 2026-09-28
updated: 2026-09-28
tags: [milestone-3b, core, tui]
priority: medium
---

# Shape median band

## Intent

On a real repo, most Clojure files have at most 200 lines, and their shapes read well.
The longer files read as near-solid blocks.
Each dot row covers about 20 lines, the minimum indent is almost always 0, and the maximum length is almost always long.
A median band keeps the typical indent and length of each row, so long files show their structure.

## Specification

In `minimap-core` `shape`:

- `LineShape::sample` keeps the minimum indent and the maximum length for a row that covers at most 4 lines.
- A row that covers more than 4 lines keeps the median indent and the median length of its non-empty lines.
  The two medians are separate, and each is the lower median (nearest rank).
- A row of only blank lines keeps length 0, as now.
- `LineShape::decode` (the fake data) does not change. The fake repo's `m` field keeps its minimum and maximum.

In the TUI, `Braille::new` does not change. A dot row that spans several shape rows still takes their minimum indent and maximum length.

Fallback: if the visual check still shows long files as blocks, do not keep the median.
In that case, show the shape only for files up to 200 lines, and give its rows to the outline for longer files.
Record which path was taken in the completion report and in `docs/design.md`.

Update `docs/design.md` `Selection panel` (`Line shape data`) with the rule.

## Validation

- [x] Unit test: a 10-line file keeps one row per line with each line's indent and length.
- [x] Unit test: a 1,000-line file whose rows alternate short and long lines gets median values, not the minimum and maximum.
- [x] Unit test: a row of 5 lines with one blank line takes the medians of its 4 non-empty lines.
- [x] Real-repo test: for every Clojure file over 500 lines, count the dot rows (12 rows, 40 columns) whose right end differs from the row above by at least 2 dots.
  Print the median share for the minimum-and-maximum rule and for the median rule. The median rule's share must be higher.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: one file over 1,000 lines, one of 400–600 lines, and one under 200 lines.
  Save before and after captures under `.private/captures/`.
- [x] The quayside snapshots do not change, because the fake data keeps its own shape.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-28)

- The median band stayed; the fallback was not needed.
- On the real repo's Clojure files over 500 lines, the median share of lit dots fell to about half.
  The spec's right-end metric moved only a little: the old rule's right ends already varied.
  The block look came from the fill from column 0 to the right end, which the lit-dot share measures.
- Captures: `.private/captures/shape-median-band.txt` (before and after for a long, a medium, and a short file) and `.private/captures/visual-028-long-file-panel.txt`.
  The short file is unchanged; the long files show indented sections and short-line regions.
- `LineShape::from_text_with` takes the threshold, so tests can draw the old rule.

## Scope

- No change to the braille grid size, the markers, or the line numbers.
- The viewer's marks on the shape (visible lines, cursor, hunks) are units `029` and `030`.

## Context

- Shape code: `crates/core/src/shape.rs`, `crates/tui/src/shape.rs`, `crates/tui/src/ui.rs` `draw_shape`.
- Decided in the M3 review (2026-09-28): the median band, with the "short files only" fallback.
