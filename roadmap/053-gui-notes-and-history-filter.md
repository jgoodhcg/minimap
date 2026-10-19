---
title: "GUI notes on the map and a history filter"
status: done
description: "Show notes in the window: a numbered mark on each noted card at every zoom (layer 2), a glow, note rows in the agent history, and filter chips that show one kind of event"
created: 2026-10-09
updated: 2026-10-09
tags: [milestone-g7, gui]
priority: high
---

# GUI notes on the map and a history filter

## Intent

The user asked to see the files of a review "highlighted when I'm zoomed out" and to filter the event history to notes "to kind of jump around" (2026-10-09).
A guided review sends some notes among many reads and searches, so the history needs a filter, and the map needs a mark that lasts longer than a glow.
This is layer 2 of `docs/design.md` `Layers` (focus, `#f2d15c`, numbered tiles), drawn on cards.

## Specification

The model:

- `GuiApp::notes: minimap_core::notes::Notes`. `record_event` records `Note` and `ClearNotes` events in it.
- A note event starts or restarts its path's glow in `palette::FOCUS`. `scene::Glow` carries its color in place of the trail kind.
- `ClearNotes` empties the notes and their marks. The history keeps its rows, as for a trail `clear`.

Layer 2 on the map (`Toggles::notes`, on at startup; `2` and the status bar's `2 notes` toggle it):

- At the block level, a noted card draws a mark above the cards and headers: the card grown about its center to at least 8 points (`NOTE_MIN`),
  in `FOCUS` at 90 % over the pane, so a noted file shows at the fit view.
- From the icon level, a noted card's fill is `FOCUS` at 30 % over the card (`NOTE_MIX`). It wins over the trail's tint; the hover fill wins over it.
- From the icon level, a badge sits on the card's top-right corner, centered on the corner: a `FOCUS` circle as wide as the name row (18 world points),
  with the ordinal of the file's first note in `BAR`, bold, at 12 world points (a pill when the ordinal has 3 digits).
  The ordinal follows the text-in-the-world rules: text from 7 points on the screen, else no number.
- A noted file's tree row has `▌` in `FOCUS` in its first cell, in place of the trail's mark.
- Marks draw above the cards and headers and below the glows. They never move a card, and they do not count for hits.
  Layer 2 off hides the marks and the note glows. The history and its note rows stay.

The agent history (unit `051`):

- A note row: the time, `note` in `FOCUS`, the path, the lines (`40–58`), the note's first text line in `INK`, and the session.
  The path takes at most half of the room left after the kind, so the text shows.
- Filter chips in the head row, after `AGENT HISTORY`: `all N`, `notes N`, `edits N`, `reads N`, `searches N`, with the row counts of this window.
  The active chip is `CURSOR` with white text, another chip is `DIM`, and a hovered chip is `TILE_HI`. A click on a chip shows only that kind's rows,
  and the strip scrolls to its newest row. The right side reads `K sessions`, after `M newer above` while scrolled.
- A filtered strip without rows reads `No notes yet. An agent sends them with minimap note.` (or `No edits yet.`, `No reads yet.`, `No searches yet.`).
- A click on a note row selects its file, as a click on any row does (unit `051`). Unit `054` changes it to open the reader.
- A chip row that does not fit the strip's width drops chips from the right; the counts at the right drop first.

## Validation

- [x] App-state tests on `quayside` with no window: a note event adds a note, a glow in `FOCUS`, and a history row with `note` and its text;
  a `ClearNotes` event empties the notes and keeps the rows; the block level draws one note mark per noted visible card, at least 8 points wide;
  from the icon level the card fill is the note mix and the badge has the ordinal; `2` hides the marks and the note glows and keeps the rows;
  no card's world rect changes when notes arrive, clear, or the layer toggles.
- [x] Strip tests: the chips and their counts; a click on `notes` shows only note rows, newest first, and scrolls to the top;
  the empty texts per filter; a press on a chip, then a new event, then the release, sets the filter; a narrow strip drops chips from the right.
- [x] A headless frame draws the badge's number at card width 200.
- [x] Real-repo test: 30 notes on real-repo files at the fit view give one block mark per visible noted card, and the card rects do not change.
- [x] Headless frame probe on the real repo with 30 notes and the strip filtered: p95 under 8 ms.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass. GUI visual check: deferred to the milestone exit, with the user's consent.

## Result (2026-10-09)

- Scene: `Glow { path, color, note, at }` in place of the trail kind; `Toggles::notes`; `Map::noted` (the ordinal of each file's first note);
  `note_marks` with `Kind::NoteMark`, `NoteBadge`, `NoteNumber`; `NOTE_MIN` 8, `NOTE_MIX` 0.30, `BADGE_NUMBER_PT` 12. A noted card's fill wins over the trail's tint.
- History: `activity::Act` (`Search`, `Read`, `Edit`, `Note`), `Filter` with its chips, `StripHit` (`Row`, `Filter`), `shown`, `first_line`, `NOTE_ROW_CHARS` 200;
  `Record` has `note` and `text`. `GuiApp`: `notes`, `activity_filter`, `set_filter`, `activity_rows`, `update_notes`, `Toggle::Notes`, `2`.
  A trail `clear` keeps the note glows; a notes clear keeps the trail glows. The tree's `Look` has `notes`.
- Tests: `crates/gui/tests/notes.rs` (7). Changed: `activity.rs` (row hits leave out the chips; the head row's counts), `glow.rs` (`color`), `cards.rs` and `real_repo.rs` (the new fields).
  Real-repo: `notes_on_the_target_map` (30 marks at the fit view, a badge number at card width 200, no card moved).
  The frame probe now also draws 30 notes with the strip filtered to them: p95 under 2 ms in both passes; the atlas did not grow.
- Validation: levels 1–3 and 4b pass: 454 tests. The real-repo suite (`--no-fail-fast`): 39 passed, 1 failed, the known fuzzy timing flake,
  which passed alone. GUI visual check: not run; it waits for the milestone exit and the user's consent.

## Scope

- No reader and no panel section (unit `054`). No key for the filter: the chips are clickable.
- No note marks in the TUI.

## Context

- `crates/gui/src/scene.rs` (`card`, `glows`, `Toggles`, `Glow`), `crates/gui/src/activity.rs` (`build`, `Strip`), `crates/gui/src/tree.rs`, `crates/gui/src/app.rs`.
- `docs/design.md` `GUI (prototype)`: Trail, Glow, Agent history; `Layers`.
