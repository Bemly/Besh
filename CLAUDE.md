# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Besh** (Bemly's Shell) is a Unix shell implemented in Rust, inspired by lsh. It provides both an interactive REPL and single-command execution modes.

## Architecture

### Entry Point
`src/main.rs` - Dispatches to shell implementations based on CLI arguments:
- `common_shell`: Rust stdlib-based shell (primary implementation)
- `better_truth_tty`: libc-based shell (NOT implemented - empty stub)

### Core Modules

**`src/common_shell.rs`**
- Main shell implementation using Rust's `std::process::Command`
- REPL loop with user@hostname prompt
- Built-in commands: `cd`, `exit/quit/q/:q`
- `Homedir` struct handles `~` expansion for path display
- Uses `Command::new().args().output()` for external command execution
- Issues: `~` expansion in `cd` has known bugs (line 106 comments)

**`src/better_truth_tty.rs`**
- Placeholder for libc-based implementation
- Currently empty - activated via `-u | --use-libc` flag but does nothing
- Technical comments explain intended use of `execvp` via libc

**`src/error.rs`**
- Error string constants (currently unused - defined but not referenced)

**`tests/command_exec_test.rs`**
- Integration tests for `Command` behavior (spawn, status, exec)

## Build Commands

```bash
# Build release binary
cargo build --release

# Run shell
./target/release/besh

# Run tests
cargo test

# Run specific test
cargo test test_spawn
cargo test test_status
```

## CLI Usage

```bash
besh                   # Enter interactive shell
besh <command [args]>  # Execute single command
besh -h | --help       # Show help
besh -v | --version    # Show version
besh -u | --use-libc   # Use libc shell (not implemented)
besh <*.besh [args]>   # Execute script (not implemented)
```

## Known Issues & Technical Debt

- `better_truth_tty.rs` is completely empty - libc implementation is stub only
- `~` path expansion in `cd` command has bugs (line 106 in common_shell.rs)
- Error constants in `error.rs` are defined but unused throughout codebase
- Script execution (*.besh) is documented but not implemented
- TODO comments indicate code quality issues with whitespace parsing (line 89)

## Implementation Notes

The shell uses `Command::output()` which spawns a new process and blocks waiting for completion. Contrast with `Command::exec()` which would replace the current process (requires unsafe libc calls).

For future libc implementation, refer to comments in `better_truth_tty.rs` explaining the POSIX `execvp` protocol and Rust's internal `do_exec` implementation.
