//! Job control for the Besh shell.
//!
//! Manages background and foreground jobs with process group control.

use crate::error::{Result, ShellError};
use crate::process::{Fd, Pipe};
use crate::signal::{set_foreground_pgroup, get_foreground_pgroup};

/// Job state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Stopped,
    Done,
    Terminated,
}

/// Represents a job (process group)
#[derive(Debug, Clone)]
pub struct Job {
    /// Process group ID
    pub pgid: libc::pid_t,
    /// Command string
    pub command: String,
    /// Job state
    pub state: JobState,
    /// Is background job
    #[allow(dead_code)]
    pub background: bool,
    /// Job number
    pub job_num: usize,
    /// Notified flag
    #[allow(dead_code)]
    pub notified: bool,
}

impl Job {
    /// Create a new job
    pub fn new(pgid: libc::pid_t, command: String, background: bool, job_num: usize) -> Self {
        Job {
            pgid,
            command,
            state: JobState::Running,
            background,
            job_num,
            notified: false,
        }
    }

    /// Get job ID string
    pub fn job_id(&self) -> String {
        format!("%{}", self.job_num)
    }

    /// Update job state from wait status
    pub fn update_state(&mut self, status: libc::c_int) {
        if libc::WIFEXITED(status) {
            self.state = JobState::Done;
        } else if libc::WIFSIGNALED(status) {
            self.state = JobState::Terminated;
        } else if libc::WIFSTOPPED(status) {
            self.state = JobState::Stopped;
        } else if libc::WIFCONTINUED(status) {
            self.state = JobState::Running;
        }
    }
}

/// Job control manager
#[derive(Debug)]
pub struct JobControl {
    /// List of jobs
    jobs: Vec<Job>,
    /// Next job number
    next_job_num: usize,
    /// Shell's process group ID
    shell_pgid: libc::pid_t,
    /// Terminal file descriptor
    terminal_fd: Fd,
}

impl JobControl {
    /// Create a new job control manager
    pub fn new() -> Result<Self> {
        let shell_pgid = unsafe { libc::getpgrp() };
        let terminal_fd = libc::STDIN_FILENO;

        Ok(JobControl {
            jobs: Vec::new(),
            next_job_num: 1,
            shell_pgid,
            terminal_fd,
        })
    }

    /// Add process to a job (set process group)
    #[allow(dead_code)]
    pub fn add_process_to_job(&self, pid: libc::pid_t, pgid: libc::pid_t) -> Result<()> {
        unsafe {
            if libc::setpgid(pid, pgid) < 0 {
                // May fail if process has already executed
                if std::io::Error::last_os_error().raw_os_error() != Some(libc::EPERM) {
                    return Err(ShellError::IoError(std::io::Error::last_os_error()));
                }
            }
        }
        Ok(())
    }

    /// Add a new job
    pub fn add_job(&mut self, pgid: libc::pid_t, command: String, background: bool) -> usize {
        let job = Job::new(pgid, command, background, self.next_job_num);
        let job_num = job.job_num;
        self.jobs.push(job);
        self.next_job_num += 1;
        job_num
    }

    /// Put job in foreground
    pub fn put_job_in_foreground(&mut self, pgid: libc::pid_t) -> Result<JobState> {
        // Give terminal control to the job
        set_foreground_pgroup(self.terminal_fd, pgid)?;

        // Wait for all processes in the job
        let mut last_status = 0i32;

        unsafe {
            loop {
                let mut status = 0;
                let pid = libc::waitpid(-pgid, &mut status, libc::WUNTRACED);

                if pid < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() != Some(libc::ECHILD) {
                        return Err(ShellError::IoError(err));
                    }
                    break;
                }

                last_status = status;

                if libc::WIFEXITED(status) || libc::WIFSIGNALED(status) {
                    // Process completed
                } else if libc::WIFSTOPPED(status) {
                    // Process was stopped
                    if let Some(_job) = self.find_job_by_pgid(pgid) {
                        self.update_job_state(pgid, status);
                        return Ok(JobState::Stopped);
                    }
                }

                // Break if no more children
                if libc::WIFEXITED(status) || libc::WIFSIGNALED(status) {
                    // Wait for all processes in the group
                    // We'll continue until we get ECHILD
                }
            }
        }

        // Restore terminal control to shell
        let _result = set_foreground_pgroup(self.terminal_fd, self.shell_pgid);

        if let Some(job) = self.find_job_by_pgid(pgid) {
            let job_state = job.state;
            if last_status != 0 {
                self.update_job_state(pgid, last_status);
            }
            Ok(job_state)
        } else {
            let state = if libc::WIFEXITED(last_status) {
                JobState::Done
            } else if libc::WIFSIGNALED(last_status) {
                JobState::Terminated
            } else {
                JobState::Running
            };
            Ok(state)
        }
    }

    /// Put job in background
    pub fn put_job_in_background(&self, pgid: libc::pid_t) -> Result<()> {
        // Continue the job
        unsafe {
            if libc::kill(-pgid, libc::SIGCONT) < 0 {
                return Err(ShellError::IoError(std::io::Error::last_os_error()));
            }
        }
        Ok(())
    }

    /// Find job by process group ID
    pub fn find_job_by_pgid(&self, pgid: libc::pid_t) -> Option<&Job> {
        self.jobs.iter().find(|j| j.pgid == pgid)
    }

    /// Find job by job number
    #[allow(dead_code)]
    pub fn find_job_by_num(&mut self, job_num: usize) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.job_num == job_num)
    }

    /// Update job state
    pub fn update_job_state(&mut self, pgid: libc::pid_t, status: libc::c_int) {
        if let Some(job) = self.find_job_by_pgid_mut(pgid) {
            job.update_state(status);
        }
    }

    fn find_job_by_pgid_mut(&mut self, pgid: libc::pid_t) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.pgid == pgid)
    }

    /// Remove completed jobs
    pub fn cleanup_jobs(&mut self) {
        self.jobs.retain(|j| !matches!(j.state, JobState::Done | JobState::Terminated));
        // Renumber jobs
        self.renumber_jobs();
    }

    fn renumber_jobs(&mut self) {
        self.next_job_num = 1;
        for (i, job) in self.jobs.iter_mut().enumerate() {
            job.job_num = i + 1;
        }
        self.next_job_num = self.jobs.len() + 1;
    }

    /// List jobs
    #[allow(dead_code)]
    pub fn list_jobs(&self) -> Vec<&Job> {
        self.jobs.iter().collect()
    }

    /// Get shell's process group
    #[allow(dead_code)]
    pub fn shell_pgid(&self) -> libc::pid_t {
        self.shell_pgid
    }

    /// Check if terminal has foreground control
    #[allow(dead_code)]
    pub fn has_terminal_control(&mut self) -> bool {
        match get_foreground_pgroup(self.terminal_fd) {
            Ok(pgrp) => pgrp == self.shell_pgid,
            Err(_) => false,
        }
    }

    /// Wait for any job to complete or change state (non-blocking via WNOHANG)
    #[allow(dead_code)]
    pub fn check_jobs(&mut self) -> Vec<(libc::pid_t, libc::c_int)> {
        let mut results = Vec::new();

        unsafe {
            let mut status = 0;
            let pid = libc::waitpid(-1, &mut status, libc::WNOHANG);

            if pid > 0 {
                results.push((pid, status));
            }
        }

        results
    }

    /// Create pipes for a pipeline
    pub fn create_pipes(&self, num_commands: usize) -> Result<Vec<Pipe>> {
        if num_commands <= 1 {
            return Ok(Vec::new());
        }

        let mut pipes = Vec::with_capacity(num_commands - 1);
        for _ in 0..num_commands - 1 {
            pipes.push(Pipe::new()?);
        }
        Ok(pipes)
    }
}

impl Default for JobControl {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_control_creation() {
        let jc = JobControl::new().unwrap();
        assert_eq!(jc.jobs.len(), 0);
        assert!(jc.shell_pgid() > 0);
    }

    #[test]
    fn test_add_job() {
        let mut jc = JobControl::new().unwrap();
        let job_num = jc.add_job(123, "test command".to_string(), true);
        assert_eq!(job_num, 1);
        assert_eq!(jc.jobs.len(), 1);
    }

    #[test]
    fn test_find_job() {
        let mut jc = JobControl::new().unwrap();
        jc.add_job(123, "test command".to_string(), false);
        assert!(jc.find_job_by_pgid(123).is_some());
        assert!(jc.find_job_by_pgid(456).is_none());
    }
}
