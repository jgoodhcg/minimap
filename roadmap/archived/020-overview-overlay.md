---
title: "Overview overlay"
status: done
description: "Show the whole map's shape and the viewport in the map's top-left corner, and move the view by clicking or dragging there"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2, tui]
priority: high
---

# Overview overlay

## Intent

The character map does not fit the screen.
A small overview keeps the whole map's shape in sight and shows where the view is, without competing with the map for attention.

## Specification

Per `docs/design.md` `Overview` (reference: `mockups/v5-template.html` `ovRect`, `drawOverview`):

- The overview sits in the top-left corner of the map region, on `#0b1015` with a 1-cell margin.
  Its size is at most 26 × 9 cells and 30 % of the map region, keeping the map's aspect.
- One braille dot per visible file, all in `#71879b`. No selection, no layers, no text.
- The viewport: fill `#1a2632`, edge `#3a526a`, dots inside `#aeb9c4`.
- A click or a drag in the overview centers the view at that point, then clamps the camera.
- The camera's lower x limit becomes `-(overview width + 2)`, so every tile can pan out from under the overview.
- Selection scrolling never leaves the selection under the overview: when it would, move the camera so the selection is right of the overview.

## Validation

- [x] Unit test: for a known camera, the viewport box covers the expected overview cells.
- [x] Unit test: a click at an overview cell centers the camera on the matching layout point (clamped).
- [x] Unit test: with the camera at its lower limits, no tile is under the overview.
- [x] Unit test: selecting a file that lies under the overview moves the camera so the file is visible.
- [x] Snapshot tests: 160 × 45 initial view with the overview; after panning to the far right and bottom.
- [x] Mouse test: a drag across the overview moves the camera with it.
- [x] Real-repo test: the overview fits its limits and the viewport box stays inside it at the four camera corners.
- [x] Visual check on the real target repo: the overview shows the map's shape; WASD moves the viewport box; a click in the overview jumps.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- The overview shows nothing but the shape and the viewport.
- This unit replaces unit `005` (dropped): the right-column overview with layer colors and the selected cell.

## Context

- `docs/design.md` `Overview`, `Map` `Camera`.
- Braille bit order: `mockups/v3-template.html:216` (`BIT`).
