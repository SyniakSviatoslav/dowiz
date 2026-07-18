//! types.rs — BLUEPRINT P71 §2 predefined types & constants.
//!
//! All types are spec-named BEFORE implementation (standard item 4). This is
//! the **surface's** view of the cross-repo contracts it consumes; it mirrors
//! the shapes, it does NOT import the upstream crates (cross-repo boundary).

use dowiz_engine::semantics::Role;

/// Which shell the courier app runs in (P39-rev §1.2 / P63 SP-2 verdict).
/// The rider is MOBILE-PRIMARY → `TauriMobile` is the daily-use default (§16.8).
/// `WinitDesktop` is only the back-office/dispatcher variant. Both host a
/// FULL-WGPU surface (§16.30/§16.34) — NOT a lighter native UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourierShell {
    TauriMobile,
    WinitDesktop,
}

/// Coarse geographic reference for a pickup venue. P71 renders this; it computes
/// no routing (P51's lane — see `render::no_routing_code` grep gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GeoRef {
    pub lat_micro: i32,
    pub lon_micro: i32,
}

/// Coarse dropoff *area* (not a precise pin) — the pre-accept privacy boundary
/// (symmetric with P51's position-emit window: the precise pin unlocks on
/// accept). Carries no routing math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZoneRef {
    pub zone_id: u32,
}

/// SUPERSEDES P52 `ClaimCard` AND P52 `OFFER_DECISION_WINDOW_SECS`.
/// The offer and its deadline now come from P65's `DispatchEvent::Offered` —
/// the surface computes NO expiry of its own. `deadline_ts` is P65's hub-side
/// `now_ts + OFFER_TIMEOUT_SECS` (the ONLY expiry authority).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferCard {
    pub claim_id: u64,
    pub order_id: u64,
    /// P65 LiveOffer.deadline_ts — the ONLY expiry authority.
    pub deadline_ts: i64,
    /// pickup venue, for the voice read-back + map marker.
    pub pickup: GeoRef,
    /// COARSE dropoff area pre-accept; precise pin unlocks on accept.
    pub dropoff_coarse: ZoneRef,
    /// The ORDER's delivery fee (hub-derived), NOT a courier metric; rendered
    /// via `TweenGuard` money law (no count-up).
    pub payout_i64: i64,
}

/// The active delivery run (K3), consumed from P52's kernel fold. Rendered by the
/// run screen. The precise position track lives inside it (privacy window).
///
/// Contains `TrackFrame` (f32 fields) ⇒ `PartialEq` only (no `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRun {
    pub run_id: u64,
    pub claim_id: u64,
    pub order_id: u64,
    /// in-transit flag — drives `CourierInMotion` (R3). Fold-derived, never a
    /// stored "human" bit. A machine courier simply holds this flag the same way.
    pub in_transit: bool,
    /// the P51 `TrackFrame` is consumed verbatim by `render` when present
    /// (None ⇒ honest no-track state, R4 adversarial).
    pub track: Option<TrackFrame>,
}

/// P51 `TrackFrame` (P51:354) — consumed VERBATIM (two-consumers, one
/// implementation). P71 renders it; it computes no ETA/routing.
///
/// NOTE: `v_mps`/`remaining_m` are `f32`, so this type derives `PartialEq` only
/// (f32 does not implement `Eq`). That is sufficient for `assert_eq!`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TrackFrame {
    pub est: i64,           // epoch seconds of estimate
    pub v_mps: f32,         // ground speed m/s
    pub eta_s: i64,         // seconds to arrival
    pub remaining_m: f32,   // remaining distance metres
    pub route_version: u64, // P51 privacy: stale route_version ⇒ honest no-track
}

/// The surface's view of P65's driver. P71 CONSUMES `DispatchEvent` (inbound)
/// and EMITS `DispatchInput` (outbound). It mirrors the event shape, it does
/// NOT import the bebop2 dispatch crate (cross-repo boundary — same discipline
/// P52 uses for proto-cap events).
///
/// Contains `ActiveRun`/`OfferCard` (f32 via TrackFrame) ⇒ `PartialEq` only.
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceOfferState {
    /// no live offer — duty screen (K1).
    Idle,
    /// P65 Offered — render countdown to `card.deadline_ts` (R2).
    Live { card: OfferCard },
    /// P65 Advanced{TimedOut} | StaleAccept — offer gone, rendered honestly with
    /// NO penalty/metric shown (§4.1). `stale` is an OFFER property, not a
    /// courier counter.
    Passed { stale: bool },
    /// P65 Assigned — claim Offered→Claimed; P52's ActiveRun (K3).
    Accepted { run: ActiveRun },
}

/// Accept/decline are emitted ONLY from `Live` (type-level witness — an
/// accept/decline in any other state does not construct). Maps to P65
/// `DispatchInput`. NOT a `CommitToken` gate: accepting a delivery moves no
/// money (K7 cash amount is hub-derived, P52:394); `CommitToken` (P64/P60) is
/// money-only and unreachable from this surface (§4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchInputFrame {
    pub courier: CourierKey,
    pub kind: DispatchInputKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchInputKind {
    Accept,
    Decline,
}

/// P65 `CourierKey = [u8;32]` — a cert-holder, not a person (§17.6). P71 emits
/// `DispatchInput::Accept{courier}` that is byte-identical whether a human said
/// "accept" or an autonomous agent's controller emitted it.
pub type CourierKey = [u8; 32];

/// The last stretch of the offer window where the audible+visual urgency cue
/// rises, so a rider with eyes on the road HEARS the window closing. Reuses
/// P64's audio-channel equivalence (`AudioParams`) — same signal drives the
/// visual field intensity, parity-pinned (P2).
pub const OFFER_URGENCY_WINDOW_SECS: i64 = 10; // final 10s of P65's 30s window

/// The eyes-free accept: spoken read-back of the offer + an affirmation token
/// ("accept"/"yes"); "skip"/"decline"/silence-to-deadline = the safe pole (no
/// accept). Reuses P64's read-back+affirm shape for coordination reliability —
/// NOT because a `CommitToken` is required.
pub const AFFIRM_TIMEOUT_SECS: i64 = 4;

// ── R5 — battery DoD, SOURCED from P63 SP-5 (P63:170-173) — NOT invented here ──
// P63-VERDICTS.md is ABSENT this pass (SP-5 verdict = Blocked{physical budget
// Android}, P63:346). The gate is DEFINED now; its assertion is
// `#[ignore="P63-SP5-baseline"]` until SP-5 lands a real `VerdictRecord`.
pub const BATTERY_SHIFT_HOURS: f64 = 6.0; // P63 SHIFT_HOURS_SIM
pub const BATTERY_SETTLED_DRAIN_MAX_PCT_HR: f64 = 4.0; // P63 SETTLED_DRAIN_BAR_PCT_PER_HR
pub const BATTERY_SETTLE_SAVINGS_MIN_PCT: f64 = 30.0; // P63 SETTLE_SAVINGS_MIN_PCT
pub const BATTERY_THERMAL_SUSTAIN_MS: f64 = 33.0; // P63 THERMAL_SUSTAIN_MS

// ── R1 — render-correctness DoD, IMPORTED from P63 SP-6 (P63:368-374) ─────────
pub const FLOOR_PARITY_DELTA_MAX: f64 = 0.02; // P63 SP-6 PARITY_PERCEPTUAL_DELTA_MAX

// A11y-mirror role the blueprint requires on every K-screen (P58; §16.30
// "cover every screen"). This is the accesskit role the mirrored node carries.
pub const COURIER_A11Y_ROLE: Role = Role::Group;
