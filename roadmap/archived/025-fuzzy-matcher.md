---
title: "Fuzzy matcher"
status: done
description: "Port telescope-fzf-native's fzf matcher to minimap-core, and check it against the compiled C library"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-2c, core]
priority: high
---

# Fuzzy matcher

## Intent

The user finds files in nvim with Telescope and `telescope-fzf-native` (`~/.config/nvim/init.lua`), so fzf's ranking and query syntax are what their hands expect.
minimap's search should rank paths the same way. The matcher lives in `minimap-core`, so another front end can reuse it.

## Specification

New module `minimap_core::fuzzy`. No new crates (`AGENTS.md`: new crates need confirmation; this unit writes the algorithm).

- `Pattern::parse(query)`: fzf extended search syntax. Terms split on spaces, and every term must match (AND).
  - `foo` fuzzy; `'foo` exact substring; `^foo` prefix of the path; `foo$` suffix of the path; `!foo` the path must not contain `foo` (also `!^foo`, `!foo$`).
  - Smart case per term: a term with an uppercase letter is case-sensitive, else case-insensitive.
  - A query with no terms matches every path with score 0.
- `Pattern::score(path) -> Option<Match { score, positions }>`: `positions` are the char indices of the matched characters, sorted, for highlighting.
  The score is the sum of the term scores.
- The reference is the C port that the user's nvim runs: `telescope-fzf-native` `src/fzf.c`.
  It is older than fzf's current `path` scheme, and the user's ranking habits come from it, so port it exactly:
  - `fzf_fuzzy_match_v2` (a Smith-Waterman variant with affine gaps), with its backtrace for the positions;
  - match 16, gap start -3, gap extension -1, consecutive bonus 4, first-character bonus ×2;
  - boundary bonus 8 after any non-word character (`/` included), camel case and digit bonus 7, non-word character bonus 8;
  - the text starts as if after a non-word character; a boundary resets a consecutive chunk.
- Exact, prefix, and suffix terms use `calculate_score` on the matched span. An exact term takes the occurrence whose first character has the highest bonus.
- The C port reads bytes. This port reads chars and gives non-ASCII letters and digits their Unicode classes; only ASCII paths are compared.
- `Candidate::new(path)` prepares a path once; `rank` takes candidates, so a keystroke does not re-prepare every path.
- `scripts/fzf-golden.py` loads the compiled `libfzf.so` with `ctypes` and writes golden scores and positions:
  `crates/core/tests/fixtures/quayside-fzf.json` (committed) and `.private/fixtures/target-fzf.json`.
- `rank(paths, pattern)`: the matches sorted by score (descending), then with the first match in the file name before a match only in the directories, then shorter path, then input order.

## Validation

- [x] Parity: every quayside golden hit has the C library's score, and the positions of each query's top 50 match. The same on the target golden.
- [x] Unit tests against hand-checked fzf behavior:
  - `rules` ranks `berth/rules.clj` above a path with the letters scattered;
  - `bc` prefers the starts of `berth/core.clj` over a mid-word match;
  - consecutive characters beat a gapped match;
  - the positions are the expected char indices.
- [x] Unit tests for the syntax: AND of two terms; `'exact`; `^prefix`; `suffix$`; `!negate`; smart case (`Rules` needs an uppercase `R`); an empty query.
- [x] Unit test: ranking tiebreaks (file name match, then length, then input order).
- [x] Real-repo test: ranking every visible file for 1-, 3-, and 8-character queries takes under 16 ms each in the test build. Each top result contains the query as a subsequence.
- [x] All `AGENTS.md` validation levels 1–3 pass.

## Scope

- Paths only; no content search.
- No `|` (OR) and no escaped spaces from fzf's syntax.
- No UI (unit `026`).

## Context

- `~/.local/share/nvim/site/pack/core/opt/telescope-fzf-native.nvim/src/fzf.c` is the reference; fzf `src/algo/algo.go` is its origin.
- `crates/tui/src/search.rs` has the unit `021` substring search that unit `026` replaces.
