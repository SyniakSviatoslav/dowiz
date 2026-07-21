//! `kernel::parse` — kernel-native structured data extraction (pure `std`).
//!
//! Replaces shell/python parsing primitives with deterministic, zero-dep kernel
//! functions. Every function is pure-`std`, testable in-process, and returns
//! `Result` (never panics on malformed input).
//!
//! | Module | Replaces | Pattern |
//! |--------|----------|---------|
//! | `tsv`  | `awk -F'\t'` in 5+ scripts | `parse_rows(src, n_cols) -> Vec<Vec<&str>>` |
//! | `env`  | `split('=')` in TS scripts | `parse_env(src) -> EnvMap` |

pub mod env;
pub mod tsv;
