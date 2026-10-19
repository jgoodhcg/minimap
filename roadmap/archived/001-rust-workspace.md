---
title: "Rust workspace and toolchain"
status: done
description: "Create the pinned Rust workspace with a core crate and a TUI crate, and make all validation levels runnable"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-1, scaffold]
priority: high
---

# Rust workspace and toolchain

## Intent

Every later unit needs a building workspace and working validation commands.
The core/renderer split must exist from the first commit, so a later pixel renderer can attach without a rewrite.

## Specification

Precondition: the Rust toolchain.

1. Run `cargo --version`.
   If it fails and `$HOME/.cargo/bin/cargo` exists, prefix each command with `source "$HOME/.cargo/env" &&`.
2. If Rust is not installed, print these commands for the user and wait. Do not run them.

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source "$HOME/.cargo/env"
   rustc --version && cargo --version
   ```

Workspace:

- Root `Cargo.toml`: `[workspace]` with `members = ["crates/core", "crates/tui"]`, `resolver = "3"`, and shared `[workspace.package]` fields (`edition = "2024"`, `license = "MIT OR Apache-2.0"`, `version = "0.1.0"`).
- `rust-toolchain.toml`: `channel` = the exact version that `rustc --version` reports, `components = ["rustfmt", "clippy"]`.
  rustup can download that named toolchain on the first `cargo` run. This is expected and needs no confirmation.
- `crates/core`: package `minimap-core`, a library. No terminal or renderer dependencies.
- `crates/tui`: package `minimap-tui`, with `[[bin]] name = "minimap"`, `path = "src/main.rs"`. It depends on `minimap-core`.
  - Dependencies: `ratatui` 0.30 (use its re-export `ratatui::crossterm`; do not add a separate `crossterm` version), `clap` 4 with `derive`, `anyhow`.
  - Dev dependency: `insta`.
- `minimap --version` prints `minimap 0.1.0`.
- `minimap` with no arguments opens the alternate screen, draws a bordered frame titled `minimap`, and exits on `q` or Ctrl+C.
- A panic hook restores the terminal (leave the alternate screen, show the cursor, disable raw mode) before the panic message prints.
- `crates/tui/tests/render.rs`: one test renders the frame into a Ratatui `TestBackend` of 60 × 10 and asserts it with `insta::assert_snapshot!`. This makes validation level 4 exist.
- Commit `Cargo.lock`.

## Validation

- [x] `cargo fmt --all --check` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] `cargo build --workspace --locked` passes.
- [x] `cargo test --workspace --locked` passes, including the `render` snapshot test.
- [x] `cargo run -p minimap-tui -- --version` prints `minimap 0.1.0`.
- [x] Visual check (see `AGENTS.md` `Visual check`): the frame shows in a herdr pane, and `q` exits and restores the shell prompt.

## Scope

- No index, no layout, no map rendering.
- No CI configuration.
- No publish metadata beyond the workspace fields above.

## Context

- Architecture split: `docs/design.md` `Architecture`.
- Crate versions checked on crates.io on 2026-09-24: `ratatui` 0.30.2, `crossterm` 0.29.0, `insta` 1.48.0, `gix` 0.87.1, `tree-sitter` 0.27.0.
- `AGENTS.md` `Never Run`: no `cargo publish`.
