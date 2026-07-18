//! surface.rs — BLUEPRINT P71 R2: the `CourierSurface` state machine.
//!
//! The only mutator of the offer sub-state (besides accept/decline emission).
//! It consumes P65 `DispatchEvent` (inbound) and emits `DispatchInput` (the
//! `DispatchInputFrame` wire frame) — from a `Live` witness ONLY. The surface
//! owns NO offer timeout: with no courier action, it emits nothing; the state
//! advances only when P65's inbound `Advanced{TimedOut}` arrives.

use crate::dispatch::{accept_input, decline_input, DispatchEvent, DispatchInput};
use crate::types::{ActiveRun, CourierKey, DispatchInputFrame, DispatchInputKind, OfferCard, SurfaceOfferState};

/// The courier surface. Holds the offer sub-state plus the courier's own key.
/// The `state` is single-valued (at-most-one live offer — P52 no-batching law).
#[derive(Debug, Clone, PartialEq)]
pub struct CourierSurface {
    pub me: CourierKey,
    pub state: SurfaceOfferState,
}

impl CourierSurface {
    /// A fresh surface for `me`, at the duty screen (Idle).
    pub fn new(me: CourierKey) -> Self {
        Self {
            me,
            state: SurfaceOfferState::Idle,
        }
    }

    /// Apply one inbound P65 `DispatchEvent`. Returns the events the surface
    /// consumes (for test assertions). The ONLY transitions it performs:
    /// - `Offered`   ⇒ `Idle → Live`
    /// - `Assigned`  ⇒ `Live → Accepted` (if for `me`)
    /// - `Advanced`/`StaleAccept` ⇒ `Live → Passed`
    pub fn on_event(&mut self, ev: &DispatchEvent) -> Vec<SurfaceConsume> {
        let mut consumed = Vec::new();
        match ev {
            DispatchEvent::Offered { courier, deadline_ts } if *courier == self.me => {
                // Build the OfferCard from P65's deadline (the ONLY expiry authority).
                let card = OfferCard {
                    claim_id: 0,
                    order_id: 0,
                    deadline_ts: *deadline_ts,
                    pickup: Default::default(),
                    dropoff_coarse: Default::default(),
                    payout_i64: 0,
                };
                self.state = SurfaceOfferState::Live { card: card.clone() };
                consumed.push(SurfaceConsume::Live(card));
            }
            DispatchEvent::Assigned { courier } if *courier == self.me => {
                // Offered→Claimed. The run is the P52 ActiveRun (K3); here we
                // synthesize its shell from the live card (the kernel fold is the
                // substrate P71 renders — see §1.1).
                if let SurfaceOfferState::Live { card } = &self.state {
                    let run = ActiveRun {
                        run_id: card.claim_id,
                        claim_id: card.claim_id,
                        order_id: card.order_id,
                        in_transit: false,
                        track: None,
                    };
                    self.state = SurfaceOfferState::Accepted { run: run.clone() };
                    consumed.push(SurfaceConsume::Accepted(run));
                }
            }
            DispatchEvent::Advanced { from, .. } if *from == self.me => {
                // Offer passed (timed out / higher-ranked). `stale = false`.
                self.state = SurfaceOfferState::Passed { stale: false };
                consumed.push(SurfaceConsume::Passed { stale: false });
            }
            DispatchEvent::StaleAccept { courier } if *courier == self.me => {
                // A late accept ⇒ already Released. Rendered honestly as
                // "offer passed to another courier", exactly one `Assigned` ever.
                self.state = SurfaceOfferState::Passed { stale: true };
                consumed.push(SurfaceConsume::Passed { stale: true });
            }
            _ => { /* events for other couriers are ignored by this surface */ }
        }
        consumed
    }

    /// Emit `Accept{me}` — ONLY from `Live`. Consumes the `Live` witness so a
    /// second call on the same `Live` does not construct (no double-accept).
    /// Returns `None` unless the state is `Live` (type-level witness).
    pub fn emit_accept(&mut self) -> Option<DispatchInputFrame> {
        if let SurfaceOfferState::Live { .. } = &self.state {
            // Witness consumed: leaving Live means a second accept cannot build.
            self.state = SurfaceOfferState::Passed { stale: false };
            Some(DispatchInputFrame {
                courier: self.me,
                kind: DispatchInputKind::Accept,
            })
        } else {
            None
        }
    }

    /// Emit `Decline{me}` — ONLY from `Live` (same witness discipline).
    pub fn emit_decline(&mut self) -> Option<DispatchInputFrame> {
        if let SurfaceOfferState::Live { .. } = &self.state {
            self.state = SurfaceOfferState::Passed { stale: false };
            Some(DispatchInputFrame {
                courier: self.me,
                kind: DispatchInputKind::Decline,
            })
        } else {
            None
        }
    }

    /// Offline-island accept: render `pending-unconfirmed`. The state does NOT
    /// become `Accepted`; it stays `Live` until P65 confirms `Assigned` on
    /// rejoin (never a locally-fabricated claim).
    pub fn emit_accept_pending(&self) -> Option<DispatchInputFrame> {
        if let SurfaceOfferState::Live { .. } = &self.state {
            Some(DispatchInputFrame {
                courier: self.me,
                kind: DispatchInputKind::Accept,
            })
        } else {
            None
        }
    }

    /// Convenience: the `DispatchInput` P65 expects from a frame.
    pub fn to_dispatch_input(frame: &DispatchInputFrame) -> DispatchInput {
        match frame.kind {
            DispatchInputKind::Accept => accept_input(frame.courier),
            DispatchInputKind::Decline => decline_input(frame.courier),
        }
    }
}

/// What the surface did with a consumed event (for test assertions — event
/// sequence, not end-state).
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceConsume {
    Live(OfferCard),
    Accepted(ActiveRun),
    Passed { stale: bool },
}
