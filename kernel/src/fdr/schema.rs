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
    /// No usable PMU counter interface for this reading: `perf_event_open(2)` reported the
    /// hardware/interface itself absent (ENOENT/ENODEV/EOPNOTSUPP), the syscall was filtered
    /// (ENOSYS), or the build target has no zero-dep counter path. Distinct from
    /// [`Absence::PermissionDenied`], which is `perf_event_paranoid`/`CAP_PERFMON` gating an
    /// interface that DOES exist. Used by [`super::pmu::PmuStamp`] (roadmap item 27).
    NoPmuInterface,
    /// Item 69 (water/carbon as derived views of joules): the operator-supplied regional
    /// grid constant (gCO₂e/kWh or L/kWh) is absent, so the derived footprint cannot be
    /// computed. Never a fabricated constant — the kernel ships with the constant absent.
    NoRegionalConstant,
    /// Item 69 (on-site water): facility cooling is not software-observable by a local
    /// device, so on-site water is a PERMANENT named absence under every input. This variant
    /// names that invariant (the derivation module NEVER produces an on-site-water value).
    NotSoftwareObservable,
    /// Item 62 (FDR relational linkage): this record is a root of a span tree — it has no
    /// causal parent. Serialized as `"no_parent"`, greppable; never a magic `0` or a missing
    /// key.
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
    /// Item 48 (FDR blind-spot closure, closure b). Periodic liveness heartbeat emitted by
    /// the HOST loop — the kernel provides ONLY the record type + emit fn (`emit_heartbeat`);
    /// cadence and JUDGMENT live in the deployment layer (systemd `WatchdogSec` /
    /// `hub_supervisor`). An external liveness check converts a *missed* heartbeat into the
    /// already-survivable kill-9 crash class. Additive variant: every other record
    /// serializes byte-identically (item-27 optional-field discipline).
    Heartbeat,
    /// Item 51 (shadow-mode divergence telemetry). ADVISORY, non-gating record: on every
    /// decision where AI advice was present, logs whether the proposed action AGREED with
    /// the kernel's deterministic decision D — digests only, never full payloads. The variant
    /// is write-only from the kernel's perspective (no code path reads it to change a
    /// decision), so the record is pure telemetry. Additive variant: every non-shadow FDR
    /// record serializes byte-identically (item-27 optional-field discipline).
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

/// Item 58: the closed set of workload kinds — the *what* a span produced. Each variant
/// maps to a greppable snake_case string. `Copy` + `Clone` because workload metadata is
/// small, fixed, and carried on hot paths.
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

/// Item 58: a workload counter — the *how much* a span produced. Carried on SpanClose
/// records as `Some(Work)`; `None` on every other record class (byte-identical to pre-item-58).
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
    /// Monotonic ns since process start (`typed_metrics::mono_now_ns`).
    pub mono_ns: u128,
    pub level: Level,
    pub kind: Kind,
    pub name: String,
    pub hw: HwStamp,
    /// Optional per-classification-window PMU companion stamp (roadmap item 27). Present
    /// ONLY on verdict-emission records that bracket a `Verdict`/`DriftClass` classification
    /// (`super::pmu`); `None` — and therefore ABSENT from the serialized record, keeping all
    /// other FDR records byte-identical — everywhere else. Lives on the P3 forensic plane,
    /// excluded from every hash/signature/gate surface (see `pmu` module doc).
    pub pmu: Option<super::pmu::PmuStamp>,
    /// Item 62 (FDR relational linkage): per-process monotone span id. `Some` on span-tree
    /// records (SpanClose); `None` on non-span records. Serialized only when present —
    /// non-span records stay byte-identical to pre-item-62 (the pmu-absence precedent).
    /// Never feeds a hash, signature, idempotency, or replay surface (P3 firewall, procedure
    /// step 7).
    pub span_id: Option<u64>,
    /// Item 62 (FDR relational linkage): causal parent of this span. A root span carries
    /// `Some(Reading::Unavailable(Absence::NoParent))` — the named-absence doctrine (no magic
    /// `0`). A child carries `Some(Reading::Value(parent_span_id))`. `None` on non-span
    /// records (byte-identical to pre-item-62). Serialized only when `Some`, following the
    /// pmu optional-field discipline.
    pub parent_span_id: Option<Reading<u64>>,
    /// Item 58: workload counter — what the span produced and how much. Present ONLY on
    /// SpanClose records; `None` everywhere else. Serialized only when `Some`, following the
    /// pmu optional-field discipline. Byte-identical to pre-item-58 when absent.
    pub work: Option<Work>,
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
        // `pmu` rides alongside `hw` ONLY on verdict-emission records; absent otherwise, so
        // every other FDR record serializes byte-identically to before item 27.
        let w = match self.pmu {
            Some(p) => p.write(w),
            None => w,
        };
        // Item 62 (FDR relational linkage): span_id and parent_span_id ride on span-tree records
        // (SpanClose). Non-span records have `None` for both — serialized fields absent,
        // byte-identical to pre-item-62 (the pmu-absence precedent). On span records, span_id
        // is always present; parent_span_id is a `Reading` — root spans serialize as
        // `"parent_span_id":{"unavailable":"no_parent"}`, children as `"parent_span_id":<u64>`.
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
    fn rapl_absent_host_reports_named_absence_not_missing_key() {
        // This host has no RAPL interface (blueprint §3.2 / verified empty
        // /sys/class/powercap). The field must be PRESENT with a named reason.
        let r = read_joules_uj();
        assert!(
            r.is_unavailable(),
            "expected RAPL-less host to report Unavailable"
        );
        let w = r.write_field(JsonWriter::obj(), "joules_uj").finish();
        // Greppable named absence — key present, reason spelled out (§G.9 proof).
        assert!(
            w.contains("\"joules_uj\":{\"unavailable\":"),
            "field must be present: {w}"
        );
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
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
            fields: vec![("subtotal_cents", "500".into())],
        };
        let j = ev.to_json();
        assert!(j.starts_with("{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,"));
        assert!(j.contains("\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\""));
        assert!(j.contains("\"fields\":{\"subtotal_cents\":\"500\"}"));
        // Item 62: non-span records must NOT contain span_id or parent_span_id.
        assert!(!j.contains("\"span_id\""), "non-span must not carry span_id: {j}");
        assert!(!j.contains("\"parent_span_id\""), "non-span must not carry parent_span_id: {j}");
    }

    // Item 48 (closure b): the `Heartbeat` variant serializes to a byte-stable record and is
    // additive — every non-heartbeat record is unaffected (the item-27 byte-identity proof
    // is asserted separately in `fdr::mod` against an `Event` of identical shape).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn heartbeat_record_serializes_with_heartbeat_kind_and_progress() {
        let ev = FdrEvent {
            seq: 3,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: Level::Info,
            kind: Kind::Heartbeat,
            name: "heartbeat".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
            fields: vec![("tick", "3".into())],
        };
        let j = ev.to_json();
        assert!(
            j.contains("\"kind\":\"heartbeat\""),
            "must serialize as heartbeat: {j}"
        );
        assert!(
            j.contains("\"name\":\"heartbeat\""),
            "name carries heartbeat: {j}"
        );
        assert!(
            j.contains("\"fields\":{\"tick\":\"3\"}"),
            "progress counters present: {j}"
        );
        // Stable, deterministic envelope order — the same shape an `Event` would take.
        assert!(j.starts_with("{\"seq\":3,\"ts_unix_ns\":1,\"mono_ns\":2,"));
    }

    // Item 48 byte-identity (item 27): an `Event` record's JSON must be byte-identical to
    // what it would have been before the `Heartbeat` variant was added. The `Kind` enum grew
    // but `Kind::as_str(Kind::Event)` and the `to_json` field order are unchanged, so the
    // serialized bytes are identical. We pin the exact string.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn event_byte_identity_preserved_after_heartbeat_variant_added() {
        let ev = FdrEvent {
            seq: 7,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: Level::Info,
            kind: Kind::Event,
            name: "place_order".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
            fields: vec![("subtotal_cents", "500".into())],
        };
        // Captured golden string — MUST NOT change after the Heartbeat variant is added.
        assert_eq!(
            ev.to_json(),
            "{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\",\"hw\":{\"cpu_ticks\":{\"unavailable\":\"sampling_disabled\"},\"rss_kb\":{\"unavailable\":\"sampling_disabled\"},\"joules_uj\":{\"unavailable\":\"sampling_disabled\"}},\"fields\":{\"subtotal_cents\":\"500\"}}"
        );
    }

    // ── Item 62: FDR relational linkage tests ──────────────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn span_close_root_serializes_no_parent_absence() {
        let ev = FdrEvent {
            seq: 10,
            ts_unix_ns: 100,
            mono_ns: 200,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "test_span".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: Some(42),
            parent_span_id: Some(Reading::Unavailable(Absence::NoParent)),
            work: None,
            fields: vec![("dur_us", "150".into())],
        };
        let j = ev.to_json();
        assert!(
            j.contains("\"span_id\":42"),
            "span_id must be present on span records: {j}"
        );
        assert!(
            j.contains("\"parent_span_id\":{\"unavailable\":\"no_parent\"}"),
            "root must carry no_parent named absence: {j}"
        );
        // Field order: span_id before parent_span_id, both before fields.
        let sid_pos = j.find("\"span_id\"").unwrap();
        let pid_pos = j.find("\"parent_span_id\"").unwrap();
        let fields_pos = j.find("\"fields\"").unwrap();
        assert!(sid_pos < pid_pos, "span_id must precede parent_span_id");
        assert!(pid_pos < fields_pos, "parent_span_id must precede fields");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn span_close_child_serializes_parent_span_id_as_value() {
        let ev = FdrEvent {
            seq: 11,
            ts_unix_ns: 100,
            mono_ns: 200,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "child_span".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: Some(43),
            parent_span_id: Some(Reading::Value(42)),
            work: None,
            fields: vec![("dur_us", "80".into())],
        };
        let j = ev.to_json();
        assert!(
            j.contains("\"span_id\":43"),
            "child span_id must be present: {j}"
        );
        assert!(
            j.contains("\"parent_span_id\":42"),
            "child parent must be a bare value: {j}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn event_record_byte_identity_preserved_after_item_62() {
        let ev = FdrEvent {
            seq: 7,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: Level::Info,
            kind: Kind::Event,
            name: "place_order".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
            fields: vec![("subtotal_cents", "500".into())],
        };
        // Golden string is UNCHANGED from the pre-item-62 test — no span fields present.
        assert_eq!(
            ev.to_json(),
            "{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\",\"hw\":{\"cpu_ticks\":{\"unavailable\":\"sampling_disabled\"},\"rss_kb\":{\"unavailable\":\"sampling_disabled\"},\"joules_uj\":{\"unavailable\":\"sampling_disabled\"}},\"fields\":{\"subtotal_cents\":\"500\"}}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn span_tree_reconstruction_from_recovered_ring() {
        let root = FdrEvent {
            seq: 1,
            ts_unix_ns: 1000,
            mono_ns: 2000,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "root_span".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: Some(100),
            parent_span_id: Some(Reading::Unavailable(Absence::NoParent)),
            work: None,
            fields: vec![("dur_us", "500".into())],
        };
        let child = FdrEvent {
            seq: 2,
            ts_unix_ns: 1100,
            mono_ns: 2100,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "child_span".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: Some(101),
            parent_span_id: Some(Reading::Value(100)),
            work: None,
            fields: vec![("dur_us", "200".into())],
        };
        // Simulate "recovered ring" — a list of serialized records.
        let ring = vec![root.to_json(), child.to_json()];
        // Reconstruct: parse span_id + parent_span_id from each record.
        let mut by_id: std::collections::HashMap<u64, (String, Option<u64>)> =
            std::collections::HashMap::new();
        for line in &ring {
            let sid = parse_field_u64(line, "span_id").expect("span_id must be present");
            let pid = parse_parent_span_id(line);
            let name = parse_field_str(line, "name")
                .expect("name must be present")
                .to_string();
            by_id.insert(sid, (name, pid));
        }
        // Walk the tree: root has no parent, child has parent = root's span_id.
        let root_entry = by_id.get(&100).expect("root must exist");
        assert!(
            root_entry.1.is_none(),
            "root must have no parent (NoParent absence)"
        );
        let child_entry = by_id.get(&101).expect("child must exist");
        assert_eq!(
            child_entry.1,
            Some(100),
            "child's parent must be root (100)"
        );
        // Tree shape: root is parent of child.
        let children_of_root: Vec<_> = by_id
            .iter()
            .filter(|(_, (_, pid))| *pid == Some(100))
            .map(|(sid, (name, _))| (*sid, name.clone()))
            .collect();
        assert_eq!(children_of_root.len(), 1, "root must have exactly 1 child");
        assert_eq!(children_of_root[0].0, 101);
        assert_eq!(children_of_root[0].1, "child_span");
    }

    // ── Item 62 test helpers ───────────────────────────────────────────────────────

    /// Parse a `"key":<number>` field from a JSON line (minimal, no serde).
    fn parse_field_u64(json: &str, key: &str) -> Option<u64> {
        let pattern = format!("\"{key}\":");
        let pos = json.find(&pattern)?;
        let rest = &json[pos + pattern.len()..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        rest[..end].parse().ok()
    }

    /// Parse `"name":"<string>"` from a JSON line.
    fn parse_field_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
        let pattern = format!("\"{key}\":\"");
        let pos = json.find(&pattern)?;
        let start = pos + pattern.len();
        let end = json[start..].find('"')?;
        Some(&json[start..start + end])
    }

    /// Parse `parent_span_id` — returns `Some(u64)` for a bare value,
    /// `None` for `{"unavailable":"no_parent"}`.
    fn parse_parent_span_id(json: &str) -> Option<u64> {
        let pattern = "\"parent_span_id\":";
        let pos = json.find(pattern)?;
        let rest = &json[pos + pattern.len()..];
        if rest.starts_with('{') {
            None
        } else {
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            Some(rest[..end].parse().ok()?)
        }
    }

    // ── Item 58: WorkloadKind / Work tests ───────────────────────────────────────────

    #[test]
    fn workload_kind_serialization() {
        assert_eq!(WorkloadKind::DecisionUnitsImported.as_str(), "decision_units_imported");
        assert_eq!(WorkloadKind::FdrRecordsAppended.as_str(), "fdr_records_appended");
        assert_eq!(WorkloadKind::TransitionsFolded.as_str(), "transitions_folded");
        assert_eq!(WorkloadKind::TokensGenerated.as_str(), "tokens_generated");
        assert_eq!(WorkloadKind::FramesRendered.as_str(), "frames_rendered");
        assert_eq!(WorkloadKind::EigensolvesCompleted.as_str(), "eigensolves_completed");
        assert_eq!(WorkloadKind::SignaturesVerified.as_str(), "signatures_verified");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn span_close_with_work_serializes_work() {
        let ev = FdrEvent {
            seq: 20,
            ts_unix_ns: 200,
            mono_ns: 300,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "fdr_flush".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: Some(99),
            parent_span_id: Some(Reading::Unavailable(Absence::NoParent)),
            work: Some(Work { kind: WorkloadKind::FdrRecordsAppended, delta_count: 42 }),
            fields: vec![("dur_us", "300".into())],
        };
        let j = ev.to_json();
        assert!(
            j.contains("\"work\":{\"kind\":\"fdr_records_appended\",\"delta_count\":42}"),
            "work field must be serialized when Some: {j}"
        );
        let work_pos = j.find("\"work\"").unwrap();
        let fields_pos = j.find("\"fields\"").unwrap();
        assert!(work_pos < fields_pos, "work must precede fields: {j}");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn event_without_work_byte_identity() {
        let ev = FdrEvent {
            seq: 7,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: Level::Info,
            kind: Kind::Event,
            name: "place_order".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
            fields: vec![("subtotal_cents", "500".into())],
        };
        assert_eq!(
            ev.to_json(),
            "{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\",\"hw\":{\"cpu_ticks\":{\"unavailable\":\"sampling_disabled\"},\"rss_kb\":{\"unavailable\":\"sampling_disabled\"},\"joules_uj\":{\"unavailable\":\"sampling_disabled\"}},\"fields\":{\"subtotal_cents\":\"500\"}}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn work_field_position_is_before_fields() {
        let ev = FdrEvent {
            seq: 21,
            ts_unix_ns: 200,
            mono_ns: 300,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "eigen".into(),
            hw: HwStamp::sample(StampPolicy::Cheap),
            pmu: None,
            span_id: Some(10),
            parent_span_id: Some(Reading::Value(9)),
            work: Some(Work { kind: WorkloadKind::EigensolvesCompleted, delta_count: 7 }),
            fields: vec![("dur_us", "100".into())],
        };
        let j = ev.to_json();
        let span_pos = j.find("\"span_id\"").unwrap();
        let pid_pos = j.find("\"parent_span_id\"").unwrap();
        let work_pos = j.find("\"work\"").unwrap();
        let fields_pos = j.find("\"fields\"").unwrap();
        assert!(span_pos < pid_pos, "span_id before parent_span_id");
        assert!(pid_pos < work_pos, "parent_span_id before work");
        assert!(work_pos < fields_pos, "work before fields");
    }

    #[test]
    fn workload_kind_roundtrip() {
        for s in &[
            "decision_units_imported",
            "fdr_records_appended",
            "transitions_folded",
            "tokens_generated",
            "frames_rendered",
            "eigensolves_completed",
            "signatures_verified",
        ] {
            let wk = WorkloadKind::from_str(s).expect(s);
            assert_eq!(wk.as_str(), *s);
        }
        assert!(WorkloadKind::from_str("unknown").is_none());
    }
}
