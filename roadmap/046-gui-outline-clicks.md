---
title: "GUI outline clicks"
status: done
description: "Click an outline item in the selection panel to zoom the map to its line and mark the line on the card; scroll the outline; click a directory's largest files"
created: 2026-10-02
updated: 2026-10-02
tags: [milestone-g4, gui]
priority: medium
---

# GUI outline clicks

## Intent

In the TUI, Enter on an outline item opens the viewer at its line. The GUI has no viewer.
A zoom to the item's line on the card shows where the definition lives in the file's shape, which is what the map is for.
This unit makes the panel's rows act: outline items, the outline's scroll, and a directory's largest files.

## Specification

Outline items:

- A click on an outline row picks that item. The picked row has the background `CURSOR` and white text, as the TUI's browsed item. A hovered outline row has `TILE_HI`.
- The pick moves the map, eased over 240 ms (`ZOOM_TO_EASE`), to the card at a card width of 420 points (`ZOOM_TO_WIDTH`), centered horizontally.
  A card that fits the region's height is centered in it. For a taller card, the item's line is 24 points (`ZOOM_TO_TOP`) below the region's top.
  The camera limits apply. A line past a capped card's shown lines (unit `041`) counts as its last shown line.
- The card marks the picked line: a bar in `WHITE` across the card's left padding, centered on the line, as tall as a line and at least 2 points.
  It shows from the shape level up while the shape toggle is on. It changes no card's rect.
- The panel's shape (unit `045`) marks it as the TUI does: `◀` in `WHITE` in the marker column at the line, in place of its tick,
  and the sample bar that holds the line in `WHITE`.
- A click on the picked row picks it again, and the map moves to it again. Another selection, or Esc, clears the pick.
- No outline keys: `j` and `k` stay the tree's keys, and `o` stays unbound.

Outline scroll:

- The wheel over the outline rows scrolls them by rows: 3 rows per notch, and trackpad points by the row height, as the tree scrolls.
  The first row shown stays from 0 to the first row of the last full page. `… K more` counts the items below the rows shown,
  and `K above` replaces the count at the label's right while rows are above.
- A new selection shows the outline from its first item. A pick keeps its row on screen.
- The wheel over the rest of the panel does nothing, as in unit `044`.

Largest files:

- A click on a largest-file row of a directory selects that file, as a tree click does: the tree opens to it, and the map moves only as far as needed, with no zoom change.
  A double-click on the row zooms to the file, as `f` does.

Hints: the outline label reads `OUTLINE · click to zoom`.

Code: `panel::build` returns the rects of its clickable rows and of the outline's rows. `scene::Options` gains the picked line, so the scene draws the mark.

Docs: `docs/design.md` `GUI (prototype)` (the panel's clicks and the mark, and the key table).

## Validation

- [x] GUI app-state tests on `quayside`, with no window:
  - A click on an outline row picks it. The camera eases to the card at width 420. For a tall card, the line is 24 points below the region's top; a short card is centered.
  - The scene has the mark at the line, inside the card's left padding, from the shape level with the shape on, and not below it or with the shape off.
  - No card's world rect changes.
  - Another selection and Esc clear the pick. A click on the picked row moves the map again.
  - The wheel over the outline scrolls it within its limits, and the camera does not move. A new selection starts at the top.
  - A click on a largest-file row selects the file, opens the tree to it, and shows its card with the margin. A double-click zooms to it.
- [x] Panel tests: the picked row's background; `◀` and the white bar on the shape; `K above` and `… K more` while scrolled.
- [x] Headless egui frames: a click on an outline row through `run_ui` picks it; the frames tessellate.
- [x] Real-repo test (read only): a pick of the longest hand-written file's last item puts its line 24 points below the region's top. Print the camera.
- [x] The headless frame probe on the real repo, in a release build: p95 under 8 ms, and the font atlas does not grow on the second sweep.
- [x] GUI visual check: at the review, after the user agreed (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-02)

- Panel: `panel::Hit` (`Outline`, `File`), `OutlineView` (the item rows, the first item, and the largest first item), `Panel::hits`, `hit`, and `rect_of`,
  and `OUTLINE_LABEL`. `Facts` gains `picked`, `outline_top`, and `hovered`. A clickable row spans the text and its 1-cell margins, as the TUI's selected-row fill.
  The outline shows from `outline_top`, clamped to the last full page; the label's note is `K above` while items are above.
- `GuiApp`: `pick`, `outline_top`, `panel_hover`, `outline`, `picked_line`, `pick_item`, `line_camera`, `click_panel`, and `scroll_outline`.
  `select` clears the pick and the scroll when the selection changes. A press on the panel records the row it began on, and the release acts only when it ends on that row.
- Scene: `Options::mark`, `Kind::Mark`, `MARK_MIN` 2, and `scene::line_top`, the world y of a line as the shape draws it, which `line_camera` uses.
- Deviation: the spec asked a double-click on a largest-file row to zoom to the file. The first click selects the file, so the file's panel replaces the directory's
  under the pointer, and the second click would land on the file's own rows (an outline item, for example). `click_panel` keeps the first click's time and point,
  and a second click there within 400 ms zooms to the file and picks nothing.
- Deviation: a pick does not hold the outline's scroll. The clicked row is on screen; the wheel can scroll it out of view afterwards, as the tree's wheel scrolls past the cursor.
- Addition: a hovered largest-file row hovers its card on the map, as a tree row does.
- The unit-044 and -045 tests use `OUTLINE_LABEL` for the new label.
- Tests: `crates/gui/tests/outline_clicks.rs`, 8 tests with no window: the zoom to a line on a tall card (24 points below the region's top) and on a short card (centered),
  the card's mark (in the padding, centered on the line, off with the shape off and below the shape level, and no card moved), the pick's clearing and a second click,
  the outline's scroll and its limits, a click and a double-click on a largest-file row, the picked row and `◀` and the white bar on the shape,
  and headless `run_ui` frames that hover and click rows.
- Real repo (read only, release build): a pick of the longest hand-written file's last item put its line 24.0 points below the region's top at zoom 2.5.
- Headless frame probe, now with a picked item where the file has one (release, atlas limit 8,192): p95 under 2 ms in both passes,
  for a short file and for the longest file, at 1 and 2 pixels per point. The atlas was the same size after both passes.
- Tests: 405 workspace tests pass, and no render snapshot changed. Real-repo tests: all 36 pass, the flaky fuzzy test included.
- GUI visual check, in the captures of unit `044` (2026-10-05): the outline label reads `OUTLINE · click to zoom`. The capture script sends no clicks,
  so the app-state tests check the pick, the mark, and the scroll.

## Scope

- No viewer, no editor handoff, and no outline keys.
- No clicks on the git commits (the GUI has no commit diff), the facts, or the shape.
- No change to the TUI.

## Context

- Decided in the plan (2026-10-02): a click zooms to the card with the line near the top and marks the line (see unit `044` `Context`).
- The TUI's outline browse: `crates/tui/src/ui.rs` `draw_outline` (`outline_top`, the browsed row), and `draw_shape` (`◀`, the white row).
- The GUI's zoom to a card: `GuiApp::card_camera` and `zoom_to` in `crates/gui/src/app.rs`.
- Units `044` and `045` come first.
