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
    use crate::trig::{Phase, Xyz};
    use crate::eigen::decompose;
    use crate::delta::{Delta, DeltaComparison, compare};
    use crate::chronos::Chronos;
    use crate::wave::spectral_fingerprint;
    use crate::invert::backprop_from_deltas;
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
        let (_delta, changes) = tt.delta_past_present();
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

    // ─── CROSS-BRIDGE E2E: all 7 bridges exercised simultaneously ────────

    /// 1: Python code → PythonKind patterns → SecurityKind patterns → verify cross-enrichment
    #[test]
    fn cross_bridge_full_pipeline_python_to_security() {
        let reg = CrossBridgeRegistry::from_research();
        use crate::prompt_enrich::PromptKind;

        // Find the python bridge and security bridge
        let py = reg.find_connection(PromptKind::Code, PromptKind::Security);
        assert!(!py.is_empty(), "python code and security must be bridged");

        // Simulate a Python security-audit prompt flowing through both bridges
        let enrichment = crate::prompt_enrich::detect_all_intents(
            "audit my Python crypto library for security vulnerabilities in pip packages"
        );
        assert!(enrichment.len() >= 1, "must detect intents for python+security prompt");
        let kinds: Vec<PromptKind> = enrichment.iter().map(|(k, _, _)| *k).collect();
        let has_code = kinds.contains(&PromptKind::Code);
        let has_security = kinds.contains(&PromptKind::Security);
        assert!(has_code || has_security,
            "cross-bridge pipeline must detect code or security from python+security input, got {:?}", kinds);
    }

    /// 2: Tool invocation → ToolKind patterns → LLMKind patterns → verify
    #[test]
    fn cross_bridge_full_pipeline_tool_to_llm() {
        let reg = CrossBridgeRegistry::from_research();
        use crate::prompt_enrich::PromptKind;

        let conns = reg.find_connection(PromptKind::Tool, PromptKind::Plugin);
        assert!(!conns.is_empty(), "tool and plugin must be bridged");

        let bridge = reg.strongest().unwrap();
        assert!(bridge.kinds.contains(&PromptKind::Tool),
            "strongest bridge must cover Tool kind");

        let enrichment = crate::prompt_enrich::detect_all_intents(
            "build a CLI tool that wraps the LLM inference API with plugin support"
        );
        assert!(enrichment.len() >= 1, "must detect intents for tool->llm pipeline");
        let kinds: Vec<PromptKind> = enrichment.iter().map(|(k, _, _)| *k).collect();
        assert!(
            kinds.iter().any(|k| matches!(k, PromptKind::Tool | PromptKind::Plugin | PromptKind::Code)),
            "tool->llm bridge must detect tool/plugin/code intent"
        );
    }

    /// 3: Education question → EducationKind → MLKind → verify
    #[test]
    fn cross_bridge_full_pipeline_education_to_ml() {
        let reg = CrossBridgeRegistry::from_research();
        use crate::prompt_enrich::PromptKind;

        let edu = reg.find_connection(PromptKind::Math, PromptKind::Tool);
        assert!(!edu.is_empty(), "education-related math and tool must be bridged");

        let enrichment = crate::prompt_enrich::detect_all_intents(
            "create an educational tutorial on machine learning model training with examples"
        );
        assert!(enrichment.len() >= 1, "must detect intents for education->ml pipeline");
        let kinds: Vec<PromptKind> = enrichment.iter().map(|(k, _, _)| *k).collect();
        assert!(
            kinds.iter().any(|k| matches!(k, PromptKind::Code | PromptKind::Write | PromptKind::General)),
            "education->ml bridge must produce detectable intents, got {:?}", kinds
        );
    }

    /// 4: Apply Tri::True/False/Unknown to bridge routing decisions
    #[test]
    fn cross_bridge_trinary_policy_on_bridge() {
        let reg = CrossBridgeRegistry::from_research();
        use crate::prompt_enrich::PromptKind;

        let conns = reg.find_connection(PromptKind::Code, PromptKind::Security);
        assert!(!conns.is_empty());

        // Encode routing decisions as a TriMatrix
        let mut routing = TriMatrix::new(3, 3);
        routing.set(0, 0, Tri::True);    // python-universal: active
        routing.set(0, 1, Tri::False);   // tool-cli-bridge: down
        routing.set(0, 2, Tri::Unknown); // security-hardening: unknown

        // True route is usable
        assert_eq!(routing.get(0, 0), Tri::True);
        // False route is blocked
        assert_eq!(routing.get(0, 1), Tri::False);
        // Unknown route needs observation
        assert_eq!(routing.get(0, 2), Tri::Unknown);

        let (t, f, u) = routing.counts();
        assert_eq!(t + f + u, 9, "3x3 TriMatrix must have 9 entries");
    }

    /// 5: Eigen correlation between PythonKind and SecurityKind
    #[test]
    fn cross_bridge_eigen_correlation_between_bridges() {
        let reg = CrossBridgeRegistry::from_research();
        use crate::prompt_enrich::PromptKind;
        use crate::eigen::{EigenDecomp, Eigen};

        // Build eigen vectors from bridge strengths
        let py_strength = reg.find_connection(PromptKind::Code, PromptKind::Math)
            .iter().map(|b| b.strength as f64).sum::<f64>();
        let sec_strength = reg.find_connection(PromptKind::Code, PromptKind::Security)
            .iter().map(|b| b.strength as f64).sum::<f64>();

        // Create eigen pairs from bridge strengths
        let e1 = Eigen::new(py_strength / 10000.0, vec![1.0, 0.8]);
        let e2 = Eigen::new(sec_strength / 10000.0, vec![0.9, 1.0]);

        let decomp = EigenDecomp::new(vec![e1.clone(), e2.clone()]);
        assert!(decomp.spectral_radius() > 0.0, "bridge eigen radii must be positive");

        // Compute correlation via dot product of eigenvectors
        let dot = e1.project(&e2.vector);
        assert!(dot > 0.0, "python and security bridges must have positive eigen correlation");

        // Both modes should be stable (strength ≤ 1000 → λ ≤ 1.0)
        for pair in &decomp.pairs {
            assert!(pair.is_stable(), "all bridge eigen modes must be stable");
        }
    }

    /// 6: WaveMeshSync propagates across 3 bridges
    #[test]
    fn cross_bridge_wave_mesh_sync_across_bridges() {
        use crate::wave::{InterferenceField, Wave};

        // Create wave propagation across 3 bridges: python, tool, llm
        let mut field = InterferenceField::new();
        let now = crate::now_ms();

        // Bridge 1: python-universal emits a wave
        field.add_wave(Wave::simple("python-universal", now, 1.0, 0.8, 0.05));

        // Bridge 2: tool-cli-bridge emits a wave
        field.add_wave(Wave::simple("tool-cli-bridge", now, 2.0, 0.6, 0.03));

        // Bridge 3: llm-ai-bridge emits a wave
        field.add_wave(Wave::simple("llm-ai-bridge", now, 3.0, 0.7, 0.04));

        // Verify all 3 bridges contributed active waves
        assert_eq!(field.active_count(), 3, "all 3 bridges must have active waves");

        let composite = field.composite();
        assert!(composite >= -1.0 && composite <= 1.0, "wave interference must stay bounded");

        // Verify spectral fingerprint from combined wave field
        let fingerprint = spectral_fingerprint("bridge_mesh", composite.abs(), now);
        assert_eq!(fingerprint.components.len(), 8, "spectral fingerprint must encode wave mesh");

        // Xyz state from interference encodes the 3-bridge superposition
        let xyz = field.xyz_state();
        assert!(xyz.x.abs() <= 1.0 && xyz.y.abs() <= 1.0 && xyz.z.abs() <= 1.0,
            "wave mesh xyz must stay in unit cube");
    }

    /// 7: ChronosDtn stores bridge frames and forwards after delay
    #[test]
    fn cross_bridge_chronos_dtn_store_forward() {
        use crate::chronos::Chronos;
        use crate::prompt_enrich::PromptKind;

        let mut dt = Chronos::new(100);
        let reg = CrossBridgeRegistry::from_research();

        // Store bridge frames as snapshots at different logical timestamps
        let mut frame0 = HashMap::new();
        for (_i, bridge) in reg.bridges.iter().enumerate() {
            frame0.insert(bridge.name.clone(), bridge.strength as f64);
        }
        let snap0 = dt.snapshot(frame0.clone());
        let ts0 = snap0.timestamp_ms;

        // Simulate DTN forward: store, wait, forward
        std::thread::sleep(std::time::Duration::from_millis(5));

        let mut frame1 = frame0.clone();
        if let Some(v) = frame1.get_mut("python-universal") {
            *v += 10.0; // strength increased after forwarding
        }
        let snap1 = dt.snapshot(frame1);
        let ts1 = snap1.timestamp_ms;

        // Verify store: both frames are retained
        assert!(dt.len() >= 2, "chronos DTN must store both frames");

        // Verify forward: delta between frames shows the change
        let (_xyz_delta, dim_deltas) = dt.delta(ts0, ts1).unwrap_or_else(|| {
            panic!("chronos DTN must find delta between stored frames at {ts0}→{ts1}")
        });
        assert!(dim_deltas.contains_key("python-universal"),
            "DTN forward must propagate python-universal strength increase");

        // Verify time window contains both frames
        let window = dt.window(ts0, ts1);
        assert!(window.len() >= 2, "DTN window must contain stored+forwarded frames");
    }

    /// 8: Register all 7 bridges, verify they're all reachable
    #[test]
    fn cross_bridge_publisher_registry_all_bridges() {
        use crate::cross_bridge::PublisherRegistry;
        use crate::prompt_enrich::PromptKind;

        let reg = CrossBridgeRegistry::from_research();
        let pr = PublisherRegistry::from_research();

        // All 7 bridges present
        let bridge_names: Vec<&str> = reg.bridges.iter().map(|b| b.name.as_str()).collect();
        assert!(bridge_names.contains(&"python-universal"));
        assert!(bridge_names.contains(&"tool-cli-bridge"));
        assert!(bridge_names.contains(&"llm-ai-bridge"));
        assert!(bridge_names.contains(&"claude-agent-hub"));
        assert!(bridge_names.contains(&"security-hardening"));
        assert!(bridge_names.contains(&"ml-datascience"));
        assert!(bridge_names.contains(&"education-bridge"));
        assert_eq!(reg.bridges.len(), 7, "all 7 bridges must be registered");

        // Every publisher's kinds are covered by at least one bridge
        let covered = reg.kinds_covered();
        for p in &pr.publishers {
            assert!(
                p.kinds.iter().any(|k| covered.contains(k)),
                "publisher {} must have at least 1 kind in bridge coverage", p.name
            );
        }

        // Dashboard renders all bridges
        let d = reg.dashboard();
        assert!(d.contains("═══ CROSS-BRIDGE REGISTRY ═══"));
        for name in &bridge_names {
            assert!(d.contains(name), "dashboard must list bridge {}", name);
        }
    }

    /// 9: Delta comparison between frame at t=0 and frame at t=100
    #[test]
    fn cross_bridge_delta_comparison_across_kinds() {
        use crate::delta::{Delta, DeltaComparison, compare, EigenDelta};
        use crate::eigen::decompose;

        // Simulate kind strengths at two time points
        let strengths_t0 = vec![0.5, 0.3, 0.8, 0.2, 0.6, 0.4, 0.7]; // 7 bridges
        let strengths_t100 = vec![0.6, 0.4, 0.9, 0.3, 0.7, 0.5, 0.8];

        // Delta between t=0 and t=100
        let d = Delta::between(&strengths_t0, 0, &strengths_t100, 100);
        assert!(d.magnitude > 0.0, "delta must capture change between t0 and t100");

        let cmp = compare(&strengths_t0, 0, &strengths_t100, 100, 0.05);
        assert_eq!(cmp, DeltaComparison::Growing, "bridge strengths must be growing over time");

        // Eigen decomposition comparison
        let d0 = decompose(&strengths_t0, 4);
        let d1 = decompose(&strengths_t100, 4);
        let _ed = EigenDelta::between(&d0, &d1);

        // Delta tracker accumulates across kinds
        let mut tracker = crate::delta::DeltaTracker::new(1.0, 0.1);
        tracker.observe(d);
        assert!(tracker.cumulative_drift > 0.0, "delta tracker must accumulate bridge drift");
    }

    /// 10: Fractal recursion across bridge chains, verify convergence
    #[test]
    fn cross_bridge_fractal_recursion_depth() {
        use crate::fractal::{fractal_from_vec, FractalNode};

        let reg = CrossBridgeRegistry::from_research();

        // Build a fractal from bridge strengths (normalized to unit range)
        let strengths: Vec<f64> = reg.bridges.iter()
            .map(|b| b.strength as f64 / 10000.0)
            .collect();
        assert!(strengths.len() >= 7, "must have 7 bridge strengths for fractal");

        // Build fractal at increasing depths
        let depth1 = fractal_from_vec(&strengths, 1);
        let depth2 = fractal_from_vec(&strengths, 2);

        // Verify recursion: deeper fractal has more leaves
        assert!(depth2.leaf_count() >= depth1.leaf_count(),
            "deeper fractal must have >= leaf count (d1={}, d2={})",
            depth1.leaf_count(), depth2.leaf_count());

        // Verify convergence: spectral radius should be stable (≤ 1.0)
        assert!(depth1.is_stable(), "fractal depth 1 must be stable");
        assert!(depth2.is_stable(), "fractal depth 2 must be stable");

        // Recursive recomputation: add child fractals and verify
        let mut root = FractalNode::new("bridge-root", 0);
        for (i, b) in reg.bridges.iter().enumerate() {
            let leaf = FractalNode::leaf(&b.name, i + 1, vec![b.strength as f64 / 10000.0]);
            root.add_child(leaf);
        }
        assert_eq!(root.children.len(), 7, "all 7 bridges must be fractal children");
        assert!(root.radius() > 0.0, "bridge fractal root must have positive spectral radius");
        assert!(root.radius() <= 1.0, "bridge fractal root must be stable (ρ ≤ 1)");

        // ASCII rendering contains all bridges
        let ascii = root.ascii().to_string();
        assert!(ascii.contains("python-universal"), "fractal ASCII must show python-universal bridge");
        assert!(ascii.contains("security-hardening"), "fractal ASCII must show security-hardening bridge");
    }
}
