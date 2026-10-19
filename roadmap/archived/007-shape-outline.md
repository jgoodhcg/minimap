---
title: "Shape and outline"
status: done
description: "Show the selected file's braille shape, tree-sitter outline, and namespace, starting with Clojure"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-3, core, tui, tree-sitter]
priority: high
---

# Shape and outline

## Intent

The shape and the outline answer "what is in this file" without opening it.
They carry the editor-outline memory aid that the chat stream lost.

## Specification

In `minimap-core`:

- Compute file details on demand for the selected file, and cache them in memory. Do not compute them at startup.
- Line shape: sample the file into at most 64 rows. Each row keeps the minimum indent and the maximum length (capped at 126) of its non-empty lines (`mockups/fakegen.py` `minimap`). Expand tabs to 2 spaces.
  Every text file gets a shape. Binary and missing files get none.
- Outline: a `Language` trait with `extensions()`, `outline(source)`, and `namespace(source) -> Option<String>`.
  An outline item has a line, a kind, a name, and a private flag. Register Clojure for `.clj`, `.cljs`, `.cljc`, `.bb`.
  - Grammar crate: try `tree-sitter-clojure-orchard` first, then `tree-sitter-clojure`. Use the first one that builds with `tree-sitter` 0.27. Record the choice in the completion report.
  - A top-level list form is an outline item when the name part of its head symbol (after any `alias/`) starts with `def` or `reg-`.
    This covers what the user's nvim outline shows (clojure-lsp document symbols: `def`, `defn`, `defmethod`, `deftest`, `s/def`, re-frame `rf/reg-sub`, …) and other `def…` macros such as Pathom's `pc/defresolver`.
    Forms inside a top-level reader conditional (`#?(…)`, `#?@(…)`) count too. Forms nested in other forms, such as `comment`, do not.
  - `kind` is the head symbol as written. `name` is the next symbol or keyword as written (`get-berth`, `::berth-id`, `:berth/list`), with metadata (`^:private`, `^{…}`) skipped.
  - `defmethod` items show `name dispatch-value`.
  - An item is private when its kind ends in `-` or its name has `:private` metadata.
  - The namespace is the symbol after `ns` in the first `(ns …)` form, with metadata skipped.
  - Files in other languages get no outline and no namespace.
- `--from-json` mode reads `m`, `o`, and `ns` from the JSON when present, matched by path, so the fake repo shows shapes and outlines.
  `m` holds two base64 characters per row: indent / 2 and (length + 1) / 2.

In the selection panel (`mockups/v5-template.html` `drawSelection`, `docs/design.md` `Selection panel`), below the repo stats:

- The namespace line under the directory.
- SHAPE section: braille grid up to 40 columns and 4–12 rows, line numbers at the first, middle, and last rows, and `·` markers at definition rows.
- OUTLINE section: `line  name` rows. Private items are faint.
  - `o` enters outline browse mode. With no outline, or with the panel hidden, it flashes why instead.
  - In browse mode, `j` / `k` and Down / Up move. Esc or `o` leaves. Enter flashes `Opening files arrives in milestone 6`. `q` and Ctrl+C quit. Other keys do nothing.
  - The status bar shows ` OUTLINE ` and the browse keys. A selection change leaves browse mode.
  - The selected outline row shows `◀` at its shape row.
- `Y` copies the namespace. If the file has none, flash `This file has no namespace. Press y to copy its path.`
- The help overlay and `docs/design.md` `Keys` list `o` and `Y`.

## Validation

- [x] Unit tests on fixture files in `crates/core/tests/fixtures/clojure/`: `defn`, `defn-`, `def`, `defmacro`, `defmulti` with `defmethod`, `defprotocol`, `defrecord`, `^:private` metadata, a docstring, `s/def` and `rf/reg-sub` with keyword names, a reader conditional in `.cljc`, a `comment` form, and `(ns ^:no-doc a.b …)`.
- [x] Unit test: line shape rows for a 10-line file and a 1,000-line file.
- [x] Unit test: a file in an unregistered language returns no outline.
- [x] Snapshot test at 150 × 45: panel for a quayside fixture file with `m` and `o`, in normal mode and in outline mode.
- [x] Real-repo test: every Clojure file parses; outline lines rise and stay within the file; the outline has at least as many items as lines matching `^\((\S+/)?(def|reg-)`; the namespace's last segment matches the file name for most files; a 1,000-line file's details compute under 50 ms.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: select a `.cljs` file; the shape, outline, and namespace show.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- Clojure only. Other languages are Quick Ideas.
- No opening files. Enter opens at a line in unit `013`.
- No per-repo outline config, and no reading of `.clj-kondo` config.

## Context

- The selection panel is the right column from unit `018` (`docs/design.md` `Selection panel`). Mockup: `mockups/v5-template.html` `drawSelection` and `fileShape`.
- The user's nvim outline is `hedyhli/outline.nvim` over clojure-lsp document symbols (checked 2026-09-25).
- The real target repo's main file types are in `.private/targets.md`.
