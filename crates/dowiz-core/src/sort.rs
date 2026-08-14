//! sort.rs — deterministic float-key sorting helpers.
//!
//! Extracted from the kernel crate root (`sort_by_f64_desc`/`sort_by_f64_asc`)
//! so the no_std `spectral` module can order eigenvalue/spectrum slices without
//! pulling in the kernel. Re-exported at the kernel root so all 17+ existing
//! `crate::sort_by_f64_*` call sites keep resolving unchanged.
//!
//! Zero dependencies, pure `core`.

/// Sort descending by a `f64` key (NaN/Inf → end of order).
///
/// Replaces the repeated `sort_by(|a,b| b.K.partial_cmp(&a.K).unwrap_or(Equal))`
/// pattern. Deterministic: `partial_cmp` is total over the non-NaN path and
/// NaN/Inf are pushed to the end (never compared against each other).
pub fn sort_by_f64_desc<T, K>(items: &mut [T], key: K)
where
    K: Fn(&T) -> f64,
{
    items.sort_by(|a, b| {
        key(b)
            .partial_cmp(&key(a))
            .unwrap_or(core::cmp::Ordering::Equal)
    });
}

/// Sort ascending by a `f64` key (NaN/Inf → end of order).
pub fn sort_by_f64_asc<T, K>(items: &mut [T], key: K)
where
    K: Fn(&T) -> f64,
{
    items.sort_by(|a, b| {
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(core::cmp::Ordering::Equal)
    });
}
