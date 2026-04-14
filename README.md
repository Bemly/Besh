# BESH

Bemly's Shell — a Unix shell written in Rust with libc.

Inspired by [lsh](https://github.com/brenns10/lsh).

## Features

- Interactive REPL with colored prompt
- Command pipelines (`cmd1 | cmd2 | cmd3`)
- I/O redirection (`>`, `>>`, `<`, `2>`)
- Background jobs (`cmd &`, `jobs`)
- Tab completion (commands + file paths)
- Arrow key history navigation
- Line editing (Ctrl+A/E/K/U)
- Environment variable management
- Script execution (`.besh` files)
- Signal handling (Ctrl+C, Ctrl+Z)
- Job control

## Build

```bash
git clone https://github.com/Bemly/Besh.git
cd Besh
cargo build --release
```

The binary is at `target/release/besh`.

Optionally install to `~/.cargo/bin`:

```bash
cargo install --path .
```

## Usage

### Interactive Mode

```bash
./target/release/besh
```

```
user@hostname ~> ls -la
user@hostname ~> echo hello world
user@hostname ~> exit
```

### Single Command

```bash
./target/release/besh echo "hello world"
./target/release/besh ls -la /tmp
```

### Script Execution

```bash
cat > test.besh << 'EOF'
#!/usr/bin/env besh
echo "Running script"
export VAR=value
echo $VAR
ls -la | head -3
EOF

./target/release/besh test.besh
```

## Built-in Commands

| Command | Description |
|---------|-------------|
| `cd [path]` | Change directory (supports `~`) |
| `pwd` | Print working directory |
| `echo [args...]` | Print to stdout |
| `export VAR=val` | Set environment variable |
| `unset VAR` | Unset variable |
| `env` | Print all environment variables |
| `set` | Print all shell variables |
| `history [n]` | Show last `n` commands |
| `jobs` | List background jobs |
| `exit` / `quit` / `q` | Exit shell |

## Pipes & Redirection

```bash
# Pipe
cat file.txt | grep "pattern" | wc -l

# Output redirect
ls -la > listing.txt

# Append
echo "line" >> file.txt

# Input redirect
sort < unsorted.txt

# Stderr redirect
ls /notfound 2> errors.txt
```

## Background Jobs

```bash
sleep 60 &
jobs
```

## Tab Completion

- Press `Tab` to complete commands and file paths
- First word: completes built-in commands and executables from `$PATH`
- Other words: completes file/directory paths
- Multiple matches are displayed, common prefix is auto-completed

## Line Editing

| Key | Action |
|-----|--------|
| `←` / `→` | Move cursor |
| `↑` / `↓` | Navigate history |
| `Tab` | Complete |
| `Ctrl+A` | Move to start |
| `Ctrl+E` | Move to end |
| `Ctrl+K` | Delete to end of line |
| `Ctrl+U` | Delete to start of line |
| `Ctrl+C` | Cancel input |
| `Ctrl+D` | EOF / exit |

## Environment Variables

```bash
export PATH=/usr/local/bin:$PATH
export EDITOR=vim
echo $HOME
echo $USER
```

## Disable Colors

```bash
NO_COLOR=1 ./target/release/besh
```

## CLI Options

```
besh                    Enter interactive shell
besh <command [args]>   Execute a single command
besh <script.besh>      Execute a script file
besh -h, --help         Show help
besh -v, --version      Show version
```

## Project Structure

```
src/
├── main.rs          Entry point
├── shell.rs         REPL loop and command execution
├── parser.rs        Command line parsing (pipes, redirects, variables)
├── process.rs       Fork/exec/wait via libc
├── terminal.rs      Raw mode, line editing, tab completion, colors
├── builtin.rs       Built-in command implementations
├── environment.rs   Variable management and prompt formatting
├── history.rs       Command history with file persistence
├── job_control.rs   Background/foreground job management
├── signal.rs        SIGINT, SIGTSTP, SIGCHLD handlers
├── error.rs         Error types
├── common_shell.rs  Legacy stdlib-based shell (fallback)
└── better_truth_tty.rs  Placeholder for future libc shell
```

API documentation is generated in `docs/` via `cargo doc`.

## Dependencies

- `libc` — POSIX system calls
- `dirs` — Home directory detection

## License

See [LICENSE](LICENSE).
