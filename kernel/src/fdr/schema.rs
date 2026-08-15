//! `fdr/schema.rs` — std shim over the pure no_std core.
//!
//! The envelope types ([`FdrEvent`], [`HwStamp`], [`Reading`], [`Absence`], [`Kind`],
//! [`WorkloadKind`], [`Work`]) and the deterministic NDJSON serializer (`to_json`) live in
//! `dowiz_core::fdr::schema` and are re-exported here. This shim adds ONLY the std
//! constructors: sampling real `/proc`/`/sys` hardware ([`hw_sample`]), stamping a record
//! with real clocks ([`fdr_event_stamp`]), and the RAPL reader ([`read_joules_uj`]).

pub use dowiz_core::fdr::schema::*;

use dowiz_core::fdr::Level;

/// utime+stime ticks, via the reused `typed_metrics` reader. `None` (non-Linux/unreadable)
/// ⇒ `NonLinuxHost`.
fn read_cpu_ticks() -> Reading<u64> {
    match crate::typed_metrics::proc_cpu_sample_from_proc_self() {
        Some(s) => Reading::Value(s.utime_ticks + s.stime_ticks),
        None => Reading::Unavailable(Absence::NonLinuxHost),
    }
}

/// VmRSS kB, via the reused `typed_metrics` reader.
fn read_rss_kb() -> Reading<u64> {
    match crate::typed_metrics::mem_sample_from_proc_self() {
        Some(m) => Reading::Value(m.vm_rss_kb),
        None => Reading::Unavailable(Absence::NonLinuxHost),
    }
}

/// RAPL energy counter (µJ) from `intel-rapl:0`. Degrades to a *named* absence on every
/// failure mode; never fabricates a `0`.
pub fn read_joules_uj() -> Reading<u64> {
    #[cfg(not(target_os = "linux"))]
    {
        Reading::Unavailable(Absence::NonLinuxHost)
    }
    #[cfg(target_os = "linux")]
    {
        const PATH: &str = "/sys/class/powercap/intel-rapl:0/energy_uj";
        match crate::vfs::read_to_string(PATH) {
            Ok(s) => match s.trim().parse::<u64>() {
                Ok(v) => Reading::Value(v),
                Err(_) => Reading::Unavailable(Absence::ReadError),
            },
            Err(e) => match e {
                crate::vfs::VfsError::NotFound => Reading::Unavailable(Absence::NoRaplInterface),
                crate::vfs::VfsError::PermissionDenied => {
                    Reading::Unavailable(Absence::PermissionDenied)
                }
                _ => Reading::Unavailable(Absence::ReadError),
            },
        }
    }
}

/// Sample the hardware stamp under `policy`. `Cheap` is the pure [`HwStamp::cheap`];
/// `Full` reads `/proc`+`/sys` (µs-scale syscalls) for alarm-class records.
pub fn hw_sample(policy: StampPolicy) -> HwStamp {
    match policy {
        StampPolicy::Cheap => HwStamp::cheap(),
        StampPolicy::Full => HwStamp {
            cpu_ticks: read_cpu_ticks(),
            rss_kb: read_rss_kb(),
            joules_uj: read_joules_uj(),
        },
    }
}

/// Build a record and stamp `ts`/`mono`/`hw`. Non-wasm: `SystemTime::now()` and
/// `mono_now_ns()`; the FDR write path is never reached on wasm (no sink is installed there).
#[cfg(not(target_arch = "wasm32"))]
pub fn fdr_event_stamp(
    seq: u64,
    level: Level,
    kind: Kind,
    name: String,
    hw_policy: StampPolicy,
    fields: Vec<(&'static str, String)>,
) -> FdrEvent {
    let ts_unix_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mono_ns = crate::typed_metrics::mono_now_ns();
    FdrEvent::stamp_with(
        seq,
        level,
        kind,
        name,
        hw_sample(hw_policy),
        ts_unix_ns,
        mono_ns,
        fields,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapl_absent_host_reports_named_absence_not_missing_key() {
        let r = read_joules_uj();
        assert!(
            r.is_unavailable(),
            "expected RAPL-less host to report Unavailable"
        );
        let w = r.write_field(dowiz_core::fdr::json::JsonWriter::obj(), "joules_uj").finish();
        assert!(
            w.contains("\"joules_uj\":{\"unavailable\":"),
            "field must be present: {w}"
        );
        assert!(w.contains("unavailable"), "reason must be greppable: {w}");
    }

    #[test]
    fn hw_sample_cheap_equals_pure_cheap() {
        assert_eq!(hw_sample(StampPolicy::Cheap), HwStamp::cheap());
        // Cheap stamp is all SamplingDisabled.
        let c = HwStamp::cheap();
        assert_eq!(c.cpu_ticks, Reading::Unavailable(Absence::SamplingDisabled));
        assert_eq!(c.rss_kb, Reading::Unavailable(Absence::SamplingDisabled));
        assert_eq!(c.joules_uj, Reading::Unavailable(Absence::SamplingDisabled));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn fdr_event_stamp_stamps_real_monotonic_and_wall_clocks() {
        let ev = fdr_event_stamp(
            1,
            Level::Info,
            Kind::Event,
            "test".into(),
            StampPolicy::Cheap,
            vec![("k", "v".into())],
        );
        assert!(ev.ts_unix_ns > 0, "wall-clock must be non-zero: {}", ev.ts_unix_ns);
        assert!(ev.mono_ns >= 0, "mono must be stamped");
        assert_eq!(ev.kind, Kind::Event);
        // Deterministic serialization still holds after stamping.
        let j = ev.to_json();
        assert!(j.contains("\"seq\":1"), "seq present: {j}");
        assert!(j.contains("\"name\":\"test\""), "name present: {j}");
    }

    // ── Item 48 byte-identity (golden): an `Event` record's JSON must be byte-identical to
    // what it would have been before the `Heartbeat` variant / item 62 / item 58 additions. ──

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
            hw: HwStamp::cheap(),
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
    fn span_close_root_serializes_no_parent_absence() {
        let ev = FdrEvent {
            seq: 10,
            ts_unix_ns: 100,
            mono_ns: 200,
            level: Level::Info,
            kind: Kind::SpanClose,
            name: "test_span".into(),
            hw: HwStamp::cheap(),
            pmu: None,
            span_id: Some(42),
            parent_span_id: Some(Reading::Unavailable(Absence::NoParent)),
            work: None,
            fields: vec![("dur_us", "150".into())],
        };
        let j = ev.to_json();
        assert!(j.contains("\"span_id\":42"), "span_id present: {j}");
        assert!(
            j.contains("\"parent_span_id\":{\"unavailable\":\"no_parent\"}"),
            "root must carry no_parent absence: {j}"
        );
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
            hw: HwStamp::cheap(),
            pmu: None,
            span_id: Some(43),
            parent_span_id: Some(Reading::Value(42)),
            work: None,
            fields: vec![("dur_us", "80".into())],
        };
        let j = ev.to_json();
        assert!(j.contains("\"span_id\":43"), "child span_id present: {j}");
        assert!(j.contains("\"parent_span_id\":42"), "child parent is a bare value: {j}");
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
            hw: HwStamp::cheap(),
            pmu: None,
            span_id: Some(99),
            parent_span_id: Some(Reading::Unavailable(Absence::NoParent)),
            work: Some(Work {
                kind: WorkloadKind::FdrRecordsAppended,
                delta_count: 42,
            }),
            fields: vec![("dur_us", "300".into())],
        };
        let j = ev.to_json();
        assert!(
            j.contains("\"work\":{\"kind\":\"fdr_records_appended\",\"delta_count\":42}"),
            "work field serialized when Some: {j}"
        );
        let work_pos = j.find("\"work\"").unwrap();
        let fields_pos = j.find("\"fields\"").unwrap();
        assert!(work_pos < fields_pos, "work must precede fields: {j}");
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
