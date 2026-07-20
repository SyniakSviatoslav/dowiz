//! `fdr/pmu.rs` — PMU (performance-monitoring-unit) companion stamps for the kernel's
//! two classifiers, `markov::Verdict` and `spectral::DriftClass` (roadmap item 27,
//! classifier-INPUT half; blueprint `BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md`).
//!
//! # What this is
//!
//! A [`PmuStamp`] is a sibling of [`super::schema::HwStamp`], built from the SAME
//! [`Reading<T>`]/[`Absence`] machinery and the SAME raw-monotone-counters rule: the kernel
//! emits raw counters only; deltas/IPC/miss-rates are a consumer concern (see
//! [`PmuStamp::delta`] which is the one sanctioned exception — a bracketed before/after
//! subtraction, still emitted as raw `u64` counts).
//!
//! It carries two tiers of signal:
//!   * **Tier A** — zero-permission, always-available: the `rdtsc` reference-cycle counter
//!     (one stable intrinsic) plus page-fault and context-switch counters from `/proc`.
//!     These read real data on any Linux x86_64 host with no capabilities.
//!   * **Tier B** — true hardware PMU events (instructions, cpu-cycles, cache-misses,
//!     branch-misses → IPC / miss-rates). The only zero-dependency access path is a
//!     hand-rolled `perf_event_open(2)` raw syscall (no `libc`, no `perf-event` crate).
//!     On a host with `kernel.perf_event_paranoid >= 3` (this deploy target reads **4**)
//!     the syscall returns `EACCES` and every Tier-B field degrades to
//!     `Unavailable(PermissionDenied)` — a greppable named absence, never a fabricated 0.
//!
//! # Plane doctrine (enforced, verbatim from blueprint §4.4)
//!
//! PMU values are nondeterministic by nature and live on the P3 forensic/display plane,
//! exactly like FDR wall-clock timestamps: they are categorically excluded from every hash,
//! signature, idempotency, or gate-verdict surface. Nothing here changes a `Verdict`, a
//! `DriftClass`, a threshold, or any behavior. `analyze_detailed`/`classify_drift` stay
//! byte-identical pure functions; the PMU stamp rides ALONGSIDE each verdict emission on the
//! same [`super::schema::FdrEvent`] record (correlation is exact and free — same `seq`),
//! recorded input, never a decision variable (that is the Tier-4 response half, gated on
//! items 9 + 21). Diagnostic-grade; NO CI job is keyed to any PMU value.

use super::schema::{Absence, Reading};

/// A per-classification-window PMU stamp. All fields are `Reading<u64>` and are ALWAYS
/// serialized (value or named absence). Raw monotone counters only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PmuStamp {
    // ── Tier A — sampled fresh per stamp (one rdtsc + two /proc reads; µs-scale) ──
    /// `rdtsc` timestamp counter (reference cycles, NOT retired core cycles; a
    /// virtualized-TSC value under KVM). x86_64 only; a named absence elsewhere.
    pub tsc_cycles: Reading<u64>,
    /// Minor page faults (`/proc/self/stat` field 10) — no-I/O faults; the
    /// zero-permission proxy for allocation / working-set churn.
    pub minflt: Reading<u64>,
    /// Major page faults (`/proc/self/stat` field 12) — faults that hit backing store.
    pub majflt: Reading<u64>,
    /// Swap count (`/proc/self/stat` field 36). The kernel has not maintained this
    /// per-process since 2.6 and reports a genuine `0`; carried for parity with the
    /// classic PMU-proxy set (a truthful 0, not a fabricated absence).
    pub nswap: Reading<u64>,
    /// Voluntary context switches (`/proc/self/status`) — blocked-on-I/O / yield.
    pub vol_ctxt_switches: Reading<u64>,
    /// Nonvoluntary context switches (`/proc/self/status`) — preempted; the
    /// zero-permission proxy for CPU contention.
    pub nonvol_ctxt_switches: Reading<u64>,

    // ── Tier B — perf_event_open(2); on this host today: PermissionDenied ──
    /// Retired instructions (`PERF_COUNT_HW_INSTRUCTIONS`).
    pub hw_instructions: Reading<u64>,
    /// CPU cycles (`PERF_COUNT_HW_CPU_CYCLES`) — with instructions gives IPC.
    pub hw_cpu_cycles: Reading<u64>,
    /// Last-level cache misses (`PERF_COUNT_HW_CACHE_MISSES`).
    pub hw_cache_misses: Reading<u64>,
    /// Mispredicted branches (`PERF_COUNT_HW_BRANCH_MISSES`).
    pub hw_branch_misses: Reading<u64>,
}

impl PmuStamp {
    /// Serialize as a nested `"pmu":{...}` object onto `w`. Every field is present —
    /// value or `{"unavailable":"<reason>"}` — the same guarantee as `HwStamp::write`.
    pub fn write(self, w: super::json::JsonWriter) -> super::json::JsonWriter {
        let inner = super::json::JsonWriter::obj();
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
    /// `Value` only when BOTH endpoints are `Value`; otherwise it carries `end`'s absence
    /// (the reason a counter could not be read at the end of the window is the reason its
    /// delta is unavailable). Raw counts stay raw — a bracketed subtraction is the ONE
    /// sanctioned delta (blueprint §4.2), still emitted as a `u64` count, not a rate.
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

// ── Tier A readers ──────────────────────────────────────────────────────────────────

/// `rdtsc` reference-cycle counter. x86_64: a stable intrinsic that never faults in ring 3
/// (CR4.TSD is unset in practice). Non-x86_64 builds have no equivalent zero-dep intrinsic
/// here, so they report a named absence (`NoPmuInterface`) rather than a fake 0.
#[inline]
pub fn read_tsc() -> Reading<u64> {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: `_rdtsc` reads the timestamp counter; no memory access, no fault path in
        // user mode on this host (rdtsc/rdtscp advertised in cpuflags).
        let v = unsafe { core::arch::x86_64::_rdtsc() };
        Reading::Value(v)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        Reading::Unavailable(Absence::NoPmuInterface)
    }
}

/// `(minflt, majflt, nswap)` from `/proc/self/stat`. Uses the SAME robust `comm`-aware
/// parse as `typed_metrics::ProcCpuSample` (`comm` field 2 may contain `)`, so split after
/// the LAST `)`; the remaining whitespace fields are 3..N ⇒ index = field_number − 3:
/// minflt=field10→7, majflt=field12→9, nswap=field36→33). Any read/parse failure is a
/// named absence, never a fabricated 0.
fn read_proc_stat_faults() -> (Reading<u64>, Reading<u64>, Reading<u64>) {
    let stat = match std::fs::read_to_string("/proc/self/stat") {
        Ok(s) => s,
        Err(e) => {
            let a = io_absence(&e);
            return (
                Reading::Unavailable(a),
                Reading::Unavailable(a),
                Reading::Unavailable(a),
            );
        }
    };
    let after = match stat.rsplit(')').next() {
        Some(a) => a,
        None => {
            return (
                Reading::Unavailable(Absence::ReadError),
                Reading::Unavailable(Absence::ReadError),
                Reading::Unavailable(Absence::ReadError),
            )
        }
    };
    let fields: Vec<&str> = after.split_whitespace().collect();
    let get = |idx: usize| -> Reading<u64> {
        match fields.get(idx).and_then(|s| s.parse::<u64>().ok()) {
            Some(v) => Reading::Value(v),
            None => Reading::Unavailable(Absence::ReadError),
        }
    };
    (get(7), get(9), get(33))
}

/// `(voluntary, nonvoluntary)` context-switch counts from `/proc/self/status`.
fn read_proc_ctxt_switches() -> (Reading<u64>, Reading<u64>) {
    let status = match std::fs::read_to_string("/proc/self/status") {
        Ok(s) => s,
        Err(e) => {
            let a = io_absence(&e);
            return (Reading::Unavailable(a), Reading::Unavailable(a));
        }
    };
    let mut vol: Reading<u64> = Reading::Unavailable(Absence::ReadError);
    let mut nonvol: Reading<u64> = Reading::Unavailable(Absence::ReadError);
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("voluntary_ctxt_switches:") {
            if let Some(v) = rest.trim().split_whitespace().next().and_then(|s| s.parse().ok()) {
                vol = Reading::Value(v);
            }
        } else if let Some(rest) = line.strip_prefix("nonvoluntary_ctxt_switches:") {
            if let Some(v) = rest.trim().split_whitespace().next().and_then(|s| s.parse().ok()) {
                nonvol = Reading::Value(v);
            }
        }
    }
    (vol, nonvol)
}

/// Map a filesystem `io::Error` to the closest `Absence`.
fn io_absence(e: &std::io::Error) -> Absence {
    match e.kind() {
        std::io::ErrorKind::NotFound => Absence::NonLinuxHost, // no /proc ⇒ not Linux
        std::io::ErrorKind::PermissionDenied => Absence::PermissionDenied,
        _ => Absence::ReadError,
    }
}

// ── Tier B: hand-rolled perf_event_open(2) plumbing ─────────────────────────────────
//
// Zero-dependency: no `libc`, no `perf-event` crate. A `#[repr(C)]` `perf_event_attr`
// declared to the PERF_ATTR_SIZE_VER0 (64-byte) ABI, submitted through a raw `syscall`
// instruction via `core::arch::asm!`. On failure the errno maps to a named `Absence`.
// EACCES/EPERM (paranoid ≥ 1..4) ⇒ PermissionDenied; ENOENT/ENODEV/EOPNOTSUPP/ENOSYS ⇒
// NoPmuInterface; anything else ⇒ ReadError. This host (`perf_event_paranoid = 4`) always
// takes the PermissionDenied branch — that is correct, tested behavior, not a bug.

/// The four hardware events we open, in `PmuStamp` Tier-B field order.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const HW_EVENTS: [u64; 4] = [
    1, // PERF_COUNT_HW_INSTRUCTIONS
    0, // PERF_COUNT_HW_CPU_CYCLES
    3, // PERF_COUNT_HW_CACHE_MISSES
    5, // PERF_COUNT_HW_BRANCH_MISSES
];

/// `perf_event_attr` to the VER0 (64-byte) ABI. Exactly 64 bytes; `size` is set to 64 so
/// the running kernel treats trailing (zero) space as compatible.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[repr(C)]
#[derive(Default)]
struct PerfEventAttr {
    type_: u32,         // offset 0  — PERF_TYPE_HARDWARE = 0
    size: u32,          // offset 4  — PERF_ATTR_SIZE_VER0 = 64
    config: u64,        // offset 8  — the HW_EVENTS selector
    sample_period: u64, // offset 16
    sample_type: u64,   // offset 24
    read_format: u64,   // offset 32
    flags: u64,         // offset 40 — bitfield: bit0 disabled, bit5 exclude_kernel, bit6 exclude_hv
    wakeup_events: u32, // offset 48
    bp_type: u32,       // offset 52
    config1: u64,       // offset 56  (total 64)
}

/// Raw 5-argument `syscall` (x86_64 Linux): number in `rax`, args in
/// `rdi/rsi/rdx/r10/r8`, result in `rax`; the kernel clobbers `rcx` and `r11`.
/// Returns the raw kernel result (negative = `-errno`).
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[inline]
unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8") a5,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack),
    );
    ret
}

/// Map a kernel `-errno` result to an `Absence` (blueprint §4.2 errno table).
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn errno_absence(ret: i64) -> Absence {
    match -ret {
        1 | 13 => Absence::PermissionDenied,      // EPERM / EACCES — perf_event_paranoid
        2 | 19 | 38 | 95 => Absence::NoPmuInterface, // ENOENT/ENODEV/ENOSYS/EOPNOTSUPP
        _ => Absence::ReadError,
    }
}

/// A live perf fd; closes on drop.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
struct PerfFd(i32);

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for PerfFd {
    fn drop(&mut self) {
        // SAFETY: close(2) on our own fd.
        unsafe {
            let _ = syscall5(3, self.0 as i64, 0, 0, 0, 0);
        }
    }
}

/// Open one hardware-event counter for THIS process (`pid=0`), any CPU (`cpu=-1`), no
/// group (`group_fd=-1`), running (not disabled). `Ok(fd)` or the errno-derived `Absence`.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn perf_open_one(config: u64) -> Result<PerfFd, Absence> {
    const SYS_PERF_EVENT_OPEN: i64 = 298;
    const PERF_FLAG_FD_CLOEXEC: i64 = 8;
    // exclude_kernel (bit5) | exclude_hv (bit6). `disabled` left 0 ⇒ counter runs at open.
    const FLAGS: u64 = (1 << 5) | (1 << 6);
    let attr = PerfEventAttr {
        type_: 0, // PERF_TYPE_HARDWARE
        size: 64, // PERF_ATTR_SIZE_VER0
        config,
        flags: FLAGS,
        ..Default::default()
    };
    // SAFETY: `&attr` points at a live 64-byte repr(C) struct for the duration of the call;
    // all other args are plain integers. The syscall reads only within `size` bytes.
    let ret = unsafe {
        syscall5(
            SYS_PERF_EVENT_OPEN,
            &attr as *const PerfEventAttr as i64,
            0,  // pid = self
            -1, // cpu = any
            -1, // group_fd = none
            PERF_FLAG_FD_CLOEXEC,
        )
    };
    if ret < 0 {
        Err(errno_absence(ret))
    } else {
        Ok(PerfFd(ret as i32))
    }
}

/// `read(2)` a running counter fd into a `u64`. Any short/failed read is a named absence.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn perf_read(fd: &PerfFd) -> Reading<u64> {
    const SYS_READ: i64 = 0;
    let mut val: u64 = 0;
    // SAFETY: read(2) into an 8-byte owned stack slot; `count = 8`.
    let n = unsafe {
        syscall5(
            SYS_READ,
            fd.0 as i64,
            &mut val as *mut u64 as i64,
            8,
            0,
            0,
        )
    };
    if n == 8 {
        Reading::Value(val)
    } else if n < 0 {
        Reading::Unavailable(errno_absence(n))
    } else {
        Reading::Unavailable(Absence::ReadError)
    }
}

/// The Tier-B counter station: opens the four fds ONCE (on construction) and caches the
/// errno-derived absence per counter so a blocked host is not re-probed every sample
/// (cost + log-noise control, blueprint §4.2). On non-Linux / non-x86_64 builds every
/// counter is a compile-time `NoPmuInterface`.
pub struct PmuStation {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fds: [Result<PerfFd, Absence>; 4],
}

impl PmuStation {
    /// Open the Tier-B counters once. Tier A needs no setup (sampled fresh each call).
    pub fn new() -> Self {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            PmuStation {
                fds: HW_EVENTS.map(perf_open_one),
            }
        }
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            PmuStation {}
        }
    }

    /// Read one Tier-B counter (or its cached absence). Non-Linux/x86_64 ⇒ `NoPmuInterface`.
    #[allow(unused_variables)]
    fn tier_b(&self, i: usize) -> Reading<u64> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            match &self.fds[i] {
                Ok(fd) => perf_read(fd),
                Err(a) => Reading::Unavailable(*a),
            }
        }
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            Reading::Unavailable(Absence::NoPmuInterface)
        }
    }

    /// Take a full stamp: Tier A fresh (rdtsc + two `/proc` reads), Tier B from the
    /// once-opened counters.
    pub fn sample(&self) -> PmuStamp {
        let (minflt, majflt, nswap) = read_proc_stat_faults();
        let (vol, nonvol) = read_proc_ctxt_switches();
        PmuStamp {
            tsc_cycles: read_tsc(),
            minflt,
            majflt,
            nswap,
            vol_ctxt_switches: vol,
            nonvol_ctxt_switches: nonvol,
            hw_instructions: self.tier_b(0),
            hw_cpu_cycles: self.tier_b(1),
            hw_cache_misses: self.tier_b(2),
            hw_branch_misses: self.tier_b(3),
        }
    }

    /// Window-bracket a classification-relevant call: sample before, run `f`, sample after,
    /// return `f`'s result together with the absence-propagating PMU delta. The classifier
    /// (`analyze_detailed`/`classify_drift`) is invoked EXACTLY as before — `f` is a plain
    /// closure over it, its signature and behavior untouched.
    pub fn bracket<T>(&self, f: impl FnOnce() -> T) -> (T, PmuStamp) {
        let start = self.sample();
        let out = f();
        let end = self.sample();
        (out, PmuStamp::delta(start, end))
    }
}

impl Default for PmuStation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_a_reads_real_nonzero_counters() {
        // rdtsc must advance across two reads, and the /proc fault/ctxt counters must parse
        // to real Values on this Linux x86_64 host — NOT stub zeros, NOT absences.
        let s1 = read_tsc();
        // Do a little work so the second tsc read is strictly greater.
        let mut acc = 0u64;
        for i in 0..10_000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(2654435761));
        }
        core::hint::black_box(acc);
        let s2 = read_tsc();
        match (s1, s2) {
            (Reading::Value(a), Reading::Value(b)) => {
                assert!(b > a, "rdtsc must advance: {a} -> {b}");
            }
            _ => panic!("rdtsc must read a Value on x86_64"),
        }

        let (minflt, majflt, nswap) = read_proc_stat_faults();
        assert!(!minflt.is_unavailable(), "minflt must be a real Value: {minflt:?}");
        assert!(!majflt.is_unavailable(), "majflt must be a real Value: {majflt:?}");
        assert!(!nswap.is_unavailable(), "nswap must parse (a truthful 0): {nswap:?}");
        // A live process always has taken minor faults.
        assert!(
            matches!(minflt, Reading::Value(v) if v > 0),
            "a running process must have >0 minor faults: {minflt:?}"
        );

        let (vol, nonvol) = read_proc_ctxt_switches();
        assert!(!vol.is_unavailable(), "vol ctxt switches must parse: {vol:?}");
        assert!(!nonvol.is_unavailable(), "nonvol ctxt switches must parse: {nonvol:?}");
    }

    #[test]
    fn tier_b_never_panics_and_reports_a_value_or_a_named_absence() {
        // The load-bearing correctness property: perf_event_open never panics/SIGSEGVs and
        // every Tier-B field is EITHER a real Value (permissive host: paranoid<=2 OR the
        // process holds CAP_PERFMON/root — which bypasses paranoid entirely) OR a *named*
        // absence (PermissionDenied on a gated host, NoPmuInterface where no PMU exists) —
        // NEVER a fabricated 0 and never silently-wrong data. A Value, when present, must be
        // hardware-plausible (a live process has retired instructions and taken cycles),
        // which is how we detect a garbage-returning syscall as opposed to a real read.
        let station = PmuStation::new();
        // Do measurable work between open and read so a working counter is strictly > 0.
        let mut acc = 0u64;
        for i in 0..200_000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(2654435761) ^ (i >> 3));
        }
        core::hint::black_box(acc);
        let stamp = station.sample();
        for (name, r) in [
            ("hw_instructions", stamp.hw_instructions),
            ("hw_cpu_cycles", stamp.hw_cpu_cycles),
            ("hw_cache_misses", stamp.hw_cache_misses),
            ("hw_branch_misses", stamp.hw_branch_misses),
        ] {
            match r {
                Reading::Unavailable(a) => assert!(
                    matches!(a, Absence::PermissionDenied | Absence::NoPmuInterface),
                    "{name}: unexpected absence {a:?}"
                ),
                Reading::Value(v) => {
                    // instructions & cpu_cycles MUST be nonzero on a live process if the
                    // read succeeded (a zero would betray a bogus fd / garbage read).
                    if name == "hw_instructions" || name == "hw_cpu_cycles" {
                        assert!(v > 0, "{name}: a successful HW read must be > 0, got {v}");
                    }
                    eprintln!("TIER_B_LIVE {name} = {v}");
                }
            }
        }
    }

    #[test]
    fn stamp_always_serializes_every_field_named_absence_not_missing_key() {
        let station = PmuStation::new();
        let json = station.sample().write(super::super::json::JsonWriter::obj()).finish();
        // Every field key present (value or {"unavailable":...}).
        for key in [
            "tsc_cycles",
            "minflt",
            "majflt",
            "nswap",
            "vol_ctxt_switches",
            "nonvol_ctxt_switches",
            "hw_instructions",
            "hw_cpu_cycles",
            "hw_cache_misses",
            "hw_branch_misses",
        ] {
            assert!(json.contains(&format!("\"{key}\":")), "missing key {key} in {json}");
        }
    }

    /// Deterministic proof of the graceful-degradation MECHANISM, independent of this
    /// process's privilege level: every `perf_event_open` errno maps to the correct named
    /// `Absence`, and a forced Tier-B absence serializes as a greppable `{"unavailable":...}`
    /// (never a fabricated 0, never a missing key). This is the load-bearing property when
    /// the process is genuinely unprivileged (paranoid>=1 without CAP_PERFMON) — which this
    /// root+CAP_PERFMON agent process is NOT, so it is asserted here by construction.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn errno_maps_to_named_absence_and_serializes() {
        assert_eq!(errno_absence(-1), Absence::PermissionDenied, "EPERM");
        assert_eq!(errno_absence(-13), Absence::PermissionDenied, "EACCES (paranoid gate)");
        assert_eq!(errno_absence(-2), Absence::NoPmuInterface, "ENOENT");
        assert_eq!(errno_absence(-19), Absence::NoPmuInterface, "ENODEV");
        assert_eq!(errno_absence(-38), Absence::NoPmuInterface, "ENOSYS (seccomp)");
        assert_eq!(errno_absence(-95), Absence::NoPmuInterface, "EOPNOTSUPP");
        assert_eq!(errno_absence(-22), Absence::ReadError, "EINVAL → generic ReadError");

        // A stamp whose Tier-B fields were blocked must serialize named absences.
        let blocked = PmuStamp {
            tsc_cycles: Reading::Value(1),
            minflt: Reading::Value(1),
            majflt: Reading::Value(0),
            nswap: Reading::Value(0),
            vol_ctxt_switches: Reading::Value(0),
            nonvol_ctxt_switches: Reading::Value(0),
            hw_instructions: Reading::Unavailable(Absence::PermissionDenied),
            hw_cpu_cycles: Reading::Unavailable(Absence::PermissionDenied),
            hw_cache_misses: Reading::Unavailable(Absence::PermissionDenied),
            hw_branch_misses: Reading::Unavailable(Absence::PermissionDenied),
        };
        let json = blocked.write(super::super::json::JsonWriter::obj()).finish();
        assert!(
            json.contains("\"hw_instructions\":{\"unavailable\":\"permission_denied\"}"),
            "blocked Tier B must be a greppable named absence: {json}"
        );
    }

    #[test]
    fn delta_propagates_absence_and_subtracts_values() {
        let start = PmuStamp {
            tsc_cycles: Reading::Value(100),
            minflt: Reading::Value(5),
            majflt: Reading::Value(0),
            nswap: Reading::Value(0),
            vol_ctxt_switches: Reading::Value(2),
            nonvol_ctxt_switches: Reading::Value(1),
            hw_instructions: Reading::Unavailable(Absence::PermissionDenied),
            hw_cpu_cycles: Reading::Unavailable(Absence::PermissionDenied),
            hw_cache_misses: Reading::Unavailable(Absence::PermissionDenied),
            hw_branch_misses: Reading::Unavailable(Absence::PermissionDenied),
        };
        let end = PmuStamp {
            tsc_cycles: Reading::Value(180),
            minflt: Reading::Value(9),
            ..start
        };
        let d = PmuStamp::delta(start, end);
        assert_eq!(d.tsc_cycles, Reading::Value(80));
        assert_eq!(d.minflt, Reading::Value(4));
        // Absence at either endpoint propagates.
        assert_eq!(
            d.hw_instructions,
            Reading::Unavailable(Absence::PermissionDenied)
        );
    }

    #[test]
    fn bracket_runs_classifier_untouched_and_records_a_delta() {
        // Bracket a REAL markov classification. The verdict is exactly what the pure
        // function returns (bracketing must not change it), and the PMU delta records a
        // real, monotone rdtsc advance across the window.
        use crate::markov::{analyze_detailed, Verdict};
        let toks: Vec<&str> = {
            let mut v = Vec::new();
            for _ in 0..8 {
                v.push("edit");
                v.push("run_ok");
            }
            v
        };
        let station = PmuStation::new();
        let (report, delta) = station.bracket(|| analyze_detailed(&toks));
        // Purity: identical to the un-bracketed call.
        assert_eq!(report.report.verdict, analyze_detailed(&toks).report.verdict);
        assert_eq!(report.report.verdict, Verdict::Healthy);
        // The window's rdtsc delta is a real (nonzero, on any real classification) count.
        assert!(
            matches!(delta.tsc_cycles, Reading::Value(v) if v > 0),
            "bracket must record a real tsc delta: {:?}",
            delta.tsc_cycles
        );
        // The Tier-B delta is a Value on this privileged host (perf_event_open bypasses
        // paranoid under CAP_PERFMON) or a named absence on a gated host — either is correct.
        // If it is a Value, it must be hardware-plausible (>0 instructions retired doing the
        // classification), never a fabricated 0.
        match delta.hw_instructions {
            Reading::Value(v) => {
                assert!(v > 0, "bracketed instruction delta must be > 0 if readable: {v}");
                eprintln!("BRACKET_TIER_B_DELTA hw_instructions = {v}");
            }
            Reading::Unavailable(a) => assert!(matches!(
                a,
                Absence::PermissionDenied | Absence::NoPmuInterface
            )),
        }
    }

    #[test]
    fn drift_class_lane_bracket_preserves_verdict_and_records_pmu_delta() {
        // Blueprint §4.3 — DriftClass lane: bracket a REAL `classify_drift` call the same
        // way a verdict-emission point would. The `DriftClass` verdict must be EXACTLY what
        // the pure function returns (bracketing must not change it — the classifier stays
        // byte-identical pure), and the PMU delta records a real, monotone rdtsc advance
        // across the window. This exercises the DriftClass emission path that the existing
        // Verdict-lane tests do not cover, proving the companion-record design works for
        // both classifiers.
        use crate::spectral::{classify_drift, DriftClass};
        // A well-formed operator with spectral radius < 1 ⇒ Damped (fail-closed guards pass).
        let op: Vec<Vec<f64>> = vec![vec![0.5, 0.0], vec![0.0, 0.3]];
        let station = PmuStation::new();
        let (drift, delta) = station.bracket(|| classify_drift(&op));
        // Purity: identical to the un-bracketed call.
        assert_eq!(drift, classify_drift(&op));
        assert_eq!(drift, DriftClass::Damped);
        // The window's rdtsc delta is a real (nonzero, on any real classification) count.
        assert!(
            matches!(delta.tsc_cycles, Reading::Value(v) if v > 0),
            "drift-lane bracket must record a real tsc delta: {:?}",
            delta.tsc_cycles
        );
        // The Tier-B delta is a Value on this privileged host (perf_event_open bypasses
        // paranoid under CAP_PERFMON) or a named absence on a gated host — either is correct.
        // If it is a Value, it must be hardware-plausible (>0 instructions retired doing the
        // classification), never a fabricated 0.
        match delta.hw_instructions {
            Reading::Value(v) => {
                assert!(
                    v > 0,
                    "drift-lane bracketed instruction delta must be > 0 if readable: {v}"
                );
            }
            Reading::Unavailable(a) => {
                assert!(matches!(a, Absence::PermissionDenied | Absence::NoPmuInterface))
            }
        }
    }
}
