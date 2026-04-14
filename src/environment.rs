//! Environment variable management for the Besh shell.

use crate::error::Result;
use std::collections::HashMap;

extern "C" {
    static environ: *mut *mut libc::c_char;
}

/// Get a pointer to the process environment
pub fn environ_ptr() -> *mut *mut libc::c_char {
    unsafe { environ }
}

/// Environment variable manager
#[derive(Debug)]
pub struct Environment {
    /// Internal shell variables
    variables: HashMap<String, String>,
    /// Track which variables are exported to environ
    exported: HashMap<String, bool>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    /// Create a new environment
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            exported: HashMap::new(),
        }
    }

    /// Set a shell variable
    pub fn set(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    /// Get a variable (shell variables first, then process environment)
    pub fn get(&self, name: &str) -> Option<String> {
        self.variables.get(name).cloned().or_else(|| std::env::var(name).ok())
    }

    /// Unset a variable
    pub fn unset(&mut self, name: &str) -> Result<()> {
        self.variables.remove(name);

        unsafe {
            // Remove from environ
            let mut ptr = crate::environment::environ_ptr();
            let mut prev_ptr: *mut *mut libc::c_char = crate::environment::environ_ptr();

            while !ptr.is_null() && !(*ptr).is_null() {
                let var = std::ffi::CStr::from_ptr(*ptr).to_string_lossy();

                if var.starts_with(&format!("{}=", name)) {
                    // Move remaining entries down
                    let mut current = ptr;
                    while !(*current).is_null() {
                        *current =*(current.add(1));
                        current = current.add(1);
                    }
                } else {
                    prev_ptr = ptr;
                }

                ptr = ptr.add(1);
            }
        }

        Ok(())
    }

    /// Export a variable to the process environment
    pub fn export(&mut self, name: &str, value: &str) -> Result<()> {
        self.set(name, value);

        // Set in process environment
        std::env::set_var(name, value);
        self.exported.insert(name.to_string(), true);

        Ok(())
    }

    /// Check if a variable is exported
    pub fn is_exported(&self, name: &str) -> bool {
        self.exported.get(name).copied().unwrap_or(false)
    }

    /// Get all variables
    pub fn all(&self) -> HashMap<String, String> {
        let mut all = HashMap::new();

        // Add process environment variables
        for (k, v) in std::env::vars() {
            all.insert(k, v);
        }

        // Override with shell variables
        for (k, v) in &self.variables {
            all.insert(k.clone(), v.clone());
        }

        all
    }

    /// Get all exported variables
    pub fn exported_vars(&self) -> Vec<(String, String)> {
        self.all()
            .into_iter()
            .filter(|(k, _)| self.is_exported(k) || has_var_in_environ(k))
            .collect()
    }

    /// Expand variables in a string
    pub fn expand(&self, input: &str) -> String {
        crate::parser::expand_variables(input, |k| self.get(k))
    }
}

/// Load all environment variables
pub fn load_environment(environment: &mut Environment) {
    for (key, value) in std::env::vars() {
        environment.set(&key, &value);
        environment.exported.insert(key, true);
    }
}

/// Check if a variable exists in the process environ
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

/// Get shell prompt components
#[derive(Debug, Default)]
pub struct PromptComponents {
    pub username: String,
    pub hostname: String,
    pub cwd: String,
    pub home: String,
}

impl PromptComponents {
    pub fn new() -> Result<Self> {
        let username = std::env::var("USER")
            .unwrap_or_else(|_| "user".to_string());

        let hostname = std::env::var("HOSTNAME")
            .unwrap_or_else(|_| {
                unsafe {
                    let mut hostname = [0u8; 256];
                    if libc::gethostname(hostname.as_mut_ptr() as *mut libc::c_char, 256) == 0 {
                        std::ffi::CStr::from_ptr(hostname.as_ptr() as *const libc::c_char)
                            .to_string_lossy()
                            .to_string()
                    } else {
                        "localhost".to_string()
                    }
                }
            });

        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .display()
            .to_string();

        let home = dirs::home_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::new());

        Ok(PromptComponents {
            username,
            hostname,
            cwd,
            home,
        })
    }

    /// Format the default prompt with colors
    pub fn format(&self) -> String {
        let path_display = if let Some(rel) = self.cwd.strip_prefix(&self.home) {
            format!("~{}", rel)
        } else {
            self.cwd.clone()
        };

        if crate::terminal::isatty() && crate::terminal::color::enabled() {
            use crate::terminal::color;
            format!(
                "{}{}{}{}@{}{}{}{} {}{}> {}",
                color::BOLD, color::GREEN, self.username,
                color::RESET, color::BOLD, color::CYAN, self.hostname,
                color::RESET, color::BLUE, path_display,
                color::RESET
            )
        } else {
            format!("{}@{} {}> ", self.username, self.hostname, path_display)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_set_get() {
        let mut env = Environment::new();
        env.set("TEST", "value");

        assert_eq!(env.get("TEST"), Some("value".to_string()));
    }

    #[test]
    fn test_environment_export() {
        let mut env = Environment::new();
        env.export("TEST_VAR", "test_value").unwrap();

        assert!(env.is_exported("TEST_VAR"));
        assert_eq!(env.get("TEST_VAR"), Some("test_value".to_string()));
    }

    #[test]
    fn test_expand_variables() {
        let mut env = Environment::new();
        env.set("NAME", "World");

        let input = "Hello $NAME";
        let expanded = env.expand(input);

        assert_eq!(expanded, "Hello World");
    }
}
