---
title: "GUI note reader"
status: done
description: "Read each note beside its lines in the map region: the noted lines by default, the whole file with the note in place on z; n and N step through the notes"
created: 2026-10-09
updated: 2026-10-09
tags: [milestone-g7, gui]
priority: high
---

# GUI note reader

## Intent

The user asked (2026-10-09): "When clicking on them or using the keyboard shortcut I'd love to render the selection in the file reader with the note annotation around the lines.
Maybe default to showing just the line selection with an option to show the whole file and the note near it."
The window has no file reader yet; the TUI's viewer (unit `029`) is the model. This reader shows one note at a time, so a review reads as a sequence of stops.

## Specification

Place: the reader takes the map region, in place of the map, as the TUI's viewer does. The tree, the panel, and the agent history stay.
Esc returns to the map. The map keeps its zoom while the reader shows; it moves only to show the selected note's card, as a tree click does.

Content (`crates/gui/src/reader.rs`, `build` to draw items and `paint`, as `panel` and `activity` do):

- A head row (20 points): `NOTE 3 of 7` (`LABEL`, bold), the path (`INK2`, cut at the left with `…`) and `:40–58` (`DIM`),
  and at the right two chips, `lines` and `whole file`. The active chip is `CURSOR` with white text; the other is `DIM`; a click switches the mode. A 1-point `RULE` below it.
- Rows of 18 points (`CODE_ROW`) with 13-point text. A gutter of 6 cells holds the line number (`FAINT`; `FOCUS` on noted lines). Tabs run to the next multiple of 4 columns.
  Lines do not wrap; the region clips them. Unit `055` adds syntax colors; until then, text is `INK2`.
- The note box: a label row, `note 3 of 7` (`FOCUS`, bold) and `lines 40–58` (`DIM`), then the text wrapped to the box (`INK`).
  The box runs from the gutter to 1 cell before the region's right edge, on `FOCUS` at 14 % over the pane.
  The noted lines follow it on `FOCUS` at 5 %. A 1-point `FOCUS` frame goes around the box and the noted lines, so the note sits on its lines.
- `lines` mode (the default): 2 context lines in `DIM` above, the box, the noted lines, and 2 context lines below.
- `whole file` mode (`z`, or the chip): every line, with each of the file's notes in a box above its first line.
  The current note's frame is `FOCUS`; another note's frame is `FAINT` and its label `DIM`. On entry, the current box's top is a quarter of the body below its top.
- A note without lines shows the whole file in both modes, with its box above line 1. Noted lines past the file's end are clamped to it.
- Messages in place of the text, below the box: no text source (`--from-json`), a binary file, a file over 2 MiB, and a read error with the error text.
- The mode stays for the next note until it is switched again.

Opening and moving:

- `n` / `N` (Shift+N): the next or previous note in arrival order. From the map, `n` opens the note after the last one read (the first at the start),
  and `N` the one before it (the last at the start). At an end, the reader stays and the top bar flashes `Last note.` or `First note.`
  Without notes it flashes `No notes yet. An agent sends them with minimap note.`
- A click on a note row in the agent history opens its note, also for a path without a card, such as a deleted file.
  A note removed by a clear opens nothing; the click then selects the file, as other rows do.
- The panel shows a `NOTES` section for the selected file, after the trail line and before the shape: the label and the count,
  then one row per note: the ordinal (`FOCUS`), the lines (`DIM`), and the first text line (`INK2`); at most 4 rows, then `… K more`.
  A click on a row opens its note; the reader's note row is `CURSOR` with white text, and a hovered row is `TILE_HI`.
- Opening a note selects its file through the one selection, as a tree click does: the tree opens to it, the panel shows it,
  and the map behind the reader moves only as far as needed to show the card, with no zoom change. So Esc shows the last note's card.
- A click that selects something else closes the reader: a tree row, a history row that is not an open note, an outline row, or a largest-file row.

Keys while the reader shows: Esc closes it; `n` `N`; `z`; `j` / `k` scroll one row; Page Down / Page Up a body less 2 rows; `g` / `G`, Home / End, the first or last row;
`f` and Enter close it and zoom the map to the note's line on its card (`GuiApp::line_camera`). `t`, `p`, `r`, `1`, `2`, `7`, `8`, `x`, `[`, `]` work as on the map.
The map keys (`w` `a` `s` `d`, `+` `-` `0`, `h` `l`) and pinch zooms do nothing. The wheel over the reader scrolls 3 rows per notch, and trackpad points by the row height.
The status bar's hint while reading: `n N notes · z lines / whole file · j k g G scroll · f map · esc close`. The map's hint gains `n notes`.

Live changes: when the live index reports the reader's file changed, the reader reads it again and keeps its scroll. A `ClearNotes` event closes it.

## Validation

- [x] Reader tests with `Content::memory` on `quayside` paths, no window: `lines` mode rows (2 context, box, noted lines, 2 context) and the frame rect;
  `whole file` mode rows and the entry scroll; a note without lines; lines past the end; a long note text wraps in the box; the tab expansion;
  the messages for no source, a binary file, and a read error.
- [x] App-state tests: `n` from the map opens note 1, then 2; `N` at the start opens the last; the end flashes; `z` and the chips switch the mode, and the mode stays for the next note;
  Esc closes it, the zoom is unchanged, and the map shows the note's card; a tree click closes it; a history note row opens its note; a panel note row opens its note;
  the wheel and `j` `k` `g` `G` scroll within limits; map keys do nothing while it shows; `f` closes it and the camera shows the note's line; a `ClearNotes` closes it.
- [x] A headless frame draws the head row, the box, and the lines.
- [x] Real-repo test: notes on the 20 largest real-repo text files and a 1-line file build in both modes; the largest file's build takes under 4 ms in release.
- [x] Headless frame probe on the real repo with the reader on the largest file in `whole file` mode: p95 under 8 ms.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass. GUI visual check: deferred to the milestone exit, with the user's consent.

## Result (2026-10-09)

- GUI: `crates/gui/src/reader.rs`: `ReaderView`, `Text` (`load`, `of`), `Row`, `Facts`, `ReaderHit`, `Page`, `layout`, `build`, `entry_top`, `paint`,
  `span`, `note_lines`, `expand`, `lines_text`, and `CODE_ROW` 18, `CONTEXT` 2, `GUTTER` 6, `TAB_WIDTH` 4, `BOX_MIX` 0.14, `LINES_MIX` 0.05, `MAX_BYTES` 2 MiB.
  `color::mix_hex`. `GuiApp`: `Reading`, `reader`, `reader_whole`, `note_cursor`, `reader_hover`, `content`, `open_note`, `close_reader`, `step_note`,
  `set_reader_whole`, `scroll_reader`, `reader_to_map`, `reader_page`, `note_rows`; `PressOn::Reader`; `MAP_HINT`, `READER_HINT`, `NO_NOTES`, `LAST_NOTE`, `FIRST_NOTE`.
  `select` closes the reader for another entry; tree, outline, and non-note history clicks close it. `main.rs` sets `Content::Repo`.
- Panel: `NoteRow`, `Facts::notes` and `reading`, `Hit::Note`, `notes_list`, `NOTES_LABEL`, `NOTE_ROWS` 4.
- Core: `Notes::record` puts a note's lines in order, also for an event that was not parsed.
- Tests: `crates/gui/tests/reader.rs` (14; a note row on a deleted file clicks and opens the reader with its message). Real-repo: `the_reader_on_the_target` (21 notes read in both modes; the largest file
  builds in `whole file` mode well under 1 ms in release) and `reader_frame_probe_on_the_target` (whole frames while scrolling: p95 under 1 ms; the atlas did not grow).
- Validation: levels 1–3 and 4b pass: 467 tests. The real-repo suite (`--no-fail-fast`): 41 passed, 1 failed, the known fuzzy timing flake, which passed alone.
  GUI visual check: not run; it waits for the milestone exit and the user's consent.

## Scope

- The reader shows notes only: no way to open an arbitrary file, no cursor, no find, no diff mode, no editor handoff.
- No horizontal scroll. No syntax colors (unit `055`).

## Context

- `crates/tui/src/viewer.rs` and `docs/design.md` `Viewer` (the TUI's viewer), `crates/core/src/content.rs` (`Content`).
- `crates/gui/src/panel.rs` (`Column`, `Hit`), `crates/gui/src/activity.rs`, `crates/gui/src/app.rs` (`select`, `line_camera`, `on_event`).
