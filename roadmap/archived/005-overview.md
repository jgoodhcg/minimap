---
title: "Overview"
status: dropped
description: "Draw the whole repo as braille in the right column with a viewport box, and move the map by clicking or dragging in it"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2, tui]
priority: high
---

# Overview

> Dropped 2026-09-25: the v5 design replaced this right-column overview with the map-corner overview in unit `020`.

## Intent

The map shows only part of the repo at a time.
The overview keeps the whole repo in sight, shows where the view is, and gives a fast way to jump.
It is the always-visible anchor for spatial memory.

## Specification

- Right column, above the legend: a label line `<repo>  N files`, then the overview, then a `─` rule, then the legend from unit `004`.
- Overview drawing per `docs/design.md` `Overview` and `mockups/v3-template.html:436-467`:
  - Scale `s = min(W / layout.w, maxH / layout.h)`, with `maxH = clamp(floor(rows * 0.24), 5, 14)`.
  - Each tile maps to a dot rectangle from `(x, y)` to `(x + TW - 2, y + TH - 2)`, scaled to 2 × 4 dots per cell.
  - Draw each cell as the braille character `U+2800 + bits`, colored by the highest-priority tile in it. In this unit only the district color (priority 1) exists. Keep the priority mechanism so later layers slot in.
  - Viewport box: background `#1a2632` inside, `#3a526a` on the edge, `#0b1015` outside.
- Mouse: a click in the overview centers the map at that point. A drag keeps centering while the pointer moves (`:462-465`).
- Legend: a click on a district row centers the map on that district's header (`:476-477`).
- Keyboard: no new keys.

## Validation

- [ ] Snapshot test at 150 × 45 on the quayside fixture: overview and legend region.
- [ ] Unit test: a known tile maps to the expected overview cell at a fixed scale.
- [ ] Unit test: after `d` three times, the viewport box moves right.
- [ ] Unit test: a simulated click at an overview cell sets the camera so that point is at the map center (after clamping).
- [ ] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: the overview shows colored braille and a viewport box that follows panning.
- [ ] Ask the user once to run `printf '⣿⡇\n'` in WezTerm and confirm dots, not boxes. If the font fails, stop and report; do not build a fallback in this unit.
- [ ] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No cursor mark in the overview (unit `006`), no layer colors (units `008`, `010`, `012`).
- No zoom.

## Context

- `docs/design.md` `Overview`, `Screen regions`.
- Milestone 2 exit: after this unit, stop and ask the user to review.
