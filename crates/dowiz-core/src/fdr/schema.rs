//! `fdr/schema.rs` — the FDR event envelope (pure no_std core).
//!
//! Every FDR record carries a fixed envelope whose `hw` field is a NON-optional struct,
//! so schema-level omission of the hardware stamp is unrepresentable. Each hardware
//! reading is a [`Reading<T>`] = `Value(T) | Unavailable(Absence)` with a CLOSED reason
//! enum, and the field is ALWAYS serialized — never silently dropped.
//!
//! This is the *pure* half: the envelope types + the deterministic NDJSON serializer.
//! The std side — stamping a record with real clocks (`SystemTime`/monotonic) and sampling
//! `/proc`/`/sys` hardware (`HwStamp::sample`) — lives in the kernel shim
//! (`dowiz-kernel`'s `fdr::schema::{fdr_event_stamp, hw_sample, read_joules_uj}`), which
//! re-exports these types and adds the std constructors as free functions.

use crate::fdr::json::JsonWriter;
use crate::fdr::Level;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

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
    /// Stamping was intentionally skipped for this record class (cost control).
    SamplingDisabled,
    /// No usable PMU counter interface for this reading.
    NoPmuInterface,
    /// Item 69: the operator-supplied regional grid constant is absent.
    NoRegionalConstant,
    /// Item 69 (on-site water): a PERMANENT named absence under every input.
    NotSoftwareObservable,
    /// Item 62: this record is a root of a span tree — it has no causal parent.
    NoParent,
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
            Absence::NoPmuInterface => "no_pmu_interface",
            Absence::NoRegionalConstant => "no_regional_constant",
            Absence::NotSoftwareObservable => "not_software_observable",
            Absence::NoParent => "no_parent",
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
    /// `"key":{"unavailable":"<reason>"}`. Either way the key is emitted.
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
    /// utime+stime ticks (`/proc/self/stat`).
    pub cpu_ticks: Reading<u64>,
    /// VmRSS kB (`/proc/self/status`).
    pub rss_kb: Reading<u64>,
    /// RAPL energy counter in µJ (`/sys/class/powercap/intel-rapl:0/energy_uj`).
    pub joules_uj: Reading<u64>,
}

/// Stamp cost policy (blueprint §4.2 "honest cost control"). `Full` reads `/proc`+`/sys`;
/// `Cheap` records a first-class `SamplingDisabled` instead of taxing hot paths.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StampPolicy {
    Full,
    Cheap,
}

impl HwStamp {
    /// The `Cheap` stamp: all three fields are a first-class `SamplingDisabled` (no I/O).
    /// This is the pure half of `HwStamp::sample`; the `Full` half (real `/proc`/`/sys`
    /// reads) lives in the kernel shim (`hw_sample`).
    pub fn cheap() -> Self {
        HwStamp {
            cpu_ticks: Reading::Unavailable(Absence::SamplingDisabled),
            rss_kb: Reading::Unavailable(Absence::SamplingDisabled),
            joules_uj: Reading::Unavailable(Absence::SamplingDisabled),
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

/// Record kind (closed enum). `Tuning` is reserved for item-21's FDR-logged adjustments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Event,
    SpanClose,
    Alarm,
    PostMortem,
    Tuning,
    /// Written on orderly shutdown — its presence marks a clean stop.
    CleanShutdown,
    /// Item 48 (closure b): periodic liveness heartbeat.
    Heartbeat,
    /// Item 51 (shadow-mode divergence telemetry): ADVISORY, non-gating record.
    ShadowDivergence,
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
            Kind::Heartbeat => "heartbeat",
            Kind::ShadowDivergence => "shadow_divergence",
        }
    }
}

/// Item 58: the closed set of workload kinds — the *what* a span produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkloadKind {
    DecisionUnitsImported,
    FdrRecordsAppended,
    TransitionsFolded,
    TokensGenerated,
    FramesRendered,
    EigensolvesCompleted,
    SignaturesVerified,
}

impl WorkloadKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkloadKind::DecisionUnitsImported => "decision_units_imported",
            WorkloadKind::FdrRecordsAppended => "fdr_records_appended",
            WorkloadKind::TransitionsFolded => "transitions_folded",
            WorkloadKind::TokensGenerated => "tokens_generated",
            WorkloadKind::FramesRendered => "frames_rendered",
            WorkloadKind::EigensolvesCompleted => "eigensolves_completed",
            WorkloadKind::SignaturesVerified => "signatures_verified",
        }
    }

    pub fn from_str(s: &str) -> Option<WorkloadKind> {
        match s {
            "decision_units_imported" => Some(WorkloadKind::DecisionUnitsImported),
            "fdr_records_appended" => Some(WorkloadKind::FdrRecordsAppended),
            "transitions_folded" => Some(WorkloadKind::TransitionsFolded),
            "tokens_generated" => Some(WorkloadKind::TokensGenerated),
            "frames_rendered" => Some(WorkloadKind::FramesRendered),
            "eigensolves_completed" => Some(WorkloadKind::EigensolvesCompleted),
            "signatures_verified" => Some(WorkloadKind::SignaturesVerified),
            _ => None,
        }
    }
}

/// Item 58: a workload counter — the *how much* a span produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Work {
    pub kind: WorkloadKind,
    pub delta_count: u64,
}

impl Work {
    pub fn write(self, w: JsonWriter) -> JsonWriter {
        let mut inner = JsonWriter::obj();
        inner = inner.field_str("kind", self.kind.as_str());
        inner = inner.field_u64("delta_count", self.delta_count);
        w.field_raw("work", &inner.finish())
    }
}

/// One FDR record. Fixed envelope; `hw` is first-class (never `Option`).
#[derive(Clone, Debug)]
pub struct FdrEvent {
    /// Monotonic per-process sequence number (the recovery ordering key).
    pub seq: u64,
    /// Wall-clock ns (forensic/display plane; NOT the replay-ordering key).
    pub ts_unix_ns: u128,
    /// Monotonic ns since process start.
    pub mono_ns: u128,
    pub level: Level,
    pub kind: Kind,
    pub name: String,
    pub hw: HwStamp,
    /// Optional per-classification-window PMU companion stamp (roadmap item 27).
    pub pmu: Option<crate::fdr::pmu::PmuStamp>,
    /// Item 62: per-process monotone span id.
    pub span_id: Option<u64>,
    /// Item 62: causal parent of this span.
    pub parent_span_id: Option<Reading<u64>>,
    /// Item 58: workload counter.
    pub work: Option<Work>,
    pub fields: Vec<(&'static str, String)>,
}

impl FdrEvent {
    /// Pure constructor: build a record with caller-supplied `hw`, wall-clock `ts_unix_ns`,
    /// and monotonic `mono_ns` (the no_std form). The kernel shim's `fdr_event_stamp`
    /// fills these in from `SystemTime::now()` / `mono_now_ns()` / `HwStamp::sample`.
    pub fn stamp_with(
        seq: u64,
        level: Level,
        kind: Kind,
        name: String,
        hw: HwStamp,
        ts_unix_ns: u128,
        mono_ns: u128,
        fields: Vec<(&'static str, String)>,
    ) -> Self {
        FdrEvent {
            seq,
            ts_unix_ns,
            mono_ns,
            level,
            kind,
            name,
            hw,
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
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
        let w = match self.pmu {
            Some(p) => p.write(w),
            None => w,
        };
        let w = match self.span_id {
            Some(id) => w.field_u64("span_id", id),
            None => w,
        };
        let w = match self.parent_span_id {
            Some(reading) => reading.write_field(w, "parent_span_id"),
            None => w,
        };
        let w = match self.work {
            Some(work) => work.write(w),
            None => w,
        };
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
    fn reading_value_serializes_as_bare_number() {
        let s = Reading::Value(12345u64)
            .write_field(JsonWriter::obj(), "joules_uj")
            .finish();
        assert_eq!(s, "{\"joules_uj\":12345}");
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
    fn event_roundtrips_to_deterministic_json() {
        let ev = FdrEvent::stamp_with(
            7,
            Level::Info,
            Kind::Event,
            "place_order".into(),
            HwStamp::cheap(),
            1,
            2,
            vec![("subtotal_cents", "500".into())],
        );
        let j = ev.to_json();
        assert!(j.starts_with("{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,"));
        assert!(j.contains("\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\""));
        assert!(j.contains("\"fields\":{\"subtotal_cents\":\"500\"}"));
        assert!(!j.contains("\"span_id\""), "non-span must not carry span_id: {j}");
        assert!(
            !j.contains("\"parent_span_id\""),
            "non-span must not carry parent_span_id: {j}"
        );
    }

    #[test]
    fn workload_kind_serialization() {
        assert_eq!(
            WorkloadKind::DecisionUnitsImported.as_str(),
            "decision_units_imported"
        );
        assert_eq!(WorkloadKind::FdrRecordsAppended.as_str(), "fdr_records_appended");
        assert_eq!(WorkloadKind::TransitionsFolded.as_str(), "transitions_folded");
        assert_eq!(WorkloadKind::TokensGenerated.as_str(), "tokens_generated");
        assert_eq!(WorkloadKind::FramesRendered.as_str(), "frames_rendered");
        assert_eq!(WorkloadKind::EigensolvesCompleted.as_str(), "eigensolves_completed");
        assert_eq!(WorkloadKind::SignaturesVerified.as_str(), "signatures_verified");
    }
}
