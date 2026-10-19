---
title: "Hover, selection, and zoom to a card"
status: done
description: "Hover and select cards and directories in the GUI, show the path in the top bar, zoom to a card with a double-click, and record the prototype in the design doc"
created: 2026-10-01
updated: 2026-10-01
tags: [milestone-g1, gui, docs]
priority: medium
---

# Hover, selection, and zoom to a card

## Intent

At the far zoom levels a card shows no name, so the user needs another way to know which file a card is.
A selection also gives the zoom a target: the user picks a card and zooms to it.
This unit finishes the prototype for the Milestone G1 review, and records what it built in `docs/design.md`.

## Specification

- Hover: the hovered card's fill is `TILE_HI`. The top bar shows the hovered path at the right, as the TUI does.
- Click:
  - A click on a card selects the file. The card gets a 2-point ring in `SEL_HI`, and its icon is `WHITE`.
  - A click on a header selects the directory. Its block gets the fill `BLOCK_BG`, and its header text is `WHITE`.
  - A press that moves more than 4 points before release is a pan, not a click.
  - Esc clears the selection. A click on empty map does not change it.
- The top bar shows the breadcrumb of the selection at the left, with the file name bright, as the TUI does.
- Zoom to:
  - A double-click on a card eases the camera over 240 ms, to center the card at a width of 420 points.
  - A double-click on a header fits its block in the map region with a 24-point margin.
  - `f` does the same for the selection. `0` still fits the whole map.
- `--select PATH` selects that file or directory at startup and zooms to it. Capture mode uses it.
- The selection does not pulse. The TUI's pulse stays in the TUI.
- `docs/design.md`:
  - `Purpose`: the GUI prototype runs beside the TUI, on the same core.
  - `Vocabulary`: `card` (one file in the GUI's map) and `detail level`.
  - A section `GUI (prototype)`: the window, `CARDS`, the camera and its inputs, the detail levels, the card geometry, the shape scale, the toggles, and the keys,
    with the values from the Results of units `035`–`038`.
  - `Architecture`: `minimap-gui`, the egui renderer and the second binary.

## Validation

- [x] Hit tests at three zooms on quayside: a point maps to the right card, the right header, or nothing. Gaps between cards hit nothing.
- [x] Input tests on the app state, with no window: a click selects; a press that moves 5 points pans and does not select;
  a double-click sets the camera target that centers the card at width 420; Esc clears; `f` with no selection does nothing.
- [x] The breadcrumb text for a file and for a directory equals the TUI's for the same selection.
- [x] Selection and hover never change a card's world rect.
- [x] GUI visual check on the real target with `--select` and a file path from `.private/targets.md`: the ring, the breadcrumb, and the zoomed card show.
  Keep every PNG under `.private/`, and keep the path out of tracked files.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass.

## Result (2026-10-01)

- `minimap_core::layout::Breadcrumb` (`Tree::file_crumb`, `dir_crumb`, `root_crumb`, `text`, `bright`) and `Tree::dir_label` hold the breadcrumb and the hover text.
  The TUI's top bar uses them, adds its leading space, and cuts at the left as before. No TUI snapshot changed.
  A TUI test checks that its top bar starts with the core breadcrumb for a file and for a directory; a GUI test checks the GUI's line against the same breadcrumb.
- The GUI reads the pointer events itself: a primary press that moves more than 4 points before release pans, with the content catching up from the press point.
  A shorter press is a click. Two clicks on one entry within 0.4 s are a double-click, as in the TUI.
- Hit tests use the drawn items, topmost first: an enlarged district name over cards hits its directory, a card and anything on it hit its file,
  and a header name or its rule's row hits its directory. Gaps and the selected block's fill hit nothing.
- The hover comes from the frame's own items, and only a new hover redraws, so the scene is built once in most frames. The capture shows no hover, because it has no pointer.
- `--select` takes a file or a directory (with or without `/`). The first frame shows it with the zoom-to camera. Capture mode adds a last step, `selection`.
- Visual check on the real repo, 2026-10-01 13:37: with a file selected, the ring, the white icon, the bright file name in the breadcrumb,
  and the card at 420 points showed. With a directory selected, its block fill, its white name, and the fitted block showed.
  Idle CPU with the loader threads: 0.0 %. PNGs: `.private/captures/038/`.
- `docs/design.md` records the prototype: `Purpose`, `Vocabulary` (`card`, `detail level`), the `CARDS` column of `Layout`, a section `GUI (prototype)`, and `Architecture`.
- For the G1 review: after a double-click on a large directory, its fitted block can be at a zoom where its name is below 9 points, so only the breadcrumb names it.

## Scope

- No file tree, viewer, selection panel, search, or trail in the GUI. The Milestone G1 review decides what comes next.
- No arrow-key selection between cards.
- No link between the GUI's selection and a running TUI.

## Context

- Breadcrumb and hover text: the TUI top bar in `crates/tui/src/ui.rs`. Selection colors: `docs/design.md` `Map`, `Selection`.
- Move any breadcrumb logic that both front ends need into `minimap-core`, with no change to the TUI's output.
- `AGENTS.md` `Project-Specific Rules`: selection must never move a tile.
