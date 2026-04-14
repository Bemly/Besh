//! Error types for the Besh shell.
//!
//! Defines `ShellError` enum for all shell errors with proper error messages.

use std::fmt;
use std::io;

/// Shell error types
#[derive(Debug)]
pub enum ShellError {
    /// Command not found
    CommandNotFound(String),
    /// IO error
    IoError(io::Error),
    /// Parse error with message
    ParseError(String),
    /// Signal error with message
    SignalError(String),
    /// Job control error
    JobError(String),
    /// Variable error
    VariableError(String),
    /// Already exists
    AlreadyExists(String),
    /// Not found
    NotFound(String),
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellError::CommandNotFound(cmd) => write!(f, "besh: command not found: {}", cmd),
            ShellError::IoError(err) => write!(f, "besh: io error: {}", err),
            ShellError::ParseError(msg) => write!(f, "besh: parse error: {}", msg),
            ShellError::SignalError(msg) => write!(f, "besh: signal error: {}", msg),
            ShellError::JobError(msg) => write!(f, "besh: job error: {}", msg),
            ShellError::VariableError(msg) => write!(f, "besh: variable error: {}", msg),
            ShellError::AlreadyExists(what) => write!(f, "besh: already exists: {}", what),
            ShellError::NotFound(what) => write!(f, "besh: not found: {}", what),
        }
    }
}

impl std::error::Error for ShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ShellError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for ShellError {
    fn from(err: io::Error) -> Self {
        ShellError::IoError(err)
    }
}

impl From<String> for ShellError {
    fn from(msg: String) -> Self {
        ShellError::ParseError(msg)
    }
}

impl From<&str> for ShellError {
    fn from(msg: &str) -> Self {
        ShellError::ParseError(msg.to_string())
    }
}

/// Result type for shell operations
pub type Result<T> = std::result::Result<T, ShellError>;

// Legacy error constants (deprecated but kept for compatibility)
#[deprecated(note = "Use ShellError instead")]
pub const _FAIL_REFRESH: &str = "a failed refresh 输出流刷新失败";
#[deprecated(note = "Use ShellError instead")]
pub const _FAIL_READ_LINE: &str = "failed to read line. 读取输入失败";
#[deprecated(note = "Use ShellError instead")]
pub const _NOT_GET_PROGRAM_NAME: &str = "failed to get program name. 获取程序名失败";
#[deprecated(note = "Use ShellError instead")]
pub const _FAIL_STR_TRANFORM: &str = "Failed to convert output to string. 输出转换为字符串失败";
#[deprecated(note = "Use ShellError instead")]
pub const _NOT_FIND_CRR_DIR: &str = "failed to find current directory. 获取当前目录失败";
