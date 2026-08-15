//! typed_metrics.rs — std host shim (pure types from `dowiz_core::typed_metrics`,
//! std/platform-dependent impls stay here).
//!
//! The no_std core (`dowiz_core::typed_metrics`) holds the pure data types
//! ([`ProcCpuSample`], [`MemSample`], [`GpuSample`], [`MetricLine`],
//! [`MetricSample`]) and the fixed-field-order `to_line()` / `parse_line()`
//! pair. The std-dependent operations — the monotonic clock and the `/proc`
//! readers — are redefined here (shadowing the no_std fallbacks) so the kernel
//! reads real data:
//! - [`mono_now_ns`] — `std::time::Instant` + `std::sync::OnceLock`.
//! - [`proc_cpu_sample_from_proc_self`] / [`mem_sample_from_proc_self`] —
//!   real `/proc` reads via [`crate::vfs`].
//!
//! The kernel port swaps the *impl* (procfs → ramfs/kernel VFS), never the
//! call sites.

pub use dowiz_core::typed_metrics::*;

use std::sync::OnceLock;
use std::time::Instant;

/// Monotonic nanosecond counter anchored at first call. `Instant::now()` is
/// monotonic but has no absolute epoch, so we measure the delta from a
/// lazily-captured process-start `Instant`. Uses `Instant::now()` — panics on
/// `wasm32`, so callers on the FDR write path are all gated off wasm (no sink
/// is ever installed there).
static MONO_EPOCH: OnceLock<Instant> = OnceLock::new();

/// Monotonic ns since process start. `pub` so the `fdr` event schema can stamp
/// its replay-ordering key through the SAME clock the P08 metrics use (one mono
/// epoch per process).
pub fn mono_now_ns() -> u128 {
    let start = MONO_EPOCH.get_or_init(Instant::now);
    Instant::now().duration_since(*start).as_nanos()
}

/// Read the current process's CPU accounting from `/proc/self/stat`.
/// Returns `None` when `/proc/self/stat` is unreadable (e.g. non-Linux
/// test envs) — typed absence, never a fake.
///
/// `comm` (field 2) may contain `)`, so we split on `)` and take the part
/// after the LAST `)`, then index the remaining whitespace-separated
/// fields. `pid` is taken from the part before the first `(`.
pub fn proc_cpu_sample_from_proc_self() -> Option<ProcCpuSample> {
    let stat = crate::vfs::read_to_string("/proc/self/stat").ok()?;
    let pid: u32 = stat.split('(').next()?.trim().parse().ok()?;
    let after = stat.rsplit(')').next()?;
    // After `comm`, fields are 3..N (1-based from line start). Field 14 =
    // utime, field 15 = stime → indices 11 / 12 in this iterator.
    let fields: Vec<&str> = after.split_whitespace().collect();
    let utime_ticks: u64 = fields.get(11)?.parse().ok()?;
    let stime_ticks: u64 = fields.get(12)?.parse().ok()?;
    let clk_tck: u64 = 100; // Linux USER_HZ default; see struct docs.
    let mono_ns = mono_now_ns();
    Some(ProcCpuSample {
        pid,
        utime_ticks,
        stime_ticks,
        clk_tck,
        mono_ns,
    })
}

/// Read `VmRSS` / `VmHWM` from `/proc/self/status`. Returns `None` when the
/// file is unreadable (non-Linux) — typed absence.
pub fn mem_sample_from_proc_self() -> Option<MemSample> {
    let status = crate::vfs::read_to_string("/proc/self/status").ok()?;
    let mut vm_rss_kb: Option<u64> = None;
    let mut vm_hwm_kb: Option<u64> = None;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            vm_rss_kb = rest
                .trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok());
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            vm_hwm_kb = rest
                .trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok());
        }
    }
    Some(MemSample {
        vm_rss_kb: vm_rss_kb?,
        vm_hwm_kb: vm_hwm_kb?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On Linux, `proc_cpu_sample_from_proc_self()` returns `Some` with
    /// utime+stime > 0 (the test process has used some CPU). On non-Linux, skip.
    #[test]
    fn proc_cpu_sample_from_proc_self_nonempty() {
        #[cfg(not(target_os = "linux"))]
        {
            return;
        }
        #[cfg(target_os = "linux")]
        {
            // Burn real CPU (ticks are 1/clk_tck ≈ 10 ms granularity, so the
            // loop must actually execute — black_box inside prevents the
            // optimizer from eliminating it) so the sampler observes > 0 ticks.
            let mut acc: u64 = 0;
            for i in 0..50_000_000u64 {
                acc = acc.wrapping_add(i);
                core::hint::black_box(acc);
            }

            let s = proc_cpu_sample_from_proc_self()
                .expect("proc_cpu_sample_from_proc_self must read /proc/self/stat on Linux");
            assert!(
                s.utime_ticks + s.stime_ticks > 0,
                "test process should have consumed some CPU"
            );
            // VmRSS/VmHWM should also be available.
            let m = mem_sample_from_proc_self().expect("status readable on Linux");
            assert!(m.vm_rss_kb > 0, "process should have resident memory");
        }
    }
}
