---
title: "Hidden files toggle"
status: done
description: "Sort hidden files last in each directory and toggle them with '.', without moving any tile"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1b, core, tui]
priority: high
---

# Hidden files toggle

## Intent

M1 review feedback: dotfiles and dot-directories are noise most of the time, but sometimes they matter.
The user must be able to hide and show them without the map moving, because spatial memory needs stable positions.

## Specification

- A path is hidden when any of its segments starts with `.` (for example `.gitignore`, `.github/workflows/ci.yml`).
- Layout sort (core): files in a directory sort by (`ignored`, `hidden`, name). A hidden file therefore takes a slot at the end of its directory's grid, before allowlisted ignored files.
  Child directories keep their v3 sort and packing.
- Update the mockup to the same sort (`mockups/v3-template.html:82`), so the mockup stays the layout reference.
  Regenerate `crates/core/tests/fixtures/quayside-layout.json` and the private target golden files with `mockups/export-layout.mjs`.
- `.` toggles hidden files. They show at startup.
  When hidden: hidden tiles are not drawn, and the header of a hidden directory is not drawn. The slots and block areas stay empty. No tile moves.
- The breadcrumb's nearest-file search skips hidden files while they are hidden.
- Status bar, left: `. ● hidden` in ink while shown, `. ○ hidden` in faint while hidden.
- Help overlay: add `.  show or hide hidden files`.
- Update `docs/design.md` `Layout` (sort) and `Keys`.

## Validation

- [x] Core test: in a directory with `.env`, `b.clj`, and `a.clj`, the grid order is `a.clj`, `b.clj`, `.env`.
- [x] Core test: an ignored file still sorts after a hidden file.
- [x] Parity test: the Rust layout equals the regenerated mockup golden file for every quayside file.
- [x] One-time movement check (a script, not a committed test): compare the old and new quayside and target golden files. Every moved tile must be in a directory that has a hidden file. Record the counts in the completion report.
  Result: a few quayside tiles moved, all in the root directory. The real-target counts are in the completion report. No header or block moved.
- [x] Toggle test: render 140 × 40 with hidden files shown and hidden. Every cell of every visible non-hidden tile is the same in both buffers.
- [x] Snapshot test: 140 × 40 with hidden files hidden.
- [x] Real-repo tests: parity with the regenerated private golden file. The toggle test also passes on the real repo.
- [x] Visual check on the real target repo: `.` hides and shows dotfiles, and no visible tile moves.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- Hidden is a view toggle, not an exclude. The index still lists hidden files.
- No config default for the toggle.
- Gitignored files stay the separate `ignored` layer (layer 6).

## Context

- M1 review, 2026-09-25. The user chose "sort last, never move" over gaps at the start of grids and over re-packing.
- `AGENTS.md` `Project-Specific Rules`: a toggle must never move a tile.
- Real-target hidden-path counts are in `.private/targets.md`.
