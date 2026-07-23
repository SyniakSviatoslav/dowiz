//! BLUEPRINT P71 render-pipeline integration test.
//!
//! Exercises the full courier render pipeline:
//! 1. Engine FieldFrame creation and evolution
//! 2. CourierFrame composition from a FieldFrame
//! 3. CourierSurface state-machine transitions (Idle→Live→Accepted→Passed)
//! 4. Field cell value validity
//! 5. GeoRef coordinate validity
//! 6. DispatchInputFrame creation and consumption by DispatchSession

use dowiz_courier::*;
use dowiz_courier::dispatch::*;
use dowiz_courier::render::*;
use dowiz_courier::surface::*;
use dowiz_courier::types::*;
use dowiz_engine::field_frame::{FieldEquilibrium, FieldFrame};
use dowiz_engine::scene::{Scene, SdfShape};

const ME: CourierKey = [1u8; 32];

fn geo_ref(lat: i32, lon: i32) -> GeoRef {
    GeoRef {
        lat_micro: lat,
        lon_micro: lon,
    }
}

// ── 1. FieldFrame creation and evolution ───────────────────────────────────

#[test]
fn field_frame_creates_and_evolves() {
    let w = 16usize;
    let h = 16usize;
    let eq = FieldEquilibrium::default();

    let mut scene = Scene::new().with_scale(0.5);
    scene.add(SdfShape::Circle {
        cx: 0.0,
        cy: 0.0,
        r: 3.0,
    });
    let source = scene.render_frame(w, h);

    let mut frame = FieldFrame::new(w, h);
    // Initial field should be all zeros
    for &v in frame.u() {
        assert!(v.is_finite(), "initial field must be finite");
        assert_eq!(v, 0.0, "initial field must be zero");
    }

    // Evolve a few steps
    for _ in 0..10 {
        frame.step(&source, &eq);
    }

    // After evolution, field must be finite everywhere
    for &v in frame.u() {
        assert!(v.is_finite(), "evolved field must be finite");
    }

    // RGBA output has correct dimensions
    let rgba = frame.frame_rgba();
    assert_eq!(rgba.len(), w * h * 4, "RGBA frame must be w*h*4 bytes");

    // RGBA bytes are all valid (0-255)
    for &b in &rgba {
        assert!(b <= 255, "RGBA byte must be <= 255");
    }
}

// ── 2. CourierFrame composition from a Scene ────────────────────────────────

#[test]
fn courier_frame_composes_from_scene() {
    let run = ActiveRun {
        run_id: 42,
        claim_id: 7,
        order_id: 100,
        in_transit: true,
        track: Some(TrackFrame {
            est: 1000,
            v_mps: 5.5,
            eta_s: 300,
            remaining_m: 1650.0,
            route_version: 1,
        }),
    };

    let frame = compose_run(&run);
    assert!(!frame.field.is_empty(), "field must not be empty");
    assert_eq!(frame.field.len(), 96 * 48, "run frame is 96x48");
    assert_eq!(frame.a11y.nodes.len(), 2, "run frame has marker + ETA a11y nodes");

    // hash_f32 is stable
    let h1 = hash_f32(&frame.field);
    let h2 = hash_f32(&frame.field);
    assert_eq!(h1, h2, "hash_f32 must be deterministic");
    assert_eq!(h1, frame.field_hash, "field_hash must match embedded hash");
}

#[test]
fn duty_frame_composes_correct_dims() {
    let frame = compose_duty(CourierShell::TauriMobile, true);
    assert_eq!(frame.field.len(), 64 * 32, "duty frame is 64x32");
    assert!(!frame.a11y.nodes.is_empty(), "duty frame has a11y nodes");
}

#[test]
fn no_track_frame_is_honest() {
    let run = ActiveRun {
        run_id: 1,
        claim_id: 1,
        order_id: 1,
        in_transit: false,
        track: None,
    };
    let frame = compose_run_no_track(&run);
    assert_eq!(frame.field.len(), 96 * 48, "no-track frame is 96x48");
    // The a11y tree must contain the NO_TRACK status text
    let has_no_track = frame.a11y.nodes.iter().any(|n| {
        n.state.value_text.as_deref() == Some("NO_TRACK")
    });
    assert!(has_no_track, "no-track frame must advertise NO_TRACK");
}

// ── 3. CourierSurface state-machine transitions ────────────────────────────

#[test]
fn surface_full_state_machine() {
    let mut surface = CourierSurface::new(ME);

    // Idle initially
    assert!(matches!(surface.state, SurfaceOfferState::Idle));

    // Offered → Live
    let consumed = surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 1000,
    });
    assert_eq!(consumed.len(), 1);
    assert!(matches!(consumed[0], SurfaceConsume::Live(_)));
    match &surface.state {
        SurfaceOfferState::Live { card } => {
            assert_eq!(card.deadline_ts, 1000);
        }
        other => panic!("expected Live, got {other:?}"),
    }

    // Accept emits a DispatchInputFrame from Live
    let frame = surface.emit_accept();
    assert!(frame.is_some());
    let frame = frame.unwrap();
    assert_eq!(frame.courier, ME);
    assert_eq!(frame.kind, DispatchInputKind::Accept);

    // State after accept is Passed (witness consumed)
    assert!(matches!(surface.state, SurfaceOfferState::Passed { .. }));

    // Double accept fails
    assert!(surface.emit_accept().is_none());
}

#[test]
fn surface_live_to_accepted_via_assigned() {
    let mut surface = CourierSurface::new(ME);
    surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 500,
    });
    assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));

    // Assigned by hub → Accepted with an ActiveRun
    let consumed = surface.on_event(&DispatchEvent::Assigned { courier: ME });
    assert_eq!(consumed.len(), 1);
    match &consumed[0] {
        SurfaceConsume::Accepted(run) => {
            assert!(!run.in_transit, "newly accepted run is not yet in transit");
        }
        other => panic!("expected Accepted, got {other:?}"),
    }
    assert!(matches!(surface.state, SurfaceOfferState::Accepted { .. }));
}

#[test]
fn surface_decline_advances_to_passed() {
    let mut surface = CourierSurface::new(ME);
    surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 500,
    });

    let frame = surface.emit_decline();
    assert!(frame.is_some());
    assert_eq!(frame.unwrap().kind, DispatchInputKind::Decline);
    assert!(matches!(surface.state, SurfaceOfferState::Passed { .. }));
}

#[test]
fn surface_stale_accept_is_passed_stale() {
    let mut surface = CourierSurface::new(ME);
    // StaleAccept while Idle (no offer was ever live)
    let consumed = surface.on_event(&DispatchEvent::StaleAccept { courier: ME });
    assert_eq!(consumed.len(), 1);
    match &consumed[0] {
        SurfaceConsume::Passed { stale } => assert!(*stale),
        other => panic!("expected Passed{{stale:true}}, got {other:?}"),
    }
    assert!(matches!(surface.state, SurfaceOfferState::Passed { stale: true }));
}

#[test]
fn surface_ignores_other_courier_in_all_states() {
    let other: CourierKey = [99u8; 32];
    let mut surface = CourierSurface::new(ME);

    // Idle: offer to other is ignored
    let c = surface.on_event(&DispatchEvent::Offered {
        courier: other,
        deadline_ts: 500,
    });
    assert!(c.is_empty());
    assert!(matches!(surface.state, SurfaceOfferState::Idle));

    // Give ME a live offer
    surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 500,
    });

    // Assigned to other is ignored
    let c = surface.on_event(&DispatchEvent::Assigned { courier: other });
    assert!(c.is_empty());
    assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));
}

// ── 4. Field cell value validity ────────────────────────────────────────────

#[test]
fn field_cell_values_are_valid() {
    let mut scene = Scene::new().with_scale(1.0);
    scene.add(SdfShape::Circle {
        cx: 0.0,
        cy: 0.0,
        r: 4.0,
    });
    let field = scene.render_frame(32, 32);

    // Every cell must be finite
    for &v in &field {
        assert!(v.is_finite(), "SDF value must be finite");
    }

    // Center must be inside (negative SDF)
    let cx = 15usize;
    let cy = 15usize;
    let center = field[cy * 32 + cx];
    assert!(center < 0.0, "centre cell must be inside (negative)");

    // Corner must be outside (positive SDF)
    let corner = field[0];
    assert!(corner > 0.0, "corner cell must be outside (positive)");

    // Field evolves under equilibrium and stays finite
    let eq = FieldEquilibrium::default();
    let mut frame = FieldFrame::new(32, 32);
    for _ in 0..50 {
        frame.step(&field, &eq);
    }
    for &v in frame.u() {
        assert!(v.is_finite(), "evolved field must stay finite");
    }
}

// ── 5. GeoRef coordinate validity ───────────────────────────────────────────

#[test]
fn geo_ref_values_are_valid() {
    let pickup = geo_ref(40_712_000, -74_006_000);
    assert_eq!(pickup.lat_micro, 40_712_000);
    assert_eq!(pickup.lon_micro, -74_006_000);

    let dropoff = ZoneRef { zone_id: 7 };
    assert_eq!(dropoff.zone_id, 7);

    // GeoRef from OfferCard is valid
    let card = OfferCard {
        claim_id: 1,
        order_id: 42,
        deadline_ts: 1000,
        pickup,
        dropoff_coarse: dropoff,
        payout_i64: 500,
    };
    assert_eq!(card.pickup.lat_micro, 40_712_000);
    assert_eq!(card.pickup.lon_micro, -74_006_000);
    assert_eq!(card.dropoff_coarse.zone_id, 7);
    assert_eq!(card.deadline_ts, 1000);
    assert_eq!(card.payout_i64, 500);
}

#[test]
fn geo_ref_default_is_zero() {
    let zero = GeoRef::default();
    assert_eq!(zero.lat_micro, 0);
    assert_eq!(zero.lon_micro, 0);
}

// ── 6. DispatchInputFrame creation and consumption ──────────────────────────

#[test]
fn dispatch_input_frame_creation_from_live() {
    let mut surface = CourierSurface::new(ME);
    // No accept from Idle
    assert!(surface.emit_accept().is_none());
    assert!(surface.emit_decline().is_none());
    assert!(surface.emit_accept_pending().is_none());

    surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 500,
    });

    // Accept frame from Live
    let frame = surface.emit_accept().unwrap();
    assert_eq!(frame.courier, ME);
    assert_eq!(frame.kind, DispatchInputKind::Accept);

    // Pending accept from Live (doesn't consume witness — test with fresh surface)
    let mut s2 = CourierSurface::new(ME);
    s2.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 500,
    });
    let pending = s2.emit_accept_pending().unwrap();
    assert_eq!(pending.kind, DispatchInputKind::Accept);
    assert!(matches!(s2.state, SurfaceOfferState::Live { .. }), "pending does NOT consume the Live witness");
}

#[test]
fn dispatch_input_frame_consumed_by_session() {
    let mut session = DispatchSession::new();
    let me: CourierKey = [1u8; 32];

    session.offer(me, 100);
    assert!(session.live_offer().is_some());

    // Accept within window → assigned
    let events = session.tick(accept_input(me), &[me], 110);
    assert!(events.iter().any(|e| matches!(e, DispatchEvent::Assigned { .. })));
    assert_eq!(session.assigned(), Some(me));
}

#[test]
fn dispatch_input_frame_declined() {
    let mut session = DispatchSession::new();
    let me: CourierKey = [1u8; 32];

    session.offer(me, 100);
    let events = session.tick(decline_input(me), &[me], 105);
    assert!(events.iter().any(|e| matches!(e, DispatchEvent::Advanced { .. })));
    assert!(events.iter().any(|e| matches!(e, DispatchEvent::Requeued)));
    assert!(session.live_offer().is_none());
}

#[test]
fn dispatch_input_frame_expired_accept_is_stale() {
    let mut session = DispatchSession::new();
    let me: CourierKey = [1u8; 32];

    session.offer(me, 100);
    // Tick past deadline
    session.tick(DispatchInput::Tick, &[], 200);
    assert!(session.live_offer().is_none());
    // Late accept → stale
    let events = session.tick(accept_input(me), &[], 200);
    assert!(events.iter().any(|e| matches!(e, DispatchEvent::StaleAccept { .. })));
}

// ── 7. Voice profile integration with surface states ────────────────────────

#[test]
fn voice_profile_tracks_all_surface_states() {
    // Idle → Balanced
    let surface = CourierSurface::new(ME);
    let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
    assert_eq!(input_profile_for(&state), InputProfile::Balanced);

    // Live with offer → Balanced (not in transit)
    let mut surface = CourierSurface::new(ME);
    surface.on_event(&DispatchEvent::Offered {
        courier: ME,
        deadline_ts: 500,
    });
    let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
    assert_eq!(input_profile_for(&state), InputProfile::Balanced);

    // Accepted but not in transit → Balanced
    let mut surface = CourierSurface::new(ME);
    surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 500 });
    surface.on_event(&DispatchEvent::Assigned { courier: ME });
    let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
    assert_eq!(input_profile_for(&state), InputProfile::Balanced);

    // In transit run → CourierInMotion
    let motion_state = CourierSurfaceState {
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
    assert_eq!(input_profile_for(&motion_state), InputProfile::CourierInMotion);
}

// ── 8. Render corpus integrity ──────────────────────────────────────────────

#[test]
fn render_corpus_passes_floor_parity() {
    let run = ActiveRun {
        run_id: 1,
        claim_id: 1,
        order_id: 42,
        in_transit: true,
        track: Some(TrackFrame {
            est: 1000,
            v_mps: 5.0,
            eta_s: 120,
            remaining_m: 600.0,
            route_version: 1,
        }),
    };
    let corpus = courier_corpus(&run);
    assert_eq!(corpus.len(), 3, "corpus has duty, run, no-track");
    assert!(floor_parity_courier_corpus(&corpus), "floor parity must hold");
}

// ── 9. Payout presentation (TweenGuard money law) ──────────────────────────

#[test]
fn render_payout_law_holds() {
    // Integer payout is accepted
    assert_eq!(present_payout(500.0), Ok(500));
    assert_eq!(present_payout(0.0), Ok(0));

    // Fractional payout is rejected (tweening RED)
    assert!(present_payout(500.5).is_err());
    assert!(present_payout(-1.5).is_err());
}

// ── 10. Voice classify uses real signal processing for spectral features ────

#[test]
fn voice_classify_all_recognized_commands() {
    for word in &["accept", "yes", "confirm", "skip", "decline", "no"] {
        let phrase = VoicePhrase {
            transcript: (*word).into(),
            confidence: 1.0,
            is_final: true,
        };
        assert_eq!(
            classify(&phrase),
            Classification::Resolved(Intent::Command),
            "keyword '{word}' must classify as Command"
        );
    }
}

#[test]
fn voice_classify_navigation_keywords() {
    for word in &["navigate", "status", "where", "directions"] {
        let phrase = VoicePhrase {
            transcript: (*word).into(),
            confidence: 1.0,
            is_final: true,
        };
        assert_eq!(
            classify(&phrase),
            Classification::Resolved(Intent::Navigate),
            "keyword '{word}' must classify as Navigate"
        );
    }
}

#[test]
fn voice_arbitrary_text_is_rejected() {
    for word in &["hello", "goodbye", "thanks", ""] {
        let phrase = VoicePhrase {
            transcript: (*word).into(),
            confidence: 1.0,
            is_final: true,
        };
        assert_eq!(
            classify(&phrase),
            Classification::Rejected,
            "non-command '{word}' must be Rejected"
        );
    }
}

#[test]
fn voice_respects_confidence_threshold() {
    let phrase = VoicePhrase {
        transcript: "accept".into(),
        confidence: 0.3,
        is_final: true,
    };
    assert_eq!(classify(&phrase), Classification::Rejected);
}

#[test]
fn voice_respects_is_final() {
    let phrase = VoicePhrase {
        transcript: "accept".into(),
        confidence: 0.95,
        is_final: false,
    };
    assert_eq!(classify(&phrase), Classification::Rejected);
}

// ── 11. Types size contracts verify across pipeline ─────────────────────────

#[test]
fn pipeline_types_are_compact() {
    assert_eq!(std::mem::size_of::<CourierShell>(), 1);
    assert_eq!(std::mem::size_of::<GeoRef>(), 8);
    assert_eq!(std::mem::size_of::<ZoneRef>(), 4);
    assert_eq!(std::mem::size_of::<CourierKey>(), 32);
    assert_eq!(std::mem::size_of::<DispatchInputKind>(), 1);
    assert_eq!(std::mem::size_of::<AdvanceReason>(), 1);
    assert_eq!(std::mem::size_of::<InputProfile>(), 1);
    assert_eq!(std::mem::size_of::<AiMode>(), 1);
    assert_eq!(std::mem::size_of::<HwClass>(), 1);
    assert_eq!(std::mem::size_of::<LiveOffer>(), 40);
}
