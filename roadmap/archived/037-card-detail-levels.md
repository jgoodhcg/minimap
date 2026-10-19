---
title: "Card detail levels and toggles"
status: done
description: "Show each card's file name, shape, line count, and outline as the zoom allows, and let the user toggle names, shapes, and outlines with 7, 8, and 9"
created: 2026-10-01
updated: 2026-10-01
tags: [milestone-g1, gui]
priority: high
---

# Card detail levels and toggles

## Intent

The user asked for a card per file with an icon, the file name, and the shape of its code (2026-10-01).
The shape gives a sense of the file's size and acts as a visual aid to remember the file.
The user also asked to toggle the shape, the names, and the outline.
Zoom decides what fits on a card. The toggles decide what the user wants to see.

## Specification

Detail levels, by the card's width `w` on the screen, in points. Each level adds to the levels above it:

| Level | From `w` | Adds |
|---|---|---|
| block | 0 | A filled rect in the type color (unit `035`) |
| icon | 20 | The card and the icon (unit `035`) |
| name | 60 | The file name |
| shape | 110 | The shape and the line count |
| outline | 160 | The outline items |

Card geometry, as fractions of the card. A toggle or a zoom never moves an element inside the card:

- Padding: `p` = 6 % of the card width.
- Name row: at the top, with height `max(card height / 6, name size + 4)`. The icon, then the name in `INK`, cut with `…` at the end when it does not fit.
  At the shape level and above, the line count is right-aligned in the row in `DIM`, as digits only.
- Body: from below the name row to `p` above the bottom.
  The shape column is at the left, 28 % of the card width. The outline column runs from `p` right of the shape column to `p` left of the right edge.
- Text sizes snap down to the ladder of unit `035`. The name size is `clamp(0.13 × card height, 9, 15)`. The outline size is `clamp(0.10 × card height, 9, 13)`.
  Text stops growing at the top of its range, so a larger card shows more outline rows, not larger text.

Shape, from the file's `LineShape` (`docs/design.md` `Selection panel`, the line shape data):

- Height: `body height × min(1, sqrt(lines / 1000))`, from the top of the body. A 90-line file uses 30 % of the body, and a file of 1,000 lines or more uses all of it.
- Each sample row is a bar from `indent` to `indent + len`, with 120 characters across the column. Longer rows clamp at the column's edge.
  The bar height is the shape height divided by the row count, less a 1-point gap when the bar is 3 points or taller.
- Fill: the type color at 70 % over the card.
- A file of more than 1,000 lines gets a 2-point line in `DIM` under the shape, because the scale stops there.
- A file with no shape (binary or empty) has an empty body. While its details load, the shape column shows a rect in `FAINT` at 20 %.

Outline, from the file's outline items:

- One item per row: the name, indented 2 characters per depth level. Private items in `DIM`, others in `INK2`. No line numbers.
- When the items do not all fit, the last row shows `+N` in `DIM`, where N counts the items not shown.

Toggles:

- `7` names, `8` shape, `9` outline. All are on at startup.
- The status bar shows `7 names  8 shape  9 outline`, each in `INK` when on and `FAINT` when off. A click on an entry toggles it.
- A toggle that is off removes its element at every level. The other elements keep their places in the card.

Details:

- The GUI owns a `details::Loader` (unit `036`). With `--from-json`, the loader uses the JSON's details.
- When the set of visible cards at the shape level or above changes, the GUI requests their details, nearest to the map region's center first.
- The loader's `wake` callback calls `request_repaint` on the egui context.

Capture mode (unit `035`) adds steps at card width 200 with each toggle off in turn, and one step at width 400 with all toggles on.

## Validation

- [x] Scene tests on quayside with `crates/core/tests/fixtures/quayside-details.json`:
  - The level changes exactly at each threshold (for example, `w` 59.9 is icon and 60 is name).
  - The shape rect and the outline rect of a card are the same with each toggle on and off, and at two zooms with the same level.
  - The shape height follows the formula: 90 lines give 30 %, 1,000 and 5,000 lines give 100 %.
  - Every text size is on the ladder.
  - An outline with more items than rows ends with the right `+N`.
  - An insta snapshot of one card's draw items at `w` 200.
- [x] A toggle never changes a card's world rect (the test lists every card).
- [x] Real-repo test (read only): at card width 200 on the map center, every visible card has details within 1 s in a release build. Print the time.
- [x] GUI visual check on the real target: names, shapes, line counts, and outlines show at their levels, each toggle removes only its element,
  and short and long files have visibly different shapes. Keep every PNG under `.private/`.
- [x] `frames.txt` on the real target, in a release build: the p95 frame CPU time is under 8 ms with all toggles on. The font atlas check of unit `035` still holds.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass.

## Result (2026-10-01)

- Levels at card widths 20 (icon), 60 (name), 110 (shape), and 160 (outline) points. `scene::Level`, `scene::Toggles`, and `scene::CardGeometry` hold them.
- The shape scale stays at `sqrt(lines / 1000)`: on a real repo, short, medium, and long files read as clearly different heights at card width 200,
  and a typical file of 50 to 200 lines uses 22 % to 45 % of the body. The constant is `scene::SHAPE_FULL`.
- Deviations and choices:
  - A shape bar runs from the row's indent to its end column: `ShapeRow::len` is the end column (the TUI's braille shape reads it so), not a length after the indent.
  - The line count belongs to the shape toggle: `8` removes the shape and the count. Both say how long the file is.
  - The name row starts half a padding below the card's top. The name starts at the icon's square's right edge.
  - The loading rect is as tall as the shape will be, from the index's line count, so the shape replaces it in place. Binary and empty files show none.
  - The `+N` row takes the last row that fits. A row is 1.25 times the outline text size.
- `details::Loader` runs in the GUI: `--from-json` preloads the JSON's details, and a repo gets `available_parallelism - 1` workers.
  Each frame, the GUI asks for the visible cards at the shape level or above, nearest to the center first, when that list changed. `wake` calls `request_repaint`.
- `scene::scene` takes a `Lookup` for details (the loader, `Details`, a `HashMap`, or `NoDetails`). `Body` replaced `Text` for what an item draws, with `Bars` for the shape.
- Capture mode adds `w200-no-names`, `w200-no-shape`, `w200-no-outline`, and `w400-all`. Each step waits for its visible cards' details (at most 900 frames).
- Real-repo test (read only, release build): at card width 200 on the map center, the visible cards had details well within 1 s.
- `frames.txt` on the real repo, release build, all toggles on: p95 under 4 ms in both passes.
  The font atlas stayed small and almost empty after both passes, as in unit `035`.
- Capture on the real repo, 2026-10-01 13:36, after the screen was unlocked (it was locked from about 12:05 to 13:35, and a locked screen draws no window).
  It ran the build of unit `038`, so the selected card has its ring. Names, shapes, line counts, and outlines showed at their levels,
  each toggle removed only its element, and nothing else moved. PNGs: `.private/captures/037/`.
- For the G1 review: at card width 80 the file names show (9 points), but directory names below depth 1 do not, because a header name needs a header row of 11.25 points (card width 105).

## Scope

- No selection and no hover (unit `038`).
- No syntax colors or code text in the cards. The shape stands for the code.
- No outline browse, and no jump from an outline item to a line.
- No change to the layout. Cards keep one size.

## Context

- Shape data and sampling: `crates/core/src/shape.rs` (`LineShape`, `ShapeRow`, `MAX_LEN` 126). The TUI's braille shape: `crates/tui/src/shape.rs`.
- quayside line counts in `mockups/fake.json`: most files are short, and the generated lock file is the longest. The square-root scale keeps short files visible and still tells long files apart.
  The constant 1,000 is a first guess. Tune it at the visual check, and record the value in the Result.
- Outline items: `crates/core/src/outline.rs` (`OutlineItem`: `line`, `kind`, `name`, `private`, `depth`).
- Keys `7`–`9` follow the TUI's layer keys `1`–`6` and do not clash with any TUI key (`docs/design.md` `Keys`).
