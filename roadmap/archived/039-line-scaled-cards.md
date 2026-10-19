---
title: "Line-scaled cards"
status: done
description: "Make each card as tall as its file's lines, with one line the same height on every card, and take the outline off the cards"
created: 2026-10-01
updated: 2026-10-01
tags: [milestone-g2, gui, core]
priority: high
---

# Line-scaled cards

## Intent

G1 review feedback (2026-10-01).
The user wants the cards to show how long each file is: one line of code has the same height on every card,
so a card's height is proportional to its line count, and cards are not one size. The card width stays the same for every card.
This is the SeeSoft view (Eick, 1992): each file is a column, and each line is a thin row.
The outline takes much of the card, and it is not useful at card size. It leaves the cards; a later selection panel shows it.

This reverses a settled rule. `docs/design.md` `Stability principle` says every tile has one size,
because a growing file reshuffles the packing. The user accepted the trade. Rounding keeps the moves small, and a stability report measures them.

## Specification

Layout (`minimap-core`), `CARDS` only. `CHARS` and `TILES` do not change.

- The tree carries each file's line count from the index: `IndexFile::lines`, which is 0 for binary and missing files.
  The layout of `CARDS` is then a deterministic function of the file tree and the line counts.
- One line is `LINE_PT` points high at zoom 1. Start with 1 point, and tune it at the visual check.
- A card is the full card width (`tile_size().0`, 21 cells) and has a name row of 1 cell row, then a body.
  The body is `ceil(step(lines) × LINE_PT / PY)` rows, where `step` rounds the line count up to a `Growth(1.25)` step.
  A file with 0 lines has no body. A file keeps its card height until its line count passes a step.
- Files in a directory flow in name order into columns: down a column, then to the next column.
  The column count comes from the rounded capacity, as today. The column height is the least height at which the files fit in that many columns,
  and at least the tallest card. It rounds up to a `Growth(1.25)` step.
  Hidden files keep their place after the other files, so the hidden toggle moves no card.
- Blocks at depth 1 and 2 keep their rounded heights. Child blocks pack in shelves as today.
- `root_k` and `fc_k` can change at the visual check. Record the values in the Result.

Cards (`minimap-gui`):

- No outline on the cards: remove the outline level, the `+N` row, the `9` toggle, and `9 outline` in the status bar. `9` is unbound.
  The details loader still computes outlines for a later selection panel.
- The name row is at the top of the card. Sizes that came from the card height (the icon, the name, the name row) use `w × 108 / 168`,
  the card height of today for card width `w`, so the name row is the same on every card. Unit `040` replaces these sizes.
- The body runs from below the name row to the bottom padding. The shape uses the full body width, with 120 columns across it.
- The shape is `lines × LINE_PT` high at zoom 1, from the body's top. The rounding leaves at most a quarter of the body empty below it.
- Each line has its own row: `FileDetails` keeps the measured lines (indent and end column per line, 4 bytes each) beside the 64-row `LineShape`.
  The GUI samples the measured lines to the card's rows on the screen, at most one sample per screen pixel row. The TUI keeps `LineShape`.
  With `--from-json`, the details have only the 64-row shape: each row then draws `lines / rows` lines high.
- A bar is 1 point shorter than its row when the row is 3 points or taller on the screen.
- The 2-point line under shapes of more than 1,000 lines goes: the scale has no cap now.
- At the block level, a card's rect has the card's height, so the map shows file lengths from the fit view.
- The loading rect stays: as high as the shape will be, from the index's line count.
- Zoom to a card: the card is 420 points wide and centered horizontally. A card that fits the region's height is centered in it;
  a taller card has its top 24 points below the region's top.
- The detail-level thresholds can change at the visual check, because cards now have no outline. Record the values.

Stability report:

- Extend the `stability` example (unit `009`) with `--cards`. It compares today's `CARDS` (one card size) with the line-scaled `CARDS`
  over a real repo's last 12 months (50 samples). It reads each file's line count at each sampled commit from its blob, cached by blob ID.
- The report gives, for both layouts: summed card moves, the share of cards that moved, the map size, and the fill (card area over map area).
  Write it to `.private/stability-<date>.md`.
- The report is not a gate. The Result records its numbers for the G2 review.

Docs:

- `AGENTS.md` `Project-Specific Rules`: the layout is a deterministic function of the file tree and, for `CARDS`, the files' line counts.
- `docs/design.md`: `Stability principle` (one card width; card heights from rounded line counts), the `CARDS` column of `Layout`,
  and `GUI (prototype)` (card geometry, shape, levels, keys).

## Validation

- [x] Core layout tests on quayside (`crates/core/tests/fixtures/quayside-files.json`, with line counts):
  - A card's body rows follow the formula, and a 0-line file has no body.
  - A file that grows within its step moves no card.
  - A file that grows past its step moves no card outside its depth-2 ancestor block while that block's rounded height holds.
  - Files in a directory are in name order down each column, and no two cards overlap.
  - The hidden toggle and the selection change no card's world rect.
  - The `CARDS` insta snapshot changes. The `CHARS` and `TILES` snapshots and the parity tests do not.
- [x] GUI scene tests: a 300-line file's shape is 6 times as high as a 50-line file's at any zoom; no outline item at any card width; `9` changes nothing.
- [x] No TUI snapshot changes.
- [x] Real-repo test (read only): the target's map size, card count, total card area against today's, and the tallest card. Print them.
- [x] The stability report on the target, under `.private/`.
- [x] GUI visual check on the target: long and short files differ in card height at every zoom,
  lines have one height across cards, and no card shows an outline. Run the capture with a short file and with the longest file as `--select`.
- [x] `frames.txt` on the target, in a release build: p95 frame CPU time under 8 ms. The font atlas check of unit `035` still holds.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass.

## Result (2026-10-01)

- Layout: `LayoutConfig::line_pt` (`LINE_PT` 1.0 for `CARDS`, 0 for `CHARS` and `TILES`), `body_rows`, `tile_rows`, and `Layout::tile_h`.
  `TreeFile::lines` carries the index's line count; `Tree::from_files` takes `(path, ignored, lines)`, and `Tree::from_paths` gives 0 lines.
  One-size layouts keep the row-major grid; the parity tests and the `CHARS` and `TILES` results did not change.
- Line-scaled files flow down columns in name order. The column height is found by a binary search over the greedy flow, then rounds up to a `Growth(1.25)` step.
  The greedy flow at the rounded height can leave a last column empty, in a few percent of the grid columns.
- `root_k` 3.2 and `fc_k` 0.7 stay. From 3.2 to 10, `root_k` changes nothing on the real repo or quayside: the widest district sets the root width.
- Real repo (read only): the map grew taller, the summed card area grew, and the fill fell, against one card size.
  The longest file gives a card many times taller than the median card, and a card at the fit view is a few points wide.
- Layout tests on quayside: a file that grows within its step moves no card. A file that grows past its step kept its rounded block's size
  in 242 of 254 sampled cases, and then moved nothing outside the block; in 12 cases the block changed, 9 of them by width only.
  Block widths are not rounded, so the condition is the block's size, not only its height.
- Stability report (`.private/stability-2026-10-01.md`, 50 samples of the last year's first-parent commits):
  line-scaled cards moved several times more than one card size, and more sample pairs moved. Name order and rounded widths cut the moves a little,
  but made the map larger and sparser.
  Most line-scaled moves shift whole districts by one column (22 cells): every district is in one shelf, and a district's width changes when its children repack.
  The two variants use knobs the layout already has. They are measured for the G2 review and not adopted.
- Details: `FileDetails::measured` keeps every line (`shape::measure_lines`, 4 bytes per line), beside the 64-row shape. `shape::summarize` is the row rule
  that `LineShape::sample` and the GUI share. JSON details leave `measured` empty.
- Cards: no outline, no `+N`, no `9`, no scale line. The name row is the card's first layout row, and the body runs from it to the card's bottom.
  Deviation: the G1 card had half a padding above the name row and a padding under the body (15 points at width 168).
  The body formula leaves no room for them: a card's body can be as little as 1 point taller than its shape. So the padding is at the sides only.
  The icon and the name keep the G1 formulas on the base height `w x 108 / 168`, capped at the name row's height.
- The shape draws one bar per line, or one sample per run of lines below a pixel row (`scene::lines_per_sample`, with `Options::pixels_per_point`),
  and only the rows in the map region. The bars go to the painter as one mesh of plain quads per shape.
  The emptiest body of 10 rows or more on quayside is about a quarter empty; shorter files have more room under the shape (a 1-line file: 17 of 18 points).
- Zoom to a card: `GuiApp::card_camera`. Capture steps at a card width use it too, so a tall selected card shows its top.
- Real-repo GUI test: the cards nearest the map center (the center itself is empty now) got details within a few milliseconds, and the scene at width 200 drew their shape bars.
- Visual check on the real repo, 2026-10-01 19:07, in a release build: two captures, with a short file and with the longest file as `--select`.
  The screen was locked from 17:41 to 19:06; a first try at 17:55 drew nothing, and the watchdog stopped it.
  Card heights follow the line counts at every zoom: at the fit view, long files are tall bars; at card widths 130 and 200, one line has one height on every card.
  No card shows an outline. The zoom to the longest file puts its top 24 points under the region's top. PNGs: `.private/captures/039/`.
- `frames.txt`: p95 under 5 ms in both passes for both files.
  The font atlas was the same size after the second sweep.
- Tuned values: `LINE_PT` stays 1.0. At card widths 130 to 200 a line is 0.8 to 1.2 points, and the shapes read as code.
  The thresholds stay 20, 60, and 110: the G1 text sizes have a 9-point floor that overflows a smaller card's name row. Unit `040` moves them.
- For the G2 review: the empty space grew. A directory's grid is as tall as its tallest card, and its child blocks start below the grid,
  so a district with a long file of its own (a lock file, for example) leaves a tall empty band beside that card. Below card width 110 a card is an empty box.

## Scope

- No change to text sizing beyond the card-width rule above. Unit `040` makes text scale with the zoom and sizes directory names.
- No selection panel and no outline anywhere in the GUI.
- No work on empty space: the user deferred it (G1 review). The report measures the fill for later.
- No change to the TUI's map or its behavior.
- No direct `winit` dependency, so the capture window still takes the focus. The user did not confirm the crate at the G1 review.

## Context

- Decided in the G1 review (2026-10-01): the card width is locked, the height follows the line count, and the outline leaves the cards.
  The user left the method to the agent: failures in the work or at the review show whether it meets the standard.
- On a real repo, most text files are short and a few are thousands of lines long.
  At 1 point per line with the rounding above, the median card is shorter than today's card, and the tallest is many times taller.
  The summed card area grows against today's.
- Layout: `crates/core/src/layout.rs` (`LayoutConfig::CARDS`, `Tree`, `TreeFile`, `Layout::compute`, `tile_size`). Index line counts: `crates/core/src/index.rs`.
- Shape: `crates/core/src/shape.rs` (`LineShape::sample`, `ShapeRow`, `MAX_ROWS`). Details: `crates/core/src/details.rs` (`FileDetails`).
- Card drawing: `crates/gui/src/scene.rs` (`CardGeometry`, `Level`, `Toggles`, `SHAPE_FULL`). Stability tool: `crates/core/examples/stability.rs`.
- Unit `027`'s rounding record: `roadmap/archived/027-quantized-blocks.md`.

