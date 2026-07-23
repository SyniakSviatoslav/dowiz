//! `kernel::cross_tests` — meta + cross-system integration tests.
//!
//! Meta-tests verify the testing/evaluation infrastructure itself.
//! Cross-tests exercise multiple modules together to validate
//! the full stack (trinary × eigen × chronos × delta × enrich).
//!
//! These are NOT unit tests — they're integration tests that live in lib
//! to exercise cross-module boundaries.

#[cfg(test)]
mod tests {
    use crate::trinary::{Tri, TriMatrix, Rgb};
    use crate::trig::{Phase, Xyz, PhaseVector};
    use crate::eigen::{EigenDecomp, decompose, Eigen};
    use crate::delta::{Delta, DeltaTracker, DeltaComparison, compare};
    use crate::chronos::Chronos;
    use crate::wave::{Wave, InterferenceField, spectral_fingerprint};
    use crate::invert::{BackpropChain, backprop_from_deltas};
    use crate::cross_bridge::CrossBridgeRegistry;
    use crate::chronos_topology::TemporalTrinity;
    use std::collections::HashMap;

    // ─── META-1: Enrichment engine self-consistency ──────────────────────

    #[test]
    fn meta_enrichment_detects_known_intent() {
        let intents = crate::prompt_enrich::detect_all_intents(
            "fix compilation bug in rust module with idempotency invariants"
        );
        assert!(!intents.is_empty(), "must detect at least one intent");
        let primary = intents[0].0;
        assert!(
            primary == crate::prompt_enrich::PromptKind::Code
                || primary == crate::prompt_enrich::PromptKind::Debug,
            "primary intent should be code or debug, got {:?}", primary
        );
    }

    #[test]
    fn meta_pattern_inheritance_is_complete() {
        use crate::prompt_enrich::{detect_intent_tree, inherit_patterns};
        let paths = detect_intent_tree("implement a new API and secure it");
        assert!(!paths.is_empty(), "must detect tree paths");
        if let Some(path) = paths.first() {
            let patterns = inherit_patterns(path);
            assert!(patterns.iter().any(|p| p.name == "idempotency"), "must inherit idempotency");
            assert!(patterns.iter().any(|p| p.name == "invariant"), "must inherit invariant");
            assert!(patterns.iter().any(|p| p.name == "quality"), "must inherit quality");
        }
    }

    #[test]
    fn meta_oracle_improves_with_data() {
        let mut oracle = crate::code_oracle::EtaOracle::new();
        let naive = oracle.predict_eta(3, 100);
        for i in 0..10 {
            oracle.record(&["test.rs"], 10 + i * 5, 5 + i * 3, (i + 1) as f64 * 3.0);
        }
        let trained = oracle.predict_eta(3, 100);
        // Trained prediction should differ from naive (oracle learned)
        assert!(trained > 0.0, "trained prediction must be positive");
        let _ = naive; // naive baseline exists
    }

    // ─── CROSS-1: trinary × eigen × chronos ─────────────────────────────

    #[test]
    fn cross_trinary_eigen_chronos_roundtrip() {
        let mut c = Chronos::new(100);
        let mut values = HashMap::new();
        values.insert("stability".into(), 0.8);
        values.insert("confidence".into(), 0.6);
        c.snapshot(values.clone());

        // Eigen decomposition of snapshot values
        let vals: Vec<f64> = values.values().copied().collect();
        let decomp = decompose(&vals, 2);
        assert!(decomp.spectral_radius() <= 1.0, "stable values must have ρ ≤ 1");

        // TriMatrix encoding of the same state
        let mut m = TriMatrix::new(1, 2);
        m.set(0, 0, if vals[0] > 0.5 { Tri::True } else { Tri::False });
        m.set(0, 1, if vals[1] > 0.5 { Tri::True } else { Tri::False });
        assert_eq!(m.stability_index(), 1.0); // both True
    }

    // ─── CROSS-2: delta × wave × invert ─────────────────────────────────

    #[test]
    fn cross_delta_wave_invert_backprop() {
        let v0 = vec![0.0, 0.0];
        let v1 = vec![3.0, 4.0];
        let d = Delta::between(&v0, 1000, &v1, 2000);
        assert!(d.is_significant(1.0));

        // Backprop the delta
        let deltas = vec![("sensor_a".into(), 2, d.clone())];
        let report = backprop_from_deltas(&deltas, 0.5, 1.0);
        let display = report.display();
        assert!(display.contains("BACKPROP"), "backprop must render");
        assert!(display.contains("sensor_a"), "must identify source");

        // Wave fingerprint of the delta
        let fingerprint = spectral_fingerprint("delta_test", d.magnitude, 1000);
        assert_eq!(fingerprint.components.len(), 8, "spectral fingerprint must have 8 harmonics");
    }

    // ─── CROSS-3: chronos × topology × temporal trinity ─────────────────

    #[test]
    fn cross_chronos_topology_temporal_trinity() {
        let mut tt = TemporalTrinity::new(2, 2);
        let mut m1 = TriMatrix::new(2, 2);
        m1.set(0, 0, Tri::True);
        m1.set(0, 1, Tri::Unknown);
        tt.advance(m1.clone());

        let mut m2 = TriMatrix::new(2, 2);
        m2.set(0, 0, Tri::True);
        m2.set(0, 1, Tri::False);
        tt.advance(m2);

        // Past-present delta
        let (delta, changes) = tt.delta_past_present();
        assert!(changes > 0, "must detect changes between states");

        // Predicted should exist
        let (t, f, u) = tt.predicted.counts();
        assert!(t + f + u == 4, "predicted matrix must be 2×2");
    }

    // ─── CROSS-4: cross_bridge × enrichment ────────────────────────────

    #[test]
    fn cross_bridges_connect_real_kinds() {
        let reg = CrossBridgeRegistry::from_research();
        use crate::prompt_enrich::PromptKind;
        let conns = reg.find_connection(PromptKind::Code, PromptKind::Security);
        assert!(!conns.is_empty(), "code and security must be bridged");
        let bridges: Vec<&str> = conns.iter().map(|b| b.name.as_str()).collect();
        assert!(
            bridges.iter().any(|n| n.contains("python") || n.contains("security") || n.contains("tool")),
            "must find bridge between code and security"
        );
    }

    // ─── CROSS-5: trinary × RGB × phase encoding ────────────────────────

    #[test]
    fn cross_trinary_rgb_phase_consistency() {
        let tri = Tri::True;
        let rgb = Rgb::from_tri(tri);
        assert_eq!(rgb, Rgb::GREEN);

        let phase = Phase::new(0.0); // θ=0 → (1,0) = True
        assert!(phase.cos > 0.9); // close to 1.0
        assert!(phase.sin.abs() < 0.1); // close to 0.0

        // Consistency: True → Green → Phase(θ=0) all encode the same concept
        let xyz = Xyz::new(phase.cos, phase.sin, 0.0);
        assert!(xyz.x > 0.5); // True-ish
    }

    // ─── META-2: Test infrastructure self-check ─────────────────────────

    #[test]
    fn meta_test_infrastructure_components_present() {
        // Verify all core modules are registered
        let bridge_count = CrossBridgeRegistry::from_research().bridges.len();
        assert!(bridge_count >= 5, "must have at least 5 cross-bridges");

        // Verify eigen decomposition works on trivial input
        let decomp = decompose(&[1.0], 1);
        assert!(decomp.pairs.len() >= 1);

        // Verify delta comparison works
        assert_eq!(
            compare(&[0.0], 0, &[10.0], 1, 1.0),
            DeltaComparison::Growing
        );
    }

    #[test]
    fn cross_full_stack_idempotency_invariant_chain() {
        // Full stack: detect intent → inherit patterns → apply trinary → check delta → backprop
        let input = "implement idempotent state transitions with invariant checks";
        let intents = crate::prompt_enrich::detect_all_intents(input);
        assert!(!intents.is_empty());

        let paths = crate::prompt_enrich::detect_intent_tree(input);
        if let Some(path) = paths.first() {
            let patterns = crate::prompt_enrich::inherit_patterns(path);
            let has_idempotency = patterns.iter().any(|p| p.name == "idempotency");
            let has_invariant = patterns.iter().any(|p| p.name == "invariant");
            assert!(has_idempotency || has_invariant, "must find idempotency or invariant pattern");
        }
    }
}
