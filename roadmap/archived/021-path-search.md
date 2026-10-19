---
title: "Path search"
status: done
description: "Search file paths with /, select the first match, cycle with n and N, and mark the matches on the map"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2, tui]
priority: high
---

# Path search

## Intent

The user asked for a way to find files by name.
Search finds a file fast, and the map then shows where it lives, which builds the spatial memory the tool is for.

## Specification

Reference: `mockups/v3-template.html:655-660` (`updateSearch`) and `:666-673` (keys).

- `/` opens the search line in the status bar: `/query▏` on `#28402f`, then `N matches · ⏎ keep · esc cancel` in dim.
- Typing edits the query; Backspace deletes. With 2 or more characters, the matches are the visible files whose repo-relative path contains the query, case-insensitive.
  Files whose name contains the query come first; otherwise keep index order.
- Each query change selects the first match (the tree opens to it and the map scrolls to it). No match: the selection stays.
- Match tiles are white on the map (the highest layer color, `docs/design.md` `Map`).
- Enter closes the search line and keeps the matches marked. `n` and `N` then select the next and previous match, wrapping.
- Esc, in the search line or after Enter, clears the search and its marks.
- Hidden files (while hidden) and ignored files are never matches. The hidden toggle recomputes the matches.
- The help overlay lists `/`, `n`, `N`.

## Validation

- [x] Unit tests: matching is case-insensitive; a 1-character query has no matches; name matches sort before path-only matches; hidden files match only while shown.
- [x] Key tests: `/` then typing selects the first match; Enter keeps the marks; `n`/`N` wrap; Esc clears.
- [x] Snapshot tests: 160 × 45 with the search line open and matches marked; after Enter.
- [x] Real-repo test: the match count for a query equals a direct filter over the index.
- [x] Visual check on the real target repo: search for a file name; the map jumps to it; `n` cycles.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- Paths only. No content search, no fuzzy matching.
- The tree shows the selection only; it does not filter its rows.

## Context

- `docs/design.md` `Keys`, `Map`.
- The roadmap Quick Idea "Path search with `/`, `n`, `N`" became this unit.
