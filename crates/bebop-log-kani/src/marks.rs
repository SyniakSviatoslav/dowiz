//! `declare::declarations` tries TRUNCATE, FORGOTTEN, REVOKE and MERGE in that order and takes the
//! first that fits, and `settle.rs` asks ADMIT apart. So the order must not matter: no body may be
//! two kinds.
//!
//! BOUNDS: none needed: the marks are constants and every pair is compared. All five read as
//! distinct prefixes today; this keeps that true when one is renamed or versioned.

use bebop_log::doc::keys::{ADMIT_MARK, REVOKE_MARK};
use bebop_log::doc::merge_body::MERGE_MARK;
use bebop_log::doc::seg::{FORGOTTEN_MARK, TRUNCATE_MARK};

/// PROPERTY: no mark is a prefix of another, so one body starts with at most one mark, and the
/// parsers, each of which opens with `strip_prefix` of its own mark, can accept it at most once.
#[kani::proof]
#[kani::unwind(20)] // past the longest mark, 17 bytes
fn marks_are_prefix_free() {
    let marks = [TRUNCATE_MARK, FORGOTTEN_MARK, REVOKE_MARK, ADMIT_MARK, MERGE_MARK];
    for (i, a) in marks.iter().enumerate() {
        for (j, b) in marks.iter().enumerate() {
            if i != j {
                assert!(!a.starts_with(b));
            }
        }
    }
}
