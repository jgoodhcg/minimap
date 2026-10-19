---
title: "File-type colors only"
status: done
description: "Remove district colors, give each file type one low-chroma color on its icon, and move state, starting with search matches, to the cell background"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2b, core, tui]
priority: high
---

# File-type colors only

## Intent

M2 review feedback.
Color should mean one thing on the map: the file type, carried by the icon.
District colors compete with that, so they go.
Later layers (git status, agent activity) may use color too. They must stay distinguishable from file types.
The decision (the user's request for a best guess, `docs/design.md` `Color`): the glyph foreground shows the file type in low chroma, and the cell background shows state in high chroma.

## Specification

Per `docs/design.md` `Color`, `Map`, `File tree`, `Selection panel`:

- Remove the district palette and every use of it:
  - Map headers: depth 1 is the name and `/` in ink, bold, then a `━` rule in `#26313c`. Depth 2 and deeper do not change.
  - File tree: top-level directory rows are in ink, bold.
  - Selection panel: a directory's path is in ink, bold. The file line is `N lines` only; the district name goes, because the directory line already shows it.
  - Remove `minimap-core::district`, `Tree::district_of`, `theme::DISTRICTS`, `theme::ROOT_FILES`, and `theme::district` when nothing else uses them.
- Type colors: each entry in `crates/tui/src/icons.rs` has HSL saturation at most 40 % and lightness 58–87 %, so it reads on the pane. Grays stay gray.
  A plain saturation cap made `.cljs`, `.ts`, `.css`, and Docker one blue, and `.sql`, `.py`, `.js` one tan.
  So the common types (`.cljs`, `.clj`, `.edn`, `.sql`, `.ts`, `.js`, `.py`) get hand-spaced hues. Keep the values in one table, so a visual review can tune them.
- Only icons carry the type color:
  - Map: each file's icon (or `■` with `--no-icons`) is in its type color.
  - File tree: the icon is in its type color; the file name is in ink2. The selected row keeps white text.
  - Selection panel: the file's icon in its type color, then its name in ink, bold.
  - Binary files keep `#66717c` on the icon.
- State moves to the cell background. In this unit that is the search match: the background `#3fd67a` at 45 % over the pane, and the glyph keeps its type color.
  The selection pulse and hover already use the background, and they win over a search match on the same cell.
  The CC, mine, and heat layers do not exist yet; units `008` and `010`–`013` follow `docs/design.md` `Color` when they are revised.

## Validation

- [x] Unit test: every type color has HSL saturation at most 0.40, and no two file types share a color unless they share an icon.
- [x] Unit test: no drawn cell in a 160 × 45 quayside frame uses a former district color, and every depth-1 header is ink and bold.
- [x] Unit test: tree file names are ink2 and tree icons are their type color.
- [x] Unit test: a search match cell has the search background and its type-colored glyph; the selected match keeps the pulse.
- [x] Snapshot tests: refresh the quayside snapshots (text does not change except the panel's file line).
- [x] Real-repo test: the frame budget test still passes.
- [x] Visual check on the real target repo: the common file types (`.cljs`, `.clj`, `.sql`, `.ts`, `.js`, `.py`, `.md`) tell apart by icon color; no header or tree row has a district color. Capture an ANSI read under `.private/captures/` for the user to review.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No git or agent coloring. This unit sets the channels and moves search matches only.
- No new icons.

## Context

- `docs/design.md` `Color`, `Map`, `File tree`, `Selection panel`, `Vocabulary`.
- `crates/tui/src/icons.rs`, `crates/tui/src/theme.rs`, `crates/tui/src/ui.rs`, `crates/core/src/district.rs`.
- The M2 review screenshot showed white `.cljs` icons: those were kept search matches (white), not a type color. Unit `024` makes a kept search visible.
- Unit `021` (archived) specified white match glyphs; this unit replaces that with the background.
