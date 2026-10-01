//! One tenant's hub: the order log, over a bebop store, as PURE logic.
//!
//! Bytes in, bytes out. No filesystem, no network, no clock, no randomness — the
//! caller supplies the store image, the time and the ids, and gets a new image
//! back. That is what makes this testable natively today while the Worker shell
//! around it (a Durable Object for single-writer serialisation, object storage
//! for the image) is still unbuilt: the part that holds the data is proven
//! before the part that moves it exists.
//!
//! WHY AN EVENT LOG AND NOT A TABLE. An order's state is a FOLD over what
//! happened to it, not a mutable row — the kernel's own decide/fold law. The
//! bebop event log appends ONE object per event and relinks the root, which is
//! O(1); the KV layout would rewrite the world on every put. The log is the
//! truth and any projection is rebuilt from it.
//!
//! HONEST COST. Reading one order walks the chain, which is O(n) in the log.
//! At one restaurant's volume — a few hundred orders a day — that is nothing,
//! and the walk happens in memory over an image already loaded. It is stated
//! here rather than discovered later: at platform volume this wants an index,
//! and the index would be a projection rebuilt from the log, never a second
//! source of truth.

#![forbid(unsafe_code)]

pub mod block;
pub mod caps;
pub mod catalog;
pub mod consent;
pub mod crypto;
pub mod hours;
pub mod import;
pub mod media;
pub mod minijson;
pub mod modifiers;
pub mod palette;
pub mod activation;
pub mod allergens;
pub mod brand;
pub mod features;
pub mod graph;
pub mod post;
pub mod promo;
pub mod redact;
pub mod roster;
pub mod room;
pub mod settings;
pub mod stock;
pub mod prep;
pub mod voice;
pub mod zone;
pub mod logimage;
pub mod table;
pub mod tables;
pub mod tz;
pub mod token;
pub mod forget;
pub mod lang;
// DG10: crypto-shredding for new logs; off by default (see Cargo.toml `shred`).
#[cfg(feature = "shred")]
pub mod shred;
#[cfg(test)] mod store_tests; // W-AUDIT S1/S2: named corrupted cells in the store beneath the hub

// The order log itself, split by what each part is about. Private modules:
// every public name is re-exported below at the path it always had.
mod chain;
mod error;
mod event;
mod log;
mod read;
mod rotate;
mod usage;
#[cfg(test)]
mod tests;

pub use chain::ChainCheck;
pub use error::HubError;
pub use event::{Event, EventKind, Quarantined};
pub use usage::Usage;
pub(crate) use chain::{chain_is_whole, content_id_chained, hex32};
pub(crate) use error::e_is_full;
pub(crate) use event::{decode, decode_or_reason};
pub(crate) use usage::{ceiling_cells, usage_of, usage_of_kind};

use bebop_store::Store;

/// Default image size for a fresh hub. 4 MiB holds a few thousand orders at the
/// measured 47 cells per append, and the image is only as large as it is written.
pub const DEFAULT_IMAGE_BYTES: usize = 4 * 1024 * 1024;

/// THE REFUSAL POINT of every compacted image and table: 10 MiB (operator,
/// 2026-09-25). A CEILING, not an allocation: those images start at 16 KiB,
/// double only when an entry does not fit, and persist only what is live, so a
/// high ceiling costs nothing until it is used. The append logs (`Hub`,
/// `StockLog`, `LogImage`) have no ceiling at all -- they grow by doubling --
/// and their `DEFAULT_*_BYTES` are birth sizes, deliberately NOT tied to this.
pub const CEILING_BYTES: usize = 10 * 1024 * 1024;

/// One hub's store image, in memory.
pub struct Hub {
    store: Store,
}
