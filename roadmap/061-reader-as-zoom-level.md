---
title: "The reader as a zoom level"
status: draft
description: "Draw the agent's notes on the code in place, step through them on the map, and select and copy code on the map, so the separate note reader can go"
created: 2026-10-30
updated: 2026-10-30
tags: [milestone-g11, gui]
priority: medium
---

# The reader as a zoom level

## Intent

Unit `060` draws a card's code in place at a deep zoom. The note reader (units `054`, `055`, `057`) still takes the map region to show a note on its lines.
This unit is slice 2 of "the reader becomes a zoom level" (`Quick Ideas`, usage observations): the notes, the tour, and text copy move onto the map,
so a guided review stays on one surface and the user never leaves the map to read.

Start this unit after the user has used unit `060`. Its use decides the open questions below.

## Specification

Proposed; the open questions below can change it.

Notes in place, at the code level (unit `060`):

1. A note's lines get the reader's tint (`FOCUS` at 5 %) and a 1-point `FOCUS` frame.
2. The note box draws in the card body's free right part, beside the note's first line, as a margin note.
   A 120-column line covers about 35 % of the body (unit `060`), so the right part is free for most lines.
   The box starts after the file's longest shown line, or at column 128, whichever is smaller. It is at most 80 columns wide, and the note text wraps inside it.
   The label row and the colors are the reader's: `note 3 of 7` in `FOCUS`, bold, and the text in `INK`.
3. When two boxes on one card overlap, the later note's box moves down below the earlier one. A box can draw past its card's bottom, above the cards below it.
4. A note without lines puts its box beside line 1.
5. The note marks at lower zooms (unit `053`) do not change.

The tour on the map:

6. `n` and `N` ease the map to the next or previous note at the reading zoom: its box and its lines in view, with the lines a quarter of the region's height from the top.
   The reader does not open.
7. A click on a note row in the agent history, or in the panel's `NOTES` section, eases to the note in the same way.
8. The `Last note.`, `First note.`, and `No notes yet.` flashes stay.

Text selection and copy, at the code level:

9. Select code by row and column, as in the reader (unit `057`). Cmd+C copies whole source lines with their tabs, one line per row, and a note box's text lines.
   The gesture that starts a selection is open question 1.

The reader:

10. Retire the note reader, or keep it as a view (open question 2). When it goes, its keys, hints, chips, and tests go with it,
    and `docs/design.md` `Note reader` is rewritten as the in-place rules.

## Validation

To be written when the open questions are answered. Expected:

- [ ] Scene tests on `quayside`: noted lines draw the tint and the frame; a box sits beside its first line at the step 2 column; overlapping boxes stack.
- [ ] Tour tests: `n` and `N` ease to each note at zoom 18 in tour order; a history row click and a panel row click ease to the note.
- [ ] Selection tests: the copied text for a drag over code rows, a drag into a note box, and a click that clears the selection.
- [ ] Stability: the layout and the card rects are unchanged.
- [ ] All `AGENTS.md` validation levels 1–4b pass. GUI visual check at the milestone exit, only with the user's consent.

## Scope

- No change to the note command or the note event (unit `052`). No change to how notes anchor to lines.
- No notes on diffs: annotated diffs stay in `Quick Ideas`.
- No markdown in notes unless the user decides it (open question 5).
- No change to the TUI.

## Context

- Unit `060` (code in place), units `052`–`055` and `057` (notes, the reader, colors, selection and copy).
- `crates/gui/src/reader.rs` (`build`, `paint`, the selection math), `crates/gui/src/app.rs` (`open_note`, `step_note`, `reader_to_map`, the pointer handling).
- `docs/design.md` `GUI (prototype)`: `Note reader`, the notes on the map, keys.
- `roadmap/index.md` `Quick Ideas`, usage observations: the notes items (a second note on one file, overlapping lines, markdown).

## Open Questions (draft only)

1. A drag pans the map today. At the code level, does a drag select text? Candidates:
   - Shift+drag selects, and a drag pans.
   - A drag that starts on code selects, and a two-finger scroll or a drag on the padding pans.
2. Retire the note reader, or keep it as a view for a long note or a small screen?
3. The reader's `lines` mode shows only the note with 2 lines of context. In place, does the rest of the card dim around the current note, or stay as it is?
4. Margin boxes (step 2), or a box over the code above the note's first line, as in the reader?
5. The rules for a second note on the same lines, overlapping lines, and markdown in note text (from `Quick Ideas`).
