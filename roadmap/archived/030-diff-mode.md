---
title: "Diff mode"
status: done
description: "Show a file's uncommitted changes against HEAD in the viewer, inline and syntax-colored, with hunk motions and change marks"
created: 2026-09-28
updated: 2026-09-28
tags: [milestone-3b, core, tui, viewer, git]
priority: high
---

# Diff mode

## Intent

A coding agent changes files and usually does not commit.
The user needs to see what changed in a file, in place, with the whole file around it.
Diff mode shows the uncommitted changes against `HEAD` in the viewer from unit `029`.

## Specification

In `minimap-core`, module `diff`:

- `text_at(root, rev, path) -> Result<Option<String>>`: the file's blob at a revision (`HEAD` for the viewer), read with `gix`.
  `None` when the revision has no such file (a new file) or the branch is unborn.
- `FileDiff::compute(old, new)`: a line diff with `similar` (Myers).
  It gives the rows of the new file with the removed old lines between them, each row tagged same, added, or removed, with its old and new line numbers.
  It also gives the hunks (runs of changed rows) and the added and removed counts.
- `Content` (unit `029`) gets `head(path)`: `Repo` reads it with `text_at(root, "HEAD", path)`, and `Memory` holds a second map.

In the viewer:

- When the viewer opens a file, it computes the diff against `HEAD` on the background thread that highlights the file.
  The removed lines are highlighted from the `HEAD` text.
- When the file has uncommitted changes, the status bar shows `+N −M · = diff`.
- File mode (the default) marks the changes in a sign column right of the line numbers, as gitsigns does:
  `▎` in the add color on added lines, and `▁` in the remove color on the line above a removal (`▔` on line 1 for a removal at the top).
- `=` toggles diff mode. With no changes, it flashes `No uncommitted changes in this file.`
- Diff mode shows the working-tree file with the removed `HEAD` lines between its lines, where they were removed:
  - Added rows: `+` in the sign column and the background `#3fb950` at 22 % over the pane.
  - Removed rows: `-` in the sign column, no line number, and the background `#f85149` at 22 % over the pane.
  - The cursor moves over every row. On a removed row, `y` and `e` use the next working-tree line.
  - The status bar shows ` DIFF ` in place of ` VIEW `, then `+N −M`.
- `]c` / `[c`: the next or previous hunk, in both modes.
- Toggling `=` keeps the cursor on the same working-tree line.
- In the selection panel, the shape marks rows with changed lines with `▐` in the add color. It takes the place of `·` on those rows.
- A file whose text equals `HEAD` shows no marks. A file that `HEAD` does not have shows every line as added.
- The help overlay and `docs/design.md` `Viewer`, `Keys`, and `Color` list `=`, `]c`, `[c`, and the diff colors.

## Validation

- [x] Unit tests on a temporary repo with scripted commits: `text_at` returns the committed text; a working-tree edit, an addition, and a removal give the expected rows and counts;
  a staged new file is all added; an unborn branch gives `None`.
- [x] Unit test: in the temporary repo, the added and removed counts match `git diff --numstat` for the same edits.
- [x] Unit tests of the viewer: `]c`/`[c` at the ends, `=` keeps the working-tree line, and `y` on a removed row copies the next line.
- [x] Snapshot tests at 150 × 45, with a `Content::Memory` holding a `HEAD` and a working version of the berth fixture: file mode with signs and the status hint; diff mode; after `]c`.
- [x] Real-repo test (read only): for 40 files changed in recent non-merge commits, the diff of the file between the commit's parent and the commit
  is a valid edit script, and its size lies between `git diff --numstat --minimal` and `git diff --numstat` (changed from "equals `git diff --numstat`"; see Result).
- [x] Visual check (see `AGENTS.md` `Visual check`) on the minimap repo itself, which has uncommitted changes during this milestone: open a changed file, see the signs, press `=`, and step with `]c`.
  On the real target repo: a clean file shows no signs, and `=` flashes.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-28)

- `text_at` takes `HEAD` or a full commit id, so it needs no `gix` `revision` feature. `similar` 3.2 computes the diff.
- Git's default Myers is not always minimal, and `similar`'s Myers is not either, so equal counts were not reachable.
  On one real-repo file, our counts fell between `git diff --minimal` and git's default.
  The real-repo check became: a valid edit script, between git's minimal and default counts. Almost every checked file equals git's default.
- The colors and the diff share the background thread. A file whose text changed since its last view gets a new diff, and its old colors are dropped.
- The viewer's text moved one column right for the sign column, so the unit `029` snapshots changed.
- Visual check on the minimap repo: signs, `+254 −13`, diff backgrounds `#1a3a27` and `#432326`, and `]c` all worked (`.private/captures/visual-030-diff-self.txt`).
  On the clean real repo, a file showed no signs, and `=` flashed.

## Scope

- Uncommitted changes against `HEAD` only (M3 review answer). No commit diffs, no staged-only view, no side-by-side view.
- No git status layer on the map.
- No word-level highlighting inside changed lines.

## Context

- New crate, in `minimap-core`: `similar`.
- `gix` is already a dependency with `index` and `sha1`. Reading a tree entry and a blob needs no new feature; confirm at build time.
- Git reading patterns: `crates/core/src/history.rs`, `crates/core/tests/history.rs` (scripted temporary repos).
- `docs/design.md` `Selection panel` already names `▐` for an edited range.
- Never write to the target repo (`AGENTS.md` `Never Run`), so its visual check uses a clean file.
