//! `json_query` — kernel-native JSON field extraction CLI.
//!
//! Replaces all `node -e "process.stdin.on('data', d => { ... })"` patterns
//! in shell scripts. Reads JSON from stdin, extracts a value by dot-path, prints
//! the result to stdout.
//!
//! # Usage
//! ```sh
//! echo '{"name":"test","nested":{"key":"val"}}' | json_query "nested.key"
//! # → val
//!
//! echo '{"items":[1,2,3]}' | json_query "items.1"
//! # → 2
//!
//! echo '{"a":null}' | json_query "a"
//! # → null
//! ```
//!
//! Exit codes: 0 = success, 1 = parse error or missing path.

use dowiz_kernel::json::{self, Value};
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: json_query <dot.path>");
        eprintln!("Reads JSON from stdin, extracts value at dot.path, prints to stdout.");
        std::process::exit(1);
    }

    let path = &args[1];

    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        eprintln!("error: failed to read stdin");
        std::process::exit(1);
    }

    let parsed = match json::parse(&buf) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    match resolve_path(&parsed, path) {
        Some(val) => {
            print_value(val);
            println!();
        }
        None => {
            eprintln!("error: path '{path}' not found");
            std::process::exit(1);
        }
    }
}

/// Resolve a dot-separated path against a JSON value.
/// Supports object keys and integer array indices.
fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        current = match current {
            Value::Object(members) => {
                // last-wins on duplicate keys, matching json::Value::get
                members.iter().rev().find(|(k, _)| k == part).map(|(_, v)| v)?
            }
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                arr.get(idx)?
            }
            _ => return None,
        };
    }
    Some(current)
}

/// Print a JSON value to stdout (without trailing newline — caller adds it).
fn print_value(val: &Value) {
    match val {
        Value::Null => print!("null"),
        Value::Bool(b) => print!("{b}"),
        Value::Int(i) => print!("{i}"),
        Value::Float(f) => print!("{f}"),
        Value::Str(s) => print!("{s}"),
        Value::Array(arr) => {
            print!("[");
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                print_value(v);
            }
            print!("]");
        }
        Value::Object(members) => {
            print!("{{");
            for (i, (k, v)) in members.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                print!("\"{k}\":");
                print_value(v);
            }
            print!("}}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_simple_key() {
        let val = json::parse(r#"{"name":"test"}"#).unwrap();
        assert_eq!(resolve_path(&val, "name"), Some(&Value::Str("test".to_string())));
    }

    #[test]
    fn resolve_nested() {
        let val = json::parse(r#"{"a":{"b":{"c":42}}}"#).unwrap();
        assert_eq!(resolve_path(&val, "a.b.c"), Some(&Value::Int(42)));
    }

    #[test]
    fn resolve_array_index() {
        let val = json::parse(r#"{"items":[10,20,30]}"#).unwrap();
        assert_eq!(resolve_path(&val, "items.1"), Some(&Value::Int(20)));
    }

    #[test]
    fn resolve_missing_path() {
        let val = json::parse(r#"{"a":1}"#).unwrap();
        assert_eq!(resolve_path(&val, "b"), None);
    }

    #[test]
    fn resolve_deep_missing() {
        let val = json::parse(r#"{"a":{"b":1}}"#).unwrap();
        assert_eq!(resolve_path(&val, "a.c"), None);
    }

    #[test]
    fn resolve_null_value() {
        let val = json::parse(r#"{"x":null}"#).unwrap();
        assert_eq!(resolve_path(&val, "x"), Some(&Value::Null));
    }

    #[test]
    fn resolve_object_in_array() {
        let val = json::parse(r#"{"data":[{"id":1},{"id":2}]}"#).unwrap();
        let inner = resolve_path(&val, "data.1").unwrap();
        assert_eq!(resolve_path(inner, "id"), Some(&Value::Int(2)));
    }
}
