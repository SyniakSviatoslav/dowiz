//! decision/import.rs — kernel-side shim: the import gate now lives in
//! `dowiz_core::decision::import` (no_std). Re-export it so `crate::decision::import::…`
//! callers are unaffected.
pub use dowiz_core::decision::import;
