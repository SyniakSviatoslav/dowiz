//! P08 typed metrics — **pure core** (no egress / no signing change).
//!
//! Deterministic, serde-free text marshalling for typed metric records.
//! The default kernel build is `std`-only (no `wasm` → no serde), so this
//! module uses a hand-written `to_line()` / `parse_line()` pair with a FIXED
//! field order rather than JSON. CPU-% is a *derived* consumer concern: we
//! emit raw ticks only. GPU is typed-absent (`Option<GpuSample> = None`) until
//! hardware exists — we never fake a `0`.
//!
//! Blueprint P08 §2/§3 (pure core only). This module does NOT touch
//! `tools/telemetry` or any egress/signing path.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

/// Monotonic nanosecond counter anchored at first call (pure-std; avoids
/// `libc` so the native rlib build stays dependency-free). `Instant::now()`
/// is monotonic but has no absolute epoch, so we measure the delta from a
/// lazily-captured process-start `Instant`.
static MONO_EPOCH: OnceLock<Instant> = OnceLock::new();

fn mono_now_ns() -> u128 {
    let start = MONO_EPOCH.get_or_init(Instant::now);
    Instant::now().duration_since(*start).as_nanos()
}

/// A single process CPU sample, read from `/proc/self/stat`.
///
/// `utime`/`stime` are clock ticks (fields 14/15, 1-based). `clk_tck` is the
/// kernel tick rate (Linux `USER_HZ`, conventionally 100 — `libc::sysconf` is
/// intentionally avoided to keep the native build serde/libc-free). `mono_ns`
/// is a monotonic timestamp for downstream delta computation. CPU-% is
/// intentionally NOT computed here — it is a derived consumer concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcCpuSample {
    pub pid: u32,
    pub utime_ticks: u64,
    pub stime_ticks: u64,
    pub clk_tck: u64,
    pub mono_ns: u128,
}

impl ProcCpuSample {
    /// Read the current process's CPU accounting from `/proc/self/stat`.
    /// Returns `None` when `/proc/self/stat` is unreadable (e.g. non-Linux
    /// test envs) — typed absence, never a fake.
    ///
    /// `comm` (field 2) may contain `)`, so we split on `)` and take the part
    /// after the LAST `)`, then index the remaining whitespace-separated
    /// fields. `pid` is taken from the part before the first `(`.
    pub fn from_proc_self() -> Option<Self> {
        let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
        let pid: u32 = stat.split('(').next()?.trim().parse().ok()?;
        let after = stat.rsplit(')').next()?;
        // After `comm`, fields are 3..N (1-based from line start). Field 14 =
        // utime, field 15 = stime → indices 11 / 12 in this iterator.
        let fields: Vec<&str> = after.split_whitespace().collect();
        let utime_ticks: u64 = fields.get(11)?.parse().ok()?;
        let stime_ticks: u64 = fields.get(12)?.parse().ok()?;
        let clk_tck: u64 = 100; // Linux USER_HZ default; see struct docs.
        let mono_ns = mono_now_ns();
        Some(Self {
            pid,
            utime_ticks,
            stime_ticks,
            clk_tck,
            mono_ns,
        })
    }
}

/// A single process memory sample, read from `/proc/self/status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemSample {
    pub vm_rss_kb: u64,
    pub vm_hwm_kb: u64,
}

impl MemSample {
    /// Read `VmRSS` / `VmHWM` from `/proc/self/status`. Returns `None` when the
    /// file is unreadable (non-Linux) — typed absence.
    pub fn from_proc_self() -> Option<Self> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let mut vm_rss_kb: Option<u64> = None;
        let mut vm_hwm_kb: Option<u64> = None;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                vm_rss_kb = rest.trim().split_whitespace().next().and_then(|s| s.parse().ok());
            } else if let Some(rest) = line.strip_prefix("VmHWM:") {
                vm_hwm_kb = rest.trim().split_whitespace().next().and_then(|s| s.parse().ok());
            }
        }
        Some(Self {
            vm_rss_kb: vm_rss_kb?,
            vm_hwm_kb: vm_hwm_kb?,
        })
    }
}

/// A GPU utilization sample. Only constructed when hardware exists — there is
/// NO `from_proc_self()` here (hardware detection is out of scope for the pure
/// core). Today the metric sample carries `gpu: Option<GpuSample> = None`; we
/// never fake a `0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSample {
    pub util_pct: f32,
    pub mem_used_mb: u64,
}

/// A typed, serializable metric record. `to_line()` / `parse_line()` form a
/// deterministic, fixed-field-order pair (parse-or-reject: malformed input or
/// type mismatches return `Err`, never a best-effort default).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricLine {
    Cpu(ProcCpuSample),
    Mem(MemSample),
    Gpu(GpuSample),
}

impl MetricLine {
    /// Deterministic, fixed field-order serialization.
    pub fn to_line(&self) -> String {
        match self {
            MetricLine::Cpu(c) => {
                format!(
                    "Cpu pid={} utime_ticks={} stime_ticks={} clk_tck={} mono_ns={}",
                    c.pid, c.utime_ticks, c.stime_ticks, c.clk_tck, c.mono_ns
                )
            }
            MetricLine::Mem(m) => {
                format!("Mem vm_rss_kb={} vm_hwm_kb={}", m.vm_rss_kb, m.vm_hwm_kb)
            }
            MetricLine::Gpu(g) => {
                format!("Gpu util_pct={} mem_used_mb={}", g.util_pct, g.mem_used_mb)
            }
        }
    }

    /// Parse-or-reject: returns `Err` on unknown type tag, missing keys,
    /// unknown keys, or type mismatches (e.g. non-numeric where a number is
    /// required — this is the F32/integer parse-or-reject property).
    pub fn parse_line(s: &str) -> Result<MetricLine, &'static str> {
        let s = s.trim();
        let mut it = s.split_whitespace();
        let tag = it.next().ok_or("empty line")?;
        let mut kv: HashMap<&str, &str> = HashMap::new();
        for tok in it {
            let (k, v) = tok.split_once('=').ok_or("malformed key=val token")?;
            kv.insert(k, v);
        }
        match tag {
            "Cpu" => {
                check_keys(&kv, &["pid", "utime_ticks", "stime_ticks", "clk_tck", "mono_ns"])?;
                let pid: u32 = kv["pid"].parse().map_err(|_| "bad pid")?;
                let utime_ticks: u64 = kv["utime_ticks"].parse().map_err(|_| "bad utime_ticks")?;
                let stime_ticks: u64 = kv["stime_ticks"].parse().map_err(|_| "bad stime_ticks")?;
                let clk_tck: u64 = kv["clk_tck"].parse().map_err(|_| "bad clk_tck")?;
                let mono_ns: u128 = kv["mono_ns"].parse().map_err(|_| "bad mono_ns")?;
                Ok(MetricLine::Cpu(ProcCpuSample {
                    pid,
                    utime_ticks,
                    stime_ticks,
                    clk_tck,
                    mono_ns,
                }))
            }
            "Mem" => {
                check_keys(&kv, &["vm_rss_kb", "vm_hwm_kb"])?;
                let vm_rss_kb: u64 = kv["vm_rss_kb"].parse().map_err(|_| "bad vm_rss_kb")?;
                let vm_hwm_kb: u64 = kv["vm_hwm_kb"].parse().map_err(|_| "bad vm_hwm_kb")?;
                Ok(MetricLine::Mem(MemSample {
                    vm_rss_kb,
                    vm_hwm_kb,
                }))
            }
            "Gpu" => {
                check_keys(&kv, &["util_pct", "mem_used_mb"])?;
                let util_pct: f32 = kv["util_pct"].parse().map_err(|_| "bad util_pct")?;
                let mem_used_mb: u64 = kv["mem_used_mb"].parse().map_err(|_| "bad mem_used_mb")?;
                Ok(MetricLine::Gpu(GpuSample {
                    util_pct,
                    mem_used_mb,
                }))
            }
            _ => Err("unknown metric type"),
        }
    }
}

/// Reject unless the key set equals `required` exactly: every required key
/// must be present (missing key ⇒ `Err`) and no other key is allowed (unknown
/// key ⇒ `Err`). This is the parse-or-reject property for the schema.
fn check_keys(kv: &HashMap<&str, &str>, required: &[&str]) -> Result<(), &'static str> {
    for r in required {
        if !kv.contains_key(r) {
            return Err("missing required key");
        }
    }
    for k in kv.keys() {
        if !required.contains(k) {
            return Err("unknown key");
        }
    }
    Ok(())
}

/// A point-in-time aggregate of all typed samples. GPU is typed-absent by
/// default (`None`); it is only ever populated by an EXPLICIT `Some(GpuSample)`
/// when hardware exists. This is the typed-absence contract — never a fake 0.
#[derive(Debug, Clone, Default)]
pub struct MetricSample {
    pub cpu: Option<ProcCpuSample>,
    pub mem: Option<MemSample>,
    pub gpu: Option<GpuSample>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On Linux, `from_proc_self()` returns `Some` with utime+stime > 0 (the
    /// test process has used some CPU). On non-Linux, skip.
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
                std::hint::black_box(acc);
            }

            let s = ProcCpuSample::from_proc_self()
                .expect("from_proc_self must read /proc/self/stat on Linux");
            assert!(
                s.utime_ticks + s.stime_ticks > 0,
                "test process should have consumed some CPU"
            );
            // VmRSS/VmHWM should also be available.
            let m = MemSample::from_proc_self().expect("status readable on Linux");
            assert!(m.vm_rss_kb > 0, "process should have resident memory");
        }
    }

    /// Build a Cpu sample, `to_line()`, `parse_line()` back equals original;
    /// malformed lines return `Err` (parse-or-reject / F32 fix).
    #[test]
    fn metric_line_roundtrip_and_reject() {
        let cpu = ProcCpuSample {
            pid: 1234,
            utime_ticks: 100,
            stime_ticks: 50,
            clk_tck: 100,
            mono_ns: 999,
        };
        let line = MetricLine::Cpu(cpu).to_line();
        let parsed = MetricLine::parse_line(&line).expect("roundtrip must succeed");
        match parsed {
            MetricLine::Cpu(c) => assert_eq!(c, cpu),
            _ => panic!("wrong variant on roundtrip"),
        }

        // Mem + Gpu also roundtrip (Gpu exercises the F32 field).
        let gpu = GpuSample {
            util_pct: 42.5,
            mem_used_mb: 1024,
        };
        let gline = MetricLine::Gpu(gpu).to_line();
        assert_eq!(MetricLine::parse_line(&gline).unwrap(), MetricLine::Gpu(gpu));

        // Parse-or-reject: unknown tag.
        assert!(MetricLine::parse_line("garbage:::").is_err());
        // Parse-or-reject: malformed token that is not a clean "Cpu ..." record.
        assert!(MetricLine::parse_line("Cpu{x=notanumber}").is_err());
        // Parse-or-reject: type mismatch (non-numeric where integer required).
        assert!(
            MetricLine::parse_line("Cpu pid=abc utime_ticks=1 stime_ticks=1 clk_tck=1 mono_ns=1")
                .is_err()
        );
        // Parse-or-reject: unknown key.
        assert!(
            MetricLine::parse_line(
                "Cpu pid=1 utime_ticks=1 stime_ticks=1 clk_tck=1 mono_ns=1 foo=bar"
            )
            .is_err()
        );
        // Parse-or-reject: missing required field (was a latent Index panic →
        // now correctly Err).
        assert!(MetricLine::parse_line("Cpu pid=1 utime_ticks=1").is_err());
        // Parse-or-reject: non-numeric F32 (note: "nan"/"inf" DO parse as f32
        // in Rust, so we use a genuinely non-numeric token to exercise the
        // F32 parse-or-reject property).
        assert!(MetricLine::parse_line("Gpu util_pct=1.2.3 mem_used_mb=1").is_err());
    }

    /// Typed-absence contract: a Gpu-carrying line is only constructible via an
    /// explicit `GpuSample`, and the default metric sample keeps `gpu = None`
    /// (never a fake 0).
    #[test]
    fn gpu_typed_absence() {
        let sample = MetricSample::default();
        assert!(sample.gpu.is_none(), "default sample must NOT fake a GPU 0");
        assert!(sample.cpu.is_none());
        assert!(sample.mem.is_none());

        // Constructing a Gpu metric line REQUIRES an explicit GpuSample value.
        let gpu = GpuSample {
            util_pct: 42.5,
            mem_used_mb: 2048,
        };
        let line = MetricLine::Gpu(gpu); // explicit — no implicit zero
        assert!(gpu.util_pct > 0.0);
        assert_eq!(MetricLine::parse_line(&line.to_line()).unwrap(), line);
    }
}
