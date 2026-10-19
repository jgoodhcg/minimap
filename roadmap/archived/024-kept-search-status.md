---
title: "Kept search in the status bar"
status: done
description: "After Enter, show the kept query and match count in the status bar, so the match marks on the map always have a visible cause"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2b, tui]
priority: medium
---

# Kept search in the status bar

## Intent

In the M2 review, `.cljs` icons under one directory were white and the user could not tell why.
They were search matches that Enter had kept. After Enter the status bar showed nothing about the search, so the marks had no visible cause.

## Specification

- While a search is kept (after Enter, before Esc), the status bar starts with the kept search, then the usual toggles:
  ` /query ` in white on `#28402f`, then ` N matches · n/N · esc clear ` in dim.
- With 0 matches, it still shows, so the user sees why nothing is marked.
- The search line while typing does not change (unit `021`).

## Validation

- [x] Key test: after `/rules` and Enter the status bar starts with ` /rules ` and shows the match count; Esc removes it.
- [x] Snapshot test: 160 × 45 after Enter.
- [x] Visual check on the real target repo: search, Enter, and the status bar keeps the query; Esc clears it and the match marks.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No search history, no highlight in the tree.

## Context

- `crates/tui/src/ui.rs` `draw_status`; unit `021` (archived).
