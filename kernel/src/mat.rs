//! mat.rs — contiguous (row-major) dense matrix helper for the kernel.
//!
//! The spectral/absorbing subsystems historically used `Vec<Vec<f64>>` (a
//! pointer-chasing vector of heap rows). For the data-oriented + SIMD invariant
//! we want a single contiguous `Vec<f64>` laid out row-major so a matmul walks
//! linear memory and auto-vectorizes. This module is the ONE backing store and
//! the ONE matmul implementation; the `&[Vec<f64>]` entry points in
//! `spectral`/`absorbing` convert at the boundary and stay for wasm/API compat.
//!
//! Zero-dep, plain `std`, deterministic. Small accessors are `#[inline]`.

/// Row-major dense matrix backed by a single contiguous `Vec<f64>`.
///
/// Layout: element `(i, j)` lives at `data[i * ncols + j]`. No per-row heap
/// allocation, so a matmul strides cache-friendly linear memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Mat {
    nrows: usize,
    ncols: usize,
    data: Vec<f64>,
}

impl Mat {
    /// Construct an `r × c` zero matrix.
    #[inline]
    pub fn zeros(nrows: usize, ncols: usize) -> Self {
        Self {
            nrows,
            ncols,
            data: vec![0.0; nrows * ncols],
        }
    }

    /// Identity matrix of dimension `n`.
    #[inline]
    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.set(i, i, 1.0);
        }
        m
    }

    #[inline]
    pub fn nrows(&self) -> usize {
        self.nrows
    }

    #[inline]
    pub fn ncols(&self) -> usize {
        self.ncols
    }

    /// Read element `(i, j)`.
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[i * self.ncols + j]
    }

    /// Write element `(i, j)`.
    #[inline]
    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.data[i * self.ncols + j] = v;
    }

    /// Build from a `Vec<Vec<f64>>` (row-major, rectangular allowed).
    /// Panics if rows are ragged — callers pass well-formed matrices.
    pub fn from_vecvec(m: &[Vec<f64>]) -> Self {
        let nrows = m.len();
        let ncols = m.first().map_or(0, |r| r.len());
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in m {
            debug_assert_eq!(row.len(), ncols, "ragged matrix in from_vecvec");
            data.extend_from_slice(row);
        }
        Self { nrows, ncols, data }
    }

    /// Materialize back to `Vec<Vec<f64>>` for backwards-compatible returns
    /// (wasm surface, tests, external `&[Vec<f64>]` consumers).
    pub fn into_vecvec(self) -> Vec<Vec<f64>> {
        let mut out = Vec::with_capacity(self.nrows);
        for i in 0..self.nrows {
            out.push(self.data[i * self.ncols..(i + 1) * self.ncols].to_vec());
        }
        out
    }
}

/// The single contiguous matrix product. `a` is `(m × k)`, `b` is `(k × n)`,
/// producing `(m × n)`. The `aik == 0` short-circuit is preserved so sparse-ish
/// rows skip the inner loop exactly as the old `Vec<Vec<f64>>` impl did.
pub fn matmul_contig(a: &Mat, b: &Mat) -> Mat {
    let m = a.nrows;
    let k = a.ncols;
    let n = b.ncols;
    debug_assert_eq!(k, b.nrows, "matmul_contig: inner dims must agree");
    let mut c = Mat::zeros(m, n);
    for i in 0..m {
        for kk in 0..k {
            let aik = a.get(i, kk);
            if aik == 0.0 {
                continue;
            }
            for j in 0..n {
                let cij = c.get(i, j) + aik * b.get(kk, j);
                c.set(i, j, cij);
            }
        }
    }
    c
}
