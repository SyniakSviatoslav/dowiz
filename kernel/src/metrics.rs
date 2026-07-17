//! P08 typed local-observability core — the pure-std, no-network, no-signing HALF.
//!
//! This module implements the typed-metrics schema + closed `LogEvent` enum from
//! BLUEPRINT-P08 §2/§3 and the claim-latency anomaly detector from §4. It is the
//! fail-closed LOCAL sink core: serialization is deterministic (fixed field order)
//! and ingest is parse-or-reject (a malformed or wrong-typed line is returned as
//! `Err`, never silently coerced).
//!
//! // innovate: F40 ML-DSA envelope DEFERRED pending bebop2 C4b (constant-time
//! // mod_l); typed local sink only. Do NOT build signing or any remote sink here.
//!
//! Design constraints honored:
//! - Pure `std` only. The default kernel build is `std`-only (no `wasm` feature →
//!   no serde), so this module uses a hand-rolled deterministic line format and
//!   pulls ZERO new crates.
//! - GPU is typed-absent: `gpu: Option<GpuSample> = None` TODAY — never a fake 0.
//! - Reuses the kernel's crash-safe `Spool` state machine ONLY as the intended
//!   local-sink carrier (the I/O adapter that drains to a local JSONL file lives
//!   outside the kernel behind the pure-std firewall). This module only needs the
//!   schema + detector; the spool is referenced for documentation/intent.

/// A host identifier. Newtype around `String` to keep the schema strongly typed
/// and to avoid passing bare strings through the typed event surface.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HostId(pub String);

impl HostId {
    pub fn new(s: impl Into<String>) -> Self {
        HostId(s.into())
    }
}

/// A single per-process CPU sample (from `/proc/self/stat`). `utime`/`stime` are
/// clock ticks; `clk_tck` is the tick rate (USER_HZ). CPU-% is a DERIVED consumer
/// concern and is intentionally NOT computed in the typed schema — raw counters
/// stay lossless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcCpuSample {
    pub pid: u32,
    pub utime_ticks: u64,
    pub stime_ticks: u64,
    pub clk_tck: u64,
    /// Monotonic timestamp (ns) paired with the sample so the Δwall denominator
    /// is immune to wall-clock jumps.
    pub mono_ns: u128,
}

/// A single per-process memory sample (from `/proc/self/status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemSample {
    pub vm_rss_kb: u64,
    pub vm_hwm_kb: u64,
}

/// A GPU utilization sample. Only ever constructed when hardware exists. There is
/// NO default-`Some` path: today the metric sample carries `gpu: None` (typed
/// absence), never a fabricated `0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSample {
    pub util_pct: f32,
    pub mem_used_mb: u64,
}

/// A point-in-time typed metric record. Carries `gpu: None` TODAY (typed
/// absence — never a fake 0). `gpu` becomes `Some` ONLY behind the compute-phase
/// GPU port.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricSample {
    pub ts_unix_ns: u128,
    pub host_id: HostId,
    pub cpu: ProcCpuSample,
    pub mem: MemSample,
    pub gpu: Option<GpuSample>,
}

/// The claim-latency ledger record (consumed from Phase 1's ledger): per commit,
/// the diff size and the claimed delta between commit and first-GREEN.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimLatencyRecord {
    pub commit: String,
    pub diff_lines: usize,
    pub delta_seconds: f64,
}

/// A raised claim-latency anomaly flag (advisory only — does NOT gate a merge).
#[derive(Debug, Clone, PartialEq)]
pub struct AnomalyFlag {
    pub commit: String,
    pub diff_lines: usize,
    pub delta_seconds: f64,
}

/// A benchmark timing record (typed so a bench line is never an untyped string).
#[derive(Debug, Clone, PartialEq)]
pub struct BenchRecord {
    pub bench: String,
    pub wall_ms: f64,
}

/// A CLOSED, typed log event. Ingest is parse-or-reject: a candidate line is
/// deserialized into one of these exact variants; anything else is `Err`.
#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    Metric(MetricSample),
    ClaimLatency(ClaimLatencyRecord),
    ClaimLatencyAnomaly(AnomalyFlag),
    Bench(BenchRecord),
}

// ── deterministic line format ───────────────────────────────────────────────
//
// Each variant serializes to a single line with a FIXED field order. Fields are
// separated by '|'; nested numeric fields are separated by ':' (a value may NOT
// contain '|', so any injected delimiter changes the part count and is rejected).
//
//   Metric(M):        M|<ts_unix_ns>|<host>|<pid>:<utime>:<stime>:<clk>:<mono>|<rss>:<hwm>|<gpu>
//                     <gpu> = "null"                  (typed absence)
//                            | "<util_pct>:<mem_mb>"  (explicit Some)
//   ClaimLatency(C):  C|<commit>|<diff_lines>|<delta_seconds>
//   ClaimLatencyAnomaly(A): A|<commit>|<diff_lines>|<delta_seconds>
//   Bench(B):         B|<bench>|<wall_ms>
//
// Determinism: identical records produce byte-identical lines (fixed order, no
// map iteration, no locale-formatted numbers).

impl LogEvent {
    /// Deterministic, fixed-field-order serialization to a single line.
    pub fn to_line(&self) -> String {
        match self {
            LogEvent::Metric(m) => {
                let gpu = match &m.gpu {
                    None => "null".to_string(),
                    Some(g) => format!("{}:{}", g.util_pct, g.mem_used_mb),
                };
                format!(
                    "M|{}|{}|{}:{}:{}:{}:{}|{}:{}|{}",
                    m.ts_unix_ns,
                    m.host_id.0,
                    m.cpu.pid,
                    m.cpu.utime_ticks,
                    m.cpu.stime_ticks,
                    m.cpu.clk_tck,
                    m.cpu.mono_ns,
                    m.mem.vm_rss_kb,
                    m.mem.vm_hwm_kb,
                    gpu
                )
            }
            LogEvent::ClaimLatency(r) => {
                format!("C|{}|{}|{}", r.commit, r.diff_lines, r.delta_seconds)
            }
            LogEvent::ClaimLatencyAnomaly(a) => {
                format!("A|{}|{}|{}", a.commit, a.diff_lines, a.delta_seconds)
            }
            LogEvent::Bench(b) => {
                format!("B|{}|{}", b.bench, b.wall_ms)
            }
        }
    }

    /// Parse-or-reject: deserialize a line into a known `LogEvent` variant.
    /// Returns `Err(reason)` if the line does not match a known variant or any
    /// field has the wrong type — it NEVER silently coerces or defaults.
    pub fn from_line(s: &str) -> Result<LogEvent, String> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('|').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err("empty line".to_string());
        }
        match parts[0] {
            "M" => {
                if parts.len() != 6 {
                    return Err(format!("Metric: expected 6 fields, got {}", parts.len()));
                }
                let ts_unix_ns: u128 = parts[1]
                    .parse()
                    .map_err(|_| format!("Metric: bad ts_unix_ns {:?}", parts[1]))?;
                let host_id = HostId(parts[2].to_string());
                let cpu = split_fixed(parts[3], ':', 5, "Metric.cpu")?;
                let pid: u32 = cpu[0]
                    .parse()
                    .map_err(|_| format!("Metric: bad pid {:?}", cpu[0]))?;
                let utime_ticks: u64 = cpu[1]
                    .parse()
                    .map_err(|_| format!("Metric: bad utime_ticks {:?}", cpu[1]))?;
                let stime_ticks: u64 = cpu[2]
                    .parse()
                    .map_err(|_| format!("Metric: bad stime_ticks {:?}", cpu[2]))?;
                let clk_tck: u64 = cpu[3]
                    .parse()
                    .map_err(|_| format!("Metric: bad clk_tck {:?}", cpu[3]))?;
                let mono_ns: u128 = cpu[4]
                    .parse()
                    .map_err(|_| format!("Metric: bad mono_ns {:?}", cpu[4]))?;
                let mem = split_fixed(parts[4], ':', 2, "Metric.mem")?;
                let vm_rss_kb: u64 = mem[0]
                    .parse()
                    .map_err(|_| format!("Metric: bad vm_rss_kb {:?}", mem[0]))?;
                let vm_hwm_kb: u64 = mem[1]
                    .parse()
                    .map_err(|_| format!("Metric: bad vm_hwm_kb {:?}", mem[1]))?;
                let gpu = if parts[5] == "null" {
                    None
                } else {
                    let g = split_fixed(parts[5], ':', 2, "Metric.gpu")?;
                    let util_pct: f32 = g[0]
                        .parse()
                        .map_err(|_| format!("Metric: bad gpu.util_pct {:?}", g[0]))?;
                    let mem_used_mb: u64 = g[1]
                        .parse()
                        .map_err(|_| format!("Metric: bad gpu.mem_used_mb {:?}", g[1]))?;
                    Some(GpuSample {
                        util_pct,
                        mem_used_mb,
                    })
                };
                Ok(LogEvent::Metric(MetricSample {
                    ts_unix_ns,
                    host_id,
                    cpu: ProcCpuSample {
                        pid,
                        utime_ticks,
                        stime_ticks,
                        clk_tck,
                        mono_ns,
                    },
                    mem: MemSample {
                        vm_rss_kb,
                        vm_hwm_kb,
                    },
                    gpu,
                }))
            }
            "C" => {
                if parts.len() != 4 {
                    return Err(format!(
                        "ClaimLatency: expected 4 fields, got {}",
                        parts.len()
                    ));
                }
                let diff_lines: usize = parts[2]
                    .parse()
                    .map_err(|_| format!("ClaimLatency: bad diff_lines {:?}", parts[2]))?;
                let delta_seconds: f64 = parts[3]
                    .parse()
                    .map_err(|_| format!("ClaimLatency: bad delta_seconds {:?}", parts[3]))?;
                Ok(LogEvent::ClaimLatency(ClaimLatencyRecord {
                    commit: parts[1].to_string(),
                    diff_lines,
                    delta_seconds,
                }))
            }
            "A" => {
                if parts.len() != 4 {
                    return Err(format!(
                        "ClaimLatencyAnomaly: expected 4 fields, got {}",
                        parts.len()
                    ));
                }
                let diff_lines: usize = parts[2]
                    .parse()
                    .map_err(|_| format!("ClaimLatencyAnomaly: bad diff_lines {:?}", parts[2]))?;
                let delta_seconds: f64 = parts[3].parse().map_err(|_| {
                    format!("ClaimLatencyAnomaly: bad delta_seconds {:?}", parts[3])
                })?;
                Ok(LogEvent::ClaimLatencyAnomaly(AnomalyFlag {
                    commit: parts[1].to_string(),
                    diff_lines,
                    delta_seconds,
                }))
            }
            "B" => {
                if parts.len() != 3 {
                    return Err(format!("Bench: expected 3 fields, got {}", parts.len()));
                }
                let wall_ms: f64 = parts[2]
                    .parse()
                    .map_err(|_| format!("Bench: bad wall_ms {:?}", parts[2]))?;
                Ok(LogEvent::Bench(BenchRecord {
                    bench: parts[1].to_string(),
                    wall_ms,
                }))
            }
            other => Err(format!("unknown LogEvent variant tag {other:?}")),
        }
    }
}

/// Split `s` on `sep` and require EXACTLY `n` non-empty parts. This is the
/// parse-or-reject guard for nested fields: a wrong arity (too few or too many)
/// is rejected rather than best-effort parsed.
fn split_fixed<'a>(s: &'a str, sep: char, n: usize, ctx: &str) -> Result<Vec<&'a str>, String> {
    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() != n {
        return Err(format!("{ctx}: expected {n} fields, got {}", parts.len()));
    }
    for p in &parts {
        if p.is_empty() {
            return Err(format!("{ctx}: empty field"));
        }
    }
    Ok(parts)
}

// ── claim-latency anomaly detector (§4) ───────────────────────────────────────
//
// F36/E47 (built once). Encodes a falsifiable minimum plausible verification time
// as a function of diff size. `delta_seconds` below
// `(diff_lines/100) * min_sec_per_100_lines` ⇒ claimed implausibly fast ⇒ flag.
//
// Worked example at a 5 s/100-line floor: 1610 lines ⇒ ~80.5 s plausible minimum;
// a recorded 52 s < 80.5 s ⇒ FLAG (the real "52s GREEN on a 1610-line diff"
// BRAIN-TOPOLOGY self-certification residue).

/// Named, reviewed floor constant (VERIFIED-BY-MATH style: documented, tunable,
/// reviewable — never a magic number). Tuned against the ledger's own history;
/// §6.5 pins the 52 s/1610-line reproduction.
pub const MIN_SECONDS_PER_100_LINES: f64 = 5.0;

/// Pure anomaly predicate. Returns `true` when `delta_seconds` is implausibly
/// small for `diff_lines` under `min_sec_per_100_lines` (claimed too fast).
pub fn claim_latency_floor(
    diff_lines: usize,
    delta_seconds: f64,
    min_sec_per_100_lines: f64,
) -> bool {
    let floor = (diff_lines as f64 / 100.0) * min_sec_per_100_lines;
    delta_seconds < floor
}

/// Emit a typed `LogEvent::ClaimLatencyAnomaly` when the claim is implausibly
/// fast. ADVISORY only — does not gate a merge (Phase 6's verifier owns gating).
pub fn check_claim_latency(
    commit: &str,
    diff_lines: usize,
    delta_seconds: f64,
) -> Option<LogEvent> {
    if claim_latency_floor(diff_lines, delta_seconds, MIN_SECONDS_PER_100_LINES) {
        Some(LogEvent::ClaimLatencyAnomaly(AnomalyFlag {
            commit: commit.to_string(),
            diff_lines,
            delta_seconds,
        }))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (a) typed sample round-trips through to_line/from_line; a malformed /
    //     wrong-typed line is rejected (Err).
    #[test]
    fn metric_sample_roundtrip_and_reject() {
        let m = MetricSample {
            ts_unix_ns: 1_700_000_000_000_000_000,
            host_id: HostId::new("host-xyz"),
            cpu: ProcCpuSample {
                pid: 1234,
                utime_ticks: 100,
                stime_ticks: 50,
                clk_tck: 100,
                mono_ns: 999,
            },
            mem: MemSample {
                vm_rss_kb: 4096,
                vm_hwm_kb: 8192,
            },
            gpu: Some(GpuSample {
                util_pct: 42.5,
                mem_used_mb: 2048,
            }),
        };
        let line = LogEvent::Metric(m.clone()).to_line();
        let parsed = LogEvent::from_line(&line).expect("metric roundtrip must succeed");
        assert_eq!(parsed, LogEvent::Metric(m));

        // A malformed / untyped line is REJECTED, never coerced.
        assert!(LogEvent::from_line("garbage with no delimiters").is_err());
        // Unknown variant tag.
        assert!(LogEvent::from_line("Z|1|2|3").is_err());
        // Wrong-typed numeric field (string where u64 required) → Err.
        assert!(LogEvent::from_line("M|notanumber|host|1:1:1:1:1|1:1|null").is_err());
        // Wrong arity (missing nested field) → Err.
        assert!(LogEvent::from_line("M|1|host|1:1:1:1|1:1|null").is_err());
        // Wrong-typed GPU field (non-numeric util_pct) → Err.
        assert!(LogEvent::from_line("M|1|host|1:1:1:1:1|1:1|abc:2048").is_err());
    }

    // (b) gpu: Option::None is the honest default — serializes with an EXPLICIT
    //     "null" and deserializes back to None (no fake 0, never omitted-ambiguous).
    #[test]
    fn gpu_typed_none_is_explicit() {
        let m = MetricSample {
            ts_unix_ns: 1,
            host_id: HostId::new("h"),
            cpu: ProcCpuSample {
                pid: 1,
                utime_ticks: 0,
                stime_ticks: 0,
                clk_tck: 100,
                mono_ns: 0,
            },
            mem: MemSample {
                vm_rss_kb: 0,
                vm_hwm_kb: 0,
            },
            gpu: None,
        };
        let line = LogEvent::Metric(m.clone()).to_line();
        // Explicit null proves typed absence (not a fabricated 0, not omitted).
        assert!(
            line.ends_with("|null"),
            "gpu=None must serialize as explicit 'null', got {line:?}"
        );
        let parsed = LogEvent::from_line(&line).expect("null gpu must parse");
        match parsed {
            LogEvent::Metric(p) => assert!(p.gpu.is_none(), "gpu must deserialize back to None"),
            _ => panic!("wrong variant"),
        }

        // An explicitly-constructed sample without a GPU keeps gpu = None
        // (never fakes a 0).
        let d = MetricSample {
            ts_unix_ns: 0,
            host_id: HostId::new("h"),
            cpu: ProcCpuSample {
                pid: 0,
                utime_ticks: 0,
                stime_ticks: 0,
                clk_tck: 100,
                mono_ns: 0,
            },
            mem: MemSample {
                vm_rss_kb: 0,
                vm_hwm_kb: 0,
            },
            gpu: None,
        };
        assert!(d.gpu.is_none());
    }

    // (c) claim_latency_floor flags the documented real case and NOT a plausible one.
    #[test]
    fn claim_latency_floor_cases() {
        // Documented real case: 1610 lines, 52 s, floor 5 s/100-line.
        // 52 < (1610/100)*5 = 80.5 ⇒ flagged.
        assert!(claim_latency_floor(1610, 52.0, MIN_SECONDS_PER_100_LINES));
        // Same diff, plausible 90 s: 90 < 80.5 is false ⇒ NOT flagged.
        assert!(!claim_latency_floor(1610, 90.0, MIN_SECONDS_PER_100_LINES));
        // Boundary: exactly at the floor is NOT flagged (< is strict).
        assert!(!claim_latency_floor(1610, 80.5, MIN_SECONDS_PER_100_LINES));

        // The typed wrapper emits a LogEvent::ClaimLatencyAnomaly when flagged.
        let flagged = check_claim_latency("deadbeef", 1610, 52.0);
        assert!(matches!(flagged, Some(LogEvent::ClaimLatencyAnomaly(_))));
        let not_flagged = check_claim_latency("deadbeef", 1610, 90.0);
        assert!(not_flagged.is_none());
    }

    // (d) LogEvent::ClaimLatencyAnomaly exists and round-trips.
    #[test]
    fn anomaly_variant_roundtrip() {
        let a = AnomalyFlag {
            commit: "abc123".to_string(),
            diff_lines: 1610,
            delta_seconds: 52.0,
        };
        let line = LogEvent::ClaimLatencyAnomaly(a.clone()).to_line();
        assert!(
            line.starts_with("A|"),
            "anomaly line must use 'A' tag: {line}"
        );
        let parsed = LogEvent::from_line(&line).expect("anomaly must roundtrip");
        assert_eq!(parsed, LogEvent::ClaimLatencyAnomaly(a));

        // The other variants also round-trip (closed enum, fixed order).
        let c = LogEvent::ClaimLatency(ClaimLatencyRecord {
            commit: "abc123".to_string(),
            diff_lines: 200,
            delta_seconds: 30.0,
        });
        assert_eq!(LogEvent::from_line(&c.to_line()).unwrap(), c);

        let b = LogEvent::Bench(BenchRecord {
            bench: "fsm_fold".to_string(),
            wall_ms: 12.5,
        });
        assert_eq!(LogEvent::from_line(&b.to_line()).unwrap(), b);
    }
}
