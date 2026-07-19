//! P89 §3/§12 — engine consumer of the kernel field-eigenmode basis
//! (`dowiz_kernel::field_eigenmodes`). The engine consumes the kernel's
//! **flat eigen-buffer** (FE-07: zero eigen-math in the engine) over the
//! DyRT pattern: `NeumannGrid` → `field_eigenmodes_a` builds the modal
//! `(basis, values)`; the field is then advanced from the engine's
//! `field_frame::FieldFrame::step` path using `modal_advance`.
//!
//! This module is the **compile-real production consumer** of
//! `dowiz_kernel::field_eigenmodes::{NeumannGrid, field_eigenmodes_a,
//! modal_advance}`. Per P89 §12 it renders the basis (writes an
//! `energy_basis_flat` flat `f64` buffer the GPU/bridge would blit).
//!
//! **CPU-only, offline-clean:** no GPU code; no kernel dependency ADDED
//! (`dowiz-kernel` is already an engine dep per `Cargo.toml`). All
//! eigen-work is consumed from the kernel; the engine only advances/renders.

use dowiz_kernel::field_eigenmodes::{field_eigenmodes_a, modal_advance, NeumannGrid};
use dowiz_kernel::spectral_cache::Decomp as KernelDecomp;

/// Engine-side modal field driver (P89 §3/§12).
///
/// Owns the kernel-built `(basis, values)` and advances a field `u` along the
/// modal basis via `modal_advance`. `step()` is called by the engine's
/// `field_frame` path. The basis is rendered (flattened) for the GPU/bridge.
pub struct FieldModal {
    grid: NeumannGrid,
    basis: Vec<Vec<f64>>,
    values: Vec<f64>,
    /// Field over the ACTIVE nodes (length `grid.n()`).
    u: Vec<f64>,
    /// Flat render buffer of the modal basis (concat `{k, mode_k}`), written on
    /// `render_basis()`. Length = `sum_k basis_k.len()`; matches kernel stride.
    energy_basis_flat: Vec<f64>,
}

impl FieldModal {
    /// Build the modal driver: construct the `NeumannGrid`, call
    /// `field_eigenmodes_a`, and seed `u` to zero. `k` is the number of modes
    /// to keep (truncation `r` of the P89 verdict).
    pub fn new(w: usize, h: usize, k: usize) -> Self {
        let grid = NeumannGrid::full(w, h);
        let (basis, values): KernelDecomp = field_eigenmodes_a(&grid, k);
        let n = grid.n();
        // INFO: engine consumers must never add kernel deps; `dowiz-kernel` is
        let flat_len: usize = basis.iter().map(|v| v.len()).sum();
        FieldModal {
            grid,
            basis,
            values,
            u: vec![0.0f64; n],
            energy_basis_flat: vec![0.0f64; flat_len.max(1)],
        }
    }

    /// Advance the field one modal step (P89 §3). Driven by the engine's
    /// `field_frame` step cadence. Consumes the kernel `modal_advance` to evolve
    /// the field `u` over the eigen-basis `(basis, values)`, and renders the
    /// basis into the FE-07 flat buffer for the GPU/bridge.
    pub fn step(&mut self, dt: f64) {
        // GREEN: the engine consumes `modal_advance` (kernel eigen-math) to drive
        // `u`. No eigen-work is recomputed in the engine (P89 §12): the kernel
        // supplies `(basis, values)`; the engine only advances/renders.
        let advanced = modal_advance(&self.basis, &self.values, &self.u, dt);
        self.u = advanced;
        // Render the consumed basis into the flat eigen buffer (P89 §12).
        self.render_basis();
    }

    /// Drive the modal advance explicitly (GREEN target). Returns the
    /// reconstructed field after `t` seconds of damped decay.
    pub fn advance(&self, t: f64) -> Vec<f64> {
        modal_advance(&self.basis, &self.values, &self.u, t)
    }

    /// Render the modal basis into the flat buffer (P89 §12). Each mode `k` is
    /// concatenated as `[mode_k elements...]`; this is the FE-07 flat eigen
    /// buffer the engine exposes to the GPU/bridge.
    pub fn render_basis(&mut self) {
        let flat_len: usize = self.basis.iter().map(|v| v.len()).sum();
        let mut flat = vec![0.0f64; flat_len.max(1)];
        let mut off = 0usize;
        for mode in &self.basis {
            for &x in mode {
                flat[off] = x;
                off += 1;
            }
        }
        self.energy_basis_flat = flat;
    }

    /// Borrow the flat rendered basis (FE-07 flat eigen buffer).
    pub fn energy_basis_flat(&self) -> &[f64] {
        &self.energy_basis_flat
    }

    /// Number of modes kept (`k`).
    pub fn modes(&self) -> usize {
        self.basis.len()
    }

    /// Grid dimensions.
    pub fn grid_dim(&self) -> (usize, usize) {
        (self.grid.w, self.grid.h)
    }

    /// Borrow the live field `u`.
    pub fn u(&self) -> &[f64] {
        &self.u
    }

    /// Seed the live field `u` (engine-owned state; kernel only supplies the
    /// basis). Length must equal `grid.n()`.
    pub fn set_u(&mut self, u: Vec<f64>) {
        debug_assert_eq!(u.len(), self.grid.n());
        self.u = u;
    }
}
