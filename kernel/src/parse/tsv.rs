//! Kernel-native TSV parser — replaces `awk -F'\t'` in shell scripts.
//!
//! Pure-`std`, zero-dep, deterministic. Parses tab-separated values into rows
//! of owned columns. Handles: empty lines (skipped), trailing newlines, fields
//! containing tabs (split at every tab, not just the first N), and Windows `\r\n`.
//!
//! # Usage
//! ```ignore
//! let tsv = "a\tb\tc\n1\t2\t3\n";
//! let rows = parse_rows(tsv, 3).unwrap();
//! assert_eq!(rows, vec![vec!["a","b","c"], vec!["1","2","3"]]);
//! ```

/// Maximum number of columns per row. Bounds memory on adversarial input.
pub const MAX_COLUMNS: usize = 1024;

/// Maximum input length in bytes. Above any real TSV payload.
pub const MAX_INPUT_LEN: usize = 4 * 1024 * 1024;

/// Parse a TSV string into rows of exactly `n_cols` columns.
///
/// - Empty lines and lines with only whitespace are skipped.
/// - Lines with fewer columns than `n_cols` are padded with empty strings.
/// - Lines with more columns than `n_cols` are truncated.
/// - Returns `Err` on empty input or if `n_cols == 0` or `n_cols > MAX_COLUMNS`.
pub fn parse_rows<'a>(src: &'a str, n_cols: usize) -> Result<Vec<Vec<&'a str>>, ParseError> {
    if n_cols == 0 || n_cols > MAX_COLUMNS {
        return Err(ParseError::InvalidColumns);
    }
    if src.len() > MAX_INPUT_LEN {
        return Err(ParseError::InputTooLarge);
    }

    let mut rows = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.is_empty() || trimmed.chars().all(|c| c.is_whitespace()) {
            continue;
        }
        let mut cols: Vec<&str> = trimmed.split('\t').collect();
        if cols.len() < n_cols {
            cols.resize(n_cols, "");
        } else if cols.len() > n_cols {
            cols.truncate(n_cols);
        }
        rows.push(cols);
    }
    if rows.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    Ok(rows)
}

/// Extract a single column by 1-based index from a TSV string.
/// Returns the column values as a vector of `&str`.
/// Returns `Err(ParseError::ColumnOutOfRange)` if the column index exceeds
/// the actual number of columns in the input (first data row determines width).
///
/// # Usage
/// ```ignore
/// let tsv = "name\tage\nAlice\t30\nBob\t25\n";
/// let ages = parse_column(tsv, 2).unwrap();
/// assert_eq!(ages, vec!["age", "30", "25"]);
/// ```
pub fn parse_column<'a>(src: &'a str, col: usize) -> Result<Vec<&'a str>, ParseError> {
    if col == 0 || col > MAX_COLUMNS {
        return Err(ParseError::InvalidColumns);
    }
    // Detect actual column count from first non-empty line
    let actual_width = src
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .find(|l| !l.is_empty() && !l.chars().all(|c| c.is_whitespace()))
        .map(|l| l.split('\t').count())
        .unwrap_or(0);
    if col > actual_width {
        return Err(ParseError::ColumnOutOfRange);
    }
    let rows = parse_rows(src, actual_width)?;
    Ok(rows.into_iter().map(|r| r[col - 1]).collect())
}

/// Errors from TSV parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// `n_cols` was 0 or exceeded `MAX_COLUMNS`.
    InvalidColumns,
    /// Input exceeds `MAX_INPUT_LEN`.
    InputTooLarge,
    /// Input contained no non-empty lines.
    EmptyInput,
    /// Requested column index exceeds the actual number of columns in the input.
    ColumnOutOfRange,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidColumns => write!(f, "invalid column count (0 or > {MAX_COLUMNS})"),
            ParseError::InputTooLarge => write!(f, "input exceeds {MAX_INPUT_LEN} bytes"),
            ParseError::EmptyInput => write!(f, "input contains no data rows"),
            ParseError::ColumnOutOfRange => write!(f, "column index exceeds actual TSV width"),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tsv_parse() {
        let tsv = "name\tage\tcity\nAlice\t30\tNYC\nBob\t25\tLA\n";
        let rows = parse_rows(tsv, 3).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["name", "age", "city"]);
        assert_eq!(rows[1], vec!["Alice", "30", "NYC"]);
        assert_eq!(rows[2], vec!["Bob", "25", "LA"]);
    }

    #[test]
    fn empty_lines_skipped() {
        let tsv = "a\tb\n\n\nc\td\n";
        let rows = parse_rows(tsv, 2).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a", "b"]);
        assert_eq!(rows[1], vec!["c", "d"]);
    }

    #[test]
    fn short_row_padded() {
        let tsv = "a\tb\n1\n";
        let rows = parse_rows(tsv, 3).unwrap();
        assert_eq!(rows[0], vec!["a", "b", ""]);
        assert_eq!(rows[1], vec!["1", "", ""]);
    }

    #[test]
    fn long_row_truncated() {
        let tsv = "a\tb\tc\td\te\n";
        let rows = parse_rows(tsv, 2).unwrap();
        assert_eq!(rows[0], vec!["a", "b"]);
    }

    #[test]
    fn windows_line_endings() {
        let tsv = "a\tb\r\n1\t2\r\n";
        let rows = parse_rows(tsv, 2).unwrap();
        assert_eq!(rows[0], vec!["a", "b"]);
        assert_eq!(rows[1], vec!["1", "2"]);
    }

    #[test]
    fn empty_input_error() {
        assert_eq!(parse_rows("", 2), Err(ParseError::EmptyInput));
    }

    #[test]
    fn zero_columns_error() {
        assert_eq!(parse_rows("a\tb", 0), Err(ParseError::InvalidColumns));
    }

    #[test]
    fn too_many_columns_error() {
        assert_eq!(parse_rows("a\tb", MAX_COLUMNS + 1), Err(ParseError::InvalidColumns));
    }

    #[test]
    fn single_column() {
        let tsv = "fruit\napple\nbanana\n";
        let rows = parse_rows(tsv, 1).unwrap();
        assert_eq!(rows[0], vec!["fruit"]);
        assert_eq!(rows[1], vec!["apple"]);
        assert_eq!(rows[2], vec!["banana"]);
    }

    #[test]
    fn parse_column_basic() {
        let tsv = "name\tage\nAlice\t30\nBob\t25\n";
        let ages = parse_column(tsv, 2).unwrap();
        assert_eq!(ages, vec!["age", "30", "25"]);
    }

    #[test]
    fn parse_column_out_of_range() {
        let tsv = "a\tb\n1\t2\n";
        assert_eq!(parse_column(tsv, 3), Err(ParseError::ColumnOutOfRange));
    }

    #[test]
    fn hot_paths_tsv_fixture() {
        // Simulates parsing docs/audits/hardening/HOT-PATHS.tsv
        let tsv = "zone\teff\tmodule\nhot\t10\torder_machine\ncold\t2\tgeo\n";
        let rows = parse_rows(tsv, 3).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1][0], "hot");
        assert_eq!(rows[1][1], "10");
        assert_eq!(rows[1][2], "order_machine");
    }
}
