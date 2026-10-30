//! KV v4: THE COPY-ON-WRITE B+TREE (W-ES-P8B, R-EMBED-STORE §12; design `es-p8.design.md` §1-§3).
//!
//! v1-v3 keep every key in ONE sorted array, so a write either rewrites the image or grows a
//! delta chain that compaction folds back (59 201 cells written per put at 1e5, ES-BENCH §5). v4
//! keeps the entries in a B+tree of ordinary bebop objects, and a commit writes only the path from
//! each changed leaf to the root: every untouched subtree is SHARED with the previous generation by
//! address, never copied.
//!
//! ── LAYOUT (normative text: design §1.2) ──
//!
//! ```text
//! KV root v4 (DIGEST_KV_ROOT, exactly 8 cells, the digest v1-v3 use):
//!   {n, 0, 0, 0, 0, VERSION=4, ref TREE (0 = the empty map), DEPTH}
//!   cells 1..4 are ZERO, so a reader that ignored the version finds no arrays and refuses.
//! LEAF (DIGEST_KV4_LEAF): [META = 1<<56 | m] [K_i, V_i = ref VAL_i]... [key area]
//! NODE (DIGEST_KV4_NODE): [META = 2<<56 | hc<<48 | m] [C_i = ref child_i, K_i, hc hash cells]...
//!                         [key area]; K_i is the FIRST key of child_i
//! VAL  (DIGEST_KV4_VAL):  [vlen] [the value packed eight bytes to a cell]
//! K = (byte offset into the key area << 16) | key length; keys packed in slot order.
//! ```
//!
//! THE RULES A READER REFUSES ON, never guesses past (each is a `TreeError` variant):
//!   1. every ref inside the tree is STRICTLY BACKWARD (`off < 0`), so any walk terminates on any
//!      bytes -- a commit allocates values, then leaves, then nodes, then the root;
//!   2. a leaf sits at exactly DEPTH levels (`DEPTH <= MAX_TREE_DEPTH`): a node of the wrong kind
//!      at a level is `Depth`;
//!   3. keys ascend strictly, and child i's first key IS separator i (`verify`);
//!   4. `n` is a claim: `verify` counts the leaves and refuses a mismatch;
//!   5. lengths are exact (`verify` and the writer read every slot; `get` bounds each probe).
//!
//! `get` and the scans check rules 1, 2 and the bounds of every cell they touch, and allocate
//! nothing (a value borrows the image where the image is bytes). `verify` is the full check: the
//! crc of every reachable object plus rules 1-5 -- garbage is never read.
//!
//! WIRED (P8c): `KvIn` and `Kv::load_checked` read v4 (`kvin.rs`; only with the `tree` feature,
//! native -- every other build keeps refusing it until P8d), bebop-io's `Writer::commit_tree` / `Db` write it, and `compact.rs`
//! is the dead-ratio copy compaction. Readers in other languages are P8d.

pub mod build;
pub mod commit;
pub mod compact;
pub mod iter;
pub mod kvin;
mod node;
pub mod read;
pub mod write;

pub use build::{migrate_to_tree, stage_build, tree_bytes_fit};
pub use commit::{commit_tree_bytes, stage_tree};
pub use compact::{dead_over, tree_compacted};
pub use kvin::v4_here;
pub use iter::Iter;
pub use read::TreeIn;
pub use write::WriteError;

use crate::verify::BadCrc;

/// The version a v4 root says (root payload cell 5).
pub const VERSION_TREE: i64 = 4;
/// Cells in a v4 root.
pub const ROOT_V4: usize = 8;
/// `st_digest("KV4L{i64,[i64]}")`: a leaf. Derived in `tree/tests.rs` from the layout string.
pub const DIGEST_KV4_LEAF: i64 = 284_439_384;
/// `st_digest("KV4N{i64,[i64]}")`: an internal node.
pub const DIGEST_KV4_NODE: i64 = 3_702_325_768;
/// `st_digest("KV4V{i64,[i64]}")`: one value.
pub const DIGEST_KV4_VAL: i64 = 44_497_123;
/// The layout strings the three digests are `st_digest` of.
pub const LAYOUT_KV4_LEAF: &str = "KV4L{i64,[i64]}";
pub const LAYOUT_KV4_NODE: &str = "KV4N{i64,[i64]}";
pub const LAYOUT_KV4_VAL: &str = "KV4V{i64,[i64]}";
/// META's kind byte.
pub const KIND_LEAF: u64 = 1;
pub const KIND_NODE: u64 = 2;
/// Levels below the KV root. At fanout 32 that is 32^16 entries, far past a 2^40-byte image.
pub const MAX_TREE_DEPTH: usize = 16;
/// The writer's fanout (design §1.1: the knee of the measured sweep). A WRITER parameter only:
/// readers take `m` from each node, so it can change without a format version.
pub const FANOUT: usize = 32;
/// The fanouts a writer accepts: below 4 the even split can leave a one-entry node, and `m` is
/// a 16-bit field.
pub const MIN_FANOUT: usize = 4;
pub const MAX_FANOUT: usize = 65_535;

/// Why a v4 tree was not read. Every variant names the object it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeError {
    /// No KV root, or a root that is not version 4 (a v1-v3 image is `KvIn`'s, not this module's).
    NotTree,
    /// The v4 root breaks its own shape: `why` names the cell.
    Root { why: &'static str },
    /// A node or value object breaks its layout (digest, META, a length, a slot past the object).
    Node { obj: usize, why: &'static str },
    /// A ref inside the tree that does not point strictly backward (rule 1).
    Forward { obj: usize },
    /// A node of the wrong kind for its level: the DEPTH claim does not hold (rule 2).
    Depth { obj: usize },
    /// Keys out of order, or a separator that is not its child's first key (rule 3).
    Order { obj: usize },
    /// The root's `n` is not the number of entries the leaves hold (rule 4).
    Count { claim: i64, got: i64 },
    /// An object's crc does not match.
    BadCrc(BadCrc),
    /// The merged-index API (`KvIn::key(i)`, `value(i)`, `seek`, `prefix_range`) on a v4 image:
    /// a COW tree keeps no subtree counts, so it is refused by name, never answered in O(i).
    Unindexed,
}

/// FNV-1a 64, the fold every KV version answers with (`Kv::snapshot_root_u64`).
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(crate::FNV_PRIME);
    }
    h
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod seq_tests;
#[cfg(test)]
mod corrupt_tests;
#[cfg(test)]
mod cells_tests;
#[cfg(all(test, feature = "tree"))]
mod wire_tests;
