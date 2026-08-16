//! 2×2 systolic tile: four PEs in a NW/NE/SW/SE grid.
//!
//! Phase-0 datapath is a **node-gated east-difference**:
//!
//!   output[cell] = payload[cell]                       if node is Unknown (idle)
//!                = payload[cell] − payload[east]       if node is True/False AND
//!                                                       it has an east neighbour
//!                = payload[cell]                       if node is True/False AND
//!                                                       it has no east neighbour
//!
//! Each PE has two input latches (north/east), hence 2 compute slots per PE and
//! 2 edge transfers per latch. The weight quad is a **packed 2-bit signed-trit
//! latch** carried alongside (4 weights × 2 bits = 1 byte) — phase-1 feeds it
//! into the signed MAC (add/sub/skip). Phase-0 carries it without sign-flipping;
//! that is a named ceiling, not a hidden omission.

use super::cell::State;
use super::graph::{Graph, NodeQuad};
use super::telemetry::TelemetryStats;

/// Inline payload of a 2×2 tile (NW, NE, SW, SE), wide accumulator width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadQuad {
    pub north_west: i32,
    pub north_east: i32,
    pub south_west: i32,
    pub south_east: i32,
}

impl PayloadQuad {
    pub const fn new(north_west: i32, north_east: i32, south_west: i32, south_east: i32) -> Self {
        Self {
            north_west,
            north_east,
            south_west,
            south_east,
        }
    }

    pub const fn as_array(&self) -> [i32; 4] {
        [
            self.north_west,
            self.north_east,
            self.south_west,
            self.south_east,
        ]
    }
}

/// Packed 2-bit signed-trit weights for the tile (4 weights = 1 byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightQuad {
    pub north_west: State,
    pub north_east: State,
    pub south_west: State,
    pub south_east: State,
}

impl WeightQuad {
    pub const fn new(
        north_west: State,
        north_east: State,
        south_west: State,
        south_east: State,
    ) -> Self {
        Self {
            north_west,
            north_east,
            south_west,
            south_east,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowResult {
    Values(PayloadQuad),
    InvalidEncoding,
}

#[derive(Debug, Clone, Copy)]
pub struct Tile2x2 {
    nodes: NodeQuad,
    /// Phase-2 latch weights (4 × 2-bit = 1 byte), carried alongside for API
    /// completeness. Not consumed by `fire` (phase-1); phase-2 reads them.
    #[allow(dead_code)]
    weights: WeightQuad,
}

// East-neighbour map for the NW/NE/SW/SE iteration order: NW→NE, SW→SE;
// NE and SE have no east neighbour.
const EAST: [Option<usize>; 4] = [Some(1), None, Some(3), None];

impl Tile2x2 {
    pub const fn new(nodes: NodeQuad, weights: WeightQuad) -> Self {
        Self { nodes, weights }
    }

    pub const fn weight_payload_bytes(&self) -> usize {
        1 // 4 weights × 2 bits
    }

    pub fn fire(
        &self,
        graph: &Graph,
        payload: PayloadQuad,
        stats: &mut TelemetryStats,
    ) -> FlowResult {
        stats.tile_fires = stats.tile_fires.saturating_add(1);
        stats.node_fires = stats.node_fires.saturating_add(4);
        stats.edge_transfers = stats.edge_transfers.saturating_add(8);
        stats.compute_slots = stats.compute_slots.saturating_add(8);

        // Resolve the four node states (tiles reference nodes by NodeId — they
        // never duplicate node state).
        let mut states = [State::Unknown; 4];
        for (i, id) in self.nodes.ids().iter().enumerate() {
            match graph.state(*id) {
                Some(s) => states[i] = s,
                None => {
                    stats.record_invalid_encoding();
                    return FlowResult::InvalidEncoding;
                }
            }
        }

        let p = payload.as_array();
        let mut out = [0i32; 4];
        for i in 0..4 {
            match states[i] {
                State::Unknown => {
                    out[i] = p[i];
                    stats.zero_skips = stats.zero_skips.saturating_add(2);
                }
                State::True | State::False => match EAST[i] {
                    Some(e) => {
                        out[i] = p[i] - p[e];
                        stats.add_ops = stats.add_ops.saturating_add(1);
                        stats.sub_ops = stats.sub_ops.saturating_add(1);
                    }
                    None => {
                        out[i] = p[i];
                        stats.add_ops = stats.add_ops.saturating_add(2);
                    }
                },
            }
        }

        FlowResult::Values(PayloadQuad::new(out[0], out[1], out[2], out[3]))
    }
}
