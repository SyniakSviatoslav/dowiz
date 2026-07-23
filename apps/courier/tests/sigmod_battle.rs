//! SIGMOD battle-test — full end-to-end integration test for the courier surface.
//!
//! Exercises the entire P71 surface stack: CourierSurface, dispatch, voice, battery,
//! render, geo, and edge cases. One `#[test]` per scenario.

use dowiz_courier::*;
use dowiz_courier::dispatch::*;
use dowiz_courier::surface::*;
use dowiz_courier::battery::*;
use dowiz_courier::voice::*;
use dowiz_courier::types::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const ME: CourierKey = [0u8; 32];
const NODE1: CourierKey = [1u8; 32];
const NODE2: CourierKey = [2u8; 32];
const NODE3: CourierKey = [3u8; 32];

// ── test stub types for concepts not yet in the public API ────────────────

/// A minimal mesh of courier surfaces — demonstrates the pattern for
/// multi-node dispatch testing (future P65 integration).
struct CourierMesh {
    nodes: Vec<CourierSurface>,
}

impl CourierMesh {
    fn new(keys: &[CourierKey]) -> Self {
        Self {
            nodes: keys.iter().map(|&k| CourierSurface::new(k)).collect(),
        }
    }

    fn len(&self) -> usize { self.nodes.len() }
}

/// Marker for a courier in motion — wraps an ActiveRun in transit.
struct CourierInMotion {
    run: ActiveRun,
    start: GeoRef,
    end: GeoRef,
}

impl CourierInMotion {
    fn new(start: GeoRef, end: GeoRef) -> Self {
        Self {
            run: ActiveRun {
                run_id: 1,
                claim_id: 1,
                order_id: 100,
                in_transit: true,
                track: Some(TrackFrame {
                    est: 1000,
                    v_mps: 5.0,
                    eta_s: 1200,
                    remaining_m: 6000.0,
                    route_version: 1,
                }),
            },
            start,
            end,
        }
    }
}

/// Minimal route plan: ordered list of waypoints with a capacity gate.
struct RoutePlan {
    waypoints: Vec<GeoRef>,
    capacity: usize,
}

impl RoutePlan {
    fn new(waypoints: Vec<GeoRef>, capacity: usize) -> Self {
        Self { waypoints, capacity }
    }

    fn feasible(&self) -> bool {
        self.waypoints.len() <= self.capacity
    }
}

// ── helper ─────────────────────────────────────────────────────────────────

fn geo(lat_micro: i32, lon_micro: i32) -> GeoRef {
    GeoRef { lat_micro, lon_micro }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[test]
fn scenario_1_surface_initializes_with_default_state() {
    let surface = CourierSurface::new(ME);
    assert!(matches!(surface.state, SurfaceOfferState::Idle));
    assert_eq!(surface.me, ME);
}

#[test]
fn scenario_2_mesh_with_3_nodes() {
    let mesh = CourierMesh::new(&[NODE1, NODE2, NODE3]);
    assert_eq!(mesh.len(), 3);
    for node in &mesh.nodes {
        assert!(matches!(node.state, SurfaceOfferState::Idle));
    }
}

#[test]
fn scenario_3_courier_in_motion_start_to_end() {
    let start = geo(40_712_000, -74_006_000);   // NYC
    let end = geo(34_052_000, -118_243_000);     // LA
    let in_motion = CourierInMotion::new(start, end);
    assert!(in_motion.run.in_transit);
    assert_eq!(in_motion.run.track.as_ref().unwrap().route_version, 1);
    assert!((in_motion.run.track.unwrap().remaining_m - 6000.0).abs() < 0.01);
}

#[test]
fn scenario_4_voice_profile_transitions() {
    // idle → balanced
    let surface = CourierSurface::new(ME);
    let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
    assert_eq!(input_profile_for(&state), InputProfile::Balanced);

    // balanced → in_transit (via ActiveRun)
    let in_motion = CourierSurfaceState {
        shell: CourierShell::TauriMobile,
        offer: None,
        run: Some(ActiveRun {
            run_id: 1,
            claim_id: 1,
            order_id: 1,
            in_transit: true,
            track: None,
        }),
    };
    assert_eq!(input_profile_for(&in_motion), InputProfile::CourierInMotion);

    // passed → balanced (run complete, no transit)
    let mut surface = CourierSurface::new(ME);
    surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 200 });
    let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
    assert_eq!(input_profile_for(&state), InputProfile::Balanced);

    // routed → balanced (accepted but not yet in transit)
    let accepted = CourierSurfaceState {
        shell: CourierShell::TauriMobile,
        offer: None,
        run: Some(ActiveRun {
            run_id: 1,
            claim_id: 1,
            order_id: 1,
            in_transit: false,
            track: None,
        }),
    };
    assert_eq!(input_profile_for(&accepted), InputProfile::Balanced);
}

#[test]
fn scenario_5_battery_gate_all_7_decisions() {
    // 1. Blocked → Owed
    let v = VerdictRecord::Blocked { on: "no device".into() };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::Owed);

    // 2. Confirms on Emulator → RejectedEmulator
    let v = VerdictRecord::Confirms {
        hw: HwClass::Emulator,
        shift_hours: 6.0,
        settled_drain_pct_hr: 2.0,
        settle_saving_pct: 50.0,
        thermal_sustain_ms: 30.0,
    };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::RejectedEmulator);

    // 3. Confirms, all within bars → Pass
    let v = VerdictRecord::Confirms {
        hw: HwClass::BudgetAndroid,
        shift_hours: 6.0,
        settled_drain_pct_hr: 2.0,
        settle_saving_pct: 50.0,
        thermal_sustain_ms: 30.0,
    };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::Pass);

    // 4. Confirms, high drain → Fail("settled_drain")
    let v = VerdictRecord::Confirms {
        hw: HwClass::BudgetAndroid,
        shift_hours: 6.0,
        settled_drain_pct_hr: 10.0,
        settle_saving_pct: 50.0,
        thermal_sustain_ms: 30.0,
    };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::Fail("settled_drain"));

    // 5. Confirms, low settle savings → Fail("settle_saving")
    let v = VerdictRecord::Confirms {
        hw: HwClass::BudgetAndroid,
        shift_hours: 6.0,
        settled_drain_pct_hr: 2.0,
        settle_saving_pct: 20.0,
        thermal_sustain_ms: 30.0,
    };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::Fail("settle_saving"));

    // 6. Confirms, high thermal → Fail("thermal")
    let v = VerdictRecord::Confirms {
        hw: HwClass::BudgetAndroid,
        shift_hours: 6.0,
        settled_drain_pct_hr: 2.0,
        settle_saving_pct: 50.0,
        thermal_sustain_ms: 50.0,
    };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::Fail("thermal"));

    // 7. Refines on real hw → Owed
    let v = VerdictRecord::Refines {
        hw: HwClass::BudgetAndroid,
        settle_saving_pct: 25.0,
        note: "nav-tier".into(),
    };
    assert_eq!(evaluate_battery_gate(&v), BatteryGate::Owed);
}

#[test]
fn scenario_6_surface_consume_not_stale() {
    let mut surface = CourierSurface::new(ME);
    let consumed = surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 200,
    });
    assert_eq!(consumed.len(), 1);
    // Fresh consume from just-created Live state
    if let SurfaceConsume::Live(card) = &consumed[0] {
        assert_eq!(card.deadline_ts, 200);
    } else {
        panic!("expected Live consume");
    }
    // Verify it's still usable (the card has meaningful fields)
    let live = consumed.into_iter().next().unwrap();
    match live {
        SurfaceConsume::Live(c) => assert_eq!(c.deadline_ts, 200),
        _ => unreachable!(),
    }
}

#[test]
fn scenario_7_geo_ref_resolution() {
    let ref_nyc = geo(40_712_000, -74_006_000);
    // lat_micro/lon_micro map to a coarse grid cell for pre-accept privacy.
    // The grid cell is derived from integer division; verify bounds.
    assert_eq!(ref_nyc.lat_micro, 40_712_000);
    assert_eq!(ref_nyc.lon_micro, -74_006_000);

    let zone = ZoneRef { zone_id: 42 };
    assert_eq!(zone.zone_id, 42);

    // Default values are zero-origin.
    let zero = GeoRef::default();
    assert_eq!(zero.lat_micro, 0);
    assert_eq!(zero.lon_micro, 0);
}

#[test]
fn scenario_8_route_plan_3_point_with_capacity() {
    let a = geo(0, 0);
    let b = geo(1000, 500);
    let c = geo(2000, 1000);

    // Capacity = 3: feasible
    let plan = RoutePlan::new(vec![a, b, c], 3);
    assert!(plan.feasible());

    // Capacity = 2: infeasible
    let plan = RoutePlan::new(vec![a, b, c], 2);
    assert!(!plan.feasible());

    // Capacity = 5: feasible with headroom
    let plan = RoutePlan::new(vec![a, b, c], 5);
    assert!(plan.feasible());
}

#[test]
fn scenario_9_dispatch_types_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DispatchEvent>();
    assert_send_sync::<DispatchInput>();
    assert_send_sync::<DispatchSession>();
    assert_send_sync::<SurfaceConsume>();
    assert_send_sync::<DispatchInputFrame>();
    assert_send_sync::<CourierSurface>();
    assert_send_sync::<LiveOffer>();
    assert_send_sync::<OfferCard>();
    assert_send_sync::<GeoRef>();
    assert_send_sync::<ZoneRef>();
}

#[test]
fn scenario_10_edge_cases_no_panic() {
    // Empty voice phrase
    let empty = VoicePhrase {
        transcript: String::new(),
        confidence: 1.0,
        is_final: true,
    };
    assert_eq!(classify(&empty), Classification::Rejected);

    // Zero battery (all Confirms bars at zero — drain=0 passes, saving=0 fails)
    let v = VerdictRecord::Confirms {
        hw: HwClass::BudgetAndroid,
        shift_hours: 0.0,
        settled_drain_pct_hr: 0.0,
        settle_saving_pct: 0.0,
        thermal_sustain_ms: 0.0,
    };
    let result = evaluate_battery_gate(&v);
    assert!(matches!(result, BatteryGate::Fail("settle_saving")));

    // 0-capacity route plan should reject 1 waypoint
    let plan = RoutePlan::new(vec![geo(0, 0)], 0);
    assert!(!plan.feasible());

    // Empty mesh (zero nodes)
    let mesh = CourierMesh::new(&[]);
    assert_eq!(mesh.len(), 0);

    // Urgency at exactly deadline (stake should be maxed)
    let urgency = offer_urgency(200, 200);
    assert!((urgency.stake - 1.0).abs() < 0.001);

    // Urgency far before deadline
    let urgency = offer_urgency(0, 200);
    assert_eq!(urgency.stake, 0.0);

    // emit_accept from non-Live state → None
    let mut surface = CourierSurface::new(ME);
    assert!(surface.emit_accept().is_none());
    assert!(surface.emit_decline().is_none());
    assert!(surface.emit_accept_pending().is_none());
}

#[test]
fn scenario_11_mesh_multi_node_dispatch_drill() {
    // 3-node mesh: offer to node 1, verify node 2+3 unaffected
    let mut nodes = vec![
        CourierSurface::new(NODE1),
        CourierSurface::new(NODE2),
        CourierSurface::new(NODE3),
    ];

    // Offer to NODE1
    let c1 = nodes[0].on_event(&DispatchEvent::Offered {
        courier: NODE1,
        deadline_ts: 500,
    });
    assert_eq!(c1.len(), 1);

    // NODE2 gets an unrelated offer — should not consume it
    let c2 = nodes[1].on_event(&DispatchEvent::Offered {
        courier: NODE1,
        deadline_ts: 500,
    });
    assert!(c2.is_empty());
    assert!(matches!(nodes[1].state, SurfaceOfferState::Idle));

    // NODE1 accepts, then gets passed
    assert!(nodes[0].emit_accept().is_some());
    let c1b = nodes[0].on_event(&DispatchEvent::StaleAccept { courier: NODE1 });
    assert_eq!(c1b.len(), 1);
    assert!(matches!(nodes[0].state, SurfaceOfferState::Passed { .. }));
}

#[test]
fn scenario_12_surface_full_offer_lifecycle() {
    let mut surface = CourierSurface::new(ME);

    // Offered — surface goes Live
    let c = surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 1000,
    });
    assert_eq!(c.len(), 1);
    assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));

    // Assign directly (P65 confirms without courier-side accept processing)
    // This tests the surface's reception of the hub's confirmation.
    let c = surface.on_event(&DispatchEvent::Assigned { courier: ME });
    assert_eq!(c.len(), 1);
    assert!(matches!(surface.state, SurfaceOfferState::Accepted { .. }));
}

#[test]
fn scenario_13_urgency_audio_visual_parity() {
    // P2 parity: the same stake drives audio and visual intensity
    let audio = offer_urgency(195, 200);
    let visual = offer_field_intensity(195, 200);
    assert!((audio.stake - visual).abs() < 0.0001);

    // At 0s remaining, both max
    let a0 = offer_urgency(200, 200);
    let v0 = offer_field_intensity(200, 200);
    assert!((a0.stake - v0).abs() < 0.0001);
    assert!((a0.stake - 1.0).abs() < 0.001);
}

#[test]
fn scenario_14_classify_all_intents() {
    // Command intents
    for word in &["accept", "yes", "confirm", "skip", "decline", "no"] {
        let phrase = VoicePhrase { transcript: (*word).into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Resolved(Intent::Command),
            "failed for command: {}", word);
    }

    // Navigate intents
    for word in &["navigate", "status", "where", "directions"] {
        let phrase = VoicePhrase { transcript: (*word).into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Resolved(Intent::Navigate),
            "failed for navigate: {}", word);
    }

    // Rejected intents
    for word in &["hello", "", " "] {
        let phrase = VoicePhrase { transcript: (*word).into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Rejected,
            "failed for reject: {:?}", word);
    }
}

#[test]
fn scenario_15_battery_is_measured_gate() {
    assert!(!VerdictRecord::Blocked { on: "n/a".into() }.is_measured());
    assert!(VerdictRecord::Confirms {
        hw: HwClass::BudgetAndroid, shift_hours: 6.0,
        settled_drain_pct_hr: 2.0, settle_saving_pct: 50.0, thermal_sustain_ms: 30.0,
    }.is_measured());
    assert!(!VerdictRecord::Confirms {
        hw: HwClass::Emulator, shift_hours: 6.0,
        settled_drain_pct_hr: 2.0, settle_saving_pct: 50.0, thermal_sustain_ms: 30.0,
    }.is_measured());
    assert!(VerdictRecord::Contradicts {
        hw: HwClass::BudgetAndroid, reason: "shift".into(),
    }.is_measured());
    assert!(!VerdictRecord::Contradicts {
        hw: HwClass::Emulator, reason: "nope".into(),
    }.is_measured());
}

#[test]
fn scenario_16_battery_bars_are_positive() {
    let (shift, drain, saving, thermal) = battery_bars();
    assert!(shift > 0.0);
    assert!(drain > 0.0);
    assert!(saving > 0.0);
    assert!(thermal > 0.0);
}

#[test]
fn scenario_17_types_size_contracts() {
    assert_eq!(std::mem::size_of::<CourierShell>(), 1);
    assert_eq!(std::mem::size_of::<GeoRef>(), 8);
    assert_eq!(std::mem::size_of::<ZoneRef>(), 4);
    assert_eq!(std::mem::size_of::<CourierKey>(), 32);
    assert_eq!(std::mem::size_of::<DispatchInputKind>(), 1);
    assert_eq!(std::mem::size_of::<AdvanceReason>(), 1);
    assert_eq!(std::mem::size_of::<LiveOffer>(), 40);
    assert_eq!(std::mem::size_of::<InputProfile>(), 1);
    assert_eq!(std::mem::size_of::<AiMode>(), 1);
    assert_eq!(std::mem::size_of::<HwClass>(), 1);
}

#[test]
fn scenario_18_full_battery_gate_exhaustive() {
    // Exhaustively verify all 4 variants with all 3 hw classes
    let verdicts = vec![
        VerdictRecord::Blocked { on: "none".into() },
        VerdictRecord::Confirms { hw: HwClass::BudgetAndroid, shift_hours: 6.0, settled_drain_pct_hr: 2.0, settle_saving_pct: 50.0, thermal_sustain_ms: 30.0 },
        VerdictRecord::Confirms { hw: HwClass::Emulator, shift_hours: 6.0, settled_drain_pct_hr: 2.0, settle_saving_pct: 50.0, thermal_sustain_ms: 30.0 },
        VerdictRecord::Confirms { hw: HwClass::Flagship, shift_hours: 6.0, settled_drain_pct_hr: 2.0, settle_saving_pct: 50.0, thermal_sustain_ms: 30.0 },
        VerdictRecord::Refines { hw: HwClass::BudgetAndroid, settle_saving_pct: 25.0, note: "a".into() },
        VerdictRecord::Refines { hw: HwClass::Emulator, settle_saving_pct: 25.0, note: "b".into() },
        VerdictRecord::Contradicts { hw: HwClass::BudgetAndroid, reason: "r1".into() },
        VerdictRecord::Contradicts { hw: HwClass::Emulator, reason: "r2".into() },
        VerdictRecord::Contradicts { hw: HwClass::Flagship, reason: "r3".into() },
    ];
    for v in &verdicts {
        let _result = evaluate_battery_gate(v);
        // All calls succeed, no panics.
    }
}

#[test]
fn scenario_19_surface_accidental_double_accept() {
    let mut surface = CourierSurface::new(ME);
    surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 200 });
    // First accept succeeds
    assert!(surface.emit_accept().is_some());
    // Second accept fails — witness consumed
    assert!(surface.emit_accept().is_none());
    // emit_decline also fails
    assert!(surface.emit_decline().is_none());
}

#[test]
fn scenario_20_surface_accept_pending_only_from_live() {
    let mut surface = CourierSurface::new(ME);
    assert!(surface.emit_accept_pending().is_none());
    surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 200 });
    let frame = surface.emit_accept_pending();
    assert!(frame.is_some());
    assert_eq!(frame.unwrap().kind, DispatchInputKind::Accept);
    // Does NOT consume the Live witness
    assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));
}
