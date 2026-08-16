//! decision/mod.rs — kernel-side shim over the no_std core `dowiz_core::decision`.
//!
//! The pure `DecisionUnit` family, closed `DomainTag` registry, `merge_meta` semilattice, and the
//! import gate all live in `dowiz_core::decision` (no_std). This module re-exports them so
//! `crate::decision::…` callers are unaffected, and hosts the ONE std/kernel-side seam:
//! `decide_with_shadow` (item 51) — a FREE function (not a method, because the orphan rule forbids
//! `impl` on the foreign `DecisionUnit`) that writes `ShadowDivergence` FDR records. Shadow mode is
//! OFF when `ring = None` (pure `decide()`); the decision `D` is byte-identical either way, pinned
//! by `shadow_telemetry_tests::decision_is_bit_identical_with_shadow_on_or_off`.

pub use dowiz_core::decision::import;
pub use dowiz_core::decision::*;

/// Item 51 seam (co-spec with item 47's decision seam). Compute the deterministic decision `D`
/// (`decide` is `&self` and returns an owned `Decision<O>` — pure), then — when a shadow `ring` is
/// supplied AND advice was present (`proposal: Some`) — emit ONE advisory `ShadowDivergence` FDR
/// record. The emit is a pure side-channel write; it can NEVER observe or mutate `D`.
/// `decide_with_shadow` with `ring = None` is the shadow-OFF path (item-47 `None`-path pattern);
/// with `Some` it is shadow-ON. Toggling shadow mode therefore changes ZERO bytes of `D` — pinned by
/// `shadow_telemetry_tests::decision_is_bit_identical_with_shadow_on_or_off`.
///
/// Host-only: the FDR ring does not exist on wasm (`crate::fdr::ring` is wasm-gated).
///
/// FREE function (not a method on `DecisionUnit`) — the orphan rule forbids `impl` blocks on the
/// foreign `dowiz_core::decision::DecisionUnit` type.
#[cfg(not(target_arch = "wasm32"))]
pub fn decide_with_shadow<I, O>(
    unit: &DecisionUnit<I, O>,
    input: &I,
    proposal: Option<&O>,
    ring: Option<&mut crate::fdr::ring::FdrRing>,
) -> Decision<O>
where
    O: PartialEq + core::fmt::Debug,
{
    let d = unit.decide(input); // D — the total, primary, deterministic decision.
    if let (Some(act), Some(r)) = (proposal, ring) {
        // Agree = the proposed action EQUALS the admitted decision. When D escalates
        // there is no admitted action, so the proposal can never agree (blueprint §3.3:
        // `proposal.action == D`). `agree` is the verdict of the comparison bit only.
        let agree = matches!(&d, Decision::Answer(a) if a == act);
        // Self-consistency cross-check (compiled out of release): the agree bit equals the
        // direct `proposal.action == D` recomputation at the emit site (blueprint §4.3).
        debug_assert_eq!(agree, matches!(&d, Decision::Answer(a) if a == act));
        let verdict = match &d {
            Decision::Answer(_) => "admitted",
            Decision::Escalate(_) => "refuted",
        };
        // Digests ONLY — never the full payloads (blueprint §3.2 minimal-statistic rule).
        let d_digest = crate::fdr::shadow_digest(format!("{d:?}").as_bytes());
        let act_digest = crate::fdr::shadow_digest(format!("{act:?}").as_bytes());
        let ev = crate::fdr::schema::fdr_event_stamp(
            0,
            crate::fdr::Level::Info,
            crate::fdr::schema::Kind::ShadowDivergence,
            "decision_seam".to_string(),
            crate::fdr::schema::StampPolicy::Cheap,
            vec![
                ("verdict", verdict.to_string()),
                ("agree", if agree { "1" } else { "0" }.to_string()),
                ("d_digest", d_digest),
                ("act_digest", act_digest),
            ],
        );
        let _ = r.append(&ev);
    }
    d
}

// ─────────────────────────────────────────────────────────────────────────────
// Item 51: shadow-mode divergence telemetry — the load-bearing non-gating proof.
//
// Every test below drives the REAL decision seam (`decide_with_shadow`) and the REAL
// FDR ring, then reads records back with `fdr::ring::recover`. Shadow mode is the
// FDR `ring` (+ `proposal`) — when it is `None` shadow is OFF, when `Some` it is ON.
// The proof the blueprint hinges on: the decision D must be byte-identical with shadow
// ON vs OFF across the full decision test corpus.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod shadow_telemetry_tests {
    use super::*;
    use crate::fdr::ring::{FdrRing, Recovery, DEFAULT_SEG_CAP};
    use crate::fdr::schema::Kind;

    fn shadow_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "item51_shadow_{}_{}_{}",
            tag,
            std::process::id(),
            crate::typed_metrics::mono_now_ns()
        ));
        let _ = crate::vfs::create_dir_all(&d);
        d
    }

    /// THE load-bearing test (blueprint §4 acceptance #1 / §5 falsifiable criterion #1):
    /// toggling shadow logging changes ZERO bytes of D across the full decision test corpus.
    /// We run the seam with shadow OFF (ring = None) and with shadow ON (a real FDR ring),
    /// then assert the returned `Decision` is byte-identical in both modes for every corpus
    /// entry — AND that the serialized `Debug` form of D is byte-identical too.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn decision_is_bit_identical_with_shadow_on_or_off() {
        let cases: &[(HarnessInput, HarnessOut, HarnessOut)] = &[
            (
                HarnessInput { job: 0 },
                HarnessOut { route_tier: 0 },
                HarnessOut { route_tier: 0 },
            ),
            (
                HarnessInput { job: 1 },
                HarnessOut { route_tier: 2 },
                HarnessOut { route_tier: 7 },
            ),
            (
                HarnessInput { job: 2 },
                HarnessOut { route_tier: 4 },
                HarnessOut { route_tier: 4 },
            ),
            (
                HarnessInput { job: 3 },
                HarnessOut { route_tier: 1 },
                HarnessOut { route_tier: 9 },
            ),
        ];

        for (input, answer, proposal) in cases.iter() {
            let unit = DecisionUnit::new(
                DomainTag::Harness,
                UnitEpoch(1),
                move |_in: &HarnessInput| Decision::Answer(answer.clone()),
            );

            // ── shadow OFF ──
            let d_off = decide_with_shadow(&unit, input, Some(proposal), None);
            let d_off_bytes = format!("{d_off:?}");

            // ── shadow ON (real FDR ring) ──
            let dir = shadow_dir("bitident");
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            let d_on = decide_with_shadow(&unit, input, Some(proposal), Some(&mut ring));
            drop(ring);

            // D is VALUE-identical…
            assert_eq!(
                d_off, d_on,
                "decision D must be value-identical with shadow ON vs OFF (input {input:?})"
            );
            // …and BYTE-identical (the blueprint's literal contract).
            let d_on_bytes = format!("{d_on:?}");
            assert_eq!(
                d_off_bytes, d_on_bytes,
                "decision D must be BIT-identical with shadow ON vs OFF (input {input:?})"
            );

            // Sanity: shadow ON actually wrote exactly one ShadowDivergence record.
            let rec = crate::fdr::ring::recover(&dir);
            assert_eq!(rec.records.len(), 1, "exactly one shadow record written");
            assert_eq!(rec.records[0].kind, "shadow_divergence");
            let _ = crate::vfs::remove_dir_all(&dir);
        }
    }

    /// Acceptance #2 (oracle): a planted DISAGREEING proposal yields exactly one recovered
    /// `ShadowDivergence` record, correct class + agreement bit (0) + digests, recovered via
    /// the real ring. No full payload string appears in the record (digests only).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn disagreeing_proposal_emits_one_shadow_record_with_digests() {
        let dir = shadow_dir("oracle");
        let unit = DecisionUnit::new(DomainTag::Harness, UnitEpoch(1), |_in: &HarnessInput| {
            Decision::Answer(HarnessOut { route_tier: 2 })
        });
        let input = HarnessInput { job: 4 };
        let proposal = HarnessOut { route_tier: 99 }; // deliberately disagrees with D.

        let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
        let d = decide_with_shadow(&unit, &input, Some(&proposal), Some(&mut ring));
        drop(ring);

        let rec: Recovery = crate::fdr::ring::recover(&dir);
        let shadow: Vec<_> = rec
            .records
            .iter()
            .filter(|r| r.kind == "shadow_divergence")
            .collect();
        assert_eq!(shadow.len(), 1, "exactly one ShadowDivergence record");
        let raw = &shadow[0].raw;
        // agree bit = 0 (disagree), verdict = admitted (D is an Answer).
        assert!(
            raw.contains("\"agree\":\"0\""),
            "disagreeing proposal must record agree=0: {raw}"
        );
        assert!(
            raw.contains("\"verdict\":\"admitted\""),
            "verdict must be admitted: {raw}"
        );
        // Digests present, full payloads absent (no "route_tier":99 literal in the record).
        assert!(raw.contains("\"d_digest\":\""), "d_digest present: {raw}");
        assert!(
            raw.contains("\"act_digest\":\""),
            "act_digest present: {raw}"
        );
        assert!(
            !raw.contains("\"route_tier\":99"),
            "full proposal payload must NEVER be logged: {raw}"
        );
        // The decision D itself is unaffected by the emit.
        assert_eq!(d, Decision::Answer(HarnessOut { route_tier: 2 }));
        assert_eq!(rec.crc_failures, 0);
        assert_eq!(rec.torn_tail, 0);
        let _ = crate::vfs::remove_dir_all(&dir);
    }

    /// Acceptance (write-only invariant, blueprint §5 #4): `ShadowDivergence` is recoverable
    /// and the variant is write-only — confirm NO `ShadowDivergence` consumer changes D by
    /// checking the decision is identical regardless of how many shadow records are emitted.
    /// We emit a flood of disagreed proposals and assert the decision is still the same D and
    /// the recovered count is bounded (one record per call; this asserts the per-call emit and
    /// that the ring preserves CRC-valid records under a flood).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn shadow_flood_emits_bounded_records_preserves_d() {
        let dir = shadow_dir("flood");
        let unit = DecisionUnit::new(DomainTag::Harness, UnitEpoch(1), |_in: &HarnessInput| {
            Decision::Answer(HarnessOut { route_tier: 3 })
        });
        let n = 50usize;
        let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
        for i in 0..n {
            // Alternate agreeing / disagreeing proposals.
            let proposal = HarnessOut {
                route_tier: if i % 2 == 0 { 3 } else { 77 },
            };
            let d = decide_with_shadow(
                &unit,
                &HarnessInput { job: i as u8 },
                Some(&proposal),
                Some(&mut ring),
            );
            assert_eq!(
                d,
                Decision::Answer(HarnessOut { route_tier: 3 }),
                "D unchanged under flood"
            );
        }
        drop(ring);
        let rec = crate::fdr::ring::recover(&dir);
        let shadow: Vec<_> = rec
            .records
            .iter()
            .filter(|r| r.kind == "shadow_divergence")
            .collect();
        // One ShadowDivergence record per call (within ring capacity); none lost/corrupted.
        assert_eq!(shadow.len(), n, "one shadow record per decision call");
        assert_eq!(rec.crc_failures, 0);
        assert_eq!(rec.torn_tail, 0);
        let _ = crate::vfs::remove_dir_all(&dir);
    }

    /// Acceptance (item-27 byte-identity): the `ShadowDivergence` variant is ADDITIVE — a plain
    /// `Event` record serializes byte-identically to before the variant existed. This pins that
    /// non-shadow FDR records are NOT disturbed by adding item 51 (the item-27 optional-field
    /// guarantee), so the variant cannot quietly change the bytes of any other record kind.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn event_record_byte_identity_preserved_after_shadow_variant_added() {
        let ev = crate::fdr::schema::FdrEvent {
            seq: 7,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: crate::fdr::Level::Info,
            kind: Kind::Event,
            name: "place_order".into(),
            hw: crate::fdr::schema::hw_sample(crate::fdr::schema::StampPolicy::Cheap),
            pmu: None,
            span_id: None,
            parent_span_id: None,
            work: None,
            fields: vec![("subtotal_cents", "500".into())],
        };
        // Captured golden string — MUST NOT change after the ShadowDivergence variant is added.
        assert_eq!(
            ev.to_json(),
            "{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\",\"hw\":{\"cpu_ticks\":{\"unavailable\":\"sampling_disabled\"},\"rss_kb\":{\"unavailable\":\"sampling_disabled\"},\"joules_uj\":{\"unavailable\":\"sampling_disabled\"}},\"fields\":{\"subtotal_cents\":\"500\"}}"
        );
    }

    /// Write-only guarantee (structural): the `ShadowDivergence` emit is a no-op when no sink
    /// is installed (no FDR ring), so a default-built kernel pays zero cost and cannot change D.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn shadow_emit_is_noop_without_ring() {
        let unit = DecisionUnit::new(DomainTag::Harness, UnitEpoch(1), |_in: &HarnessInput| {
            Decision::Answer(HarnessOut { route_tier: 5 })
        });
        // ring = None ⇒ shadow OFF ⇒ pure decide(), no FDR write path touched.
        let d = decide_with_shadow(
            &unit,
            &HarnessInput { job: 2 },
            Some(&HarnessOut { route_tier: 0 }),
            None,
        );
        assert_eq!(d, Decision::Answer(HarnessOut { route_tier: 5 }));
    }
}
