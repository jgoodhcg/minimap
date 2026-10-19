---
title: "View toggles, tree ends, and stats in the panel"
status: done
description: "Toggle the selection panel with p and the overview with v, jump to the tree's first and last rows with g and G, and move the repo stats to the top of the selection panel"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2b, tui]
priority: high
---

# View toggles, tree ends, and stats in the panel

## Intent

M2 review feedback.
The user wants to hide the selection panel and the overview whenever they are not useful, whatever the selection is.
The user wants to jump to the ends of the file tree.
The repo stats line crowds the map; it belongs at the top of the selection panel, as in unit `014`.

## Specification

Per `docs/design.md` `Screen regions`, `Overview`, `Keys`:

- `p` shows or hides the selection panel. It starts shown. Below 100 columns the panel stays hidden, and `p` flashes `The panel needs 100 columns`.
  Hiding it widens the map to the right. No tile moves, and the map content keeps its place on screen.
- The repo stats move to the top of the selection panel: the repo name (bold), then `N files · M dirs · K lines` (dim), right-aligned on the same row when both fit, else on the next row (the unit `014` rule).
  A `─` rule follows, then the selection.
  The map region starts on the row below the top bar, so it gains one row. While the panel is hidden, the stats do not show.
- `v` shows or hides the overview. It starts shown.
  While it is hidden, the camera's lower x limit is `-2`, selection scrolling ignores the overview, and a click there hits the map.
  Showing or hiding it moves the camera only as the clamp requires.
- `g` or Home selects the first tree row. `G` or End selects the last tree row. Each is a selection change: the tree and the map scroll to it.
- There is no deselect. The selection is the tree cursor, and it always exists. `p` hides the panel instead.
- The status bar shows the toggles after `. hidden` and `m pulse`: ` p ● panel ` and ` v ● overview ` (`○` when off).
- The help overlay lists `p`, `v`, and `g` / `G`.

## Validation

- [x] Unit tests: `p`, `v`, `g`, `G`, Home, and End never change `Layout` tile positions.
- [x] Unit test: hiding the panel widens the map region by the panel width, and a tile keeps its screen cell.
- [x] Unit test: with the overview hidden, the camera clamps at `-2`, and a click in the old overview box selects the tile below it.
- [x] Unit test: `g` and `G` select the first and last rows, and the selection is in view with the margin.
- [x] Unit test: `p` below 100 columns flashes and keeps the panel hidden.
- [x] Snapshot tests (quayside): 160 × 45 with the stats at the top of the panel; 110 × 35 with the stats on two rows; 160 × 45 with the panel hidden; 160 × 45 with the overview hidden.
- [x] Real-repo test: `G` then `g` select the last and first rows and draw them inside the map region.
- [x] Visual check on the real target repo: `p` hides and shows the panel; `v` hides and shows the overview; `G` and `g` jump; the stats show at the top of the panel.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No deselect key.
- No other panel content (units `007`, `008`).

## Context

- `docs/design.md` `Screen regions`, `Overview`, `Keys`.
- Unit `014` (archived) for the stats row rule; `crates/tui/src/ui.rs` `draw_side`, `draw_map`, `draw_status`; `crates/tui/src/app.rs` `View::new`, `clamp`, `ensure_visible`, `overview`.
