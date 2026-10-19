---
title: "Live index"
status: done
description: "Watch the working tree and re-index when files are added, removed, or changed, including untracked files, so the map shows what the agent writes"
created: 2026-09-29
updated: 2026-09-30
tags: [milestone-4, core, tui, index, git]
priority: high
---

# Live index

## Intent

minimap builds its index once, at startup, from the tracked files (`git ls-files`).
An agent creates, deletes, and edits files while minimap runs, and it seldom stages them.
Today a new file does not show until a restart, and even then only after `git add`. Unit `010`'s trail would have no tile for it.
This unit makes the map follow the working tree. It goes first in Milestone 4, because the trail needs tiles for the files the agent creates.

## Specification

In `minimap-core`:

- `Index::from_repo` lists the tracked files and the untracked files that git does not ignore:
  the same set as `git ls-files --cached --others --exclude-standard`, minus the config excludes.
  Read the untracked files with `gix`'s `dirwalk` (add the feature; confirm the name at build time). Do not run the `git` CLI.
  `IndexFile` gets `untracked: bool` (JSON `"u"`, left out when false).
- A module `watch`: `Watcher::spawn(root, options)` watches the repo root recursively with the `notify` crate (FSEvents on macOS).
  - It collects events for 250 ms after the first one, then re-indexes on its thread and sends the new `Index`.
  - Events under `.git/` count only for `.git/index`, `.git/HEAD`, `.git/packed-refs`, and `.git/refs/`. A change to `HEAD` or the refs sets `head_moved`.
  - An event for a path that is gitignored or excluded by the config does not start a re-index. A build that writes to `target/` or `node_modules/` stays quiet.
  - It sends `Update { index, changed: Vec<String>, head_moved: bool }` only when the file set, a line count, or a file's content changed,
    or `head_moved` is true. `changed` holds the paths whose content changed (by size and modification time since the last index).
  - A watcher that fails to start leaves the map static and flashes why once.

In the TUI:

- The binary starts the watcher after the first index, in repo mode only (not in `--from-json` mode).
- The main loop checks the watcher at least every 100 ms, and redraws only when an update arrived.
- `App::apply_index(update)`:
  - It rebuilds the tree, the layout, the fuzzy candidates, the tree order, and the header table. Unit `027`'s rounding keeps most tiles in place.
  - It keeps, by path, the selection, the open directories, the viewer, the last viewer place of each file, outline browse, and the kept search.
  - A removed selected file moves the selection to its nearest remaining directory. A viewer on a removed file closes and flashes `<path> was deleted.`
  - When the selected file's tile moved, the camera moves by the same amount, so the selected file stays at its place on the screen. Then the camera clamps.
    Without a selection, the camera does not change.
  - When files were added or removed, it flashes `N added · M removed · K tiles moved`, where K counts the kept files whose tile moved.
  - For each changed path, it drops the cached details (shape, outline, namespace) and the generic outline, so they reload when shown.
  - An open viewer on a changed file reloads its text, keeps the cursor on the same line number (clamped to the new length),
    and asks the background thread for new colors and a new diff.
  - With `head_moved`, it restarts the git history walk in the background. Heat and the panel's GIT section update when the walk ends.
    An open viewer asks for a new diff against the new `HEAD`.
- The selection panel shows `untracked` after the line count of an untracked file.
- `docs/design.md`: `Stability principle` says that a re-index keeps the selected file's screen place.
  `Screen regions`, `Selection panel`, and `Architecture` record the watcher, the flash, and the `untracked` note.

## Validation

- [x] Unit tests on a scripted temporary repo (as in `crates/core/tests/history.rs`): an untracked file that git does not ignore is listed with `untracked`;
  a gitignored file and a config-excluded file are not; a staged new file is listed without `untracked`.
- [x] Unit test: on the temporary repo, the index equals `git ls-files --cached --others --exclude-standard` (the test may run the `git` CLI; the code may not).
- [x] Watcher tests on a temporary repo: an added, a removed, and an edited file each give one update within 1 s, with the right `changed` paths;
  a rename gives one update with the old path gone and the new one listed; writes under a gitignored directory give no update in 1 s;
  a commit sets `head_moved`.
- [x] App tests on quayside with an in-memory update: an added file in a directory with room moves no tile outside that directory
  (the test lists which tiles move); the selection keeps its path and its screen place; the flash counts the added file and the moved tiles.
- [x] App tests: a removed selected file selects its directory; a viewer on a removed file closes with the flash;
  a viewer on a changed file reloads and keeps its line; an update with no file-set change moves no tile and shows no flash.
- [x] Real-repo test (read only): the target's index equals `git ls-files --cached --others --exclude-standard` minus the excludes;
  a full re-index takes under 150 ms warm (print the time); a watcher on the target sends no update in 10 s of no changes.
- [x] Visual check (see `AGENTS.md` `Visual check`) on a local clone of the target (`git clone --no-hardlinks` into `.private/clones/`, never the target itself):
  add a file to a directory in the clone, and see its tile and the flash within 1 s; edit the file open in the viewer, and see the new text at the same line;
  remove the file, and see the tile go. Delete the clone afterwards.
- [x] Visual check on the `minimap self` tab: an untracked scratch file in the minimap repo shows on the map, then goes when removed.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-30)

- `gix`'s `dirwalk` feature is named `dirwalk` in gix 0.87. The index lists untracked files and symlinks, and skips nested repositories.
  `minimap_core::index::scan` keeps the size and modification time of each file. A re-index reads only the files whose stamp changed.
- `notify` 8.2 alone was enough. `notify-debouncer-mini` was not added: the watcher thread collects events for 250 ms itself.
- The watcher checks whether `HEAD` points at another commit when `HEAD` or a ref changed, so a fetch that moves only remote refs sets no `head_moved`.
  Lock files under `.git/` do not count.
- A directory event counts as excluded when a glob matches every name in the directory (`public/**`), so `mkdir` for an excluded path starts no re-index.
- The watcher tests give an added file's update in about 270 ms. A `git status` refresh of the git index sends nothing.
- When a viewer's file went away, the viewer closes and the flash reads `<path> was deleted.` in place of the counts.
  A tracked file deleted from disk stays on the map as missing, as `git ls-files --cached` lists it, and a viewer on it closes the same way.
- The kept search keeps its files by path. An open picker ranks again and keeps its highlighted file.
- Real-repo tests (read only): the target's index equals `git ls-files --cached --others --exclude-standard` minus the excludes, with no untracked files.
  A warm re-index and a full index from nothing both stayed under the 150 ms limit.
  A watcher on the real repo sent no update and ran no re-index in 10 s.
- Visual check on a `git clone --no-hardlinks` of the target under `.private/clones/` (deleted afterwards): a new file in a directory with a free slot
  showed with `1 added · 0 removed · N tiles moved` within a second. The selected file kept its screen place, so the map around it shifted a little.
  The viewer on the file showed the new text with the cursor still on line 12 after an edit, and the panel read `45 lines · untracked`.
  Removing the file closed the viewer, flashed `… was deleted.`, and selected the file's directory. Captures: `.private/captures/visual-034-*.txt`.
- Visual check on the minimap repo: an untracked scratch file in `docs/` showed (`154` → `155` files) and went when removed.
  The "minimap self" tab and the target repo's tab were not open, so the checks ran in a new background tab.
- Not changed: the picker moves the camera through the intermediate matches while a query is typed, so a search for a file next to the selection
  can move the view. This is unit `026` behavior.

## Scope

- No rename tracking: a renamed selected file counts as removed, and the selection moves to its directory.
- No polling fallback for file systems without events.
- No git status layer (Quick Ideas). The `untracked` note is text in the panel only.
- No change to the layout algorithm or its constants.

## Context

- Index: `crates/core/src/index.rs` (`from_repo`). A warm re-index of the real repo takes well under 150 ms in a release build.
- Background patterns: `minimap_core::history::Loader`, `App::poll_history`, `App::next_redraw`, and the main loop in `crates/tui/src/main.rs`.
- State keyed by file index in `App`: `filetree` (cursor, `open`), `viewer`, `viewed`, `outline`, `search`, `hover`, `header_of`, `candidates`, `tree_order`.
- New crates, in `minimap-core`: `notify` (and `notify-debouncer-mini` if its debounce fits the 250 ms rule), and the `gix` `dirwalk` feature.
- `AGENTS.md` `Never Run`: never write inside the target repo. Its checks use a local clone under `.private/`.
- Decided in the M3b review (2026-09-29): add a live index to Milestone 4; list untracked files that git does not ignore; keep the selected file's screen place.
