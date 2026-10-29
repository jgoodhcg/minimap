# minimap

`minimap` draws a codebase as a stable 2D map and shows where a coding agent reads, searches, edits, and explains.
It runs beside the agent, for example in a terminal pane next to Claude Code.
The goal is to help a person understand and remember a codebase that an agent writes much of.

Each file is one tile, and tiles group in directory blocks.
The layout is a deterministic function of the file tree, so a file keeps its place from one session to the next.
Selection and layer toggles never move a tile.

## Status

This is a personal prototype. It is developed and tested on macOS. Expect breaking changes.

## Binaries

| Binary | Crate | What it is |
|---|---|---|
| `minimap` | `crates/tui` | The terminal UI (Ratatui). It is also the CLI for agents: `hook`, `note`, `send`, `index`, and `keys`. |
| `minimap-gui` | `crates/gui` | A desktop window (egui) with a map that zooms, an agent history, and a note reader. |

`crates/core` holds the file index, the layout, the git history, and the event socket. It has no terminal or renderer dependency.

## Build and install

Install [rustup](https://rustup.rs). `rust-toolchain.toml` pins the toolchain, and rustup installs it on the first build.

```sh
cargo build --workspace --locked
cargo install --path crates/tui --locked   # minimap
cargo install --path crates/gui --locked   # minimap-gui
```

## Run

```sh
minimap [PATH]       # PATH is a path inside a git repo. Default: the current directory.
minimap-gui [PATH]
```

- Press `?` in the terminal UI for the keys. `minimap keys` prints the keymap.
- For a large repo, leave out files with `--exclude 'GLOB'`. The option is repeatable.
- The terminal UI draws Nerd Font file-type icons. When the terminal font has no Nerd Font glyphs, add `--no-icons`.

To try it without a repo of your own, build the fake `quayside` repo:

```sh
python3 mockups/build.py
minimap --from-json mockups/fake.json
```

## Config

The config file is `$XDG_CONFIG_HOME/minimap/config.toml`, or `~/.config/minimap/config.toml`. `--config PATH` overrides it.
All sections are optional.

```toml
[map]                       # every repo
generated = ["*.min.js"]    # gitignore rules; `!` unmarks a file

[repos."/Users/example/projects/big-repo"]   # also every `git worktree` of it
exclude = ["web/public/icons/**"]
generated = ["schema/*.json", "!package-lock.json"]

[viewer]
theme = "~/themes/other.tmTheme"

[editor]
preset = "nvim"   # nvim | vim | vscode | emacs | custom
place = "tab"     # tab | right | down

[keys.map]        # also [keys.outline] and [keys.viewer]
"h" = "pan_left"
"a" = "none"

[gui]
font = "~/Library/Fonts/FiraCodeNerdFontMono-Regular.ttf"
```

## Agents

The map runs without an agent. Adapters connect it to one:

- Claude Code: a `PostToolUse` hook puts the agent's reads, searches, and edits on the map's trail.
  With `minimap note`, the agent points at lines of code in `minimap-gui` for a guided review.
  For the setup, see [`adapters/claude-code/README.md`](adapters/claude-code/README.md).
- Other tools can send events with `minimap send`. See `minimap send --help`.

## Docs

- [`docs/design.md`](docs/design.md): the design reference (layout, screen regions, layers, keys).
- `mockups/`: the HTML mockups. Run `python3 mockups/build.py`, then open `mockups/minimap-v5.html` in a browser.
- [`roadmap/`](roadmap/index.md): the work units.
- [`docs/git-history.md`](docs/git-history.md): the commit messages from before this repo was published.

## How this repo is built

Coding agents write most of the code, one work unit from `roadmap/` at a time.
[`AGENTS.md`](AGENTS.md) and [`AGENT_BLUEPRINT.md`](AGENT_BLUEPRINT.md) hold their operating rules.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
`crates/core/assets/` vendors the Tokyo Night theme with its own license.
