//! span_metrics — pure no_std log-bucket histogram core.
//!
//! The pure, no-std parts of P83 kernel production observability: log-bucket
//! histogram buckets, breach action types, and diagnostic formatters. The std
//! parts (observer, file I/O, perf capture) stay in the kernel shim at
//! `kernel/src/span_metrics/`.
//!
//! ZERO dependencies — pure `core::` + `alloc::`.

pub mod breach;
pub mod obs;