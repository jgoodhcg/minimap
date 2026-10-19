---
title: "Selection"
status: dropped
description: "Select files with arrows, hjkl, or the mouse, and show the file's identity in the selection panel"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-3, tui]
priority: high
---

# Selection

> Dropped 2026-09-25: unit `018` replaced it. The tree cursor and the selection became one cursor.

## Intent

Selection turns the map from a picture into a tool.
It must never move a tile or resize the map, so the right column stays fixed.

## Specification

- Cursor movement per `mockups/v3-template.html:629-641`:
  arrows and `h` `j` `k` `l` move to the best file in that direction. With no selection, the first press selects the file nearest the view center (`:295-300`).
- The camera follows the cursor only as far as needed to keep the tile visible (`:301-306`).
- Mouse: a click on a tile selects it. A click on empty map deselects. Hover gives the tile the background `#1e2a36`.
- Esc deselects.
- Selected tile: background `#2d4155`, name in `#f3f6f9` bold, and a `▔` underline in the gap row below it (`:382-387`).
- The overview marks the selected file's cell inverted (priority 10).
- The breadcrumb shows the selected file's directory, with the last part bright.
- Selection panel, replacing the legend while a file is selected (`:492-502`):
  name (file-type color, bold), directory, `N lines · district` (district in its color), and a key hint line `y path · esc`.
- `y` copies the repo-relative path with the `arboard` crate and flashes `Copied path: <path>` in the top bar for 3.2 s. If the clipboard fails, flash the path with `Copy failed`.

## Validation

- [ ] Unit tests on a small hand-built tree where the neighbor in each direction is unambiguous.
- [ ] Unit test: selecting, deselecting, and moving the cursor never change `Layout` tile positions, and the map widget width stays the same.
- [ ] Snapshot tests at 150 × 45 on the quayside fixture: a selected file with the panel; no selection with the legend.
- [ ] Visual check (see `AGENTS.md` `Visual check`): `l` `l` `j` moves the cursor, the panel updates, Esc returns to the legend.
- [ ] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No shape, outline, or git sections (units `007`, `008`).
- No Enter action (unit `013`).

## Context

- `docs/design.md` `Selection panel`, `Keys`, `Stability principle`.

## Open Questions

- Unit `017` put a file tree where this unit's selection panel replaces the legend.
  Decide whether the selection panel replaces the tree while a file is selected, or sits below it, and whether the tree cursor and the map selection become one cursor.
