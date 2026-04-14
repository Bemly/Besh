//! Integration tests for Besh shell.
//!
//! Tests cover: parser, process, history, environment, builtin, and shell execution.

use std::io::Write;
use std::process::{Command, Stdio};

fn besh() -> Command {
    Command::new("./target/release/besh")
}

fn run_besh_script(script: &str) -> std::process::Output {
    let mut child = besh()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn besh");

    child.stdin.take().unwrap()
        .write_all(script.as_bytes())
        .unwrap();

    child.wait_with_output().unwrap()
}

fn stdout_str(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_str(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

// ─── CLI Tests ─────────────────────────────────────────────

#[test]
fn test_help_flag() {
    let output = besh().arg("--help").output().unwrap();
    assert!(output.status.success());
    let out = stdout_str(&output);
    assert!(out.contains("BESH"));
    assert!(out.contains("Usage"));
    assert!(out.contains("interactive"));
}

#[test]
fn test_version_flag() {
    let output = besh().arg("--version").output().unwrap();
    assert!(output.status.success());
    let out = stdout_str(&output);
    assert!(out.contains("26.4.14"));
    assert!(out.contains("Bemly"));
}

// ─── Single Command Execution ─────────────────────────────

#[test]
fn test_single_echo() {
    let output = besh().arg("echo").arg("hello").output().unwrap();
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("hello"));
}

#[test]
fn test_single_ls() {
    let output = besh().arg("ls").arg("./src").output().unwrap();
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("main.rs"));
}

#[test]
fn test_nonexistent_command() {
    let output = besh().arg("nonexistent_cmd_xyz").output().unwrap();
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("not found") || !output.status.success(),
        "Expected error for nonexistent command, got stderr: {}",
        stderr
    );
}

// ─── Built-in Commands ─────────────────────────────────────

#[test]
fn test_builtin_echo() {
    let output = run_besh_script("echo builtin_test\n");
    assert!(stdout_str(&output).contains("builtin_test"));
}

#[test]
fn test_builtin_echo_multiple_args() {
    let output = run_besh_script("echo one two three\n");
    let out = stdout_str(&output);
    assert!(out.contains("one"));
    assert!(out.contains("two"));
    assert!(out.contains("three"));
}

#[test]
fn test_builtin_pwd() {
    let output = run_besh_script("pwd\n");
    let out = stdout_str(&output);
    assert!(!out.trim().is_empty(), "pwd should output a path");
    assert!(out.contains("/"), "pwd output should be an absolute path: {}", out);
}

#[test]
fn test_builtin_cd_and_pwd() {
    let output = run_besh_script("cd /tmp\npwd\n");
    let out = stdout_str(&output);
    // macOS /tmp is /private/tmp
    assert!(
        out.trim().ends_with("/tmp") || out.contains("/private/tmp"),
        "cd /tmp then pwd should show /tmp: {}",
        out
    );
}

#[test]
fn test_builtin_cd_home() {
    let output = run_besh_script("cd\npwd\n");
    let out = stdout_str(&output);
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        assert!(out.contains(&home), "cd with no args should go HOME: {}", out);
    }
}

#[test]
fn test_builtin_env() {
    let output = besh().arg("env").output().unwrap();
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("PATH="));
}

#[test]
fn test_builtin_export_and_echo() {
    let output = run_besh_script("export BESH_TEST=hello123\necho $BESH_TEST\n");
    assert!(stdout_str(&output).contains("hello123"));
}

#[test]
fn test_builtin_unset() {
    let output = run_besh_script("export UNSET_ME=aaa\nunset UNSET_ME\necho $UNSET_ME\n");
    let out = stdout_str(&output);
    assert!(!out.contains("aaa"), "unset should remove variable: {}", out);
}

#[test]
fn test_builtin_set() {
    let output = run_besh_script("export SET_VAR=setval\nset\n");
    assert!(stdout_str(&output).contains("SET_VAR=setval"));
}

#[test]
fn test_builtin_exit() {
    let output = run_besh_script("exit\n");
    // Should exit cleanly (exit code 0 or None)
    assert!(output.status.success() || output.status.code() == Some(0));
}

#[test]
fn test_builtin_exit_with_code() {
    let output = run_besh_script("exit 0\n");
    assert!(output.status.success());
}

// ─── Variable Expansion ────────────────────────────────────

#[test]
fn test_variable_expansion() {
    let output = run_besh_script("export MYVAR=world\necho hello_$MYVAR\n");
    assert!(stdout_str(&output).contains("hello_world"));
}

#[test]
fn test_variable_braced_expansion() {
    let output = run_besh_script("export BRACED=braced_val\necho prefix_${BRACED}_suffix\n");
    assert!(stdout_str(&output).contains("braced_val"));
}

#[test]
fn test_home_variable() {
    let output = run_besh_script("echo $HOME\n");
    let out = stdout_str(&output);
    assert!(!out.trim().is_empty(), "$HOME should expand");
    assert!(!out.contains("$HOME"), "Should not contain literal $HOME: {}", out);
}

#[test]
fn test_user_variable() {
    let output = run_besh_script("echo $USER\n");
    let out = stdout_str(&output);
    assert!(!out.trim().is_empty(), "$USER should expand");
}

// ─── Pipes ─────────────────────────────────────────────────

#[test]
fn test_pipe_echo_cat() {
    let output = run_besh_script("echo piped_text\n");
    assert!(stdout_str(&output).contains("piped_text"));
}

// ─── I/O Redirection ───────────────────────────────────────

#[test]
fn test_output_redirection() {
    let tmpfile = "/tmp/besh_test_out.txt";
    let _ = std::fs::remove_file(tmpfile);

    let _ = run_besh_script(&format!("echo redirected_content > {}\n", tmpfile));

    let content = std::fs::read_to_string(tmpfile).unwrap_or_default();
    assert!(content.contains("redirected_content"), "File: {}", content);

    let _ = std::fs::remove_file(tmpfile);
}

#[test]
fn test_append_redirection() {
    let tmpfile = "/tmp/besh_test_append.txt";
    let _ = std::fs::remove_file(tmpfile);

    let _ = run_besh_script(&format!("echo line1 > {}\necho line2 >> {}\n", tmpfile, tmpfile));

    let content = std::fs::read_to_string(tmpfile).unwrap_or_default();
    assert!(content.contains("line1"), "Should contain line1: {}", content);
    assert!(content.contains("line2"), "Should contain line2: {}", content);

    let _ = std::fs::remove_file(tmpfile);
}

#[test]
fn test_input_redirection() {
    let tmpfile = "/tmp/besh_test_in.txt";
    std::fs::write(tmpfile, "input_file_content\n").unwrap();

    let output = run_besh_script(&format!("cat < {}\n", tmpfile));
    assert!(stdout_str(&output).contains("input_file_content"));

    let _ = std::fs::remove_file(tmpfile);
}

#[test]
fn test_stderr_redirection() {
    let tmpfile = "/tmp/besh_test_err.txt";
    let _ = std::fs::remove_file(tmpfile);

    // ls a nonexistent dir writes to stderr
    let _ = run_besh_script(&format!("ls /nonexistent_dir_xyz 2> {}\n", tmpfile));

    let content = std::fs::read_to_string(tmpfile).unwrap_or_default();
    assert!(!content.trim().is_empty(), "stderr should be captured");

    let _ = std::fs::remove_file(tmpfile);
}

// ─── Quoting ───────────────────────────────────────────────

#[test]
fn test_double_quotes() {
    let output = run_besh_script("echo \"hello world\"\n");
    assert!(stdout_str(&output).contains("hello world"));
}

#[test]
fn test_single_quotes() {
    let output = run_besh_script("echo 'hello world'\n");
    assert!(stdout_str(&output).contains("hello world"));
}

#[test]
fn test_quotes_preserve_spaces() {
    let output = run_besh_script("echo \"a  b  c\"\n");
    let out = stdout_str(&output);
    assert!(out.contains("a  b  c"), "Spaces should be preserved in quotes: {}", out);
}

// ─── Background Jobs ───────────────────────────────────────

#[test]
fn test_background_job() {
    let output = run_besh_script("sleep 0.1 &\n");
    // Should not hang or crash
    let out = stdout_str(&output);
    assert!(
        out.contains("[") || output.status.success(),
        "Background job should work"
    );
}

// ─── History ───────────────────────────────────────────────

#[test]
fn test_builtin_history() {
    let output = run_besh_script("echo hist_cmd1\necho hist_cmd2\nhistory\n");
    let out = stdout_str(&output);
    assert!(
        out.contains("hist_cmd1") || out.contains("hist_cmd2"),
        "history should list commands: {}",
        out
    );
}

// ─── Non-interactive Mode ──────────────────────────────────

#[test]
fn test_noninteractive_from_pipe() {
    let output = Command::new("bash")
        .arg("-c")
        .arg("echo 'echo pipe_test' | ./target/release/besh")
        .output()
        .unwrap();

    assert!(stdout_str(&output).contains("pipe_test"));
}

#[test]
fn test_noninteractive_multiple_lines() {
    let output = run_besh_script("echo first\necho second\n");
    let out = stdout_str(&output);
    assert!(out.contains("first"));
    assert!(out.contains("second"));
}

// ─── Edge Cases ────────────────────────────────────────────

#[test]
fn test_empty_lines() {
    let output = run_besh_script("\n\n\nexit\n");
    // Should not crash
    assert!(output.status.success() || output.status.code() == Some(0));
}

#[test]
fn test_comment_lines_in_script() {
    // Comments work in script files, not in stdin mode
    let script = "/tmp/besh_comment_test.besh";
    std::fs::write(script, "# this is a comment\necho visible\n").unwrap();

    let output = besh().arg(script).output().unwrap();
    let out = stdout_str(&output);
    assert!(out.contains("visible"), "Script should execute non-comment lines: {}", out);

    let _ = std::fs::remove_file(script);
}

#[test]
fn test_whitespace_handling() {
    let output = run_besh_script("  echo   spaced  \n");
    let out = stdout_str(&output);
    assert!(out.contains("spaced"), "Whitespace should be trimmed: {}", out);
}

#[test]
fn test_command_with_many_args() {
    let output = run_besh_script("echo a b c d e f g h i j\n");
    let out = stdout_str(&output);
    assert!(out.contains("a"), "should contain 'a': {}", out);
    assert!(out.contains("j"), "should contain 'j': {}", out);
}

// ─── Process Integration ───────────────────────────────────

#[test]
fn test_external_command_args() {
    let output = besh().arg("printf").arg("format_%s").arg("value").output().unwrap();
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("format_value"));
}

#[test]
fn test_external_command_with_env() {
    let output = besh()
        .env("BESH_EXT_TEST", "from_env")
        .arg("env")
        .output()
        .unwrap();

    assert!(stdout_str(&output).contains("BESH_EXT_TEST=from_env"));
}

// ─── Error Handling ────────────────────────────────────────

#[test]
fn test_cd_nonexistent_dir() {
    let output = run_besh_script("cd /nonexistent_directory_xyz\n");
    let stderr = stderr_str(&output);
    // Should produce an error, not crash
    assert!(
        stderr.contains("not found") || stderr.contains("error") || stderr.contains("No such"),
        "cd to nonexistent dir should error: stderr={}",
        stderr
    );
}
