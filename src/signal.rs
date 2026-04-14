//! Signal handling for the Besh shell.
//!
//! Handles signals like SIGINT, SIGTSTP, SIGCHLD for proper process management.

use crate::error::{Result, ShellError};

static mut SIGNAL_RECEIVED: bool = false;

/// Signal types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    SigInt,  // Ctrl+C
    SigTstp, // Ctrl+Z
    SigChld, // Child process changed state
}

/// Signal action handler type
type SignalHandler = extern "C" fn(libc::c_int);

/// Setup signal handlers for the shell
pub fn setup_signal_handlers() -> Result<()> {
    unsafe {
        let mut sigint_action: libc::sigaction = std::mem::zeroed();
        libc::sigemptyset(&mut sigint_action.sa_mask);
        sigint_action.sa_flags = libc::SA_RESTART;
        sigint_action.sa_sigaction = sigint_handler as usize;

        if libc::sigaction(libc::SIGINT, &sigint_action, std::ptr::null_mut()) < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }

        let mut sigtstp_action: libc::sigaction = std::mem::zeroed();
        libc::sigemptyset(&mut sigtstp_action.sa_mask);
        sigtstp_action.sa_flags = libc::SA_RESTART;
        sigtstp_action.sa_sigaction = sigtstp_handler as usize;

        if libc::sigaction(libc::SIGTSTP, &sigtstp_action, std::ptr::null_mut()) < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }

        let mut sigchld_action: libc::sigaction = std::mem::zeroed();
        libc::sigemptyset(&mut sigchld_action.sa_mask);
        sigchld_action.sa_flags = libc::SA_RESTART | libc::SA_NOCLDSTOP;
        sigchld_action.sa_sigaction = sigchld_handler as usize;

        if libc::sigaction(libc::SIGCHLD, &sigchld_action, std::ptr::null_mut()) < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }
    }

    Ok(())
}

/// Signal handler for SIGINT (Ctrl+C)
extern "C" fn sigint_handler(_signum: libc::c_int) {
    unsafe {
        SIGNAL_RECEIVED = true;
        // Print newline and prompt indicator
        libc::write(libc::STDOUT_FILENO, b"^C\n".as_ptr() as *const libc::c_void, 3);
    }
}

/// Signal handler for SIGTSTP (Ctrl+Z)
extern "C" fn sigtstp_handler(_signum: libc::c_int) {
    unsafe {
        SIGNAL_RECEIVED = true;
        // Print newline and prompt indicator
        libc::write(libc::STDOUT_FILENO, b"^Z\n".as_ptr() as *const libc::c_void, 3);
    }
}

/// Signal handler for SIGCHLD
extern "C" fn sigchld_handler(_signum: libc::c_int) {
    // Just set the flag - don't reap here, let put_job_in_foreground handle it
    unsafe {
        SIGNAL_RECEIVED = true;
    }
}

/// Block signals for critical sections
pub fn block_signals() -> Result<libc::sigset_t> {
    let mut old_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    let mut block_mask: libc::sigset_t = unsafe { std::mem::zeroed() };

    unsafe {
        libc::sigemptyset(&mut block_mask);
        libc::sigaddset(&mut block_mask, libc::SIGINT);
        libc::sigaddset(&mut block_mask, libc::SIGTSTP);
        libc::sigaddset(&mut block_mask, libc::SIGCHLD);

        if libc::sigprocmask(libc::SIG_BLOCK, &block_mask, &mut old_mask) < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }
    }

    Ok(old_mask)
}

/// Unblock signals
pub fn unblock_signals(old_mask: &libc::sigset_t) -> Result<()> {
    unsafe {
        if libc::sigprocmask(libc::SIG_SETMASK, old_mask, std::ptr::null_mut()) < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

/// Check if a signal was received
pub fn was_signal_received() -> bool {
    unsafe {
        let old = SIGNAL_RECEIVED;
        SIGNAL_RECEIVED = false;
        old
    }
}

/// Temporarily block signals in a scope
pub struct SignalGuard {
    old_mask: Option<libc::sigset_t>,
}

impl SignalGuard {
    /// Create a new signal guard by blocking all shell signals
    pub fn new() -> Result<Self> {
        let old_mask = block_signals()?;
        Ok(SignalGuard {
            old_mask: Some(old_mask),
        })
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        if let Some(old_mask) = self.old_mask {
            let _ = unblock_signals(&old_mask);
        }
    }
}

/// Get the shell's process group ID
pub fn get_shell_pgid() -> libc::pid_t {
    unsafe { libc::getpgrp() }
}

/// Get the shell's process ID
pub fn get_shell_pid() -> libc::pid_t {
    unsafe { libc::getpid() }
}

/// Set terminal foreground process group
pub fn set_foreground_pgroup(fd: libc::c_int, pgrp: libc::pid_t) -> Result<()> {
    unsafe {
        if libc::tcsetpgrp(fd, pgrp) < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

/// Get terminal foreground process group
pub fn get_foreground_pgroup(fd: libc::c_int) -> Result<libc::pid_t> {
    unsafe {
        let pgrp = libc::tcgetpgrp(fd);
        if pgrp < 0 {
            return Err(ShellError::IoError(std::io::Error::last_os_error()));
        }
        Ok(pgrp)
    }
}

#[link(name = "c")]
extern "C" {
    fn tcgetpgrp(fd: libc::c_int) -> libc::pid_t;
    fn tcsetpgrp(fd: libc::c_int, pgrp: libc::pid_t) -> libc::c_int;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_setup() {
        let result = setup_signal_handlers();
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_unblock() {
        let old = block_signals().unwrap();
        let result = unblock_signals(&old);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_ids() {
        let pid = get_shell_pid();
        let pgid = get_shell_pgid();
        assert!(pid > 0);
        assert!(pgid > 0);
    }
}
