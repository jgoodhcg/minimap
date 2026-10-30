---
title: "GUI live index"
status: done
description: "Follow the working tree in the window: rebuild the map when files change, keep the selection in place on the screen, and reload changed shapes"
created: 2026-10-07
updated: 2026-10-07
tags: [milestone-g5, gui]
priority: high
---

# GUI live index

## Intent

During a real agent session, files change and new files appear. The TUI follows the working tree (unit `034`); the window shows the index of its startup.
For the 2026-10-07 demo, a file that the agent creates must appear in the window, and an edited file's shape must change.

## Specification

- The window starts `minimap_core::watch::Watcher` at startup (not with `--from-json`), with the index options it indexed with, and repaints when an update is ready.
- On an update, the window rebuilds the map (`Map::new`) from the new index. Then:
  - The selection keeps its entry by path. When the selected file or directory is gone, the selection clears.
  - The camera moves as far as the selected card or block moved in the world, so the selection keeps its place on the screen (the TUI's rule).
    Without a selection, the camera keeps its center and zoom, within the new limits.
  - The tree keeps its open directories by path, and its scroll.
  - The details loader forgets the changed files, so their shapes and outlines are computed again. The pick and the outline scroll clear when the selected file changed.
  - When `HEAD` moved, the git history walk starts again; the old history shows until it ends.
- The top bar's right side shows `N added · M removed` for 4 s after an update that adds or removes files.
- The trail (unit `047`) keeps its marks by path, so a new file that the agent wrote shows its edit mark when its card appears.

## Validation

- [x] GUI tests with no window, on a temporary git repo: a new file gets a card and a tree row; a deleted selected file clears the selection;
  a kept selected card keeps its screen place; an edited file's details are requested again; open directories stay open.
- [x] Real-repo check (read only): the watcher on the real repo starts, and an idle wait (3 s) gives no update.
- [x] The headless frame probe: p95 under 8 ms.
- [ ] GUI visual check: at the user's try-out before the demo (not run: the user was in a meeting).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-07)

- Core: `watch::Watcher::spawn_with` runs its wake after each queued update or error, through a forwarding thread; `spawn` calls it with no wake.
- GUI: `GuiApp::watcher`, `root`, `flash`, `poll_watch`, and `apply_index`, with `FLASH_SECS` 4. `poll_history` also takes a reload walk after `HEAD` moved.
  `main.rs` starts the watcher with `request_repaint` as its wake, and records the root for a new history walk.
- `apply_index` rebuilds `Map` and `Bounds`, carries the selection, the hover, and the open directories by path, keeps the tree's height and scroll,
  clears the press, the clicks, and the panel's row hover (their entries are stale), and clears the pick and the outline scroll when the selected file changed.
  The camera shifts by the selected entry's world move, then the limits apply. The loader forgets the changed paths, and the next frame requests the selection first.
- Deviation: the history walk after `HEAD` moved has no wake, so its result shows at the next repaint.
- Tests: `crates/gui/tests/live.rs`, 5 tests with no window on temporary git repos: a new file's card, tree row, and flash; a selected card that moves
  in the world keeps its screen place, and an open directory stays open; a deleted selection clears; an edited file's details and pick; and a trail mark
  recorded before its file existed. `crates/core/tests/watch.rs` checks `spawn_with`.
- Real repo (read only, release build): the watcher started, and an idle 3 s gave no update. `apply_index` took a few milliseconds.
- GUI visual check: not run. The user was in a meeting, and a window takes the focus. The user's try-out before the demo is the check.

## Scope

- No change to the layout rules: a file that crosses a growth step moves cards, as the G2 findings say.
- No viewer and no diff.

## Context

- The TUI's live index: `crates/tui/src/app.rs` `poll_watch`, `apply_index`, and `restart_history`. The watcher: `crates/core/src/watch.rs`.
- Unit `047` comes first.
