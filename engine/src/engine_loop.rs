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
        }
    }

    /// One engine frame: poll+classify+apply through `InputRouter::tick`.
    ///
    /// This is the production caller of `InputRouter::tick` (P64 §3.1 ONE code
    /// path). Returns the number of resolved intents applied this frame.
        pub fn frame(&mut self, surface: SurfaceId, profile: InputProfile) -> usize {
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
        n
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
