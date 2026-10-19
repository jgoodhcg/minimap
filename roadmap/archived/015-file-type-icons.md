---
title: "File-type icons and colors"
status: done
description: "Give each tile a Nerd Font file-type icon in a vivid type color, and color names in the same hue"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1b, tui]
priority: high
---

# File-type icons and colors

## Intent

M1 review feedback: file types must be easy to tell apart at a glance (`.md`, `.edn`, `.clj`, and so on).
The v3 pastel name colors are too close to each other to read as types.
An icon in a vivid type color gives a second, stronger cue.

## Specification

- Module `crates/tui/src/icons.rs`: a table from file name to `(glyph, color)`.
  Match special names first, then the lowercase extension:

  | Match | Glyph | Color |
  |---|---|---|
  | `clj` `cljc` `bb` | U+E768 (nf-dev-clojure) | `#8dc149` |
  | `cljs` | U+E76A (nf-dev-clojure_alt) | `#519aba` |
  | `edn` | U+E76A | `#a074c4` |
  | `md` `markdown` | U+E73E (nf-dev-markdown) | `#dddddd` |
  | `json` | U+E60B (nf-seti-json) | `#cbcb41` |
  | `yml` `yaml` `toml` `tf` `tfvars` `ini` `conf`, and `Makefile` `Procfile` | U+E615 (nf-seti-config) | `#8a9bb0` |
  | `ts` `tsx` | U+E628 (nf-seti-typescript) | `#3178c6` |
  | `js` `jsx` `mjs` `cjs` | U+E74E (nf-dev-javascript) | `#f1e05a` |
  | `py` | U+E73C (nf-dev-python) | `#ffbc03` |
  | `sql` | U+E706 (nf-dev-database) | `#e38c00` |
  | `sh` `bash` `zsh` | U+E795 (nf-dev-terminal) | `#89e051` |
  | `html` `xml` | U+E736 (nf-dev-html5) | `#e34c26` |
  | `css` `scss` `sass` | U+E749 (nf-dev-css3) | `#42a5f5` |
  | `rs` | U+E7A8 (nf-dev-rust) | `#dea584` |
  | `Dockerfile`, `*.dockerfile` | U+E7B0 (nf-dev-docker) | `#458ee6` |
  | `.gitignore` `.gitattributes` `.gitmodules` | U+E702 (nf-dev-git) | `#f14c28` |
  | `*.lock`, `package-lock.json` | U+F023 (nf-fa-lock) | `#bbbbbb` |
  | `png` `jpg` `jpeg` `gif` `svg` `ico` `webp` | U+F1C5 (nf-fa-file_image_o) | `#d49c6a` |
  | `zip` `gz` `tar` `jar` | U+F1C6 (nf-fa-file_archive_o) | `#eca517` |
  | `txt` `csv`, and `LICENSE` | U+F0F6 (nf-fa-file_text_o) | `#98a3ad` |
  | anything else | U+F016 (nf-fa-file_o) | `#8a95a0` |

- Tile row 1: marker cell, icon, one space, then the name, middle-truncated to 16 cells.
  The `●` mine cell at the right edge stays reserved.
- Name color: the icon color mixed 60 % with ink `#d4dce4`, so the name has the same hue but reads softer.
  A binary file keeps the dim binary name color `#66717c`. Its icon still comes from the table.
- `--no-icons` (CLI flag) turns icons off. The name then starts after the marker cell with 18 cells of room, as in v3.
- Update `docs/design.md` `Map` for the new tile row and the name colors.

## Validation

- [x] Unit tests: the table maps `a.clj`, `a.cljs`, `a.edn`, `README.md`, `Dockerfile`, `.gitignore`, `package-lock.json`, `yarn.lock`, and `x.unknown` to the expected glyph and color.
- [x] Snapshot tests: update the `render` snapshots (icons on), and add one 140 × 40 snapshot with `--no-icons`.
- [x] The color test asserts an icon cell has the icon color and the name cell has the mixed name color.
- [x] Visual check on the real target repo: icons render as glyphs (not boxes) in WezTerm with FiraCode Nerd Font, and `.clj`, `.cljs`, `.edn`, `.md` read as different types.
  (The check read the screen text: each type's cells hold the table's code point and color. That the glyphs draw was inferred from the font `cmap` check, not seen.)
- [x] All `AGENTS.md` validation levels 1–4 pass, including the real-repo tests.

## Scope

- No per-language outline or content inspection. The type comes from the file name only.
- No icon for directories on the map. Headers stay text.
- No config file option for icons. The flag is enough for now.

## Context

- M1 review, 2026-09-25.
- The user's terminal: WezTerm with `FiraCode Nerd Font`. All glyphs above exist in `FiraCodeNerdFont-Regular.ttf` (checked with a `cmap` read on 2026-09-25).
- v3 type colors: `mockups/v3-template.html:179-184`.
