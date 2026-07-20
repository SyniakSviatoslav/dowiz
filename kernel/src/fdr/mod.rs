//! `fdr` — the kernel's flight-data recorder: the hand-rolled logger AND the durable
//! post-mortem ring, sharing one event buffer (synthesis §5). This module is the terminal
//! state of the `tracing` / `tracing-subscriber` retirement (roadmap items 4+29).
//!
//! # Why this exists (dependency-replacement ruling — procedure §2, steps 1–5)
//!
//! Recorded here per `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` step 9(i),
//! and in full in `docs/design/BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite-2026-07-19.md`.
//!
//! 1. **Trigger:** the zero-dependency push (synthesis §0.1; roadmap §B items 4+29). The
//!    item-1 CI gate's allowlist must shrink `{regex, tracing, tracing-subscriber}` →
//!    `{regex}` in this change. (Item 5 later closed that last survivor: `{regex}` → `{}`
//!    via the kernel-owned `retrieval::pattern` matcher — the kernel's default no-dev tree
//!    is now ZERO external crates.)
//! 2. **Sweep:** the entire `tracing` surface was ~14 API lines across 7 `kernel/` files
//!    (nothing in `engine/`, `apps/`, `tools/`); 8 span names; `init_tracing` had ZERO
//!    production callers; no `info!`/`warn!`/`error!` ever used; `env-filter`'s grammar
//!    used by nobody.
//! 3. **Edge, verified in-house:** what the pair actually gave this kernel — (a) macro
//!    ergonomics; (b) span timing for 8 spans, via a `Layer` we had to deadlock-workaround
//!    and that mis-measured nested spans; (c) a global dispatch check (~one atomic load) —
//!    matched by `LEVEL` here; (d) a `fmt` dev output nobody ships. Cost, measured: 19
//!    transitive crates (a full `proc-macro2`/`quote`/`syn` toolchain to serve 6 one-line
//!    wrappers).
//! 4. **In-kernel alternative compile-checked BEFORE the flip:** this module lands complete
//!    with tests while `tracing` still present (commit 1/3); call sites flip only after
//!    green (commit 2/3); deps removed last (commit 3/3).
//! 5. **Terminal state (a) — removed outright.** Not an opt-in feature (that would keep the
//!    `SpanMetricsLayer` fork alive behind a flag — the exact outcome the procedure warns
//!    of); a logger is not a syscall/wire/ABI boundary.
//!
//! **What removal loses (honest, blueprint §5 step 3):** third-party tracing-ecosystem
//! interop (today an empty set by construction of the zero-dep push); `#[instrument]`
//! sugar; the `RUST_LOG` per-target filter grammar; span hierarchy/context propagation (no
//! consumer — the incumbent layer was explicitly single-span). **Reopening trigger (step
//! 10):** a real deployment requirement to export kernel telemetry to an external
//! tracing/OpenTelemetry collector, OR a mandatory (non-opt-in) kernel dependency that
//! needs a live tracing subscriber for its own diagnostics. Nothing else reopens it.
//!
//! # Intentional, fixed behavior divergences
//! - **Nested spans are now measured correctly.** Each `SpanGuard` owns its own `t0`, so
//!   an outer span is no longer silently dropped when an inner span is entered (the
//!   incumbent `obs.rs` thread-local stamp clobbered the outer's start — deleted here).
//! - **The `fmt` stderr format is NOT byte-reproduced.** It had zero machine consumers and
//!   zero production callers; the byte-compat contract is the parsed artifacts only
//!   (`metric.jsonl` / `alert.jsonl` / the markov CLI JSON), all golden-pinned.

pub mod json;
pub mod pmu;
pub mod schema;

#[cfg(not(target_arch = "wasm32"))]
pub mod ring;

// ── CRC32 (IEEE 802.3, reflected) — hand-rolled, table-on-first-use ──────────────────
// ALWAYS COMPILED (NOT wasm-gated). Item 54's live-struct Sentinel runs on the kernel
// decision plane that compiles to wasm32 (CLAUDE.md: "kernel … compiles to WASM"), so the
// shared CRC32 primitive must be callable from a wasm-compiled path. Lifted here from the
// wasm-gated `fdr::ring` (blueprint §3.2) — a behavior-preserving move; the KAT
// `ring::crc32_matches_known_vector` stays green. One implementation shared by FDR ring
// (at-rest per-line CRC), item 40 (weights), and item 54 (live-struct Sentinel).
static CRC_TABLE: OnceLock<[u32; 256]> = OnceLock::new();

fn crc_table() -> &'static [u32; 256] {
    CRC_TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        let mut i = 0usize;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
                k += 1;
            }
            t[i] = c;
            i += 1;
        }
        t
    })
}

/// CRC32 (IEEE, reflected, `0xFFFFFFFF` init/final-xor) over `data`. Shared by FDR ring
/// (at-rest per-line CRC), item 40 (read-only weights), and item 54 (live-struct
/// Sentinel). One implementation, always compiled.
pub fn crc32(data: &[u8]) -> u32 {
    let t = crc_table();
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = t[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Emit ONE `Alarm` FDR record (the fault-evidence kind; fsynced on append by the durable
/// ring — power-loss durable, `ring.rs:134`). No-op unless an FDR sink is installed, so
/// callers may call unconditionally; the record materializes only where FDR is enabled.
/// `struct_name` names the corrupted live struct (item 54 Safe-State evidence); `detail`
/// carries the fault reason. Item 54's fail-closed path is the value — the record is the
/// forensic side-effect.
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_alarm(struct_name: &str, detail: &str) {
    sink::emit_alarm(struct_name, detail);
}

/// On `wasm32` no FDR sink is ever installed, so alarm emission is inert. Item 54's
/// decision-plane CRC check STILL runs on wasm (it is the fail-closed path that matters);
/// only the durable record is skipped there.
#[cfg(target_arch = "wasm32")]
pub fn emit_alarm(_struct_name: &str, _detail: &str) {}

mod macros; // hoists `fdr_*` to crate root via #[macro_export]; re-exported below.

// Re-export the macros under the `fdr::` path (both `crate::fdr::info_span!` internally
// and `dowiz_kernel::fdr::info_span!` from integration tests resolve through these).
pub use crate::{
    fdr_debug as debug, fdr_error as error, fdr_event as event, fdr_info as info,
    fdr_info_span as info_span, fdr_trace as trace, fdr_warn as warn,
};

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};

// ── Level ────────────────────────────────────────────────────────────────────────────

/// Severity level. Lower = more severe (so `lvl <= LEVEL` = "enabled at this threshold").
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }

    /// Parse the level-only `DOWIZ_LOG` grammar (mirrors the old `EnvFilter::new("info")`
    /// fallback at `lib.rs:404`). Full `RUST_LOG` target-filtering is an accepted loss.
    pub fn from_env_str(s: &str) -> Option<Level> {
        match s.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Level::Error),
            "warn" => Some(Level::Warn),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            "trace" => Some(Level::Trace),
            _ => None,
        }
    }
}

/// Global level threshold. Default `Info` (matches the incumbent `EnvFilter` default).
static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

pub fn set_level(l: Level) {
    LEVEL.store(l as u8, Ordering::Relaxed);
}

// ── Enable checks (the disabled fast path) ──────────────────────────────────────────

/// True iff a sink is installed. One relaxed load — the disabled-path cost of an event
/// macro (matching `tracing`'s dispatch-check cheapness).
static SINK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Is an event at `lvl` enabled? Requires a sink installed AND `lvl <= LEVEL`. On `wasm32`
/// no sink is ever installed, so this is always `false` and no event is ever built.
#[inline]
pub fn event_enabled(lvl: Level) -> bool {
    SINK_ACTIVE.load(Ordering::Relaxed) && (lvl as u8) <= LEVEL.load(Ordering::Relaxed)
}

/// Is span timing active (an observer OR a sink is installed)? Governs whether
/// `SpanHandle::entered` takes an `Instant`. Always `false` on `wasm32` (neither is ever
/// installed there), so the hot-path span never touches the wasm-panicking clock.
#[inline]
pub fn span_active() -> bool {
    SINK_ACTIVE.load(Ordering::Relaxed)
        || GLOBAL_OBSERVER.get().is_some()
        || TL_OBSERVER.with(|o| o.borrow().is_some())
}

// ── Span observer (kernel-owned port of the tracing Layer hook) ─────────────────────

/// The kernel-owned replacement for `tracing_subscriber::Layer`'s span hook. The P83
/// `SpanMetricsObserver` (in `span_metrics/obs.rs`) implements this and folds the duration
/// into the `metric.jsonl` log-bucket histogram — byte-identical to the incumbent.
pub trait SpanObserver: Send + Sync {
    fn on_span_close(&self, name: &'static str, dur_us: u64);
}

static GLOBAL_OBSERVER: OnceLock<Arc<dyn SpanObserver>> = OnceLock::new();

thread_local! {
    /// Scoped (test) observer — mirrors the incumbent `set_default`/`DefaultGuard` pattern
    /// (thread-local, reverts on guard drop), so `span_metrics_init_wire.rs`'s scoped tests
    /// keep working without a process-global install.
    static TL_OBSERVER: RefCell<Option<Arc<dyn SpanObserver>>> = const { RefCell::new(None) };
}

/// Install a process-global span observer (set-once, like the incumbent
/// `set_global_default`). `Err(())` if one is already installed.
pub fn set_global_observer(obs: Arc<dyn SpanObserver>) -> Result<(), ()> {
    GLOBAL_OBSERVER.set(obs).map_err(|_| ())
}

/// A scoped observer install that reverts on drop (the `init_scoped` equivalent).
#[must_use = "the scoped observer is uninstalled when this guard is dropped"]
pub struct ObserverGuard {
    prev: Option<Arc<dyn SpanObserver>>,
}

impl Drop for ObserverGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        TL_OBSERVER.with(|o| *o.borrow_mut() = prev);
    }
}

/// Install `obs` as this thread's scoped observer, returning a guard that restores the
/// previous one on drop.
pub fn set_scoped_observer(obs: Arc<dyn SpanObserver>) -> ObserverGuard {
    let prev = TL_OBSERVER.with(|o| o.borrow_mut().replace(obs));
    ObserverGuard { prev }
}

/// Route a span close to the thread-local observer first (test/scoped), else the global.
fn notify_observer(name: &'static str, dur_us: u64) {
    let handled = TL_OBSERVER.with(|o| {
        if let Some(obs) = o.borrow().as_ref() {
            obs.on_span_close(name, dur_us);
            true
        } else {
            false
        }
    });
    if !handled {
        if let Some(obs) = GLOBAL_OBSERVER.get() {
            obs.on_span_close(name, dur_us);
        }
    }
}

// ── Span handle / guard ─────────────────────────────────────────────────────────────

/// A span handle produced by `fdr::info_span!`. Cheap (just a `&'static str`); does NO
/// work until `.entered()`.
pub struct SpanHandle {
    name: &'static str,
}

impl SpanHandle {
    #[inline]
    pub fn new(name: &'static str) -> Self {
        SpanHandle { name }
    }

    /// Enter the span, returning a guard whose `Drop` reports the elapsed wall-clock time.
    /// The `Instant` is taken ONLY when span timing is active, and is gated off `wasm32`
    /// entirely (blueprint §4.1 "wasm32 trap") — a naive always-stamp guard would break the
    /// cdylib, since `Instant::now()` panics on `wasm32-unknown-unknown`.
    #[inline]
    pub fn entered(self) -> SpanGuard {
        #[cfg(not(target_arch = "wasm32"))]
        let t0 = if span_active() {
            Some(std::time::Instant::now())
        } else {
            None
        };
        #[cfg(target_arch = "wasm32")]
        let t0: Option<std::time::Instant> = None;
        SpanGuard {
            name: self.name,
            t0,
        }
    }
}

/// The span timing guard. On drop (when a timing was taken) it reports the duration to the
/// observer and, if a ring sink is installed, writes a `span_close` FDR record.
pub struct SpanGuard {
    name: &'static str,
    t0: Option<std::time::Instant>,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(t0) = self.t0 {
            let dur_us = t0.elapsed().as_micros().min(u64::MAX as u128) as u64;
            notify_observer(self.name, dur_us);
            emit_span_close(self.name, dur_us);
        }
        // On wasm32, `t0` is always `None` (see `entered`) and this Drop is inert.
        #[cfg(target_arch = "wasm32")]
        let _ = &self.t0;
    }
}

// ── Event / span-close emission (sink write path) ───────────────────────────────────

/// Emit an event record (from `fdr::debug!`/`info!`/…). No-op unless a sink is installed;
/// the real body is gated off `wasm32` (the FDR write path — `SystemTime`/`Instant` stamps
/// + file I/O — is never reached on wasm, where no sink is installed).
pub fn emit_event(level: Level, msg: &str, fields: &[(&'static str, String)]) {
    #[cfg(not(target_arch = "wasm32"))]
    sink::emit_event(level, msg, fields);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (level, msg, fields);
    }
}

/// Emit a span-close FDR record to the ring sink (spans do NOT go to stderr — they go to
/// the observer's `metric.jsonl` and, when durable, the ring).
#[cfg(not(target_arch = "wasm32"))]
fn emit_span_close(name: &'static str, dur_us: u64) {
    sink::emit_span_close(name, dur_us);
}

/// Emit ONE `Tuning`-kind FDR record carrying an autonomic gain-scheduling
/// adjustment (roadmap item 21). No-op unless a sink is installed — so callers
/// may emit unconditionally at zero default cost; the record materializes only
/// where FDR is enabled. The `Kind::Tuning` variant is reserved for this use
/// (schema.rs:211).
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_tuning(name: &'static str, fields: &[(&'static str, String)]) {
    sink::emit_tuning(name, fields);
}
#[cfg(target_arch = "wasm32")]
pub fn emit_tuning(_name: &'static str, _fields: &[(&'static str, String)]) {}

/// Emit ONE `Event`-kind FDR record carrying a classifier verdict string plus the
/// window-bracketed [`pmu::PmuStamp`] companion delta (roadmap item 27, classifier-input
/// half). No-op unless a sink is installed — so callers may bracket-and-emit
/// unconditionally at zero default cost; the record materializes only where FDR is enabled.
/// The classifier (`markov::analyze_detailed` / `spectral::classify_drift`) is unchanged:
/// the PMU data rides ALONGSIDE the verdict on the same record, never as a classifier input.
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_verdict_pmu(name: &'static str, verdict: &str, pmu: pmu::PmuStamp) {
    sink::emit_verdict(name, verdict, pmu);
}

/// Emit a subprocess-reap record to the FDR ring (item 61, gap G6). Carries the REAL
/// measured wall duration + `wait4` rusage — never a fabricated 0. The record is P3-plane
/// telemetry (named `subprocess_bridge`); it is recoverable from the FDR ring after the
/// call (the red→green oracle reads it back). No-op unless a ring sink is installed.
///
/// NOTE (blueprint ambiguity resolution): the blueprint asks this to emit an item-58
/// `(work: FdrRecordsAppended …⊕ SignaturesVerified)` workload-kind pair. Item 58's
/// `WorkloadKind`/`Work`/`work`-field FDR schema is NOT present in this worktree (grep
/// confirmed), so item 61 cannot write it without forging the schema. Instead we emit the
/// same recoverable, greppable `subprocess_bridge` ring record under item 61's own name,
/// ledger-named absent via a `gap:` row in `HOT-PATHS.tsv` (item 57's zero-un-named-blind-
/// spots law). When item 58 lands, this becomes a `WorkloadKind::…` `SpanClose` pair — the
/// fields below are exactly the raw-`u64` pair the schema wants.
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_subprocess_record(
    cmd: &str,
    success: bool,
    wall_us: u64,
    utime_us: Option<u64>,
    stime_us: Option<u64>,
    maxrss_kb: Option<u64>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    sink::emit_subprocess_record(cmd, success, wall_us, utime_us, stime_us, maxrss_kb);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (cmd, success, wall_us, utime_us, stime_us, maxrss_kb);
    }
}

// ── Sink + init (non-wasm) ──────────────────────────────────────────────────────────

/// Configuration for [`init`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct FdrConfig {
    /// Write event records to stderr (deterministic NDJSON, no ANSI).
    pub stderr: bool,
    /// If `Some`, install the durable A/B segment ring under this directory.
    pub ring_dir: Option<std::path::PathBuf>,
    /// Per-segment cap (bytes).
    pub seg_cap: u64,
    /// Level threshold.
    pub level: Level,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for FdrConfig {
    fn default() -> Self {
        FdrConfig {
            stderr: true,
            ring_dir: None,
            seg_cap: ring::DEFAULT_SEG_CAP,
            level: Level::Info,
        }
    }
}

/// Install the FDR sink (replaces `init_tracing()`). Reads `DOWIZ_LOG` for the level when
/// set. Idempotent-ish: a second call is a no-op (the sink is set-once, like the incumbent
/// global subscriber). Never installed on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub fn init(config: FdrConfig) -> Result<(), ()> {
    let r = sink::init(config);
    // Item 48 (closure a): install the panic hook on the FDR init path. Idempotent via the
    // guard inside `install_panic_hook` (a second `init` is a no-op for the sink, so the
    // hook is not re-chained). Harmless when no ring is configured (the Alarm append becomes
    // a no-op — exactly like every other FDR emit).
    install_panic_hook();
    r
}

// ── Item 48: FDR blind-spot closure (panic forensics + liveness heartbeat) ──────────

static PANIC_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Item 48 (closure a): install a panic hook that emits ONE fsynced `Alarm` FDR record
/// carrying the panic `message` + `location`, then CHAINS to any previous hook. Best-effort:
/// errors are swallowed (like `emit_event`), so a panic-in-hook double-panic aborts — the OS
/// retains whatever bytes reached the page cache. **NOT** a `#[panic_handler]` (this is a
/// `std` kernel); uses `std::panic::set_hook`. Reuses the existing ring sink (which fsyncs
/// `Alarm` at `ring.rs:134`), so closure (a) needs NO new durability code — just an emitter.
/// Chains to the prior hook so a test harness's hook is not clobbered (blueprint §10
/// DESIGN NOTE). Idempotent: a second call is a no-op (guards against `init` being invoked
/// twice). Non-wasm only — matches the FDR write path (no sink is ever installed on wasm).
#[cfg(not(target_arch = "wasm32"))]
pub fn install_panic_hook() {
    if PANIC_HOOK_INSTALLED.swap(true, Ordering::Relaxed) {
        return; // already installed.
    }
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo| {
        // Build the forensic payload BEFORE touching the sink (keep the hook panic-safe /
        // allocation-light). A re-panic here aborts — acceptable; the OS keeps the page-cache
        // bytes from the first append attempt.
        let message = match info.payload().downcast_ref::<&str>() {
            Some(s) => (*s).to_string(),
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s.clone(),
                None => "<non-string panic payload>".to_string(),
            },
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown location>".to_string());
        sink::emit_panic_alarm(message, location);
        // Chain to the previous hook (a harness-installed hook must not be clobbered).
        prev(info);
    }));
}

/// Item 48 (closure b): emit ONE `Heartbeat` FDR record carrying a monotonic `seq` + the
/// caller-supplied `progress` counters. Cheap stamp policy (high-frequency; same rationale
/// as `Event` at `mod.rs:375`). Emit-only — the EXTERNAL layer owns cadence + JUDGMENT
/// (systemd `WatchdogSec` / `hub_supervisor`); the kernel NEVER runs a liveness timer and
/// NEVER self-kills. No-op unless a ring sink is installed (matches every other FDR emit).
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_heartbeat(seq: u64, progress: &[(&'static str, String)]) {
    sink::emit_heartbeat(seq, progress);
}

/// Item 48 (clean-shutdown interplay, blueprint §5 step 3): write the `CleanShutdown`
/// marker so an external liveness check sees an orderly stop and raises NO false alarm.
/// No-op when no ring sink is installed (matches every other FDR emit).
#[cfg(not(target_arch = "wasm32"))]
pub fn clean_shutdown() {
    sink::clean_shutdown();
}

#[cfg(not(target_arch = "wasm32"))]
mod sink {
    use super::schema::{FdrEvent, Kind, StampPolicy};
    use super::{ring, FdrConfig, Level, SINK_ACTIVE};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    pub struct Sink {
        pub stderr: bool,
        pub ring: Option<Mutex<ring::FdrRing>>,
        pub seq: AtomicU64,
    }

    static SINK: OnceLock<Sink> = OnceLock::new();

    pub fn init(config: FdrConfig) -> Result<(), ()> {
        let level = std::env::var("DOWIZ_LOG")
            .ok()
            .and_then(|s| Level::from_env_str(&s))
            .unwrap_or(config.level);
        super::set_level(level);

        let ring = match &config.ring_dir {
            Some(dir) => match ring::FdrRing::open(dir.clone(), config.seg_cap) {
                Ok(r) => Some(Mutex::new(r)),
                Err(_) => None, // best-effort: a failed ring open never poisons the caller.
            },
            None => None,
        };
        let sink = Sink {
            stderr: config.stderr,
            ring,
            seq: AtomicU64::new(0),
        };
        match SINK.set(sink) {
            Ok(()) => {
                SINK_ACTIVE.store(true, Ordering::Relaxed);
                Ok(())
            }
            Err(_) => Err(()), // already installed.
        }
    }

    fn next_seq(s: &Sink) -> u64 {
        s.seq.fetch_add(1, Ordering::Relaxed)
    }

    pub fn emit_event(level: Level, msg: &str, fields: &[(&'static str, String)]) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let seq = next_seq(s);
        // Event-kind records use the CHEAP hw policy (blueprint §4.2 cost control — high
        // frequency; joules-per-span is a consumer delta over the alarm-class stamps).
        let ev = FdrEvent::stamp(
            seq,
            level,
            Kind::Event,
            msg.to_string(),
            StampPolicy::Cheap,
            fields.to_vec(),
        );
        let line = ev.to_json();
        if s.stderr {
            let _ = writeln!(std::io::stderr(), "{line}");
        }
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.append(&ev);
            }
        }
    }

    /// Emit a verdict + PMU companion record (roadmap item 27). Verdict-emission points are
    /// low-frequency, so this uses a FULL hw stamp like span-close; the PMU stamp is attached
    /// verbatim (it was already sampled by the caller's bracket). NEVER called from inside a
    /// classifier — only from the verdict emission site (`bin/markov_attractor`).
    pub fn emit_verdict(name: &'static str, verdict: &str, pmu: super::pmu::PmuStamp) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let seq = next_seq(s);
        let mut ev = FdrEvent::stamp(
            seq,
            Level::Info,
            Kind::Event,
            name.to_string(),
            StampPolicy::Full,
            vec![("verdict", verdict.to_string())],
        );
        ev.pmu = Some(pmu);
        let line = ev.to_json();
        if s.stderr {
            let _ = writeln!(std::io::stderr(), "{line}");
        }
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.append(&ev);
            }
        }
    }

    /// Item 61 (gap G6): emit a `subprocess_bridge` FDR record carrying the REAL measured
    /// wall duration + `wait4` rusage (user/sys time, maxrss). The rusage fields are
    /// `Option<u64>` so a non-Linux/non-x86_64 host emits a *named* absence (the field key
    /// is present with `null`), never a fabricated `0`. The record is P3-plane telemetry and
    /// lives on the FDR ring (recoverable by the item-61 oracle). No-op unless a ring sink
    /// is installed.
    pub fn emit_subprocess_record(
        cmd: &str,
        success: bool,
        wall_us: u64,
        utime_us: Option<u64>,
        stime_us: Option<u64>,
        maxrss_kb: Option<u64>,
    ) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let r = match &s.ring {
            Some(r) => r,
            None => return, // no ring ⇒ no durable record (skip silently).
        };
        let seq = next_seq(s);
        // Subprocess reaps are low-frequency, so a FULL hw stamp (ring/event-style).
        let mut fields: Vec<(&'static str, String)> = vec![
            ("cmd", cmd.to_string()),
            ("success", success.to_string()),
            ("wall_us", wall_us.to_string()),
            (
                "utime_us",
                utime_us.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ),
            (
                "stime_us",
                stime_us.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ),
            (
                "maxrss_kb",
                maxrss_kb.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ),
        ];
        let ev = FdrEvent::stamp(
            seq,
            Level::Info,
            Kind::SpanClose,
            "subprocess_bridge".to_string(),
            StampPolicy::Full,
            fields.drain(..).collect(),
        );
        if let Ok(mut r) = r.lock() {
            let _ = r.append(&ev);
        }
    }

    pub fn emit_span_close(name: &'static str, dur_us: u64) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let r = match &s.ring {
            Some(r) => r,
            None => return, // no ring ⇒ span close already handled by the observer only.
        };
        let seq = next_seq(s);
        // Span-close of the instrumented functions gets a FULL hw stamp (blueprint §4.2).
        let ev = FdrEvent::stamp(
            seq,
            Level::Info,
            Kind::SpanClose,
            name.to_string(),
            StampPolicy::Full,
            vec![("dur_us", dur_us.to_string())],
        );
        if let Ok(mut r) = r.lock() {
            let _ = r.append(&ev);
        }
    }

    /// Item 48 (closure a): emit ONE `Alarm`-kind FDR record carrying the panic `message`
    /// + `location` into the ring sink. `Alarm` is fsynced by `FdrRing::append` (ring.rs:134)
    /// so it is power-loss durable for free — the panic forensic evidence survives even a
    /// crash before the process unwinds. Best-effort: errors are swallowed (like
    /// `emit_event` at mod.rs:389-393), so a write failure never turns a panic into a
    /// double-panic. No-op when no sink is installed (matches every other FDR emit).
    pub fn emit_panic_alarm(message: String, location: String) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let seq = next_seq(s);
        let ev = FdrEvent::stamp(
            seq,
            Level::Error,
            Kind::Alarm,
            "panic".to_string(),
            StampPolicy::Full,
            vec![("message", message), ("location", location)],
        );
        if s.stderr {
            let _ = writeln!(std::io::stderr(), "{}", ev.to_json());
        }
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.append(&ev); // Alarm ⇒ fsync inside append (ring.rs:134).
            }
        }
    }

    /// Item 54 Safe-State fault evidence: emit ONE `Alarm` FDR record naming a corrupted
    /// live struct (`name`) with the fault `detail`, fsynced via the durable ring (the ring
    /// fsyncs on `Kind::Alarm`, `ring.rs:134` — power-loss durable). This is the sink-side
    /// impl the `fdr::emit_alarm` facade (mod.rs:101) delegates to. No-op when no sink/ring
    /// is installed.
    pub fn emit_alarm(name: &str, detail: &str) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let seq = next_seq(s);
        // Alarms carry a FULL hw stamp (alarm-class, blueprint §4.2) for forensic value.
        let ev = FdrEvent::stamp(
            seq,
            Level::Error,
            Kind::Alarm,
            name.to_string(),
            StampPolicy::Full,
            vec![("detail", detail.to_string())],
        );
        let line = ev.to_json();
        if s.stderr {
            let _ = writeln!(std::io::stderr(), "{line}");
        }
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.append(&ev);
            }
        }
    }

    /// Item 48 (closure b): emit ONE `Heartbeat`-kind FDR record carrying a monotonic `seq`
    /// + the caller-supplied `progress` counters. Cheap stamp policy (high-frequency; same
    /// rationale as `Event`). The kernel provides ONLY the record + emit fn — cadence and
    /// JUDGMENT live externally (systemd `WatchdogSec` / `hub_supervisor`). No-op when no
    /// sink is installed (matches every other FDR emit).
    /// Item 21 (roadmap §E): emit ONE `Tuning`-kind FDR record carrying an
    /// autonomic gain-scheduling adjustment. Cheap stamp policy (high-frequency
    /// self-tuning events). No-op when no sink is installed (matches every other
    /// FDR emit).
    pub fn emit_tuning(name: &'static str, fields: &[(&'static str, String)]) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let seq = next_seq(s);
        let ev = FdrEvent::stamp(
            seq,
            Level::Info,
            Kind::Tuning,
            name.to_string(),
            StampPolicy::Cheap,
            fields.to_vec(),
        );
        if s.stderr {
            let _ = writeln!(std::io::stderr(), "{}", ev.to_json());
        }
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.append(&ev);
            }
        }
    }

    pub fn emit_heartbeat(seq: u64, progress: &[(&'static str, String)]) {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        let ev = FdrEvent::stamp(
            seq,
            Level::Info,
            Kind::Heartbeat,
            "heartbeat".to_string(),
            StampPolicy::Cheap,
            progress.to_vec(),
        );
        if s.stderr {
            let _ = writeln!(std::io::stderr(), "{}", ev.to_json());
        }
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.append(&ev);
            }
        }
    }

    /// Item 48 (clean-shutdown interplay, blueprint §5 step 3): write the `CleanShutdown`
    /// marker so an external liveness check sees an orderly stop and raises NO false alarm.
    /// No-op when no ring sink is installed (matches every other FDR emit).
    pub fn clean_shutdown() {
        let s = match SINK.get() {
            Some(s) => s,
            None => return,
        };
        if let Some(r) = &s.ring {
            if let Ok(mut r) = r.lock() {
                let _ = r.clean_shutdown();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering_and_parse() {
        assert!(Level::Error < Level::Info);
        assert!(Level::Debug > Level::Info);
        assert_eq!(Level::from_env_str("DEBUG"), Some(Level::Debug));
        assert_eq!(Level::from_env_str("nonsense"), None);
        assert_eq!(Level::Info.as_str(), "info");
    }

    #[test]
    fn disabled_span_takes_no_timing_and_is_inert() {
        // A span with no scoped observer on THIS thread must construct, enter, and drop
        // without panicking (and, on wasm, without ever touching the clock). We don't
        // assert span_active() here: a sibling test may have installed a process-global
        // observer, which is fine — this only pins that the disabled/no-op drop path is
        // panic-free. The wasm build is the real no-clock proof.
        let g = SpanHandle::new("noop_span").entered();
        drop(g);
    }

    #[test]
    fn scoped_observer_receives_span_close() {
        use std::sync::atomic::{AtomicU64, Ordering};
        struct Obs(Arc<AtomicU64>);
        impl SpanObserver for Obs {
            fn on_span_close(&self, name: &'static str, _dur_us: u64) {
                if name == "scoped_probe" {
                    self.0.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        let hits = Arc::new(AtomicU64::new(0));
        let _guard = set_scoped_observer(Arc::new(Obs(hits.clone())));
        {
            let _s = SpanHandle::new("scoped_probe").entered();
        }
        assert_eq!(hits.load(Ordering::Relaxed), 1, "observer must see the span close");
    }

    // Item 48 (closure b): `emit_heartbeat` is a no-op when no FDR sink is installed (the
    // `SINK.get()` early-return, matching every other emit). Guards the "zero cost, no crash"
    // failure-mode in blueprint §6.
    #[test]
    fn emit_heartbeat_is_noop_without_sink() {
        // No `fdr::init` → no sink. Must not panic and must not allocate the ring path.
        emit_heartbeat(0, &[("tick", "0".to_string())]);
    }

    // Item 48 (closure a): the panic hook CHAINS to a prior hook rather than clobbering it
    // (blueprint §10 DESIGN NOTE, §5 step 1). A harness-installed hook must still fire.
    #[test]
    fn panic_hook_chains_to_prior_hook() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static PRIOR_FIRED: AtomicBool = AtomicBool::new(false);
        // Simulate a test harness's pre-existing hook.
        std::panic::set_hook(Box::new(|_| {
            PRIOR_FIRED.store(true, Ordering::SeqCst);
        }));
        // install_panic_hook takes+chains the prior hook, then sets its own.
        install_panic_hook();
        // Trigger a panic; BOTH the item-48 hook (emit, no sink → no-op) and the prior
        // harness hook must run. We catch the abort via catch_unwind.
        let _ = std::panic::catch_unwind(|| {
            panic!("chained hook test");
        });
        assert!(
            PRIOR_FIRED.load(Ordering::SeqCst),
            "prior (harness) hook must still fire after install_panic_hook"
        );
        // Restore a default-ish hook (not strictly necessary; keeps test isolation sane).
        let _ = std::panic::take_hook();
    }
}
