# nudge

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Swift](https://img.shields.io/badge/swift-6.2+-F05138?logo=swift&logoColor=white)](https://swift.org/)
[![macOS](https://img.shields.io/badge/macOS-13%2B-000000?logo=apple&logoColor=white)](https://www.apple.com/macos/)

Apple Reminders from your terminal.

A fast CLI and interactive TUI for managing Apple Reminders on macOS. Built in Rust with a Swift bridge using EventKit for native performance.

## Install

Requires macOS 13+ and Rust/Swift toolchains.

```bash
just install
```

This builds both the Rust CLI and the Swift EventKit bridge, then copies them to `~/.local/bin`.

## Usage

Run with no arguments to launch the interactive TUI:

```bash
nudge
```

### Subcommands

```bash
nudge list                          # list incomplete reminders
nudge list -l Shopping              # filter by list
nudge list -a                       # include completed
nudge search "groceries"            # search by name
nudge add "Buy milk"                # add to default list
nudge add "Call dentist" -l Health  # add to specific list
nudge add "Submit paper" -d 2026-03-20 -p 1  # with due date and priority
nudge done "Buy milk"               # mark as completed
nudge delete "Buy milk"             # delete a reminder
nudge lists                         # show all reminder lists
```

Priority values: `1` = high, `5` = medium, `9` = low, `0` = none.

### TUI Keybindings

| Key | Action |
|-----|--------|
| `j/k` or `Up/Down` | Navigate |
| `Enter` | Complete selected reminder |
| `a` | Add new reminder |
| `r` | Refresh from Reminders.app |
| `d` | Delete selected reminder |
| `/` | Search |
| `?` | Help |
| `q` or `Esc` | Quit |

## Architecture

Rust CLI (`nudge`) delegates to a Swift helper binary (`nudge-bridge`) that uses EventKit for direct access to the Reminders database. This avoids the severe performance overhead of AppleScript/osascript.

## Permissions

On first run, macOS will prompt for Reminders access. You can manage this in System Settings > Privacy & Security > Reminders.
