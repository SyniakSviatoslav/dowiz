//! `landing` — BLUEPRINT-P73 dowiz.org landing + signup surface (BLUEPRINT-P73, W3).
//!
//! dowiz.org is PURE infrastructure for **prospective venue owners** (operator ruling
//! §16.21): a product/demo page + self-serve signup, never a customer-facing discovery surface.
//! It lists NO vendors, no menus, no links to venues (§16.21 anti-scope §2.2), and it is a
//! **full-wgpu** page (§16.56 — no static-page exception). This module is the **kernel-side,
//! render-agnostic core** of that surface:
//!
//!   * `journey`  (M1) — the CLOSED `ClaimJourney` conversion FSM (pure, Lane A).
//!   * `form`     (M2) — the `SignupForm` submit-boundary validation (Lane A wiring;
//!                      the render layer feeds it P57 `TextField::value()` strings — RECONCILE-P57).
//!   * `claim_client` (M3) — the `ClaimServicePort` client leg + a mock adapter. P73 implements
//!                      NO claim logic, mints NO certs, touches NO warm pool — P67 owns all that
//!                      (closed dowiz infra, §16.54). P73 is a client of P67's API.
//!   * the `SemanticScene` authoring (M5, Lane A) — a per-journey-step a11y scene the render
//!                      layer projects; the wgpu hero render is Lane B (O18a), `#[ignore="O18a"]`.
//!   * the opaque-cert handoff (M6, Lane A) — `CertHandoffSink` forwards the `OwnerRootCert`
//!                      exactly once and retains no copy; the real sign/verify is P59/P66/P70.
//!
//! The landing bot-pack (M4, the marketing-schema JSON-LD / robots / sitemap / OG / manifest /
//! llms.txt) lives in `json_api.rs` (the shared `json-api` boundary) — added beside P69's
//! catalog pack, reusing the slug-agnostic pure fns.
//!
//! ANTI-SCOPE (load-bearing, §2.2 / §5.1): there is **no** `VendorId`, `slug`, `catalog`, `menu`,
//! `Restaurant`, `search`, or "near you" type or string anywhere in this module. The structural
//! grep-gate `kernel/tests/landing_no_vendor_catalog.rs` enforces that — see §6 not-done clause.
//! §16.21 is enforced by the *absence* of the types, not by a policy check.
//!
//! LANE SPLIT (§2.3): everything here is **Lane A — buildable TODAY, zero network**. The only
//! Lane-B (O18a / P67) seams are `#[ignore = "O18a"]` markers (the render + the real HTTP claim
//! transport), exactly as P38/P57 do.

pub mod claim_client;
pub mod form;
pub mod journey;

// ── constants (§3) ──────────────────────────────────────────────────────────
/// P67-owned endpoint for the fast-path claim (illustrative until P67 lands its wire —
/// `RECONCILE-P67`). P73 only ever POSTs to the `ClaimServicePort`; the real adapter swaps the
/// mock for an HTTP client once P67's wire format lands.
pub const CLAIM_ENDPOINT: &str = "https://claim.dowiz.org/v1/claim";
/// P67-owned endpoint for the slow-path interest registration (§16.32 non-mandatory path).
pub const INTEREST_ENDPOINT: &str = "https://claim.dowiz.org/v1/interest";
/// The PUBLIC hub-software repo (§16.32 — the product's source; AGPLv3+TM+DCO). NOT the landing
/// page's own repo (closed dowiz infra, §16.54). Exact public slug is operator-set (ADR-020-gated).
pub const HUB_SOURCE_URL: &str = "https://github.com/dowiz/hub";
/// The canonical dowiz.org origin (used to build the bot-pack's own fixed pages).
pub const CANONICAL_URL: &str = "https://dowiz.org";
/// Degrade-closed claim ceiling: after this many ms with no assignment, the UI offers the
/// interest path explicitly rather than hanging (§4.3).
pub const CLAIM_REQUEST_TIMEOUT_MS: u32 = 8000;
/// Field caps (P57 FIELD_MAX_BYTES scope — the landing form inherits the same bounds).
pub const CONTACT_MAX_BYTES: usize = 256;
pub const VENUE_NAME_MAX_BYTES: usize = 256;
pub const NOTES_MAX_BYTES: usize = 2048;
/// Client-side single-outstanding-claim cap (X11) — reuse `kernel/src/token_bucket.rs`.
/// One in-flight claim at a time per visitor; a double-click cannot drain two pool slots (§16.57).
pub const CLAIM_BUCKET_CAPACITY: f64 = 1.0;
pub const CLAIM_BUCKET_REFILL: f64 = 0.2; // ≈ one claim / 5 s — degrade-closed on burst

// Re-export the shared claim-service types so callers import from `landing::*`.
pub use claim_client::{
    ClaimClient, ClaimError, ClaimOutcome, ClaimRequest, ClaimServicePort, ClaimedHub, ChallengeToken,
    HubId, InterestAck, InterestSubmission, MockClaimService,
};
pub use form::{FormError, FormField, SignupForm};
pub use journey::{ClaimJourney, JourneyEvent, LandingStep};

// ── M5 (Lane A) — per-step semantic scene authoring (the a11y mirror source) ──
//
// The render layer (Lane B, O18a) projects this into the live a11y tree via P58's `a11yGate`
// (RECONCILE-P58 on the provisional ARIA-textbox convention). The scene is authored from the SAME
// `ClaimJourney` state the renderer consumes (X1 draft-parity by construction) — one source of
// truth for both the pixels and the accessibility tree.

/// One semantic node in the landing's accessibility tree (a projection primitive, not pixels).
#[derive(Clone, PartialEq, Debug)]
pub struct SemanticNode {
    /// ARIA role token (mirrors P58's `role_to_aria_token` canonical set): "region", "link",
    /// "textbox", "heading", "img".
    pub role: &'static str,
    /// Human label / accessible name.
    pub label: String,
    /// Current value (filled textbox content, link href, region description).
    pub value: String,
}

/// The whole landing a11y scene for one journey step.
#[derive(Clone, PartialEq, Debug)]
pub struct SemanticScene {
    pub step: LandingStep,
    pub nodes: Vec<SemanticNode>,
}

/// Build the per-step `SemanticScene` from the current `ClaimJourney`. Pure — one source of truth
/// shared by the renderer (Lane B) and the Playwright a11y-tree harness (§4.5).
pub fn landing_scene(journey: &ClaimJourney) -> SemanticScene {
    let step = journey.step();
    let mut nodes = Vec::new();
    // Hero is always present (the live field-engine demo — the product demoing itself, §1.2).
    nodes.push(SemanticNode {
        role: "img",
        label: "dowiz — sovereign delivery infra for venue owners".into(),
        value: "Live field-engine demonstration".into(),
    });
    // The GitHub CTA (§16.32) — a prominent, non-editable affordance linking the open hub software.
    nodes.push(SemanticNode {
        role: "link",
        label: "View the open-source hub software".into(),
        value: HUB_SOURCE_URL.into(),
    });
    // Secondary "install the app" CTA (§16.8 / §16.21) — also non-editable.
    nodes.push(SemanticNode {
        role: "link",
        label: "Install the dowiz client".into(),
        value: format!("{CANONICAL_URL}/install"),
    });
    match journey {
        ClaimJourney::FormEntry(f) | ClaimJourney::Submitting(f) => {
            nodes.push(SemanticNode {
                role: "textbox",
                label: "Contact (email or Telegram handle)".into(),
                value: f.contact.clone(),
            });
            nodes.push(SemanticNode {
                role: "textbox",
                label: "Venue name".into(),
                value: f.venue_name.clone(),
            });
            nodes.push(SemanticNode {
                role: "textbox",
                label: "Notes (optional)".into(),
                value: f.notes.clone(),
            });
        }
        ClaimJourney::Claimed(hub) => {
            nodes.push(SemanticNode {
                role: "heading",
                label: "Your hub is claimed".into(),
                value: hub.hub_url.clone(),
            });
        }
        ClaimJourney::InterestRegistered(ack) => {
            nodes.push(SemanticNode {
                role: "heading",
                label: "We'll be in touch".into(),
                value: ack.ack_id.clone(),
            });
        }
        _ => {}
    }
    SemanticScene { step, nodes }
}

#[cfg(test)]
mod tests;
