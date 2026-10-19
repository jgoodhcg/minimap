---
title: "Character map"
status: done
description: "Make the layout configurable, add the one-character CHARS layout, and draw the map with one icon per file and text only for directories"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2, core, layout, tui]
priority: high
---

# Character map

## Intent

The v5 map gives each file one character and keeps names only for directories.
More of the codebase fits on screen, directory names give the landmarks, and the file icons give texture.
The selection pulses like a slow cursor instead of moving or ringing, so tree navigation stays calm.

## Specification

Core (`minimap-core`, module `layout`):

- `LayoutConfig` with the fields in `docs/design.md` `Layout`: `tw`, `th`, `ind`, `gx`, `gy`, `vg`, `head`, `px`, `py`, the file-column rule, the label width cap, and the root `k`.
  Provide `LayoutConfig::TILES` (the v3 constants, root `k` 3.2) and `LayoutConfig::CHARS` (root `k` 1.0).
- `Layout::compute(&Tree, &LayoutConfig)`. With `TILES` the result must stay identical to today's.
  With `CHARS`: block width is at least `min(name chars + 1, 28)`, and file columns follow the `CHARS` rule in `docs/design.md`.
- The drawn tile size is `(tw - 1) × max(th - 1, 1)` cells: 21 × 2 for `TILES`, 1 × 1 for `CHARS`. The no-overlap and containment tests use it.
- `mockups/export-layout.mjs`: add `--config chars`. It evaluates `makeLayout(CHARS, 1.0)` from `mockups/v5-template.html` and writes `<name>-chars-layout.json` in the same shape.
  Write `crates/core/tests/fixtures/quayside-chars-layout.json`, and the private target golden `.private/fixtures/target-chars-layout.json`.

TUI (reference: `mockups/v5-template.html` `drawMap`):

- The map uses `CHARS`. Remove the 21 × 2 tile drawing.
- Headers and tiles per `docs/design.md` `Map`: directory text only; each file is its type icon (`■` with `--no-icons`) in the name color; hover background `#1e2a36`.
- Selection per `docs/design.md` `Map` `Selection`: the selected file's background pulses between 30 % and 70 % of `#5f86ad` over the pane, with a 2 s period, and the icon is white and bold.
  A selected directory's header text pulses the same way, and its block gets `#16222e`.
  `m` toggles the pulse; off holds 50 %. The status bar shows `m ● pulse` or `m ○ pulse`.
  The pulse phase comes from an injectable clock, so tests are deterministic.
- Redraw: while a selection exists and the pulse is on, redraw about every 80 ms (`event::poll` with a timeout). Otherwise wait for input.
- Camera clamp per `docs/design.md` `Map` `Camera`, with an overview width of 0 until unit `020`.
- Keep unit `018`'s selection scrolling, clicks, and drags working on the new layout.

## Validation

- [x] Parity: `TILES` still equals `quayside-layout.json`; `CHARS` equals the new `quayside-chars-layout.json` for every quayside file and header.
- [x] No-overlap, containment, and determinism tests for both configs.
- [x] Unit test: a block is at least as wide as `min(name chars + 1, 28)`, and a two-file directory with a long name spreads its files on one row.
- [x] Pulse tests: with the clock at two phases the selected cell's background differs and stays between the 30 % and 70 % mixes; with `m` off it is the 50 % mix at every phase.
- [x] Snapshot tests: 160 × 45 initial view; after `d` `d` `s`; with a directory selected; with `--no-icons`.
- [x] Real-repo tests: `CHARS` parity with `.private/fixtures/target-chars-layout.json`, no overlap, containment, determinism, and the frame time at 200 × 60 (under 16 ms).
- [x] Visual check on the real target repo: directory names and icons show; the selection pulses (the ANSI background of its cell differs between two reads about 1 s apart); `m` stops it; WASD pans.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No overview (unit `020`), no search (unit `021`).
- No packing changes beyond the `CHARS` constants and the label width rule. Tighter packing is a later item.

## Context

- `docs/design.md` `Layout`, `Map`, `Stability principle`.
- Mockups: `mockups/v5-template.html` (`CHARS`, `makeLayout`, `drawMap`, `pulse`, `selBg`); `mockups/v3-template.html` for the `TILES` parity.
- Unit `003` parity tests: `crates/core/tests/layout.rs`, `crates/core/tests/common/mod.rs`.
