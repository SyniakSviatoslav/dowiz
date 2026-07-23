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
//!
//! ## Cache-aware block size
//!
//! [`MAT_BLOCK_SIZE`] = 128 is tuned for the EPYC-Milan target (32 MB L3,
//! 512 KB L2 per core). Rationale: three 128×128 f64 blocks (A, B, C) fit in
//! L2: 128×128×8 bytes = 128 KiB per block; 3 blocks = 384 KiB < 512 KiB L2.
//! Consumers using tiled/blocked matmul should use this constant as the tile
//! dimension so the working set stays cache-resident and avoids L3 thrashing.
//!
//! innovate: default block tuned for EPYC-Milan 32MB L3 / 512KB L2 per core.
//! Upgrade trigger: different CPU topology (different L2/L3 sizes) → recompute
//! via `cpuid::detect()` and select at runtime from a small table of known sizes.

/// Block size for tiled/blocked matrix multiply (cache-aware).
///
/// Tuned for EPYC-Milan: 128×128×8B = 128 KiB per block, 3 blocks (A, B, C)
/// = 384 KiB < 512 KiB L2 per core. This keeps the tile working set in L2
/// and avoids L3 thrash. Consumers that tile their matmul (e.g. blocked
/// `matmul_contig` variants or spectral dgemm paths) should use this as
/// the tile dimension.
pub const MAT_BLOCK_SIZE: usize = 128;

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

/// Error returned when a matrix cannot be built fail-closed.
#[derive(Debug, Clone, PartialEq)]
pub enum MatrixError {
    /// Rows are not all the same length (ragged) → `get(i,j)` would stride
    /// out of a short row's bound (index-leak / OOB read or release panic).
    Ragged,
    /// A non-finite entry (NaN poison, ±inf overflow). Root-cause of the
    /// spectral NaN-fail-open: a poisoned spectrum must not read as healthy.
    NonFinite,
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

    /// Build from a `Vec<Vec<f64>>` (row-major, rectangular required).
    ///
    /// FAIL-CLOSED: returns `Err` on a ragged matrix or any non-finite entry,
    /// instead of building a `Mat` whose `get(i,j)` would read out of bounds
    /// (index-leak) or silently mis-index. Prefer this over [`Mat::from_vecvec`]
    /// at every external / untrusted input boundary.
    pub fn from_vecvec_checked(m: &[Vec<f64>]) -> Result<Mat, MatrixError> {
        let nrows = m.len();
        let ncols = m.first().map_or(0, |r| r.len());
        for row in m {
            if row.len() != ncols {
                return Err(MatrixError::Ragged);
            }
            for &x in row {
                if !x.is_finite() {
                    return Err(MatrixError::NonFinite);
                }
            }
        }
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in m {
            data.extend_from_slice(row);
        }
        Ok(Mat { nrows, ncols, data })
    }

    /// Build from a `Vec<Vec<f64>>` (row-major, rectangular allowed).
    /// Panics if rows are ragged (callers pass well-formed matrices only) — for
    /// internal/trusted construction. Untrusted input must use
    /// [`from_vecvec_checked`] instead.
    pub fn from_vecvec(m: &[Vec<f64>]) -> Mat {
        let nrows = m.len();
        let ncols = m.first().map_or(0, |r| r.len());
        let mut data = Vec::with_capacity(nrows * ncols);
        for row in m {
            debug_assert_eq!(row.len(), ncols, "ragged matrix in from_vecvec");
            data.extend_from_slice(row);
        }
        Mat { nrows, ncols, data }
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

/// Arena-aware twin of [`matmul_contig`] (W5 — the dense `charpoly` scratch
/// path). The result matrix `c` (n² `f64`) is served from `arena` during the
/// multiply; on exhaustion (`alloc_slice` returns `None`) it degrades to the
/// heap [`matmul_contig`] (same bytes, never a panic). Byte-identical output
/// guaranteed — the arena moves where the scratch lives, never the multiply
/// order. The returned `Mat` owns its `Vec` (arena memory cannot outlive the
/// loan), so the win is keeping the transient n×n product buffer off the heap
/// for the duration of the call.
pub fn matmul_contig_in(a: &Mat, b: &Mat, arena: &crate::arena::BumpArena) -> Option<Mat> {
    let m = a.nrows;
    let k = a.ncols;
    let n = b.ncols;
    debug_assert_eq!(k, b.nrows, "matmul_contig_in: inner dims must agree");
    let scratch: &mut [f64] = arena.alloc_slice(m * n)?;
    for v in scratch.iter_mut() {
        *v = 0.0;
    }
    for i in 0..m {
        for kk in 0..k {
            let aik = a.get(i, kk);
            if aik == 0.0 {
                continue;
            }
            let row_base = i * n;
            for j in 0..n {
                scratch[row_base + j] += aik * b.get(kk, j);
            }
        }
    }
    let mut c = Mat::zeros(m, n);
    c.data.copy_from_slice(scratch);
    Some(c)
}
