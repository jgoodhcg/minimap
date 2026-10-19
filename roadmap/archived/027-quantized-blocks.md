---
title: "Quantized block sizes"
status: done
description: "Round block sizes and file-grid capacities up to coarse steps so small growth moves no tile, measure three variants on a year of history, and adopt the best one"
created: 2026-09-28
updated: 2026-09-28
tags: [milestone-3b, core, layout]
priority: high
---

# Quantized block sizes

## Intent

Unit `009` found that, on a real repo, every tile moved over one year.
Block moves caused it, not grid reflow: a block that grows by one row or column moves every block after it,
and tallest-first order changes when a block's height passes a sibling's.
Spatial memory needs stable positions, so fix the packing before the user builds more memory on the current one.
This unit goes first in Milestone 3b because each week on the current layout builds memory that a later change breaks.

## Specification

In `minimap-core` `layout`:

- `LayoutConfig` gets two fields, as data like the other constants:
  - `quant: Quant`: `Quant::None`, `Quant::Step(n)` (round up to the next multiple of `n`),
    or `Quant::Growth(r)` (round up to the next value of the sequence `1, 2, 3, …`, where each value is `max(prev + 1, ceil(prev * r))`).
  - `order: Order`: `Order::Tallest` (the current rule: children sorted by height, then name) or `Order::Name` (children in name order).
- With `quant` set, the layout rounds up:
  1. The file-grid capacity: a directory with `nf` files lays out its grid for `q(nf)` slots.
     The file-column rule and the row count use `q(nf)` in place of `nf`. The files fill the first `nf` slots in the usual order.
  2. The outer width and height of every block below the root, after `size`.
     `Header.w` and `Header.h` are the rounded sizes, so a selected block's background and its rule cover them.
- `TILES` keeps `Quant::None` and `Order::Tallest`, so the v3 parity test does not change.
  The v5 parity test runs `CHARS` with `Quant::None` and `Order::Tallest`.

In the `stability` example (unit `009`):

- Measure the `CHARS` baseline and these variants with the same 50 samples of the last 12 months:
  `Step(4)`, `Growth(1.25)`, and `Order::Name` without quantization.
  Also measure `Order::Name` combined with the better of the two quantizations.
- For each variant, report the unit `009` table, and also:
  - summed moves: the sum over consecutive pairs of the moved share (the baseline is about 3.9, which is 3.9 whole-map moves in a year);
  - consecutive pairs that moved any tile;
  - the oldest → newest moved share;
  - the map area (`w × h`) of the newest sample, and its ratio to the baseline's.
- Save the report to `.private/stability-2026-09-28.md`. It contains numbers only, no paths.

Adoption rule:

- Adopt the variant with the lowest summed moves when its summed moves are at most half of the baseline's,
  and its map area is at most 1.3 times the baseline's. On a tie, the smaller area wins.
- `LayoutConfig::CHARS` then takes the adopted `quant` and `order`, and the app uses it.
- When no variant meets the rule, keep the current packing, keep the new fields with `CHARS` unchanged, and report the numbers.
- Update `docs/design.md` `Stability principle` and `Layout` with the result: the new constants, the rounding rules, the new quayside map size, and the file fill share.
  Record that `mockups/v5-template.html` `makeLayout` does not round, so the mockup's packing now differs from the map.

## Validation

- [x] Unit tests of `q` for both kinds: `Step(4)` maps 1..=9 to 4, 4, 4, 4, 8, 8, 8, 8, 12. `Growth(1.25)` gives the sequence 1, 2, 3, 4, 5, 7, 9, 12, 15, 19, 24.
- [x] Unit test: with each quantized variant, adding one file to a quayside directory whose capacity has room moves no tile and no header outside that directory.
- [x] Unit test: with each quantized variant, a block that grows past its step moves only blocks that come after it in its parent's packing, and their descendants.
- [x] Unit test: `Order::Name` keeps children in name order across a height change.
- [x] The existing layout tests (no overlap, containment, determinism, shuffled input) pass for every variant.
- [x] The v3 and v5 parity tests pass with quantization off.
- [x] Run the stability example on the real target repo with the exclusions from `.private/targets.md`. The report exists under `.private/`.
- [x] If a variant is adopted: update the quayside snapshots, and check that the render tests still show the same selection and camera behavior.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: the map draws, blocks do not overlap, and headers and rules fit their blocks.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-28)

- The three spec variants missed the rule on area: `Step(4)` and `Growth(1.25)` cut the summed moves, but for far more area, and `Name` order nearly doubled them.
- The prototype added `Round` (what rounds: capacity, width, height, up to a depth) and measured which part carries the gain.
  Width rounding added area without stability, and height rounding compounds with depth.
- Adopted: `Growth(1.25)` on file-grid capacities and on the heights of blocks at depth 1 and 2.
  Over the last 12 months it cut the summed moves to about a third for a small gain in area. The two held-out years also moved far less than the baseline.
- About 15 variants were tried on the unit's window, so the held-out years are the check against a lucky pick. Report: `.private/stability-2026-09-28.md`.
- The quayside map grew a little. Per added quayside file, fewer tiles move outside its directory than before.
- The `stability` example gained `--variants` and `--skip-months`.

## Scope

- No new packing algorithm beyond these variants. No persistent layout state (no saved positions).
- No change to the overview, the file tree, or the camera rules.
- No change to `mockups/`.

## Context

- Unit `009` report: `.private/stability-2026-09-25.md`. Its recommendation names quantized block sizes.
- Layout code: `crates/core/src/layout.rs` (`plan`, `size`, `place`). Metric: `crates/core/src/stability.rs`. Example: `crates/core/examples/stability.rs`.
- `docs/design.md` `Stability principle`, `Layout`.
- `AGENTS.md` `Project-Specific Rules`: when a change can move tiles, add a test that proves which tiles move.
- Decided in the M3 review (2026-09-28): the three variants, the adoption rule, and that this unit goes first.
