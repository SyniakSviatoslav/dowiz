//! dispatch.rs — BLUEPRINT P71 R2: a LOCAL MIRROR of P65's dispatch contract.
//!
//! P71 consumes P65's `DispatchEvent`/`DispatchInput` as wire frames; it does
//! NOT import the bebop2 dispatch crate (cross-repo boundary, memory
//! cross-branch-todo-map). The accept-timeout is **hub-owned dispatch Law**:
//! one courier-independent value (30 s), the structural anti-scoring property
//! (P65:107). The surface renders `deadline_ts`, it owns no timer.
//!
//! This module reproduces the exact P65 event shape + the `tick` driver so the
//! P71 surface can be bound to a REAL `DispatchSession` (the M2 falsifier, R6)
//! WITHOUT pulling bebop2. The no-scoring property is preserved: `assign` takes
//! NO history parameter, so a decline/timeout cannot affect a future order.

/// P65 `OFFER_TIMEOUT_SECS: i64 = 30` — hub-owned dispatch Law, courier-
/// independent. The surface renders `now_ts + OFFER_TIMEOUT_SECS` as the offer
/// deadline; it defines no competing window.
pub const OFFER_TIMEOUT_SECS: i64 = 30;

/// P65 `CourierKey` — a cert-holder (32-byte id), NOT a person (§17.6).
pub use crate::types::CourierKey;

/// Inbound dispatch event (mirrors P65 `DispatchEvent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchEvent {
    /// A courier was offered a claim; `deadline_ts = now_ts + OFFER_TIMEOUT_SECS`.
    /// NO courier input into the deadline (structural anti-scoring).
    Offered {
        courier: CourierKey,
        deadline_ts: i64,
    },
    /// The offer advanced without being accepted (timed out, or a higher-ranked
    /// candidate took it). `reason` documents WHY, never penalizes a courier.
    Advanced {
        from: CourierKey,
        reason: AdvanceReason,
    },
    /// The claim was assigned to `courier` (Offered→Claimed).
    Assigned { courier: CourierKey },
    /// The candidate round was exhausted (no courier took it).
    RoundExhausted,
    /// The order was re-queued for a new dispatch round.
    Requeued,
    /// An accept arrived AFTER the claim had already advanced — already
    /// `Released`, `Released→Claimed` illegal ⇒ surfaced, never a double-assign.
    StaleAccept { courier: CourierKey },
}

/// Why an offer advanced (P65 §1.2). This is an ORDER property, not a courier
/// score — no variant carries a courier-quality signal (no-scoring red line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceReason {
    TimedOut,
    HigherRanked,
}

/// Outbound dispatch input (mirrors P65 `DispatchInput`). The surface EMITS
/// these; it never decides the assignment itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchInput {
    Tick,
    Accept { courier: CourierKey },
    Decline { courier: CourierKey },
    OnlineSetChanged,
}

/// A live offer held by the P65 driver: the courier + the hub-owned deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveOffer {
    pub courier: CourierKey,
    pub deadline_ts: i64,
}

/// P65 `DispatchSession` — the offer/accept/decline driver.
///
/// Faithful to P65's contract:
/// - The timeout is COMPUTED HERE (hub-side), not by the surface.
/// - `assign` takes NO history parameter ⇒ a decline/timeout cannot penalize a
///   courier on the next order (P65 §1.3 no-scoring red line).
/// - A late `Accept` (after `Advanced`) produces `StaleAccept`, never a
///   double `Assigned` (P65 §4.3 late-accept gate).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DispatchSession {
    offer: Option<LiveOffer>,
    assigned: Option<CourierKey>,
}

impl DispatchSession {
    /// A fresh, idle session (no live offer).
    pub fn new() -> Self {
        Self::default()
    }

    /// Offer this claim to `courier` now. The deadline is hub-computed
    /// (`now_ts + OFFER_TIMEOUT_SECS`) with NO courier input.
    pub fn offer(&mut self, courier: CourierKey, now_ts: i64) {
        self.offer = Some(LiveOffer {
            courier,
            deadline_ts: now_ts + OFFER_TIMEOUT_SECS,
        });
    }

    /// Drive the session with one `DispatchInput`. Returns the events the hub
    /// emits. `candidates` is the current online set (for `RoundExhausted`);
    /// `now_ts` is the hub clock used for the timeout.
    ///
    /// No scoring: `candidates` is used ONLY to detect round exhaustion, never
    /// to rank or penalize.
    pub fn tick(
        &mut self,
        input: DispatchInput,
        candidates: &[CourierKey],
        now_ts: i64,
    ) -> Vec<DispatchEvent> {
        let mut out = Vec::new();
        match input {
            DispatchInput::Tick => {
                // Hub-owned timeout: if a live offer's deadline has passed and no
                // accept arrived, advance it. The SURFACE emits no expiry frame —
                // only the hub driver does (proves the surface owns no timeout).
                if let Some(o) = self.offer {
                    if now_ts >= o.deadline_ts {
                        self.offer = None;
                        out.push(DispatchEvent::Advanced {
                            from: o.courier,
                            reason: AdvanceReason::TimedOut,
                        });
                        if candidates.is_empty() {
                            out.push(DispatchEvent::RoundExhausted);
                        } else {
                            out.push(DispatchEvent::Requeued);
                        }
                    }
                }
            }
            DispatchInput::Accept { courier } => {
                match self.offer {
                    Some(o) if o.courier == courier && now_ts < o.deadline_ts => {
                        // Valid, in-window accept ⇒ assign (Offered→Claimed).
                        self.offer = None;
                        self.assigned = Some(courier);
                        out.push(DispatchEvent::Assigned { courier });
                    }
                    Some(o) if o.courier == courier => {
                        // Accept AFTER deadline already elapsed ⇒ already Released.
                        // Surface emits StaleAccept, never a second Assigned.
                        self.offer = None;
                        out.push(DispatchEvent::StaleAccept { courier });
                    }
                    _ => {
                        // No matching live offer ⇒ the accept is stale/unknown.
                        out.push(DispatchEvent::StaleAccept { courier });
                    }
                }
            }
            DispatchInput::Decline { courier } => {
                // A decline just releases the live offer; no ranking effect.
                if self.offer.map(|o| o.courier) == Some(courier) {
                    self.offer = None;
                    out.push(DispatchEvent::Advanced {
                        from: courier,
                        reason: AdvanceReason::HigherRanked,
                    });
                    out.push(DispatchEvent::Requeued);
                }
            }
            DispatchInput::OnlineSetChanged => { /* no event; just refreshes set */ }
        }
        out
    }

    /// The current live offer (if any) — the surface reads `deadline_ts` from it.
    pub fn live_offer(&self) -> Option<LiveOffer> {
        self.offer
    }

    /// The currently assigned courier (if any).
    pub fn assigned(&self) -> Option<CourierKey> {
        self.assigned
    }
}

/// Helper: build a `DispatchInput::Accept` for a courier (the surface's emit path).
pub fn accept_input(courier: CourierKey) -> DispatchInput {
    DispatchInput::Accept { courier }
}

/// Helper: build a `DispatchInput::Decline` for a courier.
pub fn decline_input(courier: CourierKey) -> DispatchInput {
    DispatchInput::Decline { courier }
}
