---
title: "Three-column screen and one selection"
status: done
description: "Move the file tree to a toggleable left column, add the selection panel on the right, and make the tree cursor the one selection"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2, tui]
priority: high
---

# Three-column screen and one selection

## Intent

The v5 mockup review settled the screen: file tree on the left, map in the middle, selection panel on the right.
One selection, shared by the tree and the map, removes the separate tree focus mode that unit `017` added.
The tree is how the user moves through the codebase; the map shows where that is.

## Specification

Screen regions, per `docs/design.md` `Screen regions` (reference: `mockups/v5-template.html` `view`):

- File tree, left column. It reuses the unit `017` rows and drawing.
  Width 34 by default; `[` and `]` change it by 4 within 16–80; dragging its right edge sets it.
  `t` shows or hides it. The first row is `FILES` and the visible file count; the last row is a key hint.
  When the terminal is narrow, the tree width shrinks so the map keeps at least 30 columns.
- Map, middle column. First row: the repo name (bold) and the stats that unit `014` put in the right column (`N files · M dirs · K lines`, dim).
  Below it, the existing tile map, clipped to the middle column. Unit `019` replaces the tile map.
- Selection panel, right column: width 0 below 100 columns, 44 below 160, else 54.
  - File: name (name color, bold), `directory/` (dim), `N lines · district` (district in its color), and a hint row `y copy path · ? keys`.
  - Directory: its path and `/` in the district color (bold), `N files · M subdirectories`, `K lines`, then its largest files as `relative/path  lines` while rows remain.
- Top bar: the breadcrumb of the selection, with the file name bright; the hovered path on the right while the mouse is over a tile.

Selection:

- One selection: a file or a directory. It is the tree cursor.
- At startup, select the first file in index order that is visible and not at the repo root, and open the tree down to it.
- `j` / `k`: next or previous tree row. `l`: open a closed directory; on an open directory, go to its first child. `h`: close an open directory; else go to the parent directory.
  Enter: open or close a selected directory. These keys work whether the tree is shown or hidden. Remove the unit `017` tree focus mode, its `t` focus key, and its focus-only highlight.
- Arrows: move to the nearest file in that direction on the map (`mockups/v3-template.html:629-641`), using the active layout's tile pitch.
- Any selection change opens the tree down to the selection and scrolls the tree to it.
- The map shows the selection: a file tile gets the background `#2d4155` and a white bold name; a directory's header text gets the background `#2d4155`.
  The camera scrolls only as far as needed to show the selection with a 2-cell margin (for a directory: its header and as much of the block as fits).
- `y` copies the selected repo-relative path with the `arboard` crate and flashes `Copied path: <path>` in the top bar for 3.2 s. If the clipboard fails, flash `Copy failed: <path>`.
- The hidden toggle (`.`) moves a selection that it hides to the first visible tree row.

Mouse:

- A click on a tile selects the file; a click on a directory header's text selects the directory.
- A click on a tree row selects it and toggles a directory. The wheel over the tree scrolls the tree.
- Dragging the map pans it. Dragging the tree's right edge resizes the tree. The wheel over the map does nothing.

The help overlay lists only the keys that work after this unit. `docs/design.md` `Keys` stays the full design.

## Validation

- [x] Unit tests on a small tree: `j`/`k`/`l`/`h`/Enter semantics; the arrows pick the expected neighbor; a selection change opens the tree to it.
- [x] Unit test: `t`, `[`, `]`, the tree drag, and selection changes never change `Layout` tile positions.
- [x] Unit test: after a selection change the selected tile is inside the map region with a 2-cell margin.
- [x] Snapshot tests (`TestBackend`, quayside fixture): 160 × 45 with a file selected; 160 × 45 with a directory selected; 160 × 45 with the tree hidden; 110 × 35 (panel 44 wide); 90 × 30 (no panel).
- [x] Mouse tests: tile click, header click, tree row click, tree wheel, map drag, tree edge drag.
- [x] Real-repo test: every visible tree row can be selected with `j` from the first row, and each selection is drawn inside the map region.
- [x] Visual check on the real target repo: `j`/`k` move the selection and the map follows; `t` hides and shows the tree; `[`/`]` resize it; `y` flashes the path.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- The map stays the unit `004` tile map. Unit `019` replaces it.
- No overview (unit `020`), no search (unit `021`), no shape, outline, or git sections (units `007`, `008`).
- Enter on a file does nothing yet (unit `013`).

## Context

- `docs/design.md` `Screen regions`, `File tree`, `Selection panel`, `Keys`.
- Mockup: `mockups/v5-template.html` (`drawTree`, `drawSide`, `drawDirInfo`, `treeMove`, `treeIn`, `treeOut`, `moveCursor`).
- Unit `017` code: `crates/tui/src/filetree.rs`, `crates/tui/src/app.rs`, `crates/tui/src/ui.rs`.
- This unit replaces the draft unit `006` (dropped) and resolves its open question: the tree cursor and the selection are one cursor.
