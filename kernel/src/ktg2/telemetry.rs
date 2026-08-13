//! Allocation-free counters for every KTG-2 core layer.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TelemetrySnapshot {
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
    pub poison_events: u64,
    pub resource_leases: u64,
    pub resource_releases: u64,
    pub lease_failures: u64,
}

impl TelemetrySnapshot {
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
}

#[derive(Debug, Default)]
pub struct KernelTelemetry {
    counters: TelemetrySnapshot,
}

impl KernelTelemetry {
    pub const STATIC_BYTES: usize = core::mem::size_of::<TelemetrySnapshot>();

    pub const fn new() -> Self {
        Self {
            counters: TelemetrySnapshot {
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
                poison_events: 0,
                resource_leases: 0,
                resource_releases: 0,
                lease_failures: 0,
            },
        }
    }

    pub const fn snapshot(&self) -> TelemetrySnapshot {
        self.counters
    }

    pub fn record_elapsed_ns(&mut self, elapsed: u64) {
        self.counters.elapsed_ns = self.counters.elapsed_ns.saturating_add(elapsed);
    }

    pub fn record_payload_bytes(&mut self, read: u64, written: u64) {
        self.counters.payload_bytes_read = self.counters.payload_bytes_read.saturating_add(read);
        self.counters.payload_bytes_written =
            self.counters.payload_bytes_written.saturating_add(written);
    }

    pub fn record_tile_fire(
        &mut self,
        nodes: u64,
        slots: u64,
        zero_skips: u64,
        adds: u64,
        subs: u64,
    ) {
        self.counters.tile_fires = self.counters.tile_fires.saturating_add(1);
        self.counters.node_fires = self.counters.node_fires.saturating_add(nodes);
        self.counters.edge_transfers = self.counters.edge_transfers.saturating_add(slots);
        self.counters.compute_slots = self.counters.compute_slots.saturating_add(slots);
        self.counters.zero_skips = self.counters.zero_skips.saturating_add(zero_skips);
        self.counters.add_ops = self.counters.add_ops.saturating_add(adds);
        self.counters.sub_ops = self.counters.sub_ops.saturating_add(subs);
    }

    pub fn record_poison(&mut self) {
        self.counters.poison_events = self.counters.poison_events.saturating_add(1);
    }

    pub(crate) fn record_lease(&mut self) {
        self.counters.resource_leases = self.counters.resource_leases.saturating_add(1);
    }

    pub(crate) fn record_release(&mut self) {
        self.counters.resource_releases = self.counters.resource_releases.saturating_add(1);
    }

    pub(crate) fn record_lease_failure(&mut self) {
        self.counters.lease_failures = self.counters.lease_failures.saturating_add(1);
    }
}
