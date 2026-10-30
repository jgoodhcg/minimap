# minimap design reference (v5)

This file records the settled design.
It came from the kickoff design session, the v3 mockup, the M1 review, the v4 and v5 mockups, and the M2 and M2b reviews.
`mockups/v5-template.html` is the reference implementation for the screen, the map, and the keys.
`mockups/v3-template.html` stays the reference for the packing algorithm with the v3 tile constants.
When this file and a mockup disagree, this file wins, and the mockup is a bug.

## Purpose

`minimap` is a visual aid that keeps a human engaged while a coding agent writes much of the code.
It runs beside the agent, for example in a herdr pane, and shows the codebase as a stable 2D map.
When the agent reads, searches, edits, or explains a concept, the map shows where.

The map uses the visual cortex.
The user can watch changes happen, take part in decisions, and attach a spatial memory to what would otherwise be only text in a chat stream.
An editor file tree and outline used to give that memory aid. A chat stream loses it. `minimap` gets it back.
The goal is understanding and memory of the codebase, not finding files faster.

`minimap` is a terminal tool on purpose (decision confirmed 2026-09-25, after a desktop GUI and an nvim plugin were considered).
The design constraints of a TUI, and the fit into a CLI-driven agent workflow, are part of what the project explores.
It stays open about the agent and the editor: Claude Code hooks, the MCP tool, herdr, and nvim are adapters, and other tools can replace them.
On 2026-10-01 the user asked for a map that zooms, which a terminal cannot draw.
Milestone G1 prototypes it as a desktop window in egui, `minimap-gui`, that runs beside the TUI on the same core (see `GUI (prototype)`).
The TUI stays an option.

## Vocabulary

| Term | Meaning |
|---|---|
| map | The middle view: every file as one character, grouped in directory blocks with name headers. It pans. |
| overview | The whole map's shape and the viewport, as braille, in the map's top-left corner. `v` shows or hides it. |
| file tree | The left column: collapsible rows of directories and files. `t` shows or hides it. |
| selection panel | The right column: the repo stats, then the selected file or directory. `p` shows or hides it. |
| shape | The braille view of one file, in the selection panel. |
| viewer | One file's text, read-only, in the map region in place of the map (unit `029`). |
| reader | The GUI's view of one note: its file's text in the map region in place of the map, with the note on its lines (unit `054`). |
| district | A top-level directory. Districts have no color of their own (M2 review). |
| tile | One file on the map. In v5 a tile is one character. |
| layer | A toggleable coloring of tiles (trail, focus, mine, age, churn, ignored). |
| card | One file on the GUI's map: a rounded rect as tall as the file's lines, with the icon, and with the zoom, the name and the shape. |
| detail level | What a card shows at its width on the screen: block, icon, name, or shape. Every card has one width, so every card has the same level. |
| note | An agent's pointer at a file or its lines, with an optional text, from `minimap note` (unit `052`). The notes in arrival order are a tour. |

Do not call any view a "minimap". That is the tool's name.

## Stability principle

Spatial memory needs stable positions.

- The layout is a deterministic function of the file tree, and for `CARDS` of the files' line counts. Window size never changes it.
- Selecting, panning, toggling a layer, showing ignored files, hiding hidden files, and showing or hiding the file tree never move a tile.
- In `CHARS` and `TILES`, every tile has the same size. Sizing tiles by line count was rejected for them because a growing file reshuffles the packing.
- `CARDS` (the GUI) reverses this rule, by the G1 review (2026-10-01, unit `039`): every card has one width,
  and a card is as tall as its file's line count rounded up to a `Growth(1.25)` step, so one line has one height on every card.
  A file keeps its card height until its line count passes a step. The user accepted the cost, and a stability report measures it (see below).
- A generated file's card is as tall as a card of 300 lines (unit `041`, 2026-10-02), so a lock file that grows past that moves no card.
- Gitignored files that the user allowlists take reserved slots at the end of their directory. The slot stays empty while the ignored layer is off.

- Sizes round up to coarse steps, so a block keeps its size until it grows past a step (unit `027`).
  A block that keeps its size moves no tile outside it.
- The map follows the working tree (unit `034`). After a re-index, the selected file or directory keeps its place on the screen:
  the camera moves as far as its tile moved. The other tiles keep their layout places, so they can shift on the screen.

Unit `009` measured the unrounded packing over a year of a real repo's history: block moves, not grid reflow, moved the tiles, and every tile moved.
Unit `027` rounded file-grid capacities and the heights of blocks at depth 1 and 2. Over the same year, this cut the tile moves by more than half
for a little more map area. New directories and blocks that pass a step still move tiles.
Unit `039` measured `CARDS` over the same kind of history: line-scaled cards move much more than cards of one size, and every card moved in the year.
Most moves shift whole districts by one column, because a district's width changes. Name order and rounded widths cut the moves only a little, so neither was adopted.
Unit `041` capped generated files, which halved the summed card moves and made the map much shorter.
Unit `062` sized names by the count of files below each directory: the change moved every card once, and only as whole blocks.
Over a year of the real repo, count bands move a little more than width bands, for a smaller map.
The stability reports stay under `.private/`.

## Layout ("Districts")

Reference: `mockups/v3-template.html:71-168` (algorithm), `mockups/v5-template.html` `makeLayout` (constants as data).

- Each directory is a block: one header line, then a grid of files, then its child blocks.
- A chain of directories with no files and one child compacts into one header, for example `src/quayside/` (`v3:83-86`).
- Child directories sort by name. Files sort by name, with hidden files after the others and allowlisted ignored files last.
  A path is hidden when any segment starts with `.`. Hidden files show at startup. `.` hides them, and their slots stay empty (unit `016`).
- Child blocks pack into shelves of stacked columns, tallest first (`v3:119-143`).

The algorithm takes its constants as a configuration.
`CHARS` is the v5 map. `TILES` is the v3 tile map; it stays as the parity reference for the algorithm.
`CARDS` is the GUI's card map (unit `035`), so its card places differ from the TUI map's tile places.
Since unit `039` its cards are as tall as their files' lines, and files flow down columns (see below the table).

| Name | `CHARS` | `TILES` | `CARDS` | Meaning |
|---|---|---|---|---|
| `TW` | 2 | 22 | 22 | Tile pitch in columns |
| `TH` | 1 | 3 | 7 | Tile pitch in rows (`CARDS`: the gap of 1 row under a card, see below) |
| `IND` | 2 | 2 | 2 | Indent of a child block's content (depth > 0) |
| `GX` | 2 | 3 | 3 | Gap between columns in a shelf |
| `GY` | 1 | 1 | 1 | Gap between shelves, and between a file grid and its shelves |
| `VG` | 1 | 1 | 1 | Gap between blocks stacked in one column |
| `HEAD` | 1 | 1 | 1 | Header rows per block (`CARDS`: the least band, see below) |
| `PX`, `PY` | 8, 18 | 8, 18 | 8, 18 | Cell aspect used only for the packing area math |
| file columns | see below | `ceil(sqrt(nf * 0.7))` | as `CHARS`, with `0.7` for `1.1` | Tile columns in a directory with `nf` files |
| label width | `min(name length + 1, 28)` | none | as `CHARS` | Minimum block width, so every directory name shows |
| root `k` | 1.0 | 3.2 | 3.2 | Root packing aspect: target width `sqrt(area * k)`; `k` is 1.5 below the root |
| `quant` | `Growth(1.25)` | none | `Growth(1.25)` | Rounding steps: `1, 2, 3, 4, 5, 7, 9, 12, 15, 19, 24, 30, …`, each step `max(prev + 1, ceil(prev * 1.25))` |
| `round` | capacity; height at depth 1–2 | none | as `CHARS` | What rounds up: each directory's file-grid capacity, and the height of blocks at depth 1 and 2 |
| `order` | tallest first | tallest first | tallest first | Child order before packing |
| `line_pt` | 0 | 0 | 1 | Points per line at zoom 1 for line-scaled cards; 0 gives every tile one size |
| `names` | none | none | count | What sets a directory's name size: none gives every block `HEAD` header rows; count is the count of files below the directory (unit `062`) |
| `gen_lines` | 0 | 0 | 300 | A generated file's line-scaled card counts at most this many lines; 0 gives no cap |

`CHARS` file columns: `min(cap, max(ceil(sqrt(cap * 1.1)), ceil((label width - IND) / TW)))`, where `cap` is the file count `nf` rounded up to a step.
The grid has `ceil(cap / columns)` rows, and the files fill its first `nf` slots.
Small directories spread their files under the name.
Block width is `max(TW, label width, IND + max(file grid, widest shelf))`.

`CARDS` cards (unit `039`):

- A card is 21 cells wide: a name row of 1 cell, then a body of `ceil(step(lines) × LINE_PT / PY)` rows, where `step` rounds the line count up
  to a `Growth(1.25)` step. A file with 0 lines (binary, missing, or empty) has no body. A gap of 1 row follows each card.
- The file grid has `TW × columns` cells across, with the column count from the rounded capacity as above.
  Files flow in name order down a column, then to the next column. The column height is the least height at which the files fit
  in that many columns, and at least the tallest card's pitch. It rounds up to a `Growth(1.25)` step, so a later column can stay empty.
- Hidden files keep their places after the other files, so the hidden toggle moves no card.
- Blocks at depth 1 and 2 keep their rounded heights, and child blocks pack in shelves as above. Block widths are not rounded,
  so a block that passes a step can still change its width, and its parent then repacks.

`CARDS` generated files (unit `041`):

- A file is generated when one of three marks says so. Each one that has an opinion overrides the earlier ones:
  1. A built-in list of lock-file names, by exact name: `package-lock.json`, `npm-shrinkwrap.json`, `yarn.lock`, `pnpm-lock.yaml`, `Cargo.lock`, `Gemfile.lock`,
     `poetry.lock`, `Pipfile.lock`, `composer.lock`, `go.sum`, `flake.lock`, `mix.lock`, `pubspec.lock`, `Podfile.lock`, `deno.lock`, `uv.lock`.
  2. `linguist-generated` in the repo's `.gitattributes` files, as GitHub Linguist reads it: set or any value but `false` marks; `-` or `=false` unmarks; `!` returns to the list.
     Git's attribute rules apply: a later line and a deeper file win, and a directory pattern does not match the files inside.
  3. The config: `[map] generated` for every repo, then `[repos."<path>"] generated`, as gitignore rules. A leading `!` unmarks, also a lock-file name.
- A generated card counts at most `gen_lines` (300) lines: `step(300)` is 363 lines, so its body is 21 rows and the card is 396 points high.
- The index keeps the flag (`IndexFile::generated`, `"g"` in the JSON when it differs from the built-in list). The TUI does not use it.

`CARDS` header bands (unit `040`, sized by count since unit `062`):

- A directory's name size encodes the count `n` of all files below it, at every depth, so a parent's name is never smaller than its child's (the user's decision, 2026-10-30).
  The count includes hidden files, generated files, and allowlisted ignored files, because the tree holds them whatever the toggles show.
- The size, in points at zoom 1, is `14.4 × sqrt(n)`: 0.8 × the header row times the square root of the count. On log-log axes it is a straight line
  from 14.4 points at 1 file to the cap of 1,440 points (100 times the floor) at 10,000 files. A larger count keeps the cap.
  So 10 times the files give a name about 3.2 times as large, and 100 times the files a name 10 times as large.
- The block's header band has `ceil(sqrt(n))` rows, which hold 1.25 × the name size, at least `HEAD`, rounded up to a `Growth(1.25)` step.
  The band changes only when the count passes a square of a step: 1, 4, 9, 16, 25, 49, 81, 144, 225, and so on. The block's files and child blocks start below it.
- `plan` sums the counts bottom-up, so the layout stays one bottom-up pass. The band does not depend on the block's width or the name's length.
- A name wider than its band is cut with `…` (see `GUI (prototype)`). On quayside the cut names keep at least 18 characters: long compacted chains such as `src/quayside/reporting/`.
- Unit `040` sized a name by its block's width: the largest size at which the name spanned 30 % of the block, with an advance of 0.6 of the size.
  A short name then drew larger, and a child could draw larger than its parent (34 cases on quayside). `Names::Width` keeps that rule for the stability report.

Packing rule for each child `s`, in sorted order (`v3:134-141`):

1. Put `s` in an existing column of the current shelf when the column height plus `VG` plus `s` fits the shelf height.
   The child must also fit the column width, or be in the last column with the shelf width staying within the target.
2. Otherwise start a new column when the shelf width plus `GX` plus `s` fits the target.
3. Otherwise start a new shelf.

The fake `quayside` repo with `CHARS` measures 292 × 224 cells (290 × 192 without rounding): the map does not fit a view, and that is intended.
Files fill about 8 % of that area, counting a file's 2-cell pitch. A tighter packing is a later tuning item.
`mockups/v5-template.html` `makeLayout` does not round, so the mockup's packing differs from the map. The parity tests run `CHARS` without rounding.

## Screen regions

Reference: `mockups/v5-template.html` `view`, `drawTree`, `drawMap`, `drawSide`.

- Top bar, 1 row: the breadcrumb of the selection (the file name bright) on the left.
  On the right: a flash message, else the hovered path, else the CC focus banner.
  A re-index that adds or removes files flashes `N added · M removed · K tiles moved`, where K counts the kept files whose tile moved.
  A viewer whose file went away closes and flashes `<path> was deleted.` instead (unit `034`).
- File tree, the left column (unit `018`):
  - Width 34 by default. `[` and `]` change it by 4, between 16 and 80. Dragging its right edge also sets it.
    In a narrow terminal it narrows, down to 16, so the map keeps 30 columns beside the tree's edge column.
  - `t` shows or hides it. Hiding it widens the map. No tile moves, and the map content keeps its place on screen.
  - First row: `FILES` and the visible file count. Last row: a key hint.
- Map, the middle column: the map region, from the row below the top bar. The overview sits in its top-left corner.
  While the viewer is open, it takes the map region in place of the map and the overview (unit `029`, see `Viewer`).
- Selection panel, the right column: width 0 below 100 columns, 44 below 160, else 54 (unit `022`).
  - `p` shows or hides it. Hiding it widens the map. No tile moves, and the map content keeps its place on screen.
  - First row: the repo name (bold) and `N files · M dirs · K lines` (dim), right-aligned; on the next row when both do not fit.
    Then a `─` rule, then the selection.
- Status bar, 1 row: layer toggles (`1 ● trail`, then `4` age and `5` churn), `.` hidden, `m` pulse, `p` panel, `v` overview, and a `? keys` hint.
  The hint drops out when the toggles leave no room for it, as at 90 columns.
  While the viewer is open, it shows ` VIEW `, `line / total`, and the viewer keys, or the find prompt.
  During a search it shows the query and the match count. While a search is kept, it starts with the query (unit `024`).

## Map

Reference: `mockups/v5-template.html` `drawMap`.

Only directories have text on the map.

Headers:
- Root: the repo name and `/` in the focus color `#f2d15c`, bold.
- Depth 1 (district): the name and `/` in ink, bold, then a `━` rule in `#26313c`.
- Deeper: the directory's own name (a compacted chain shows as `src/quayside/`). Depth 2 in ink, bold; deeper in ink2. Then a `─` rule in `#26313c`.

Tiles (one cell each):
- The file-type icon from `crates/tui/src/icons.rs` (unit `015`). With `--no-icons`, `■`.
- Foreground: the file type's color (binary `#66717c`). Background: the cell's state, such as a search match (see `Color`).
- A CC focus file shows its number, `①②③`, instead of the icon.
- Hover: background `#1e2a36`, and the top bar shows the path.

Selection:
- The selected file's cell pulses like a slow cursor: its background eases between 30 % and 70 % of `#5f86ad` over the pane, with a 2 s period, and the icon is white and bold.
- A selected directory's header text pulses the same way, and its block gets the background `#16222e`.
- There is no other selection animation. `m` stops the pulse and holds it at 50 %.

Camera:
- `w` `a` `s` `d` pan 2 rows or 11 columns (Shift: 4×). Dragging the map pans it. The wheel does not pan the map.
- Clamp: x from `-(overview width + 2)` to `layout width - view width + 2`; y from `-1` to `layout height - view height + 1`.
  The overview width here includes its 1-cell margins. The lower x limit lets every tile pan out from under the overview.
  While the overview is hidden, the lower x limit is `-2`.
- A selection change scrolls the map only as far as needed to show the selection with a 2-cell margin, and never leaves it under the overview.
  For a directory, the target is its header and as much of the block as fits.

District colors were removed in the M2 review (unit `023`).

## Overview

Reference: `mockups/v5-template.html` `drawOverview`.

- The top-left corner of the map region, on `#0b1015` with a 1-cell margin.
  At most 26 × 9 cells and 30 % of the map region, keeping the map's aspect.
- One braille dot per visible file, in `#71879b`. No selection, no text.
- The trail (layer 1) colors a cell's dots when the cell has a trail file: edit before read before grep, inside the viewport too (unit `010`).
  Other layers do not show in the overview.
- The viewport: fill `#1a2632`, edge `#3a526a`, dots inside `#aeb9c4`. The trail does not change the fill or the edge.
- A click or a drag in the overview centers the view there.
- `v` shows or hides it (unit `022`). While it is hidden, selection scrolling ignores it.

## File tree

Reference: `mockups/v5-template.html` `treeRows`, `drawTree`. Unit `017` built the rows; unit `018` moves them left.

- Rows come from the compacted layout tree. The root has no row.
  Each directory lists its child directories first, then its files in grid order.
- Directory row: indent, `▸` or `▾`, a folder icon, `name/`. Top-level rows in ink, bold. Right-aligned: the file count, including subdirectories.
- File row: indent, the type icon in its type color, the name in ink2. A CC focus file shows its number; a CC trail file shows `▌` in the trail color.
- `g` or Home selects the first row; `G` or End selects the last row.
- The tree cursor is the selection. There is one cursor.
  Selecting a file on the map, with the arrows, by search, or with Tab opens the tree down to it and scrolls the tree to it.
- The selected row has the background `#2d4155` and white text.
- Hidden entries disappear while hidden files are hidden. A selection that disappears moves to the first visible row.

## Selection panel

Reference: `mockups/v5-template.html` `drawSelection`, `drawDirInfo`.

For a file, in order:
1. The type icon in its type color and the name (ink, bold), the directory, the namespace, then `N lines`.
   An untracked file reads `N lines · untracked` (unit `034`).
2. The CC note, if the file is in the CC focus. Then trail and mine lines.
   The trail line (unit `010`), in the kind's color: `CC edited lines 40–58 · 2m ago`, `CC read this file · 5s ago`, or `CC found this file by search · 1m ago`.
   `CC` is the label of source `claude-code`; another source shows its own string, and an event without one shows `agent`.
   The age counts `s` and `m` below one hour, then uses the age format below.
3. **Shape** (unit `007`): the whole file as braille, up to 40 columns and 4–12 rows.
   Line numbers at the first, middle, and last rows.
   Markers at the right: `◀` for the outline selection, the viewer's cursor, or the focus line, `▐` for an edited range, `·` for a definition.
   While the viewer shows the file, the rows of the lines in view get the viewport fill `#1a2632`: the shape is to the viewer what the overview is to the map.
4. **Outline** (unit `007`): top-level definitions as `line  name`. `o` enters browse mode. `j`/`k` move. Enter views the file at that line (unit `029`).
   While the viewer is open, the definition that holds the cursor is highlighted.
   Every other language gets its outline from the viewer's grammars (unit `032`): a line's first token in an `entity.name.*` scope
   (not `tag`, `label`, `filename`, or `selector`), outside calls, parameters, and type annotations, and not right after a `.`.
   Nested items indent their names 2 spaces per level, up to 2 levels; Markdown headings nest by level. An impl item reads `impl <type>`.
   The highlighting thread computes it when the selection reaches the file.
   For Clojure, a top-level form is a definition when its head's name (after any `alias/`) starts with `def` or `reg-`.
   This covers the clojure-lsp document symbols in the user's nvim outline, plus other `def…` macros. The name is the next symbol or keyword, as written.
5. **Git** (unit `008`): `N commits`, `changed X ago · created Y ago`, then recent commits as age plus description.
   Merge commits do not count. The description strips a `type(#issue):` prefix with `^[a-z]+(\([^)]*\))?!?:\s*` (`v3:211`).
6. A key hint line at the bottom.

For a directory: its path (ink, bold), `N files · M subdirectories`, `K lines`, then its largest files with line counts.

Line shape data (`mockups/fakegen.py` `minimap`):
sample the file into at most 64 rows.
A row of at most 4 lines keeps the minimum indent and the maximum length of its non-empty lines, with length capped at 126.
A row of more lines keeps the median indent and the median length (the lower median of each), so a long file shows its typical lines, not a solid block (unit `028`).
The shape view scales these rows to its braille grid.
The fake repo's `m` field keeps the minimum and maximum; it only feeds the fake repo.

Age format (`v3:207`): `today`, `Nd` below 14 days, `Nw` below 60, `Nmo` below 730, else `Ny`.

## Viewer

Unit `029`. A read-only view of the selected file in the map region. The file tree and the selection panel stay.

- Enter on a file, Enter in outline browse mode, or a double-click on a tile opens it. `q` or Esc returns to the map, which has not moved.
  A file's viewer opens again where it closed in this session.
- Rows: the line number (faint; the cursor line's in ink), then the text. The cursor line has the background `#1a2632`.
  Tabs run to the next multiple of 4 columns. Lines do not wrap.
- Syntax colors: `syntect` with the `two-face` syntax set (bat's), on a background thread.
  The default theme is tokyonight-night (`crates/core/assets/`, moved from `crates/tui/assets/` in unit `055`), to match the user's nvim. `[viewer] theme` in the config names another `.tmTheme`.
  The highlighter is `minimap_core::highlight` (unit `055`), which gives `0xRRGGBB` spans; the TUI converts them to its colors.
  Only the theme's foregrounds, bold, and italic are used. Files over 20,000 lines or 2 MiB show plain text.
- Keys: `j`/`k` with counts, Ctrl+D/U/F/B, `gg`, `G`, `12G`, `zz`/`zt`/`zb`, `]]`/`[[` (definitions), `{`/`}` (blank lines), `h`/`l`/`0` (8 columns),
  `/` find (literal, smart case; matches on `#f2d15c` at 45 %), `n`/`N`, `y` (copy `path:line`), `e` (editor, unit `031`), `t`, `p`, `?`.
  Their names in `[keys.viewer]`: `down`, `up`, `half_page_down`, `half_page_up`, `page_down`, `page_up`, `first_line`, `last_line`, `center_cursor`,
  `cursor_to_top`, `cursor_to_bottom`, `next_definition`, `previous_definition`, `next_change`, `previous_change`, `next_blank`, `previous_blank`,
  `scroll_left`, `scroll_right`, `scroll_home`, `find`, `next_match`, `previous_match`, `toggle_diff`, `copy_location`, `open_editor`,
  `toggle_tree`, `toggle_panel`, `help`, `close`. Outline browse (`[keys.outline]`): `down`, `up`, `open`, `open_editor`, `close`, `quit`.
  `[` and `]` start motions, so the tree width changes only by dragging while viewing.
- Mouse: the wheel scrolls 3 lines; a click moves the cursor; a tree file row shows that file; a tree directory row closes the viewer.

Diff mode (unit `030`): the file's uncommitted changes against `HEAD`, which is what an agent's edits usually are.

- The background thread diffs the working-tree text against `HEAD` (`similar`, Myers) when the viewer opens.
  A file that `HEAD` does not have is all added. A file that changed since its last view gets a new diff.
- A sign column follows the line numbers. In file mode it marks changes as gitsigns does: `▎` on added lines, `▁` above a removal (`▔` for one above line 1).
  The status bar shows `+N −M · = diff`.
- `=` switches to diff mode: the removed `HEAD` lines show between the lines, with `-` and no number; added rows show `+`.
  Added rows get `#3fb950` and removed rows `#f85149`, each at 22 % over the pane. Removed rows keep their syntax colors from the `HEAD` text.
  The status bar shows ` DIFF `. `=` again returns to file mode on the same line.
- `]c` / `[c`: the next or previous change, in both modes. On a removed row, `y` and `e` use the next working-tree line.
- The shape marks rows with changed lines with `▐` in `#3fb950`.
- Our diff is never larger than git's default and never smaller than `git diff --minimal`. Git's default Myers takes shortcuts, so on some real files the counts differ.

## Editor handoff

Unit `031`. `e` opens the selected file in the user's editor at a line: the viewer's cursor line, else the browsed outline item's line, else 1.
Editors are adapters; the map needs none. The `[editor]` config table picks one:

```toml
[editor]
preset = "nvim"     # nvim | vim | vscode | emacs | custom; default from $VISUAL, then $EDITOR, else nvim
place = "tab"       # tab | right | down: where herdr opens a terminal editor
command = ["zed", "{path}:{line}:{col}"]   # custom only; also {root}
terminal = false    # custom only: true runs the command in minimap's terminal until it exits
```

- nvim under herdr: the first `e` creates a focused tab labeled `minimap editor` (or splits minimap's pane) and starts `nvim --listen <socket>`.
  Later opens send `:drop <path> | call cursor(<line>, 1) | normal! zt` to the socket and focus the place.
  The socket is `$TMPDIR/minimap-$USER-<hash of the repo root>.sock`. A quit nvim restarts in its pane; a closed pane gets a new place.
- vim under herdr: vim starts in the place. Later opens type the same `:drop` line into it (`vim --remote` needs `+clientserver`).
- nvim or vim without herdr: minimap leaves the screen, the editor runs in its terminal, and minimap comes back when it exits.
- VS Code: `code --goto <path>:<line>:<col>`. Emacs: `emacsclient -n +<line>:<col> <path>` (it needs `(server-start)`). Both run detached.

## Color

Color has two channels, so a file type and a state never read as one another (M2 review, 2026-09-25; unit `023`).
The user asked for a best guess on the state channel. Revisit it after a visual review.

- A glyph's foreground carries the file type and nothing else.
  Each type has one low-chroma color (HSL saturation at most 40 %) in `crates/tui/src/icons.rs`. The main types get well-spaced hues.
  Only icons carry it: names, headers, and rules are neutral (ink, ink2, faint, `#26313c`).
- A cell's background carries state. State base colors are high chroma (HSL saturation at least 60 %).
  They are drawn at 45 % over the pane, so the glyph on top stays readable. A filled cell reads stronger than a glyph stroke.
  - Priority on one cell, highest first: selection pulse, hover, search match, CC focus, CC edit, mine, CC read, CC grep, git status, heat.
  - Search match: `#3fd67a`.
  - Trail (layer 1, unit `010`): read `#52b3e0`, grep `#d6a63d`, edit `#e2725b`, at 45 % while fresh (among the 3 newest events), else 20 %.
    Read and grep are the v3 hues (`#6cb6d9`, `#c9a14a`) raised to an HSL saturation of at least 60 %. One path keeps its strongest kind: edit, then read, then grep.
  - Heat (layers 4 and 5, which are exclusive): `#e0529c`, drawn at 45 %, 30 %, and 18 % for the three ramp steps (unit `008`).
- In the viewer, the foreground carries syntax, and the background carries the cursor row, find matches, and diff rows (units `029` and `030`).
- The selected cell's glyph is white and bold. CC focus was to replace the icon with its number `①②③`; unit `012` was dropped on 2026-10-30, so the TUI has no focus marks.

## Search

Reference: Telescope `find_files` with `telescope-fzf-native`, the user's nvim picker. Units `025` and `026`.

- `/` opens the picker in the left column, in place of the file tree. The map is its preview.
  Row 1: `SEARCH` and `N / M`. Last row: the prompt `> query▏`. Between: the results, best at the bottom next to the prompt.
- Ranking: a port of `telescope-fzf-native` (fzf's `FuzzyMatchV2` as the user's nvim runs it), with fzf's extended syntax
  (`'exact`, `^prefix`, `suffix$`, `!not`, spaces for AND) and smart case. `scripts/fzf-golden.py` checks the port against the C library.
  Ties: a match in the file name first, then the shorter path, then index order. The code is in `minimap_core::fuzzy`.
- A row: the type icon, the file name (ink), the directory (dim). Matched characters are bold in `#3fd67a`.
- The highlighted result is the selection. The results in the visible rows are marked on the map with the search background.
- Enter keeps the highlighted result and the marked rows (`n` / `N` cycle them). Esc restores the selection and camera from before `/`.

## Layers

| Key | Layer | Source | Coloring on the map |
|---|---|---|---|
| 1 | CC trail | Passive: agent reads, greps, edits, from the event socket (unit `010`) | Background: read `#52b3e0`, grep `#d6a63d`, edit `#e2725b`. `▌` in the tree, colored dots in the overview, a line in the panel. |
| 2 | Notes | Active: agent sends notes with `minimap note` (unit `052`; it replaces unit `012`'s focus tool) | Numbered cards in `#f2d15c` in the GUI (unit `053`). The TUI ignores notes. |
| 3 | Mine | Files the user opened this session | Background `#86c07a`. |
| 4 | Age heat | Git: days since last change | Background `#e0529c` ramps at ≤14 days, ≤60, ≤365. |
| 5 | Churn heat | Git: commit count | Background `#e0529c` ramps at ≥60 commits, ≥25, ≥10. |
| 6 | Ignored | Allowlisted gitignored files | Fills the reserved slots. |

Layers 4 and 5 are exclusive: turning one on turns the other off. The status bar shows the active ramp's legend.
Layers color the map only, except the trail, which also marks the tree, the overview, and the panel.
Layer 1 off hides the trail everywhere and keeps it; `x` clears it. The trail lives for the session and has no size limit.
Unit `013` predates the v5 map; revise its map rendering before it becomes `ready`. Unit `012` was dropped on 2026-10-30: the notes (units `052`, `053`) replaced it.

## Keys

Keyboard first; the mouse also works. The `Name` column is the action name for the config's `[keys]` tables (unit `033`, see `Keymap`).
`minimap keys` prints every action of each mode with its keys.

| Key | Action | Name |
|---|---|---|
| `j` / `k` | Move the selection down or up the file tree | `down`, `up` |
| `l` / `h` | Open or close a directory; on an open directory `l` goes in; on a file or closed directory `h` goes up | `tree_in`, `tree_out` |
| arrows | Move the selection to the nearest file in that direction on the map (`v3:629-641`) | `select_left`, `select_right`, `select_up`, `select_down` |
| `w` `a` `s` `d` | Pan the map (Shift: 4×) | `pan_up`, `pan_left`, `pan_down`, `pan_right`; `pan_up_far`, … |
| `t` | Show or hide the file tree | `toggle_tree` |
| `[` / `]` | Narrow or widen the file tree | `narrow_tree`, `widen_tree` |
| `/` | Search files in the picker (see `Search`); `n`, `N` cycle the kept results; Esc clears them | `search`, `next_result`, `previous_result`, `clear_search` |
| Tab | Next file in the CC focus (unit `012`, dropped 2026-10-30; unbound) | |
| Enter | On a directory: open or close it. On a file: view it (unit `029`); `e` opens it in the editor | `open` |
| `o` | Browse the outline; Enter views the file at the item | `browse_outline` |
| `e` | Open the file in the editor at the line (unit `031`, see `Editor handoff`) | `open_editor` |
| `=` | In the viewer: switch between the file and its uncommitted diff (unit `030`) | `toggle_diff` |
| `]c` / `[c` | In the viewer: next or previous uncommitted change | `next_change`, `previous_change` |
| `y` | Copy the selected path | `copy_path` |
| `Y` | Copy the selected file's namespace | `copy_namespace` |
| `H` | Copy the selected file's newest commit hash | `copy_commit` |
| `1`–`6` | Toggle layers (`1`, `4`, `5` so far) | `toggle_trail`, `toggle_age_heat`, `toggle_churn_heat` |
| `x` | Clear the trail | `clear_trail` |
| `p` | Show or hide the selection panel | `toggle_panel` |
| `v` | Show or hide the overview | `toggle_overview` |
| `g` / `G` | First or last file tree row (also Home, End) | `first_row`, `last_row` |
| `.` | Show or hide hidden files | `toggle_hidden` |
| `m` | Selection pulse on or off | `toggle_pulse` |
| Esc | Close the help, the viewer, the search, or the outline | `clear_search`, `close` |
| `?` | Help | `help` |
| `q`, Ctrl+C | Quit (Ctrl+C always quits and cannot be rebound) | `quit` |

The viewer's keys and names are in `Viewer`, and `minimap keys` lists them.

Mouse: a click on a tile or a directory name selects it; a click on a tree row selects it and toggles a directory; a double-click on a tile views it.
Dragging the map pans it. Dragging the tree edge resizes the tree. Clicking or dragging in the overview moves the view.
The wheel does not pan the map. Over the file tree it scrolls the tree.
Zoom was considered after M1 and dropped: the terminal's font zoom rescales the map, and minimap redraws at the new size.

## Keymap

Unit `033`. The config binds keys per mode, with VS Code-style key strings, because the user asked for an intuitive notation (M3b review, 2026-09-29):

```toml
[keys.map]                 # the map and the file tree
"h" = "pan_left"
"a" = "none"               # removes the default binding of a

[keys.outline]             # outline browse (o)
"ctrl+n" = "down"

[keys.viewer]              # the viewer, in file mode and diff mode
"ctrl+d" = "half_page_down"
"g g" = "first_line"       # a space separates the keys of a sequence
```

- A key the config names takes that action. A key it does not name keeps its default. `"none"` removes a binding.
- A key: one character, or a named key (`enter`, `esc`, `tab`, `space`, `backspace`, `delete`, `insert`, `home`, `end`, `pageup`, `pagedown`,
  `up`, `down`, `left`, `right`, `f1`–`f12`, `plus`), with `ctrl`, `shift`, or `alt` (`option`, `opt`, `meta`) joined by `+`. Names ignore case.
- `shift+g` is `G`; `shift` with another character is an error, so write the character itself. With `ctrl`, a letter's case does not count.
- After the first key of a sequence, minimap waits for the next key, with no timeout. A key that completes nothing ends the wait.
- Fixed keys: Ctrl+C quits; the search picker and the find prompt keep their keys; in the viewer, `1`–`9` start a count.
- Errors stop minimap at startup, before the terminal changes, with one line per problem that names the table and the key:
  a key that does not parse, an action that the mode does not have, two strings for one key, a bound key that also starts a sequence, a count digit in the viewer.
- The help, the status bar, the file tree hint, and the panel hints show the first key of each action they name. The config's keys come first.
  Hints show `ctrl+d` as `^d`, Enter as `⏎`, the arrows as `←` `→` `↑` `↓`, and a sequence without spaces (`gg`).
- `minimap keys [--config PATH]` prints the keymap as config TOML, with removed defaults as `"none"`. The output parses back to the same keymap.

## GUI (prototype)

Milestone G1, units `035`–`038`, Milestone G2, units `039`–`040`, Milestone G3, units `041`–`043`, and Milestone G4, units `044`–`046`.
`minimap-gui` is a desktop window in egui (eframe on wgpu) that draws the `CARDS` map and zooms from the whole repo to one file. It links `minimap-core` directly.
It has the file tree (unit `042`), the selection panel (unit `044`), the agent's trail (unit `047`), the live index (unit `048`),
the glow of each agent event (unit `049`), the agent history (unit `051`), and the agent's notes with a note reader (units `052`–`055`), and no file viewer, search, or other layers yet.
The G1 review (2026-10-01) answered yes and planned G2. It has no app bundle and no signing: a local build runs from the terminal.

- Arguments: `[path]`, `--config`, `--exclude`, `--from-json`, `--no-icons`, `--select PATH` (a file or directory: select it and zoom to it),
  and `--capture DIR` (see below). The window title is `minimap — <repo name>`.
- Fonts: the `[gui] font` file of the config (a leading `~/` expands), else the first `*NerdFontMono-Regular.ttf` by name in `~/Library/Fonts`,
  then `/Library/Fonts`. Its `-Bold` sibling draws bold names. Without a font, or with `--no-icons`, an icon is a square in the type color.
- Screen: a top bar of one text row, the file tree, the map region with the agent history below it, and the selection panel, and a status bar.
  Bars are `#0b1015`; the map is `#10161c`.
  - Top bar: the breadcrumb of the selection, with the file name or last directory bright, as the TUI shows it (`minimap_core::layout::Breadcrumb`);
    then `N files · M dirs` when there is room; the hovered path at the right.
  - Status bar: the zoom and the card width, the toggles `1 trail  2 notes  7 names  8 shape` (ink when on, faint when off; a click toggles), and a key hint.

File tree (unit `042`), the TUI's rows (see `File tree`) in a left column:

- The row model is `minimap_core::filetree`, which the TUI shares: the rows, the open directories, the cursor, and the scroll by whole rows.
- The column is on `SIDE`, with a 1-point edge in `RULE` at its right (2 points in `DIM` while dragged). Rows are 20 points high, with 13-point text that the zoom does not change.
  First row: `FILES` (`LABEL`, bold) and the file count. Last row: the key hint. The rows follow the TUI's: the indent, `▸` or `▾`, the folder icon, `name/`, and the count;
  a file row has its type icon in its color. Without icons, rows have no icon column. The selected row is `CURSOR` with white text; the hovered row is `TILE_HI`.
- Width: 280 points at startup, from 160 to 640. `[` and `]` change it by 40, and a drag on the edge (a 6-point zone, with a resize cursor) sets it.
  In a narrow window it narrows, down to 160, so the map keeps 240 points. `t` shows or hides it; it shows at startup.
- The wheel over the tree scrolls it by rows: 3 per notch, and trackpad points by the row height. It never zooms the map.
- One selection: a tree click selects its row, and a directory row also opens or closes. The map then moves only as far as needed to show the card,
  or the block's top, with a 24-point margin and no zoom change, eased over 160 ms; a target larger than the region shows its top and left edges.
  A double-click on a row zooms to it, as `f` does, and does not close the directory that its first click opened.
  A map click opens the tree down to the entry and scrolls its row into view, and the map does not move. A hovered row hovers its entry on the map, and the reverse.
- The tree, its width, and hiding it never move a card in the world. When the map region changes (the tree, its width, or the window), the camera moves
  so that each world point keeps its screen point, then the camera limits apply.

Selection panel (unit `044`), the TUI's panel (see `Selection panel`) in a right column:

- The column is on `SIDE`, with a 1-point edge in `RULE` at its left (2 points in `DIM` while dragged). Rows are 20 points high, with 13-point text that the zoom does not change.
  `panel::build` turns the facts into draw items, and `panel::paint` draws them, so tests read the panel with no window.
- Width: 360 points at startup, from 240 to 640. A drag on the edge (a 6-point zone, with a resize cursor) sets it; no key changes it.
  In a narrow window the panel narrows first, down to 240, then the tree, down to 160, so the map keeps 240 points. `p` shows or hides it; it shows at startup, as the TUI's does.
- Content, in the TUI's order and colors: the repo name and `N files · M dirs · K lines`, a rule, then the selection.
  A file: the icon and the name, `dir/`, the namespace, `N lines` (with ` · untracked`, and ` · generated` for a generated file, which the TUI does not show),
  the outline (`line  name` rows, `… K more` when cut), and the git group at the bottom. A directory: the path, the counts, the lines, and the largest files.
  The last row is the key hint. No selection: `Select a file in the tree or on the map.`
- Shape (unit `045`), between the facts and the outline, as the TUI's: `SHAPE`, then the file's `LineShape` (at most 64 samples) as bars in `MINI_DOT` on `MINI_BG`,
  in a box 5 cells right of the text's start, at most 40 cells wide, and 40 % of the free rows high (4 to 12), when more than 7 rows are free.
  One line is the box's height over the lines, at most 5 points (a quarter row), so a short file fills the top of the box and the section takes only its rows.
  A bar runs from its indent to its length across 126 columns, 1 point shorter from 3 points up. Line numbers in `FAINT` at the first, middle, and last rows;
  a 4-point tick in `FAINT` at each outline item's line, one cell right of the box.
- Outline clicks (unit `046`): a click on an outline row (`OUTLINE · click to zoom`) picks its item. The row is `CURSOR` with white text, and a hovered row is `TILE_HI`.
  The map eases over 240 ms to the card at width 420, centered across: a card that fits the region is centered in it, and on a taller card the item's line is 24 points below the region's top.
  The card marks the line with a white bar across the middle half of its left padding, as tall as a line and at least 2 points, from the shape level with the shape on.
  The panel's shape shows `◀` at the line and its sample bar in white. Another selection or Esc clears the pick. A line past a capped card's shown lines is its last shown line.
- The wheel over the outline's rows scrolls them: 3 rows per notch, trackpad points by the row height, from the first item to the last full page.
  While items are above, the label's note reads `K above`. A new selection starts at the first item. There are no outline keys: `j` and `k` stay the tree's.
- A click on a directory's largest-file row selects the file as a tree click does, and a hovered row hovers its card. The click replaces the panel under the pointer,
  so a second click at that point within 400 ms zooms to the file.
- The details loader asks for the selected file first at every zoom, so the panel has its outline and namespace while the map shows blocks.

Trail (unit `047`), the TUI's layer 1 (see `Color` and `Layers`) from the same event socket:

- The window listens on the repo's socket at startup and repaints on each event. The trail model is `minimap_core::trail`, shared with the TUI.
  When another minimap listens, the top bar's right side reads `Another minimap shows the trail for this repo.`
- From the icon level, a trail card's fill is its kind's color at 45 % over the card while fresh (among the 3 newest events), else 20 %. The hover fill wins.
  At the block level, the kind's color is 90 % over the pane while fresh, else 60 %, so trail files show at the fit view.
- A trail file's tree row has `▌` in its kind's color in the first cell. The panel shows the trail line below `N lines` (`CC edited lines 40–58 · 2m ago`),
  and the window repaints every second while the selected file has one.
- `1` (the status bar's first toggle) shows or hides the trail, and `x` clears it. The trail never moves a card.

Glow (unit `049`), because the tint did not show a real session's reads (try-out, 2026-10-07): the read color is blue like the ClojureScript type color,
and at the fit view a few blue specks among many blue blocks do not show. A glow differs from the cards by brightness, size, and motion:

- Each path of each event starts or restarts its glow, in the event's kind's color. Its strength is 1 for 1.0 s (`GLOW_HOLD`),
  then a smoothstep down to 0 over 3.0 s (`GLOW_FADE`): 0.5 at 2.5 s, gone at 4.0 s.
- At the block level, the glow box is the card grown about its center to at least 36 points (`GLOW_MIN`); from the icon level, it is the card.
  The halo is the kind's color at 85 % times the strength at the box's edge, fading to nothing 32 points out (`GLOW_REACH`); at the block level it also fills the box.
  The core is `WHITE` 75 % over the kind's color: at the block level it fills the card grown to at least 8 points; from the icon level it is a 3-point ring around the card,
  so the card's name and shape stay readable.
- For 0.9 s after the event, a 2-point ring grows up to 48 points out from the box (ease-out) and fades.
- A card outside the map region gives a dot (7-point radius, white center) 14 points inside the region's edge, on the line from the region's center to the card.
- Glows draw above the cards and headers, never move a card, and do not count for hits. Layer 1 off hides them, `x` and a `clear` event clear them,
  and a re-index keeps them by path. While a glow runs, the window repaints each frame; then it repaints only on input again.

Agent history (unit `051`), the user's request of 2026-10-07: every agent event that this window received, because a glow fades in seconds,
and a file can be off screen or inside a closed tree directory:

- A strip at the bottom of the middle column, below the map region and between the tree and the panel, on `SIDE` with a 1-point rule in `RULE` along its top.
  It is 9 rows of 20 points (180 points): a head row and 8 event rows, with 13-point text. The map region keeps at least 160 points of height,
  and a strip under 2 rows hides. `r` shows or hides it; it shows at startup. Hiding or showing it never moves a card, and the map content keeps its place on the screen.
- Head row: `AGENT HISTORY` (`LABEL`, bold), and at the right `N events · K sessions` (`FAINT`), after `M newer above` while scrolled.
  Without events: `No agent events yet. Claude Code's reads, searches, and edits show here.`, or the trail's notice when another minimap listens.
- One row per path of each trail event, newest first by default (the sort chip, unit `057`): the local time (`11:41:07`, `DIM`), the kind (`read`, `search`, `edit`) in its trail color,
  the path (cut at the left with `…`; `INK2`), an edit's lines (`40–58`, `DIM`), and at the right the session's first 8 characters in its session color, or `—` (`DIM`) without one.
  It keeps at most 5,000 rows. `x` and a `clear` event do not clear it.
- A row of the selected file is `CURSOR` with a white path; a row under the pointer is `TILE_HI` and hovers its card. A row for a path with no card has a `FAINT` path and does not click.
- A click on a row acts as a click on its file's tree row: the file becomes the one selection (the tree opens to it, and the panel shows it),
  and the map moves only as far as needed to show the card, with no zoom change. A double-click zooms to it.
  The first version zoomed on one click, as `f` does; at the try-out (2026-10-07, 13:05) the user asked to keep the zoom.
  A press keeps its row's event, so an event that arrives before the release does not change what the click selects.
- The wheel over the strip scrolls its rows (3 per notch, trackpad points by the row height) and never moves the map. A scrolled strip keeps its rows in place when events arrive.
- The session comes from the event's `session` (unit `050`), which `minimap hook claude-code` sets from the payload's `session_id`.
- Filter chips (unit `053`) in the head row, after the label: `all N`, `notes N`, `edits N`, `reads N`, `searches N`, with this window's row counts.
  The active chip is `CURSOR` with white text, another is `DIM`, and a hovered chip is `TILE_HI`. A click shows only that kind's rows, from the strip's first row.
  The right side reads `K sessions`, after `M newer above` while scrolled. A narrow strip drops chips from the right; the right side drops first.
  A filtered strip without rows reads `No notes yet. An agent sends them with minimap note.` (or `No edits yet.`, and so on). New rows of another kind do not move a scrolled strip.
- Sort chip (unit `057`): at the head row's right, after the session count, `newest first` (the default) or `oldest first` (`DIM`; a hovered chip is `TILE_HI`). A click toggles it.
  The order applies to the shown rows of every filter. While scrolled, the right side reads `M newer above` newest first and `M older above` oldest first.
  An order change keeps a scrolled strip's top row when that row can be the top row in the new order; else, and for a strip that is not scrolled, the strip shows the new order's first rows.
  Oldest first, new rows append at the end and do not move a scrolled strip, and the rows that drop past 5,000 move its top up. The panel's `NOTES` list, the note ordinals, and `n` and `N` keep the tour order.
  `activity::shown` takes the order, so tests read the strip with no window.
- Session colors (unit `057`): each distinct session id gets a color from `color::SESSION`, eight hues by the session's arrival order modulo 8.
  Each reads on `SIDE` (contrast at least 4.5) and is at least 20° of hue from `FOCUS`, the trail colors, `CURSOR`, and the other seven.
  A row's session text draws in its session's color. The glows and the kind colors do not change: the glow says what happened, the session text says who.
  With more than one session, the head row's count is followed by a swatch per session in arrival order: a 6-point square in its color, at most eight, then `…`.
  The right side drops before the filter chips, in this order: `M newer above`, the count with its swatches, then the sort chip.

Notes (unit `053`), layer 2, from `minimap note` (unit `052`):

- A note event adds a note to `GuiApp::notes` (`minimap_core::notes`), a history row, and a glow in `FOCUS` (`#f2d15c`). A note is not on the trail.
  A note row shows the time, `note` in `FOCUS`, the path (at most half of the room after the kind), the lines, the text's first line in `INK`, and the session.
- At the block level, a noted card's mark is the card grown about its center to at least 8 points (`NOTE_MIN`), in `FOCUS` at 90 % over the pane, so it shows at the fit view.
- From the icon level, a noted card's fill is `FOCUS` at 30 % over the card (`NOTE_MIX`): the hover wins over it, and it wins over the trail's tint.
  A badge sits centered on the card's top-right corner: a `FOCUS` circle as wide as the name row (a pill for a 3-digit number),
  with the ordinal of the file's first note in `BAR`, bold, at 12 world points, as text in the world.
- A noted file's tree row has `▌` in `FOCUS` in its first cell, in place of the trail's mark.
- The marks draw above the cards and headers and below the glows. They never move a card and do not count for hits.
  `2` (the status bar's second toggle) shows or hides the marks and the note glows; the history keeps its note rows.
  A notes clear empties the notes, their marks, and their glows, and the history keeps its rows. `x` clears the trail and keeps the notes.

Note reader (unit `054`), the user's request of 2026-10-09: each note on its lines, one note at a time, so a review reads as a sequence of stops.

- The reader takes the map region in place of the map, as the TUI's viewer does; the tree, the panel, and the agent history stay. Esc returns to the map.
  `reader::build` turns the facts into draw items and the mode chips, and `reader::paint` draws them, so tests read the reader with no window.
- Head row (20 points, on `SIDE`, a 1-point `RULE` below): `NOTE 3 of 7` (`LABEL`, bold), the path (`INK2`, cut at the left) and `:40–58` (`DIM`),
  and at the right the chips `lines` and `whole file` (the active one `CURSOR` with white text, the other `DIM`, a hovered one `TILE_HI`). A click switches the mode.
- Rows of 18 points (`CODE_ROW`) with 13-point text: a 6-cell gutter with the line number (`FAINT`; `FOCUS` on noted lines), then the text.
  Syntax colors (unit `055`): a `reader::Colorer` thread colors the file with `minimap_core::highlight` and the config's `[viewer] theme` (else tokyonight-night),
  and its answer repaints the window. Until then, and for a file over 20,000 lines or 2 MiB, the text is `INK2`. Each colored span draws as its own text; bold spans use the bold face, and italic draws upright.
  Context lines stay `DIM`. A theme that does not load flashes its error in the top bar. In a release build, a file of several thousand lines colors in less than 0.1 s.
  Tabs run to the next multiple of 4 columns; lines do not wrap, and each line is cut at the region's right edge.
- The note box: a label row, `note 3 of 7` (`FOCUS`, bold) and `lines 40–58` or `whole file` (`DIM`), then the text in `INK`, wrapped to the box, with blank lines kept.
  The box runs from half a cell to 1 cell before the region's right edge, on `FOCUS` at 14 % over the pane. The noted lines below it are on `FOCUS` at 5 %.
  A 1-point `FOCUS` frame goes around the box and the noted lines.
- `lines` mode (the default): 2 context lines in `DIM` above, the box, the noted lines, and 2 context lines below.
- `whole file` mode (`z`, or the chip): every line, each of the file's notes in a box above its first line. Another note's frame is `FAINT` and its label `DIM`.
  On entry, the current box is a quarter of the body below its top. The mode stays for the next note.
- A note without lines shows the whole file in both modes, with its box above line 1. Noted lines past the file's end are clamped to it.
- Messages below the box, in place of the text: `No file text: the window shows an index JSON.`, `<path> is a binary file.`, `<path> is too large for the reader (N MiB).` (over 2 MiB),
  `<path> is empty.`, and a read error, such as for a file that the change deleted.
- `n` and `N` open the next or previous note after the last one read (from the map too; at the start, the first or the last note).
  At an end the top bar flashes `Last note.` or `First note.`; without notes, `No notes yet. An agent sends them with minimap note.`
- A click on a note row in the agent history whose file is not the selection selects the file as a trail row click does, and the reader does not open (unit `057`):
  the user sees the file on the map first. A click on a note row whose file is the selection opens its note, so a second click opens it,
  and a double-click opens it in place of the trail row's zoom. A note row on a path without a card (a deleted file) opens its note on the first click.
  After a notes clear, a note row clicks as a trail row. Opening a note selects its file as a tree click does:
  the tree opens to it, the panel shows it, and the map behind the reader moves only as far as needed to show the card, with no zoom change.
  A click that selects something else closes the reader: a tree row, another history row, an outline row, or a largest-file row. A notes clear closes it.
- Keys while it shows: Esc, `n` `N`, `z`, `j` `k` (one row), Page Down / Page Up (the body less 2 rows), `g` `G` and Home / End (the first or last row),
  and `f` or Enter, which close it and zoom the map to the note's first line on its card. `t` `p` `r` `1` `2` `7` `8` `x` `[` `]` work as on the map;
  the map keys and a pinch do nothing. The wheel scrolls 3 rows per notch, and trackpad points by the row height.
  The status bar's hint reads `n N notes · z lines / whole file · j k g G scroll · f map · esc close · drag select · ⌘c copy`.
- Text selection (unit `057`). The reader paints with the painter, so the selection is its own: a drag over the body (more than 4 points) selects from the press point
  to the pointer, by row and column, as in a terminal: whole rows between the ends, and on the end rows from or to the column under the pointer.
  A column is `x / char_w` from the row's text start, to the nearest cell edge, and clamped to the row's whole width. Rows are the code rows and the note box's wrapped text rows;
  the gutter, the label rows, and a message are not text, and a drag that starts in the gutter selects from column 0. A pointer above the rows is the first shown row's start; below them, the last shown row's end.
  The selection draws as `CURSOR` at 25 % over the row (`SELECT_MIX`), below the text and above the noted-lines tint: the first row from its column and whole rows to the box's right edge, the last row to its column.
  Cmd+C (Ctrl+C on other platforms) arrives as `egui::Event::Copy` and copies through `egui::Context::copy_text`: a code row gives its full line with its tabs, not the part on the screen, and no gutter, one line per row;
  the rows of a note box give the note's text lines, with their own spaces. A click without a drag clears the selection. Esc with a selection clears it; Esc without one closes the reader.
  Scrolling keeps it; a note change, a mode change, a reload of the file, and a new region width clear it, because the rows change. The wheel, the keys, and the mode chips work as before.
- The live index reads the reader's file again when it changed, and the scroll stays.
- Panel (unit `054`): a `NOTES · click to read` section for the selected file, after the trail line and before the shape: the count at the right,
  then one row per note, oldest first: the ordinal (`FOCUS`, bold), the lines (`DIM`; `file` without lines), and the text's first line (`INK2`).
  At most 4 rows, then `… K more · n next`. A click reads the note; the reader's note row is `CURSOR` with white text, and a hovered row is `TILE_HI`.
- In a release build, the reader builds a file of several thousand lines in `whole file` mode in well under 1 ms,
  and a whole frame with it takes less than 1 ms.

Live index (unit `048`), the TUI's (see `Stability principle` and `Architecture`):

- The window starts `minimap_core::watch::Watcher` with the options it indexed with; each update repaints it. `--from-json` has no watcher.
- An update rebuilds the map. The selection, the hover, and the open directories keep their entries by path; a removed selection clears.
  The camera moves as far as the selected card or block moved, so the selection keeps its place on the screen. The changed files compute their details again.
  When `HEAD` moved, the history walk starts again, and the old history shows until it ends. The trail keeps its marks by path.
- The top bar's right side flashes `N added · M removed` for 4 s. A rebuild of a repo of a few thousand files takes a few milliseconds.
  The git history walk (`history::Loader::spawn_with`) starts when the window opens, and its end repaints the window. `--from-json` takes the JSON's history.
- The wheel over the panel never moves the map, and a press on it starts no pan. The panel, its width, and hiding it never move a card;
  the map content keeps its place on the screen, as for the tree.
- World coordinates: one layout cell is 8 × 18 points at zoom 1, so a card is 168 points wide, and one line is `LINE_PT` (1) point high.
  A card is 18 points for the name row, plus its body rows of 18 points each. In a repo of a few thousand files, a card at the fit view is a few points wide.

Camera:

- At startup and on `0`: fit the whole map in the map region with a 24-point margin. The zoom runs from half the fit zoom to 6.
- A pinch, and Cmd with a scroll, zoom around the pointer. A mouse wheel (line units) zooms around the pointer by 1.15 per notch.
  A trackpad scroll (point units) pans; some mice send points, and their scroll pans too.
- A press that moves more than 4 points before its release pans the map with the pointer. A shorter press is a click.
- `+` or `=` zooms in by 1.5 around the region's center, `-` zooms out, and `w` `a` `s` `d` pan 10 % of the view (Shift: 40 %).
- Key zooms and fits ease out over 160 ms. Pinch and wheel zooms are immediate. The region's center stays inside the map.
- The window repaints only on input, during an ease, when details or events arrive, or while a glow runs. An idle window uses no CPU.

Detail levels, by the card's width `w` on the screen. Each level adds to the levels above it, and toggles remove elements at every level:

| Level | From `w` | Adds |
|---|---|---|
| block | 0 | A filled rect in the type color at 55 % over the pane, as tall as the card |
| icon | 20 | The card (`#161e27`, corner radius `0.04 × w`) and the icon in the type color |
| name | 40 | The file name in ink, cut with `…` (a bar below `w` 84, see text in the world) |
| shape | 60 | The shape, and the line count (digits only, dim) at the right of the name row |

- The outline left the cards in unit `039` (G1 review). The selection panel shows it (unit `044`).
- Text in the world (unit `040`): every text and icon has a size in world points, and its screen size is that size times the zoom, with no cap or floor.
  A text 7 points or larger on the screen draws as text. From 2 to 7 points it draws as a bar (greeking): a rounded rect as wide as the text
  and half as high as its size, in the text color at 35 %. Below 2 points it does not draw. An icon from 2 to 7 points is a square in its type color.
- Glyph sizes snap down to the ladder `7 × 1.1^n`, with no top end, so the glyph atlas holds a bounded set of sizes.
  The painter lays out each text at its ladder size and scales the text's mesh to its exact size, so text grows and shrinks smoothly during a pinch.
- Card text, in proportion to the card width, at its G1 sizes for width 168: the name row is 18 world points (one layout row), the icon 16, the name and the line count 14.
  A zoom never moves an element inside a card, and a name shows the same characters at every zoom. Card text does not fade: at zoom 6 a card name is 84 points.
- Headers: the directory name as the TUI writes it, at its world size from the layout, in its block's header band, at the band's left and vertical center,
  cut with `…` to the band's width. A 1-point rule in `#26313c` runs along the band's bottom, from the block's left to one cell before its right edge; the root has none.
  No name covers a card. Root and district names follow the same rules as other names.
- A directory name over 200 points on the screen fades out over the next 50 % (to 0 at 300 points), so a name never fills the window. The breadcrumb still names the place.
  After a zoom to a directory its name shows: a block grows about as fast as the square root of its count, as its name does. On quayside every name is 14 to 136 points then.
- At the fit view of a large repo, most directory names draw as bars, and a narrow district cannot name itself:
  a district a few points wide has no room for a 7-point name of 8 characters, which is 34 points wide.
- Card geometry, from the card width, so a zoom or a toggle never moves an element inside it: padding `p` is 6 % of the width at the sides.
  The name row is the card's first layout row (18 points at zoom 1): the icon, the name, and the line count, centered in it.
  The body runs from below the name row to the card's bottom, inside the side padding.
- Shape (unit `039`): `lines × LINE_PT` points high at zoom 1, from the body's top, across the full body width with 120 columns.
  `FileDetails::measured` keeps every line's indent and end column (4 bytes each). Each line is a bar from its indent to its end column,
  less a 1-point gap when the bar's row is 3 points or taller on the screen. When a line is under a pixel row, runs of lines share one sample,
  as `LineShape` samples them, so a shape draws at most one sample per pixel row. Only the rows in the map region draw.
  With `--from-json`, the details have only the 64-row shape, and each row draws `lines / rows` lines high.
  The fill is the type color at 70 % over the card. The rounding leaves at most a quarter of a body of 10 rows or more empty under the shape.
  A shape never draws past its card's body.
- Cut mark (unit `041`): a capped generated card shows its first lines at the normal scale, as many as fit 6 world points above the body's bottom.
  A dashed line of 12 dashes in `DIM`, 2 world points high (at least 1 screen point), runs along the body's bottom from the icon level up, also with the shape off.
  The name row's line count stays the real count.
  A binary or empty file has an empty body. While a file's details load, the body shows a rect in faint at 20 %, as tall as the shape will be, and above the cut mark.
- Details come from `minimap_core::details::Loader` (unit `036`): a pool of background threads that computes shapes and outlines,
  with grammar outlines for every language. When the set of visible cards at the shape level or above changes, the GUI asks for their details,
  nearest to the region's center first. With `--from-json`, the JSON's details answer at once.

Selection and hover (unit `038`). Selection and hover never move a card:

- The hovered card's fill is `#1e2a36`. A click on a card selects the file: a 2-point ring in `#5f86ad` and a white icon.
- A click on a header name, or anywhere in its header band, selects the directory: its block gets `#16222e`, and its name is white. A click on empty map keeps the selection.
- A double-click, or `f` for the selection, zooms to it over 240 ms: a card 420 points wide, centered horizontally, or a directory's block fitted with a 24-point margin.
  A card that fits the region's height is centered in it; a taller card has its top 24 points below the region's top.
- Esc clears the selection. The selection does not pulse.

| Key | Action |
|---|---|
| pinch, Cmd + scroll, wheel | Zoom around the pointer |
| trackpad scroll, drag | Pan |
| `+` `=` / `-` | Zoom in or out around the center |
| `w` `a` `s` `d` | Pan 10 % of the view (Shift: 40 %) |
| `0` | Fit the whole map |
| `7` / `8` | Names, shape on or off (`9` is unbound since unit `039`) |
| click, double-click, `f` | Select; zoom to the card or the directory |
| Esc | Clear the selection |
| `t` | Show or hide the file tree |
| `p` | Show or hide the selection panel (unit `044`) |
| `r` | Show or hide the agent history (unit `051`) |
| click, double-click an agent history row | Select the file and show it on the map; zoom to it (unit `051`) |
| `1` / `x` | Show or hide the agent's trail and its glows; clear them (units `047`, `049`) |
| `2` | Show or hide the note marks and the note glows (unit `053`) |
| `n` / `N` | Read the next or previous note in the reader (unit `054`) |
| click a note row in the agent history | Select its file; on the selected file, or a double-click, read the note (unit `057`) |
| click a note row in the panel | Read the note (unit `054`) |
| drag, Cmd+C in the reader | Select text; copy it (unit `057`) |
| `z`, click a mode chip | In the reader: `lines` or `whole file` mode |
| Esc, `f` ⏎ | In the reader: clear the selection, else back to the map; back to the map, zoomed to the note's line |
| click a filter chip in the agent history | Show only that kind's rows (unit `053`) |
| click the sort chip in the agent history | Newest first or oldest first (unit `057`) |
| click an outline row | Pick the item: zoom to its line on the card, and mark the line (unit `046`) |
| click, double-click a largest-file row | Select the file and show it on the map; zoom to it |
| `[` / `]` | Narrow or widen the file tree by 40 points |
| click, double-click on a tree row | Select it (a directory also opens or closes) and show it on the map; zoom to it |
| `j` / `k` | The next or previous tree row; with no selection, the first row (unit `043`) |
| `l` / `h` | Open a directory, then go to its first child; close it, else go to the parent |
| `g` / `G`, Home / End | The first or last tree row |
| Enter | Zoom to the selection, as `f` does |

The tree keys are the TUI's (`Keys`), where they do not clash with the GUI's. They work while the tree is hidden, and each key repeat moves one row.
Each move goes through the one selection, so the tree scrolls to the row, and the map follows with no zoom change.
`l` and `h` decide in `minimap_core::filetree` (`FileTree::step_in`, `step_out`), so both front ends move alike. The arrows stay unbound in the GUI.

Capture mode (`--capture DIR`), for visual checks, because herdr cannot read a window: a 1,600 × 1,000 window floats above other windows
and runs a fixed script: the fit view, card widths 16, 33, 40, 80, 130, 200, and 400 centered on `--select`, card width 200 with each toggle off,
400 with all on, and the zoom to the selection. The steps center on the top of a tall selected card, as the zoom to it does. Each step waits for its cards' details and saves `DIR/NN-<step>.png`.
Two zoom sweeps from the fit view to card width 400 over 120 frames then write the frame CPU times and the font atlas size to `DIR/frames.txt`.
The window then closes. It takes the focus, and it cannot draw while the screen is locked.

## Architecture

- `minimap-core`: index, layout, the working-tree watcher, and the event socket. No terminal dependencies.
- The event socket (`minimap_core::events`, unit `010`) takes JSON lines: `read`, `grep`, `edit` (with optional `lines`), and `clear`, each with `"v":1`.
  A trail event has an optional `source` and `session` (unit `050`). The parser reads only the fields it knows, so an older minimap ignores `session`.
  It is `$TMPDIR/minimap-$USER-<FNV-1a hash of the canonical root>-events.sock` (mode 0600), in `/tmp` when that path is longer than 103 bytes.
  Absolute paths under the root become repo-relative, and other paths drop out. One minimap per repo listens;
  a second one runs without a trail and flashes `Another minimap shows the trail for this repo.`
  `minimap send` sends one event and always exits 0 with no output, so an adapter never fails or slows the agent.
- Notes (unit `052`): `{"v":1,"type":"note","path":…,"lines":[a,b],"text":…}` with optional `lines`, `text` (at most 4,000 characters), `source`, and `session`,
  and `{"v":1,"type":"clear","layer":"notes"}`. The trail ignores both. `minimap_core::notes` keeps them in arrival order (at most 1,000) for every front end.
  `minimap note [--lines A-B] PATH [TEXT]` and `minimap note --clear` send them. It reports errors (exit 1: no window, or a path outside the repo), because an agent reads them.
  `minimap note --help` explains the guided-review recipe to an agent, and `minimap --help` points to it.
- The index lists the tracked files and the untracked files that git does not ignore, as `git ls-files --cached --others --exclude-standard` does (unit `034`).
  It marks generated files on every scan, from the lock-file names, the listed `.gitattributes` files, and the config (unit `041`, see `Layout`).
  The watcher (`minimap_core::watch`, `notify` with FSEvents on macOS) collects file events for 250 ms after the first one that counts, then re-indexes
  on its thread, reading only the files whose size or modification time changed. Events for gitignored or config-excluded paths do not count.
  Under `.git/`, only the git index, `HEAD`, and the refs count. The TUI checks the watcher every 100 ms and rebuilds the tree and the layout from a new index.
  When `HEAD` moves, the git history walk restarts in the background, and the old history shows until it ends.
- `minimap-core` also holds the data that both front ends share: the syntax colors (`highlight`, unit `055`), the file-type icons and colors (`filetype`), the palette (`palette`),
  the file tree's row model (`filetree`, unit `042`), the text fitting and number formats (`text`, unit `044`), the trail model (`trail`, unit `047`), the breadcrumb, grammar outlines (`syntax`, with `syntect` and `two-face`, which parse text and draw nothing), and the background details loader.
- `minimap-tui`: the Ratatui renderer and the `minimap` binary.
- `minimap-gui`: the egui renderer (eframe on wgpu) and the second binary, `minimap-gui` (Milestone G1).
- Claude Code hooks, the MCP tool, herdr, and the editors (nvim, vim, VS Code, Emacs, a custom command) are adapters. The map runs without them.
- An editor plugin can attach to the core later. The GUI is the first pixel renderer.

## Mockup history

- v5 (current): `mockups/v5-template.html` (local; `python3 mockups/build.py`, then open `mockups/minimap-v5.html`).
  File tree left, one-character map with directory names and a top-left overview in the middle, selection panel right.
- v4 (superseded): `mockups/v4-template.html` (local). The whole repo in the middle as braille (cheap highlight) or half-block pixels (one pixel per file, dense layout), with a ping animation.
  Dropped because a whole-repo view could not show names, and the ping distracted during tree navigation.
- v3 (superseded): source in `mockups/v3-template.html`. The tile map with names; built as Milestone 1.
- v2 (superseded): a kickoff mockup, not kept in the repo. It had three zoom levels and braille shapes on each tile.
- v1 (superseded): a kickoff mockup, not kept in the repo. It had a simulated Claude Code pane, an event log, and a rejected "Rows" layout.
