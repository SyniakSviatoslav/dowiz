//! How much of an image is spent, measured against the point at which a write
//! is refused. One gauge for every image kind in this crate; see `Usage`.

use bebop_store::Store;

/// How much of an image is spent, and on what.
///
/// THE ARENA IS THE LIMIT NOBODY SEES UNTIL IT BITES. A bebop store is
/// append-only: every commit allocates a new generation and the old one is
/// never reclaimed, so an image is spent by the NUMBER OF WRITES as much as by
/// the data. `settings.rs` measured it — 313 empty commits before a fresh
/// roster refused — and the hub answers `arena_full` when it happens. By then
/// the venue is mid-service and an order is being refused.
///
/// MEASURED AGAINST THE CEILING, NOT THE CAPACITY, and the difference is the
/// whole reason this type is not two fields. The images fall into two kinds:
///
///   - The append logs (`Hub`, `StockLog`) persist with `store.to_bytes()`, so
///     the capacity they were created with is the capacity they keep. It fills,
///     and when it is full the write is refused. Ceiling == capacity.
///   - The KV images (`Catalog`, `Settings`, `Posts`) persist with
///     `compacted_bytes_fit`, which commits the live entries into a FRESH image
///     sized by doubling from 16 KiB. Their capacity is re-chosen on every save,
///     so `used/capacity` is a sawtooth: it climbs toward full, the next save
///     doubles the capacity, and it drops by half. Measured that way a healthy
///     image reads 942 per mille and the reading falls to 517 the moment it
///     grows — a gauge that cries full at something with nothing to reclaim.
///     What actually refuses them is `compacted_bytes_fit` running out of
///     doublings at `DEFAULT_*_BYTES`. That is the ceiling.
///
/// So `used_per_mille` answers one question for both kinds — how close is this
/// image to the write it will refuse — and `capacity_cells` is kept beside it
/// as the raw fact, not as the denominator.
///
/// THERE IS NO `dead` FIGURE HERE ON PURPOSE. The superblock carries a
/// `superseded_cells` column and this write path never writes it: `Tx::sup_delta`
/// is initialised to zero in `Store::begin` and nothing increments it, so
/// `live_cells` is really "every cell ever allocated" and superseded is flatly 0
/// in every image this crate produces. A `dead_per_mille` built on it would
/// return 0 forever while reading like a measurement. It is left out rather
/// than shipped as a column that cannot move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub used_cells: i64,
    /// What the image was built with. For a KV image this is re-chosen on every
    /// save; see the type docs before using it as a denominator.
    pub capacity_cells: i64,
    /// The largest this image may ever grow to — the point at which a write is
    /// refused. This is the denominator.
    pub ceiling_cells: i64,
    /// Does this image DOUBLE itself rather than refuse a write?
    ///
    /// THE TWO KINDS REFUSE DIFFERENTLY AND ONLY ONE OF THEM REFUSES AT ALL.
    /// The append logs (`Hub`, `StockLog`) copy their chain into an image twice
    /// the size when one fills, so "how full" is a sawtooth that predicts a
    /// doubling, not a failure — measured: a stock log went from 7168 cells to
    /// 523264 over four thousand events without once refusing. The compacted KV
    /// images do NOT grow past `DEFAULT_*_BYTES`; when they fill, the write is
    /// refused for real.
    ///
    /// A verdict built from all five treats an imminent doubling as an
    /// emergency and says "compact" about an image that cannot be compacted.
    pub grows: bool,
    pub generation: i64,
}

impl Usage {
    /// Tenths of a percent of the CEILING, so a caller needs no float to render it.
    pub fn used_per_mille(&self) -> i64 {
        if self.ceiling_cells <= 0 {
            return 0;
        }
        (self.used_cells * 1000) / self.ceiling_cells
    }
}

/// The usable cells in an image of `bytes` bytes: the arena is what lies past
/// the two superblocks, exactly as `Store::create_bytes` computes it.
pub(crate) fn ceiling_cells(bytes: usize) -> i64 {
    (bytes / 8) as i64 - bebop_store::ARENA as i64
}

pub(crate) fn usage_of(store: &Store, ceiling_cells: i64) -> Usage {
    usage_of_kind(store, ceiling_cells, false)
}

pub(crate) fn usage_of_kind(store: &Store, ceiling_cells: i64, grows: bool) -> Usage {
    match store.pick() {
        Some(sb) => Usage {
            // `arena_used` is an absolute cell index; the arena starts at 1024,
            // so the cells actually spent are what lies past that.
            used_cells: (sb.arena_used - bebop_store::ARENA as i64).max(0),
            capacity_cells: store.capacity_cells(),
            ceiling_cells,
            grows,
            generation: sb.generation,
        },
        None => Usage {
            used_cells: 0,
            capacity_cells: 0,
            ceiling_cells,
            grows,
            generation: 0,
        },
    }
}

#[cfg(test)]
mod tests;
