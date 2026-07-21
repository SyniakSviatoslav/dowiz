//! End-to-end integration test for the `json_query` binary.
//!
//! Spawns the real binary as a subprocess and verifies:
//! - stdin reading works end-to-end
//! - dot-path extraction produces correct stdout
//! - Exit code 0 on success
//! - Exit code 1 on parse error, missing path, or bad args

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn json_query_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_json_query"))
}

/// Run json_query with the given stdin and path argument.
/// Returns (exit_code, stdout, stderr).
fn run_json_query(stdin_data: &str, path: &str) -> (i32, String, String) {
    let mut child = Command::new(json_query_bin())
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn json_query");

    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(stdin_data.as_bytes())
        .expect("write to child stdin");

    let output = child.wait_with_output().expect("wait for json_query");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn e2e_simple_object_key() {
    let (code, stdout, _) = run_json_query(r#"{"name":"alice"}"#, "name");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "alice");
}

#[test]
fn e2e_nested_path() {
    let (code, stdout, _) = run_json_query(r#"{"a":{"b":{"c":42}}}"#, "a.b.c");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "42");
}

#[test]
fn e2e_array_index() {
    let (code, stdout, _) = run_json_query(r#"{"items":[10,20,30]}"#, "items.1");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "20");
}

#[test]
fn e2e_null_value() {
    let (code, stdout, _) = run_json_query(r#"{"x":null}"#, "x");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "null");
}

#[test]
fn e2e_boolean_value() {
    let (code, stdout, _) = run_json_query(r#"{"flag":true}"#, "flag");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "true");
}

#[test]
fn e2e_missing_path_exits_1() {
    let (code, _, stderr) = run_json_query(r#"{"a":1}"#, "b");
    assert_eq!(code, 1);
    assert!(stderr.contains("not found"));
}

#[test]
fn e2e_invalid_json_exits_1() {
    let (code, _, stderr) = run_json_query("not json at all", "x");
    assert_eq!(code, 1);
    assert!(stderr.contains("error"));
}

#[test]
fn e2e_no_args_exits_1() {
    let output = Command::new(json_query_bin())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    assert_eq!(output.status.code().unwrap_or(-1), 1);
}

#[test]
fn e2e_help_flag_exits_1() {
    let output = Command::new(json_query_bin())
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    assert_eq!(output.status.code().unwrap_or(-1), 1);
}

#[test]
fn e2e_complex_nested_structure() {
    let json = r#"{"users":[{"id":1,"name":"alice","tags":["admin","active"]},{"id":2,"name":"bob"}]}"#;
    let (code, stdout, _) = run_json_query(json, "users.0.tags.0");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "admin");
}

#[test]
fn e2e_empty_object() {
    let (code, stdout, _) = run_json_query(r#"{}"#, "missing");
    assert_eq!(code, 1);
    let _ = stdout;
}
