---
title: "GUI window and zoomable map"
status: done
description: "Add a minimap-gui crate: an egui window that draws the CARDS layout as file cards with icons and directory names, with smooth pan and zoom from the whole repo to one card"
created: 2026-10-01
updated: 2026-10-01
tags: [milestone-g1, core, gui, layout]
priority: high
---

# GUI window and zoomable map

## Intent

The user finds the TUI map limiting (2026-10-01).
The overview is too small and distorted, and the map shows only a part of the repo at a time.
The user wants a map that zooms smoothly from the whole repo to one file, with one card per file.
This unit builds the window, the card layout, and the camera.
It also tests the main risk of egui: whether text stays sharp and frames stay fast while the zoom changes.
The TUI stays. Both front ends run on `minimap-core`.

## Specification

In `minimap-core`:

- `LayoutConfig::CARDS`, in cells as `CHARS` (`px` 8, `py` 18):
  `tw` 22, `th` 7, `ind` 2, `gx` 3, `gy` 1, `vg` 1, `head` 1, `fc_k` 0.7, `label_w` 28, `root_k` 1.6,
  and the `CHARS` rounding (`Quant::Growth(1.25)`; capacity, and height at depth 1–2).
  A card is `tile_size()`, 21 × 6 cells. The visual check can tune `root_k` and `fc_k`. Record the final values in the Result.
- `minimap_core::filetype`: the icon table of `crates/tui/src/icons.rs` as data (`glyph: char`, `rgb: u32`), with `for_name`.
  `minimap_core::palette`: the hex values of `crates/tui/src/theme.rs` as `u32` constants.
  The TUI converts both to Ratatui colors. Its snapshots do not change.
- `Config` gets a `[gui]` table with `font`: a path to a Nerd Font `.ttf`. A leading `~/` expands.

A new crate `crates/gui` (`minimap-gui`, binary `minimap-gui`), with `eframe` on its `wgpu` renderer:

- Arguments as the TUI: `[path]`, `--config`, `--exclude`, `--from-json`, `--no-icons`. The window title is `minimap — <repo name>`.
  It builds the index, the tree, and the `CARDS` layout once at startup. This unit has no watcher.
- Fonts: the `[gui] font` file, else the first `*NerdFontMono-Regular.ttf` by name in `~/Library/Fonts`, then `/Library/Fonts`.
  Without a font, or with `--no-icons`, an icon is a square in the type color.
- World coordinates: one cell is 8 × 18 points at zoom 1. A card at zoom 1 is 168 × 108 points.
- Screen: a top bar of one text row (the repo name and `N files · M dirs`), the map, and a status bar of one text row (the zoom and a key hint).
  The background is `PANE`. The bars are `BAR`.
- Drawing, by the card's width `w` on the screen, in points (unit `037` adds names, shapes, and outlines):
  - `w` below 20: a filled rect in the type color at 55 % over the pane.
  - `w` 20 or more: the card (fill `TILE`, corner radius `0.04 × w`), with the icon in the type color at its top left.
    The icon size is `clamp(0.5 × card height, 8, 16)` points.
  - Headers: the directory name as the TUI draws it (root and depth 1 bold, `name/`, a compacted chain as one name), then a 1-point rule in `RULE`.
    The text size is 0.8 × the header row's height on the screen. Below 9 points, the name is not drawn.
    Root and depth-1 names are the exception: they stay at 13 points or more, over a `PANE` backing at 80 %.
  - Every text size snaps down to the ladder 9, 10, 11, 12, 13, 14, 16, 18, 20, 24, 28, 32.
    Hypothesis: a fixed ladder keeps egui's glyph atlas small while the zoom changes. The validation tests it.
  - Only cards and headers that intersect the map region are drawn.
- A pure `scene` function: (layout, tree, camera, map region, toggles) → draw items (kind, path, screen rect, detail level).
  The painter reads only the draw items, so tests check the scene without a GPU.
- Camera:
  - At startup and on `0`: fit the whole map in the map region with a 24-point margin.
  - The zoom range is from half the fit zoom to 6 (a card 1,008 points wide).
  - Pinch, and Cmd with scroll, zoom around the pointer. A mouse wheel (line units) zooms around the pointer by 1.15 per notch.
    A trackpad scroll (point units) pans. A drag with the primary button pans.
  - `+` or `=` zooms in by 1.5 around the map region's center. `-` zooms out. `w` `a` `s` `d` pan 10 % of the view (Shift: 40 %).
  - Key zooms and fits ease out over 160 ms. Pinch and wheel zooms are immediate.
  - Clamp the camera so the center of the map region stays inside the map's bounds.
  - Repaint only on input or during an animation. An idle window uses no CPU.
- Capture mode, for visual checks: `--capture DIR [--select PATH]` opens a 1,600 × 1,000 window and runs a fixed script.
  - Steps: the fit view, then card widths 16, 40, 80, 130, 200, and 400, centered on the card of `PATH`, else on the map center.
    It saves each step as `DIR/NN-<step>.png` (egui's `ViewportCommand::Screenshot`, and the `image` crate for PNG).
  - Then it zooms in steps from the fit view to card width 400 over 120 frames, two times.
    It writes the p50, p95, and max frame CPU time (`frame.info().cpu_usage`) and the font atlas size after each pass to `DIR/frames.txt`.
  - Then it exits. Confirm the egui API names at build time.
- `AGENTS.md`:
  - `Stack`: egui and eframe for the GUI, and the second binary `minimap-gui`.
  - `Validation Commands`: a level 4b row: `cargo test -p minimap-gui`, then the GUI visual check, after GUI changes.
  - `Visual check`: a GUI variant. Run `cargo run --release -p minimap-gui -- --config .private/minimap.toml --capture .private/captures/<unit> <target repo path>`.
    Then read the PNGs.

## Validation

- [x] Core tests on quayside: `CARDS` gives no overlapping card or header rects, and two runs give the same layout.
  An insta snapshot records its width, its height, and the first 20 tiles.
- [x] The TUI snapshots do not change after the icon and palette move (`cargo test -p minimap-tui --test render` writes no `.snap.new`).
- [x] GUI tests, with no window:
  - Camera: a zoom around a point keeps that world point under the pointer, within 0.01 point. The fit view contains every card. The clamp holds.
  - Scene on quayside at four zooms: an insta snapshot of the item counts per kind and detail level.
  - Culling: no item lies outside the map region.
  - A zoom change never changes a card's world rect.
- [x] Real-repo test (read only): the `CARDS` layout of the target has one card per index file and no overlaps. Print the map size in points.
- [x] GUI visual check on the real target: the PNGs show the whole repo at the fit view, readable district names at the fit view,
  and sharp icons and headers at every step. Keep every PNG under `.private/`.
- [x] `frames.txt` on the real target, in a release build: the p95 frame CPU time is under 8 ms.
  The font atlas has the same size after the second zoom pass as after the first.
  When text blurs or either check fails, stop and report to the user. This check is the go/no-go for egui.
- [x] Idle check: after 5 s with no input, `ps -o %cpu -p <pid>` reads under 1 %.
- [x] All `AGENTS.md` validation levels 1–3 pass.
- [ ] For the user at the milestone exit, not the agent: the window shows in Cmd+Tab, and pinch and wheel zoom feel smooth.

## Result (2026-10-01)

- Go for egui. On the real target in a release build, the zoom sweeps gave a p95 frame CPU time under 3 ms in both passes, and no frame over 5 ms.
  The font atlas stayed small and almost empty after both passes.
  Text and icons were sharp at every step: egui rasterizes each glyph at its ladder size, so no text is scaled.
  The capture screen had 1 pixel per point. After 6 s and 9 s with no input, `ps -o %cpu` read 0.0 %.
- `CARDS` final values: `root_k` 3.2 (was 1.6), `fc_k` 0.7. With 1.6 the map was portrait, and a card at the fit view was very narrow.
  With 2.5 or more, the root packs every district in one shelf, and a card at the fit view is wider. 3.2 keeps one shelf with room to grow.
  `fc_k` 1.0 gained nothing.
- Crates: `eframe` 0.36.2 (`egui` 0.36.2, `wgpu` 30.0.1, `winit` 0.30.13), `image` 0.25.10 (PNG only), `clap` 4.6.7, `anyhow` 1.0.104,
  and `insta` 1.48.0 as a dev-dependency of the GUI and of the core (for the `CARDS` snapshot). eframe's default features are off, except `default_fonts` and `wgpu`.
- egui 0.36 API names: `eframe::App::ui` (it replaced `update`), `ViewportCommand::Screenshot` and `Event::Screenshot`, `frame.info().cpu_usage`,
  `ctx.fonts(|f| f.font_image_size())`, `Event::Zoom` for a pinch, and `MouseWheelUnit::Line` and `Point`.
  `zoom_with_keyboard` is off, so Cmd with `+` or `-` does not scale the whole window.
- `minimap_core::filetype` (`Icon { glyph, rgb }`, `for_name`, `type_rgb`, `ALL`) and `minimap_core::palette` (`u32` constants and `rgb`) hold the data.
  The TUI's `icons` and `theme` convert them. No TUI snapshot changed.
- The bold face is the sibling file of the font with `-Bold` in place of `-Regular`, when it exists. Root, depth-1, and depth-2 names use it, as in the TUI.
- Deviations: the icon size is `clamp(0.5 x card height, 9, 16)`, not 8 at the low end, so every glyph size is on the ladder.
  A rule starts one cell after the name and ends one cell before the block's right edge, and the root has none, as in the TUI.
  An enlarged root or district name sits at the top of its header row, is drawn after every other item, and has its backing only while it is enlarged.
- Capture mode: eframe runs no UI pass while its window is covered, so the first run behind the terminal never drew.
  The capture window now floats above other windows, a watchdog stops it after 90 s, and a screenshot request without a reply after 30 frames is sent again.
  It still takes the focus: `with_active(false)` does not stop the app's activation. winit's `with_activate_ignoring_other_apps(false)` would,
  but it needs `winit` as a direct dependency, which this unit does not name.
- Captures ran on 2026-10-01 from 11:28 to 11:43: three on quayside (`--from-json mockups/fake.json`) and two on the target, then the idle check.
  PNGs and `frames.txt` of the target: `.private/captures/035/`.
- For the G1 review: below a card width of about 105 points, only root and district names show, because a deeper name needs a header row of 11.25 points.
  The Growth rounding of depth 1–2 heights leaves visible empty bands in the map. A Nerd Font Mono icon is one cell wide, so a 16-point icon is about 10 points wide.

## Scope

- No names on cards, no shapes, no outlines, and no toggles (unit `037`). No selection and no hover (unit `038`).
- No file tree, viewer, selection panel, trail, layers, search, live index, or keymap config in the GUI.
- No macOS app bundle, no signing, and no Tauri. The binary runs from the terminal.
- No change to the TUI's behavior.

## Context

- Decided with the user on 2026-10-01: a native window, not a browser tab, so the map is one thing to Cmd+Tab to.
  egui with eframe, because it links `minimap-core` directly. The TUI stays an option.
- Layout: `crates/core/src/layout.rs` (`LayoutConfig`, `Layout::compute`, `tile_size`).
  `TILES` (22 × 3) is the v3 tile map with names. `CARDS` keeps its width and adds rows for the card body.
- One card size for every file: `docs/design.md` `Stability principle`. Sizing tiles by line count reshuffles the packing. Unit `037` shows size through the shape.
- quayside: `--from-json mockups/fake.json` has shapes and outlines for most of its files. `python3 mockups/build.py` builds it; it is git-ignored.
  Committed tests use `crates/core/tests/fixtures/quayside-files.json`.
- Icons: `crates/tui/src/icons.rs`. Palette: `crates/tui/src/theme.rs`. The user has FiraCode Nerd Font Mono in `~/Library/Fonts`.
- egui mouse units: `Event::MouseWheel { unit, .. }` is `Line` for a wheel and `Point` for a trackpad. Some mice send points; their scroll pans.
- New crates that this unit names: `eframe` (with `wgpu`), `image` (PNG only), `clap`, and `anyhow`, and `insta` as a dev-dependency.
  Record their versions in the Result.
