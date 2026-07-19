//! `landing/form.rs` — M2 (BLUEPRINT-P73 §3 / §4.2).
//!
//! The interest/signup form. Every text field is a P57 `TextField` at the **render seam** (Lane B):
//! the landing's wgpu UI holds three P57 `TextField`s and, at submit time, populates `contact` /
//! `venue_name` / `notes` here via `TextField::value()`. There is **NO DOM `<input>`** anywhere
//! (§16.34 inherited). This module is the **submit-boundary contract + validation** (Lane A), so
//! the FSM logic is pure and testable without the render engine. RECONCILE-P57: the `TextField`
//! API is consumed at the render seam, not re-forked here.
//!
//! The `challenge` is an EDGE token (X11 / R2 §7) — produced at the Cloudflare edge / challenge
//! layer, NEVER embedded in the wgpu canvas (same DOM tension as X3). It is opaque to P73; the
//! claim service RE-verifies it (defense in depth) before touching a pool slot (§5.1).

use super::{ChallengeToken, CONTACT_MAX_BYTES, NOTES_MAX_BYTES, VENUE_NAME_MAX_BYTES};

/// The three signup fields (resolved Strings the render layer populates from P57 `TextField`s).
/// NOTE: we deliberately do NOT store a `TextField` here — that would pull the engine into the
/// kernel. The render layer owns the `TextField`s; P73 owns the *resolved contract* (§5.5).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SignupForm {
    /// How the operator follows up — email or Telegram handle (P73 sends it to P67, which notifies).
    pub contact: String,
    /// The prospective venue's name.
    pub venue_name: String,
    /// Optional free text (out-of-pool needs, questions).
    pub notes: String,
    /// Edge-verified anti-abuse token. `None` until the edge passes it (§5.1).
    pub challenge: Option<ChallengeToken>,
}

/// Which field a validation failure refers to (typed refusal — never a silent drop, §5.4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FormField {
    Contact,
    VenueName,
    Notes,
}

/// Submit-boundary validation error. Self-termination leg: refuse at the boundary with a typed
/// error; the journey stays in `FormEntry` (never silently drops the submission).
#[derive(Clone, PartialEq, Debug)]
pub enum FormError {
    /// `contact` is empty.
    MissingContact,
    /// `venue_name` is empty.
    MissingVenueName,
    /// A field exceeds its byte cap (P57 FIELD_MAX_BYTES scope).
    TooLong(FormField),
}

impl SignupForm {
    /// Validate at the submit boundary ONLY (§4.2). `contact` and `venue_name` are required and
    /// bounded; `notes` is optional but bounded. Returns `Ok(())` or a typed `FormError`.
    pub fn validate(&self) -> Result<(), FormError> {
        if self.contact.is_empty() {
            return Err(FormError::MissingContact);
        }
        if self.venue_name.is_empty() {
            return Err(FormError::MissingVenueName);
        }
        if self.contact.len() > CONTACT_MAX_BYTES {
            return Err(FormError::TooLong(FormField::Contact));
        }
        if self.venue_name.len() > VENUE_NAME_MAX_BYTES {
            return Err(FormError::TooLong(FormField::VenueName));
        }
        if self.notes.len() > NOTES_MAX_BYTES {
            return Err(FormError::TooLong(FormField::Notes));
        }
        Ok(())
    }

    /// Maps the resolved form into a `ClaimRequest` body. The render layer guarantees these are
    /// byte-for-byte `TextField::value()` reads (RECONCILE-P57). Panics-free: if `challenge` is
    /// absent the caller must arm it via the edge FIRST (see `journey::advance_with_service`).
    pub fn into_claim_request(&self, challenge: ChallengeToken) -> super::ClaimRequest {
        super::ClaimRequest {
            contact: self.contact.clone(),
            venue_name: self.venue_name.clone(),
            challenge,
        }
    }
}
