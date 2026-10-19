---
title: "File picker"
status: done
description: "Make / a Telescope-style picker: a fuzzy-ranked result list in the left column, the map as its preview, Enter to keep and Esc to go back"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2c, tui]
priority: high
---

# File picker

## Intent

The user asked for a search that shows results in a list with fuzzy matching, like their nvim Telescope `find_files`.
In minimap the map is the preview: moving through the list moves the selection, and the map shows where each result lives.

## Specification

Per `docs/design.md` `Search` (layout after Telescope's default: prompt at the bottom, best result just above it):

- `/` opens the picker in the left column, in place of the file tree. If the tree is hidden, the column shows while the picker is open.
  - Row 1: `SEARCH` and `N / M` (results / visible files) right-aligned.
  - The last row of the column: the prompt `> query▏`.
  - The rows between: the results, best at the bottom next to the prompt, worse results above it.
- A result row: the type icon in its type color, the file name in ink, then the directory in dim.
  Matched characters are bold in the search color `#3fd67a`. The highlighted row has the background `#2d4155` and white text.
- The query is scored with `minimap_core::fuzzy` (unit `025`) against the visible files: not ignored, and hidden files only while shown.
  An empty query lists every visible file in tree order, and the selection does not move.
- Each query change highlights the best result. The highlighted result is the selection: the map and the selection panel follow it.
- The results in the visible rows of the list are marked on the map with the search background (`docs/design.md` `Color`). Scrolling the list moves the marks.
  With an empty query nothing is marked.
- While the picker previews, the tree does not open to each result; Enter opens it to the kept result.
- Keys in the picker:
  - Text edits the query; Backspace deletes a character; Ctrl+W deletes a word; Ctrl+U clears.
  - Up or Ctrl+P moves the highlight up (to a worse result); Down or Ctrl+N moves it down (to a better one). PageUp and PageDown move by a page.
  - Enter closes the picker and keeps the highlighted result as the selection. The rows that were visible stay marked, and `n` / `N` cycle through them in rank order.
    The status bar shows ` /query  K of N · n/N · esc clear ` (unit `024`, with the kept count).
  - Esc closes the picker, restores the selection and the camera from before `/`, and clears the marks.
  - Ctrl+C quits.
- Mouse in the picker: a click on a result row highlights it; the wheel moves the highlight by 3 rows.
- The status bar while the picker is open: ` SEARCH  ↑↓ ^p ^n move · ⏎ keep · esc back `.
- The help overlay and `docs/design.md` `Keys` describe `/`, `n`, `N`.
- Remove the unit `021` substring matching (`search::find`) and the `MIN_QUERY` rule.

## Validation

- [x] Key tests: `/` opens the picker and shows all visible files; typing ranks and highlights the best result, and the selection follows; Up and Down move; Enter keeps; Esc restores the earlier selection and camera; `n` / `N` wrap through the kept rows.
- [x] Unit test: the marks on the map are exactly the visible result rows, before and after scrolling.
- [x] Unit test: matched characters in a row are bold and in the search color, at the positions from the matcher.
- [x] Unit test: with the tree hidden, `/` shows the column and Esc hides it again. No tile moves.
- [x] Snapshot tests (quayside, 160 × 45): the picker with an empty query; with `rules`; after Enter.
- [x] Mouse tests: a click on a row highlights it; the wheel moves the highlight.
- [x] Real-repo test: for queries taken from real file names, the file with that exact name is in the top 3 results; typing a 10-character query redraws each keystroke under 16 ms at 200 × 60.
- [x] Visual check on the real target repo: type a partial name; the list ranks it, the map follows, Enter keeps it, and Esc returns.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- Files only, as `find_files`. No directories, no grep, no resume.
- No prompt cursor movement (Left and Right do nothing).

## Context

- `~/.config/nvim/init.lua`: Telescope with `telescope-fzf-native`, default layout (`<leader>sf` is `find_files`).
- `crates/tui/src/search.rs`, `crates/tui/src/app.rs` (`on_search_key`, `update_search`), `crates/tui/src/ui.rs` (`draw_tree`, `draw_status`).
- Units `021` and `024` (archived) for the search behavior that this unit replaces.
