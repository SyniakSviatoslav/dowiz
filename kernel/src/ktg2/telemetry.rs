//! Allocation-free counters for every KTG-2 core layer.
//!
//! One canonical stats type ([`TelemetryStats`]). The hot path updates inline
//! `u64` counters only; snapshot/merge/aggregation happen outside the hot path.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TelemetryStats {
    pub elapsed_ns: u64,
    pub payload_bytes_read: u64,
    pub payload_bytes_written: u64,
    pub tile_fires: u64,
    pub node_fires: u64,
    pub edge_transfers: u64,
    pub compute_slots: u64,
    pub zero_skips: u64,
    pub add_ops: u64,
    pub sub_ops: u64,
    pub invalid_encodings: u64,
    pub resource_leases: u64,
    pub resource_releases: u64,
    pub lease_failures: u64,
}

impl TelemetryStats {
    /// Byte size of the canonical stats struct (used to assert no heap growth
    /// on the hot path — `size_of_val(&stats) == STATIC_BYTES`).
    pub const STATIC_BYTES: usize = core::mem::size_of::<Self>();

    #[inline]
    pub const fn new() -> Self {
        Self {
            elapsed_ns: 0,
            payload_bytes_read: 0,
            payload_bytes_written: 0,
            tile_fires: 0,
            node_fires: 0,
            edge_transfers: 0,
            compute_slots: 0,
            zero_skips: 0,
            add_ops: 0,
            sub_ops: 0,
            invalid_encodings: 0,
            resource_leases: 0,
            resource_releases: 0,
            lease_failures: 0,
        }
    }

    #[inline]
    pub const fn bytes_moved(self) -> u64 {
        self.payload_bytes_read.saturating_add(self.payload_bytes_written)
    }

    #[inline]
    pub const fn ops(self) -> u64 {
        self.compute_slots
    }

    #[inline]
    pub fn ops_per_second(self) -> u64 {
        if self.elapsed_ns == 0 {
            return 0;
        }
        ((self.ops() as u128 * 1_000_000_000u128) / self.elapsed_ns as u128) as u64
    }

    #[inline]
    pub fn record_elapsed_ns(&mut self, elapsed: u64) {
        self.elapsed_ns = self.elapsed_ns.saturating_add(elapsed);
    }

    #[inline]
    pub fn record_payload_bytes(&mut self, read: u64, written: u64) {
        self.payload_bytes_read = self.payload_bytes_read.saturating_add(read);
        self.payload_bytes_written = self.payload_bytes_written.saturating_add(written);
    }

    #[inline]
    pub fn record_tile_fire(
        &mut self,
        nodes: u64,
        slots: u64,
        zero_skips: u64,
        adds: u64,
        subs: u64,
    ) {
        self.tile_fires = self.tile_fires.saturating_add(1);
        self.node_fires = self.node_fires.saturating_add(nodes);
        self.edge_transfers = self.edge_transfers.saturating_add(slots);
        self.compute_slots = self.compute_slots.saturating_add(slots);
        self.zero_skips = self.zero_skips.saturating_add(zero_skips);
        self.add_ops = self.add_ops.saturating_add(adds);
        self.sub_ops = self.sub_ops.saturating_add(subs);
    }

    #[inline]
    pub fn record_invalid_encoding(&mut self) {
        self.invalid_encodings = self.invalid_encodings.saturating_add(1);
    }

    pub(crate) fn record_lease(&mut self) {
        self.resource_leases = self.resource_leases.saturating_add(1);
    }

    pub(crate) fn record_release(&mut self) {
        self.resource_releases = self.resource_releases.saturating_add(1);
    }

    pub(crate) fn record_lease_failure(&mut self) {
        self.lease_failures = self.lease_failures.saturating_add(1);
    }
}
