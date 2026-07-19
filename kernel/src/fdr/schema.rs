//! `fdr/schema.rs` — the FDR event envelope with energy/hardware fields **first-class
//! from day one** (synthesis §21; blueprint §4.2).
//!
//! Every FDR record carries a fixed envelope whose `hw` field is a NON-optional struct,
//! so schema-level omission of the hardware stamp is unrepresentable. Each hardware
//! reading is a [`Reading<T>`] = `Value(T) | Unavailable(Absence)` with a CLOSED reason
//! enum, and the field is ALWAYS serialized — never silently dropped. This upgrades the
//! pre-existing `gpu: Option<GpuSample> = None` "typed absence, never a fake 0" precedent
//! (`typed_metrics.rs`) by additionally carrying the *reason* for absence.
//!
//! Energy (`joules_uj`) is genuinely new code — a repo-wide sweep found ZERO prior
//! `rapl|joule|energy_uj|powercap` references (blueprint §3.2). On a host without a RAPL
//! interface (this one — `/sys/class/powercap/` is empty) the field serializes as
//! `"joules_uj":{"unavailable":"no_rapl_interface"}` — a greppable named absence, not a
//! missing key.
//!
//! `/proc` CPU/RSS readers are REUSED from `typed_metrics.rs` (blueprint §3.1), not
//! rebuilt.

use super::json::JsonWriter;
use super::Level;

/// The closed set of reasons a hardware reading can be unavailable. Serialized by name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Absence {
    /// Not a Linux host — `/proc`, `/sys` sampling is a Linux-only capability.
    NonLinuxHost,
    /// Linux, but no RAPL powercap interface is exposed (no `intel-rapl:0/energy_uj`).
    NoRaplInterface,
    /// The interface exists but is not readable by this process.
    PermissionDenied,
    /// The interface exists and is readable but returned malformed/short data.
    ReadError,
    /// Stamping was intentionally skipped for this record class (cost control — see
    /// [`StampPolicy::Cheap`]). First-class and truthful, not a silent omission.
    SamplingDisabled,
}

impl Absence {
    /// The stable serialized name (snake_case; greppable).
    pub fn as_str(self) -> &'static str {
        match self {
            Absence::NonLinuxHost => "non_linux_host",
            Absence::NoRaplInterface => "no_rapl_interface",
            Absence::PermissionDenied => "permission_denied",
            Absence::ReadError => "read_error",
            Absence::SamplingDisabled => "sampling_disabled",
        }
    }
}

/// A hardware reading: a concrete value or a *named* absence. The field is ALWAYS
/// present in the serialized record (see [`Reading::write_field`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reading<T> {
    Value(T),
    Unavailable(Absence),
}

impl Reading<u64> {
    /// Serialize as `"key":<n>` when a value exists, else
    /// `"key":{"unavailable":"<reason>"}`. Either way the key is emitted — the
    /// "named absence, not silent omission" guarantee, mechanically.
    pub fn write_field(self, w: JsonWriter, key: &str) -> JsonWriter {
        match self {
            Reading::Value(v) => w.field_u64(key, v),
            Reading::Unavailable(a) => {
                let mut raw = String::with_capacity(24);
                raw.push_str("{\"unavailable\":\"");
                raw.push_str(a.as_str());
                raw.push_str("\"}");
                w.field_raw(key, &raw)
            }
        }
    }

    /// True iff this reading is a named absence (test/consumer helper).
    pub fn is_unavailable(self) -> bool {
        matches!(self, Reading::Unavailable(_))
    }
}

/// Per-record hardware stamp. All three fields are first-class `Reading`s; the struct
/// is never `Option` — omission is unrepresentable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HwStamp {
    /// utime+stime ticks (`/proc/self/stat`, reader reused from `typed_metrics`).
    pub cpu_ticks: Reading<u64>,
    /// VmRSS kB (`/proc/self/status`, reader reused from `typed_metrics`).
    pub rss_kb: Reading<u64>,
    /// RAPL energy counter in µJ (`/sys/class/powercap/intel-rapl:0/energy_uj`). NEW.
    /// The kernel emits the raw monotone counter only; joules-per-span is a consumer
    /// delta (same losslessness rule as `metrics.rs`'s "CPU-% is a derived concern").
    pub joules_uj: Reading<u64>,
}

/// Stamp cost policy (blueprint §4.2 "honest cost control"). `Full` reads `/proc`+`/sys`
/// (µs-scale syscalls) for alarm-class records; `Cheap` records a first-class
/// `SamplingDisabled` for high-frequency event-kind records instead of taxing hot paths.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StampPolicy {
    Full,
    Cheap,
}

impl HwStamp {
    /// Sample the hardware stamp under `policy`.
    pub fn sample(policy: StampPolicy) -> Self {
        match policy {
            StampPolicy::Cheap => HwStamp {
                cpu_ticks: Reading::Unavailable(Absence::SamplingDisabled),
                rss_kb: Reading::Unavailable(Absence::SamplingDisabled),
                joules_uj: Reading::Unavailable(Absence::SamplingDisabled),
            },
            StampPolicy::Full => HwStamp {
                cpu_ticks: read_cpu_ticks(),
                rss_kb: read_rss_kb(),
                joules_uj: read_joules_uj(),
            },
        }
    }

    /// Serialize as a nested `"hw":{...}` object. Every field is present.
    pub fn write(self, w: JsonWriter) -> JsonWriter {
        let inner = JsonWriter::obj();
        let inner = self.cpu_ticks.write_field(inner, "cpu_ticks");
        let inner = self.rss_kb.write_field(inner, "rss_kb");
        let inner = self.joules_uj.write_field(inner, "joules_uj");
        w.field_raw("hw", &inner.finish())
    }
}

/// utime+stime ticks, via the reused `typed_metrics` reader. `None` (non-Linux/unreadable)
/// ⇒ `NonLinuxHost` (the reader only returns `None` when `/proc/self/stat` is unreadable).
fn read_cpu_ticks() -> Reading<u64> {
    match crate::typed_metrics::ProcCpuSample::from_proc_self() {
        Some(s) => Reading::Value(s.utime_ticks + s.stime_ticks),
        None => Reading::Unavailable(Absence::NonLinuxHost),
    }
}

/// VmRSS kB, via the reused `typed_metrics` reader.
fn read_rss_kb() -> Reading<u64> {
    match crate::typed_metrics::MemSample::from_proc_self() {
        Some(m) => Reading::Value(m.vm_rss_kb),
        None => Reading::Unavailable(Absence::NonLinuxHost),
    }
}

/// RAPL energy counter (µJ) from `intel-rapl:0` — NEW code (no prior energy reader
/// existed anywhere in the kernel). Degrades to a *named* absence on every failure
/// mode; never fabricates a `0`.
pub fn read_joules_uj() -> Reading<u64> {
    #[cfg(not(target_os = "linux"))]
    {
        Reading::Unavailable(Absence::NonLinuxHost)
    }
    #[cfg(target_os = "linux")]
    {
        const PATH: &str = "/sys/class/powercap/intel-rapl:0/energy_uj";
        match std::fs::read_to_string(PATH) {
            Ok(s) => match s.trim().parse::<u64>() {
                Ok(v) => Reading::Value(v),
                Err(_) => Reading::Unavailable(Absence::ReadError),
            },
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Reading::Unavailable(Absence::NoRaplInterface),
                std::io::ErrorKind::PermissionDenied => {
                    Reading::Unavailable(Absence::PermissionDenied)
                }
                _ => Reading::Unavailable(Absence::ReadError),
            },
        }
    }
}

/// Record kind (closed enum). `Tuning` is reserved for item-21's FDR-logged adjustments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Event,
    SpanClose,
    Alarm,
    PostMortem,
    Tuning,
    /// Written on orderly shutdown — its presence as the tail record marks a clean stop
    /// (its ABSENCE at recovery time ⇒ dirty stop ⇒ post-mortem, blueprint §4.4).
    CleanShutdown,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Event => "event",
            Kind::SpanClose => "span_close",
            Kind::Alarm => "alarm",
            Kind::PostMortem => "post_mortem",
            Kind::Tuning => "tuning",
            Kind::CleanShutdown => "clean_shutdown",
        }
    }
}

/// One FDR record. Fixed envelope; `hw` is first-class (never `Option`).
#[derive(Clone, Debug)]
pub struct FdrEvent {
    /// Monotonic per-process sequence number (the recovery ordering key).
    pub seq: u64,
    /// Wall-clock ns (forensic/display plane; NOT the replay-ordering key).
    pub ts_unix_ns: u128,
    /// Monotonic ns since process start (`typed_metrics::mono_now_ns`).
    pub mono_ns: u128,
    pub level: Level,
    pub kind: Kind,
    pub name: String,
    pub hw: HwStamp,
    pub fields: Vec<(&'static str, String)>,
}

impl FdrEvent {
    /// Build a record and stamp `ts`/`mono`/`hw`. Non-wasm: `SystemTime::now()` and
    /// `Instant::now()` (via `mono_now_ns`) panic on `wasm32-unknown-unknown`, and the
    /// FDR write path is never reached on wasm (no sink is ever installed there), so the
    /// stamping constructor is gated off wasm — the disabled/no-op path takes NO clock.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn stamp(
        seq: u64,
        level: Level,
        kind: Kind,
        name: String,
        hw_policy: StampPolicy,
        fields: Vec<(&'static str, String)>,
    ) -> Self {
        let ts_unix_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mono_ns = crate::typed_metrics::mono_now_ns();
        FdrEvent {
            seq,
            ts_unix_ns,
            mono_ns,
            level,
            kind,
            name,
            hw: HwStamp::sample(hw_policy),
            fields,
        }
    }

    /// Deterministic NDJSON serialization (fixed field order). Pure — compiles on wasm.
    pub fn to_json(&self) -> String {
        let w = JsonWriter::obj()
            .field_u64("seq", self.seq)
            .field_u128("ts_unix_ns", self.ts_unix_ns)
            .field_u128("mono_ns", self.mono_ns)
            .field_str("level", self.level.as_str())
            .field_str("kind", self.kind.as_str())
            .field_str("name", &self.name);
        let w = self.hw.write(w);
        let mut fobj = JsonWriter::obj();
        for (k, v) in &self.fields {
            fobj = fobj.field_str(k, v);
        }
        w.field_raw("fields", &fobj.finish()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapl_absent_host_reports_named_absence_not_missing_key() {
        // This host has no RAPL interface (blueprint §3.2 / verified empty
        // /sys/class/powercap). The field must be PRESENT with a named reason.
        let r = read_joules_uj();
        assert!(r.is_unavailable(), "expected RAPL-less host to report Unavailable");
        let w = r.write_field(JsonWriter::obj(), "joules_uj").finish();
        // Greppable named absence — key present, reason spelled out (§G.9 proof).
        assert!(w.contains("\"joules_uj\":{\"unavailable\":"), "field must be present: {w}");
        assert!(w.contains("unavailable"), "reason must be greppable: {w}");
    }

    #[test]
    fn hw_field_is_always_present_even_when_all_unavailable() {
        let hw = HwStamp {
            cpu_ticks: Reading::Unavailable(Absence::SamplingDisabled),
            rss_kb: Reading::Unavailable(Absence::SamplingDisabled),
            joules_uj: Reading::Unavailable(Absence::NoRaplInterface),
        };
        let s = hw.write(JsonWriter::obj()).finish();
        assert!(s.contains("\"hw\":{"), "hw must be first-class: {s}");
        assert!(s.contains("\"cpu_ticks\":{\"unavailable\":\"sampling_disabled\"}"));
        assert!(s.contains("\"joules_uj\":{\"unavailable\":\"no_rapl_interface\"}"));
    }

    #[test]
    fn reading_value_serializes_as_bare_number() {
        let s = Reading::Value(12345u64)
            .write_field(JsonWriter::obj(), "joules_uj")
            .finish();
        assert_eq!(s, "{\"joules_uj\":12345}");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn event_roundtrips_to_deterministic_json() {
        let ev = FdrEvent {
            seq: 7,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: Level::Info,
            kind: Kind::Event,
            name: "place_order".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            fields: vec![("subtotal_cents", "500".into())],
        };
        let j = ev.to_json();
        assert!(j.starts_with("{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,"));
        assert!(j.contains("\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\""));
        assert!(j.contains("\"fields\":{\"subtotal_cents\":\"500\"}"));
    }
}
