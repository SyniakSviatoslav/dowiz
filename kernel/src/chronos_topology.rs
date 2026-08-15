//! chronos_topology.rs — std host shim. The pure chrono-topological foundation
//! (`CT4`, `TemporalTrinity`, `ChronoTopology`) lives in
//! `dowiz_core::chronos_topology`. The wall-clock-stamped entry points are
//! wrapped here as free functions that stamp `crate::now_ms()`.
//! (`ChronoTopology::new` is clock-free and is used directly.)

pub use dowiz_core::chronos_topology::*;

/// `TemporalTrinity::new` stamped with the current wall clock.
pub fn trinity_new(rows: usize, cols: usize) -> TemporalTrinity {
    TemporalTrinity::new(rows, cols, crate::now_ms())
}

/// `TemporalTrinity::advance` stamped with the current wall clock.
pub fn trinity_advance(trinity: &mut TemporalTrinity, new_present: crate::trinary::TriMatrix) {
    trinity.advance(new_present, crate::now_ms());
}

/// `ChronoTopology::register` stamped with the current wall clock.
pub fn topology_register(topology: &mut ChronoTopology, name: &str, rows: usize, cols: usize) {
    topology.register(name, rows, cols, crate::now_ms());
}

/// `ChronoTopology::update` stamped with the current wall clock.
pub fn topology_update(topology: &mut ChronoTopology, name: &str, matrix: crate::trinary::TriMatrix) {
    topology.update(name, matrix, crate::now_ms());
}

/// `ChronoTopology::navigate` stamped with the current wall clock.
pub fn topology_navigate(topology: &mut ChronoTopology, row: u32, col: u32, ts: u64) {
    topology.navigate(row, col, ts, crate::now_ms());
}
