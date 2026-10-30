---
title: "Syntax colors in the GUI reader"
status: done
description: "Move the TUI's syntax highlighter into the core, and color the GUI reader's lines with it on a background thread"
created: 2026-10-09
updated: 2026-10-09
tags: [milestone-g7, core, gui]
priority: medium
---

# Syntax colors in the GUI reader

## Intent

Code in one color is hard to read in a review. The TUI colors its viewer with `syntect` and tokyonight-night (unit `029`),
but its highlighter returns `ratatui` colors, so the GUI cannot use it. The core owns what both front ends share.

## Specification

- `minimap_core::highlight` (new): `Highlighter::new(theme: Option<&Path>) -> (Highlighter, Option<String>)`,
  `highlight(path, text) -> Highlighted { syntax, lines: Vec<Vec<Span>>, outline }`, `outline(path, text)`, and `foreground() -> Option<u32>`.
  `Span { text, rgb: u32, bold, italic }`. One parse gives the colors and the grammar outline, as before.
- The built-in theme moves from `crates/tui/assets/` to `crates/core/assets/`, with its license and README.
- `crates/tui/src/syntax.rs` keeps its `Worker`, `Job`, `Done`, `Colored`, and `Span`, and builds them from the core's highlighter.
  The TUI's behavior and snapshots do not change.
- The GUI reader asks a highlight thread for its file's colors when it opens a note or the file changes, and shows plain `INK2` text until they arrive.
  The arrival repaints the window. The thread loads the syntaxes and the theme once. It uses the config's `[viewer] theme` when set, else the built-in theme.
- Bold spans use the bold face. Italic draws upright: the GUI has no italic face.
- A file over 20,000 lines or 2 MiB stays plain, as in the TUI.

## Validation

- [x] Core tests: `highlight` on a Clojure and a Rust text gives one span list per line, non-plain colors, and the same outline as `outline`.
- [x] The TUI's tests pass unchanged, with its viewer and render snapshots.
- [x] GUI tests: the reader shows plain text, then colored spans after the thread answers (wait in the test); a too-long text stays plain.
- [x] Real-repo test: the reader colors the real repo's largest Clojure and ClojureScript files; the highlight time is logged under `.private/`.
- [x] All `AGENTS.md` validation levels 1–4b pass. GUI visual check: deferred to the milestone exit, with the user's consent.

## Result (2026-10-09)

- Core: `minimap_core::highlight` (`Highlighter`, `Highlighted`, `Span` with `rgb`), with the parse loop moved from the TUI. The theme and its license moved to `crates/core/assets/`.
- TUI: `syntax::Highlighter` wraps the core's highlighter and converts the spans to `ratatui` colors. Its 191 tests and 20 render snapshots pass unchanged.
- GUI: `reader::Colorer` (a thread with `request`, `poll`, `wait`), `reader::Done`, `colorable`, `expand_from`; `Facts::colors`; `Reading::colors`;
  `GuiApp::colorer`, `poll_colors`, `request_colors`. `main.rs` starts the colorer with the config's `[viewer] theme`.
- Tests: `highlight_colors_each_line_and_outlines_in_one_parse` (core), `the_colorer_colors_the_reader_after_plain_text` and `a_text_too_long_for_colors_stays_plain` (GUI).
  Real-repo: `reader_colors_on_the_target` (release: the largest Clojure and ClojureScript files color within 0.1 s; the times are in
  `.private/reader-colors-055.md`), and the reader frame probe now waits for the colors: p95 under 1 ms.
- Validation: levels 1–4b pass: 471 tests, the TUI render snapshots, and the release frame probes. The real-repo suite (`--no-fail-fast`): 42 passed, 1 failed,
  the known fuzzy timing flake, which passed alone. GUI visual check: not run; it waits for the milestone exit and the user's consent.

## Scope

- No theme switch in the window. No diff colors in the reader.

## Context

- `crates/tui/src/syntax.rs`, `crates/core/src/syntax.rs` (`syntax_for`, `OutlineScan`), `crates/core/src/config.rs` (`viewer_theme`).
