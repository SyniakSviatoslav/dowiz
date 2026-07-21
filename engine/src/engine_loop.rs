//! P64 §3.1 — ONE code path: sources → classify → apply, wired into the engine
//! run loop. This module is the **compile-real production caller** of
//! `InputRouter::tick` (the only production caller; the prior `#[cfg(test)]`
//! caller in `intent.rs` was the only one).
//!
//! The run loop drives, per frame:
//!   1) `InputRouter::tick` polls every [`InputSource`], classifies raw input,
//!      and emits resolved [`Intent`]s (the single seam — nothing downstream
//!      ever sees a raw event);
//!   2) each emitted `Intent` is applied to the [`WidgetStore`] presentation
//!      state (HOVER toggle, probe, impulse velocity, scrub register).
//!
//! **MONEY/PAYMENT SAFETY (P64 §3.1):** consequential commands
//! (`ConfirmOrder`/`AcceptOrder`/`CancelOrder`/`DeclineOrder`/`GoCheckout`)
//! are NEVER applied by this loop. They require the friction FSM
//! (`friction.rs`) — this presentation loop does not route them. The
//! `hover_count`/`scrub`/`probe` accessors let the engine read back only
//! presentation state.

use crate::intent::{
    InputProfile, InputRouter, InputSource, Intent, IntentContext, RawInput, SurfaceId,
};
use crate::widget_store::WidgetStore;

/// HOVER flag bit in `WidgetStore::flags` (presentation-only; see §3.1).
const FLAG_HOVER: u32 = 1;

/// Item 60 (gap G3) — the engine's named frame-budget constant, the SINGLE
/// authority site. 16_667 µs = 60 fps. This is a UX/product decision (see
/// blueprint §7 [OPERATOR]); the *mechanism* (one authority, pin test, breach
/// flag) is fixed regardless. No magic frame-time numbers are scattered in the
/// loop — every budget comparison reads this constant.
pub const FRAME_BUDGET_US: u64 = 16_667;

/// The engine run loop. Owns the [`InputRouter`] and the live [`WidgetStore`]
/// and exposes `frame()` — the production caller of `InputRouter::tick`.
pub struct EngineLoop {
    router: InputRouter,
    widgets: WidgetStore,
    /// Presentation scrub register (driven by `Intent::Scrub`). Never money.
    scrub_register: f32,
    /// Debug probe position driven by `Intent::Point` (presentation telemetry).
    probe_u: f32,
    probe_v: f32,
    /// Number of resolved intents emitted by the most recent `frame()`.
    last_intents: usize,
    /// Frame-time profiler (gap G3): last-frame cost + budget breach. Always
    /// compiled (cheap floor); fed by the wasm-safe `clock::now_micros`.
    frame_profiler: crate::bridge::FrameProfiler,
}

impl EngineLoop {
    pub fn new(router: InputRouter, widgets: WidgetStore) -> Self {
        EngineLoop {
            router,
            widgets,
            scrub_register: 0.0,
            probe_u: 0.0,
            probe_v: 0.0,
            last_intents: 0,
            frame_profiler: crate::bridge::FrameProfiler::default(),
        }
    }

    /// One engine frame: poll+classify+apply through `InputRouter::tick`.
    ///
    /// This is the production caller of `InputRouter::tick` (P64 §3.1 ONE code
    /// path). Returns the number of resolved intents applied this frame.
    ///
    /// Item 60 (gap G3): the frame body is bracketed by the wasm-safe
    /// `clock::now_micros`; the elapsed cost is compared against
    /// `FRAME_BUDGET_US` and recorded in the frame profiler (cheap floor always
    /// compiled; p50/p99 under `telemetry`). On wasm the clock returns `None`
    /// (named absence) so the loop stays untimed-but-accounted and never calls
    /// `Instant::now()` (which panics on `wasm32-unknown-unknown`).
    pub fn frame(&mut self, surface: SurfaceId, profile: InputProfile) -> usize {
        let t0 = crate::clock::now_micros();
        let ctx = IntentContext {
            widgets: &self.widgets,
            surface,
            profile,
        };
        let intents = self.router.tick(&ctx);
        let n = intents.len();
        for intent in intents {
            self.apply(intent);
        }
        self.last_intents = n;
        let t1 = crate::clock::now_micros();
        // Cost = Δ between the two clock reads. `None` on either side = untimed
        // (named absence) — never coerced to a fabricated `0`.
        let cost_us = match (t0, t1) {
            (Some(a), Some(b)) => Some(b.saturating_sub(a)),
            _ => None,
        };
        let breached = cost_us.map_or(false, |us| us > FRAME_BUDGET_US);
        self.frame_profiler.record_frame(cost_us, breached);
        n
    }

    /// Borrow the frame-time profiler (gap G3). The cheap floor (`last_frame_us`,
    /// `budget_breached`) is always populated; `frame_p50_us`/`frame_p99_us` are
    /// present only under the `telemetry` feature.
    pub fn frame_profiler(&self) -> &crate::bridge::FrameProfiler {
        &self.frame_profiler
    }

    /// Presentation-only intent application. **Money/payment intents are
    /// explicitly excluded** (P64 §3.1 safety core): consequential `Command`s
    /// are never applied to the store here — they require the friction FSM.
    fn apply(&mut self, intent: Intent) {
        match intent {
            Intent::Select(w) => {
                // Toggle HOVER on the hit widget (presentation-only flag).
                if let Some(idx) = self.index_of(w) {
                    self.widgets.flags[idx] |= FLAG_HOVER;
                }
            }
            Intent::Point(pos) => {
                // Move the debug probe (presentation telemetry only).
                self.probe_u = pos.u;
                self.probe_v = pos.v;
            }
            Intent::Impulse(pos, mag) => {
                // A fling imparts a velocity to the nearest widget as feedback.
                if let Some(idx) = self.nearest(pos.u, pos.v) {
                    self.widgets.vel_y[idx] = mag;
                }
            }
            Intent::Scrub(dx) => {
                self.scrub_register += dx;
            }
            Intent::Navigate(_) => {
                // Book-keeping navigation — no payment path is opened here.
            }
            Intent::Command(_cmd) => {
                // DO NOT route payment/consequential commands in this loop.
                // (P64 §3.1: consequential intents need the friction FSM.)
            }
        }
    }

    fn index_of(&self, id: u32) -> Option<usize> {
        self.widgets.id.iter().position(|&x| x == id)
    }

    fn nearest(&self, u: f32, v: f32) -> Option<usize> {
        let mut best = None;
        let mut best_d = f32::INFINITY;
        for i in 0..self.widgets.len() {
            let d = (self.widgets.pos_x[i] - u).powi(2) + (self.widgets.pos_y[i] - v).powi(2);
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        best
    }

    /// Presentation scrub register value.
    pub fn scrub(&self) -> f32 {
        self.scrub_register
    }

    /// Debug probe position (presentation telemetry).
    pub fn probe(&self) -> (f32, f32) {
        (self.probe_u, self.probe_v)
    }

    /// Count of widgets currently carrying the HOVER flag.
    pub fn hover_count(&self) -> usize {
        self.widgets
            .flags
            .iter()
            .filter(|&&f| f & FLAG_HOVER != 0)
            .count()
    }

    /// Borrow the live widget store (read-only presentation state).
    pub fn widgets(&self) -> &WidgetStore {
        &self.widgets
    }

    /// Raw `tick` output count from the last frame (falsifiable evidence the
    /// router ran and emitted intents this frame).
    pub fn last_intents(&self) -> usize {
        self.last_intents
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{InputProfile, InputRouter, InputSource, Intent, RawInput, SurfaceId};
    use crate::widget_store::WidgetStore;

    /// A source that emits one fixed raw input every poll (router polls once per
    /// tick — the standard `InputRouter::tick` contract).
    struct ConstSource(RawInput);
    impl InputSource for ConstSource {
        fn poll(&mut self) -> Option<RawInput> {
            Some(self.0.clone())
        }
    }

    fn loop_with_select() -> EngineLoop {
        let mut widgets = WidgetStore::new(8);
        widgets.id[7] = 7;
        widgets.pos_x[7] = 5.0;
        widgets.pos_y[7] = 5.0;
        widgets.size_w[7] = 2.0;
        widgets.size_h[7] = 2.0;
        let down = RawInput::Pointer {
            pos: crate::text_input::FieldPos {
                u: 5.0,
                v: 5.0,
                w: 0.0,
            },
            phase: crate::intent::PointerPhase::Down,
            vel: (0.0, 0.0),
        };
        let router = InputRouter::new(vec![Box::new(ConstSource(down))]);
        EngineLoop::new(router, widgets)
    }

    /// A source that busy-spins inside `poll()` long enough to blow the frame
    /// budget — a planted pathological input source (the frame body becomes slow).
    struct SlowSource;
    impl InputSource for SlowSource {
        fn poll(&mut self) -> Option<RawInput> {
            // Busy-spin long enough to exceed FRAME_BUDGET_US = 16.667 µs, using
            // real arithmetic the optimizer cannot elide so the frame's measured
            // cost genuinely blows the budget (we target ~5 ms).
            let mut acc: u64 = 0xdead_beef_cafe_babe;
            let t0 = crate::clock::now_micros();
            loop {
                for _ in 0..1_000_000 {
                    acc = acc.wrapping_mul(0x2545_f491_4f6c_dd1d).wrapping_add(1);
                }
                match (t0, crate::clock::now_micros()) {
                    // Spin past the 60-fps budget (FRAME_BUDGET_US = 16_667 µs);
                    // target ~20 ms so the measured frame cost genuinely breaches.
                    (Some(a), Some(b)) if b.saturating_sub(a) >= 20_000 => break,
                    (Some(_), Some(_)) => continue,
                    _ => break,
                }
            }
            std::hint::black_box(acc);
            None
        }
    }

    // ── Item 60 (gap G3) ORACLE: `FRAME_BUDGET_US` has exactly ONE authority
    //    site and its value is pinned (P3 rate discipline). 16_667 µs = 60 fps.
    #[test]
    fn frame_budget_constant_is_pinned_authority() {
        assert_eq!(FRAME_BUDGET_US, 16_667, "frame budget = 60 fps (16_667 µs)");
        // The loop compares against this single constant, never a magic number.
        assert!(FRAME_BUDGET_US > 0);
    }

    // ── Item 60 (gap G3) ORACLE (red→green): a planted SLOW frame (a pathological
    //    input source that spins past `FRAME_BUDGET_US`) is FLAGGED as a budget
    //    breach. The cheap floor (`budget_breached`) is always compiled, so this
    //    passes on the default build when the clock is live (native).
    #[test]
    fn planted_slow_frame_is_flagged_as_breach() {
        // Only assert a breach when the clock is live (native). On the wasm named-
        // absence path the cost is `None` and breach is false — that is the honest
        // contract, never a fabricated `0`.
        if crate::clock::now_micros().is_none() {
            return;
        }
        let widgets = WidgetStore::new(8);
        let router = InputRouter::new(vec![Box::new(SlowSource)]);
        let mut loop_ = EngineLoop::new(router, widgets);
        loop_.frame(SurfaceId(0), InputProfile::Balanced);
        assert!(
            loop_.frame_profiler().budget_breached,
            "a frame spun past FRAME_BUDGET_US MUST be flagged breached"
        );
        assert!(
            loop_.frame_profiler().last_frame_us.unwrap() > FRAME_BUDGET_US,
            "measured frame cost exceeds the budget"
        );
    }

    // ── Item 60 (gap G3): a normal fast frame is NOT flagged (the cheap floor
    //    records a real cost, breach=false, when timed).
    #[test]
    fn fast_frame_is_not_flagged() {
        let mut loop_ = loop_with_select();
        loop_.frame(SurfaceId(0), InputProfile::Balanced);
        // On native the clock is live; on wasm it is untimed (None) — both honest.
        let p = loop_.frame_profiler();
        if let Some(us) = p.last_frame_us {
            assert!(!p.budget_breached, "a fast frame must not breach");
            assert!(us <= FRAME_BUDGET_US, "fast frame under budget");
        }
    }

    // ── Item 60 (gap G3): the run loop still drives the router (pre-existing
    //    GREEN gate is preserved) — instrumentation did not change the contract.
    #[test]
    fn frame_still_routes_intent_into_store() {
        let mut loop_ = loop_with_select();
        loop_.frame(SurfaceId(0), InputProfile::Balanced);
        assert_eq!(
            loop_.last_intents(),
            1,
            "router::tick still emits the intent"
        );
        assert_eq!(loop_.hover_count(), 1, "Select(7) reached the store");
    }
}
