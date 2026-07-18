//! voice.rs — BLUEPRINT P71 R3: P64 voice binding (voice-primary in motion).
//!
//! P71 sets `InputProfile::CourierInMotion` while an `ActiveRun` is in transit
//! (it BIASES toward voice without disabling taps, P64 §3.1). Idle / AtPickup /
//! Reviewing keep `Balanced` (equal channels, §16.50). The urgency cue reuses
//! P64's audio-channel equivalence (`AudioParams`) — the SAME signal drives the
//! visual field intensity, parity-pinned (P2). Voice is AiMode-independent
//! (P64:448 — a no-AI venue's courier still has voice).

/// P64 `InputProfile` (P64:97-98). `CourierInMotion` biases toward voice
/// WITHOUT disabling other channels. `Balanced` = equal channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProfile {
    /// equal channels — idle / at-pickup / reviewing-the-day.
    Balanced,
    /// voice-biased, hands-busy/eyes-on-road — set ONLY while in transit.
    CourierInMotion,
    /// hands-free mode (e.g. at a red light, fully stopped) — still voice-first.
    HandsFree,
}

/// P64 `AudioParams` (P64:205) — the parity-pinned audio signal. The SAME
/// `stake` drives the visual field intensity, so audio and visual cannot drift
/// (P2). `tremolo_hz` climbs as the offer deadline approaches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioParams {
    pub pitch_hz: f64,
    pub tremolo_hz: f64,
    pub hold_ms: f64,
    /// the single shared stake (0.0 = calm, 1.0 = urgent). Parity-pinned: both
    /// the audio cue and the visual field intensity are derived FROM this.
    pub stake: f64,
}

/// P64 `RawInput::VoicePhrase` (P64:119) — a recognized utterance.
// NOTE: `confidence: f64` ⇒ derives `PartialEq` only (f64 is not `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct VoicePhrase {
    pub transcript: String,
    pub confidence: f64,
    /// is this the final, settled recognition (vs. a partial hypothesis)?
    pub is_final: bool,
}

/// P64 `AiMode` (P64:448) — voice must WORK with `AiMode::Off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    On,
    Off,
}

/// P64 `Intent` (P64 §3) — the classified meaning of a phrase. P71 maps
/// offer/run intents to these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// accept an offer / a confirmation.
    Command,
    /// navigate / status request.
    Navigate,
}

/// P64 `Classification` (P64 §3) — the deterministic classifier's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    Resolved(Intent),
    /// ambiguous / low-confidence / consequential-but-unconfirmed ⇒ NEVER auto-
    /// accepts (P64 deterministic-classifier rule). The AI never resolves a
    /// consequential courier action.
    Rejected,
}

use crate::surface::CourierSurface;
use crate::types::{ActiveRun, CourierShell, SurfaceOfferState, OFFER_URGENCY_WINDOW_SECS};

/// The surface state P71 reasons about for the voice profile. Fold-derived.
#[derive(Debug, Clone, PartialEq)]
pub struct CourierSurfaceState {
    pub shell: CourierShell,
    pub offer: Option<OfferSnapshot>,
    pub run: Option<ActiveRun>,
}

/// Minimal snapshot of the live offer needed for voice decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfferSnapshot {
    pub deadline_ts: i64,
}

impl CourierSurfaceState {
    /// Build the voice-reasoning state from the surface + shell.
    pub fn from_surface(surface: &CourierSurface, shell: CourierShell) -> Self {
        let (offer, run) = match &surface.state {
            SurfaceOfferState::Live { card } => (
                Some(OfferSnapshot {
                    deadline_ts: card.deadline_ts,
                }),
                None,
            ),
            SurfaceOfferState::Accepted { run } => (None, Some(run.clone())),
            _ => (None, None),
        };
        Self { shell, offer, run }
    }
}

/// R3 — the active input profile. `CourierInMotion` is set ONLY while an
/// `ActiveRun` is in transit; `Balanced` when Idle / AtPickup / Reviewing
/// (§16.50 equal channels). Fold-derived from the run/claim state — never a
/// stored flag.
pub fn input_profile_for(state: &CourierSurfaceState) -> InputProfile {
    match &state.run {
        // In transit ⇒ voice-biased. A machine courier holds the same flag the
        // same way (species-agnostic, §17.6).
        Some(r) if r.in_transit => InputProfile::CourierInMotion,
        _ => InputProfile::Balanced,
    }
}

/// R3 — the parity-pinned urgency cue. As `now_ts → deadline_ts` in the final
/// `OFFER_URGENCY_WINDOW_SECS`, the `stake` (and with it `tremolo_hz`) rises.
/// The EXACT same `stake` drives the visual field intensity (see
/// `render::offer_field_intensity`), so the two channels cannot drift (P2).
pub fn offer_urgency(now_ts: i64, deadline_ts: i64) -> AudioParams {
    let remaining = (deadline_ts - now_ts).max(0) as f64;
    let window = OFFER_URGENCY_WINDOW_SECS.max(1) as f64;
    // stake ∈ [0,1]: 0 when far from the window, climbing to 1 at the deadline.
    let stake = if remaining >= window {
        0.0
    } else {
        (1.0 - remaining / window).clamp(0.0, 1.0)
    };
    AudioParams {
        pitch_hz: 220.0 + 110.0 * stake,
        // tremolo strictly increases with stake (r3_urgency_cue_rises_to_deadline).
        tremolo_hz: 2.0 + 18.0 * stake,
        hold_ms: 200.0,
        stake,
    }
}

/// The visual field intensity that MUST track `offer_urgency().stake` (P2
/// parity). Returns the same stake so a test can assert audio == visual.
pub fn offer_field_intensity(now_ts: i64, deadline_ts: i64) -> f64 {
    offer_urgency(now_ts, deadline_ts).stake
}

/// R3 — deterministic phrase classifier (P64 §3). Maps a spoken phrase to an
/// `Intent`, but REFUSES ambiguity/consequential actions (never auto-accepts).
///
/// - "accept"/"yes" ⇒ `Command` (affirmation — the safe-pole's opposite).
/// - "skip"/"decline"/"no" ⇒ `Command` (the safe pole; P65 advances).
/// - "navigate"/"status"/"where" ⇒ `Navigate`.
/// - anything that matches BOTH a command and a nav keyword, OR is not final /
///   low-confidence ⇒ `Rejected` (never auto-accepts — P64 §3 rule).
pub fn classify(phrase: &VoicePhrase) -> Classification {
    if !phrase.is_final || phrase.confidence < 0.5 {
        return Classification::Rejected;
    }
    let t = phrase.transcript.trim().to_ascii_lowercase();
    let is_accept = matches!(t.as_str(), "accept" | "yes" | "confirm");
    let is_decline = matches!(t.as_str(), "skip" | "decline" | "no");
    let is_nav = matches!(t.as_str(), "navigate" | "status" | "where" | "directions");

    // Ambiguous: matches both a consequential accept AND a nav command ⇒ reject.
    if (is_accept || is_decline) && is_nav {
        return Classification::Rejected;
    }
    if is_accept || is_decline {
        return Classification::Resolved(Intent::Command);
    }
    if is_nav {
        return Classification::Resolved(Intent::Navigate);
    }
    Classification::Rejected
}

/// R3 — voice survives AiMode::Off (P64:448). This is a compile-time proof:
/// `classify`/`offer_urgency`/`input_profile_for` take NO `AiMode` parameter,
/// so gating voice on AiMode is unrepresentable. We keep the type to document
/// the invariant and expose a trivial checker used by tests.
pub fn voice_works_without_ai(_mode: AiMode) -> bool {
    // The classifier above is AiMode-independent by construction.
    true
}
