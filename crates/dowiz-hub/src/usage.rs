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
/// THE `dead` FIGURE (W-CRC, 2026-10-05, D.1 #4). Until then nothing on this write path
/// wrote the superblock's `superseded_cells` (`Tx::sup_delta` stayed 0), and the figure was
/// left out rather than shipped as a column that could not move. Now a KV commit counts the
/// root and four arrays it replaces, and a log append counts the root it replaces, so
/// `dead_cells` is the arena a compaction would give back. What it means per kind:
///   - KV images were COMPACTED on every save (`compacted_bytes_fit`), so an image read
///     back from storage read 0. Since W-DELTA (2026-10-06) the CATALOGUE appends a delta
///     record per changed key instead, and each append adds the root it retires plus the
///     cells the edit shadows (bebop_store::kv::delta); it returns to 0 when the writer
///     compacts at a quarter of live cells or 32 records. Settings/posts still compact.
///   - Logs never compact; each append retires one 10-cell root. An image written before
///     W-CRC carries 0 here for all its old roots -- an UNDERCOUNT until it is rebuilt by
///     `grow`. It is not exposed in `/api/owner/health` for that reason (and because no
///     owner action follows from it until the KV delta chain, D.1 #3, compacts by it).
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
    /// Arena cells the image's own commits have superseded (superblock cell 8). See above.
    pub dead_cells: i64,
}

impl Usage {
    /// Tenths of a percent of the CEILING, so a caller needs no float to render it.
    pub fn used_per_mille(&self) -> i64 {
        if self.ceiling_cells <= 0 {
            return 0;
        }
        (self.used_cells * 1000) / self.ceiling_cells
    }

    /// Tenths of a percent of the SPENT arena that is dead -- what a compaction reclaims.
    pub fn dead_per_mille(&self) -> i64 {
        if self.used_cells <= 0 {
            return 0;
        }
        (self.dead_cells.clamp(0, self.used_cells) * 1000) / self.used_cells
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
            dead_cells: sb.superseded_cells.max(0),
        },
        None => Usage {
            used_cells: 0,
            capacity_cells: 0,
            ceiling_cells,
            grows,
            generation: 0,
            dead_cells: 0,
        },
    }
}

#[cfg(test)]
mod tests;
