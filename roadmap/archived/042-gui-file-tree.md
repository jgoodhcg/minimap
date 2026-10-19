---
title: "GUI file tree"
status: done
description: "Move the TUI's file tree model into minimap-core, and show it in the window as a left column that shares the selection with the map"
created: 2026-10-02
updated: 2026-10-02
tags: [milestone-g3, gui, core]
priority: high
---

# GUI file tree

## Intent

The G1 review (2026-10-01) listed what comes into the window: the file tree, the viewer, the selection panel, and the trail.
On 2026-10-02 the user picked the file tree next ("Maybe the file tree").
The tree gives the GUI a second, ordered view of the same files: the user finds a file by name and sees where it lives on the map.
This unit builds the panel, its mouse input, and the shared selection. Unit `043` adds the tree keys.

## Specification

Core:

- Move the row model of `crates/tui/src/filetree.rs` (`FileTree`, `Entry`, `Row`) to `minimap_core::filetree`, as unit `038` moved the breadcrumb.
  The TUI's `filetree` module re-exports it and keeps `WHEEL_ROWS`, so the TUI's code and output do not change. `Entry` gains `Hash`.
- The GUI's `scene::Entry` becomes the core `Entry`.

Screen (`minimap-gui`):

- The file tree is a left column between the top bar and the status bar, on `SIDE`. A 1-point edge in `RULE` runs along its right side, in `DIM` while it is dragged.
- Width: 280 points by default, from 160 to 640. In a narrow window it narrows, down to 160, so the map keeps 240 points.
- `t` shows or hides it. It shows at startup, as the TUI's tree does. `[` and `]` narrow and widen it by 40 points.
  Dragging its right edge (a 6-point zone over the edge, with a resize cursor) sets the width.
- Rows are 20 points high, with text of 13 points (the bars' size). Tree text has a screen size: the zoom does not change it.
- First row: `FILES` in `LABEL`, bold, and the file count right-aligned in `FAINT`. Last row: the key hint in `FAINT`.
- Rows come from the compacted tree, as in the TUI (`docs/design.md` `File tree`): under each open directory, its child directories, then its files in grid order.
  - Directory row: an indent of 2 characters per depth, `▸` or `▾` in `DIM`, the closed or open folder icon, and `name/`.
    Top-level rows are `INK` and bold; deeper rows are `INK2`. The file count, including subdirectories, is right-aligned in `DIM`.
  - File row: the indent, the type icon in its type color, and the name in `INK2`.
  - Without icons (`--no-icons`, or no Nerd Font), rows have no icon column, as in the TUI.
  - The selected row has the background `CURSOR` and white text. The hovered row has the background `TILE_HI`.
- The wheel over the tree scrolls it: 3 rows per wheel notch, and trackpad points by the row height. The tree scrolls by whole rows. The wheel over the tree never zooms the map.

Selection, one for the tree and the map:

- `GuiApp::selection` stays the one selection. Every change goes through one function: it sets the tree cursor,
  opens the tree down to the entry, and scrolls the tree only as far as needed to show its row.
- A click on a tree row selects it. A click on a directory row also opens or closes it, as in the TUI.
- A tree selection moves the map only as far as needed to show the card, or the block's top, with a 24-point margin, and with no zoom change.
  The move eases over 160 ms. When the target is larger than the region less its margins, its top and left edges show.
- A double-click on a tree row zooms to its entry, as `f` does.
- A map selection opens the tree to it and scrolls its row into view. It does not move the map.
- Hovering a tree row hovers its entry on the map: the card's hover fill, or nothing for a directory, and the path in the top bar.
- Esc clears the selection, and the tree then has no selected row.

Stability:

- The tree, its width, and hiding it never move a card in the world. The layout takes none of them.
- Hiding, showing, or resizing the tree keeps the map content in place on the screen:
  when the map region changes, the camera moves so that each world point keeps its screen point, then the camera limits apply.
  A window resize keeps the content in place too.

Code:

- `crates/gui/src/tree.rs`: the panel's geometry (header row, rows, hint row, edge), its painting, and its hit tests. Input goes through `GuiApp::on_event`, as the map's does.
- `GuiApp::frame` holds what `eframe::App::ui` does, with the CPU time as an argument, so a headless `Context::run_ui` drives a whole frame.

Docs: `docs/design.md` `GUI (prototype)` (the screen, the tree, the selection, and the keys `t`, `[`, `]`), and `Architecture` (the shared tree model).

## Validation

- [x] The TUI tests pass with the moved model, and `cargo test -p minimap-tui --test render` writes no `.snap.new`.
- [x] GUI app-state tests on `quayside`, with no window:
  - The first rows are the root's child directories, then its files; directory rows give their file counts.
  - A map click on a card opens the tree down to it, and its row is on screen. The camera does not move.
  - A tree click on a file row selects it. The card is inside the region with a 24-point margin, and the zoom does not change.
    A card that is already inside the margins does not move the camera.
  - A tree click on a directory row selects it and opens or closes it, and the map shows the block's top.
  - A double-click on a tree row zooms to its entry.
  - `t`, `[`, `]`, and an edge drag change no card's world rect, and a world point keeps its screen point after each. The width stays from 160 to 640.
  - The wheel over the tree scrolls its rows and not the map. The wheel over the map still zooms.
- [x] Headless egui tests (`Context::run_ui` on `GuiApp::frame`, with `RawInput` pointer events): a click on a tree row selects it;
  a drag on the edge resizes the tree, with the resize cursor over the edge; the frame tessellates.
- [x] Real-repo test (read only): the real repo's tree rows at startup and with every directory open, and the time to build the rows. Print them.
- [x] The headless frame probe on the real repo with the tree shown, in a release build: p95 under 8 ms, and the font atlas does not grow on the second sweep.
- [x] GUI visual check: at the review, after the user agreed (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-02)

- Core: `minimap_core::filetree` holds `FileTree`, `Entry` (now with `Hash`), and `Row`, moved unchanged from the TUI. The TUI's `filetree` module re-exports them
  and keeps `WHEEL_ROWS`, so no other TUI file changed. `scene::Entry` is the core `Entry`.
- GUI: `crates/gui/src/tree.rs` (`TreeView`, `entry_at`, `Look`, `paint`) and, in `GuiApp`: `filetree`, `tree_on`, `tree_w`, `tree_view`, `select`, `show`,
  `camera_showing`, `screen_layout`, `set_screen`, `set_tree`, `tree_width`, and `frame`. A press records what it began on: the map, a tree row, or the edge.
  Tree text is laid out after the bars and before the map, so the font atlas has every glyph before the map's text meshes read its size.
- `GuiApp::select` is the one selection: the map click, a tree click, Esc, and `--select` go through it. The GUI starts with no tree cursor, as it starts with no selection.
- The map follow: `camera_showing` moves each axis only as far as the rect needs to be inside the region less 24 points, and a rect larger than that shows its top or left edge.
  The move eases over 160 ms from where the camera is going. A tree click on a block larger than the region puts its top-left corner at the margin.
- The click that completes a double-click on a directory row zooms to it and does not close it again, so a double-click on a closed directory leaves it open.
- Hover works both ways: a hovered row hovers its entry on the map (the card's fill and the top bar's path), and a hovered card highlights its row.
- Stability: `Motion::keep_on_screen` replaces the clamp on a region change, so the content keeps its screen place when the tree hides, shows, or resizes.
  A window resize now keeps the content in place too, where it kept the map's center point at the region's center before.
- `t` ignores key repeats. `[` and `]` start from the width on the screen, so in a narrow window they change what shows.
- Real repo (read only): the rows at startup and with every directory open build and paint in well under 1 ms (release).
- Headless frame probe, now on whole frames (`GuiApp::frame`, with the tree shown and open to the selection), release build, atlas limit 8,192:
  p95 under 3 ms in both passes, for a short file and for the longest file, at 1 and 2 pixels per point.
  The atlas was the same size after both passes in every run.
- Tests: `crates/gui/tests/tree.rs`, 8 tests with no window: 7 app-state tests and 1 headless test of 9 `run_ui` frames
  (a hover, a click on a row, the resize cursor over the edge and during a drag, the drag's width, and the tessellation).
  A headless context needs `fonts::install(&ctx, None)` for the bold family, and its frames must clear their texture deltas.
- GUI visual check, in the captures of unit `041` (2026-10-02 at 17:30): the tree shows at the left at 280 points, and `--select` opens it down to the selection
  (a file at depth 6) with its row highlighted. The map region starts at the tree's edge, and every capture step centers in it.
  The hint row is cut at 280 points (`j/k · l/h · ⏎ zoom · t hide · [ …`). The user tried the build: "Looks good."

## Scope

- No tree keys for moving the selection (`j`, `k`, `h`, `l`, `g`, `G`, Home, End, Enter): unit `043`.
- No search, no viewer, no selection panel, no trail marks in the tree, and no hidden-files toggle in the GUI.
- No change to the TUI's tree or its behavior.
- No egui widgets that keep their own scroll state: the tree scrolls by the core model's rows, as the TUI's does.

## Context

- The TUI's tree: `crates/tui/src/filetree.rs`, `crates/tui/src/ui.rs` `draw_tree`, `crates/tui/src/app.rs` `select`, `set_open`, `tree_row_at`, `on_mouse`.
- The GUI's input and frame: `crates/gui/src/app.rs` (`on_event`, `set_region`, `ui`). The camera: `crates/gui/src/camera.rs` (`Motion::shift`, `Bounds::clamp`).
- The design: `docs/design.md` `File tree` and `Screen regions` (the TUI's tree, its width, and its edge).
