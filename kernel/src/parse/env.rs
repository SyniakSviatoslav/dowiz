//! Kernel-native `.env` file parser — replaces `split('=')` in TS scripts.
//!
//! Pure-`std`, zero-dep, deterministic. Parses `KEY=VALUE` lines into an
//! ordered map. Handles: comments (`#`), quoted values (`"..."`, `'...'`),
//! empty lines, trailing whitespace, and Windows `\r\n`.
//!
//! # Usage
//! ```ignore
//! let env = "HOST=localhost\nPORT=8080\n";
//! let map = parse_env(env).unwrap();
//! assert_eq!(map.get("PORT"), Some(&"8080".to_string()));
//! ```

use std::collections::HashMap;

/// Maximum number of key-value pairs. Bounds memory on adversarial input.
pub const MAX_ENTRIES: usize = 4096;

/// Maximum input length in bytes.
pub const MAX_INPUT_LEN: usize = 2 * 1024 * 1024;

/// Ordered environment map. Keys preserve insertion order (Vec of pairs)
/// with a HashMap for O(1) lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvMap {
    entries: Vec<(String, String)>,
    index: HashMap<String, usize>,
}

impl EnvMap {
    /// Create an empty EnvMap.
    pub fn new() -> Self {
        EnvMap {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    /// Insert a key-value pair. Overwrites if key exists.
    pub fn insert(&mut self, key: String, value: String) {
        if let Some(&i) = self.index.get(&key) {
            self.entries[i].1 = value;
        } else {
            let idx = self.entries.len();
            self.index.insert(key.clone(), idx);
            self.entries.push((key, value));
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over (key, value) pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl Default for EnvMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a `.env`-format string into an [`EnvMap`].
///
/// - Lines starting with `#` are comments (skipped).
/// - Empty lines and whitespace-only lines are skipped.
/// - Values can be optionally quoted with `"` or `'` (quotes are stripped).
/// - Trailing whitespace around values is trimmed.
/// - Returns `Err` on empty input or exceeding limits.
pub fn parse_env(src: &str) -> Result<EnvMap, EnvParseError> {
    if src.len() > MAX_INPUT_LEN {
        return Err(EnvParseError::InputTooLarge);
    }

    let mut map = EnvMap::new();
    for line in src.lines() {
        let line = line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let eq_pos = trimmed.find('=').ok_or(EnvParseError::MissingEquals)?;
        let key = trimmed[..eq_pos].trim();
        let mut val = trimmed[eq_pos + 1..].trim();

        // Strip surrounding quotes
        if val.len() >= 2 {
            let bytes = val.as_bytes();
            if (bytes[0] == b'"' && bytes[val.len() - 1] == b'"')
                || (bytes[0] == b'\'' && bytes[val.len() - 1] == b'\'')
            {
                val = &val[1..val.len() - 1];
            }
        }

        map.insert(key.to_string(), val.to_string());

        if map.len() > MAX_ENTRIES {
            return Err(EnvParseError::TooManyEntries);
        }
    }

    if map.is_empty() {
        return Err(EnvParseError::EmptyInput);
    }
    Ok(map)
}

/// Errors from `.env` parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvParseError {
    /// A non-comment, non-empty line has no `=`.
    MissingEquals,
    /// Input exceeds `MAX_INPUT_LEN`.
    InputTooLarge,
    /// More than `MAX_ENTRIES` key-value pairs.
    TooManyEntries,
    /// Input contained no valid entries.
    EmptyInput,
}

impl std::fmt::Display for EnvParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvParseError::MissingEquals => write!(f, "line missing '=' separator"),
            EnvParseError::InputTooLarge => write!(f, "input exceeds {MAX_INPUT_LEN} bytes"),
            EnvParseError::TooManyEntries => write!(f, "more than {MAX_ENTRIES} entries"),
            EnvParseError::EmptyInput => write!(f, "input contains no key-value pairs"),
        }
    }
}

impl std::error::Error for EnvParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_env_parse() {
        let env = "HOST=localhost\nPORT=8080\nDEBUG=true\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.get("HOST"), Some(&"localhost".to_string()));
        assert_eq!(map.get("PORT"), Some(&"8080".to_string()));
        assert_eq!(map.get("DEBUG"), Some(&"true".to_string()));
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn comments_skipped() {
        let env = "# This is a comment\nHOST=localhost\n# Another comment\nPORT=3000\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("HOST"), Some(&"localhost".to_string()));
    }

    #[test]
    fn quoted_values() {
        let env = "MSG=\"hello world\"\nPATH='/usr/bin'\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.get("MSG"), Some(&"hello world".to_string()));
        assert_eq!(map.get("PATH"), Some(&"/usr/bin".to_string()));
    }

    #[test]
    fn empty_and_whitespace_lines() {
        let env = "\n  \nA=1\n\nB=2\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("A"), Some(&"1".to_string()));
    }

    #[test]
    fn value_with_equals() {
        let env = "CONN=host=localhost;port=5432\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.get("CONN"), Some(&"host=localhost;port=5432".to_string()));
    }

    #[test]
    fn overwrite_duplicate_key() {
        let env = "A=1\nA=2\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.get("A"), Some(&"2".to_string()));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn empty_input_error() {
        assert_eq!(parse_env(""), Err(EnvParseError::EmptyInput));
        assert_eq!(parse_env("# only comments\n"), Err(EnvParseError::EmptyInput));
    }

    #[test]
    fn missing_equals() {
        let env = "INVALID_LINE\nA=1\n";
        assert_eq!(parse_env(env), Err(EnvParseError::MissingEquals));
    }

    #[test]
    fn windows_line_endings() {
        let env = "A=1\r\nB=2\r\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.get("A"), Some(&"1".to_string()));
        assert_eq!(map.get("B"), Some(&"2".to_string()));
    }

    #[test]
    fn insertion_order_preserved() {
        let env = "Z=1\nA=2\nM=3\n";
        let map = parse_env(env).unwrap();
        let keys: Vec<&str> = map.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["Z", "A", "M"]);
    }

    #[test]
    fn empty_value() {
        let env = "EMPTY=\n";
        let map = parse_env(env).unwrap();
        assert_eq!(map.get("EMPTY"), Some(&"".to_string()));
    }
}
