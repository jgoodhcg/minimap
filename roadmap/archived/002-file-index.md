---
title: "File index"
status: done
description: "List a repo's tracked files with exclusions, line counts, and binary flags, and dump or load the index as JSON"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1, core, index]
priority: high
---

# File index

## Intent

The map needs the repo's file list.
Real repos often track thousands of asset files that are noise on a map, so exclusions are necessary from day one.
A JSON dump and load lets the agent and the tests run the TUI on the fake `quayside` repo without a real checkout.

## Specification

In `minimap-core`, module `index`:

- `Index::from_repo(path, &IndexOptions) -> Result<Index>`.
  - Discover the repo root from `path` upward with `gix`.
  - List tracked files from the git index (the same set as `git ls-files`).
  - Drop paths that match any exclude glob (`globset`). Globs match repo-relative paths.
  - For each file: repo-relative path, line count, binary flag, missing flag.
    - Binary: the first 8 KiB contains a NUL byte. A binary file has line count 0.
    - Line count: the number of `\n` bytes, plus 1 when the file is not empty and does not end in `\n`.
    - Missing: tracked but absent from the working tree. Keep it in the index with line count 0.
  - Read files in parallel only if a single-threaded pass over the real target repo takes more than 1 s.
- `IndexOptions { excludes: Vec<String> }`.
- Config: `$XDG_CONFIG_HOME/minimap/config.toml` (default `~/.config/minimap/config.toml`), overridable with `--config <path>`.

  ```toml
  [repos."/Users/example/projects/big-repo"]
  exclude = ["web/public/icons/**"]
  ```

  Excludes from the config and from repeated `--exclude <glob>` flags combine.
  A missing config file is not an error.
- JSON shape, matching `mockups/fake.json`: `{"root": "<repo dir name>", "files": [{"p": "<path>", "n": <lines>, "b": 1?, "x": 1?}], "commits": {}}`.
  `b` is present only for binary files. `x` is present only for missing files.
  Loading must accept `mockups/fake.json` and ignore fields it does not know (`m`, `o`, `ns`, `a0`, `a1`, `c`, `r`, `ig`), except that it keeps `ig`.
- CLI in `minimap-tui`:
  - `minimap index [PATH] [--exclude <glob>]... [--config <path>]` prints the JSON to stdout, and prints `N files, M excluded, T ms` to stderr.
  - `minimap [PATH]` and `minimap --from-json <file>` are wired in unit `004`.

## Validation

- [x] Unit tests build a temporary git repo (with `tempfile` and the `git` CLI) and assert:
  tracked files are listed; untracked files are not; excludes drop matching paths;
  line counts for an empty file, a file with a trailing newline, and a file without one;
  a file with a NUL byte is binary; a deleted tracked file is missing.
- [x] A test loads `crates/core/tests/fixtures/quayside-files.json` (see unit `003`) or a small inline JSON with `ig` and unknown fields, and round-trips the known fields.
- [x] Config test: a temporary config file with an `exclude` for the temp repo's absolute path is applied.
- [x] Real-repo test (`AGENTS.md` `Real-repo tests`): the index file count equals the `git ls-files` count minus the paths the config excludes, and no indexed path matches an exclude glob. Record the index time in the completion report.
- [x] Write `minimap index --config .private/minimap.toml <target repo> > .private/fixtures/target-files.json` for unit `003`.
- [x] All `AGENTS.md` validation levels 1–3 pass.

## Scope

- No git history (unit `008`), no line shapes or outlines (unit `007`), no ignored-file allowlist (Quick Ideas).
- No file watching. The index is built once at startup.
- No writes to the target repo.

## Context

- Mockup data shape: `mockups/fakegen.py:800-834`.
- `AGENTS.md` `Project-Specific Rules`: real-repo output goes under `.private/`.
