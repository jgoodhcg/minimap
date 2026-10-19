# AGENTS

Follows `AGENT_BLUEPRINT.md` (version: 2026-09-14)

## Session Start

- Herdr tab naming `[BP-WF-HERDR]`: before other task work, when `HERDR_ENV=1` and `HERDR_TAB_ID` is set, run once `herdr tab rename "$HERDR_TAB_ID" "<label>"` (`<label>` = one relevant emoji + at most five words naming the primary task); on failure, continue silently without retry.

## Project Overview

`minimap` is a terminal UI that runs beside a coding agent.
It draws the codebase as a stable 2D map
and shows where the agent reads, searches, edits, and explains.
The goal is to help the user understand and remember a codebase that an agent writes much of.
It is written in Rust with Ratatui.
The design reference is `docs/design.md`.

## Repository Visibility

- Visibility: public (default; no remote is configured yet)
- Before staging or committing, apply `AGENT_BLUEPRINT.md` `[BP-PUBLIC]`.

## Stack

- Rust 1.98.1, stable toolchain (pinned in `rust-toolchain.toml`)
- Ratatui + crossterm for the terminal renderer
- egui + eframe (wgpu renderer) for the GUI prototype, the second binary `minimap-gui` (Milestone G1)
- tree-sitter for outlines, gix for git history
- No database and no deploy target. The products are two local binaries, `minimap` and `minimap-gui`.

## Environment

- Version manager: rustup (installs to `~/.cargo`; when `cargo` is not on `PATH`, run `source "$HOME/.cargo/env"` first)
- Version file: `rust-toolchain.toml`
- Lockfile: `Cargo.lock` (commit it)
- Setup: `cargo build --locked`
- Mockup: `python3 mockups/build.py`, then open `mockups/minimap-v5.html` in a browser.

## Commit Trailer Template

Store a template, not concrete runtime values (`BP-CORE-09`).

```text
Co-authored-by: [AI_PRODUCT_NAME] <[AI_PRODUCT_EMAIL]>
AI-Provider: [AI_PROVIDER]
AI-Product: [AI_PRODUCT_LINE]
AI-Model: [AI_MODEL]
```

Write the trailer lines consecutively. A blank line between trailers stops `git interpret-trailers` from parsing the lines above it.

- `AI_PRODUCT_LINE` ∈ `codex | claude | gemini | opencode`, derived from the current tool.
- `AI_PROVIDER` / `AI_MODEL`: runtime-derived at commit time (prefer session metadata > launch overrides > tool config > UI label).
- `AI_MODEL` is the exact runtime model ID (for example, `gpt-5.6-sol`), never a family label such as `GPT-5`.
- `AI_PRODUCT_NAME` / `AI_PRODUCT_EMAIL`: resolved from the model name via the tiered lookup in `references/commit-attribution.md`.
- Fill at commit time; never persist filled values here. Multi-model commits: see `references/commit-attribution.md`.

## Validation Commands

| Level | Command | When |
|-------|---------|------|
| 1 | `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` | After every change |
| 2 | `cargo build --workspace --locked` | After code changes |
| 3 | `cargo test --workspace --locked`, then the real-repo tests below | Before completing work |
| 4 | `cargo test -p minimap-tui --test render` (Ratatui `TestBackend` snapshots via `insta`), then the visual check below | After UI changes |
| 4b | `cargo test -p minimap-gui`, then the GUI visual check below | After GUI changes |

### Real-repo tests

Validate every unit against the real target repo. The fake repo alone does not count.
`minimap` only reads a target repo, and `Never Run` forbids writes to it.

- Write real-repo tests as `#[ignore = "needs MINIMAP_TARGET_REPO"]` tests that read `MINIMAP_TARGET_REPO` and `MINIMAP_CONFIG`.
- Run them with `MINIMAP_TARGET_REPO=<path> MINIMAP_CONFIG=.private/minimap.toml cargo test --workspace --locked -- --ignored`. The path is in `.private/targets.md`.
- Real-repo tests assert invariants. They write golden data and captures only under `.private/`.
- Committed tests and snapshots use the `quayside` fake repo, because real-repo names must not enter tracked files.

### Visual check

When a work unit asks for a visual check, run the real binary on the real target repo in a background herdr tab and read the screen:

1. Read `workspace_id` from `herdr pane current`. Then run `herdr tab create --workspace <workspace_id> --no-focus --cwd "$PWD" --label "minimap check"`. Read the new pane ID from the JSON output.
2. `herdr pane run <pane> "cargo run -q -p minimap-tui -- --config .private/minimap.toml <target repo path>"`.
3. `herdr pane send-keys <pane> <key>...` to drive it.
4. `herdr pane read <pane> --source visible` (add `--format ansi` to check colors).
5. `herdr tab close <tab>` when done.

When herdr is unavailable, report the visual check as skipped.

GUI visual check: herdr cannot read a window, so `minimap-gui` captures itself.

1. Run `cargo run --release -p minimap-gui -- --config .private/minimap.toml --capture .private/captures/<unit> [--select <file>] <target repo path>`.
   The window opens on the user's screen, floats above other windows, and closes by itself. It takes the focus.
2. Read the PNGs in `.private/captures/<unit>/` with the image reader. Crop details with `sips -c <h> <w> --cropOffset <y> <x>`.
3. Read `frames.txt`: the frame CPU times and the font atlas size of two zoom sweeps.

Report when the capture ran, because it opened a window on the user's screen.

## Execution Modes

See `AGENT_BLUEPRINT.md` `Execution Modes` for the full policy.

- `roadmap/` is the canonical planning surface; the referenced work unit is the source of scope.
- Work-unit IDs use 3-digit zero-padded prefixes: `[ID]-[slug].md`.
- Apply the validation commands above when their triggers fire.
- Keep changes minimal and scoped to the requested work unit.
- The user works hands-off. Execute the `ready` units of the current milestone in order, without stopping between units. At a milestone exit, stop for user review.
- When work needs a system-wide install (toolchains, Homebrew packages, global `npm` or `cargo install`), print the exact commands for the user and wait. Do not run them.
- Crate dependencies that the active work unit names can be added without confirmation. Other new crates need confirmation.
- Require user confirmation before `git commit` and network side effects. At a milestone exit, propose one commit per completed unit.
- Before using a vendor-hosted feature (for example, Claude Artifacts), confirm with the user unless they asked for it. Apply `AGENT_BLUEPRINT.md` `[BP-VENDOR]`.
- Stopping for clarification when scope is ambiguous is acceptable.

## Never Run

- `cargo publish` — the crates.io name `minimap` belongs to another project, and a publish cannot be undone. The user decides any publish and its name (`minimap-tui`).
- Commands that write to a target repository, such as `git -C <target> commit` or files written inside it — `minimap` reads target repos and never changes them.

## Project-Specific Rules

- Keep `minimap-core` free of terminal and renderer dependencies. The core owns the index, the layout, and the event socket. A pixel renderer must be able to attach later.
- Make the layout a deterministic function of the file tree and, for `CARDS`, the files' line counts. Selection, layer toggles, and the ignored-file toggle must never move a tile. When a change can move tiles, add a test that proves which tiles move.
- Use the in-tool terms from `docs/design.md` `Vocabulary`: map, overview, shape. Do not call a view a "minimap"; that is the tool's name.
- Keep Claude Code and herdr as adapters. The map must run and be useful with neither present.
- The real target repo is private. Its path, name, and exclusions are in `.private/targets.md`. Do not write its name, internal paths, structure, contents, or measurements into tracked files.
- When output comes from a real target repo, write it under `.private/`. This includes index dumps, screen captures, stability reports, and fixtures.
- `BP-WRITE-04` exemptions: none.

## References

- For operating rules, see `AGENT_BLUEPRINT.md`.
- For work units and execution prompts, see `roadmap/`.
- For the settled v5 design (layout, regions, layers, keys), see `docs/design.md`.
- For decision records, see `AGENT_BLUEPRINT.md` `[BP-DECISIONS]`.

## Key Files

- `docs/design.md` — settled v5 design decisions and mockup constants
- `mockups/v5-template.html` — v5 mockup source; the reference implementation for the screen, the map, and the keys
- `mockups/v3-template.html` — v3 mockup source; the reference for the packing algorithm with the v3 tile constants
- `mockups/fakegen.py` — deterministic fake Clojure monorepo generator (`quayside`, a Polylith workspace of 2,585 files)
- `mockups/build.py` — builds `mockups/minimap-v3.html`, `-v4.html`, and `-v5.html` from the templates and fake data

## User Profile (optional)

See `.agent-profile.md` (git-ignored) for interaction preferences.

## Response Style

Before every user reply, apply `AGENT_BLUEPRINT.md` `[BP-WF-PROFILE]`.
