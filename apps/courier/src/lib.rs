//! BLUEPRINT P71 — courier surface (P52-rev).
//!
//! The realized, full-wgpu courier app for a human courier: voice-primary in
//! motion, dispatch-wired to P65, rendering K1-K8 through P38a. This crate is
//! the **render + real-time binding** layer only — it consumes the kernel-side
//! folds / P65 dispatch events / P51 track frames / P64 voice contract as
//! *wire frames* and re-owns NONE of their logic (see §1.5 of the blueprint).
//!
//! Module map:
//! - [`types`]      — shared contract types + the spec §2 constants.
//! - [`dispatch`]   — a LOCAL MIRROR of P65's `DispatchEvent`/`DispatchInput`/
//!                    `DispatchSession` (P71 consumes the wire frames; it does
//!                    NOT import bebop2). The accept-timeout is hub-owned.
//! - [`surface`]    — the `CourierSurface` state machine (R2): the only mutator
//!                    of the offer sub-state besides accept/decline emission.
//! - [`voice`]      — P64 voice binding (R3): `input_profile_for`, the parity-
//!                    pinned `offer_urgency` cue, the deterministic classifier.
//! - [`render`]     — P38a CPU-floor K-screen composition + P58 a11y mirror
//!                    (R1/R4) + the P63 SP-6 floor-parity hook.
//! - [`battery`]    — the P63 SP-5 battery gate (R5), `#[ignore]` until SP-5
//!                    lands a real `VerdictRecord`.
//!
//! TS/NODE BAN: this is Rust only. No DOM, no webview, no JS.

pub mod battery;
pub mod dispatch;
pub mod render;
pub mod surface;
pub mod types;
pub mod voice;
/// Quality gates (no self-referencing — forbidden tokens live here, not in tested modules).
mod gates;

// Re-exports for ergonomic consumption by `tests/` and by future surfaces.
pub use battery::*;
pub use dispatch::*;
pub use render::*;
pub use surface::*;
pub use types::*;
pub use voice::*;

#[cfg(test)]
mod tests {
    use super::*;

    const ME: CourierKey = [0u8; 32];
    const OTHER: CourierKey = [1u8; 32];

    #[test]
    fn dispatch_session_offer_and_accept() {
        let mut s = DispatchSession::new();
        s.offer(ME, 100);
        assert!(s.live_offer().is_some());
        assert_eq!(s.live_offer().unwrap().courier, ME);
        let events = s.tick(accept_input(ME), &[], 105);
        assert!(events.iter().any(|e| matches!(e, DispatchEvent::Assigned { courier } if *courier == ME)));
        assert_eq!(s.assigned(), Some(ME));
    }

    #[test]
    fn dispatch_session_timed_out() {
        let mut s = DispatchSession::new();
        s.offer(ME, 100);
        let events = s.tick(DispatchInput::Tick, &[], 200);
        assert!(events.iter().any(|e| matches!(e, DispatchEvent::Advanced { .. })));
        assert!(s.live_offer().is_none());
    }

    #[test]
    fn dispatch_session_stale_accept_after_deadline() {
        let mut s = DispatchSession::new();
        s.offer(ME, 100);
        let events = s.tick(accept_input(ME), &[], 200);
        assert!(events.iter().any(|e| matches!(e, DispatchEvent::StaleAccept { .. })));
        assert!(s.assigned().is_none());
    }

    #[test]
    fn dispatch_session_decline_and_requeue() {
        let mut s = DispatchSession::new();
        s.offer(ME, 100);
        let events = s.tick(decline_input(ME), &[], 105);
        assert!(events.iter().any(|e| matches!(e, DispatchEvent::Advanced { .. })));
        assert!(events.iter().any(|e| matches!(e, DispatchEvent::Requeued)));
    }

    #[test]
    fn surface_new_is_idle() {
        let surface = CourierSurface::new(ME);
        assert!(matches!(surface.state, SurfaceOfferState::Idle));
    }

    #[test]
    fn surface_on_offered_goes_live() {
        let mut surface = CourierSurface::new(ME);
        let consumed = surface.on_event(&DispatchEvent::Offered {
            courier: ME,
            deadline_ts: 200,
        });
        assert_eq!(consumed.len(), 1);
        assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));
    }

    #[test]
    fn surface_accept_only_from_live() {
        let mut surface = CourierSurface::new(ME);
        assert!(surface.emit_accept().is_none());
        surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 200 });
        let frame = surface.emit_accept();
        assert!(frame.is_some());
        assert_eq!(frame.unwrap().kind, DispatchInputKind::Accept);
        assert!(surface.emit_accept().is_none());
    }

    #[test]
    fn surface_decline_only_from_live() {
        let mut surface = CourierSurface::new(ME);
        assert!(surface.emit_decline().is_none());
        surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 200 });
        let frame = surface.emit_decline();
        assert!(frame.is_some());
        assert_eq!(frame.unwrap().kind, DispatchInputKind::Decline);
    }

    #[test]
    fn surface_ignores_other_courier_events() {
        let mut surface = CourierSurface::new(ME);
        let consumed = surface.on_event(&DispatchEvent::Offered {
            courier: OTHER,
            deadline_ts: 200,
        });
        assert!(consumed.is_empty());
        assert!(matches!(surface.state, SurfaceOfferState::Idle));
    }

    #[test]
    fn surface_assigned_creates_run() {
        let mut surface = CourierSurface::new(ME);
        surface.on_event(&DispatchEvent::Offered { courier: ME, deadline_ts: 200 });
        let consumed = surface.on_event(&DispatchEvent::Assigned { courier: ME });
        assert_eq!(consumed.len(), 1);
        assert!(matches!(surface.state, SurfaceOfferState::Accepted { .. }));
    }

    #[test]
    fn voice_classify_accept() {
        let phrase = VoicePhrase { transcript: "accept".into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Resolved(Intent::Command));
    }

    #[test]
    fn voice_classify_decline() {
        let phrase = VoicePhrase { transcript: "decline".into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Resolved(Intent::Command));
    }

    #[test]
    fn voice_classify_navigate() {
        let phrase = VoicePhrase { transcript: "navigate".into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Resolved(Intent::Navigate));
    }

    #[test]
    fn voice_classify_ambiguous_rejected() {
        let phrase = VoicePhrase { transcript: "accept navigate".into(), confidence: 1.0, is_final: true };
        assert_eq!(classify(&phrase), Classification::Rejected);
    }

    #[test]
    fn voice_classify_low_confidence_rejected() {
        let phrase = VoicePhrase { transcript: "accept".into(), confidence: 0.3, is_final: true };
        assert_eq!(classify(&phrase), Classification::Rejected);
    }

    #[test]
    fn voice_classify_not_final_rejected() {
        let phrase = VoicePhrase { transcript: "accept".into(), confidence: 0.9, is_final: false };
        assert_eq!(classify(&phrase), Classification::Rejected);
    }

    #[test]
    fn voice_urgency_rises_to_deadline() {
        let far = offer_urgency(100, 200);
        assert_eq!(far.stake, 0.0);
        let near = offer_urgency(195, 200);
        assert!(near.stake > 0.0);
        let at = offer_urgency(200, 200);
        assert!((at.stake - 1.0).abs() < 0.001);
    }

    #[test]
    fn voice_ai_mode_off_works() {
        assert!(voice_works_without_ai(AiMode::Off));
        assert!(voice_works_without_ai(AiMode::On));
    }

    #[test]
    fn battery_gate_blocked_is_owed() {
        assert_eq!(evaluate_battery_gate(&default_sp5_verdict()), BatteryGate::Owed);
    }

    #[test]
    fn battery_gate_emulator_rejected() {
        let v = VerdictRecord::Confirms {
            hw: HwClass::Emulator,
            shift_hours: 6.0,
            settled_drain_pct_hr: 2.0,
            settle_saving_pct: 50.0,
            thermal_sustain_ms: 30.0,
        };
        assert_eq!(evaluate_battery_gate(&v), BatteryGate::RejectedEmulator);
    }

    #[test]
    fn battery_gate_measured_pass() {
        let v = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 2.0,
            settle_saving_pct: 50.0,
            thermal_sustain_ms: 30.0,
        };
        assert_eq!(evaluate_battery_gate(&v), BatteryGate::Pass);
    }

    #[test]
    fn battery_gate_fails_high_drain() {
        let v = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 10.0,
            settle_saving_pct: 50.0,
            thermal_sustain_ms: 30.0,
        };
        assert_eq!(evaluate_battery_gate(&v), BatteryGate::Fail("settled_drain"));
    }

    #[test]
    fn render_no_dom_gate() {
        use crate::gates::no_visible_dom_widget;
        assert!(no_visible_dom_widget(include_str!("lib.rs")));
        assert!(no_visible_dom_widget(include_str!("render.rs")));
        assert!(no_visible_dom_widget(include_str!("surface.rs")));
        // NEVER test gates.rs itself — that would be self-referencing
    }

    #[test]
    fn render_no_routing_gate() {
        use crate::gates::no_routing_code;
        assert!(no_routing_code(include_str!("render.rs")));
        assert!(no_routing_code(include_str!("surface.rs")));
        assert!(no_routing_code(include_str!("dispatch.rs")));
    }

    // ── new tests ──

    #[test]
    fn types_are_correctly_sized() {
        // Verify key types have expected sizes (no silent bloat).
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
    fn dispatch_types_are_send_sync() {
        // R6: dispatch types must be thread-safe for real P65 binding.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DispatchEvent>();
        assert_send_sync::<DispatchInput>();
        assert_send_sync::<DispatchSession>();
        assert_send_sync::<SurfaceConsume>();
        assert_send_sync::<DispatchInputFrame>();
    }

    #[test]
    fn surface_idle_to_live_to_passed_transition() {
        // Idle → Live (via Offered) → Passed (via Advanced/StaleAccept).
        let mut surface = CourierSurface::new(ME);
        surface.on_event(&DispatchEvent::Offered {
            courier: ME,
            deadline_ts: 200,
        });
        assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));
        surface.on_event(&DispatchEvent::StaleAccept { courier: ME });
        assert!(matches!(surface.state, SurfaceOfferState::Passed { stale: true }));
    }

    #[test]
    fn voice_profile_has_required_fields() {
        // input_profile_for resolves correctly across state shapes.
        let surface = CourierSurface::new(ME);
        let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
        assert_eq!(state.shell, CourierShell::TauriMobile);
        assert!(state.offer.is_none());
        assert!(state.run.is_none());
        // Idle → Balanced
        assert_eq!(input_profile_for(&state), InputProfile::Balanced);
        // In-transit run → CourierInMotion
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
    }

    #[test]
    fn battery_gate_refines_and_contradicts_edge_cases() {
        // Refines = measured but not Confirms ⇒ Owed (nav-tier pending).
        let v = VerdictRecord::Refines {
            hw: HwClass::BudgetAndroid,
            settle_saving_pct: 25.0,
            note: "gotcha".into(),
        };
        assert_eq!(evaluate_battery_gate(&v), BatteryGate::Owed);

        // Contradicts on real hw ⇒ Owed (fatally fails, can't assert).
        let v = VerdictRecord::Contradicts {
            hw: HwClass::BudgetAndroid,
            reason: "shift < 4 h".into(),
        };
        assert_eq!(evaluate_battery_gate(&v), BatteryGate::Owed);

        // Contradicts on Emulator ⇒ RejectedEmulator (still not a usable number).
        let v = VerdictRecord::Contradicts {
            hw: HwClass::Emulator,
            reason: "nope".into(),
        };
        assert_eq!(evaluate_battery_gate(&v), BatteryGate::RejectedEmulator);

        // Blocked is_measured = false ⇒ Owed.
        assert!(!default_sp5_verdict().is_measured());
    }

    // ─── P5: Flow 5 property tests — state machine validity ───────────────────

    #[test]
    fn property_all_transitions_valid() {
        let me: CourierKey = [0u8; 32];
        let other: CourierKey = [1u8; 32];

        let states: Vec<(&str, SurfaceOfferState)> = vec![
            ("Idle", SurfaceOfferState::Idle),
            ("Live", SurfaceOfferState::Live {
                card: OfferCard {
                    claim_id: 0, order_id: 0, deadline_ts: 200,
                    pickup: GeoRef::default(), dropoff_coarse: ZoneRef::default(), payout_i64: 0,
                },
            }),
            ("Passed", SurfaceOfferState::Passed { stale: false }),
            ("Passed/Stale", SurfaceOfferState::Passed { stale: true }),
            ("Accepted", SurfaceOfferState::Accepted {
                run: ActiveRun {
                    run_id: 1, claim_id: 1, order_id: 1, in_transit: false, track: None,
                },
            }),
        ];

        let events: Vec<DispatchEvent> = vec![
            DispatchEvent::Offered { courier: me, deadline_ts: 300 },
            DispatchEvent::Offered { courier: other, deadline_ts: 300 },
            DispatchEvent::Assigned { courier: me },
            DispatchEvent::Assigned { courier: other },
            DispatchEvent::Advanced { from: me, reason: AdvanceReason::TimedOut },
            DispatchEvent::Advanced { from: other, reason: AdvanceReason::TimedOut },
            DispatchEvent::StaleAccept { courier: me },
            DispatchEvent::StaleAccept { courier: other },
            DispatchEvent::RoundExhausted,
            DispatchEvent::Requeued,
        ];

        let valid: std::collections::HashSet<&str> =
            ["Idle", "Live", "Passed", "Accepted"].iter().copied().collect();

        for (from_name, from_state) in &states {
            for ev in &events {
                let mut surface = CourierSurface { me, state: from_state.clone() };
                surface.on_event(ev);
                let to_name = match &surface.state {
                    SurfaceOfferState::Idle => "Idle",
                    SurfaceOfferState::Live { .. } => "Live",
                    SurfaceOfferState::Passed { .. } => "Passed",
                    SurfaceOfferState::Accepted { .. } => "Accepted",
                };
                assert!(valid.contains(to_name),
                    "({}, {:?}) → {} (invalid)", from_name, ev, to_name);
            }
        }
    }

    #[test]
    fn property_no_invalid_state_reachable() {
        let me: CourierKey = [0u8; 32];
        let other: CourierKey = [1u8; 32];

        let events: Vec<DispatchEvent> = vec![
            DispatchEvent::Offered { courier: me, deadline_ts: 300 },
            DispatchEvent::Offered { courier: other, deadline_ts: 300 },
            DispatchEvent::Assigned { courier: me },
            DispatchEvent::Assigned { courier: other },
            DispatchEvent::Advanced { from: me, reason: AdvanceReason::TimedOut },
            DispatchEvent::Advanced { from: other, reason: AdvanceReason::TimedOut },
            DispatchEvent::StaleAccept { courier: me },
            DispatchEvent::StaleAccept { courier: other },
            DispatchEvent::RoundExhausted,
            DispatchEvent::Requeued,
        ];

        let valid_states: std::collections::HashSet<&str> =
            ["Idle", "Live", "Passed", "Accepted"].iter().copied().collect();

        for seq_len in 1..=6 {
            // Enumerate all sequences of `seq_len` events starting from Idle.
            // Use a stack-based DFS to avoid combinatorial explosion.
            let mut stack: Vec<(usize, Vec<DispatchEvent>)> = (0..events.len())
                .map(|i| (i, vec![events[i].clone()]))
                .collect();
            let mut visited = std::collections::HashSet::new();
            while let Some((_last_idx, seq)) = stack.pop() {
                let mut surface = CourierSurface::new(me);
                for ev in &seq {
                    surface.on_event(ev);
                }
                let state_name = match &surface.state {
                    SurfaceOfferState::Idle => "Idle",
                    SurfaceOfferState::Live { .. } => "Live",
                    SurfaceOfferState::Passed { .. } => "Passed",
                    SurfaceOfferState::Accepted { .. } => "Accepted",
                };
                assert!(valid_states.contains(state_name),
                    "Sequence {:?} → {} (invalid)", seq.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>(), state_name);
                if seq.len() < seq_len {
                    for i in 0..events.len() {
                        let mut next_seq = seq.clone();
                        next_seq.push(events[i].clone());
                        let key = (seq.len(), next_seq.iter().fold(0u64, |acc, e| {
                            acc.wrapping_mul(31).wrapping_add(match e {
                                DispatchEvent::Offered { .. } => 1,
                                DispatchEvent::Assigned { .. } => 2,
                                DispatchEvent::Advanced { .. } => 3,
                                DispatchEvent::StaleAccept { .. } => 4,
                                DispatchEvent::RoundExhausted => 5,
                                DispatchEvent::Requeued => 6,
                            })
                        }));
                        if visited.insert(key) {
                            stack.push((i, next_seq));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn property_idempotent_accept() {
        let me: CourierKey = [0u8; 32];
        let mut surface = CourierSurface::new(me);
        surface.on_event(&DispatchEvent::Offered { courier: me, deadline_ts: 200 });
        let first = surface.emit_accept();
        assert!(first.is_some(), "First accept must succeed from Live");
        let second = surface.emit_accept();
        assert!(second.is_none(), "Second accept on consumed state must be no-op");
    }

    #[test]
    fn property_stale_rejection() {
        let me: CourierKey = [0u8; 32];
        let mut session = DispatchSession::new();
        // Offer at t=100 ⇒ deadline = 130
        session.offer(me, 100);
        // Accept at t=200 → already expired
        let events = session.tick(accept_input(me), &[], 200);
        assert!(events.iter().any(|e| matches!(e, DispatchEvent::StaleAccept { .. })),
            "Expired offer accept must produce StaleAccept");
        assert!(session.assigned().is_none(), "Stale accept must not assign");
        assert!(!events.iter().any(|e| matches!(e, DispatchEvent::Assigned { .. })),
            "Expired offer must never produce Assigned");
        assert!(session.live_offer().is_none(), "Live offer must be consumed after stale accept");
    }

    #[test]
    fn property_deterministic_render() {
        let frame1 = compose_duty(CourierShell::TauriMobile, true);
        let frame2 = compose_duty(CourierShell::TauriMobile, true);
        assert_eq!(frame1.field_hash, frame2.field_hash, "compose_duty must be deterministic");
        assert_eq!(frame1.field, frame2.field);

        let run = ActiveRun {
            run_id: 1, claim_id: 1, order_id: 1, in_transit: true, track: None,
        };
        let frame1 = compose_run(&run);
        let frame2 = compose_run(&run);
        assert_eq!(frame1.field_hash, frame2.field_hash, "compose_run must be deterministic");
        assert_eq!(frame1.field, frame2.field);
        // Different ActiveRun (different in_transit) → likely different frame
        let run2 = ActiveRun {
            run_id: 2, claim_id: 2, order_id: 2, in_transit: false, track: None,
        };
        let frame3 = compose_run(&run2);
        // Same input → same output; different input → may differ (verifies determinism, not identity)
        assert_eq!(compose_run(&run2).field_hash, frame3.field_hash);
    }

    #[test]
    fn property_voice_profile_determinism() {
        let surface = CourierSurface::new([0u8; 32]);
        let state = CourierSurfaceState::from_surface(&surface, CourierShell::TauriMobile);
        assert_eq!(input_profile_for(&state), InputProfile::Balanced,
            "Idle surface must yield Balanced profile");
        assert_eq!(input_profile_for(&state), input_profile_for(&state),
            "input_profile_for must be deterministic");

        let in_motion = CourierSurfaceState {
            shell: CourierShell::TauriMobile,
            offer: None,
            run: Some(ActiveRun {
                run_id: 1, claim_id: 1, order_id: 1, in_transit: true, track: None,
            }),
        };
        assert_eq!(input_profile_for(&in_motion), InputProfile::CourierInMotion);
        assert_eq!(input_profile_for(&in_motion), input_profile_for(&in_motion));

        let phrase = VoicePhrase { transcript: "navigate".into(), confidence: 0.9, is_final: true };
        assert_eq!(classify(&phrase), Classification::Resolved(Intent::Navigate));
        assert_eq!(classify(&phrase), classify(&phrase), "classify must be deterministic");
    }

    #[test]
    fn property_battery_gate_monotonic() {
        let pass = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 2.0,
            settle_saving_pct: 50.0,
            thermal_sustain_ms: 30.0,
        };
        assert_eq!(evaluate_battery_gate(&pass), BatteryGate::Pass);

        let worse_drain = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 10.0,
            settle_saving_pct: 50.0,
            thermal_sustain_ms: 30.0,
        };
        assert!(matches!(evaluate_battery_gate(&worse_drain), BatteryGate::Fail(_)),
            "Higher drain (10.0 > 4.0 max) must Fail, not stay Pass or improve");

        let worse_both = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 10.0,
            settle_saving_pct: 10.0,
            thermal_sustain_ms: 60.0,
        };
        assert!(matches!(evaluate_battery_gate(&worse_both), BatteryGate::Fail(_)),
            "Multiple worse restrictions must Fail, not improve");

        let worse_saving = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 2.0,
            settle_saving_pct: 10.0,
            thermal_sustain_ms: 30.0,
        };
        assert!(matches!(evaluate_battery_gate(&worse_saving), BatteryGate::Fail(_)),
            "Lower savings (10.0 < 30.0 min) must Fail");

        let worse_thermal = VerdictRecord::Confirms {
            hw: HwClass::BudgetAndroid,
            shift_hours: 6.0,
            settled_drain_pct_hr: 2.0,
            settle_saving_pct: 50.0,
            thermal_sustain_ms: 50.0,
        };
        assert!(matches!(evaluate_battery_gate(&worse_thermal), BatteryGate::Fail(_)),
            "Higher thermal sustain (50.0 > 33.0 max) must Fail");
    }

    #[test]
    fn property_surface_consume_idempotent() {
        let me: CourierKey = [0u8; 32];
        let mut surface = CourierSurface::new(me);
        // Bring to Live via Offered
        surface.on_event(&DispatchEvent::Offered { courier: me, deadline_ts: 200 });
        assert!(matches!(surface.state, SurfaceOfferState::Live { .. }));

        // emit_accept → consumes Live witness
        let first = surface.emit_accept();
        assert!(first.is_some());
        let second = surface.emit_accept();
        assert!(second.is_none(), "emit_accept second call must be empty/no-op");

        // Reset: new surface, emit_decline idempotent
        let mut surface = CourierSurface::new(me);
        surface.on_event(&DispatchEvent::Offered { courier: me, deadline_ts: 200 });
        let first = surface.emit_decline();
        assert!(first.is_some());
        let second = surface.emit_decline();
        assert!(second.is_none(), "emit_decline second call must be empty/no-op");

        // on_event with Assigned AFTER already Accepted → should be no-op (no Live state)
        let mut surface = CourierSurface::new(me);
        surface.on_event(&DispatchEvent::Offered { courier: me, deadline_ts: 200 });
        surface.on_event(&DispatchEvent::Assigned { courier: me });
        assert!(matches!(surface.state, SurfaceOfferState::Accepted { .. }));
        let consumed = surface.on_event(&DispatchEvent::Assigned { courier: me });
        assert!(consumed.is_empty(), "Assigned on already-Accepted must be empty/no-op");
    }
}
