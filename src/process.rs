//! Process management using libc fork/exec/wait.
//!
//! Provides Process and ProcessBuilder for spawning and managing processes.

use crate::error::{Result, ShellError};
use std::ffi::CString;
use std::io;
use std::os::unix::io::RawFd;

/// File descriptors for standard streams
pub type Fd = libc::c_int;

/// Pipe descriptors
#[derive(Debug)]
pub struct Pipe {
    read_fd: Fd,
    write_fd: Fd,
}

impl Pipe {
    /// Create a new pipe
    pub fn new() -> Result<Self> {
        let mut fds = [-1 as Fd; 2];

        unsafe {
            if libc::pipe(fds.as_mut_ptr()) < 0 {
                return Err(ShellError::IoError(io::Error::last_os_error()));
            }
        }

        Ok(Pipe {
            read_fd: fds[0],
            write_fd: fds[1],
        })
    }

    /// Get the read end of the pipe
    pub fn read_fd(&self) -> Fd {
        self.read_fd
    }

    /// Get the write end of the pipe
    pub fn write_fd(&self) -> Fd {
        self.write_fd
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            if self.read_fd >= 0 {
                libc::close(self.read_fd);
            }
            if self.write_fd >= 0 {
                libc::close(self.write_fd);
            }
        }
    }
}

/// Exit status of a process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Process exited with a code
    Exited(u8),
    /// Process was terminated by a signal
    Signaled(i32),
    /// Process was stopped
    Stopped(i32),
    /// Process continued
    Continued,
}

impl ExitStatus {
    /// Returns true if the process exited successfully
    pub fn success(&self) -> bool {
        matches!(self, ExitStatus::Exited(0))
    }

    /// Returns the exit code if the process exited normally
    pub fn code(&self) -> Option<u8> {
        match self {
            ExitStatus::Exited(c) => Some(*c),
            _ => None,
        }
    }
}

/// Represents a child process
#[derive(Debug)]
pub struct Process {
    pid: libc::pid_t,
}

impl Process {
    /// Get the process ID
    pub fn pid(&self) -> libc::pid_t {
        self.pid
    }

    /// Wait for the process to complete and get its exit status
    pub fn wait(&self) -> Result<ExitStatus> {
        self.wait_with_options(0)
    }

    /// Wait for the process with WNOHANG option (non-blocking)
    pub fn try_wait(&self) -> Result<Option<ExitStatus>> {
        self.wait_with_options(libc::WNOHANG)
            .map(|s| if s == ExitStatus::Continued { None } else { Some(s) })
    }

    fn wait_with_options(&self, options: libc::c_int) -> Result<ExitStatus> {
        let mut status: libc::c_int = 0;

        unsafe {
            let result = libc::waitpid(self.pid, &mut status, options);

            if result < 0 {
                return Err(ShellError::IoError(io::Error::last_os_error()));
            }

            if result == 0 && (options & libc::WNOHANG) != 0 {
                return Ok(ExitStatus::Continued);
            }

            if libc::WIFEXITED(status) {
                Ok(ExitStatus::Exited(libc::WEXITSTATUS(status) as u8))
            } else if libc::WIFSIGNALED(status) {
                Ok(ExitStatus::Signaled(libc::WTERMSIG(status)))
            } else if libc::WIFSTOPPED(status) {
                Ok(ExitStatus::Stopped(libc::WSTOPSIG(status)))
            } else if libc::WIFCONTINUED(status) {
                Ok(ExitStatus::Continued)
            } else {
                Err(ShellError::JobError("Unknown process status".to_string()))
            }
        }
    }

    /// Kill the process with a signal
    pub fn kill(&self, signal: libc::c_int) -> Result<()> {
        unsafe {
            if libc::kill(self.pid, signal) < 0 {
                return Err(ShellError::IoError(io::Error::last_os_error()));
            }
        }
        Ok(())
    }

    /// Send SIGCONT to continue a stopped process
    pub fn continue_(&self) -> Result<()> {
        self.kill(libc::SIGCONT)
    }
}

/// builder for creating and spawning processes
#[derive(Debug, Clone)]
pub struct ProcessBuilder {
    program: String,
    args: Vec<String>,
    stdin: Option<Redirection>,
    stdout: Option<Redirection>,
    stderr: Option<Redirection>,
}

/// Type of redirection for a stream
#[derive(Debug, Clone, PartialEq)]
pub enum Redirection {
    /// Redirect from/to a file path
    File(String),
    /// Redirect from/to a file descriptor
    Fd(Fd),
    /// Redirect to a pipe (read or write end)
    Pipe(Fd),
    /// No change (use default)
    Inherit,
}

impl ProcessBuilder {
    /// Create a new ProcessBuilder for the given program
    pub fn new<S: Into<String>>(program: S) -> Self {
        ProcessBuilder {
            program: program.into(),
            args: Vec::new(),
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    /// Add an argument to the command
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments to the command
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self.args.push(arg.into());
        }
        self
    }

    /// Add arguments by reference
    pub fn args_ref<'a, I>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = &'a String>,
    {
        for arg in args {
            self.args.push(arg.clone());
        }
        self
    }

    /// Set stdin redirection
    pub fn stdin(mut self, redir: Redirection) -> Self {
        self.stdin = Some(redir);
        self
    }

    /// Set stdout redirection
    pub fn stdout(mut self, redir: Redirection) -> Self {
        self.stdout = Some(redir);
        self
    }

    /// Set stderr redirection
    pub fn stderr(mut self, redir: Redirection) -> Self {
        self.stderr = Some(redir);
        self
    }

    /// Spawn the process (fork/exec)
    pub fn spawn(self) -> Result<Process> {
        let c_program = CString::new(self.program.clone())
            .map_err(|_| ShellError::ParseError("Invalid program name".to_string()))?;

        // Convert arguments to C strings
        let mut c_args: Vec<CString> = self.args
            .iter()
            .map(|arg| {
                CString::new(arg.as_str())
                    .map_err(|_| ShellError::ParseError("Invalid argument".to_string()))
            })
            .collect::<Result<Vec<_>>>()?;

        // Create argv array (program first, then args, then null)
        let mut argv: Vec<*mut libc::c_char> = c_args
            .iter()
            .map(|s| s.as_ptr() as *mut libc::c_char)
            .collect();

        // Insert program name at the start
        argv.insert(0, c_program.as_ptr() as *mut libc::c_char);

        unsafe {
            let pid = libc::fork();

            if pid < 0 {
                return Err(ShellError::IoError(io::Error::last_os_error()));
            }

            if pid == 0 {
                // Child process
                if let Err(e) = self.set_up_redirections() {
                    eprintln!("besh: {}", e);
                    libc::_exit(127);
                }

                // Execute the program
                libc::execvp(c_program.as_ptr(), argv.as_ptr() as *const *const libc::c_char);

                // execvp only returns on error
                eprintln!("besh: {}: command not found", self.program);
                libc::_exit(127);
            }

            // Parent process
            Ok(Process { pid })
        }
    }

    fn set_up_redirections(&self) -> Result<()> {
        // Handle stdin
        if let Some(ref redir) = self.stdin {
            match redir {
                Redirection::Fd(fd) => redirect_fd(*fd, libc::STDIN_FILENO)?,
                Redirection::Pipe(fd) => redirect_fd(*fd, libc::STDIN_FILENO)?,
                Redirection::File(path) => {
                    let c_path = CString::new(path.as_str())
                        .map_err(|_| ShellError::ParseError("Invalid path".to_string()))?;
                    unsafe {
                        let fd = libc::open(c_path.as_ptr(), libc::O_RDONLY, 0);
                        if fd < 0 {
                            return Err(ShellError::IoError(io::Error::last_os_error()));
                        }
                        redirect_fd(fd, libc::STDIN_FILENO)?;
                        libc::close(fd);
                    }
                }
                Redirection::Inherit => {}
            }
        }

        // Handle stdout
        if let Some(ref redir) = self.stdout {
            match redir {
                Redirection::Fd(fd) => redirect_fd(*fd, libc::STDOUT_FILENO)?,
                Redirection::Pipe(fd) => redirect_fd(*fd, libc::STDOUT_FILENO)?,
                Redirection::File(path) => {
                    let c_path = CString::new(path.as_str())
                        .map_err(|_| ShellError::ParseError("Invalid path".to_string()))?;
                    unsafe {
                        let fd = libc::open(
                            c_path.as_ptr(),
                            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                            0o644,
                        );
                        if fd < 0 {
                            return Err(ShellError::IoError(io::Error::last_os_error()));
                        }
                        redirect_fd(fd, libc::STDOUT_FILENO)?;
                        libc::close(fd);
                    }
                }
                Redirection::Inherit => {}
            }
        }

        // Handle stderr
        if let Some(ref redir) = self.stderr {
            match redir {
                Redirection::Fd(fd) => redirect_fd(*fd, libc::STDERR_FILENO)?,
                Redirection::Pipe(fd) => redirect_fd(*fd, libc::STDERR_FILENO)?,
                Redirection::File(path) => {
                    let c_path = CString::new(path.as_str())
                        .map_err(|_| ShellError::ParseError("Invalid path".to_string()))?;
                    unsafe {
                        let fd = libc::open(
                            c_path.as_ptr(),
                            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                            0o644,
                        );
                        if fd < 0 {
                            return Err(ShellError::IoError(io::Error::last_os_error()));
                        }
                        redirect_fd(fd, libc::STDERR_FILENO)?;
                        libc::close(fd);
                    }
                }
                Redirection::Inherit => {}
            }
        }

        Ok(())
    }
}

/// Redirect a file descriptor to another
fn redirect_fd(from: Fd, to: Fd) -> Result<()> {
    unsafe {
        if libc::dup2(from, to) < 0 {
            return Err(ShellError::IoError(io::Error::last_os_error()));
        }
    }
    Ok(())
}

/// Close all file descriptors greater than 2
pub fn close_all_fds_except(except: &[Fd]) -> Result<()> {
    unsafe {
        let mut fd = 3;
        loop {
            // Use getrlimit to find max fd
            let mut rlimit = std::mem::zeroed::<libc::rlimit>();
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlimit) < 0 {
                return Err(ShellError::IoError(io::Error::last_os_error()));
            }

            let max_fd = if rlimit.rlim_cur == libc::RLIM_INFINITY {
                1024
            } else {
                rlimit.rlim_cur as Fd
            };

            while fd <= max_fd {
                if !except.contains(&fd) {
                    libc::close(fd);
                }
                fd += 1;
            }
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_simple() {
        let builder = ProcessBuilder::new("echo").arg("hello");

        let process = builder.spawn().unwrap();
        let status = process.wait().unwrap();

        assert!(status.success());
    }

    #[test]
    fn test_pipe_creation() {
        let pipe = Pipe::new().unwrap();
        assert!(pipe.read_fd() >= 0);
        assert!(pipe.write_fd() >= 0);
    }

    #[test]
    fn test_exit_status() {
        assert!(ExitStatus::Exited(0).success());
        assert!(!ExitStatus::Exited(1).success());
        assert_eq!(ExitStatus::Exited(42).code(), Some(42));
        assert_eq!(ExitStatus::Signaled(9).code(), None);
    }
}
