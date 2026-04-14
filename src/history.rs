//! Command history for the Besh shell.
//!
//! Stores and retrieves command history with file persistence.

use crate::error::{Result, ShellError};
use dirs::home_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Command history storage
#[derive(Debug)]
pub struct History {
    entries: Vec<String>,
    max_size: usize,
    history_file: PathBuf,
    current_index: Option<usize>,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// Create a new history manager
    pub fn new() -> Self {
        let history_file = home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".besh_history");

        History {
            entries: Vec::new(),
            max_size: 1000,
            history_file,
            current_index: None,
        }
    }

    /// Load history from file
    pub fn load(&mut self) -> Result<()> {
        self.entries.clear();

        if !self.history_file.exists() {
            return Ok(());
        }

        let file = File::open(&self.history_file)
            .map_err(|e| ShellError::IoError(e))?;

        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .filter_map(|l| l.ok())
            .collect();

        // Keep only the last max_size entries
        if lines.len() > self.max_size {
            let start = lines.len() - self.max_size;
            self.entries = lines[start..].to_vec();
        } else {
            self.entries = lines;
        }

        Ok(())
    }

    /// Save history to file
    pub fn save(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.history_file)
            .map_err(|e| ShellError::IoError(e))?;

        for entry in &self.entries {
            writeln!(file, "{}", entry)
                .map_err(|e| ShellError::IoError(e))?;
        }

        Ok(())
    }

    /// Add a command to history
    pub fn add(&mut self, command: String) {
        let trimmed = command.trim();

        // Don't add empty commands or duplicates of the last command
        if trimmed.is_empty() {
            return;
        }

        if let Some(last) = self.entries.last() {
            if last == trimmed {
                return;
            }
        }

        self.entries.push(trimmed.to_string());

        // Trim to max size
        if self.entries.len() > self.max_size {
            self.entries.drain(0..(self.entries.len() - self.max_size));
        }

        self.current_index = None;
    }

    /// Get history entry by index (positive or negative)
    pub fn get(&self, index: Option<i32>) -> Option<String> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        let idx = match index {
            Some(i) => {
                if i >= 0 {
                    i as usize
                } else {
                    (len as i32 + i) as usize
                }
            }
            None => len - 1,
        };

        if idx < len {
            self.entries.get(idx).cloned()
        } else {
            None
        }
    }

    /// Get the next history entry (for arrow up/down)
    pub fn next(&mut self) -> Option<String> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        self.current_index = match self.current_index {
            None => Some(len - 1),
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(0)
                }
            }
        };

        self.get(Some(self.current_index.unwrap() as i32))
    }

    /// Get the previous history entry (for arrow down)
    pub fn prev(&mut self) -> Option<String> {
        let len = self.entries.len();

        match self.current_index {
            None => None,
            Some(i) => {
                if i + 1 < len {
                    self.current_index = Some(i + 1);
                    self.get(Some(self.current_index.unwrap() as i32))
                } else {
                    self.current_index = None;
                    None
                }
            }
        }
    }

    /// Get all history entries
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_index = None;
    }

    /// Get history file path
    pub fn file(&self) -> &PathBuf {
        &self.history_file
    }
}

/// Save history on drop
impl Drop for History {
    fn drop(&mut self) {
        let _ = self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_add() {
        let mut hist = History::new();
        hist.add("echo hello".to_string());
        hist.add("ls -la".to_string());

        assert_eq!(hist.entries().len(), 2);
        assert_eq!(hist.get(None), Some("ls -la".to_string()));
    }

    #[test]
    fn test_history_navigation() {
        let mut hist = History::new();
        hist.add("cmd1".to_string());
        hist.add("cmd2".to_string());
        hist.add("cmd3".to_string());

        assert_eq!(hist.next(), Some("cmd3".to_string()));
        assert_eq!(hist.next(), Some("cmd2".to_string()));
        assert_eq!(hist.next(), Some("cmd1".to_string()));
        assert_eq!(hist.next(), Some("cmd1".to_string())); // Stays at top

        assert_eq!(hist.prev(), Some("cmd2".to_string()));
        assert_eq!(hist.prev(), Some("cmd3".to_string()));
        assert_eq!(hist.prev(), None);
    }

    #[test]
    fn test_no_duplicates() {
        let mut hist = History::new();
        hist.add("echo".to_string());
        hist.add("echo".to_string());
        hist.add("ls".to_string());

        assert_eq!(hist.entries().len(), 2);
        assert_eq!(hist.get(None), Some("ls".to_string()));
    }

    #[test]
    fn test_empty_command_ignored() {
        let mut hist = History::new();
        hist.add("".to_string());
        hist.add("   ".to_string());
        assert_eq!(hist.entries().len(), 0);
    }

    #[test]
    fn test_get_by_index() {
        let mut hist = History::new();
        hist.add("a".to_string());
        hist.add("b".to_string());
        hist.add("c".to_string());

        assert_eq!(hist.get(Some(0)), Some("a".to_string()));
        assert_eq!(hist.get(Some(2)), Some("c".to_string()));
        assert_eq!(hist.get(Some(10)), None);
        // Negative index: -1 = last, -2 = second to last
        assert_eq!(hist.get(Some(-1)), Some("c".to_string()));
        assert_eq!(hist.get(Some(-2)), Some("b".to_string()));
    }

    #[test]
    fn test_clear_history() {
        let mut hist = History::new();
        hist.add("a".to_string());
        hist.add("b".to_string());
        hist.clear();
        assert_eq!(hist.entries().len(), 0);
        assert_eq!(hist.next(), None);
    }

    #[test]
    fn test_history_file_path() {
        let hist = History::new();
        assert!(hist.file().ends_with(".besh_history"));
    }

    #[test]
    fn test_save_and_load() {
        let mut hist = History::new();
        // Use a unique file to avoid race conditions
        let unique_file = std::env::temp_dir().join(format!("besh_test_hist_{}", std::process::id()));
        hist.history_file = unique_file.clone();
        hist.add("saved_cmd".to_string());
        hist.save().unwrap();

        let mut hist2 = History::new();
        hist2.history_file = unique_file.clone();
        hist2.load().unwrap();
        assert!(hist2.entries().contains(&"saved_cmd".to_string()));

        // Cleanup
        let _ = std::fs::remove_file(&unique_file);
    }
}
