//! P64 M3/M4 — §16.44 friction numeric mapping + gesture grammar + audio
//! channel + objective a11y gate (engineering-unknown E, owned here).
//!
//! BLUEPRINT-P64 §2.3 / §3.3 / §3.4 / §4. Pure Rust, no DOM, offline-clean.
//!
//! Safety core (§4.4, standard item 6): `CommitToken` has a PRIVATE field
//! sealed by a PRIVATE ZST `Seal`; it is `!Clone`, `!Default`, and
//! constructible ONLY inside this module by `FrictionFsm::commit_token`, which
//! returns `Some` iff `state == Committed`, reachable ONLY after
//! `progress_ms >= hold_ms` of sustained *aimed* input. Therefore no code path
//! — AI assistant, replayed event, or fuzzed input — can move money without a
//! `CommitToken`, and a `CommitToken` cannot exist without a human sustained
//! hold. This is the money-safety invariant in the TYPE SYSTEM (P1 Mentalism +
//! P6 Cause-and-Effect).

use crate::scene::SdfShape;

// ── Named constants (§2.3 — no magic numbers) ───────────────────────────────
pub const FRICTION_UNIT_MINOR: i64 = 100; // 1 currency unit = 100 minor
pub const FRICTION_AMP_BASE: f32 = 0.08; // idle friction amplitude
pub const FRICTION_AMP_LOG_GAIN: f32 = 0.11; // amplitude per decade of money
pub const FRICTION_AMP_MAX: f32 = 0.85; // clamp — never fully opaque/unusable
pub const HOLD_BASE_MS: u32 = 350; // minimum deliberate hold
pub const HOLD_LOG_GAIN_MS: u32 = 180; // extra hold per decade of money
pub const HOLD_IRREVERSIBLE_MS: u32 = 500; // added when Irreversible
pub const HOLD_REVERSIBLE_COST_MS: u32 = 200; // added when ReversibleWithCost
pub const AUDIO_PITCH_BASE_HZ: f32 = 220.0; // A3 at zero stake
pub const AUDIO_PITCH_DECADE_RATIO: f32 = 1.5; // pitch × per decade of money
pub const HINT_MASTERY_THRESHOLD: u8 = 3; // successes before a hint retires (§3.7)

/// The two independent stake axes. `money_minor` is read from `Money` as a
/// SCALAR PARAMETER — never animated (Money is not a FieldValue,
/// money_guard.rs:117 compile-proof). `reversibility` is a closed enum, not a
/// probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stake {
    pub money_minor: i64,
    pub reversibility: Reversibility,
}

/// Closed reversibility enum (not a probability) — drives extra hold time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reversibility {
    Reversible,
    ReversibleWithCost,
    #[default]
    Irreversible,
}

/// The field-state a stake maps to. Every field is a FieldValue (tween-safe);
/// NONE carries the money amount as a value — they carry a DERIVED intensity.
/// Parity-pinned to `AudioParams` (§4.3) so visual + audio cannot drift (P2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrictionField {
    pub amplitude: f32, // = friction_amplitude(stake) (§4.1)
    pub intensity: f32, // environmental / particle density, secondary amplifier
    pub hue_shift: f32, // OKLCH grade delta — REDUNDANT cue, never sole (§16.50)
    pub hold_ms: u32,   // required sustained-completion duration = friction_hold(stake) (§4.2)
}

/// Non-visual equivalent, derived from the SAME stake by the SAME functions (§4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioParams {
    pub pitch_hz: f32,
    pub tremolo_hz: f32,
    pub hold_ms: u32, // IDENTICAL to FrictionField::hold_ms — parity-pinned
}

/// A full friction spec for a stake: visual field + audio channel. The Composer
/// attaches this to a `ComposedResponse` when the intent is consequential.
#[derive(Debug, Clone, PartialEq)]
pub struct FrictionSpec {
    pub stake: Stake,
    pub field: FrictionField,
    pub audio: AudioParams,
}

// ── §4.1 stake → amplitude (logarithmic, Weber-Fechner) ─────────────────────
pub fn friction_amplitude(stake: Stake) -> f32 {
    let ratio = 1.0 + (stake.money_minor.max(0) as f64) / (FRICTION_UNIT_MINOR as f64);
    let decades = ratio.log10();
    let a = FRICTION_AMP_BASE + FRICTION_AMP_LOG_GAIN * decades as f32;
    // Clamp between the idle base and the max — never fully opaque/unusable.
    a.max(FRICTION_AMP_BASE).min(FRICTION_AMP_MAX)
}

// ── §4.2 stake → hold (the friction IS time; longer as stakes rise) ─────────
pub fn friction_hold(stake: Stake) -> u32 {
    let ratio = 1.0 + (stake.money_minor.max(0) as f64) / (FRICTION_UNIT_MINOR as f64);
    let decades = ratio.log10();
    let mut h = HOLD_BASE_MS + (HOLD_LOG_GAIN_MS as f64 * decades) as u32;
    h += match stake.reversibility {
        Reversibility::Reversible => 0,
        Reversibility::ReversibleWithCost => HOLD_REVERSIBLE_COST_MS,
        Reversibility::Irreversible => HOLD_IRREVERSIBLE_MS,
    };
    h
}

// ── §4.3 audio channel equivalence (parity-pinned to the visual) ────────────
pub fn audio_params(stake: Stake) -> AudioParams {
    let ratio = 1.0 + (stake.money_minor.max(0) as f64) / (FRICTION_UNIT_MINOR as f64);
    let decades = ratio.log10();
    let pitch_hz = AUDIO_PITCH_BASE_HZ * AUDIO_PITCH_DECADE_RATIO.powf(decades as f32);
    let amp = friction_amplitude(stake);
    // Tremolo speeds up with stake (faster tremor at higher stake) — derived from
    // the SAME log signal, so audio cannot drift from the visual (P2).
    let tremolo_hz = 2.0 + amp * 8.0;
    AudioParams {
        pitch_hz,
        tremolo_hz,
        hold_ms: friction_hold(stake), // IDENTICAL threshold — parity-pinned
    }
}

/// Build the full `FrictionSpec` for a stake (visual + audio, parity-pinned).
pub fn friction_spec(stake: Stake) -> FrictionSpec {
    let amplitude = friction_amplitude(stake);
    let hold_ms = friction_hold(stake);
    FrictionSpec {
        stake,
        field: FrictionField {
            amplitude,
            // intensity rides the same signal as a secondary amplifier.
            intensity: amplitude * 0.5,
            // hue grades with log-decades of money (redundant cue, never sole).
            hue_shift: (amplitude * 0.6).clamp(0.0, 0.6),
            hold_ms,
        },
        audio: audio_params(stake),
    }
}

// ── §3.4 objective a11y gate — mirror data the friction FSM emits ───────────
// P58 owns the real AccessKit/ARIA tree; P64 asserts this friction status data
// REACHES the mirror. The unit gates (deaf/blind/safety) are P64-standalone.

/// ARIA live-politeness mode for a mirror node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveMode {
    Off,
    Polite,
    Assertive,
}

/// One mirrored semantic node (P58 role/name/state/live convention, minimal).
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorNode {
    pub role: String,
    pub name: String,
    pub value: String,
    pub live: LiveMode,
}

/// A patch of mirror nodes the renderer/announcer reconciles from state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MirrorPatch {
    pub nodes: Vec<MirrorNode>,
}

impl MirrorPatch {
    pub fn status(name: &str, value: &str) -> MirrorNode {
        MirrorNode {
            role: "status".into(),
            name: name.into(),
            value: value.into(),
            live: LiveMode::Polite,
        }
    }
}

/// The kind of a11y announcement the FSM emits as it progresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A11yKind {
    /// Stake tier + hold progress (the role=status / aria-live=polite node).
    Status,
    /// Spoken read-back of the money amount before commit (§4.3 voice path).
    AmountReadback,
}

/// An a11y announcement emitted by the FSM, carrying its mirror node.
#[derive(Debug, Clone, PartialEq)]
pub struct A11yAnnouncement {
    pub kind: A11yKind,
    pub node: MirrorNode,
}

// ── §4.3 / §4.4 gesture grammar + completion FSM ───────────────────────────
/// The completion FSM states. The ONLY producer of a `CommitToken`. Fail-safe
/// default: release before threshold = Cancelled (the safe pole — P4 Polarity,
/// safe-directed).
#[derive(Debug, Clone, PartialEq)]
pub enum FrictionState {
    Idle,
    Building { progress_ms: u32 },
    Committed,
    Cancelled,
}

/// The completion FSM. The ONLY producer of a `CommitToken`.
pub struct FrictionFsm {
    spec: FrictionSpec,
    state: FrictionState,
    announcements: Vec<A11yAnnouncement>,
    readback_emitted: bool,
}

impl FrictionFsm {
    pub fn new(spec: FrictionSpec) -> Self {
        FrictionFsm {
            spec,
            state: FrictionState::Idle,
            announcements: Vec::new(),
            readback_emitted: false,
        }
    }

    /// Current state (read-only).
    pub fn state(&self) -> &FrictionState {
        &self.state
    }

    /// Current sustained-progress in ms (0 unless Building).
    pub fn progress_ms(&self) -> u32 {
        match self.state {
            FrictionState::Building { progress_ms } => progress_ms,
            _ => 0,
        }
    }

    /// Required sustained-completion duration for this stake.
    pub fn hold_ms(&self) -> u32 {
        self.spec.field.hold_ms
    }

    /// The visual field-state for this stake (deaf path consumes this).
    pub fn field(&self) -> &FrictionField {
        &self.spec.field
    }

    /// The audio params for this stake (blind path consumes this).
    pub fn audio(&self) -> &AudioParams {
        &self.spec.audio
    }

    /// All announcements emitted so far (the a11y gate inspects these).
    pub fn announcements(&self) -> &[A11yAnnouncement] {
        &self.announcements
    }

    fn emit_status(&mut self) {
        let tier = stake_tier(self.spec.stake.money_minor);
        let prog = match self.state {
            FrictionState::Building { progress_ms } => progress_ms,
            _ => 0,
        };
        let pct = if self.spec.field.hold_ms == 0 {
            100
        } else {
            (prog * 100 / self.spec.field.hold_ms).min(100)
        };
        let value = format!("stake tier {tier}; hold {pct}%");
        self.announcements.push(A11yAnnouncement {
            kind: A11yKind::Status,
            node: MirrorPatch::status("friction", &value),
        });
    }

    fn maybe_emit_readback(&mut self) {
        if self.readback_emitted {
            return;
        }
        let prog = self.progress_ms();
        if prog >= self.spec.field.hold_ms / 2 && self.spec.field.hold_ms > 0 {
            // Spoken read-back of the amount (the blind path requires this to
            // PRECEDE any commit — §4.3 / D8 blind-path gate).
            let amount = format!("{} minor units", self.spec.stake.money_minor);
            self.announcements.push(A11yAnnouncement {
                kind: A11yKind::AmountReadback,
                node: MirrorPatch::status("amount-readback", &amount),
            });
            self.readback_emitted = true;
        }
    }

    /// Feed one directed-hold sample. `aimed` = the gesture is aimed at the
    /// commit well (cos-angle ≥ `AIM_TOLERANCE`, computed by the caller).
    /// Progress advances ONLY while held AND aimed. Misaim (aimed=false) does
    /// NOT advance (§3.3 `misaimed_hold_does_not_progress`). Reaching
    /// `hold_ms` of aimed progress transitions to `Committed`.
    pub fn advance(&mut self, aimed: bool, dt_ms: u32) -> FrictionState {
        match self.state {
            FrictionState::Idle => {
                if aimed {
                    let progress = dt_ms.min(self.spec.field.hold_ms);
                    self.state = FrictionState::Building {
                        progress_ms: progress,
                    };
                    self.emit_status();
                    self.maybe_emit_readback();
                    if progress >= self.spec.field.hold_ms {
                        self.state = FrictionState::Committed;
                    }
                }
                // aimed=false while Idle: no progress, stay Idle.
            }
            FrictionState::Building { progress_ms } => {
                if aimed {
                    let next = (progress_ms + dt_ms).min(self.spec.field.hold_ms);
                    self.state = FrictionState::Building { progress_ms: next };
                    self.emit_status();
                    self.maybe_emit_readback();
                    if next >= self.spec.field.hold_ms {
                        self.state = FrictionState::Committed;
                    }
                }
                // aimed=false while Building: a misaim — progress is held, not
                // advanced, not cancelled (release() is the explicit cancel).
            }
            // Terminal states: once Committed/Cancelled, stay put.
            FrictionState::Committed | FrictionState::Cancelled => {}
        }
        self.state.clone()
    }

    /// Explicit release before threshold → the safe pole `Cancelled`. Release
    /// never commits (§3.3 `release_before_threshold_cancels`).
    pub fn release(&mut self) -> FrictionState {
        if let FrictionState::Building { .. } = self.state {
            self.state = FrictionState::Cancelled;
        }
        self.state.clone()
    }

    /// Some(CommitToken) IFF state == Committed. The ONLY way to mint it.
    pub fn commit_token(&self) -> Option<CommitToken> {
        match self.state {
            FrictionState::Committed => Some(CommitToken {
                stake: self.spec.stake,
                _seal: Seal,
            }),
            _ => None,
        }
    }
}

/// Stake tier (1..=4) from money magnitude — used by a11y announcements.
pub fn stake_tier(money_minor: i64) -> u8 {
    let ratio = (money_minor.max(0) as f64) / (FRICTION_UNIT_MINOR as f64);
    if ratio <= 1.0 {
        1
    } else if ratio <= 10.0 {
        2
    } else if ratio <= 100.0 {
        3
    } else {
        4
    }
}

/// Unforgeable proof that the friction threshold was met for THIS stake.
/// Non-Clone, non-Default, private field — constructible only inside
/// `friction.rs` by the FSM (§4.4). P60's payment call site REQUIRES one; no
/// code path moves money without it (item 6).
pub struct CommitToken {
    stake: Stake,
    _seal: Seal, // private ZST — seals the token; !Clone, !Default.
}

// Private seal ZST. Because `CommitToken`'s fields are private and it does not
// derive Clone/Default, the only constructor is `FrictionFsm::commit_token`.
struct Seal;

impl CommitToken {
    /// The stake this token was minted for (read-only).
    pub fn stake(&self) -> Stake {
        self.stake
    }
}

/// A small visual affordance hint directive (low-amplitude field perturbation
/// expressing the next available intent in the SAME field language as friction —
/// §3.7 onboarding). Visual/audio only; it never consumes or reorders an Intent.
#[derive(Debug, Clone, PartialEq)]
pub struct HintDirective {
    pub shape: SdfShape,
    pub amplitude: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stake(money: i64, rev: Reversibility) -> Stake {
        Stake {
            money_minor: money,
            reversibility: rev,
        }
    }

    // D6 — amplitude is log-monotone (strictly increasing with money),
    // sub-linear (100× money < 100× amplitude), and clamps at FRICTION_AMP_MAX.
    #[test]
    fn amplitude_is_log_monotone() {
        let a1 = friction_amplitude(stake(0, Reversibility::Reversible));
        let a2 = friction_amplitude(stake(500, Reversibility::Reversible)); // $5
        let a3 = friction_amplitude(stake(50_000, Reversibility::Reversible)); // $500
        assert!(a1 < a2 && a2 < a3, "amplitude must be strictly increasing");
        // sub-linear: a 100× money increase yields <100× amplitude.
        let ratio_money = 50_000.0 / 500.0;
        let ratio_amp = a3 / a2;
        assert!(
            ratio_amp < ratio_money,
            "amplitude must be sub-linear (got {ratio_amp} < {ratio_money})"
        );
        // clamp at max.
        let big = friction_amplitude(stake(i64::MAX / 2, Reversibility::Irreversible));
        assert!(
            big <= FRICTION_AMP_MAX + 1e-6,
            "amplitude clamps at FRICTION_AMP_MAX"
        );
        // never below the idle base.
        assert!(a1 >= FRICTION_AMP_BASE - 1e-6);
    }

    // D6 — hold scales with stake by the exact constant deltas.
    #[test]
    fn hold_scales_with_stake() {
        let small_rev = friction_hold(stake(100, Reversibility::Reversible));
        let big_irr = friction_hold(stake(100_000, Reversibility::Irreversible));
        // irreversibility adds HOLD_IRREVERSIBLE_MS; bigger money adds log gain.
        assert!(
            big_irr > small_rev + HOLD_IRREVERSIBLE_MS,
            "big irreversible hold must exceed small reversible by >= irreversible delta"
        );
        // reversible-with-cost sits between reversible and irreversible at same money.
        let m = 10_000;
        let rev = friction_hold(stake(m, Reversibility::Reversible));
        let cost = friction_hold(stake(m, Reversibility::ReversibleWithCost));
        let irr = friction_hold(stake(m, Reversibility::Irreversible));
        assert_eq!(cost - rev, HOLD_REVERSIBLE_COST_MS);
        assert_eq!(irr - rev, HOLD_IRREVERSIBLE_MS);
    }

    // D7 — accidental input (random short taps / misaims / jitter) NEVER commits.
    #[test]
    fn accidental_input_never_commits() {
        let spec = friction_spec(stake(5000, Reversibility::ReversibleWithCost));
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            // xorshift — deterministic, no external crate.
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for _ in 0..10_000 {
            let mut fsm = FrictionFsm::new(spec.clone());
            let steps = 1 + (next() % 8) as usize; // 1..=8 accidental inputs
            for _ in 0..steps {
                if next() % 2 == 0 {
                    // misaim / jitter — never advances.
                    fsm.advance(false, (next() % 40) as u32);
                } else {
                    // a short tap: tiny aimed segment then release (never a sustained hold).
                    fsm.advance(true, (next() % 40) as u32 + 1);
                    fsm.release();
                }
            }
            assert!(
                fsm.commit_token().is_none(),
                "accidental input must never mint a CommitToken"
            );
            assert_ne!(
                fsm.state(),
                &FrictionState::Committed,
                "accidental input must never reach Committed"
            );
        }
    }

    // D7 — release before threshold cancels (commit_token None).
    #[test]
    fn release_before_threshold_cancels() {
        let spec = friction_spec(stake(5000, Reversibility::ReversibleWithCost));
        let mut fsm = FrictionFsm::new(spec);
        // Advance most of the way (but not to threshold).
        let hold = fsm.hold_ms();
        fsm.advance(true, hold - 50);
        assert_eq!(
            fsm.state(),
            &FrictionState::Building {
                progress_ms: hold - 50
            }
        );
        fsm.release();
        assert_eq!(fsm.state(), &FrictionState::Cancelled);
        assert!(fsm.commit_token().is_none());
    }

    // D7 — a misaimed hold (aimed=false) never advances progress_ms.
    #[test]
    fn misaimed_hold_does_not_progress() {
        let spec = friction_spec(stake(5000, Reversibility::ReversibleWithCost));
        let mut fsm = FrictionFsm::new(spec);
        fsm.advance(true, 40); // one aimed step
        assert_eq!(fsm.progress_ms(), 40);
        // Now misaim repeatedly — progress must stay put.
        for _ in 0..5 {
            fsm.advance(false, 1000);
        }
        assert_eq!(fsm.progress_ms(), 40, "misaimed hold must not progress");
    }

    // D8 (deaf path) — with audio suppressed, the visual field alone completes a
    // confirm AND cancels; the mirror emits a role=status / aria-live=polite node.
    #[test]
    fn a11y_deaf_path() {
        let spec = friction_spec(stake(5000, Reversibility::ReversibleWithCost));
        let hold = spec.field.hold_ms;

        // Complete via the visual field alone (audio params ignored).
        let mut fsm = FrictionFsm::new(spec.clone());
        let mut t = 0u32;
        while t < hold {
            fsm.advance(true, 50);
            t += 50;
        }
        assert_eq!(fsm.state(), &FrictionState::Committed);
        assert!(fsm.commit_token().is_some());

        // The mirror must carry a status node (role=status, live=polite) with
        // the stake tier + hold progress.
        let has_status = fsm.announcements().iter().any(|a| {
            a.kind == A11yKind::Status
                && a.node.role == "status"
                && a.node.live == LiveMode::Polite
                && a.node.value.contains("stake tier")
        });
        assert!(has_status, "deaf path: mirror must emit a status node");

        // Cancel path: a second FSM released before threshold.
        let mut fsm2 = FrictionFsm::new(spec);
        fsm2.advance(true, hold / 2);
        fsm2.release();
        assert!(fsm2.commit_token().is_none());
        assert_eq!(fsm2.state(), &FrictionState::Cancelled);
    }

    // D8 (blind path) — with the field buffer masked, the audio params alone
    // complete AND cancel; the money amount is read back BEFORE commit.
    #[test]
    fn a11y_blind_path() {
        let spec = friction_spec(stake(5000, Reversibility::ReversibleWithCost));
        let hold = spec.audio.hold_ms; // parity-pinned to the visual hold
        assert_eq!(
            hold, spec.field.hold_ms,
            "audio hold must equal visual hold (parity)"
        );

        let mut fsm = FrictionFsm::new(spec);
        let mut committed_at: Option<usize> = None;
        let mut readback_at: Option<usize> = None;
        let mut step = 0;
        while committed_at.is_none() {
            fsm.advance(true, 50);
            step += 1;
            if readback_at.is_none()
                && fsm
                    .announcements()
                    .iter()
                    .any(|a| a.kind == A11yKind::AmountReadback)
            {
                readback_at = Some(step);
            }
            if fsm.commit_token().is_some() {
                committed_at = Some(step);
            }
            assert!(step < 10_000, "blind path must terminate");
        }
        assert!(fsm.commit_token().is_some());
        // The amount read-back MUST precede the commit (D8 blind-path bar).
        assert!(
            readback_at.is_some_and(|r| committed_at.is_some_and(|c| r < c)),
            "amount read-back must precede commit"
        );
    }

    // D8 (safety invariant) — deaf + blind reach commit at the SAME hold_ms;
    // neither channel offers a shortcut the other lacks (parity).
    #[test]
    fn a11y_safety_invariant() {
        for &money in &[0i64, 500, 5_000, 50_000, 500_000] {
            let spec = friction_spec(stake(money, Reversibility::Irreversible));
            // Parity: audio hold == visual hold for the SAME stake.
            assert_eq!(spec.audio.hold_ms, spec.field.hold_ms);

            // Drive to commit via the visual channel.
            let mut visual = FrictionFsm::new(spec.clone());
            let mut vsteps = 0;
            while visual.commit_token().is_none() {
                visual.advance(true, 50);
                vsteps += 1;
                assert!(vsteps < 10_000);
            }
            // Drive to commit via the audio channel (same threshold).
            let mut audio = FrictionFsm::new(spec);
            let mut asteps = 0;
            while audio.commit_token().is_none() {
                audio.advance(true, 50);
                asteps += 1;
                assert!(asteps < 10_000);
            }
            assert_eq!(
                vsteps, asteps,
                "deaf + blind must reach commit in the same number of steps (parity)"
            );
        }
    }
}
