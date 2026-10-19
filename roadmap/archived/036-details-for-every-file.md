---
title: "Details for every file in core"
status: done
description: "Move the grammar outline from the TUI into minimap-core, and add a background loader that computes shapes and outlines for many files in priority order"
created: 2026-10-01
updated: 2026-10-01
tags: [milestone-g1, core, outline, tui]
priority: high
---

# Details for every file in core

## Intent

The GUI shows the shape and the outline of every visible card at once, not only of one selected file.
Two things block that today.
The TUI computes details for one file at a time, on its main thread, when the selection reaches the file.
And the outline for languages without a tree-sitter outline comes from the TUI's highlighter (unit `032`), so only the TUI has it.
Many indexed files in a real repo are not Clojure.
This unit moves the grammar outline into `minimap-core` and adds a background loader that both front ends can use.

## Specification

In `minimap-core`:

- `minimap_core::syntax`: the `two-face` syntax set, loaded once and shared across threads, and the definition-scope rules of `crates/tui/src/syntax.rs`
  (`Scopes`, `kind_of`, the `entity.name.*` exclusions, the heading levels, nesting, and `impl <type>` names).
  - An `OutlineScan` takes each line's scope operations and text, and returns the outline items at the end.
    The TUI's highlighter feeds it during its one parse for colors, so the TUI keeps one parse pass per file.
  - `outline::generic(path, text) -> Vec<OutlineItem>` runs a parse with `OutlineScan` alone.
  - `syntax::too_long(text)` moves with it: files over 20,000 lines or 2 MiB get no grammar outline.
- `FileDetails::compute` does not change. It stays tree-sitter only, so the TUI's selection does not get slower.
- `details::Loader`, a background pool for many files:
  - `Loader::spawn(source, wake)`: `available_parallelism − 1` worker threads, at least 1.
    The source is a repo root or preloaded details (`Details::from_json`). `wake` is a callback that a worker calls after each result.
  - `request(paths)` replaces the queue of paths not yet started, in the given priority order. Paths already done or in progress are skipped.
  - A worker computes `FileDetails::compute`. When `outline::language_for(path)` has no outline, it adds `outline::generic`.
  - `poll()` returns the finished `(path, FileDetails)` pairs. `forget(path)` drops a cached result, for the live index later.
  - Dropping the loader stops the workers after their current file.

In the TUI:

- `crates/tui/src/syntax.rs` uses `minimap_core::syntax` for the syntax set, the scope rules, and `OutlineScan`. The TUI's behavior does not change.

## Validation

- [x] The TUI tests pass unchanged: `crates/tui/tests/outline_generic.rs`, the viewer tests, and every snapshot. No `.snap.new` is written.
- [x] Core tests for `outline::generic`: the cases of `crates/tui/tests/outline_generic.rs` give the same items through core.
- [x] Loader tests on a temporary repo with quayside paths:
  results equal `FileDetails::compute` plus the generic outline; a second `request` reorders the pending paths;
  `wake` runs once per result; dropping the loader joins its threads within 1 s.
- [x] Real-repo test (read only), in a release build: the loader computes every index file of the target.
  Print the total time and the 5 slowest files with their times. Every text file under the size limit gets a shape.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-10-01)

- `minimap_core::syntax` holds the `two-face` syntax set (a `OnceLock`, shared across threads), `syntax_for`, `too_long` (`MAX_LINES`, `MAX_BYTES`),
  the scope rules, nesting, and `OutlineScan` (`line(n, text, ops)`, then `finish()`). `outline::generic(path, text)` parses with `OutlineScan` alone
  and returns nothing for a text over the size limit.
- The TUI's `Highlighter` uses the shared syntax set and feeds `OutlineScan` during its one color parse. It re-exports `too_long` and the limits.
  The TUI no longer depends on `two-face` directly. Its tests and snapshots did not change.
- `details::Loader`: `spawn(details, wake)` (`available_parallelism - 1` workers, at least 1), `with_workers` (0 workers keeps requests queued, for tests),
  `request(paths)`, `poll()`, `get(path)`, `forget(path)`, `pending()`, and `busy()`. Preloaded details (`--from-json`) answer a request at once, with no workers.
  `details::compute_file(path, bytes)` is the workers' function: `FileDetails::compute`, plus the grammar outline for a text file without a tree-sitter outline.
  The loader keeps the results that `poll()` took, so `get()` reads them.
- Crates in the core: `syntect` 5.3 (`parsing`, `plist-load`, `regex-fancy`) and `two-face` 0.5.2 (`syntect-fancy`), at the TUI's versions.
- The loader tests run on a temporary directory with 300 quayside paths and generated texts. `quayside-details.json` has shapes and outlines
  for `components/berth/src/quayside/berth/` only; the preloaded test uses that directory.
- Real-repo test (read only, release build): the loader computed every real-repo file within a few seconds.
  Every text file under the size limit got a shape, and most files got an outline, some of them from grammars.
  The slowest files were long TypeScript files: the TypeScript grammar is the slow part.
  One by one on one thread, all files took many times longer. The file times are in `.private/loader-036.txt`.
- TUI visual check on the real repo (herdr, background tab): a Python file showed its grammar outline in the panel, and its viewer drew 27 syntax colors.
  Captures: `.private/captures/visual-036-tui*`.

## Scope

- No GUI change. Unit `037` uses the loader.
- No change to which files get an outline, or to the outline rules.
- No details cache on disk.

## Context

- Grammar outline today: `crates/tui/src/syntax.rs` (`Highlighter::parse`, `Scopes`, `kind_of`, `too_long`). Unit `032` record: `roadmap/archived/032-generic-outline.md`.
- Details: `crates/core/src/details.rs` (`FileDetails::compute`, `Details`, `Source`). Background pattern: `minimap_core::history::Loader`.
- `minimap-core` must stay free of terminal and renderer dependencies. `syntect` and `two-face` parse text and draw nothing, so they can move into core.
  Use the TUI's features (`parsing`, `plist-load`, `regex-fancy`; `two-face` `syntect-fancy`).
- New crates for core that this unit names: `syntect` and `two-face`, at the TUI's versions.
