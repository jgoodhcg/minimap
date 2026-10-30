---
title: "minimap Roadmap"
goal: "Give the user a stable spatial map of a codebase that shows where a coding agent works, so the user keeps a mental model of code they did not type."
---

# Roadmap

## Current Focus

Milestone G1, a GUI map prototype in egui (units `035`–`038`), is `done` and reviewed (2026-10-01). The G1 answer is yes (see below).
Milestone G2 (units `039`, `040`) is `done` (2026-10-02): cards as tall as their files, and names that scale with the zoom.
The user's first reaction (2026-10-02): "This is really nice. I enjoy zooming around a lot." The full G2 review is open, and joins the G3 exit review.
Milestone G3 (units `041`–`043`) is `done` (2026-10-02): generated files have a capped card height, and the file tree is in the window with the TUI's tree keys.
No window opened during the stint; the captures ran at the exit review, after the user agreed. Review on 2026-10-02: "Looks good."
Milestone G4 (units `044`–`046`) is `done` (2026-10-02): the user picked "the right panel" next, a GUI selection panel.
The panel shows the selection's facts, shape, outline, and git history; it shows at startup, as the TUI's does, and `p` hides it.
A click on an outline item zooms to its line on the card and marks the line. No window opened during the stint; the captures ran at the exit review (2026-10-05), after the user agreed.
The open G2 findings below were not discussed.
Milestone G5 (units `047`, `048`) is `done` (2026-10-07): the agent's trail and the live index in the window, for a demo at 14:00 the same day.
The user runs one front end at a time. Unit `011` (the hook) was installed at user level on 2026-10-07 and works on the TUI; its Grep payloads are not captured yet.
At the try-out (11:41) the window received a real session's reads, but the tint did not show them: the user asked for a fading glow and an agent history panel.
Milestone G6 (units `049`–`051`) is `done` (2026-10-07, 12:15): a glow on each event's card that fades out, session ids on events, and the agent history below the map (`r`).
No window opened during the stint: the user's try-out with a real Claude Code session is the visual check. The hook shows sessions after `cargo install --path crates/tui --locked`.
Milestone G7 (units `052`–`055`) is `done` (2026-10-09): notes for a guided review. The user asked for an agent to guide a pull request review on the map:
a note command and event, a history filter, notes in the panel, and a reader that shows each note around its lines. The note command replaces unit `012`'s MCP plan.
No window opened during the stint; the GUI visual check waits for the exit review and the user's consent.
Milestone G7 was committed on 2026-10-13 and both binaries installed; the user's first worktree run showed that `[repos]` keys miss worktrees, so Milestone G8 (unit `056`) makes a key cover them.
Milestone G9 (unit `057`) ships the five usage observations (2026-10-14 to 2026-10-16) that need no decision; the other nine wait in `Quick Ideas` for the user.
Unit `057` is `done` (2026-10-16). No window opened during the stint; the GUI visual check waits for the exit review and the user's consent.
Unit `058` (2026-10-19) prepares publication: the fake `quayside` repo is a Polylith workspace of its own, tracked files keep no facts about the private target repo,
and the repo starts over as one commit; `docs/git-history.md` keeps the earlier commit messages.
Unit `059` (active, 2026-10-28) finishes the G9 exit and publishes the repo: the user skipped the GUI visual check, the two G9 choices moved to `Quick Ideas`, and `README.md` and an MIT `LICENSE` are added.

Open G2 findings for the review (units `039`, `040`; numbers after unit `041`):

- Line-scaled cards move more than cards of one size, also with the generated cap. Mostly whole districts shift by one column.
  Rounded widths would cut the moves only a little; not adopted.
- At the fit view of a real repo, few districts read as text and some are bars; the rest draw nothing: a narrow district cannot hold a readable name in its band.
- Empty space: files fill only a small share of the map, and the name bands take a noticeable share of it.
- A card name shows the same 10 to 13 characters at every zoom.

Milestone 4 is open: units `034` (live index) and `010` (event socket and trail) are `done` (2026-09-30). Unit `011` stays `ready`.
Unit `011` stops once for the user to install the binary and the hook.
The heat color (`#e0529c`) stays until the user has looked at layers `4` and `5`.

## Direction

Recorded after a demo of `minimap-gui` on 2026-10-29. `docs/design.md` `Purpose` stays the settled statement; this section adds the intent behind it.

- Purpose: agent tools pull a user toward cognitive surrender.
  The agent does the work, the user stops reading the implementation, and skills and the mental model of the codebase atrophy.
  `minimap` is one tool for intentional expertise-building: it keeps the human in the loop with spatial signals instead of text.
- The problem: an agent chat stream drops the visual signals that the visual cortex reads fast.
  Syntax colors, the file tree, the outline, and semantic colors from the LSP are gone.
  The chat stream is also a poor place to follow along: the user scrolls back to find an old message, and the agent's reasoning is separate from the code it discusses.
- Position: an accessory beside any agent harness, not a feature of one harness.
  It connects through standard interfaces (hooks, the `minimap` CLI, MCP), and Claude Code and herdr stay adapters.
  A viewer at the demo said the tool is compelling only when it is next to or inline with the harness, because switching between windows is not.
- Long-term direction: one surface where the user reads the code and the agent's work, and types input that `minimap` forwards to the harness.
  The map becomes the interface the user looks at most, on top of any agent.

## Milestones

Each milestone ends with something the user can look at and judge.
At each exit, stop and ask the user for review before the next milestone.

| # | Milestone | Units | Exit: the user can… |
|---|---|---|---|
| 1 | Map with names | `001` `002` `003` `004` | Run `minimap` on the real target repo and pan a stable district map of every file name. |
| 1b | M1 review changes | `014` `015` `016` `017` | See repo stats by the repo name, tell file types apart by icon and color, hide dotfiles without the map moving, and find files in a collapsible tree that moves the map. |
| 2 | v5 map | `018` `019` `020` `021` | Run `minimap` on the real target repo: move through a toggleable file tree on the left, see the one-character map with directory names and a corner overview, read the selection on the right, and search paths. |
| 2c | Fuzzy search | `025` `026` | Press `/`, type part of a name, and pick from an fzf-ranked list while the map shows where each result lives. |
| 2b | M2 review changes | `022` `023` `024` | Hide and show the selection panel and the overview, jump to the tree's ends, read the repo stats atop the panel, tell file types apart by icon color alone, and see a kept search in the status bar. |
| 3 | File info | `007` `008` | Read the selected file's shape, outline, namespace, and git history in the selection panel. Toggle age and churn heat. |
| 3b | M3 review changes | `027` `028` `029` `030` `031` `032` | See tiles stay put as the repo grows, read long files' shapes, press Enter to read a file or its uncommitted diff in minimap with syntax colors and vim motions, press `e` to open it at the line in nvim, and see an outline for every language. |
| 3c | Keymap | `033` | Bind any key or key sequence to any action in the config, and see the bound keys in the help and the hints. |
| 4 | CC trail | `034` `010` `011` | Watch Claude Code's reads, searches, and edits appear on the map as they happen, including files it creates. |
| 5 | CC focus | `012` | Ask Claude Code to explain a concept and see its files numbered and annotated on the map. |
| 6 | Mine layer | `013` | See "mine" marks on the files opened this session. |
| G1 | GUI map prototype | `035` `036` `037` `038` | Run `minimap-gui` on the real target repo in its own window, zoom smoothly from the whole repo to one file's outline, and turn file names, shapes, and outlines on and off. |
| G2 | Card map v2 | `039` `040` | Run `minimap-gui` on the real target repo: see every card as tall as its file's lines, with one line height on every card and no outline, and read directory names at every zoom as they grow and shrink with it. |
| G3 | Generated files and the GUI tree | `041` `042` `043` | Run `minimap-gui` on the real target repo: see lock files as short cards with a cut mark, and find files in a collapsible tree beside the map that shares the selection with it and moves the map to it. |
| G4 | GUI selection panel | `044` `045` `046` | Run `minimap-gui` on the real target repo: read the selection's facts, shape, outline, and git history in a right panel that `p` hides, and click an outline item to see its line on the card. |
| G5 | Live GUI | `047` `048` | Run `minimap-gui` on the real target repo beside a Claude Code session: see its reads, searches, and edits tint the cards, and see new files appear. |
| G6 | Live agent activity | `049` `050` `051` | Run `minimap-gui` on the real target repo beside a Claude Code session: see each read, search, and edit glow on the map and fade, and click an event in the agent history to select its file and zoom to it. |
| G7 | Agent notes | `052` `053` `054` `055` | Run `minimap-gui` on the real target repo beside a Claude Code session that guides a review with `minimap note`: see the noted files numbered on the map at every zoom, filter the agent history to notes, see a file's notes in the panel, and step through the notes with `n` and `N` in a reader that shows each note on its lines with syntax colors, or in the whole file with `z`. |
| G8 | Worktree config | `056` | Run `minimap-gui` in a worktree of the real target repo and see the main checkout's excludes apply without a new config entry. |
| G9 | Usage quick fixes | `057` | Run `minimap-gui` on the real target repo beside a Claude Code session: click a note row to see its file on the map and click again to read it, sort the history oldest first, tell sessions apart by color, see `N` in the hint, and drag over the reader's text and copy it. |

Every unit validates against the real target repo (`AGENTS.md` `Real-repo tests`). Committed tests use the fake `quayside` repo.

Milestones 1–3 need only the repo on disk.
Units `012` and `013` predate the v5 map; revise their map rendering specs before marking them `ready`. Units `010` and `011` were revised on 2026-09-29.
Milestones 4–6 add live inputs, and each works without the others.
The order of 4–6 can change; 4 comes first because the trail is the core of the idea.

Unit `009` (layout stability measurement) needs only `003`.
Run it after Milestone 1 or between later milestones.

The M1 question: does the user enjoy looking at their code this way?
If the answer is no, stop and redesign before Milestone 2.
Answer on 2026-09-25: yes. The user likes WASD, drag, and `?`, and asked for the Milestone 1b changes. Scroll-to-zoom was considered and dropped.

The G1 question: does a zoomable card map serve the user better than the TUI map?
If yes, plan a G2 milestone at the review: bring the file tree, the viewer, the selection panel, and the trail into the window, or link the window to a running TUI.
If no, drop the GUI and keep the TUI.
Answer on 2026-10-01: yes. Zooming on pixels is more interesting than the TUI map. The review decided:

- The outline leaves the cards. It belongs in a GUI selection panel, which comes after G2 with the file tree, the viewer, and the trail.
- A card's height is proportional to its line count, and one line has the same height on every card. The card width stays locked.
  This reverses "every tile has one size" for `CARDS`; rounding and a stability report (unit `039`) keep the cost in view.
- Directory names must show at zoom 0.199 (card width 33). Text grows and shrinks with the zoom (unit `040`).
- Empty space on the map waits until the other changes are in.
- `CARDS` places differ from the TUI map's. The agent raised this again, and the user did not object. G2 makes the two maps differ more.
- The user left the methods to the agent: failures in the work or at the G2 review show whether they meet the standard.

## Work Units

See individual `[ID]-[slug].md` files in this directory, and `archived/` for `done` and `dropped` units. Use `draft` during clarification and `ready` when autonomous execution can begin.

| ID | Unit | Milestone | Status |
|---|---|---|---|
| `001` | Rust workspace and toolchain | 1 | done |
| `002` | File index | 1 | done |
| `003` | Districts layout | 1 | done |
| `004` | Map view | 1 | done |
| `005` | Overview | 2 | dropped (replaced by `020`) |
| `006` | Selection | 3 | dropped (replaced by `018`) |
| `007` | Shape and outline | 3 | done |
| `008` | Git history and heat layers | 3 | done |
| `009` | Layout stability measurement | any, after `003` | done |
| `010` | Event socket and trail layer | 4 | done |
| `011` | Claude Code hooks adapter | 4 | ready |
| `012` | Focus MCP tool and focus layer | 5 | draft |
| `013` | Mine layer (editor handoff moved to `031`) | 6 | draft |
| `014` | Panel header stats and no wheel panning | 1b | done |
| `015` | File-type icons and colors | 1b | done |
| `016` | Hidden files toggle | 1b | done |
| `017` | File tree panel | 1b | done |
| `018` | Three-column screen and one selection | 2 | done |
| `019` | Character map | 2 | done |
| `020` | Overview overlay | 2 | done |
| `021` | Path search | 2 | done |
| `022` | View toggles, tree ends, and stats in the panel | 2b | done |
| `023` | File-type colors only | 2b | done |
| `024` | Kept search in the status bar | 2b | done |
| `025` | Fuzzy matcher | 2c | done |
| `026` | File picker | 2c | done |
| `027` | Quantized block sizes | 3b | done |
| `028` | Shape median band | 3b | done |
| `029` | File viewer | 3b | done |
| `030` | Diff mode | 3b | done |
| `031` | Editor handoff with presets | 3b | done |
| `032` | Outlines for every language | 3b | done |
| `033` | Keymap overrides | 3c | done |
| `034` | Live index | 4 | done |
| `035` | GUI window and zoomable map | G1 | done |
| `036` | Details for every file in core | G1 | done |
| `037` | Card detail levels and toggles | G1 | done |
| `038` | Hover, selection, and zoom to a card | G1 | done |
| `039` | Line-scaled cards | G2 | done |
| `040` | Names that scale with the zoom | G2 | done |
| `041` | Generated files | G3 | done |
| `042` | GUI file tree | G3 | done |
| `043` | GUI tree keys | G3 | done |
| `044` | GUI selection panel | G4 | done |
| `045` | GUI panel shape | G4 | done |
| `046` | GUI outline clicks | G4 | done |
| `047` | GUI trail layer | G5 | done |
| `048` | GUI live index | G5 | done |
| `049` | GUI glow | G6 | done |
| `050` | Session ids on events | G6 | done |
| `051` | GUI agent history | G6 | done |
| `052` | Note events and `minimap note` | G7 | done |
| `053` | GUI notes on the map and a history filter | G7 | done |
| `054` | GUI note reader | G7 | done |
| `055` | Syntax colors in the GUI reader | G7 | done |
| `056` | Repo config applies to worktrees | G8 | done |
| `057` | Usage quick fixes: note clicks, history sort, session colors, text copy | G9 | done |
| `058` | Publication prep: a fake repo of its own and a one-commit history | — | done |
| `059` | Finish the G9 exit and publish the repo | G9 | active |

Real target repo details (path, exclusions) are in `.private/targets.md`. That file is git-ignored.

## Quick Ideas

Ideas not yet promoted to work units:

- Allowlisted ignored files (layer 6). The layout already reserves their slots; the index and config do not produce them yet.
- An nvim adapter: a `BufEnter` autocmd sends "mine" events, so files opened outside `minimap` count.
- Diffs of commits from the GIT group, and a hunks-only diff view (unit `030` shows uncommitted changes inline only).
- Annotated diffs (the user, 2026-10-30): an agent puts notes on a diff, and the reader shows each note at its hunk, with syntax colors for code inside the note text.
  Today a note (unit `052`) points at working-tree lines only, so a removed line has no anchor. The GUI reader (units `054`, `055`) has no diff mode and draws note text in plain `INK`.
- An index and git cache keyed by `HEAD`, if startup is slow.
- Heat-layer polish (ramps, legends).
- An nvim plugin as another front end on `minimap-core`. The desktop GUI became Milestone G1.
- Other visualizations beside the map: ERDs, sequence diagrams, and flow charts, drawn from code or sent by the agent (for example through the focus MCP tool).
- A tighter `CHARS` packing: files fill only about 8 % of the quayside map area.
- Less empty space on the `CARDS` map: shelf holes, the Growth bands, and a camera that can show far past the map's edge (G1 review, deferred). Unit `039` measures the fill.
- Follow mode: the selection follows the agent's newest event.
- A git base layer for a review: mark the files that differ from a base branch (`--base main`), so a pull request shows on the map without notes.
- Open any file in the GUI reader, not only a note's file.
- The newest trail event in the top bar, as an event log of one line.
- A git status layer: modified and untracked files on the map (unit `034` lists untracked files).
- Fluency-illusion check: does the map build real understanding? Deferred until the user knows whether they enjoy using it.

### Usage observations (2026-10-14 to 2026-10-16)

From the user's first sessions with `minimap-gui` on a real repo. Each item is a candidate for a work unit.
Five items that needed no decision became unit `057` (Milestone G9): `N` in the map hint, a note row click that selects first and opens second,
a sort toggle for the history, a color per session, and text selection with copy in the reader. The items below wait for the user's decisions.

Map:

- Heading size carries no meaning. A child directory such as `src/` can draw bigger than its parent. Size should encode one of: the child count with subdirectories, the depth in the tree, or the edit activity. The goal is waymarkers that help the visual cortex remember the codebase.
  Decided (the user, 2026-10-30): the count of all files below the directory, at every depth, not only its direct children.
  Today the size uses no count: it is the largest at which the name spans 30 % of its block's width (unit `040`, `LayoutConfig::name_band`), so a short name draws bigger.
  The header band's rows follow the name size, so the change moves tiles and needs a test that proves which tiles move.
- Files with notes or highlights need a stronger mark at the fit view. Options: a glow on the marked cards, or a dimmer map around them.
- An optional split: the map at the left and the reader at the right, with the tree and the panel in place, for a wide screen. On a laptop screen the reader keeps the map region. The split also moves the reader's wrap or cut point.
- Highlight every file that a branch or a commit changed. This extends the git base layer idea above with commits as a second source.

Selection:

- Multi-select: Shift+click in the tree and in the agent history selects several files. The map and the tree show them as they show the focus. The panel shows collective stats for the selection.
- A persistent multi-highlight from the agent, for example "the files relevant to this discussion". The marks stay until a clear.

Notes:

- Define the rule for a second note on the same file, and the rule for notes with overlapping lines.
- Decide whether notes render markdown.

Text:

- Text in the interface cannot be selected or copied. Unit `057` covers the reader and the note text. The ideal is every text in the interface.

Agent history (open choices from unit `057`, moved here on 2026-10-29):

- Oldest first, a strip that is scrolled to its end does not follow new rows. A follow at the end, as `tail -f` does, is a candidate.
- A row without a session draws `—` in `DIM` since unit `057`; it was `FAINT`. Going back to `FAINT` is a candidate.

Instances and sessions:

- Several `minimap-gui` instances on different repos, each color coded so a window reads at a glance. The alternative is tabs in one window, one repo per tab.

Lower priority:

- Show the outline at every zoom level. A close enough zoom shows the file text with syntax colors, as the reader shows it. This makes the reader a zoom level rather than a view.

### Demo feedback (2026-10-29)

From a demo of `minimap-gui` to another developer. Each item is a candidate for a work unit or a milestone.

Beyond files:

- Represent things other than files: application concepts and how they connect. A file map does not show concepts.
- Dynamic entity relationship diagrams that update as the code changes. This extends the ERD idea above.
- More LSP integration: show a symbol's callers, and a dependency graph of imports and exports.
- A second map that groups by connection (imports, calls) instead of the file tree.
  It cannot replace the `CARDS` map: the layout must stay a deterministic function of the file tree (`AGENTS.md`).

Harness:

- Run inline with the harness or beside it with no window switch. A desktop agent app with an MCP integration can be an easier host than a terminal harness.
- Input from the map: the user types in `minimap`, and `minimap` forwards the input to the harness. See `Direction`.

Timeline:

- A time-centric view of a session, after the timeline in video and audio editors.
  Agent events, reasoning, and notes sit on a track that the user scrubs and steps through, instead of a chat stream they scroll back through.
- Open question: what is on the time axis, and how does it map to the other signals: icons, color, position on the map, the file tree, and LSP entities?
  The agent history (unit `051`) and the glow (unit `049`) are a start: the history is a list, not a track.
