//! Command parsing for the Besh shell.
//!
//! Parses command lines into executable commands with support for:
//! - Simple commands
//! - Pipes (|)
//! - Redirections (>, >>, <, 2>)
//! - Background execution (&)
//! - Quoted strings
//! - Variable expansion

use crate::error::{Result, ShellError};
use crate::process::Redirection;
use std::collections::VecDeque;

/// A parsed command ready for execution
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    /// The program name
    pub program: String,
    /// Command arguments
    pub args: Vec<String>,
    /// stdin redirection
    pub stdin: Option<Redirection>,
    /// stdout redirection
    pub stdout: Option<Redirection>,
    /// stderr redirection
    pub stderr: Option<Redirection>,
    /// Run in background
    pub background: bool,
}

impl Command {
    /// Create a new empty command
    pub fn new(program: String) -> Self {
        Command {
            program,
            args: Vec::new(),
            stdin: None,
            stdout: None,
            stderr: None,
            background: false,
        }
    }

    /// Get the command as an argv slice
    pub fn as_argv(&self) -> Vec<String> {
        let mut argv = vec![self.program.clone()];
        argv.extend(self.args.iter().cloned());
        argv
    }
}

/// Token types for parsing
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Pipe,
    RedirectIn,
    RedirectOut,
    RedirectAppend,
    RedirectErr,
    RedirectErrAppend,
    Background,
}

/// Lexer for tokenizing command line input
struct Lexer<'a> {
    input: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Lexer {
            input: input.chars().peekable(),
        }
    }

    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn next(&mut self) -> Option<char> {
        self.input.next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.peek() {
            if c.is_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    fn lex(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(&c) = self.peek() {
            if c.is_whitespace() {
                self.skip_whitespace();
                continue;
            }

            match c {
                '|' => {
                    self.next();
                    tokens.push(Token::Pipe);
                }
                '<' => {
                    self.next();
                    tokens.push(Token::RedirectIn);
                }
                '>' => {
                    self.next();
                    // Check for 2> pattern first (look back at previous token)
                    let is_stderr = if let Some(Token::Word(ref w)) = tokens.last() {
                        w == "2"
                    } else {
                        false
                    };

                    if is_stderr {
                        if let Some(Token::Word(_)) = tokens.pop() {
                            // Was "2", now check if it's >> or >
                            if let Some(&'>') = self.peek() {
                                self.next();
                                tokens.push(Token::RedirectErrAppend);
                            } else {
                                tokens.push(Token::RedirectErr);
                            }
                        }
                    } else {
                        if let Some(&'>') = self.peek() {
                            self.next();
                            tokens.push(Token::RedirectAppend);
                        } else {
                            tokens.push(Token::RedirectOut);
                        }
                    }
                }
                '&' => {
                    self.next();
                    tokens.push(Token::Background);
                }
                '\'' => self.lex_single_quoted(&mut tokens)?,
                '"' => self.lex_double_quoted(&mut tokens)?,
                '$' => self.lex_variable(&mut tokens)?,
                _ => self.lex_word(&mut tokens)?,
            }
        }

        Ok(tokens)
    }

    fn lex_single_quoted(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        self.next(); // Skip opening quote
        let mut word = String::new();

        while let Some(&c) = self.peek() {
            if c == '\'' {
                self.next();
                tokens.push(Token::Word(word));
                return Ok(());
            }
            word.push(c);
            self.next();
        }

        Err(ShellError::ParseError("Unterminated single quote".to_string()))
    }

    fn lex_double_quoted(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        self.next(); // Skip opening quote
        let mut word = String::new();

        while let Some(&c) = self.peek() {
            if c == '"' {
                self.next();
                tokens.push(Token::Word(word));
                return Ok(());
            }
            if c == '\\' {
                self.next(); // Skip backslash
                if let Some(&next_c) = self.peek() {
                    word.push(next_c);
                    self.next();
                }
            } else if c == '$' {
                self.next();
                let mut var_name = String::new();
                while let Some(&v) = self.peek() {
                    if v.is_alphanumeric() || v == '_' {
                        var_name.push(v);
                        self.next();
                    } else if v == '{' {
                        self.next();
                        while let Some(&v) = self.peek() {
                            if v == '}' {
                                self.next();
                                break;
                            }
                            var_name.push(v);
                            self.next();
                        }
                        break;
                    } else {
                        break;
                    }
                }
                word.push_str(&format!("${}", var_name));
                // Variable expansion will be done later
                tokens.push(Token::Word(word.clone()));
                word.clear();
            } else {
                word.push(c);
                self.next();
            }
        }

        Err(ShellError::ParseError("Unterminated double quote".to_string()))
    }

    fn lex_variable(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        self.next(); // Skip $
        let mut var_name = String::new();

        if let Some(&'{') = self.peek() {
            self.next();
            while let Some(&c) = self.peek() {
                if c == '}' {
                    self.next();
                    break;
                }
                var_name.push(c);
                self.next();
            }
        } else {
            while let Some(&c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var_name.push(c);
                    self.next();
                } else {
                    break;
                }
            }
        }

        tokens.push(Token::Word(format!("${}", var_name)));
        Ok(())
    }

    fn lex_word(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let mut word = String::new();

        while let Some(&c) = self.peek() {
            if c.is_whitespace() || matches!(c, '|' | '>' | '<' | '&') {
                break;
            }
            if c == '\\' {
                self.next();
                if let Some(&next_c) = self.peek() {
                    word.push(next_c);
                    self.next();
                }
            } else {
                word.push(c);
                self.next();
            }
        }

        if !word.is_empty() {
            tokens.push(Token::Word(word));
        }

        Ok(())
    }
}

/// Parse a command line into a sequence of commands
pub fn parse_command_line(input: &str) -> Result<VecDeque<Command>> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(VecDeque::new());
    }

    let mut lexer = Lexer::new(input);
    let tokens = lexer.lex()?;

    let mut commands = VecDeque::new();
    let mut current_cmd: Option<Command> = None;
    let mut background = false;

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(word) => {
                if current_cmd.is_none() {
                    current_cmd = Some(Command::new(word.clone()));
                } else {
                    if let Some(cmd) = &mut current_cmd {
                        cmd.args.push(word.clone());
                    }
                }
            }
            Token::Pipe => {
                if let Some(mut cmd) = current_cmd.take() {
                    cmd.background = false; // Background set for pipeline later
                    commands.push_back(cmd);
                }
            }
            Token::Background => {
                background = true;
            }
            Token::RedirectIn => {
                if i + 1 < tokens.len() {
                    if let Token::Word(path) = &tokens[i + 1] {
                        if let Some(cmd) = &mut current_cmd {
                            cmd.stdin = Some(Redirection::File(path.clone()));
                        }
                        i += 1;
                    }
                }
            }
            Token::RedirectOut => {
                if i + 1 < tokens.len() {
                    if let Token::Word(path) = &tokens[i + 1] {
                        if let Some(cmd) = &mut current_cmd {
                            cmd.stdout = Some(Redirection::File(path.clone()));
                        }
                        i += 1;
                    }
                }
            }
            Token::RedirectAppend => {
                if i + 1 < tokens.len() {
                    if let Token::Word(path) = &tokens[i + 1] {
                        // For append, we'd need to use O_APPEND flag
                        // For now, just treat as file
                        if let Some(cmd) = &mut current_cmd {
                            cmd.stdout = Some(Redirection::File(format!("append:{}", path)));
                        }
                        i += 1;
                    }
                }
            }
            Token::RedirectErr => {
                if i + 1 < tokens.len() {
                    if let Token::Word(path) = &tokens[i + 1] {
                        if let Some(cmd) = &mut current_cmd {
                            cmd.stderr = Some(Redirection::File(path.clone()));
                        }
                        i += 1;
                    }
                }
            }
            Token::RedirectErrAppend => {
                if i + 1 < tokens.len() {
                    if let Token::Word(path) = &tokens[i + 1] {
                        if let Some(cmd) = &mut current_cmd {
                            cmd.stderr = Some(Redirection::File(format!("append:{}", path)));
                        }
                        i += 1;
                    }
                }
            }
        }
        i += 1;
    }

    // Push the last command
    if let Some(mut cmd) = current_cmd {
        cmd.background = background;
        commands.push_back(cmd);
    }

    Ok(commands)
}

/// Expand variables in a string (replace $VAR and ${VAR})
pub fn expand_variables(input: &str, get_var: impl Fn(&str) -> Option<String>) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            let mut var_name = String::new();
            let mut is_braced = false;

            if let Some(&'{') = chars.peek() {
                is_braced = true;
                chars.next(); // Skip {
            }

            while let Some(&next) = chars.peek() {
                if is_braced {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                } else if !next.is_alphanumeric() && next != '_' {
                    break;
                }
                var_name.push(next);
                chars.next();
            }

            if let Some(value) = get_var(&var_name) {
                result.push_str(&value);
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let commands = parse_command_line("echo hello").unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].program, "echo");
        assert_eq!(commands[0].args, vec!["hello"]);
    }

    #[test]
    fn test_pipeline() {
        let commands = parse_command_line("cat file | grep foo").unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].program, "cat");
        assert_eq!(commands[0].args, vec!["file"]);
        assert_eq!(commands[1].program, "grep");
        assert_eq!(commands[1].args, vec!["foo"]);
    }

    #[test]
    fn test_redirections() {
        let commands = parse_command_line("cat > output < input").unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].stdin.is_some());
        assert!(commands[0].stdout.is_some());
    }

    #[test]
    fn test_background() {
        let commands = parse_command_line("sleep 10 &").unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].background);
    }

    #[test]
    fn test_quotes() {
        let commands = parse_command_line("echo \"hello world\"").unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].args, vec!["hello world"]);
    }

    #[test]
    fn test_variable_expansion() {
        assert_eq!(
            expand_variables("Hello $USER", |k| k.is_empty().then_some("World".to_string())),
            "Hello World"
        );
    }
}
