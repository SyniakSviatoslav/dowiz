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
}
