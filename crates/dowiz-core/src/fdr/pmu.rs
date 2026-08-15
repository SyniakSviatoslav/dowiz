//! `fdr/pmu.rs` — PMU companion stamps for the kernel's classifiers (pure no_std core).
//!
//! A [`PmuStamp`] is a sibling of [`crate::fdr::schema::HwStamp`], built from the SAME
//! [`Reading<T>`]/[`Absence`] machinery and the SAME raw-monotone-counters rule: the kernel
//! emits raw counters only; deltas/IPC/miss-rates are a consumer concern (the ONE sanctioned
//! exception is [`PmuStamp::delta`] — a bracketed before/after subtraction).
//!
//! This is the *pure* half: the stamp type + serialization + the absence-propagating delta.
//! The std side — Tier-A `/proc` readers, the `rdtsc` sampler, and the Tier-B
//! `perf_event_open(2)` station — lives in the kernel shim (`dowiz-kernel`'s `fdr::pmu`),
//! which re-exports these types and adds the sampling station on top.

use crate::fdr::json::JsonWriter;
use crate::fdr::schema::{Absence, Reading};

/// A per-classification-window PMU stamp. All fields are `Reading<u64>` and are ALWAYS
/// serialized (value or named absence). Raw monotone counters only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PmuStamp {
    // ── Tier A — zero-permission: rdtsc + /proc page-fault / context-switch counters ──
    /// `rdtsc` timestamp counter (reference cycles, NOT retired core cycles).
    pub tsc_cycles: Reading<u64>,
    /// Minor page faults (`/proc/self/stat` field 10).
    pub minflt: Reading<u64>,
    /// Major page faults (`/proc/self/stat` field 12).
    pub majflt: Reading<u64>,
    /// Swap count (`/proc/self/stat` field 36).
    pub nswap: Reading<u64>,
    /// Voluntary context switches (`/proc/self/status`).
    pub vol_ctxt_switches: Reading<u64>,
    /// Nonvoluntary context switches (`/proc/self/status`).
    pub nonvol_ctxt_switches: Reading<u64>,

    // ── Tier B — perf_event_open(2); on a locked-down host: PermissionDenied ──
    /// Retired instructions (`PERF_COUNT_HW_INSTRUCTIONS`).
    pub hw_instructions: Reading<u64>,
    /// CPU cycles (`PERF_COUNT_HW_CPU_CYCLES`).
    pub hw_cpu_cycles: Reading<u64>,
    /// Last-level cache misses (`PERF_COUNT_HW_CACHE_MISSES`).
    pub hw_cache_misses: Reading<u64>,
    /// Mispredicted branches (`PERF_COUNT_HW_BRANCH_MISSES`).
    pub hw_branch_misses: Reading<u64>,
}

impl PmuStamp {
    /// Serialize as a nested `"pmu":{...}` object onto `w`. Every field is present —
    /// value or `{"unavailable":"<reason>"}` — the same guarantee as `HwStamp::write`.
    pub fn write(self, w: JsonWriter) -> JsonWriter {
        let inner = JsonWriter::obj();
        let inner = self.tsc_cycles.write_field(inner, "tsc_cycles");
        let inner = self.minflt.write_field(inner, "minflt");
        let inner = self.majflt.write_field(inner, "majflt");
        let inner = self.nswap.write_field(inner, "nswap");
        let inner = self
            .vol_ctxt_switches
            .write_field(inner, "vol_ctxt_switches");
        let inner = self
            .nonvol_ctxt_switches
            .write_field(inner, "nonvol_ctxt_switches");
        let inner = self.hw_instructions.write_field(inner, "hw_instructions");
        let inner = self.hw_cpu_cycles.write_field(inner, "hw_cpu_cycles");
        let inner = self.hw_cache_misses.write_field(inner, "hw_cache_misses");
        let inner = self.hw_branch_misses.write_field(inner, "hw_branch_misses");
        w.field_raw("pmu", &inner.finish())
    }

    /// Absence-propagating wrapping delta `end - start`, field by field. A field is a
    /// `Value` only when BOTH endpoints are `Value`; otherwise it carries `end`'s absence.
    /// Raw counts stay raw — a bracketed subtraction is the ONE sanctioned delta.
    pub fn delta(start: PmuStamp, end: PmuStamp) -> PmuStamp {
        fn d(s: Reading<u64>, e: Reading<u64>) -> Reading<u64> {
            match (s, e) {
                (Reading::Value(a), Reading::Value(b)) => Reading::Value(b.wrapping_sub(a)),
                (_, Reading::Unavailable(x)) => Reading::Unavailable(x),
                (Reading::Unavailable(x), _) => Reading::Unavailable(x),
            }
        }
        PmuStamp {
            tsc_cycles: d(start.tsc_cycles, end.tsc_cycles),
            minflt: d(start.minflt, end.minflt),
            majflt: d(start.majflt, end.majflt),
            nswap: d(start.nswap, end.nswap),
            vol_ctxt_switches: d(start.vol_ctxt_switches, end.vol_ctxt_switches),
            nonvol_ctxt_switches: d(start.nonvol_ctxt_switches, end.nonvol_ctxt_switches),
            hw_instructions: d(start.hw_instructions, end.hw_instructions),
            hw_cpu_cycles: d(start.hw_cpu_cycles, end.hw_cpu_cycles),
            hw_cache_misses: d(start.hw_cache_misses, end.hw_cache_misses),
            hw_branch_misses: d(start.hw_branch_misses, end.hw_branch_misses),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn val(v: u64) -> Reading<u64> {
        Reading::Value(v)
    }

    #[test]
    fn delta_subtracts_values_and_propagates_absence() {
        let start = PmuStamp {
            tsc_cycles: val(100),
            minflt: val(10),
            majflt: val(2),
            nswap: val(0),
            vol_ctxt_switches: val(5),
            nonvol_ctxt_switches: val(3),
            hw_instructions: val(1000),
            hw_cpu_cycles: val(900),
            hw_cache_misses: val(40),
            hw_branch_misses: val(20),
        };
        let end = PmuStamp {
            tsc_cycles: val(250),
            minflt: val(12),
            majflt: val(2),
            nswap: val(0),
            vol_ctxt_switches: val(7),
            nonvol_ctxt_switches: val(3),
            hw_instructions: Reading::Unavailable(Absence::PermissionDenied),
            hw_cpu_cycles: val(1900),
            hw_cache_misses: val(44),
            hw_branch_misses: val(21),
        };
        let d = PmuStamp::delta(start, end);
        assert_eq!(d.tsc_cycles, val(150));
        assert_eq!(d.minflt, val(2));
        assert_eq!(d.majflt, val(0));
        assert_eq!(d.vol_ctxt_switches, val(2));
        assert_eq!(d.nonvol_ctxt_switches, val(0));
        // end-side absence propagates to the delta.
        assert_eq!(
            d.hw_instructions,
            Reading::Unavailable(Absence::PermissionDenied)
        );
        assert_eq!(d.hw_cpu_cycles, val(1000));
    }

    #[test]
    fn delta_is_wrapping_subtraction() {
        let start = PmuStamp {
            tsc_cycles: val(u64::MAX - 5),
            minflt: val(0),
            majflt: val(0),
            nswap: val(0),
            vol_ctxt_switches: val(0),
            nonvol_ctxt_switches: val(0),
            hw_instructions: val(0),
            hw_cpu_cycles: val(0),
            hw_cache_misses: val(0),
            hw_branch_misses: val(0),
        };
        let end = PmuStamp {
            tsc_cycles: val(3),
            minflt: val(0),
            majflt: val(0),
            nswap: val(0),
            vol_ctxt_switches: val(0),
            nonvol_ctxt_switches: val(0),
            hw_instructions: val(0),
            hw_cpu_cycles: val(0),
            hw_cache_misses: val(0),
            hw_branch_misses: val(0),
        };
        // (MAX-5) -> 3 wraps to 9.
        assert_eq!(PmuStamp::delta(start, end).tsc_cycles, val(9));
    }
}
