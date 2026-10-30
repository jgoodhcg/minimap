---
title: "Heading size by the count of files below"
status: done
description: "Size each directory name on the CARDS map by the count of all files below it, at every depth, so a parent's name is never smaller than its child's"
created: 2026-10-30
updated: 2026-10-30
tags: [milestone-g12, gui, core, layout]
priority: high
---

# Heading size by the count of files below

## Intent

Usage observation (2026-10-14 to 2026-10-16): heading size carries no meaning. A child directory such as `src/` can draw bigger than its parent.
The goal is waymarkers that help the visual cortex remember the codebase.
The user decided on 2026-10-30: a directory name's size encodes the count of all files below it, at every depth, not only its direct children.

Today (unit `040`) the size is the largest at which the name spans 30 % (`NAME_SHARE`) of its block's width.
So a short name draws bigger than a long one, and on quayside 34 child names draw bigger than their parents'.
The header band's rows follow the name size, so the change moves tiles. This unit proves which tiles move and measures the cost.

## Specification

Name size and band (`minimap-core` layout, `CARDS` only):

1. `LayoutConfig::name_share` becomes `LayoutConfig::names`, an enum `Names`: `None` (every block has `HEAD` header rows), `Count` (this unit),
   and `Width(share)` (unit `040`'s rule, kept for the stability report and the move tests). `CARDS` uses `Count`. `CHARS` and `TILES` use `None`.
2. `n` is the count of all files below the directory, at every depth: its own files and the files of every block below it.
   The tree's files count, so hidden, generated, and allowlisted ignored files count, and no toggle changes `n`.
   `plan` sums `n` bottom-up, before `size` reads it, so the layout stays one bottom-up pass.
3. The name size in points at zoom 1 is `14.4 × sqrt(min(n, 10,000))`, at least 14.4 (one file or none).
   On log-log axes this is a straight line from the current floor (0.8 × the header row, 14.4 points) at 1 file to a cap of 1,440 points (100 times the floor) at 10,000 files.
4. The band has `ceil(sqrt(min(n, 10,000)))` rows, at least `HEAD`, rounded up to a `Growth(1.25)` step. Because `BAND_K × NAME_MIN` is 1, the band holds 1.25 × the name size, as before.
   Compute the band with integer math (`isqrt`), so a square count gives the same band on every platform.
5. `LayoutConfig::name_band(w, chars, files)` gives the size and the band. With `Count`, `w` and `chars` do not change the result.
6. The GUI draws each name at its layout size, as now. A name wider than its band is cut with `…` at its end, as now. No GUI code changes, only comments.

Tests and reports:

7. Replace the unit `040` band tests with count tests on quayside: the formula per block, monotone in the count, independent of the width and the name, the cap,
   no child name larger than its parent's (34 with the width rule), and a block that keeps its band while its count stays inside its step.
8. A move test on quayside compares the width rule with the count rule and proves which tiles move:
   every card keeps its place in its directory's file grid; a block whose band and whose descendants' bands held keeps its size, and its content keeps its places relative to its top-left;
   and it pins the counts of changed bands, resized blocks, blocks that moved as a whole, moved cards, and districts that kept their place.
9. The `--cards` stability example adds the row "count bands, generated cap" beside "width bands, generated cap" (the earlier reports' "name bands").
   Run it on the real target repo and write the report under `.private/`.
10. A core real-repo test (read only, numbers only) checks the band formula, the parent rule, and the file-grid rule on the target, and prints both rules' map sizes, band shares, and cut names.
11. Update `docs/design.md`: the `Layout` table, the `CARDS` header bands, `Stability principle`, and the GUI's name fade note.

## Validation

- [x] Core layout tests on quayside: the band formula; a larger count gives a name at least as large; the width and the name length change nothing;
  the cap at 10,000 files; no child name larger than its parent's (34 with the width rule); a block keeps its band while its count stays inside its step.
- [x] The move test on quayside (`count_bands_move_blocks_and_never_a_card_inside_its_grid`), with its counts pinned.
- [x] The `CARDS` snapshot changes. No `CHARS` or `TILES` snapshot or parity test changes, and no TUI render snapshot changes.
- [x] GUI scene tests: after a zoom to any quayside directory, its name reads as text and does not fade; the item count snapshot changes; each name stays in its band.
- [x] Real-repo tests (read only): `count_name_bands_on_the_target`, the GUI names at the fit view and at card width 33, and the `TILES` and `CHARS` golden layouts.
- [x] Stability report on the real target repo under `.private/`, compared with the earlier reports.
- [x] `AGENTS.md` validation levels 1–3 and 4b's tests pass. Level 4's TUI render snapshots pass unchanged; the TUI visual check is skipped because the TUI does not change.
- [x] No window opens during the stint.
- [ ] GUI visual check on the real target repo: waits for the user's consent at the G12 exit (it opens a window).
  Check the root and the districts at the fit view, a zoom to a small and to a large directory, and the cut names of long compacted chains.

## Result (2026-10-30)

- Layout: `Names::{None, Count, Width}`, `LayoutConfig::names`, `name_band(w, chars, files)`, `NAME_CAP_FILES` (10,000), and `Plan::files` (the counts, summed in `plan`).
  `NAME_SHARE`, `NAME_ADVANCE`, and `BAND_K` stay for `Names::Width`. `CHARS` and `TILES` use `Names::None` and do not change.
- The TUI's `CHARS` map does not use the bands: its blocks have one header row. No TUI tile moves, and the TUI's render snapshots and the real repo's `CHARS` golden layout pass unchanged.
- quayside: the root's name is `14.4 × sqrt(2,585)` = 732 points in a band of 60 rows (was 1,602 points and 118 rows).
  449 of 468 names are larger than the floor. No child's name is larger than its parent's (was 34).
  The map is 3,663 × 801 cells (was 3,604 × 859), and the bands take 17 % of its area (was 24 %).
- Which tiles move on quayside, from the width rule to the count rule: 228 bands grew and 90 shrank of 468; 275 blocks changed size; 119 blocks moved as a whole.
  Every one of the 2,585 cards moved on the map: the root band lost 58 rows, and 15 cards moved by that alone. 1 of 14 districts kept its place under the root band;
  the others moved because the new band heights repack the root's shelf. No card moved inside its directory's file grid.
- A block keeps its band while its count stays inside its step, whatever its width does: in the test, 48 blocks changed width and kept their bands.
  The band changes when the count passes a square of a step: 1, 4, 9, 16, 25, 49, 81, 144, and so on. Small directories pass steps often; large ones seldom.
- Names after a zoom to their directory: every quayside name reads, from 14.4 to 136 points (unit `040` checked only blocks 200 cells wide or wider).
- Cut names: 63 of 468 quayside names are wider than their band. They keep at least 18 characters (median 21): long compacted Polylith chains such as `src/quayside/reporting/` lose their last segment.
- Alternatives measured and not adopted: 2 times per tenfold count (log-log slope 0.30) and slope 0.40. At the fit view of quayside and of the real repo,
  slope 0.30 drew 0 or 1 names as text and slope 0.40 drew 4; the square root keeps about as many as the width rule.
- Real repo (read only): the bands take about half their former share of a smaller map, and no child name is larger than its parent's.
  At the fit view, a few names fewer read as text and more draw as bars; the districts that read as text stay the same in number.
  Every card moved once, and only as part of a block. Report: `.private/stability-2026-10-30-count-bands.md`; names: `.private/names-062-2026-10-30.txt`.
- Stability over a year of the real repo (same 50 samples): count bands moved a little more than width bands, and the oldest-to-newest moves are longer.
  The earlier reports used other windows; the report compares with them.
- Real-repo test runs: `diff_counts_sit_between_git_minimal_and_git_default_on_recent_commits` fails on a new commit of the target, in unit `030`'s diff code, which this unit does not touch.
  The fuzzy timing test failed under load in the full run and passed alone. The tests that read `.private/fixtures/` ran from this worktree through a temporary `.private/` directory with links to the main checkout's fixtures and captures.
- No window opened during the stint.

## Scope

- Directory name sizes and header bands on `CARDS` only. No change to card sizes, the packing order, or the rounding.
- No change to the TUI's map or its text.
- No change to the GUI's text thresholds, fades, or detail levels. No middle cut for long names.
- No other encodings (depth in the tree, edit activity): the user chose the count.

## Context

- Decided (the user, 2026-10-30): the size encodes the count of all files below the directory, at every depth. The user left the mapping to the agent.
- Defaults taken (2026-10-30), for the user to override at review:
  the square root of the count (a straight line on log-log axes); the floor 14.4 points at 1 file (the current floor);
  the cap at 10,000 files, 1,440 points; the band `ceil(sqrt(n))` rows with the existing `Growth(1.25)` rounding;
  every file in the tree counts (hidden, generated, and ignored files); a cut name keeps its start and ends with `…`, as before.
- Why the square root: a block grows about as the square root of its area, so the name keeps about the same share of its block, and the fit view keeps its readable names.
  Its bands are whole rows at square counts, so integer math gives them exactly.
- Code: `crates/core/src/layout.rs` (`Names`, `LayoutConfig::name_band`, `plan`, `size`), `crates/core/examples/stability.rs` (`--cards`),
  `crates/core/tests/layout.rs`, `crates/core/tests/real_repo.rs`, `crates/gui/tests/scene.rs`, `crates/gui/tests/real_repo.rs`.
- Units `040` (names that scale with the zoom, the width rule), `039` (line-scaled cards and the `--cards` report), `041` (the generated cap), and `009` (the stability measurement).
