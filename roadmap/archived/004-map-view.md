---
title: "Map view"
status: done
description: "Render the district map with file names in the terminal, with panning, bars, and a district legend"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1, tui]
priority: high
---

# Map view

## Intent

This is the Milestone 1 deliverable.
The user runs `minimap` on a real repo and pans a stable map of every file name.
It answers the first question: does the user enjoy looking at their code this way?

## Specification

CLI:

- `minimap [PATH] [--exclude <glob>]... [--config <path>]` indexes the repo (unit `002`), lays it out (unit `003`), and opens the TUI.
- `minimap --from-json <file>` loads an index dump or `mockups/fake.json` instead of a repo.

Screen regions (see `docs/design.md` `Screen regions`):

- Top bar: breadcrumb of the directory of the file nearest the view center (`mockups/v3-template.html:295-300`, `:570-585`).
- Map: a custom Ratatui widget that writes into the `Buffer` and draws only visible headers and tiles.
  - Headers per `docs/design.md` `Map` (root, district, deeper).
  - Tiles: 21 × 2 cells on the tile background `#161e27`. Row 1 is the name, middle-truncated with `…`, colored by file type (`mockups/v3-template.html:179-184`). Row 2 is `N lines` or `binary`, in the dim color `#7b8896`.
  - Use `Color::Rgb` for all colors (`mockups/v3-template.html:171-182`).
- Right column: width per `docs/design.md`. For now it shows the repo name, the file count, and the district legend (swatch, name, file count), as in `mockups/v3-template.html:468-491` without the layers section. Unit `005` adds the overview above it.
- Status bar: `? keys` and `q quit` on the right.

Input:

- `w` `a` `s` `d` pan 11 columns or 2 rows. Shift multiplies by 4.
- Mouse wheel pans. Mouse drag pans. Enable mouse capture.
- Clamp the camera as in `mockups/v3-template.html:290-294`.
- `?` toggles a help overlay that lists only the keys that work now.
- `q` and Ctrl+C quit. Terminal resize redraws.
- Initial camera: the top-left of the layout.

Rendering must stay under 16 ms per frame on the real target repo. Measure it once and record the number.

## Validation

- [x] Snapshot tests (`TestBackend`, `insta`) on `crates/core/tests/fixtures/quayside-files.json`:
  - 140 × 40, initial view.
  - 140 × 40 after `d` three times and `s` twice.
  - 80 × 24: the right column is hidden.
  - 140 × 40 with the help overlay.
- [x] A test asserts a district header cell has the district color from the palette, and a tile name cell has its file-type color.
- [x] A test asserts the camera clamps at the layout edges.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: headers, tile names, and the legend show. `d` pans. `q` exits cleanly. Record the startup time and frame time. Do not commit the captured output.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No overview (unit `005`), no selection or cursor (unit `006`), no ages (unit `008`), no layers.
- No Enter or copy keys.

## Context

- `docs/design.md` `Screen regions`, `Map`, `Keys`.
- Mockup drawing: `mockups/v3-template.html:321-381`, `:569-603`.
- Milestone 1 exit: after this unit, stop and ask the user to try the map on the real repo. Start it for them in a herdr pane.
