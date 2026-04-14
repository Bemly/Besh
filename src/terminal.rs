//! Terminal handling for the Besh shell.
//!
//! Provides raw mode terminal handling, character input, and line editing.

use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;

pub use libc::{c_int, ECHO, ICANON, ISIG, TCSANOW, VMIN, VTIME, termios};

/// Terminal control for raw mode and character input
pub struct Terminal {
    original_termios: Option<termios>,
    is_raw: bool,
}

impl Terminal {
    /// Create a new terminal controller
    pub fn new() -> io::Result<Self> {
        Ok(Terminal {
            original_termios: None,
            is_raw: false,
        })
    }

    /// Get the current terminal attributes
    fn get_termios() -> io::Result<termios> {
        unsafe {
            let mut termios = std::mem::zeroed();
            if tcgetattr(libc::STDIN_FILENO, &mut termios) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(termios)
        }
    }

    /// Set raw mode - disables canonical mode and echo
    pub fn set_raw_mode(&mut self) -> io::Result<()> {
        if self.is_raw {
            return Ok(());
        }

        let original = Self::get_termios()?;

        if self.original_termios.is_none() {
            self.original_termios = Some(original);
        }

        let mut raw = original;

        // Disable canonical mode and echo
        raw.c_lflag &= !(ICANON | ECHO | ISIG);

        // Set minimum characters to 1, timeout to 0 (non-blocking)
        raw.c_cc[VMIN] = 1;
        raw.c_cc[VTIME] = 0;

        unsafe {
            if tcsetattr(libc::STDIN_FILENO, TCSANOW, &raw) < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        self.is_raw = true;
        Ok(())
    }

    /// Restore original terminal mode
    pub fn restore_mode(&mut self) -> io::Result<()> {
        if !self.is_raw {
            return Ok(());
        }

        if let Some(original) = self.original_termios {
            unsafe {
                if tcsetattr(libc::STDIN_FILENO, TCSANOW, &original) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
        }

        self.is_raw = false;
        Ok(())
    }

    /// Read a single character
    pub fn read_char(&self) -> io::Result<char> {
        let mut buffer = [0u8; 1];

        unsafe {
            // Non-blocking read
            if libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr() as *mut libc::c_void, 1) < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(buffer[0] as char)
    }

    /// Check if there's input available (non-blocking)
    pub fn has_input(&self) -> io::Result<bool> {
        unsafe {
            let mut read_fds = std::mem::zeroed::<libc::fd_set>();
            libc::FD_SET(libc::STDIN_FILENO, &mut read_fds);

            let mut timeout = libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            };

            let result = libc::select(
                libc::STDIN_FILENO + 1,
                &mut read_fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut timeout,
            );

            Ok(result > 0)
        }
    }

    /// Read a line with editing, history navigation, and tab completion
    pub fn read_line(&mut self, history: &mut crate::history::History) -> io::Result<String> {
        // Enter raw mode for character-by-character input
        self.set_raw_mode()?;

        let mut line = String::new();
        let mut cursor: usize = 0;
        let mut history_mode = false;
        let mut saved_line = String::new();

        loop {
            let c = self.read_char()?;

            match c {
                '\n' | '\r' => {
                    // Enter - submit line
                    println!();
                    self.restore_mode()?;
                    return Ok(line);
                }
                '\x08' | '\x7f' => {
                    // Backspace
                    if cursor > 0 {
                        line.remove(cursor - 1);
                        cursor -= 1;
                        redraw_line(&line, cursor)?;
                    }
                }
                '\x03' => {
                    // Ctrl+C - clear line
                    println!();
                    self.restore_mode()?;
                    return Ok(String::new());
                }
                '\x04' => {
                    // Ctrl+D (EOF)
                    if line.is_empty() {
                        self.restore_mode()?;
                        return Ok(String::new());
                    }
                }
                '\x01' => {
                    // Ctrl+A - move to beginning
                    cursor = 0;
                    redraw_line(&line, cursor)?;
                }
                '\x05' => {
                    // Ctrl+E - move to end
                    cursor = line.len();
                    redraw_line(&line, cursor)?;
                }
                '\x0b' => {
                    // Ctrl+K - kill to end of line
                    line.truncate(cursor);
                    redraw_line(&line, cursor)?;
                }
                '\x15' => {
                    // Ctrl+U - kill to beginning of line
                    line.drain(0..cursor);
                    cursor = 0;
                    redraw_line(&line, cursor)?;
                }
                '\x1b' => {
                    // Escape sequence - read [ then the key
                    let next = self.read_char()?;
                    if next == '[' {
                        let key = self.read_char()?;
                        match key {
                            'A' => {
                                // Up arrow - previous history entry
                                if !history_mode {
                                    saved_line = line.clone();
                                    history_mode = true;
                                }
                                if let Some(entry) = history.next() {
                                    line = entry;
                                    cursor = line.len();
                                    redraw_line(&line, cursor)?;
                                }
                            }
                            'B' => {
                                // Down arrow - next history entry
                                if history_mode {
                                    if let Some(entry) = history.prev() {
                                        line = entry;
                                        cursor = line.len();
                                        redraw_line(&line, cursor)?;
                                    } else {
                                        // Back to saved line
                                        line = saved_line.clone();
                                        cursor = line.len();
                                        history_mode = false;
                                        redraw_line(&line, cursor)?;
                                    }
                                }
                            }
                            'C' => {
                                // Right arrow
                                if cursor < line.len() {
                                    cursor += 1;
                                    print!("\x1b[C");
                                    io::stdout().flush()?;
                                }
                            }
                            'D' => {
                                // Left arrow
                                if cursor > 0 {
                                    cursor -= 1;
                                    print!("\x1b[D");
                                    io::stdout().flush()?;
                                }
                            }
                            _ => {}
                        }
                    } else if next == 'O' {
                        let key = self.read_char()?;
                        match key {
                            'H' => {
                                // Home key
                                cursor = 0;
                                redraw_line(&line, cursor)?;
                            }
                            'F' => {
                                // End key
                                cursor = line.len();
                                redraw_line(&line, cursor)?;
                            }
                            _ => {}
                        }
                    }
                }
                '\t' => {
                    // Tab completion
                    let completions = complete(&line, cursor);
                    if completions.len() == 1 {
                        // Single match - insert it
                        let replacement = &completions[0];
                        // Find the start of the current word
                        let word_start = line[..cursor].rfind(|c: char| c.is_whitespace()).map_or(0, |i| i + 1);
                        line.replace_range(word_start..cursor, replacement);
                        cursor = word_start + replacement.len();
                        // Add space after completion
                        line.insert(cursor, ' ');
                        cursor += 1;
                        redraw_line(&line, cursor)?;
                    } else if completions.len() > 1 {
                        // Multiple matches - show them and find common prefix
                        println!();
                        for comp in &completions {
                            print!("{}  ", comp);
                        }
                        println!();

                        // Find common prefix
                        if let Some(first) = completions.first() {
                            let mut prefix_len = first.len();
                            for comp in &completions[1..] {
                                prefix_len = prefix_len.min(comp.len());
                                for i in 0..prefix_len {
                                    if first.as_bytes()[i] != comp.as_bytes()[i] {
                                        prefix_len = i;
                                        break;
                                    }
                                }
                            }
                            if prefix_len > 0 {
                                let word_start = line[..cursor].rfind(|c: char| c.is_whitespace()).map_or(0, |i| i + 1);
                                let current_word_len = cursor - word_start;
                                if prefix_len > current_word_len {
                                    let prefix = &first[..prefix_len];
                                    line.replace_range(word_start..cursor, prefix);
                                    cursor = word_start + prefix_len;
                                }
                            }
                        }
                        redraw_line(&line, cursor)?;
                    }
                }
                _ if c.is_ascii_graphic() || c == ' ' => {
                    // Regular character - insert at cursor
                    line.insert(cursor, c);
                    cursor += 1;
                    history_mode = false;
                    redraw_line(&line, cursor)?;
                }
                _ => {}
            }
        }
    }

    /// Is terminal in raw mode?
    pub fn is_raw(&self) -> bool {
        self.is_raw
    }
}

/// Redraw the current line, positioning the cursor correctly
fn redraw_line(line: &str, cursor: usize) -> io::Result<()> {
    // Move to start of line (carriage return)
    print!("\r");
    // Clear from cursor to end of line
    print!("\x1b[K");
    // Print the line
    print!("{}", line);
    // Move cursor back to correct position
    if cursor < line.len() {
        let move_back = line.len() - cursor;
        print!("\x1b[{}D", move_back);
    }
    io::stdout().flush()
}

/// Complete a word at the cursor position
/// Returns a list of possible completions
fn complete(line: &str, cursor: usize) -> Vec<String> {
    // Find the current word being typed
    let word_start = line[..cursor].rfind(|c: char| c.is_whitespace()).map_or(0, |i| i + 1);
    let word = &line[word_start..cursor];

    if word.is_empty() {
        return Vec::new();
    }

    let is_first_word = line[..word_start].trim().is_empty();
    let mut completions = Vec::new();

    if is_first_word {
        // Command completion: builtins + PATH executables
        let builtins = ["cd", "exit", "quit", "q", "pwd", "echo", "export",
            "unset", "env", "history", "set", "jobs", "fg", "bg", "source"];

        for &b in &builtins {
            if b.starts_with(word) && b != word {
                completions.push(b.to_string());
            }
        }

        // Search PATH for matching executables
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with(word) && name != word {
                                if !completions.contains(&name.to_string()) {
                                    completions.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // File/directory completion (always available)
    let (dir_prefix, file_prefix) = if let Some(pos) = word.rfind('/') {
        let dir = &word[..pos + 1];
        let file = &word[pos + 1..];
        let expanded = if dir.starts_with('~') {
            let home = dirs::home_dir().unwrap_or_default();
            dir.replacen('~', &home.to_string_lossy(), 1)
        } else {
            dir.to_string()
        };
        (expanded, file.to_string())
    } else {
        (".".to_string(), word.to_string())
    };

    if let Ok(entries) = std::fs::read_dir(&dir_prefix) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&file_prefix) && name != file_prefix {
                    let path = if dir_prefix == "." {
                        name.to_string()
                    } else {
                        format!("{}{}", dir_prefix, name)
                    };
                    // Add trailing slash for directories
                    let full_path = if dir_prefix == "." {
                        std::path::PathBuf::from(name)
                    } else {
                        std::path::PathBuf::from(&dir_prefix).join(name)
                    };
                    let display = if full_path.is_dir() {
                        format!("{}/", path)
                    } else {
                        path
                    };
                    if !completions.contains(&display) {
                        completions.push(display);
                    }
                }
            }
        }
    }

    completions.sort();
    completions
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore_mode();
    }
}

/// ANSI color codes for terminal output
pub mod color {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    // Foreground colors
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    // Bright foreground colors
    pub const BRIGHT_BLACK: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";

    /// Check if color output should be used (respects NO_COLOR env var)
    pub fn enabled() -> bool {
        std::env::var("NO_COLOR").is_err()
    }
}

/// Check if stdout is a terminal
pub fn isatty() -> bool {
    unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
}

/// Get terminal size
pub fn terminal_size() -> Option<(usize, usize)> {
    unsafe {
        let mut winsize = std::mem::zeroed::<libc::winsize>();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) < 0 {
            return None;
        }
        Some((
            winsize.ws_col as usize,
            winsize.ws_row as usize,
        ))
    }
}

/// External libc functions
#[link(name = "c")]
extern "C" {
    fn tcgetattr(fd: c_int, termios_p: *mut termios) -> c_int;
    fn tcsetattr(fd: c_int, optional_actions: c_int, termios_p: *const termios) -> c_int;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_modes() {
        if isatty() {
            let mut terminal = Terminal::new().unwrap();
            assert!(!terminal.is_raw());

            terminal.set_raw_mode().unwrap();
            assert!(terminal.is_raw());

            terminal.restore_mode().unwrap();
            assert!(!terminal.is_raw());
        }
    }

    #[test]
    fn test_terminal_size() {
        if isatty() {
            let size = terminal_size();
            assert!(size.is_some());
            let (cols, rows) = size.unwrap();
            assert!(cols > 0);
            assert!(rows > 0);
        }
    }
}
