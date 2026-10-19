---
title: "File tree panel"
status: done
description: "Show a collapsible file tree in the right column that centers the map on the entry under its cursor"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1b, tui]
priority: high
---

# File tree panel

## Intent

M1 review feedback: the user wants a collapsible file tree in the right column.
A tree is the familiar way to find a file by name.
Linking it to the map teaches where that file lives: moving through the tree moves the map to the entry.

## Specification

Panel:

- The tree replaces the district legend below the right column's header line and rule.
- Rows come from the compacted layout tree, so a chain shows as one row (`src/quayside/`), as on the map.
  The repo root has no row. Its child directories come first, sorted by name, then its files in grid order.
  Each directory lists its child directories first, then its files, in the same order.
- Directory row: indent (2 cells per depth), `▸` or `▾`, a folder icon (U+F07B closed, U+F07C open), then `name/`.
  A top-level directory (a district) shows in its district color, bold. Deeper directories show in ink2.
  Right-aligned in dim: the directory's file count, including all subdirectories (thousands format).
- File row: indent, the file-type icon and name colors from unit `015`, then the name.
- All directories start collapsed.
- Hidden entries (unit `016`) are not listed while hidden files are hidden.
- The last panel row is a key hint in faint: `t tree · j/k move · l/h open/close · esc`.
- When the rows do not fit, the list scrolls to keep the cursor row visible.

Focus and keys:

- `t` focuses the tree. `t` or Esc returns focus to the map. With no right column (below 90 columns), `t` does nothing.
- In tree focus:
  - `j`/`k` and down/up move the cursor one row.
  - `l`/right opens a collapsed directory; on an open directory it moves to the first child.
  - `h`/left closes an open directory; otherwise it moves to the parent row.
  - Enter and Space toggle a directory.
  - PageDown/PageUp move one panel height.
- `w` `a` `s` `d`, `.`, `?`, and `q` keep working in tree focus.
- The cursor row has the cursor background `#2d4155` and white text while the tree has focus. Without focus, the cursor row has no background.

Map link:

- Each cursor move centers the map on the cursor entry: a file's tile, or a directory's block clipped to the view, as the mockup's legend click does (`mockups/v3-template.html:477`).
- While the tree has focus, the cursor file's tile has the cursor background and a white bold name.
  A cursor directory's header text gets the cursor background.
- Collapsing or expanding never changes the layout. Only the camera moves.

Mouse:

- A click on a row focuses the tree, moves the cursor there, and centers the map. A click on a directory row also toggles it.
- The wheel over the tree scrolls the tree rows by 3. The wheel over the map still does nothing (unit `014`).

Help overlay: add `t  file tree (j/k, l/h, enter)`.
Update `docs/design.md` `Screen regions` and `Keys`.

## Validation

- [x] Unit tests on the tree rows for quayside: initial rows are the top-level directories then root files; opening a district lists its children with correct indent; the counts match the file counts per directory; hidden rows disappear when hidden files are hidden.
- [x] Key tests: `t` then `j` `j` `l` moves and opens as specified; `h` on a child goes to its parent; Esc leaves tree focus.
- [x] A test asserts that a cursor move centers the camera on the entry (clamped), and that the layout tiles are unchanged after opening and closing directories.
- [x] Snapshot tests: 140 × 40 with the tree focused and a district opened, and with the cursor on a file inside it.
- [x] Mouse test: a click on a directory row toggles it and centers the map; the wheel over the tree scrolls it.
  (Unit test only: herdr cannot send mouse events.)
- [x] Real-repo test: fully opening every directory lists every indexed file exactly once, and every directory block once.
- [x] Visual check on the real target repo: open a district, move to a file, and see the map center on its tile.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No search in the tree (the `/` search is a Quick Idea).
- No persistent selection. Unit `006` adds selection. The tree's highlight shows only while the tree has focus.
- No map-to-tree sync: panning the map does not move the tree cursor.
- The overview (unit `005`) and the selection panel (unit `006`) must fit around the tree later. Unit `005` puts the overview above the rule.

## Context

- M1 review, 2026-09-25.
- The tree data: `minimap_core::layout::Tree` (compacted nodes, files in grid order).
- District colors and counts: `minimap_core::district`.
- `Tab` is reserved for the CC focus (`docs/design.md` `Keys`), so the tree uses `t`.
