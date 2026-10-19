---
title: "GUI selection panel"
status: done
description: "Show the selected file's or directory's facts, outline, and git history in a right column of the window, as the TUI's selection panel does"
created: 2026-10-02
updated: 2026-10-02
tags: [milestone-g4, gui, core]
priority: high
---

# GUI selection panel

## Intent

The G1 review (2026-10-01) took the outline off the cards: "It belongs in a GUI selection panel, which comes after G2 with the file tree, the viewer, and the trail."
At the G3 exit (2026-10-02) the user picked "the right panel" next.
The panel gives the window what the TUI's selection panel gives the terminal: the selection's facts, its outline, and its git history,
beside the map that shows where the selection lives.
This unit builds the column, the facts, the outline list, and the git group. Unit `045` adds the shape, and unit `046` the outline clicks.

## Specification

Core:

- Move the TUI's text helpers (`fit`, `fit_left`, `fit_mid`, `wrap`, `thousands`, `kilo`) and their tests to `minimap_core::text`.
  The TUI's `text` module re-exports them, so no other TUI file and no TUI output changes.
  The GUI's `text` module re-exports them too, in place of its copies of `fit` and `thousands`.
- `history::Loader::spawn_with(root, paths, now, wake)`: as `spawn`, and `wake` runs once on the walk's thread after the result is sent, so a window can repaint.
  `spawn` calls it with no wake.

Screen (`minimap-gui`):

- The selection panel is a right column between the top bar and the status bar, on `SIDE`.
  A 1-point edge in `RULE` runs along its left side, 2 points in `DIM` while it is dragged.
- Width: 360 points by default, from 240 to 640. A drag on its left edge (a 6-point zone over the edge, with a resize cursor) sets the width.
  No key changes the width; the TUI's panel has no width keys.
- In a narrow window the panel narrows first, down to 240, and then the tree, down to 160, so the map keeps 240 points.
- `p` shows or hides it. A key repeat does nothing. It shows at startup, as the TUI's panel does (decided in the plan, see `Context`).
- Rows are 20 points high, with text of 13 points, as the tree's rows. Panel text has a screen size: the zoom does not change it.
  Text columns are monospace cells, as the tree counts them, with one cell of margin at each side.
- The wheel over the panel never zooms or pans the map. A press on the panel starts no pan, and a click on it selects nothing in this unit.

Content, in the TUI's order and colors (`docs/design.md` `Selection panel`, `crates/tui/src/ui.rs` `draw_side`):

- First row: the repo name (`INK`, bold) and `N files · M dirs · K lines` (`DIM`) right-aligned; the counts go on the next row when both do not fit.
  Then a row with a 1-point rule in `RULE` across the panel.
- No selection: `Select a file in the tree or on the map.` in `FAINT`.
- A file, from the top:
  1. The type icon in its type color, and the name (`INK`, bold). Without icons, the row has no icon column.
  2. The directory as `dir/` (`DIM`; `./` for a file at the root), word-wrapped to 2 rows at most.
     Then the namespace (`INK2`), 2 rows at most, when the file's details have one.
  3. `N lines` (`DIM`), or `binary`. An untracked file adds ` · untracked`, as in the TUI.
     A generated file (unit `041`) adds ` · generated`, so the panel explains the card's cut mark. The TUI does not show it. Then a blank row.
  4. `OUTLINE` (`LABEL`, bold) with the item count right-aligned in `FAINT`, then one row per item: the line right-aligned in 4 cells, 2 spaces,
     2 spaces of indent per depth, and the name, cut with `…`. Private items are `FAINT`, others `DIM`.
     When the items do not fit, the last row reads `  … K more` in `FAINT`. The section shows when the file has items and at least 2 rows are free above the git group.
  5. `GIT`, at the bottom above the hint row: `GIT` (`LABEL`, bold) with `N commits` (`1 commit`) right-aligned in `FAINT`,
     `changed X ago · created Y ago` (`DIM`), then the recent commits (up to 5): the age right-aligned in 5 cells (`INK2`) and, from cell 7,
     the description (`DIM`, `minimap_core::history::describe`), cut with `…`.
     While the walk runs: `GIT` with `loading`, then `Reading the git history…` (`FAINT`). After a failure: `Git history failed: <error>`.
     A file with no commits: `0 commits`, then `No commits yet.`. Without a history source (an index JSON without git fields): no group.
     The group starts one row below the outline at the earliest, as in the TUI.
  6. The hint row, the panel's last row, in `FAINT`: `f ⏎ zoom · p hide`.
- A directory: its path and `/` (`INK`, bold; the root shows the repo name and `/`), `N files · M subdirectories` (`DIM`), `K lines` (`DIM`, `kilo`),
  a blank row, `LARGEST FILES` with `lines` right-aligned, then the text files under it, most lines first (tree order for a tie):
  the icon, the path below the directory (`INK2`), and the line count right-aligned (`DIM`), as many as fit above the hint row.
  The counts include the files that the map draws (ignored files have no card). Hint: `l open · h close · j/k move · p hide`.
- The status bar's hint adds `p panel`.

Details and history:

- The details loader asks for the selected file first, at every zoom, so the panel has its outline and namespace while the map shows blocks.
  The visible cards at the shape level follow it, nearest to the center first, as before.
- The git history walk (`minimap_core::history::Loader::spawn_with`) starts in the background when the window opens, as the TUI starts it.
  When it ends, the window repaints. With `--from-json`, the JSON's history, when it has one.

Stability:

- The panel, its width, and hiding it never move a card in the world. The layout takes none of them.
- Hiding, showing, or resizing the panel keeps the map content in place on the screen: `GuiApp::set_region` already moves the camera
  so that each world point keeps its screen point (`Motion::keep_on_screen`), then the camera limits apply.
- The map follow (`GuiApp::show`, 24-point margin) and the zoom to an entry use the narrower region.

Code:

- `crates/gui/src/panel.rs`: the column's geometry (`PanelView`: the column, the rows, the hint row, and the edge),
  its content as a list of draw items for a set of facts (`panel::build`), and their painting.
  The content is a pure function of its inputs, so tests read it with no window, as `scene` does for the map.
- `GuiApp`: `panel_on`, `panel_w`, `panel_view`, `set_panel`, `panel_width`, `git`, and `poll_history`. Input goes through `on_event`.
  `Screen` gains the panel's rect.
- The panel lays out its text after the tree and before the map, so the font atlas has every glyph before the map's text meshes read its size.

Docs: `docs/design.md` `GUI (prototype)` (the screen, the panel, the key `p`), and `Architecture` (the shared text helpers).

## Validation

- [x] Core tests: the text helpers' tests pass in `minimap-core`; `spawn_with` runs its wake once, after the result can be polled.
- [x] The TUI tests pass, and `cargo test -p minimap-tui --test render` writes no `.snap.new`.
- [x] GUI app-state and panel tests on `quayside`, with no window (details and history from `quayside-details.json`):
  - The panel shows at startup at 360 points, and the map region ends at its edge.
  - A file's content from the top: the name, `dir/`, the namespace, `N lines`, the outline rows in order (with `… K more` when cut),
    and the git group with its commits directly above the hint row.
  - A directory's content: the path, the counts, the lines, and the largest files in order. The root shows the repo name.
  - No selection: the message. A binary file: `binary`. A generated file: `· generated`. The git states: loading, failed, no commits, and no source.
  - `p` and an edge drag change no card's world rect, and a world point keeps its screen point after each. The width stays from 240 to 640.
  - A narrow window narrows the panel first, then the tree.
  - A press and a release on the panel select nothing, and the wheel over it does not move the camera.
  - At the block level, the loader's request starts with the selected file.
  - A tree click with the panel shown: the card is inside the narrower region with the 24-point margin.
- [x] Headless egui test (`Context::run_ui` on `GuiApp::frame`): the panel paints; the resize cursor over its edge and during a drag; the drag's width; the frame tessellates.
- [x] Real-repo test (read only): the panel's rows for the longest hand-written file and for the root directory, the history walk time,
  and the time to build the panel's items. Print them.
- [x] The headless frame probe on the real repo, with the panel shown and the git history loaded, in a release build:
  p95 under 8 ms, and the font atlas does not grow on the second sweep.
- [x] GUI visual check: at the review, after the user agreed (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-02)

- Core: `minimap_core::text` holds `fit`, `fit_left`, `fit_mid`, `wrap`, `thousands`, and `kilo` with their tests, moved unchanged from the TUI.
  The TUI's and the GUI's `text` modules re-export them, so no other TUI file changed. `history::Loader::spawn_with` runs its wake after the send; `spawn` calls it with none.
- GUI: `crates/gui/src/panel.rs` (`PanelView`, `Facts`, `Draw`, `Panel`, `build`, `paint`, and `GitHistory`, the TUI's enum) and, in `GuiApp`:
  `panel_on`, `panel_w`, `panel_view`, `git`, `total_lines`, `set_panel`, `panel_width`, `file_details`, `poll_history`, `panel_loading`, and `panel`.
  `Screen` has the panel's rect. A press records `PressOn::Panel` or `PressOn::PanelEdge`; a drag on the edge sets the width from where the press began.
- `panel::build` is a port of the TUI's `draw_side` with its `Column`: rows of 20 points, cells of the monospace advance, and the git group placed
  directly above the hint row (one row below the outline at the earliest). The root directory shows the repo name.
- `update_details` puts the selected file first in every request. The capture waits for the git history and the selected file's details too (`panel_loading`).
- `main.rs` starts the walk in the window's creator, with `request_repaint` as the wake. `--from-json` takes the JSON's history when it has one.
- Tests: `crates/gui/tests/panel.rs`, 12 tests with no window: the column at startup, a file's content in the TUI's order, the cut outline,
  a directory's content and the root, no selection and binary and generated files, the four git states and the walk's end,
  stability under `p` and an edge drag, the narrow-window order, a press and the wheel on the panel, the request order, the map follow into the narrower region,
  and 6 headless `run_ui` frames (the paint, the resize cursor over the edge and during a drag, the drag's width, `p`, and the tessellation).
  `tests/tree.rs` hides the panel before its narrow-window check, which tests the tree alone.
- Real repo (read only, release build): the history walk ends within a second on its thread, and a panel builds in well under 1 ms.
- Headless frame probe, with the panel shown, the selection's outline, and the git history loaded (release, atlas limit 8,192):
  p95 under 2 ms in both passes, for a short file and for the longest file, at 1 and 2 pixels per point. The atlas did not grow on the second pass.
  The p95 is below G3's, because the map region is 960 points wide in place of 1,320, so fewer cards draw.
- Real-repo tests: all 35 pass with `--no-fail-fast`. In the first run, `fuzzy_ranking_of_the_whole_index_takes_under_a_frame` failed under parallel load
  and stopped the run (see unit `041`); it took about half of its 16 ms alone.
- Side measure for the G2 review: at the fit view (the 1,600-point region of `common::region()`), most narrow districts draw their names as bars, not text.
  Unit `041`'s cap changed the map's size, and a few more names became bars.
- GUI visual check on the real repo, 2026-10-05, release build, at the exit review after the user agreed (no window opened during the stint):
  three captures, with a short file, the longest hand-written file, and the largest lock file as `--select`. The window ran at 2 pixels per point.
  The panel shows at the right at 360 points: the stats, the facts, the shape, the outline, and the git group directly above the hint row.
  At the fit view it shows the selection's facts and outline while the map shows blocks. The lock file reads `N lines · generated`.
  For the review: a directory path longer than the panel breaks inside a name, as the TUI's `wrap` breaks a long word: the last letter of a name went to a second row.
  `frames.txt`: p95 under 5 ms in both passes for each capture.
  The atlas was the same size after both sweeps (8,192 x 2,048, and 8,192 x 4,096 for the lock file). PNGs: `.private/captures/g4/`.

## Scope

- No shape in the panel: unit `045`. No outline clicks, no outline scroll, and no clicks on the largest files: unit `046`.
- No trail, mine, or focus lines: the GUI has no trail yet.
- No key for the panel's width, and no change to the tree's keys.
- No change to the TUI's panel, its keys, or its output.
- No new crate and no new crate feature.

## Context

- Decided in the plan (2026-10-02), on the user's go-ahead "start a new session to do the right panel":
  - The panel shows at startup, as the TUI's does. `p` hides it. The tree shows at startup too, so the default window (1,400 points) gives the map 760 points.
  - A click on an outline item zooms to the card with the item's line near the region's top, and marks the line on the card (unit `046`).
    The GUI has no viewer, and a zoom to the line shows where the definition lives in the file's shape.
- The TUI's panel: `crates/tui/src/ui.rs` `draw_side`, `draw_repo_stats`, `draw_file_info`, `draw_git`, `draw_outline`, `draw_dir_info`, and `Column`.
- The GUI's tree column, the model for this one: `crates/gui/src/tree.rs`, and `GuiApp::screen_layout`, `set_screen`, `set_tree`, `on_event` in `crates/gui/src/app.rs`.
- History: `crates/core/src/history.rs` (`Loader`, `History::from_json`). Details: `crates/core/src/details.rs` (`Loader::request`).
- Units `042` and `043`: `roadmap/archived/`.
