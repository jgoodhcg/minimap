---
title: "GUI panel shape"
status: done
description: "Draw the selected file's whole shape in the selection panel, with line numbers and a tick at each definition, as the TUI's SHAPE section"
created: 2026-10-02
updated: 2026-10-02
tags: [milestone-g4, gui]
priority: medium
---

# GUI panel shape

## Intent

The TUI's selection panel draws the whole file as braille, with line numbers and a marker at each definition (`docs/design.md` `Selection panel`, item 3).
A card shows its shape only from a card width of 60 points; the panel's shape shows the whole selected file at every zoom,
and its ticks show where the outline's items sit in the file.

## Specification

Panel (`crates/gui/src/panel.rs`), for a file, between the file facts and the outline, as in the TUI:

- `SHAPE` (`LABEL`, bold), then the shape of the file's `LineShape` (at most 64 samples, `FileDetails::shape`), drawn as bars.
- The box: the text width less a gutter of 5 cells at the left and 2 cells at the right, at most 40 cells wide.
  Its height is `rows` text rows: 40 % of the rows that are free above the git group and the hint row, from 4 to 12, as the TUI counts them.
  The section shows when more than 7 rows are free.
- One line is `box height / lines` points high, at most a quarter row (5 points), as one braille dot row in the TUI:
  a short file does not stretch, and it fills the top of the box. The section takes the rows that its lines use, rounded up, then a blank row.
- The box is `MINI_BG`, and the bars are `MINI_DOT`, the TUI's colors. A sample that covers lines `a` to `b` is a bar from its indent to its length,
  across 126 text columns (`MAX_LEN`), as tall as its lines, less a 1-point gap when it is 3 points or taller.
- Line numbers in `FAINT`, right-aligned in the gutter, as in the TUI: the first line at the first row, the first line of the middle row,
  and the line count at the last row of the lines.
- Definition ticks: in the marker column, 1 cell right of the box, a tick in `FAINT` at each outline item's line: 4 points wide, as tall as a line, at least 1 point.
- No section for a binary or empty file, or while the file's details load.

Docs: `docs/design.md` `GUI (prototype)` (the panel's shape).

## Validation

- [x] Panel tests on `quayside`, with no window:
  - The shape's items lie between the file facts and the outline, and every bar lies inside the box.
  - A line's height: a file with fewer lines than the box holds at 5 points uses 5 points per line from the box's top; a long file fills the box.
  - The line numbers read `1`, the middle row's first line, and the line count.
  - One tick per outline item, at its line's height in the box.
  - `rows` clamps from 4 to 12, and the section drops when 7 rows or fewer are free. A binary file has no section.
- [x] Headless egui frames with the shape tessellate.
- [x] Real-repo test (read only): the shape of the longest hand-written file (its bars and ticks). Print them.
- [x] The headless frame probe on the real repo, in a release build: p95 under 8 ms, and the font atlas does not grow on the second sweep.
- [x] GUI visual check: at the review, after the user agreed (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-02)

- `panel::shape_box`, with `GUTTER` 5, `SHAPE_COLS` 40, `SHAPE_ROWS` (4, 12), `SHAPE_LINE` 0.25, `SHAPE_GAP_FROM` 3, and `TICK_W` 4.
  The shape shows by the TUI's rule: `avail` is the rows to the hint row less the git group and its gap, and the section needs more than 7.
- A line is `rows × 20 / lines` points high, at most 5. The section takes the rows that the lines use, rounded up, then a blank row; the TUI takes the whole box.
  The 64 samples of `LineShape` draw as bars; `measured` (every line) stays for the cards.
- The unit-044 tests changed where the shape takes rows: the file test runs in a window 1,200 points high, so the whole outline fits,
  and the cut-outline test at 600 points shows 4 items and `… 15 more`.
- Tests: `crates/gui/tests/panel_shape.rs`, 6 tests with no window: the place between the facts and the outline with the bars in the box,
  a 47-line file at 5 points a line from the box's top, the line numbers (1, 114, and 272 for a 272-line file), one tick per outline item at its line,
  the row limits (no shape at 7 free rows, 4 rows at 8, 12 in a tall window) and no shape for a binary file, and a headless frame that tessellates.
- Real repo (read only, release build): the longest hand-written file's shape fills the box with 64 bars and a tick per outline item.
- Headless frame probe (release, atlas limit 8,192): p95 under 2 ms in both passes, for a short file and for the longest file, at 1 and 2 pixels per point.
  The atlas was the same size after both passes.
- Tests: 397 workspace tests pass, and no render snapshot changed. Real-repo tests: 34 of 35 passed in the parallel run;
  `fuzzy_ranking_of_the_whole_index_takes_under_a_frame` failed under load again and passed alone.
- GUI visual check, in the captures of unit `044` (2026-10-05): the 27-line file's shape takes 7 rows at 5 points a line, with `1`, `13`, and `27`;
  the longest file's fills 12 rows, with its first, middle, and last line numbers. For the review: a long file's 64 samples read as stripes (3.75 points each, with a 1-point gap),
  and a tick on a long file is 4 x 1 points in `FAINT`, which is hard to see.

## Scope

- No `◀` for a picked outline item: unit `046`.
- No viewer fill and no changed-line marks: the GUI has no viewer and no diff.
- No hover or click on the shape.

## Context

- The TUI's shape: `crates/tui/src/ui.rs` `draw_shape`, and `crates/tui/src/shape.rs` (`Braille`: one dot row per line for a short file, `MAX_LEN` columns).
- The card's shape: `crates/gui/src/scene.rs` `shape` (the bar from indent to length, and the 1-point gap).
- Unit `044` comes first.
