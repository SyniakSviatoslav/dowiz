//! `dowiz_core::academia_p2p` — minimal metric-tensor stub.
//!
//! The FULL speculative physics-inspired P2P design (`AcademiaMesh`, quantum /
//! relativistic simulation, etc.) is a 5224-line kernel module gated behind
//! `feature = "speculative"` — a feature that is NOT defined in the kernel's
//! `Cargo.toml`, so that code is never compiled. `memory_search` only needs
//! `MetricTensor` + `GEO_DIMS`, so the core carries this minimal Euclidean stub
//! that mirrors what the kernel's `#[cfg(not(feature = "speculative"))]` block
//! previously provided inline.

/// Кількість геометричних вимірів для метричного тензора.
pub const GEO_DIMS: usize = 8;

/// Метричний тензор g_ij (симетричний, додатно визначений),
/// stub-версія — завжди евклідова.
#[derive(Debug, Clone)]
pub struct MetricTensor {
    pub g: [[f64; GEO_DIMS]; GEO_DIMS],
}

impl MetricTensor {
    pub fn euclidean() -> Self {
        let mut g = [[0.0; GEO_DIMS]; GEO_DIMS];
        for i in 0..GEO_DIMS {
            g[i][i] = 1.0;
        }
        MetricTensor { g }
    }
}
