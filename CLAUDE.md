# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Nudge?

Nudge is a macOS-only CLI for managing Apple Reminders from the terminal. It provides subcommands for listing, searching, adding, completing, and deleting reminders, plus an interactive TUI mode (default when run with no arguments).

## Build & Development Commands

```bash
# Full build (Swift bridge + Rust CLI)
just build                   # debug build
just release                 # release build
just run                     # build + run TUI mode
just install                 # install both binaries to ~/.local/bin
just clean                   # clean all build artifacts

# Rust-only (won't work at runtime without nudge-bridge)
cargo build
cargo run -- <command>       # run a specific subcommand

# Swift bridge only
cd swift-bridge && swift build

# Lint & format
cargo fmt                    # format code
cargo fmt --check            # check formatting
cargo clippy                 # run lints
```

`just build`/`just run` is the typical dev workflow — it builds the Swift bridge first, copies it to `target/debug/`, then builds and runs the Rust binary.

No automated tests exist — testing is manual against the macOS Reminders app.

## Architecture

Two-process architecture: a Rust CLI (`nudge`) shells out to a Swift helper binary (`nudge-bridge`) that uses EventKit for direct Reminders database access.

### Rust (`src/`)

- **main.rs** — CLI entry point using clap derive macros. Defines subcommands (`list`, `search`, `add`, `done`, `delete`, `lists`) and `list` subcommands (`new`, `rename`, `delete`). No subcommand launches the TUI.
- **reminders.rs** — Backend that calls `nudge-bridge` via `std::process::Command`. The bridge binary must be adjacent to the `nudge` executable. Parses `|||`-delimited stdout lines into `Reminder { id, name, list, due_date, completed, priority }`.
- **tui.rs** — Interactive mode built with ratatui + crossterm. Modes: Browse, Search, Add, Help, ThemePicker, ListPicker, ListInput, MovePicker. Manages raw mode and alternate screen.
- **theme.rs** — Color themes using ratatui `Color::Indexed`. Themes: synthwave (default), monochrome, ocean, sunset, forest, tokyo night moon.
- **config.rs** — Persists user preferences (currently just theme) to `~/.config/nudge/config.toml` via serde + toml.

### Swift (`swift-bridge/Sources/main.swift`)

Single-file Swift CLI using EventKit. Accepts commands (`list`, `search`, `add`, `complete`, `delete`, `move`, `create-list`, `rename-list`, `delete-list`) and outputs `|||`-delimited fields to stdout. Errors go to stderr with `exit(1)`.

### IPC Protocol

Rust → Swift communication is via CLI args and stdout. Output format for reminders: `list|||id|||name|||due_date|||completed|||priority` (one per line).

## Key Patterns

- **Error handling**: `anyhow::Result` throughout Rust code, with `.context()` for user-facing messages and `bail!()` for early returns.
- **Binary co-location**: `nudge-bridge` must be in the same directory as `nudge` — `reminders.rs:bridge_path()` resolves it relative to `current_exe()`.
- **TUI lifecycle**: Enter raw mode / alternate screen → event loop → restore terminal on exit.

## Requirements

- macOS 13+ (EventKit Reminders access)
- Rust 2024 edition + Swift toolchain
