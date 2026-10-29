//! zlib CRC-32 steps: since P12 (W-SPEED-A, 2026-10-29) they live in `bebop-crc` (tables + the
//! CPU's crc instructions); re-exported so `crate::crc::step_*` keeps its meaning.

pub use bebop_crc::{step_bytes, step_cell};
