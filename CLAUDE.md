# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Nudge?

Nudge is a macOS-only CLI for managing Apple Reminders from the terminal. It provides subcommands for listing, searching, adding, completing, and deleting reminders, plus an interactive TUI mode (default when run with no arguments). All Reminders interactions happen via AppleScript executed through `osascript`.

## Build & Development Commands

```bash
# Build / run
cargo build                  # debug build
cargo build --release        # release build
cargo run                    # run TUI mode
cargo run -- <command>       # run a specific subcommand

# Lint & format
cargo fmt                    # format code
cargo fmt --check            # check formatting
cargo clippy                 # run lints

# Task runner (justfile)
just build                   # debug build
just release                 # release build
just run                     # run dev build
just install                 # install to ~/.local/bin
just clean                   # clean artifacts
```

No automated tests exist yet — testing is manual against the macOS Reminders app.

## Architecture

Three source files in `src/`:

- **main.rs** — CLI entry point using clap derive macros. Defines subcommands (`list`, `search`, `add`, `done`, `delete`, `lists`) and dispatches to the appropriate module. No subcommand launches the TUI.
- **reminders.rs** — Apple Reminders backend. All interaction goes through `run_applescript()` which shells out to `osascript`. Reminders are parsed from AppleScript output using `|||` as a delimiter. String escaping via `escape_applescript()` is critical for correctness.
- **tui.rs** — Interactive browse/search/add mode built with ratatui + crossterm. Modes: Browse (j/k navigation), Search (incremental filtering), Add (quick inline entry). Manages raw mode and alternate screen.

## Key Patterns

- **Error handling**: `anyhow::Result` throughout, with `.context()` for user-facing messages and `bail!()` for early returns.
- **Data flow**: CLI → `reminders.rs` (AppleScript via osascript) → parse `|||`-delimited output → `Reminder { id, name, list, due_date, completed, priority }`.
- **TUI lifecycle**: Enter raw mode / alternate screen → event loop → restore terminal on exit.

## Requirements

- macOS 13+ (depends on Apple Reminders app and AppleScript)
- Rust 2024 edition
