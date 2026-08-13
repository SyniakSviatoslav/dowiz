//! Packed graph-node state store. Four node states occupy one byte.

use super::cell::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeQuad {
    pub north_west: NodeId,
    pub north_east: NodeId,
    pub south_west: NodeId,
    pub south_east: NodeId,
}

impl NodeQuad {
    pub const fn new(
        north_west: NodeId,
        north_east: NodeId,
        south_west: NodeId,
        south_east: NodeId,
    ) -> Self {
        Self {
            north_west,
            north_east,
            south_west,
            south_east,
        }
    }

    /// Iteration order used by tiles: NW, NE, SW, SE.
    pub const fn ids(&self) -> [NodeId; 4] {
        [self.north_west, self.north_east, self.south_west, self.south_east]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphError {
    CapacityExhausted,
}

#[derive(Debug, Clone)]
pub struct Graph {
    state_words: Vec<u8>,
    node_count: u32,
    node_capacity: u32,
}

impl Graph {
    pub fn with_node_capacity(capacity: usize) -> Self {
        let capacity = capacity.min(u32::MAX as usize) as u32;
        let bytes = (capacity as usize).saturating_add(3) / 4;
        Self {
            // Unused slots carry `0b11` (the invalid-encoding sentinel), so an
            // out-of-shape read can never look like a valid state.
            state_words: vec![0b11_11_11_11; bytes],
            node_count: 0,
            node_capacity: capacity,
        }
    }

    pub fn add_node(&mut self, state: State) -> Result<NodeId, GraphError> {
        if self.node_count >= self.node_capacity {
            return Err(GraphError::CapacityExhausted);
        }
        let id = NodeId(self.node_count);
        self.write_state(id, state);
        self.node_count += 1;
        Ok(id)
    }

    pub fn state(&self, id: NodeId) -> Option<State> {
        if id.0 >= self.node_count {
            return None;
        }
        let index = id.0 as usize;
        let byte = self.state_words[index >> 2];
        let shift = (index & 3) << 1;
        State::from_bits((byte >> shift) & 0b11).ok()
    }

    pub const fn node_count(&self) -> usize {
        self.node_count as usize
    }

    pub const fn bits_per_node_state(&self) -> usize {
        2
    }

    pub fn state_payload_bytes(&self) -> usize {
        (self.node_count as usize).saturating_add(3) / 4
    }

    pub fn state_capacity_bytes(&self) -> usize {
        self.state_words.len()
    }

    fn write_state(&mut self, id: NodeId, state: State) {
        let index = id.0 as usize;
        let shift = (index & 3) << 1;
        let mask = !(0b11u8 << shift);
        let word = &mut self.state_words[index >> 2];
        *word = (*word & mask) | (state.bits() << shift);
    }
}
