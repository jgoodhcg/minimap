---
title: "Outlines for every language"
status: done
description: "Outline every file the viewer's syntaxes parse, from their definition scopes, so Rust, TypeScript, Python, SQL, Markdown, and the rest get an outline"
created: 2026-09-28
updated: 2026-09-28
tags: [milestone-3b, tui, outline, syntax]
priority: high
---

# Outlines for every language

## Intent

Only Clojure files have an outline (unit `007`). On `minimap`'s own Rust files and on TypeScript, SQL, and Python files,
the panel has no outline, and `]]` has nothing to jump to (M3b review, 2026-09-28).
The viewer's syntaxes (unit `029`) already parse about 200 languages, and their grammars tag definitions with `entity.name.*` scopes,
as Sublime's "Go to Symbol" uses them. One scope walk gives every one of those languages an outline, with no new crate.

## Specification

In the TUI (`crates/tui/src/syntax.rs`):

- `Highlighter::outline(path, text)` parses the text with the file's syntax and returns outline items:
  - A line gives at most one item: its first token with an `entity.name.*` scope, except `entity.name.tag`, `entity.name.label`,
    `entity.name.filename`, and `entity.name.selector`.
  - Tokens inside `meta.function-call`, `meta.parameters`, `meta.type.annotation`, or `meta.return.type` do not count,
    because some grammars (TypeScript) tag calls and type references with `entity.name.*` too.
  - The name is the token, joined with the next tokens on the line that keep the same scope. The kind comes from the scope:
    `fn`, `class`, `struct`, `enum`, `trait`, `interface`, `impl`, `type`, `module`, `macro`, `const`, `section`, or the scope's own word.
  - `depth` is the item's nesting level: its indent's rank among the items' distinct indents. For Markdown headings it is the heading level minus 1.
    Items deeper than 2 are left out.
- The highlighting thread computes the outline in the same parse as the colors. An outline-only job parses without colors.
- Files over 20,000 lines or 2 MiB get no generic outline.

In `minimap-core`: `OutlineItem` gets `depth` (0 for Clojure and for the fake repo's JSON).

In the app:

- A file whose language `minimap-core` outlines (Clojure) keeps that outline. Every other text file gets the generic outline.
- When the selection moves to such a file, the app sends an outline-only job, once per file in the session. A viewer job also brings the outline.
- The panel, `o` browse mode, `]]`/`[[`, the shape's `·` markers, and `e` on an outline item use the generic outline the same way.
  While it is loading, `o` flashes `The outline is still loading.`
- The OUTLINE rows indent each item's name by 2 spaces per depth.
- `docs/design.md` `Selection panel` records the rule.

## Validation

- [x] Unit tests of `Highlighter::outline`: Rust (struct, enum, trait, impl with a method at depth 1, fn, macro), TypeScript
  (interface, function, class with a method, type alias, enum; calls and type references left out), Python (class, method, def; calls left out),
  JavaScript, SQL (`create function`), and Markdown (headings, depth by level). Plain text has none.
- [x] Unit test: a Clojure file keeps its tree-sitter outline, not the generic one.
- [x] App test: selecting a Rust file in memory brings its outline; the panel shows it, and `]]` in the viewer jumps to the next item.
- [x] Snapshot test at 150 × 45: the panel with a Rust outline.
- [x] Real-repo test: for the real repo's `.ts`, `.js`, `.py`, `.sql`, and `.md` files, and `minimap`'s own `.rs` files, print the share of files with an outline
  and its median size. Every item line is within the file, lines rise, and no item is a call. Of the files whose text matches a definition regex
  (`fn`, `function`, `class`, `def`, `interface`, `create function`, `#` heading), at least 90 % get an outline.
- [x] Visual check (see `AGENTS.md` `Visual check`): `layout.rs` in `minimap`'s own repo and a `.ts` file on the real repo show an outline, and `]]` follows it.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-28)

- Coverage on the real repo: every file with a named definition got an outline.
  All 41 of `minimap`'s `.rs` files got one too. A SQL file that holds only a `select` query has no definitions.
- Two more rules came from real files. The coverage check counted anonymous `function () {` callbacks, which name nothing, so it now requires a name.
  The JavaScript grammar misread code after an object literal and tagged `console.log(` as a method, so a name right after `.` never counts.
- Markdown levels come from the line's leading `#` count, because the grammar does not give level 3 the same way.
- Rust `const` items get no scope from this grammar, so they are not in the outline. The SQL grammar tags every created object as a function.
- An impl item reads `impl <type>`, so it does not look like a second copy of the type.
- Visual check: `layout.rs` showed 41 items with nested methods, and `]]` stepped through them.
  A `.ts` spec on the real repo showed its interface and functions, without its method calls (`.private/captures/visual-032-ts-outline.txt`).

## Scope

- No outline config per language, and no language server.
- Clojure keeps unit `007`'s outline. No change to the namespace line.
- No tree view with folding: depth only indents the names.

## Context

- Unit `029` syntax thread: `crates/tui/src/syntax.rs` `Worker`, `prepare`. Outline users: `crates/tui/src/app.rs` (`open_outline`, `on_outline_key`, `on_viewer_key`), `crates/tui/src/ui.rs` (`draw_file_info`, `draw_outline`, `draw_shape`).
- Prototype (2026-09-28): Rust gave `entity.name.{struct,enum,impl,function,type}.rust`, Markdown `entity.name.section.markdown`,
  Python `entity.name.function.python`, SQL `entity.name.function.sql`. TypeScript also tagged calls (`meta.function-call`) and type annotations.
- Decided in the M3b review (2026-09-28): add it to 3b before the milestone commits.
