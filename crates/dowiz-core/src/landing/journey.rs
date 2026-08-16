//! `landing/journey.rs` — M1 (BLUEPRINT-P73 §3 / §4.1).
//!
//! The visitor → claimed-hub / interest **conversion FSM** — a CLOSED, deterministic state machine.
//! Tests assert on **event SEQUENCES**, not end-state (standard item 3). Mirrors P69's checkout
//! journey discipline but for ONE funnel.
//!
//! Load-bearing rule (§4.1): `PoolEmpty` is **NOT** a failure state. A visitor who hits an empty
//! warm pool is a *lead* — the journey deterministically routes to `register_interest` and lands in
//! `InterestRegistered`, never `Failed`. `Failed` is reserved for transport/challenge failure ONLY
//! (a recoverable, retryable state whose form value survives).

use super::form::SignupForm;
use super::{
    ClaimError, ClaimOutcome, ClaimRequest, ClaimServicePort, ClaimedHub, InterestAck,
    InterestSubmission,
};

/// The closed conversion states. `PoolEmpty` is intentionally NOT here (it is a claim *outcome*,
/// not a journey *state* — see `JourneyEvent::PoolEmptyFellBackToInterest`).
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimJourney {
    /// Landing — hero field-demo + narrative + GitHub CTA + "claim / try it".
    Landing,
    /// FormEntry — the signup form open; edge-challenge seam armed (or not yet).
    FormEntry(SignupForm),
    /// Submitting — request in flight (challenge verified at the edge FIRST).
    Submitting(SignupForm),
    /// Claimed — FAST PATH: warm-pool hub assigned, online, fixture-populated.
    Claimed(ClaimedHub),
    /// InterestRegistered — SLOW PATH: pool-empty / out-of-pool → operator notified.
    InterestRegistered(InterestAck),
    /// Failed — transport/challenge failure ONLY (recoverable, retryable).
    Failed { form: SignupForm, error: ClaimError },
}

/// The events the journey folds. One source feeds render AND a11y mirror (X1).
#[derive(Clone, PartialEq, Debug)]
pub enum JourneyEvent {
    Started,
    FormOpened,
    FieldEdited,
    SubmitRequested,
    ChallengePassed,
    ChallengeFailed,
    ClaimAssigned(ClaimedHub),
    PoolEmptyFellBackToInterest,
    InterestAcked(InterestAck),
    TransportFailed(ClaimError),
    Retried,
}

impl ClaimJourney {
    /// The current coarse step (used by the a11y scene + the render).
    pub fn step(&self) -> LandingStep {
        match self {
            ClaimJourney::Landing => LandingStep::Landing,
            ClaimJourney::FormEntry(_) | ClaimJourney::Submitting(_) => LandingStep::Form,
            ClaimJourney::Claimed(_) => LandingStep::Claimed,
            ClaimJourney::InterestRegistered(_) => LandingStep::Interest,
            ClaimJourney::Failed { .. } => LandingStep::Failed,
        }
    }

    /// Open the signup form (Landing/Claimed/Interest/Failed → FormEntry, carrying any prior form).
    /// The form value SURVIVES a `Failed` retry (§4.1) — it is never cleared on a recoverable error.
    pub fn open_form(&self) -> ClaimJourney {
        let f = self.form().cloned().unwrap_or_default();
        ClaimJourney::FormEntry(f)
    }

    /// The signup form, if any state carries one.
    pub fn form(&self) -> Option<&SignupForm> {
        match self {
            ClaimJourney::FormEntry(f) | ClaimJourney::Submitting(f) => Some(f),
            ClaimJourney::Failed { form: f, .. } => Some(f),
            _ => None,
        }
    }

    /// Arms the edge challenge. Only meaningful from `FormEntry`; elsewhere it is a no-op.
    pub fn with_challenge_passed(&self) -> ClaimJourney {
        match self {
            ClaimJourney::FormEntry(f) => ClaimJourney::FormEntry(f.clone()),
            other => other.clone(),
        }
    }

    /// Drive a submit. Returns:
    ///   * `Err(FormError)` if the boundary validation refuses (journey STAYS in `FormEntry`).
    ///   * `Ok(Submitting)` if valid — the caller then performs the network call via `fold_claim`.
    pub fn submit(&self) -> Result<ClaimJourney, super::form::FormError> {
        match self {
            ClaimJourney::FormEntry(f) => {
                f.validate()?; // self-termination leg: refuse at the boundary, never a silent drop
                Ok(ClaimJourney::Submitting(f.clone()))
            }
            _ => Ok(self.clone()),
        }
    }

    /// Fold a single journey event into the next state. Deterministic, pure. The claim **outcome**
    /// (fast/slow path) is resolved by `advance_with_service` (which talks to the `ClaimServicePort`).
    pub fn fold(&self, ev: JourneyEvent) -> ClaimJourney {
        match (self, ev) {
            (ClaimJourney::Landing, JourneyEvent::FormOpened) => self.open_form(),
            (ClaimJourney::FormEntry(_), JourneyEvent::FormOpened) => self.clone(),
            (_, JourneyEvent::FieldEdited) => self.clone(),
            // A failed challenge must NEVER leave the form and NEVER emit a claim request (§5.1).
            (ClaimJourney::FormEntry(f), JourneyEvent::ChallengeFailed) => {
                ClaimJourney::FormEntry(f.clone())
            }
            (ClaimJourney::FormEntry(f), JourneyEvent::ChallengePassed) => {
                ClaimJourney::FormEntry(f.clone())
            }
            // Retry from a recoverable failure returns to FormEntry WITH the same form value.
            (ClaimJourney::Failed { form: f, .. }, JourneyEvent::Retried) => {
                ClaimJourney::FormEntry(f.clone())
            }
            // A claim assignment (fast path) lands in Claimed.
            (ClaimJourney::Submitting(_), JourneyEvent::ClaimAssigned(h)) => {
                ClaimJourney::Claimed(h)
            }
            // Pool-empty deterministically falls to interest — NEVER to Failed.
            (ClaimJourney::Submitting(_), JourneyEvent::PoolEmptyFellBackToInterest) => {
                ClaimJourney::Submitting(SignupForm::default())
            }
            (ClaimJourney::Submitting(_), JourneyEvent::InterestAcked(ack)) => {
                ClaimJourney::InterestRegistered(ack)
            }
            (ClaimJourney::Submitting(_), JourneyEvent::TransportFailed(e)) => {
                let f = self.form().cloned().unwrap_or_default();
                ClaimJourney::Failed { form: f, error: e }
            }
            _ => self.clone(),
        }
    }

    /// Advance the `Submitting` state by calling the claim service. This is the ONLY place a claim
    /// request leaves the funnel (§5.1). Routes `PoolEmpty` → interest, `Timeout`/`Transport` →
    /// `Failed` (retryable). The `ClaimServicePort` is P67's service (a mock under Lane A).
    ///
    /// `sink` (M6) receives the opaque `OwnerRootCert` exactly once on a fast-path claim — the
    /// handoff forwards the bytes and retains no copy (forwarded-once by construction: we move the
    /// cert out of `ClaimedHub` into the sink, leaving the journey's view of it irrelevant).
    pub fn advance_with_service<S: ClaimServicePort, H: CertHandoffSink>(
        &self,
        svc: &S,
        sink: &mut H,
    ) -> ClaimJourney {
        let f = match self {
            ClaimJourney::Submitting(f) => f,
            _ => return self.clone(),
        };

        // Build the two requests from the form (byte-for-byte via TextField::value() at the render
        // seam — RECONCILE-P57; here we read the resolved Strings the render populated).
        let challenge = match &f.challenge {
            Some(c) => c.clone(),
            // No challenge token → the edge gate was never passed. This is the unrepresentable
            // branch: a claim MUST be challenged before it touches a pool slot. Without it we
            // cannot proceed, so we refuse (the render layer arms `challenge` before calling this).
            None => {
                return ClaimJourney::Failed {
                    form: f.clone(),
                    error: ClaimError::ChallengeRejected,
                }
            }
        };
        let req = ClaimRequest {
            contact: f.contact.clone(),
            venue_name: f.venue_name.clone(),
            challenge,
        };
        let interest = InterestSubmission {
            contact: f.contact.clone(),
            venue_name: f.venue_name.clone(),
            notes: f.notes.clone(),
            challenge: req.challenge.clone(),
        };

        match svc.claim_warm_pool_hub(req) {
            Ok(ClaimOutcome::Claimed(hub)) => {
                // M6: forward the opaque cert ONCE. An empty cert is a malformed/useless hub
                // (an unusable hub with no owner root) — treat it as a transport failure, never a
                // "successful claim with no credential" (§4.6 adversarial).
                if hub.owner_root_cert.0.is_empty() {
                    return ClaimJourney::Failed {
                        form: f.clone(),
                        error: ClaimError::Transport("claimed hub with empty owner root".into()),
                    };
                }
                sink.handoff(&hub.owner_root_cert.0);
                ClaimJourney::Claimed(hub)
            }
            Ok(ClaimOutcome::PoolEmpty) => {
                // SLOW PATH — deterministically call register_interest (§4.3). PoolEmpty ≠ Failed.
                match svc.register_interest(interest) {
                    Ok(ack) => ClaimJourney::InterestRegistered(ack),
                    Err(e) => ClaimJourney::Failed {
                        form: f.clone(),
                        error: e,
                    },
                }
            }
            Err(ClaimError::Timeout) | Err(ClaimError::Transport(_)) => ClaimJourney::Failed {
                form: f.clone(),
                error: ClaimError::Timeout,
            },
            Err(e) => ClaimJourney::Failed {
                form: f.clone(),
                error: e,
            },
        }
    }
}

/// The coarse landing step (for the a11y scene + render gating).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LandingStep {
    Landing,
    Form,
    Claimed,
    Interest,
    Failed,
}

/// M6 — the cert handoff sink. The landing forwards the opaque `OwnerRootCert` bytes exactly once
/// and retains no copy. The real custody sink is P66's wallet / P70's owner surface; the landing
/// is a terminal/leaf public surface and never parses, validates, or long-term-stores the cert.
pub trait CertHandoffSink {
    /// Forward the cert bytes exactly once. Implementors must NOT retain a copy beyond the call.
    fn handoff(&mut self, cert_bytes: &[u8]);
}
