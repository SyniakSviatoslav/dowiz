//! chronos.rs — std host shim. The pure time-navigation engine (`Snapshot`,
//! `Chronos`) lives in `dowiz_core::chronos`. `Chronos::snapshot_at` takes
//! `now_ms` explicitly; here it is wrapped as `snapshot_now` which stamps
//! `crate::now_ms()`. (`Chronos::new` is clock-free and used directly.)

pub use dowiz_core::chronos::*;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// `Chronos::snapshot_at` stamped with the current wall clock.
pub fn snapshot_now(
    chronos: &mut Chronos,
    values: BTreeMap<String, f64>,
) -> &Snapshot {
    chronos.snapshot_at(values, crate::now_ms())
}
