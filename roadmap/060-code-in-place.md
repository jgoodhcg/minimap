---
title: "Code in place"
status: ready
description: "Zoom past the shape into a card until its file text draws in place, line for line, with syntax colors"
created: 2026-10-30
updated: 2026-10-30
tags: [milestone-g10, gui]
priority: medium
---

# Code in place

## Intent

The user enjoys zooming around the map (G2 reaction, 2026-10-02), but the zoom stops at the shape. To read code, the user leaves the map for the note reader,
and the reader opens only a note's file. `Direction` in `roadmap/index.md` asks for one surface where the user reads the code and the agent's work.
This unit is slice 1 of "the reader becomes a zoom level" (`Quick Ideas`, usage observations): a deeper zoom draws each card's file text in place of its shape.
Slice 2 (unit `061`) moves the notes, the tour, and text copy onto the map.

A line-scaled card (unit `039`) already gives each source line one row of `LINE_PT` (1) world point. The text fills rows that exist,
so this unit changes rendering only. No tile moves.

## Specification

Facts that set the numbers:

- Today `MAX_ZOOM` is 6, so a line is at most 6 points high on the screen, under the 7-point text threshold (unit `040`). Text cannot show.
- The reader draws 13-point text in 18-point rows (`CODE_ROW`). This unit keeps that ratio: code text is `CODE_PT` = 0.72 world points, so at zoom 18 it matches the reader.
- The shape spreads 120 columns (`SHAPE_CHARS`) across the body, about 1.23 world points per column. Monospace text at `CODE_PT` advances about 0.43 points per column,
  so a 120-column line covers about 35 % of the body. The text cannot line up with the shape's bars, so the two cross-fade.

Camera:

1. `MAX_ZOOM` goes from 6 to 24 (a card is 4,032 points wide; a line is 24 points high). The zoom runs from half the fit zoom to 24.
2. Reading zoom: 18, where a line is 18 points high and the code text is 13 points, as in the reader.

Code level, a new detail level after `shape`, by the line height on the screen (`zoom × LINE_PT`):

3. From a line height of 10 points (card width 1,680) to 13 points (card width 2,184), the shape fades out and the code fades in.
   From 13 points up, only the code draws. Below 10 points, nothing changes.
4. Each source line draws at its line's row on the card, from the body's left edge, at `CODE_PT`, through the text ladder (unit `040`).
   Tabs run to the next multiple of 4 columns. Lines do not wrap; each line is cut at the body's right edge.
   Only the rows in the map region draw, as for the shape.
5. Syntax colors come from the `reader::Colorer` thread (unit `055`), as in the reader. Until they arrive, and for a file over 20,000 lines, the text is `INK2`.
   Bold spans use the bold face, and italic draws upright.
6. Line numbers draw in `FAINT` at `CODE_PT`, right-aligned in the card's left padding, one column advance left of the body.
   The outline pick's line mark (unit `046`) draws above them.
7. The name row (icon, name, line count) fades out with the shape, over the same range. At the code level the card shows only code.
   The top bar's breadcrumb and hovered path still name the file. This changes the unit `040` rule "Card text does not fade" for the code level only.
8. A capped generated card (unit `041`) draws only its shown lines. The cut mark stays.
9. A card shows no text, and keeps its shape at every zoom, when:
   - the window shows an index JSON (`--from-json`),
   - the file is binary, empty, or over 2 MiB,
   - the file cannot be read (for example, a deleted file).

Text source:

10. When the set of visible cards at the code level changes, the GUI loads their text with `reader::Text::load` and asks the colorer for their colors,
    nearest to the region's center first, as the details loader does.
    It keeps at most 16 files' text and colors, and drops the least recently drawn file first.
11. The live index (unit `048`) reloads the text and colors of a changed file that is in the cache.

Toggles and keys:

12. `9` turns the code on or off (unbound since unit `039`). With the code off, the shape and the name row stay at every zoom.
    The status bar's toggles read `1 trail  2 notes  7 names  8 shape  9 code`. With `8` off, the code still fades in from its level.
13. Enter on the map, with a file selected, zooms to its code at the reading zoom over 240 ms.
    The card's left edge sits 24 points inside the region's left edge.
    The outline pick's line, else the first line, sits at a quarter of the region's height.
    Enter with a directory selected zooms to it, as `f` does. `f` and a double-click keep their 420-point card zoom.
14. In the reader, Enter closes it and zooms to the note's first line at the reading zoom, as in step 13. `f` keeps its zoom.
15. Clicks, hovers, and drags at the code level act as at other zoom levels: a click selects the card, and a drag pans. Text selection waits for unit `061`.
16. Update the key hints and the `Keys` table in `docs/design.md`.

Capture mode:

17. The `--capture` script adds steps at line heights 10, 13, and 18 (card widths 1,680, 2,184, and 3,024), centered on the selection's first lines.
    The zoom sweeps run from the fit view to card width 3,024.

## Validation

- [ ] Camera tests: the zoom clamps at 24; Enter on a file eases to zoom 18 with the step 13 target; Enter on a directory equals `f`; `f` keeps width 420.
- [ ] Scene tests on `quayside`: below line height 10, the scene has no code items and is unchanged; at 11.5, the shape, the name row, and the code draw at partial opacity;
  at 18, only code, line numbers, and the cut mark draw, and the code rows are the rows inside the region.
- [ ] Scene tests: tabs expand to multiples of 4; a long line is cut at the body's right edge; a capped card draws only its shown lines.
- [ ] Scene tests: `--from-json`, a binary file, an empty file, and a file over 2 MiB keep their shape at zoom 18 and draw no code.
- [ ] Colorer test: a card at the code level draws `INK2` text, then colored spans after the thread answers (wait in the test).
- [ ] Cache tests: the 17th file drops the least recently drawn one; a live-index change reloads a cached file's text.
- [ ] Toggle tests: `9` off keeps the shape and the name row at zoom 18; `8` off with `9` on draws the code from its level.
- [ ] Stability: the layout and the card rects are unchanged. The layout tests and snapshots pass unchanged.
- [ ] Real-repo tests: at zoom 18 over the real map's far corner, the camera maps world points to screen points within 0.25 points;
  the zoom sweep to card width 3,024 keeps the frame p95 under 8 ms in release, as the current sweep test asserts. Timings go under `.private/`.
- [ ] All `AGENTS.md` validation levels 1–4b pass.
- [ ] GUI visual check: at the milestone exit, only with the user's consent (it opens a window). Check the cross-fade at line heights 10 to 13,
  the colors at 18, and whether the line numbers read in the padding.

## Scope

- Rendering and camera only. No layout change, and no tile moves.
- No notes on the code, no `n`/`N` tour on the map, and no text selection or copy on the map. These belong to unit `061`. The note reader stays as it is.
- No outline labels at middle zooms (the first sentence of the `Quick Ideas` item stays there).
- No line-level glows for trail events. No diff view on the card.
- No change to the TUI.

## Context

- `crates/gui/src/camera.rs` (`MAX_ZOOM`, the zoom range), `crates/gui/src/scene.rs` (`shape`, `ladder`, `fade`, the detail levels), `crates/gui/src/paint.rs`.
- `crates/gui/src/reader.rs` (`Text::load`, `Colorer`, `colorable`), `crates/gui/src/app.rs` (`zoom_to`, `ZOOM_TO_WIDTH`, the key handling, `request_colors`).
- `crates/gui/src/capture.rs` (`WIDTHS`, the sweeps), `crates/gui/tests/real_repo.rs` (the frame probe, p95 under 8 ms).
- `docs/design.md` `GUI (prototype)`: detail levels, text in the world, card geometry, shape, cut mark, note reader, keys, capture mode.
- Units `039` (line-scaled cards), `040` (text in the world), `041` (cut mark), `046` (outline pick), `054`/`055` (reader and colors).
- Defaults taken (2026-10-30), for the user to override at review: `MAX_ZOOM` 24; reading zoom 18; the cross-fade range of 10 to 13 points;
  the name row fades at the code level; `9` toggles the code; Enter zooms to the code, and `f` keeps its zoom.
