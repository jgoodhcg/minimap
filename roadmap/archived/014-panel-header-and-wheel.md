---
title: "Panel header stats and no wheel panning"
status: done
description: "Show file, directory, and line counts beside the repo name in the right column, and stop the mouse wheel from panning the map"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1b, tui]
priority: high
---

# Panel header stats and no wheel panning

## Intent

M1 review feedback.
The repo stats belong next to the repo name, at the top of the right column.
The mouse wheel must not move the map. Keys and drag are the ways to pan.
Zoom was considered and dropped: the terminal's own font zoom (for example Cmd+= in WezTerm) already rescales the map, and minimap redraws at the new size.

## Specification

- Right column, first line: the repo name (ink, bold) on the left.
  On the right, in dim: `N files · M dirs · K lines`, with `N` in thousands format (`2,585`) and `K` in `kfmt` format (`312k`).
  `M` is the directory block count (`Tree.nodes.len()`).
- When the name, one space, and the stats do not fit the column width, the stats go on the next line, right-aligned.
- Remove the `N directories · K lines` line below the district legend.
- The mouse wheel does nothing over the map. Wheel events must not change the camera.
- Remove wheel panning from the help overlay. The mouse line reads `drag to pan`.

## Validation

- [x] Snapshot tests: update the four `render` snapshots. At 140 × 40 the stats share the name line.
- [x] A test at a 100-column terminal (right column 40 wide) shows the stats on the second line.
- [x] The mouse test asserts that wheel events leave the camera unchanged, and that drag still pans.
- [x] Visual check on the real target repo: the stats show beside or under the name. The wheel does not move the map. Drag pans.
  (herdr cannot send mouse events. The mouse test covers the wheel and drag; the visual check covered the stats.)
- [x] All `AGENTS.md` validation levels 1–4 pass, including the real-repo tests.

## Scope

- No zoom levels.
- No new stats.

## Context

- M1 review, 2026-09-25.
- Right column writer: `crates/tui/src/ui.rs` (`draw_side`, `Column`).
- Mouse input: `crates/tui/src/app.rs` (`on_mouse`).
