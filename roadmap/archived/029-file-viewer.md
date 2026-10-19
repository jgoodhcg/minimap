---
title: "File viewer"
status: done
description: "Enter shows the selected file in place of the map, read-only, with syntax colors, vim motions, and the panel's shape and outline following the cursor"
created: 2026-09-28
updated: 2026-09-28
tags: [milestone-3b, core, tui, viewer]
priority: high
---

# File viewer

## Intent

The user wants to read a file without leaving the map's context.
A read-only viewer inside minimap keeps the file tree and the selection panel in view.
It works without herdr or an editor, so the map stays useful with no adapters.
The shape is to the viewer what the overview is to the map: it shows where the visible lines sit in the whole file.

## Specification

In `minimap-core`:

- A module `content`: the source of file text. `Content::Repo(root)` reads the working tree. `Content::Memory(map)` holds text by path for tests.
  `--from-json` mode has no content.
- `Config` gets a `[viewer]` table with `theme`: an optional path to a `.tmTheme` file (with `~/` expanded).
  The core only stores the path.

In the TUI (`crates/tui/src/viewer.rs` for state and keys, `crates/tui/src/syntax.rs` for highlighting):

- Opening:
  - Enter on a file opens the viewer at the line where this file's viewer last closed in this session, else at line 1.
  - Enter in outline browse mode opens the viewer at the item's line, with that line at the top.
  - A double-click on a map tile (two left clicks on one cell within 400 ms) opens the viewer for that file.
  - A binary file flashes `Binary file: nothing to view.` A missing file flashes `The file is not in the working tree.`
    Without content (`--from-json`), flash `No file contents in --from-json mode.`
- Screen (`docs/design.md` `Screen regions`):
  - The viewer takes the map region. The overview does not show. The file tree and the selection panel stay.
  - Each row: the line number, right-aligned in a gutter of `max(3, digits of the line count) + 1` cells, then a space, then the text.
  - The cursor line has the background `#1a2632` across the row, and its number is ink. Other numbers are faint.
  - Tabs expand to the next multiple of 4 columns. Characters take their display width (`unicode-width`). Lines do not wrap; they cut at the right edge.
  - The status bar shows ` VIEW ` and `line / total`, then the keys: `j/k ^d/^u · / find · ]] def · y copy · e edit · esc map`.
  - The top bar keeps the breadcrumb of the selection.
- Syntax colors:
  - `syntect` with the `two-face` syntax set (bat's syntaxes), matched by file name, then extension; plain text when none matches.
  - The default theme is `tokyonight_night.tmTheme` from `tokyonight.nvim`, vendored in `crates/tui/assets/` with its Apache-2.0 license file.
    `[viewer] theme` in the config replaces it. A theme that fails to load flashes why, and the default is used.
  - Only the theme's foreground colors, bold, and italic are used. The background stays the pane color.
  - Highlighting runs on a background thread. Plain text shows until the colored lines arrive, and the main loop polls like the git history.
    Keep the colored lines of the last 8 files in memory.
  - Files over 20,000 lines or 2 MiB show plain text, and the status bar says `plain`.
- Keys in the viewer (a mode, like outline browse):
  - `j` / `k`, Down / Up: move the cursor one line. A count prefix repeats a motion (`5j`, `12G`).
  - Ctrl+D / Ctrl+U: half a page. Ctrl+F / Ctrl+B, PageDown / PageUp: a page.
  - `gg` / `G`: the first or last line. `{count}G` or `{count}gg`: that line.
  - `zz`, `zt`, `zb`: put the cursor line at the middle, top, or bottom of the view.
  - `]]` / `[[`: the next or previous outline item. `}` / `{`: the next or previous blank line.
  - `h` / `l`, Left / Right: scroll 8 columns. `0`: back to column 0.
  - `/`: find in the file. The status bar shows ` / query▏` and the match count. Literal text, smart case.
    Enter jumps to the first match at or after the cursor. Matched text has the background `#f2d15c` at 45 % over the pane.
    `n` / `N` go to the next or previous match and wrap. Esc in the prompt cancels it.
  - `y`: copy `path:line` for the cursor line.
  - `e`: flashes `Editor handoff arrives in unit 031` until unit `031` adds it.
  - `t`, `p`: the file tree and panel toggles work as in the map.
    `[` and `]` start `[[` and `]]` (and `[c`, `]c` in unit `030`), so while viewing, the tree width changes only by dragging its edge.
  - `?`: help. `q` or Esc: close the viewer (Esc first closes the find prompt). Ctrl+C quits.
  - Other keys do nothing.
- Mouse in the viewer: the wheel scrolls 3 lines and keeps the cursor in view. A click on a line moves the cursor there.
  A click on a file row in the tree shows that file in the viewer. A click on a directory row selects it, toggles it, and closes the viewer.
- Selection panel while viewing (`docs/design.md` `Selection panel`):
  - The shape rows that show the visible lines get the viewport fill `#1a2632`, and `◀` marks the cursor line's row.
  - The outline row of the definition that holds the cursor (the last item at or above the cursor line) has the cursor background. The outline scrolls to keep it in view.
  - The key hint is `e editor · y copy line · ? keys`.
- Closing the viewer shows the map as it was: no tile moves, and the camera does not change.
- The help overlay lists the viewer keys. `docs/design.md` gets a `Viewer` section, and its `Keys` and `Screen regions` sections change to match.

## Validation

- [x] Unit tests of the motions on a 100-line text with a 5-item outline: `j`/`k` with counts, Ctrl+D/U/F/B, `gg`, `G`, `12G`, `zz`/`zt`/`zb`, `]]`/`[[`, `{`/`}`, and the ends of the file.
- [x] Unit tests of find: smart case, wrap with `n` and `N`, no match, and Esc.
- [x] Unit test: tab expansion and a wide character.
- [x] Unit test: `.clj`, `.cljs`, `.cljc`, `.sql`, `.ts`, `.js`, `.py`, `.md`, `.json`, `.yml`, `.toml`, `.sh`, and `.rs` resolve to a syntax other than plain text.
- [x] Unit test: the vendored theme loads, and a Clojure keyword and a string get different foreground colors.
- [x] Unit test: opening and closing the viewer changes no `Layout` tile position and no camera position.
- [x] Snapshot tests at 150 × 45, on quayside with a `Content::Memory` that holds `crates/core/tests/fixtures/clojure/berth.clj` at a quayside path:
  the viewer at line 1; after `]]` twice; with find matches; with the tree hidden.
- [x] Real-repo test: open the target's largest Clojure file. Plain text draws in under 16 ms, and the highlight time is printed.
  For each main file type in `.private/targets.md`, the first file resolves to a syntax other than plain text.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: Enter on a `.cljs` file shows colored text that matches tokyonight-night (read with `--format ansi`).
  `]]`, `/`, and `y` work. The panel's shape and outline follow the cursor. Esc returns to the same map.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-28)

- Crates: `syntect` 5.3 (fancy-regex, plist themes), `two-face` 0.5.2 (bat 0.26.1 syntaxes), `unicode-width` 0.2.
- On the real repo, the largest Clojure file opens and draws in a few milliseconds. The thread colors it in under 0.1 s, and a colored frame draws in about 1 ms.
  `.cljs`, `.clj`, `.sql`, `.ts`, `.js`, and `.py` resolve to their syntaxes.
- Visual check: the colors are tokyonight-night (`#c0caf5`, `#7aa2f7`, `#89ddff`, `#e0af68`, …). `]]`, `/defn`, and `y` (copied `path:61`) worked.
  After Esc, the map matched the screen from before the viewer. Capture: `.private/captures/visual-029-map-after-viewer.txt`.
- Fixed during the unit: the overview, hidden under the viewer, took clicks and moved the camera.
- `[` and `]` start `[[`/`]]` in the viewer, so they do not resize the tree there (the spec changed to match).
- Enter in outline browse mode keeps browsing when the viewer cannot open (for example, `--from-json`).
- The panel's key hint became `⏎ view · o outline · y path · ? keys`, so it fits the 44-column panel.

## Scope

- Read-only: no editing, no saving, no visual selection.
- No diff (unit `030`) and no editor handoff (unit `031`).
- No line wrap, no column cursor, no folding, no sticky context.
- No per-file theme or font settings.

## Context

- New crates, in `minimap-tui`: `syntect` (fancy-regex, no C regex), `two-face`, `unicode-width`.
- The theme file comes from the user's `tokyonight.nvim` (`extras/sublime/tokyonight_night.tmTheme`), Apache-2.0. The user's nvim uses `tokyonight-night`.
- Outline browse and the panel: `crates/tui/src/app.rs` `on_outline_key`, `crates/tui/src/ui.rs` `draw_file_info`, `draw_shape`, `draw_outline`.
- Background work pattern: `minimap_core::history::Loader` and `App::poll_history`.
- Decided in the M3 review (2026-09-28): the viewer takes the map column on Enter; `e` is the editor; the shape marks the view.
