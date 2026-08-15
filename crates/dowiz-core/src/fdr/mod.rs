//! `fdr` core (no_std) — the pure, allocation-only logging primitives shared by the
//! kernel's flight-data recorder.
//!
//! The std side (the durable `ring`, the `sink`/stderr writer, span timing via
//! `std::time::Instant`, the panic hook, and the `fdr_*` macros) stays in
//! `dowiz-kernel`; this crate owns the deterministic primitives that compile on every
//! target: severity [`Level`], the shared [`crc32`] authority, the enable/span-seq
//! gates, and the JSON write authority ([`json`]).
//!
//! Seam note: the kernel owns the *sink* (which is std: `Mutex<FdrRing>` + stderr + env).
//! The enable flag [`set_sink_active`]/[`sink_active`] is a no_std atomic here; the kernel
//! flips it when it installs its sink, so the disabled fast path stays a single relaxed
//! load with zero std.

pub mod cost_oracle;
pub mod digital_twin;
pub mod footprint;
pub mod json;
pub mod pmu;
pub mod schema;

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

// ── CRC32 (IEEE 802.3, reflected) — hand-rolled, compile-time table ────────────────
// ALWAYS COMPILED (NOT wasm-gated). Item 54's live-struct Sentinel runs on the kernel
// decision plane that compiles to wasm32, so the shared CRC32 primitive must be callable
// from a wasm-compiled path. One implementation shared by FDR ring (at-rest per-line
// CRC), item 40 (weights), and item 54 (live-struct Sentinel). The table is computed at
// compile time (no `OnceLock`, no runtime lazy-init — `core::sync::OnceLock` is
// unavailable on this toolchain).

const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

static CRC_TABLE: [u32; 256] = crc_table();

/// CRC32 (IEEE, reflected, `0xFFFFFFFF` init/final-xor) over `data`. Shared by FDR ring
/// (at-rest per-line CRC), item 40 (read-only weights), and item 54 (live-struct
/// Sentinel). One implementation, always compiled.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = CRC_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

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
    /// fallback). Full `RUST_LOG` target-filtering is an accepted loss.
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
/// macro. The kernel flips this when it installs its (std) sink.
static SINK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Mark a sink as installed/uninstalled (called by the kernel's std sink).
pub fn set_sink_active(v: bool) {
    SINK_ACTIVE.store(v, Ordering::Relaxed);
}

/// Is a sink installed?
pub fn sink_active() -> bool {
    SINK_ACTIVE.load(Ordering::Relaxed)
}

/// Is an event at `lvl` enabled? Requires a sink installed AND `lvl <= LEVEL`.
#[inline]
pub fn event_enabled(lvl: Level) -> bool {
    sink_active() && (lvl as u8) <= LEVEL.load(Ordering::Relaxed)
}

// ── Item 62: per-process span id minter ─────────────────────────────────────────────

/// Per-process monotone span id counter (FDR relational linkage).
static SPAN_SEQ: AtomicU64 = AtomicU64::new(0);

/// Mint the next span id. Call once per span entry. Monotone, per-process, wraps at
/// u64::MAX.
#[inline]
pub fn next_span_id() -> u64 {
    SPAN_SEQ.fetch_add(1, Ordering::Relaxed)
}
