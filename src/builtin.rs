//! Built-in command implementations for the Besh shell.

use crate::error::{Result, ShellError};
use crate::parser::Command;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Shell state for built-in commands
#[derive(Debug)]
pub struct ShellState {
    /// Current working directory
    pub cwd: PathBuf,
    /// Internal shell variables
    pub variables: HashMap<String, String>,
    /// Home directory
    pub home: PathBuf,
    /// Exit code
    pub exit_code: u8,
}

impl ShellState {
    /// Create a new shell state
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| ShellError::NotFound("HOME directory".to_string()))?;

        let cwd = env::current_dir().unwrap_or_else(|_| home.clone());

        Ok(ShellState {
            cwd,
            variables: HashMap::new(),
            home,
            exit_code: 0,
        })
    }

    /// Change directory
    pub fn change_dir(&mut self, path: Option<String>) -> Result<ExitStatus> {
        let target = match path {
            Some(p) => {
                if p.starts_with('~') {
                    let normalized = p.replacen('~', &self.home.to_string_lossy(), 1);
                    PathBuf::from(normalized)
                } else if p.is_empty() {
                    self.home.clone()
                } else {
                    PathBuf::from(p)
                }
            }
            None => self.home.clone(),
        };

        let absolute = if target.is_absolute() {
            target.clone()
        } else {
            let mut abs = self.cwd.clone();
            abs.push(&target);
            abs
        };

        // Resolve canonical path
        let canonical = fs::canonicalize(&absolute)
            .map_err(|e| ShellError::IoError(e))?;

        if !canonical.is_dir() {
            return Err(ShellError::NotFound(format!("{} is not a directory", target.display())));
        }

        env::set_current_dir(&canonical)
            .map_err(|e| ShellError::IoError(e))?;

        self.cwd = canonical;
        Ok(ExitStatus::Success(0))
    }

    /// Set a variable
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    /// Get a variable (shell variables first, then environment)
    pub fn get_var(&self, name: &str) -> Option<String> {
        self.variables.get(name).cloned().or_else(|| env::var(name).ok())
    }

    /// Export a variable to the environment
    pub fn export_var(&mut self, name: &str, value: Option<String>) -> Result<ExitStatus> {
        if let Some(v) = value {
            self.variables.insert(name.to_string(), v.clone());
            env::set_var(name, v);
        } else if let Some(v) = self.variables.get(name) {
            env::set_var(name, v);
        }

        Ok(ExitStatus::Success(0))
    }

    /// Unset a variable
    pub fn unset_var(&mut self, name: &str) -> Result<ExitStatus> {
        self.variables.remove(name);
        env::remove_var(name);
        Ok(ExitStatus::Success(0))
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Exit status for built-in commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Command succeeded
    Success(u8),
    /// Command failed
    #[allow(dead_code)]
    Failure(u8),
}

impl ExitStatus {
    /// Returns true if the command exited successfully
    #[allow(dead_code)]
    pub fn success(&self) -> bool {
        matches!(self, ExitStatus::Success(0))
    }

    /// Returns the exit code
    pub fn code(&self) -> u8 {
        match self {
            ExitStatus::Success(c) => *c,
            ExitStatus::Failure(c) => *c,
        }
    }
}

/// Check if a command is a built-in
pub fn is_builtin(program: &str) -> bool {
    matches!(
        program,
        "cd" | "exit" | "quit" | "q" | "pwd" | "echo"
            | "export" | "unset" | "env" | "history"
            | "set" | "jobs" | "fg" | "bg"
    )
}

/// Execute a built-in command
pub fn execute_builtin(cmd: &Command, state: &mut ShellState) -> Result<ExitStatus> {
    match cmd.program.as_str() {
        "cd" => builtin_cd(cmd, state),
        "exit" | "quit" | "q" | "qui" | "qu" => builtin_exit(cmd, state),
        "pwd" => builtin_pwd(cmd, state),
        "echo" => builtin_echo(cmd, state),
        "export" => builtin_export(cmd, state),
        "unset" => builtin_unset(cmd, state),
        "env" => builtin_env(cmd, state),
        "history" => builtin_history(cmd, state),
        "set" => builtin_set(cmd, state),
        "jobs" => builtin_jobs(cmd, state),
        _ => Err(ShellError::CommandNotFound(cmd.program.clone())),
    }
}

/// Built-in: change directory
fn builtin_cd(cmd: &Command, state: &mut ShellState) -> Result<ExitStatus> {
    state.change_dir(cmd.args.first().map(|s| s.clone()))?;
    Ok(ExitStatus::Success(0))
}

/// Built-in: exit shell
fn builtin_exit(cmd: &Command, state: &mut ShellState) -> Result<ExitStatus> {
    let code = if let Some(arg) = cmd.args.first() {
        arg.parse::<u8>()
            .unwrap_or(state.exit_code)
    } else {
        state.exit_code
    };
    std::process::exit(code as i32);
}

/// Built-in: print working directory
fn builtin_pwd(_cmd: &Command, state: &ShellState) -> Result<ExitStatus> {
    println!("{}", state.cwd.display());
    Ok(ExitStatus::Success(0))
}

/// Built-in: echo arguments
fn builtin_echo(cmd: &Command, state: &ShellState) -> Result<ExitStatus> {
    let output = if cmd.args.is_empty() {
        String::new()
    } else {
        // Expand variables in arguments
        let expanded: Vec<String> = cmd.args
            .iter()
            .map(|arg| crate::parser::expand_variables(arg, |k| state.get_var(k)))
            .collect();
        expanded.join(" ")
    };
    println!("{}", output);
    Ok(ExitStatus::Success(0))
}

/// Built-in: export variable
fn builtin_export(cmd: &Command, state: &mut ShellState) -> Result<ExitStatus> {
    if cmd.args.is_empty() {
        // List all exported variables
        for (key, value) in env::vars().filter(|(k, _)| state.variables.contains_key(k) || has_var_in_environ(k)) {
            println!("{}={}", key, value);
        }
        return Ok(ExitStatus::Success(0));
    }

    for arg in &cmd.args {
        if let Some((name, value)) = arg.split_once('=') {
            state.export_var(name, Some(value.to_string()))?;
        } else {
            // Export existing variable
            state.export_var(arg, None)?;
        }
    }

    Ok(ExitStatus::Success(0))
}

/// Built-in: unset variable
fn builtin_unset(cmd: &Command, state: &mut ShellState) -> Result<ExitStatus> {
    for arg in &cmd.args {
        state.unset_var(arg)?;
    }
    Ok(ExitStatus::Success(0))
}

/// Built-in: print environment
fn builtin_env(_cmd: &Command, _state: &ShellState) -> Result<ExitStatus> {
    for (key, value) in env::vars() {
        println!("{}={}", key, value);
    }
    Ok(ExitStatus::Success(0))
}

/// Built-in: command history
fn builtin_history(cmd: &Command, _state: &ShellState) -> Result<ExitStatus> {
    let history_file = dirs::home_dir()
        .ok_or_else(|| ShellError::NotFound("HOME directory".to_string()))?
        .join(".besh_history");

    if !history_file.exists() {
        println!("No history");
        return Ok(ExitStatus::Success(0));
    }

    let content = fs::read_to_string(&history_file)
        .map_err(|e| ShellError::IoError(e))?;

    let lines: Vec<&str> = content.lines().collect();

    let count = if let Some(arg) = cmd.args.first() {
        arg.parse::<usize>().unwrap_or(lines.len())
    } else {
        lines.len()
    };

    let start = if count >= lines.len() {
        0
    } else {
        lines.len() - count
    };

    for (i, line) in lines.iter().enumerate().skip(start) {
        if crate::terminal::isatty() && crate::terminal::color::enabled() {
            use crate::terminal::color;
            println!("{}{:5}{}  {}", color::DIM, i + 1, color::RESET, line);
        } else {
            println!("{:5}  {}", i + 1, line);
        }
    }

    Ok(ExitStatus::Success(0))
}

/// Built-in: print shell variables
fn builtin_set(_cmd: &Command, state: &ShellState) -> Result<ExitStatus> {
    for (key, value) in &state.variables {
        println!("{}={}", key, value);
    }

    // Also show environment variables
    for (key, value) in env::vars() {
        if !state.variables.contains_key(&key) {
            println!("{}={}", key, value);
        }
    }

    Ok(ExitStatus::Success(0))
}

/// Built-in: list jobs (placeholder for now)
fn builtin_jobs(_cmd: &Command, _state: &ShellState) -> Result<ExitStatus> {
    println!("No jobs running");
    Ok(ExitStatus::Success(0))
}

/// Check if a variable is in the current environment
fn has_var_in_environ(name: &str) -> bool {
    unsafe {
        let name_eq = format!("{}=", name);
        let mut ptr = crate::environment::environ_ptr();
        while !ptr.is_null() && !(*ptr).is_null() {
            let var = std::ffi::CStr::from_ptr(*ptr).to_string_lossy();
            if var.starts_with(&name_eq) {
                return true;
            }
            ptr = ptr.add(1);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_state() {
        let mut state = ShellState::new().unwrap();
        state.set_var("TEST", "value");

        assert_eq!(state.get_var("TEST"), Some("value".to_string()));

        state.export_var("TEST", Some("value2".to_string())).unwrap();
        assert_eq!(env::var("TEST"), Ok("value2".to_string()));
    }

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("cd"));
        assert!(is_builtin("echo"));
        assert!(!is_builtin("ls"));
    }

    #[test]
    fn test_echo() {
        let mut state = ShellState::new().unwrap();
        let cmd = Command::new("echo".to_string());
        let result = execute_builtin(&cmd, &mut state);
        assert!(result.is_ok());
        assert!(result.unwrap().success());
    }

    #[test]
    fn test_pwd() {
        let mut state = ShellState::new().unwrap();
        let cmd = Command::new("pwd".to_string());
        let result = execute_builtin(&cmd, &mut state);
        assert!(result.is_ok());
        assert!(result.unwrap().success());
    }

    #[test]
    fn test_all_builtins_recognized() {
        let builtins = ["cd", "exit", "quit", "q", "pwd", "echo", "export",
            "unset", "env", "history", "set", "jobs", "fg", "bg"];
        for &b in &builtins {
            assert!(is_builtin(b), "{} should be a builtin", b);
        }
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("cat"));
        assert!(!is_builtin("grep"));
    }

    #[test]
    fn test_export_builtin() {
        let mut state = ShellState::new().unwrap();
        let cmd = Command {
            program: "export".to_string(),
            args: vec!["TEST_EXPORT=val123".to_string()],
            stdin: None, stdout: None, stderr: None, background: false,
        };
        let result = execute_builtin(&cmd, &mut state);
        assert!(result.is_ok());
        assert_eq!(state.get_var("TEST_EXPORT"), Some("val123".to_string()));
    }

    #[test]
    fn test_unset_builtin() {
        let mut state = ShellState::new().unwrap();
        state.set_var("TO_REMOVE", "yes");
        let cmd = Command {
            program: "unset".to_string(),
            args: vec!["TO_REMOVE".to_string()],
            stdin: None, stdout: None, stderr: None, background: false,
        };
        execute_builtin(&cmd, &mut state).unwrap();
        assert_eq!(state.get_var("TO_REMOVE"), None);
    }

    #[test]
    fn test_cd_builtin() {
        let mut state = ShellState::new().unwrap();
        let original_cwd = state.cwd.clone();
        let cmd = Command {
            program: "cd".to_string(),
            args: vec!["/tmp".to_string()],
            stdin: None, stdout: None, stderr: None, background: false,
        };
        let result = execute_builtin(&cmd, &mut state);
        assert!(result.is_ok());
        assert!(state.cwd.to_string_lossy().contains("tmp"));

        // Restore
        let back = Command {
            program: "cd".to_string(),
            args: vec![original_cwd.to_string_lossy().to_string()],
            stdin: None, stdout: None, stderr: None, background: false,
        };
        execute_builtin(&back, &mut state).unwrap();
    }

    #[test]
    fn test_cd_home() {
        let mut state = ShellState::new().unwrap();
        let cmd = Command {
            program: "cd".to_string(),
            args: vec![],
            stdin: None, stdout: None, stderr: None, background: false,
        };
        let result = execute_builtin(&cmd, &mut state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_state_var_operations() {
        let mut state = ShellState::new().unwrap();
        state.set_var("A", "1");
        state.set_var("B", "2");
        assert_eq!(state.get_var("A"), Some("1".to_string()));
        assert_eq!(state.get_var("B"), Some("2".to_string()));
        assert_eq!(state.get_var("C"), None);

        state.unset_var("A").unwrap();
        assert_eq!(state.get_var("A"), None);
    }

    #[test]
    fn test_exit_status_code() {
        assert_eq!(ExitStatus::Success(0).code(), 0);
        assert!(ExitStatus::Success(0).success());
        assert!(!ExitStatus::Failure(1).success());
        assert_eq!(ExitStatus::Failure(42).code(), 42);
    }

    #[test]
    fn test_builtin_not_found() {
        let mut state = ShellState::new().unwrap();
        let cmd = Command::new("not_a_builtin".to_string());
        let result = execute_builtin(&cmd, &mut state);
        assert!(result.is_err());
    }
}
