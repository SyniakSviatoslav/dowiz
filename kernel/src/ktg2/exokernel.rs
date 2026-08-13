//! Minimal exokernel resource protection for KTG-2 graph execution.
//!
//! The kernel validates and leases physical graph resources. Applications own
//! scheduling policy and graph construction; no high-level abstraction is
//! forced into the privileged core.

use super::telemetry::TelemetryStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    GraphNodes,
    Tile2x2,
    StreamCredits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lease {
    slot: u32,
    generation: u32,
    units: u32,
}

impl Lease {
    pub const fn units(self) -> u32 {
        self.units
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseError {
    ZeroUnits,
    StaleLease,
}

#[derive(Debug, Clone, Copy)]
struct LeaseSlot {
    generation: u32,
    active: bool,
    kind: ResourceKind,
    units: u32,
}

pub struct ExoKernel {
    slots: Vec<LeaseSlot>,
    stats: TelemetryStats,
}

impl ExoKernel {
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            stats: TelemetryStats::new(),
        }
    }

    pub fn lease(&mut self, kind: ResourceKind, units: u32) -> Result<Lease, LeaseError> {
        if units == 0 {
            self.stats.record_lease_failure();
            return Err(LeaseError::ZeroUnits);
        }
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if !slot.active {
                slot.active = true;
                slot.kind = kind;
                slot.units = units;
                slot.generation = slot.generation.wrapping_add(1).max(1);
                self.stats.record_lease();
                return Ok(Lease {
                    slot: index as u32,
                    generation: slot.generation,
                    units,
                });
            }
        }
        let generation = 1;
        let slot = self.slots.len() as u32;
        self.slots.push(LeaseSlot {
            generation,
            active: true,
            kind,
            units,
        });
        self.stats.record_lease();
        Ok(Lease {
            slot,
            generation,
            units,
        })
    }

    pub fn release(&mut self, lease: Lease) -> Result<(), LeaseError> {
        let Some(slot) = self.slots.get_mut(lease.slot as usize) else {
            self.stats.record_lease_failure();
            return Err(LeaseError::StaleLease);
        };
        if !slot.active || slot.generation != lease.generation || slot.units != lease.units {
            self.stats.record_lease_failure();
            return Err(LeaseError::StaleLease);
        }
        slot.active = false;
        self.stats.record_release();
        Ok(())
    }

    pub const fn stats(&self) -> &TelemetryStats {
        &self.stats
    }
}

impl Default for ExoKernel {
    fn default() -> Self {
        Self::new()
    }
}
