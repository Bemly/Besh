//! Main shell implementation using libc.

use crate::builtin::{execute_builtin, is_builtin, ShellState};
use crate::environment::{load_environment, PromptComponents};
use crate::error::{Result, ShellError};
use crate::history::History;
use crate::job_control::JobControl;
use crate::parser::{parse_command_line, Command};
use crate::process::{Pipe, ProcessBuilder, Redirection};
use crate::signal::{setup_signal_handlers, was_signal_received};
use crate::terminal::{isatty, Terminal, color};
use std::collections::VecDeque;
use std::io::{self, Write};

/// Main shell REPL
pub fn run_shell(args: Vec<String>) -> Result<()> {
    let is_tty = isatty();

    // Check if this is a script file execution
    if !args.is_empty() {
        let first_arg = &args[0];

        // Check if it's a .besh file or a shebang script
        if first_arg.ends_with(".besh") || std::path::Path::new(first_arg).is_file() {
            return run_script(&args);
        }
    }

    if !is_tty {
        // If args were provided, execute them as a command directly
        if !args.is_empty() {
            return run_command_args(args);
        }
        // No args - read from stdin
        return run_non_interactive(args);
    }

    // Interactive mode

    // Setup signal handlers
    setup_signal_handlers()?;

    // Initialize shell state
    let mut state = ShellState::new()?;
    let mut environment = crate::environment::Environment::new();
    load_environment(&mut environment);

    // Initialize history
    let mut history = History::new();
    let _ = history.load();

    // Initialize job control
    let mut job_control = JobControl::new()?;

    // Enter REPL loop
    repl(&mut state, &mut environment, &mut history, &mut job_control)
}

/// Run a script file
fn run_script(args: &[String]) -> Result<()> {
    let script_path = &args[0];
    let script_args = &args[1..];

    let mut state = ShellState::new()?;

    // Set script arguments as positional parameters
    state.set_var("0", script_path);
    for (i, arg) in script_args.iter().enumerate() {
        state.set_var(&format!("{}", i + 1), arg);
    }

    // Read the script file
    let content = std::fs::read_to_string(script_path)
        .map_err(|e| ShellError::IoError(e))?;

    // Execute each line
    for (line_num, line) in content.lines().enumerate() {
        let input = line.trim();

        // Skip comments and empty lines
        if input.is_empty() || input.starts_with('#') {
            continue;
        }

        // Parse and execute
        let commands = parse_command_line(input)?;

        if !commands.is_empty() {
            if is_exit_command(&commands[0]) {
                break;
            }
        }

        for cmd in commands {
            if is_builtin(&cmd.program) {
                let result = execute_builtin(&cmd, &mut state);
                if let Ok(exit_status) = result {
                    state.exit_code = exit_status.code();
                }
                if let Err(e) = result {
                    if !matches!(e, ShellError::CommandNotFound(_)) {
                        eprintln!("besh:{}: {}", line_num + 1, e);
                    }
                }
            } else {
                execute_single_command(&cmd, &mut state)?;
            }
        }
    }

    Ok(())
}

/// Execute command arguments directly (e.g., besh echo hello)
fn run_command_args(args: Vec<String>) -> Result<()> {
    let mut state = ShellState::new()?;
    let input = args.join(" ");
    let commands = parse_command_line(&input)?;

    for cmd in commands {
        if is_builtin(&cmd.program) {
            let _guard = setup_builtin_redirections(&cmd)?;
            let result = execute_builtin(&cmd, &mut state);
            if let Ok(exit_status) = result {
                state.exit_code = exit_status.code();
            }
            if let Err(e) = result {
                if !matches!(e, ShellError::CommandNotFound(_)) {
                    eprintln!("besh: {}", e);
                }
            }
        } else {
            execute_single_command(&cmd, &mut state)?;
        }
    }
    Ok(())
}

/// Temporarily redirect stdout/stderr for builtins
struct FdGuard {
    saved_stdout: Option<libc::c_int>,
    saved_stderr: Option<libc::c_int>,
    out_fd: Option<libc::c_int>,
    err_fd: Option<libc::c_int>,
}

impl Drop for FdGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(saved) = self.saved_stdout {
                libc::dup2(saved, libc::STDOUT_FILENO);
                libc::close(saved);
            }
            if let Some(saved) = self.saved_stderr {
                libc::dup2(saved, libc::STDERR_FILENO);
                libc::close(saved);
            }
            if let Some(fd) = self.out_fd {
                libc::close(fd);
            }
            if let Some(fd) = self.err_fd {
                libc::close(fd);
            }
        }
    }
}

fn setup_builtin_redirections(cmd: &Command) -> Result<FdGuard> {
    let mut guard = FdGuard {
        saved_stdout: None, saved_stderr: None,
        out_fd: None, err_fd: None,
    };

    unsafe {
        if let Some(ref redir) = cmd.stdout {
            if let Redirection::File(path) = redir {
                let c_path = std::ffi::CString::new(
                    if path.starts_with("append:") { &path[7..] } else { path.as_str() }
                ).map_err(|_| ShellError::ParseError("Invalid path".to_string()))?;
                let flags = if path.starts_with("append:") {
                    libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND
                } else {
                    libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC
                };
                let fd = libc::open(c_path.as_ptr(), flags, 0o644);
                if fd < 0 {
                    return Err(ShellError::IoError(io::Error::last_os_error()));
                }
                let saved = libc::dup(libc::STDOUT_FILENO);
                libc::dup2(fd, libc::STDOUT_FILENO);
                guard.saved_stdout = Some(saved);
                guard.out_fd = Some(fd);
            }
        }

        if let Some(ref redir) = cmd.stderr {
            if let Redirection::File(path) = redir {
                let c_path = std::ffi::CString::new(
                    if path.starts_with("append:") { &path[7..] } else { path.as_str() }
                ).map_err(|_| ShellError::ParseError("Invalid path".to_string()))?;
                let flags = if path.starts_with("append:") {
                    libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND
                } else {
                    libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC
                };
                let fd = libc::open(c_path.as_ptr(), flags, 0o644);
                if fd < 0 {
                    return Err(ShellError::IoError(io::Error::last_os_error()));
                }
                let saved = libc::dup(libc::STDERR_FILENO);
                libc::dup2(fd, libc::STDERR_FILENO);
                guard.saved_stderr = Some(saved);
                guard.err_fd = Some(fd);
            }
        }
    }

    Ok(guard)
}

/// Run in non-interactive mode
fn run_non_interactive(_args: Vec<String>) -> Result<()> {
    let mut state = ShellState::new()?;

    // Read from stdin and execute each line
    loop {
        let mut line = String::new();
        let bytes_read = std::io::stdin().read_line(&mut line)
            .map_err(ShellError::IoError)?;

        if bytes_read == 0 {
            break; // EOF
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        // Parse and execute
        let commands = parse_command_line(input)?;

        if !commands.is_empty() {
            if is_exit_command(&commands[0]) {
                return Ok(());
            }
        }

        // Execute without job control for non-interactive mode
        for cmd in commands {
            if is_builtin(&cmd.program) {
                let _guard = setup_builtin_redirections(&cmd)?;
                let result = execute_builtin(&cmd, &mut state);
                if let Ok(exit_status) = result {
                    state.exit_code = exit_status.code();
                }
                if let Err(e) = result {
                    if !matches!(e, ShellError::CommandNotFound(_)) {
                        eprintln!("besh: {}", e);
                    }
                }
            } else {
                // Execute external command using process builder (simplified)
                execute_single_command(&cmd, &mut state)?;
            }
        }
    }

    Ok(())
}

/// Execute a single command in non-interactive mode
fn execute_single_command(cmd: &Command, _state: &mut ShellState) -> Result<()> {
    let mut sys_cmd = std::process::Command::new(&cmd.program);
    sys_cmd.args(&cmd.args);

    // Apply stdin redirection
    if let Some(ref redir) = cmd.stdin {
        match redir {
            Redirection::File(path) => {
                let file = std::fs::File::open(path)
                    .map_err(ShellError::IoError)?;
                sys_cmd.stdin(file);
            }
            _ => {}
        }
    }

    // Apply stdout redirection
    if let Some(ref redir) = cmd.stdout {
        match redir {
            Redirection::File(path) => {
                if path.starts_with("append:") {
                    let file = std::fs::OpenOptions::new()
                        .create(true).append(true)
                        .open(&path[7..])
                        .map_err(ShellError::IoError)?;
                    sys_cmd.stdout(file);
                } else {
                    let file = std::fs::File::create(path)
                        .map_err(ShellError::IoError)?;
                    sys_cmd.stdout(file);
                }
            }
            _ => {}
        }
    }

    // Apply stderr redirection
    if let Some(ref redir) = cmd.stderr {
        match redir {
            Redirection::File(path) => {
                if path.starts_with("append:") {
                    let file = std::fs::OpenOptions::new()
                        .create(true).append(true)
                        .open(&path[7..])
                        .map_err(ShellError::IoError)?;
                    sys_cmd.stderr(file);
                } else {
                    let file = std::fs::File::create(path)
                        .map_err(ShellError::IoError)?;
                    sys_cmd.stderr(file);
                }
            }
            _ => {}
        }
    }

    let output = sys_cmd.output()?;

    // Print stdout
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Print stderr
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

/// REPL (Read-Eval-Print Loop)
fn repl(
    state: &mut ShellState,
    _environment: &mut crate::environment::Environment,
    history: &mut History,
    job_control: &mut JobControl,
) -> Result<()> {
    let mut terminal = Terminal::new()?;
    let is_tty = isatty();

    loop {
        // Check for completed background jobs
        job_control.cleanup_jobs();

        // Display prompt
        let prompt = PromptComponents::new()
            .map(|p| p.format())
            .unwrap_or_else(|_| "> ".to_string());

        print!("{}", prompt);
        io::stdout().flush()?;

        // Read command line
        let input = if is_tty {
            read_line(&mut terminal, history)?
        } else {
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            line.trim().to_string()
        };

        // Handle empty input
        let input = input.trim();
        if input.is_empty() {
            if was_signal_received() {
                continue;
            }
            continue;
        }

        // Check if signal was received during input
        if was_signal_received() {
            continue;
        }

        // Add to history
        history.add(input.to_string());

        // Parse command line
        let commands = parse_command_line(input)?;

        // Check for exit/quit
        if !commands.is_empty() {
            if is_exit_command(&commands[0]) {
                return Ok(());
            }
        }

        // Execute commands
        match execute_commands(commands, state, job_control) {
            Ok(()) => {}
            Err(ShellError::CommandNotFound(ref cmd)) => {
                if color::enabled() {
                    eprintln!("{}besh:{} command not found: {}{}", color::RED, color::RESET, color::BOLD, cmd);
                    eprint!("{}", color::RESET);
                }
            }
            Err(e) => {
                if color::enabled() {
                    eprintln!("{}besh: {}{}{}", color::RED, color::RESET, e, color::RESET);
                } else {
                    eprintln!("besh: {}", e);
                }
            }
        }
    }
}

/// Read a line from stdin
fn read_line(_terminal: &mut Terminal, _history: &mut History) -> Result<String> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)
        .map_err(ShellError::IoError)?;
    Ok(buffer.trim().to_string())
}

/// Check if this is an exit command
fn is_exit_command(cmd: &Command) -> bool {
    matches!(
        cmd.program.to_lowercase().as_str(),
        "exit" | "quit" | "q" | "qui" | "qu" | ":q"
    )
}

/// Execute parsed commands
fn execute_commands(
    commands: VecDeque<Command>,
    state: &mut ShellState,
    job_control: &mut JobControl,
) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }

    // Check if first command is a built-in (single command case)
    if commands.len() == 1 && is_builtin(&commands[0].program) {
        let result = execute_builtin(&commands[0], state);
        if let Ok(exit_status) = result {
            state.exit_code = exit_status.code();
        }
        return result.map(|_| ());
    }

    // Single external command - use simple fork/wait without job control
    if commands.len() == 1 && !commands[0].background {
        return execute_single_foreground(&commands[0], state);
    }

    // Handle pipeline or background jobs
    let is_pipeline = commands.len() > 1;

    // Create pipes if needed
    let pipes: Option<Vec<Pipe>> = if is_pipeline {
        Some(job_control.create_pipes(commands.len())?)
    } else {
        None
    };

    let mut first_pgid = 0;
    let mut processes = Vec::new();
    let mut command_str = String::new();
    let background = commands.iter().any(|c| c.background);

    // Build the command string first
    for (i, cmd) in commands.iter().enumerate() {
        if i == 0 {
            command_str = cmd.program.clone();
            if !cmd.args.is_empty() {
                command_str.push(' ');
                command_str.push_str(&cmd.args.join(" "));
            }
        } else {
            command_str.push_str(" | ");
            command_str.push_str(&cmd.program);
            if !cmd.args.is_empty() {
                command_str.push(' ');
                command_str.push_str(&cmd.args.join(" "));
            }
        }
    }

    // Now spawn each process with configured redirections
    for (i, cmd) in commands.iter().enumerate() {
        // Collect all redirection configurations first
        let stdin_redir = match &cmd.stdin {
            Some(Redirection::File(path)) => {
                Some((open_file_to_redir(path, libc::O_RDONLY, 0)?, "fd"))
            }
            Some(Redirection::Pipe(pipe_fd)) => {
                Some((Redirection::Pipe(*pipe_fd), "pipe"))
            }
            None if i > 0 => {
                // Connect to previous pipe
                if let Some(pipes) = &pipes {
                    Some((Redirection::Pipe(pipes[i - 1].read_fd()), "pipe"))
                } else {
                    None
                }
            }
            _ => None,
        };

        let stdout_redir = match &cmd.stdout {
            Some(Redirection::File(path)) => {
                if path.starts_with("append:") {
                    Some(open_file_to_redir(&path[7..], libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND, 0o644)?)
                } else {
                    Some(open_file_to_redir(path, libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644)?)
                }
            }
            Some(Redirection::Pipe(pipe_fd)) => {
                Some(Redirection::Pipe(*pipe_fd))
            }
            None if i < commands.len() - 1 => {
                // Connect to next pipe
                if let Some(pipes) = &pipes {
                    Some(Redirection::Pipe(pipes[i].write_fd()))
                } else {
                    None
                }
            }
            _ => None,
        };

        let stderr_redir = if let Some(Redirection::File(path)) = &cmd.stderr {
            if path.starts_with("append:") {
                Some(open_file_to_redir(&path[7..], libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND, 0o644)?)
            } else {
                Some(open_file_to_redir(path, libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644)?)
            }
        } else {
            None
        };

        // Build the builder with all redirections using the builder pattern
        let mut builder = ProcessBuilder::new(&cmd.program);
        builder = builder.args_ref(&cmd.args);

        if let Some((redir, _)) = stdin_redir {
            builder = builder.stdin(redir);
        }

        if let Some(redir) = stdout_redir {
            builder = builder.stdout(redir);
        }

        if let Some(redir) = stderr_redir {
            builder = builder.stderr(redir);
        }

        // Set process group - first process becomes the group leader
        let pgid = if first_pgid == 0 {
            0 // Child will use its own pid as pgid
        } else {
            first_pgid
        };
        builder = builder.pgid(pgid);

        // Spawn process
        let process = builder.spawn()?;

        // Track process group
        let pgid = if first_pgid == 0 {
            process.pid() // First process is the group leader
        } else {
            first_pgid
        };

        if first_pgid == 0 {
            first_pgid = pgid;
        }

        processes.push(process);
    }

    // Close pipe ends in parent
    drop(pipes);

    // Add job to job control
    let job_num = job_control.add_job(first_pgid, command_str, background);

    // Wait for foreground jobs, run background jobs
    if background {
        let cmd_str = job_control.find_job_by_pgid(first_pgid)
            .map(|j| j.command.clone())
            .unwrap_or_default();
        if color::enabled() {
            println!("{}[{}]{} {}", color::YELLOW, job_num, color::RESET, cmd_str);
        } else {
            println!("[{}] {}", job_num, cmd_str);
        }
        job_control.put_job_in_background(first_pgid)?;
    } else {
        let job_state = job_control.put_job_in_foreground(first_pgid)?;
        if job_state == crate::job_control::JobState::Stopped {
            let job_id = job_control.find_job_by_pgid(first_pgid)
                .map(|j| j.job_id())
                .unwrap_or("%1".to_string());
            if color::enabled() {
                println!("{}[{}]{} Stopped", color::YELLOW, job_id, color::RESET);
            } else {
                println!("[{}] Stopped", job_id);
            }
        }
    }

    Ok(())
}

/// Open file and return redirection type
fn open_file_to_redir(path: &str, flags: libc::c_int, mode: libc::c_int) -> Result<Redirection> {
    let c_path = std::ffi::CString::new(path)
        .map_err(|_| ShellError::ParseError("Invalid path".to_string()))?;

    unsafe {
        let fd = libc::open(c_path.as_ptr(), flags, mode);
        if fd < 0 {
            return Err(ShellError::IoError(io::Error::last_os_error()));
        }
        Ok(Redirection::Fd(fd))
    }
}

/// Execute a single external command in the foreground (simple fork/wait)
fn execute_single_foreground(cmd: &Command, _state: &mut ShellState) -> Result<()> {
    let mut builder = ProcessBuilder::new(&cmd.program);
    builder = builder.args_ref(&cmd.args);

    if let Some(ref redir) = cmd.stdin {
        builder = builder.stdin(redir.clone());
    }
    if let Some(ref redir) = cmd.stdout {
        builder = builder.stdout(redir.clone());
    }
    if let Some(ref redir) = cmd.stderr {
        builder = builder.stderr(redir.clone());
    }

    let process = builder.spawn()?;

    // Simple wait for the child process
    let mut status: libc::c_int = 0;
    unsafe {
        libc::waitpid(process.pid(), &mut status, 0);
    }

    Ok(())
}
