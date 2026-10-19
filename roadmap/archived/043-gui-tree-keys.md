---
title: "GUI tree keys"
status: done
description: "Move the selection through the GUI's file tree with the TUI's tree keys, and let the map follow"
created: 2026-10-02
updated: 2026-10-02
tags: [milestone-g3, gui, core]
priority: medium
---

# GUI tree keys

## Intent

The TUI moves through its tree with `j`, `k`, `l`, `h`, `g`, and `G` (`docs/design.md` `Keys`). The GUI's tree (unit `042`) has only the mouse.
The user asked for the TUI's tree keys where they do not clash with the GUI's keys (`docs/design.md` `GUI (prototype)` key table).

## Specification

Keys, in the GUI:

| Key | Action |
|---|---|
| `j` / `k` | The next or previous tree row. With no selection, the first row. |
| `l` | Open a closed directory; on an open directory, go to its first child row. |
| `h` | Close an open directory; else go to the parent directory. |
| `g` / `G`, Home / End | The first or last tree row. |
| Enter | Zoom to the selection, as `f` does. |

- None of these keys has a GUI binding today. `w` `a` `s` `d`, `f`, `t`, `[`, `]`, `0`, `7`, `8`, `+`, `=`, `-`, and Esc keep their actions.
  The arrows stay unbound: in the TUI they move on the map, not in the tree.
- The keys work while the tree is hidden, as in the TUI: the tree cursor is the selection.
- A held key repeats: each repeat event moves one row.
- Each move goes through unit `042`'s selection function: the tree scrolls to the row, and the map moves only as far as needed, with no zoom change.
- The decisions of `l` and `h` move into `minimap_core::filetree`, and the TUI's `tree_in` and `tree_out` call them, with no change to the TUI's output.
- The tree's hint row and the status bar's hint name the keys.

Docs: `docs/design.md` `GUI (prototype)` key table.

## Validation

- [x] GUI app-state tests on `quayside`, with no window: each key's move; `l` then `h` on a directory; `g`, `G`, Home, and End;
  Enter zooms to a file and to a directory; repeat events move one row each; the keys work while the tree is hidden;
  after each move, the selected card is inside the region with a 24-point margin, and the zoom does not change.
- [x] Core tests for the shared `l` and `h` decisions.
- [x] The TUI tests pass, and `cargo test -p minimap-tui --test render` writes no `.snap.new`.
- [x] GUI visual check: at the review (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-02)

- Core: `filetree::Step` (`Open`, `Close`, `Select`), `FileTree::step_in`, and `FileTree::step_out`. The TUI's `tree_in` and `tree_out` call them through one `tree_step`;
  its tests and render snapshots did not change.
- GUI: `GuiApp::tree_move`, `tree_end`, and `tree_step`; `j`, `k`, `l`, `h`, `g`, `G` (`g` with Shift), Home, and End in `on_event`. Enter shares `f`'s arm.
  `j` and `k` start from the selection's row; with no selection, both select the first row, as in the TUI.
- Hints: the tree's last row reads `j/k · l/h · ⏎ zoom · t hide · [ ] width`, and the status bar names `j k h l tree · t hide`.
- Tests: `crates/core/tests/filetree.rs` (3) and `crates/gui/tests/keys.rs` (6), with no window. On quayside the repeat test moves 20 rows from the root.
- Real-repo tests: 33 of 34 passed in the parallel run; `fuzzy_ranking_of_the_whole_index_takes_under_a_frame` failed under load again (see unit `041`) and passed alone.
- GUI visual check: the capture script sends no keys, so the app-state tests check the moves. The captures of unit `041` show the hint row.
  The user tried the build, keys included, on 2026-10-02: "Looks good."

## Scope

- No keymap config for the GUI: its keys stay fixed, as in units `035`–`040`.
- No page keys, search, or viewer keys.
- No change to the TUI's keys or behavior.

## Context

- The TUI's tree keys: `crates/tui/src/app.rs` `tree_move`, `tree_end`, `tree_in`, `tree_out`.
- The GUI's keys: `crates/gui/src/app.rs` `on_event`.
- Unit `042` comes first.
