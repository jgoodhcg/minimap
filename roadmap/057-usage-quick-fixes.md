---
title: "Usage quick fixes: note clicks, history sort, session colors, text copy"
status: done
description: "Five changes from the first usage observations that need no decision: a note row click selects before it opens, a sort chip for the history, a color per session, N in the map hint, and text selection with copy in the reader"
created: 2026-10-16
updated: 2026-10-16
tags: [milestone-g9, gui]
priority: high
---

# Usage quick fixes

## Intent

The user's first sessions with `minimap-gui` on the target repo gave fourteen observations (`roadmap/index.md` `Usage observations`).
Five of them need no design decision and conflict with no stated goal. This unit ships them together.
The other observations stay in `Quick Ideas` until the user decides them.

## Specification

Each part changes `docs/design.md` where it changes a settled rule. Each part keeps the map's tiles in place.

### 1. `N` in the map hint

- `MAP_HINT` (`crates/gui/src/app.rs`) ends with `n N notes` in place of `n notes`, so the backward key shows where the forward key shows.

### 2. A note row click selects first, then opens

Today a click on a note row in the agent history opens the reader at once (unit `054`). The user wants to see the file on the map first.

- A click on a note row whose file is not the selection selects the file as a trail row click does (unit `051`): the tree opens to it, the panel shows it,
  and the map moves only as far as needed to show the card, with no zoom change. The reader does not open.
- A click on a note row whose file is already the selection opens the note in the reader. A second click on the same row therefore opens it.
- A double-click on a note row opens the note in the reader, in place of the zoom that a trail row gets, so a fast user needs one gesture.
- A note row for a path without a card (a deleted file) opens the reader on the first click, as today, because there is nothing to select.
- A note row in the panel's `NOTES` list keeps its rule: its file is the selection, so a click opens the note.
- `n` and `N` keep their rule: they open the reader.

### 3. A sort chip for the agent history

- The head row gets a chip at the right, after the session count: `newest first` or `oldest first`. The default is `newest first`, today's order.
  A click toggles it. The hovered chip draws as a hovered filter chip does.
- The order applies to the shown rows of every filter, so the `notes` filter sorts too. A scrolled strip keeps its top row when the order changes
  when it can; else it shows the first rows of the new order.
- New rows under `oldest first` append at the end and do not move a scrolled strip, as under `newest first` new rows do not move a scrolled strip today.
- The panel's `NOTES` list, the note ordinals, and `n` and `N` keep the tour order. The chip sorts only the strip.
- `activity::shown` takes the order, so the tests read the strip without a window.

### 4. A color per session

- Each distinct session id gets a color from a fixed palette of eight hues that read on `SIDE` and differ from `FOCUS`, the trail kind colors, and `CURSOR`.
  The palette index is the session's arrival order modulo eight. Events without a session keep `DIM`.
- The history row's session text draws in its session's color. The glows and the kind colors do not change: the glow says what happened, the session text says who.
- The head row's session count stays. When there is more than one session, the count is followed by a swatch per session in arrival order: a 6-point square in its color,
  at most eight, with a `…` after the eighth.
- The palette lives in `color.rs` as `SESSION: [u32; 8]`.

### 5. Text selection and copy in the reader

The reader paints its text with the painter (unit `054`), not with widgets, so selection is the reader's own.

- A drag over the reader's body selects text from the press point to the pointer, by row and column, as in a terminal: whole rows between the ends,
  and on the end rows from or to the column under the pointer. A column is `x / char_w` from the text's left edge, clamped to the row's length.
  Rows are the code rows and the note box's wrapped text rows; the gutter is not selectable, and a drag that starts in the gutter selects from column 0.
- The selection draws as `CURSOR` at 25 % over the row, below the text, and above the noted-lines tint.
- Cmd+C (Ctrl+C on other platforms) copies the selected text through `egui::Context::copy_text`, one line per row, with no gutter and without the trailing cut of long lines:
  the copy takes the full line text, not the part on the screen. Rows of the note box copy as the note's text lines.
- A click without a drag clears the selection. Esc with a selection clears it; Esc without one closes the reader. Scrolling keeps the selection; a note change or a reload clears it.
- The status bar hint of the reader ends with `drag select · ⌘c copy`.
- The wheel, the keys, and the mode chips work as before during a selection.

## Validation

- [x] `MAP_HINT` test: the hint contains `n N notes`.
- [x] GUI test: a click on a note row selects its file and does not open the reader; a second click opens the reader; a double-click opens the reader; a note row without a card opens on the first click.
- [x] GUI test: `activity::shown` with `oldest first` gives the rows in arrival order for `all` and for `notes`; a new row does not change the top of a scrolled strip.
- [x] GUI test: two sessions get two colors; the ninth session repeats the first color; a row without a session is `DIM`.
- [x] GUI test: a drag from row 3 column 4 to row 5 column 2 selects the right spans; the copied text has the full lines without the gutter; a click clears it.
- [x] Snapshot tests (`crates/gui/tests/snapshots`) updated where the hint or the head row changed. No snapshot holds the hint or the head row, so none changed.
- [x] `AGENTS.md` validation levels 1–3 and 4b's tests pass. The real-repo tests run with `--no-fail-fast`.
- [x] No window opens during the stint (the user is working). The GUI visual check waits for the user's consent at the milestone exit.
- [x] GUI visual check: the user skipped it at the milestone exit (2026-10-28, unit `059`). The selection fill's contrast, the session swatches, and the `⌘` glyph are not checked on a real window.

## Result (2026-10-16)

- `app.rs`: `MAP_HINT` ends with `n N notes`, and `READER_HINT` with `drag select · ⌘c copy`. `click_activity` opens a note only when its file is the selection or has no card;
  else it clicks as a trail row. After a notes clear, a note row clicks as a trail row.
- `activity.rs`: `Order`, `shown(records, filter, order)` in the strip's order, the sort chip (`StripHit::Order`), `session_rgb`, and the head row's swatches.
  While scrolled oldest first, the right side reads `M older above`. A narrow strip drops the scroll count, then the sessions, then the sort chip.
  `GuiApp::set_order` keeps the top row when it can. Oldest first, rows that drop past 5,000 leave the scrolled top.
- `color.rs`: `SESSION`, eight hues 23° or more from `FOCUS`, the trail colors, and `CURSOR`, 26° or more apart, and a contrast of 6.6 or more on `SIDE`.
  A row without a session draws `—` in `DIM`; it was `FAINT`.
- `reader.rs`: `Spot`, `Selection`, `spot_at`, `selected_text`, and `box_rows`, which wraps as `text::wrap` does and keeps each char's place in the note's text.
  The note box copies as the note's text lines with their own spaces. Columns round to the nearest cell edge.
  A mode change and a new region width also clear the selection, because the rows change.
- `reader.rs` renames the closure `ordinal` in `build` to `number`. The TUI real-repo test `grammar_outlines_cover_the_main_non_clojure_types_and_minimaps_rust`
  read `let ordinal = |id| facts.notes.ordinal(id)` as a call outlined as an item. The line came from unit `054`, so the test failed at `HEAD` too.
- Tests: `crates/gui/tests/activity.rs` (sort, scroll, session colors, palette), `notes.rs` (hints), `reader.rs` (note row clicks, drag and copy, the note box copy, a headless frame's `CopyText`),
  and `real_repo.rs` `usage_fixes_on_the_target`.
- On the real repo (debug build): 21 note rows click-select, then read. A selection of every line of the 20 largest text files and a 1-line file copies each file's text exactly,
  also for lines wider than the reader. The copy of the largest file takes under 10 ms. The `quayside` test covers tabs.

## Scope

- No markdown in notes, no multi-select, no split view, no heading sizes: those observations wait for decisions.
- No selection in the panel, the tree, or the history strip; the reader and its note box only.
- No change to the TUI.

## Context

- `crates/gui/src/app.rs`: `MAP_HINT`, `READER_HINT`, `click_activity`, `open_note`, keys.
- `crates/gui/src/activity.rs`: `shown`, `Filter`, the head row, `short_session`.
- `crates/gui/src/reader.rs`: `build`, `paint`, `Row`, hits.
- `crates/gui/src/color.rs`: the palette.
- `docs/design.md`: `Agent history`, `Note reader`, `Notes`.
- `roadmap/index.md` `Usage observations`: the source of the five items.
