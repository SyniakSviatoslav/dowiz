//! open_science.rs — std host shim. The pure research-workbench logic lives in
//! `dowiz_core::open_science`. The clock-stamped entry points (`add_paper`,
//! `create_notebook`, `generate_report`) take `now_us` explicitly; there are no
//! kernel-side callers, so a caller would just pass `SystemTime::now()…as_micros()`.

pub use dowiz_core::open_science::*;
