// SPDX-License-Identifier: MIT OR Apache-2.0
//! The status vocabulary in words: docs/STATUS-CODES.md §2 for the codes a read can give, plus
//! the three this ABI adds for a call made against an open image (70, 71, 73: above the C ABI's
//! 40-60 range, so no number of that table is reused). `bw_status_text` hands a
//! host the line for a code so a person sees "BAD_CRC: an object the read reached fails its
//! crc" and not `10`.

/// The codes a read of an image can give (STATUS-CODES §2, the reader rows).
pub const OK: i32 = 0;
pub const NO_SUPERBLOCK: i32 = 1;
pub const NOT_A_KV: i32 = 2;
pub const NOT_A_LOG: i32 = 3;
pub const TRUNCATED: i32 = 4;
pub const NULL_ARG: i32 = 5;
pub const BAD_CRC: i32 = 10;
pub const UNKNOWN_VERSION: i32 = 12;
pub const LAST_COMMIT: i32 = 41;
/// A call that needs an open image when none is (`bw_open` was not called, or refused).
pub const NOT_OPEN: i32 = 70;
/// `bw_get`: no entry under that key.
pub const NOT_FOUND: i32 = 71;
/// `bw_log_at`: the index is past the last record. (72 is left free: a buffer too small is
/// answered with the length needed, not a status -- abi.rs.)
pub const OUT_OF_RANGE: i32 = 73;

/// Every code this crate can give, in order, with its name and meaning. `tests/conformance.rs`
/// holds this table total over `Refusal` and over every `abi` return.
pub const TABLE: &[(i32, &str)] = &[
    (OK, "OK: read; n = entries or records, root = the FNV-1a 64 fold"),
    (NO_SUPERBLOCK, "NO_SUPERBLOCK: no valid superblock, or the live one was refused structurally (arena_used or PartTab outside, PartTab misplaced or its crc bad, root outside)"),
    (NOT_A_KV, "NOT_A_KV: a superblock but no KV root, or a KV claim (a slice, a chain, a v4 rule) that does not hold"),
    (NOT_A_LOG, "NOT_A_LOG: asked for the log of an image whose root is not a log root"),
    (TRUNCATED, "TRUNCATED: the log's root claims more records than its chain delivers, or the chain never ends"),
    (NULL_ARG, "NULL_ARG: a null pointer at the ABI"),
    (BAD_CRC, "BAD_CRC: an object the read reached fails its crc (root, an array, a delta record, a node, a value, a log record, an object of a torn last commit)"),
    (UNKNOWN_VERSION, "UNKNOWN_VERSION: the KV root names a version above 4"),
    (LAST_COMMIT, "LAST_COMMIT: the last commit's range does not hash to its anchor and every object in it checks (a re-sealed object, a changed header); never fallen back past"),
    (NOT_OPEN, "NOT_OPEN: no image is open (bw_open was not called, or refused)"),
    (NOT_FOUND, "NOT_FOUND: no entry under that key"),
    (OUT_OF_RANGE, "OUT_OF_RANGE: the index is past the last record"),
];

/// The line for `code`, or `None` for a number this crate never gives.
pub fn text(code: i32) -> Option<&'static str> {
    TABLE.iter().find(|(c, _)| *c == code).map(|(_, t)| *t)
}
