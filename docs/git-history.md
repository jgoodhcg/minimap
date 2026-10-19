# Git history before publication

The repository was squashed into one commit on 2026-10-19, before it was published.
This file keeps the 70 earlier commits, oldest first: the date, the old short hash, the subject, the body, and the model.
One person authored every commit, and Claude co-authored each one (`AI-Provider: Anthropic`, `AI-Product: claude`).
Some bodies were edited to remove measurements of a private test repo.
The old hashes do not resolve in this repository; some roadmap files cite them.

## 2026-09-25

### `85b9390` Adopt agent blueprint and draft roadmap

Added AGENTS.md, a CLAUDE.md wrapper, blueprint references, and public-repo ignore rules. Added the v3 design reference, the mockup sources, and a roadmap of 13 work units in six milestones. Moved the kickoff brief to the git-ignored .private/ after folding it in.

Model: `claude-opus-5-5[1m]`

### `43d2f2f` Validate units against the real target repo

Added real-repo tests gated on MINIMAP_TARGET_REPO, and pointed visual checks at the real repo in the current herdr workspace. Committed tests keep the fake repo so that no real-repo names enter tracked files.

Model: `claude-opus-5-5[1m]`

### `15acf78` Add Rust workspace and minimap binary

Added minimap-core and minimap-tui on Rust 1.98.1. The binary drew a bordered frame and restored the terminal on exit and on panic. A TestBackend snapshot test made validation level 4 runnable.

Model: `claude-opus-5-5[1m]`

### `8ef38d3` Add file index and minimap index command

Listed tracked files through gix with exclude globs, line counts, and binary and missing flags. A per-repo config added excludes, and the JSON dump matched the mockup data shape.

Model: `claude-opus-5-5[1m]`

### `5c016f9` Port the Districts layout from the v3 mockup

Ported size, plan, and place to minimap-core. A Node script exported golden layouts from the mockup code, and the port matched every quayside tile and header.

Model: `claude-opus-5-5[1m]`

### `900597b` Render the district map in the terminal

Drew headers, tiles, the district legend, and the breadcrumb bar with Ratatui. WASD, mouse drag, and wheel panned a clamped camera, and ? showed the help overlay.

Model: `claude-opus-5-5[1m]`

### `699521b` Plan Milestone 1b from the M1 review

Added units 014 to 017 for the M1 review feedback. Scroll-to-zoom was dropped, because the terminal font zoom already rescales the map.

Model: `claude-opus-5-5[1m]`

### `b819915` Move repo stats to the panel header

The right column header showed file, directory, and line counts beside the repo name. The mouse wheel no longer panned the map.

Model: `claude-opus-5-5[1m]`

### `e8c9423` Add file-type icons and colors

Tiles showed a Nerd Font icon in a vivid type color, and names took the same hue. The --no-icons flag restored the v3 text layout.

Model: `claude-opus-5-5[1m]`

### `8caa8d2` Add a hidden files toggle that moves no tile

Hidden files sorted last in each directory, in the core and in the mockup, and the . key hid them. Their slots stayed empty, so no visible tile moved.

Model: `claude-opus-5-5[1m]`

### `a8b1d3b` Add a collapsible file tree to the right column

The tree replaced the district legend and took focus with t. Moving its cursor centered the map on the entry, and opening rows never changed the layout.

Model: `claude-opus-5-5[1m]`

### `f9ee4bb` Plan the v5 map redesign

Added the v4 and v5 mockups and rewrote the design doc for the v5 screen. Added units 018 to 021 and dropped units 005 and 006, which they replace.

Model: `claude-opus-5-5[1m]`

### `345febe` Tidy the Milestone 2 unit files

Removed stray blank lines from the Context lists in units 007 to 009. Units 018 and 021 now say that the help overlay lists only the keys that work, and that docs/design.md keeps the full key design.

Model: `claude-opus-5-5[1m]`

### `758740f` Move the file tree left and make it the one selection

The screen got three columns: the file tree, the map, and a selection panel. The tree cursor became the only selection, and the tree focus mode went away. t hid the tree, [ and ] and an edge drag set its width, and y copied the selected path. A selection change opened the tree to it and scrolled the map only as far as needed.

Model: `claude-opus-5-5[1m]`

### `752a36a` Draw the map with one character per file

The layout took its constants as a LayoutConfig. TILES kept the v3 map, and CHARS gave each file one cell and each block room for its directory name. Both matched their mockup goldens tile for tile. The map drew file icons and directory names only, and the selection pulsed until m stopped it.

Model: `claude-opus-5-5[1m]`

### `e5aafb6` Add the overview to the map's top-left corner

The overview drew one braille dot per visible file and a viewport box. A click or a drag in it moved the view. The camera could pan every tile out from under it, and a selection never stayed under it.

Model: `claude-opus-5-5[1m]`

### `2056661` Search file paths with /, n, and N

A query of two or more characters selected the first matching path and marked all matches white on the map. File name matches came first. Enter kept the matches for n and N, and Esc cleared them.

Model: `claude-opus-5-5[1m]`

### `9cf5738` Plan Milestone 2b from the M2 review

Added units 022 to 024 from the M2 review and archived units 018 to 021. The design moved the repo stats to the selection panel, added the p, v, g, and G keys, and removed district colors. A new Color section put file types on the glyph in low chroma and state on the cell background. Unit 008 went back to draft, because its heat coloring predated that rule.

Model: `claude-opus-5-5[1m]`

### `28b4de8` Add panel and overview toggles and jumps to the tree's ends

p hid the selection panel and v hid the overview, and the map kept its place on screen. g and G selected the first and last tree rows. The repo stats moved to the top of the selection panel, and the map gained a row.

Model: `claude-opus-5-5[1m]`

### `1235652` Color only file types, on their icons

District colors went from the map, the tree, and the panel, with the core district module. Each file type got one low-chroma color, and only its icon carried it. Search matches moved from white glyphs to a green cell background, so a match kept its type color.

Model: `claude-opus-5-5[1m]`

### `ccb9195` Show a kept search in the status bar

After Enter, the status bar kept the query and the match count, so the marks on the map had a visible cause. Esc cleared both.

Model: `claude-opus-5-5[1m]`

### `59eed18` Plan Milestone 2c: fuzzy file search

Added units 025 and 026 from the request for a Telescope-style search, and archived units 022 to 024. The design gained a Search section: a picker in the left column, the map as its preview, and fzf ranking.

Model: `claude-opus-5-5[1m]`

### `060a6ae` Port telescope-fzf-native's fuzzy matcher to the core

minimap_core::fuzzy scored paths with the fzf v2 algorithm and fzf's extended syntax, as the C port in the user's nvim does. scripts/fzf-golden.py compared the port with the compiled C library: every score matched on the fake repo and on a real repo, and so did the positions of each query's top 50.

Model: `claude-opus-5-5[1m]`

### `aebb68c` Make / a fuzzy file picker in the left column

The picker replaced the file tree while open, with the best result next to the prompt. The highlighted result became the selection, so the map showed where it lived, and its visible rows were marked. Enter kept the result and the rows for n and N; Esc restored the earlier selection and camera.

Model: `claude-opus-5-5[1m]`

## 2026-09-28

### `41c625b` Revise Milestone 3 units against the v5 design

Units 007 to 009 dropped the v3 tile references. 007 took its outline rule from the user's nvim outline (def and reg- heads). 008 skipped merge commits and moved heat to the cell background in one magenta hue. 009 gained a reflow column. The design gained the Y and H keys and the heat color.

Model: `claude-opus-5-5[1m]`

### `06145af` Measure layout stability across a year of history

The stability example sampled first-parent commits, laid out each tree, and reported moved tiles, displacement, reflow, and moved headers for CHARS and TILES. On a real repo's history, block moves, not grid reflow, moved the tiles. The report recommended quantized block sizes.

Model: `claude-opus-5-5[1m]`

### `2d0acba` Show the file's shape, outline, and namespace in the panel

The core read the selected file on demand and cached its line shape, its tree-sitter outline, and its namespace. The panel drew the shape as braille with line numbers and definition markers, then the outline. o browsed the outline, and Y copied the namespace. Dependencies built optimized in dev builds, so parsing stayed within a frame.

Model: `claude-opus-5-5[1m]`

### `6c0837c` Load git history in the background and add heat layers

A background walk recorded each file's non-merge commit count, first and last change, and five recent commits. On a real repo it matched git rev-list. The panel showed a git group, H copied the newest commit hash, and 4 and 5 colored map cell backgrounds by age or churn.

Model: `claude-opus-5-5[1m]`

## 2026-09-29

### `d1ab2b4` Plan Milestone 3b from the M3 review

The review asked for stable blocks, a better shape for long files, a file viewer with diff mode, and editor handoff. Units 027-031 recorded the answers. Unit 010 recorded the trail in the overview, and unit 013 kept only the mine layer.

Model: `claude-opus-5-5[1m]`

### `8d558ce` Round block sizes so small growth moves no tile

Rounding file-grid capacities and depth 1-2 block heights cut a year of tile moves on a real repo by more than half, for a little more area. The stability example compared the variants and checked two older years. The mockup parity tests ran without rounding.

Model: `claude-opus-5-5[1m]`

### `4ceb9aa` Draw long files' shapes as a median band

Shape rows over 4 lines kept the median indent and length. On long files the lit-dot share fell by about half, and short files did not change.

Model: `claude-opus-5-5[1m]`

### `5f2aa87` View files in the map column with syntax colors

Enter showed the selected file read-only in place of the map, with tokyonight-night syntax colors and vim motions. The panel's shape marked the lines in view, and the outline followed the cursor. Esc returned to the map unchanged.

Model: `claude-opus-5-5[1m]`

### `883b021` Show a file's uncommitted changes in the viewer

The viewer diffed the file against HEAD on its background thread and marked changes in a sign column. = showed the removed lines inline, and ]c and [c stepped through the changes.

Model: `claude-opus-5-5[1m]`

### `e39d79f` Open files in the editor at the line with e

e opened the file in nvim in a reused herdr tab, through its --listen socket after the first open. Presets covered vim, VS Code, Emacs, and a custom command. Without herdr, terminal editors took minimap's terminal until they exited.

Model: `claude-opus-5-5[1m]`

### `3db6531` Outline every language from its grammar

Files without a core outline got one from their grammar's entity.name scopes, in the same parse as the colors. Calls, parameters, type annotations, and names after a dot did not count, and nested items were indented. On a real repo every file with a named definition got an outline.

Model: `claude-opus-5-5[1m]`

## 2026-09-30

### `1484d99` Plan Milestone 3c and revise Milestone 4

Unit 033 planned key overrides from the config, and unit 034 planned a live index that lists untracked files. Units 010 and 011 were revised for the v5 map and the review answers. The done units moved to roadmap/archived/.

Model: `claude-opus-5-5[1m]`

### `c2496c9` Let the config override every key

A keymap merged the defaults with the config's [keys.map], [keys.outline], and [keys.viewer] tables, in VS Code-style key strings with sequences. The help and the hints showed the bound keys, and minimap keys printed the keymap as config TOML. A bad binding stopped minimap before the terminal changed.

Model: `claude-opus-5-5[1m]`

## 2026-10-01

### `6bfb337` Follow the working tree with a live index

Unit 034 added a watcher that re-indexed the repo when files changed, and the index listed untracked files that git did not ignore. After a re-index, the selected file kept its place on the screen. Unit 033 moved to the archive.

Model: `claude-opus-5-5[1m]`

### `0ddf40c` Show the agent's trail from an event socket

Unit 010 added a JSON-line event socket to the core and a trail layer to the map, the file tree, the overview, and the panel. `minimap send` sent one event and always exited 0.

Model: `claude-opus-5-5[1m]`

### `2c0479e` Add the Claude Code hook command

Unit 011 added `minimap hook claude-code`, which read one hook payload on stdin and sent its trail events, and the hook settings in `adapters/claude-code/`. The unit stayed open, because the payload capture waited for the user to install the hook.

Model: `claude-opus-5-5[1m]`

### `2aba8a0` Plan Milestone G1, a GUI map prototype

Units 035 to 038 planned an egui window that zoomed from the whole repo to one file's card, with names, shapes, and outlines as toggles. The TUI stayed an option on the same core.

Model: `claude-opus-5-5[1m]`

### `c412ba2` Add a GUI window with a zoomable card map

Unit 035 added the minimap-gui crate: an egui window that drew the CARDS layout as file cards and zoomed from the whole repo to one card. It passed the egui go/no-go on a real repo. The file-type icons and the palette moved into minimap-core.

Units 034 and 010 moved to the archive, and the roadmap noted that Milestone G1 started before unit 011 was done.

Model: `claude-opus-5-5[1m]`

### `b99fa42` Compute file details for many files in the core

Unit 036 moved the grammar outline into minimap-core and added a background loader that computed shapes and outlines in priority order. The TUI fed the core's outline scan during its one color parse.

Model: `claude-opus-5-5[1m]`

### `69bb7c4` Show names, shapes, and outlines on the cards

Unit 037 added the card detail levels and the 7, 8, and 9 toggles. The GUI loaded the details of the visible cards in the background, nearest to the center first.

Model: `claude-opus-5-5[1m]`

### `08c972d` Select and zoom to cards in the GUI

Unit 038 added hover, selection, and zoom to a card or a directory, and moved the breadcrumb into minimap-core. The design doc recorded the GUI prototype.

Model: `claude-opus-5-5[1m]`

### `6e985fe` Plan Milestone G2 from the G1 review

The G1 review answered yes: a zoomable card map serves the user better than the TUI map. Units 039 and 040 planned cards as tall as their files' lines, with no outline, and names that grow and shrink with the zoom. The done G1 units moved to the archive.

Model: `claude-opus-5-5[1m]`

## 2026-10-02

### `3fda856` Make cards as tall as their files' lines

Unit 039 made each CARDS card a name row and a body as tall as its rounded line count, and files flowed down columns. The outline left the cards, and the shape drew one bar per line from the measured lines in the details. The stability example compared one card size with line-scaled cards on a real repo's history.

Model: `claude-opus-5-5[1m]`

### `402cb6c` Scale names with the zoom and size them by their blocks

Unit 040 gave each CARDS block a header band that holds a name sized by the block's width. Every GUI text got a world size, a glyph ladder with no top end, and greeking below 7 points. Names started at card width 40 and shapes at 60, and directory names faded from 200 points. The roadmap marked Milestone G2 done.

Model: `claude-opus-5-5[1m]`

### `8def382` Plan Milestone G3 from the G2 reaction

The user's first G2 reaction was positive. On 2026-10-02 the user decided how to mark and cap generated files, and picked the file tree next. Units 041 to 043 planned generated files, the GUI file tree, and the tree keys. The done G2 units moved to the archive.

Model: `claude-opus-5-5[1m]`

### `77ae470` Cap the cards of generated files

Unit 041 marked generated files by the lock-file names, linguist-generated in .gitattributes, and config rules, and capped their CARDS cards at the height of 300 lines. A capped card shows its first lines above a dashed cut mark. On a real repo the map got much shorter, and the summed card moves over a year fell by about half.

Model: `claude-opus-5-5[1m]`

### `91b10b1` Show the file tree in the GUI

Unit 042 moved the TUI's tree model into minimap-core and drew it in a left column of the window. The tree and the map share one selection: a tree click moves the map only as far as needed, and a map click opens the tree to the card. Hiding or resizing the tree keeps the map content in place on the screen.

Model: `claude-opus-5-5[1m]`

### `472d14b` Move through the GUI's tree with the TUI's keys

Unit 043 added j, k, l, h, g, G, Home, End, and Enter to the GUI. The decisions of l and h moved into minimap-core, and the TUI uses them too. The roadmap marked Milestone G3 done and reviewed.

Model: `claude-opus-5-5[1m]`

## 2026-10-05

### `142b0a5` Plan Milestone G4, the GUI selection panel

The user picked the right panel at the G3 exit. Units 044 to 046 planned the panel, its shape, and the outline clicks, and the plan decided that the panel shows at startup and that an outline click zooms to the line. The done G3 units moved to the archive.

Model: `claude-opus-5-5[1m]`

### `d877520` Show the selection panel in the GUI

Unit 044 drew the TUI's selection panel in a right column of the window: the repo stats, the file or directory facts, the outline, and the git history from a background walk. The text helpers moved into minimap-core for both front ends. Hiding or resizing the panel keeps the map content in place on the screen.

Model: `claude-opus-5-5[1m]`

### `078e168` Draw the file's shape in the GUI panel

Unit 045 added the TUI's SHAPE section to the panel: the whole file as bars, with line numbers and a tick at each definition.

Model: `claude-opus-5-5[1m]`

### `92bddf8` Zoom to an outline item's line on its card

Unit 046 made the panel's rows clickable: an outline item zooms the map to its line and marks the line on the card, the wheel scrolls the outline, and a largest-file row selects its file. The roadmap marked Milestone G4 done.

Model: `claude-opus-5-5[1m]`

### `94e6be4` Remove a private name from unit 034

Model: `claude-opus-5-5[1m]`

## 2026-10-07

### `b3155c4` Plan Milestone G5, the live GUI

The user asked to demo the TUI and the window beside a real Claude Code session on 2026-10-07. Units 047 and 048 planned the agent's trail and the live index in the window. The hook of unit 011 was installed at user level.

Model: `claude-opus-5-5[1m]`

### `dd978fc` Show the agent's trail in the GUI

Unit 047 moved the trail model into minimap-core and made the window listen on the repo's event socket. Cards tint by read, grep, or edit, the tree marks trail files, and the panel shows the trail line; 1 toggles the layer and x clears it.

Model: `claude-opus-5-5[1m]`

### `1e851ef` Follow the working tree in the GUI

Unit 048 started the watcher in the window and rebuilt the map on each update. The selection, the open directories, and the trail keep their entries by path, and the selected card keeps its place on the screen.

Model: `claude-opus-5-5[1m]`

### `4975e37` Plan Milestone G6, live agent activity

At the 11:41 try-out a real Claude Code session read 7 files, and the map showed no change: the read color is blue like the ClojureScript type color. The user asked for a glow that fades and an agent history panel. Units 049 to 051 plan the glow, session ids on events, and the agent history.

Model: `claude-opus-5-5[1m]`

### `3a73573` Glow each agent event on the GUI map

Unit 049 lights each event's card with a halo, a near-white core, and a ripple: full for 1 s, then fading out over 3 s. At the fit view the glow box is at least 36 points, and an off-screen card gets a dot at the region's edge. The window repaints only while a glow runs.

Model: `claude-opus-5-5[1m]`

### `4730f3e` Carry the agent session's id on events

Unit 050 adds an optional session field to the event line, still version 1, and the Claude Code hook sets it from the payload's session_id.

Model: `claude-opus-5-5[1m]`

### `43a84d8` List agent events in a GUI history strip

Unit 051 adds a strip below the map that lists every agent event of the window, newest first: time, kind, file, lines, and session. A click selects the file through the one selection and zooms to it; r hides the strip.

Model: `claude-opus-5-5[1m]`

### `1fd3de8` Keep the zoom on an agent history click

At the try-out the user asked that a click on an agent history row not change the zoom. A click now acts as a click on the file's tree row: it selects the file and moves the map only as far as needed to show the card. A double-click zooms to it.

Model: `claude-opus-5-5[1m]`

## 2026-10-13

### `47cc8de` Add agent notes for a guided review

Milestone G7 (units 052–055). A `note` event and `minimap note` let an agent point the person at a file or its lines with a text; `minimap note --clear` empties the notes. The GUI numbers the noted files on the map at every zoom, filters the agent history to notes, lists a file's notes in the panel, and steps through them with `n` and `N` in a reader that shows each note on its lines, or the whole file with `z`. The syntax highlighter moves from the TUI into the core so the reader shares its colors.

Model: `claude-fable-5-1, claude-opus-5-5[1m]`

### `11f9296` Apply a repo's config to its worktrees

Unit 056. A [repos."<path>"] key now matches the main working tree of the repository that the mapped root belongs to, so one entry covers a checkout and every git worktree of it. The main tree is the parent of gix's common dir, canonicalized first because a linked worktree's common dir ends in `/../..`.

Model: `claude-fable-5-1`

## 2026-10-16

### `57a47ed` Record the first usage observations in the roadmap

Add a dated subsection under Quick Ideas with fourteen observations from the user's first sessions on a real repo, grouped by map, selection, notes, text, instances and sessions, and lower priority.

Model: `claude-fable-5-1`

### `56625ca` Plan Milestone G9: five usage quick fixes as unit 057

Pull the five usage observations that need no decision into unit 057: N in the map hint, a note row click that selects first and opens second, a sort chip for the history, a color per session, and text selection with copy in the reader. Add the split-view observation.

Model: `claude-fable-5-1`

### `d865071` Ship the first usage quick fixes in the GUI

Unit 057. A note row click in the agent history selects its file first, and a second click or a double-click reads the note. A chip sorts the history newest or oldest first, each session has a color with swatches in the head row, the map hint shows `n N notes`, and a drag in the reader selects text that Cmd+C copies as full lines.

Model: `claude-opus-5-5`
