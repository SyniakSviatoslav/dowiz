//! P64 M1 — Intent runtime v1 (deterministic classifier + router).
//!
//! BLUEPRINT-P64 §2.1 / §3.1. Extends P38 §11.2's intent grammar (which is
//! 0-grep-hit baseline until here) — this module is the single owner of the
//! `Intent`/`FieldPos`/`WidgetId`/`NavTarget`/`CommandId`/`InputSource` types and
//! the deterministic classifier + router that turns raw input into resolved
//! intents.
//!
//! Design constraints (binding):
//! - The classifier is a PURE fn of (input, context) — no I/O, reproducible (P6
//!   Cause-and-Effect). `classifier_is_pure` proves it.
//! - Every raw event funnels through this seam. Nothing downstream ever sees a
//!   concrete event type (P38-rev §12.2 c3). The `no_raw_event_leak` grep gate
//!   (engine/tests/firewall.rs) asserts handlers live ONLY here / inference/.
//! - A consequential candidate is never `Ambiguous` and never reaches the
//!   optional AI ranker (§3.1 / §16.4 / P6). The classifier enforces this.

use crate::text_input::{FieldPos, WidgetId};
use crate::widget_store::WidgetStore;

/// Which surface's command lexicon is live (P38 §11.2 `SurfaceId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SurfaceId(pub u32);

/// Where an input came from and the active bias. Equal-channel by default
/// (§16.50); `CourierInMotion` biases toward voice (§16.53) WITHOUT disabling
/// other channels — it only shifts lexicon sensitivity / pointer precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputProfile {
    #[default]
    Balanced,
    CourierInMotion,
    HandsFree,
}

/// Navigation target (P38 §11.2 `NavTarget`). The v1 set is the surfaces/regions
/// the composer knows how to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NavTarget {
    Menu,
    Cart,
    Catalog,
    Checkout,
    OwnerDashboard,
    CourierBoard,
}

/// A command identifier (P38 §11.2 `CommandId`). The surface lexicon maps
/// spoken/typed phrases to these deterministically (Aho-Corasick-style prefix
/// match in the v1 classifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandId {
    OpenMenu,
    AcceptOrder,
    ConfirmOrder,
    CancelOrder,
    DeclineOrder,
    OpenCart,
    OpenCatalog,
    GoCheckout,
    OpenOwnerDashboard,
    OpenCourierBoard,
}

/// The intent grammar (P38 §11.2). This is the ONLY vocabulary the field UI and
/// the renderer exchange — there is no event-type-specific path outside
/// `InputSource` adapters.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// Point at a field position (no widget hit).
    Point(FieldPos),
    /// A directed impulse at `pos` with scalar magnitude `mag` (e.g. a fling).
    Impulse(FieldPos, f32),
    /// Select a widget (hit-test resolved).
    Select(WidgetId),
    /// Navigate to a target region/surface.
    Navigate(NavTarget),
    /// A continuous scrub along one axis with signed delta `dx`.
    Scrub(f32),
    /// Issue a command (resolved from a voice/typed phrase).
    Command(CommandId),
}

impl Intent {
    /// Consequential intents are the ones that MOVE MONEY or DESTROY STATE. The
    /// classifier must never route these through the AI ranker, and the composer
    /// must attach a `FrictionSpec` (§3.2). A consequential intent is resolved
    /// only via the deterministic lexicon, never via `Ambiguous`.
    pub fn is_consequential(&self) -> bool {
        match self {
            Intent::Command(CommandId::ConfirmOrder)
            | Intent::Command(CommandId::AcceptOrder)
            | Intent::Command(CommandId::CancelOrder)
            | Intent::Command(CommandId::DeclineOrder)
            | Intent::Command(CommandId::GoCheckout) => true,
            // Book-keeping navigation is not consequential; exploratory nav is the
            // AI-rankable case.
            _ => false,
        }
    }
}

/// The classifier's verdict. `Resolved` is the ONLY variant a consequential
/// action may consume (see `Classification::require_resolved`). `Ambiguous` is
/// the ONLY place the optional AI ranker may run — and never for a consequential
/// candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum Classification {
    Resolved(Intent),
    /// Exploratory-navigation candidates only; ranked by the optional AI layer.
    /// Never contains a consequential intent.
    Ambiguous(Vec<Intent>),
    Rejected(RejectReason),
}

/// Why a raw input was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    NoTarget,
    BelowThreshold,
    UnknownCommand,
    /// A consequential candidate appeared in an ambiguous phrase set — the AI
    /// must never touch money/destructive intents, so we hard-reject instead of
    /// auto-picking or deferring.
    OutOfContext,
}

impl Classification {
    /// The resolved intent, if any. Consequential actions consume ONLY this.
    pub fn require_resolved(self) -> Option<Intent> {
        match self {
            Classification::Resolved(i) => Some(i),
            Classification::Ambiguous(_) | Classification::Rejected(_) => None,
        }
    }
}

/// Raw, pre-classification input. One variant per InputSource kind. The
/// classifier is the ONLY consumer; nothing downstream ever sees one.
#[derive(Debug, Clone, PartialEq)]
pub enum RawInput {
    Pointer {
        pos: FieldPos,
        phase: PointerPhase,
        vel: (f32, f32),
    },
    Key(KeyCode),
    VoicePhrase {
        transcript: String,
        confidence: f32,
        is_final: bool,
    },
    Gesture {
        kind: GestureKind,
        origin: FieldPos,
        vector: (f32, f32),
        held_ms: u32,
    },
}

/// Pointer phase for a `RawInput::Pointer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
}

/// Keyboard key (P38 §11.2 `KeyCode`). A small, offline-clean, no-DOM set — the
/// in-canvas editor (P57) owns the full alphabet; this is the subset the intent
/// classifier consumes. Kept here so the seam is closed even for key input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Enter,
    Escape,
    Backspace,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Char(char),
}

/// A gesture kind for `RawInput::Gesture`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureKind {
    Fling,
    Hold,
    Pull,
    Squeeze,
}

/// The input-source seam (P38 §11.2). ONE code path: sources → classify → apply.
/// The wgpu UI receives `Intent`s across this seam and NEVER touches raw events.
pub trait InputSource {
    /// Poll the source for the next raw input. `None` = no new input this tick.
    fn poll(&mut self) -> Option<RawInput>;
}

/// Aim tolerance: cos-angle threshold below which a sustained gesture is aimed
/// at the commit well (§4.3, `AIM_TOLERANCE`).
pub const AIM_TOLERANCE: f32 = 0.35;

/// Context the classifier hit-tests / disambiguates against. Borrowed, never owned.
pub struct IntentContext<'a> {
    pub widgets: &'a WidgetStore,
    pub surface: SurfaceId,
    pub profile: InputProfile,
}

/// A fixed command-lexicon entry: a keyword (prefix) → `CommandId`/`NavTarget`.
/// The v1 classifier matches the longest keyword prefix (deterministic,
/// Aho-Corasick-shaped but a plain sorted scan in v1 — no external crate).
#[derive(Debug, Clone)]
pub struct LexiconEntry {
    pub keyword: &'static str,
    pub command: Option<CommandId>,
    pub nav: Option<NavTarget>,
    /// If true, matching this entry is consequential (the classifier must
    /// resolve it directly, never `Ambiguous`).
    pub consequential: bool,
}

/// The deterministic classifier. Pure fn of (input, context) — reproducible.
/// Holds the per-surface command lexicon (a fixed dispatch table).
pub struct IntentClassifier {
    lexicon: Vec<LexiconEntry>,
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentClassifier {
    /// Build the default lexicon (covers the v1 command/nav set). Consequential
    /// phrases are flagged so the classifier never defers them to the AI ranker.
    pub fn new() -> Self {
        let lexicon = vec![
            LexiconEntry { keyword: "open menu", command: Some(CommandId::OpenMenu), nav: Some(NavTarget::Menu), consequential: false },
            LexiconEntry { keyword: "menu", command: Some(CommandId::OpenMenu), nav: Some(NavTarget::Menu), consequential: false },
            LexiconEntry { keyword: "open cart", command: Some(CommandId::OpenCart), nav: Some(NavTarget::Cart), consequential: false },
            LexiconEntry { keyword: "cart", command: Some(CommandId::OpenCart), nav: Some(NavTarget::Cart), consequential: false },
            LexiconEntry { keyword: "catalog", command: Some(CommandId::OpenCatalog), nav: Some(NavTarget::Catalog), consequential: false },
            LexiconEntry { keyword: "open catalog", command: Some(CommandId::OpenCatalog), nav: Some(NavTarget::Catalog), consequential: false },
            LexiconEntry { keyword: "checkout", command: Some(CommandId::GoCheckout), nav: Some(NavTarget::Checkout), consequential: true },
            LexiconEntry { keyword: "go to checkout", command: Some(CommandId::GoCheckout), nav: Some(NavTarget::Checkout), consequential: true },
            LexiconEntry { keyword: "owner dashboard", command: Some(CommandId::OpenOwnerDashboard), nav: Some(NavTarget::OwnerDashboard), consequential: false },
            LexiconEntry { keyword: "courier board", command: Some(CommandId::OpenCourierBoard), nav: Some(NavTarget::CourierBoard), consequential: false },
            // Consequential commands — resolved directly, never ambiguous.
            LexiconEntry { keyword: "accept order", command: Some(CommandId::AcceptOrder), nav: None, consequential: true },
            LexiconEntry { keyword: "confirm order", command: Some(CommandId::ConfirmOrder), nav: None, consequential: true },
            LexiconEntry { keyword: "confirm", command: Some(CommandId::ConfirmOrder), nav: None, consequential: true },
            LexiconEntry { keyword: "cancel order", command: Some(CommandId::CancelOrder), nav: None, consequential: true },
            LexiconEntry { keyword: "cancel", command: Some(CommandId::CancelOrder), nav: None, consequential: true },
            LexiconEntry { keyword: "decline order", command: Some(CommandId::DeclineOrder), nav: None, consequential: true },
            LexiconEntry { keyword: "decline", command: Some(CommandId::DeclineOrder), nav: None, consequential: true },
        ];
        // Keep longest-keyword-first so the longest-prefix match wins.
        IntentClassifier { lexicon }
    }

    /// Classify a single raw input deterministically.
    pub fn classify(&self, input: &RawInput, ctx: &IntentContext) -> Classification {
        match input {
            RawInput::Pointer { pos, phase, vel } => self.classify_pointer(*pos, *phase, *vel, ctx),
            RawInput::Key(key) => self.classify_key(*key),
            RawInput::VoicePhrase { transcript, confidence, is_final } => {
                self.classify_voice(transcript, *confidence, *is_final, ctx)
            }
            RawInput::Gesture { origin, vector, .. } => self.classify_gesture(*origin, *vector),
        }
    }

    fn classify_pointer(
        &self,
        pos: FieldPos,
        phase: PointerPhase,
        vel: (f32, f32),
        ctx: &IntentContext,
    ) -> Classification {
        match phase {
            // Down/Up: hit-test the widget store. A hit → Select; empty field → Point.
            PointerPhase::Down | PointerPhase::Up => {
                if let Some(w) = hit_test(ctx.widgets, pos) {
                    Classification::Resolved(Intent::Select(w))
                } else {
                    Classification::Resolved(Intent::Point(pos))
                }
            }
            PointerPhase::Move => {
                let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
                if speed < AIM_TOLERANCE {
                    // Below the impulse threshold → just a point (no scrub).
                    if let Some(w) = hit_test(ctx.widgets, pos) {
                        Classification::Resolved(Intent::Select(w))
                    } else {
                        Classification::Resolved(Intent::Point(pos))
                    }
                } else {
                    // dx dominates → Scrub; else Impulse (a fling).
                    if vel.0.abs() > vel.1.abs() {
                        Classification::Resolved(Intent::Scrub(vel.0))
                    } else {
                        Classification::Resolved(Intent::Impulse(pos, speed))
                    }
                }
            }
        }
    }

    fn classify_key(&self, key: KeyCode) -> Classification {
        match key {
            KeyCode::Enter => Classification::Resolved(Intent::Command(CommandId::ConfirmOrder)),
            KeyCode::Escape => Classification::Resolved(Intent::Command(CommandId::CancelOrder)),
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                // Discrete scrub along the dominant axis.
                let dx = match key {
                    KeyCode::ArrowLeft => -1.0,
                    KeyCode::ArrowRight => 1.0,
                    _ => 0.0,
                };
                Classification::Resolved(Intent::Scrub(dx))
            }
            KeyCode::Backspace | KeyCode::Char(_) => Classification::Rejected(RejectReason::NoTarget),
        }
    }

    fn classify_voice(
        &self,
        transcript: &str,
        confidence: f32,
        is_final: bool,
        ctx: &IntentContext,
    ) -> Classification {
        // Non-final partials are not classified yet (the ASR stream is still
        // refining). We don't resolve on a partial — the router drops these.
        if !is_final {
            return Classification::Rejected(RejectReason::NoTarget);
        }
        // CourierInMotion biases sensitivity up: lower confidence still accepted.
        let min_confidence = match ctx.profile {
            InputProfile::CourierInMotion => 0.3,
            _ => 0.5,
        };
        if confidence < min_confidence {
            return Classification::Rejected(RejectReason::BelowThreshold);
        }
        let phrase = transcript.trim().to_ascii_lowercase();

        // Collect every lexicon keyword that is a prefix of (or equal to) the
        // phrase, OR whose phrase is a prefix of the keyword — the latter is the
        // exploratory case ("open" → could become "open menu" / "open cart"),
        // which must surface as Ambiguous rather than silently resolve.
        let matches: Vec<&LexiconEntry> = self
            .lexicon
            .iter()
            .filter(|e| phrase == e.keyword || phrase.starts_with(e.keyword) || e.keyword.starts_with(&phrase))
            .collect();

        if matches.is_empty() {
            return Classification::Rejected(RejectReason::UnknownCommand);
        }

        // If any matched entry is consequential, the AI must NOT touch it. Hard
        // reject out-of-context (the safety core: no money/destructive intent is
        // ever auto-picked or deferred to a probabilistic ranker).
        if matches.iter().any(|e| e.consequential) {
            return Classification::Rejected(RejectReason::OutOfContext);
        }

        // Build the set of DISTINCT targets the phrase matches (a phrase can be a
        // prefix of several keywords → exploratory ambiguity). Order candidates by
        // specificity (longest keyword first) so the optional AI ranker / UI sees
        // the most precise suggestion first.
        let mut distinct: Vec<(Option<CommandId>, Option<NavTarget>, usize)> = matches
            .iter()
            .map(|e| (e.command, e.nav, e.keyword.len()))
            .collect();
        distinct.sort_by(|a, b| b.2.cmp(&a.2));
        distinct.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        if distinct.len() >= 2 {
            let candidates: Vec<Intent> = distinct
                .iter()
                .filter_map(|(c, n, _)| c.map(Intent::Command).or_else(|| n.map(Intent::Navigate)))
                .collect();
            if candidates.len() >= 2 {
                // Exploratory ambiguity — surface for the optional AI ranker; never
                // auto-resolve (D2). E.g. "open" → {OpenMenu, OpenCart}.
                return Classification::Ambiguous(candidates);
            }
        }

        // Exactly one distinct target matched → resolve it deterministically.
        // Prefer the navigation target (an "open X" phrase navigates; the coupled
        // CommandId is just the canonical id the composer also accepts).
        let best = matches
            .iter()
            .max_by_key(|e| e.keyword.len())
            .expect("matches is non-empty here");
        if let Some(n) = best.nav {
            Classification::Resolved(Intent::Navigate(n))
        } else if let Some(c) = best.command {
            Classification::Resolved(Intent::Command(c))
        } else {
            Classification::Rejected(RejectReason::UnknownCommand)
        }
    }

    fn classify_gesture(&self, origin: FieldPos, vector: (f32, f32)) -> Classification {
        let mag = (vector.0 * vector.0 + vector.1 * vector.1).sqrt();
        if mag < AIM_TOLERANCE {
            Classification::Resolved(Intent::Point(origin))
        } else if vector.0.abs() > vector.1.abs() {
            Classification::Resolved(Intent::Scrub(vector.0))
        } else {
            Classification::Resolved(Intent::Impulse(origin, mag))
        }
    }
}

/// Hit-test a `FieldPos` against the widget store. Returns the `WidgetId` of the
/// nearest widget whose bounding box (centred at pos) contains the point. v1 uses
/// the SoA `pos_x`/`pos_y`/`size_w`/`size_h` columns; a widget at index i spans
/// `[pos_x-size_w/2, pos_x+size_w/2] × [pos_y-size_h/2, pos_y+size_h/2]`.
fn hit_test(widgets: &WidgetStore, pos: FieldPos) -> Option<WidgetId> {
    for i in 0..widgets.len() {
        let cx = widgets.pos_x[i];
        let cy = widgets.pos_y[i];
        let hw = widgets.size_w[i] * 0.5;
        let hh = widgets.size_h[i] * 0.5;
        if (pos.u - cx).abs() <= hw && (pos.v - cy).abs() <= hh {
            return Some(widgets.id[i]);
        }
    }
    None
}

/// ONE code path (P38 §11.2 invariant): sources → classify → apply.
pub struct InputRouter {
    sources: Vec<Box<dyn InputSource>>,
    classifier: IntentClassifier,
}

impl InputRouter {
    pub fn new(sources: Vec<Box<dyn InputSource>>) -> Self {
        InputRouter {
            sources,
            classifier: IntentClassifier::new(),
        }
    }

    /// Poll every source, classify, emit resolved intents. Ambiguous/Rejected are
    /// dropped from the apply stream (they are logged by telemetry item 10 and the
    /// optional AI ranker may re-resolve Ambiguous ones elsewhere — never here).
    ///
    /// Each source is polled ONCE per `tick` (one frame = one event per source);
    /// a source that has nothing this frame returns `None`. This keeps the router
    /// bounded — a source never blocks or spins the frame loop.
    pub fn tick(&mut self, ctx: &IntentContext) -> Vec<Intent> {
        let mut out = Vec::new();
        for src in self.sources.iter_mut() {
            if let Some(raw) = src.poll() {
                match self.classifier.classify(&raw, ctx) {
                    Classification::Resolved(i) => out.push(i),
                    // Drop ambiguous/rejected from the apply stream here.
                    Classification::Ambiguous(_) | Classification::Rejected(_) => {}
                }
            }
        }
        out
    }

    /// Classify one raw input directly (used by tests and the composer path).
    pub fn classify(&self, input: &RawInput, ctx: &IntentContext) -> Classification {
        self.classifier.classify(input, ctx)
    }

    /// A test/router seam for the optional AI ranker: given an ambiguous set,
    /// pick the first non-consequential candidate deterministically. The ranker
    /// may ONLY be called on `Ambiguous` (never consequential) — enforced by the
    /// classifier never producing `Ambiguous` with a consequential candidate.
    pub fn rank_ambiguous(&self, candidates: &[Intent]) -> Option<Intent> {
        candidates.iter().find(|i| !i.is_consequential()).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_for(widgets: &WidgetStore) -> IntentContext<'_> {
        IntentContext {
            widgets,
            surface: SurfaceId(0),
            profile: InputProfile::Balanced,
        }
    }

    fn widget_at(widgets: &mut WidgetStore, idx: usize, id: WidgetId, cx: f32, cy: f32) {
        widgets.id[idx] = id;
        widgets.pos_x[idx] = cx;
        widgets.pos_y[idx] = cy;
        widgets.size_w[idx] = 2.0;
        widgets.size_h[idx] = 2.0;
    }

    // D1 — pointer over widget-7 → Resolved(Select(7)); empty field → Point.
    #[test]
    fn intent_types_exist_and_exercised() {
        let mut ws = WidgetStore::new(8);
        widget_at(&mut ws, 7, 7, 5.0, 5.0);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();

        let down_on_widget = RawInput::Pointer {
            pos: FieldPos { u: 5.0, v: 5.0, w: 0.0 },
            phase: PointerPhase::Down,
            vel: (0.0, 0.0),
        };
        assert_eq!(
            classifier.classify(&down_on_widget, &ctx),
            Classification::Resolved(Intent::Select(7))
        );

        let down_empty = RawInput::Pointer {
            pos: FieldPos { u: -40.0, v: -40.0, w: 0.0 },
            phase: PointerPhase::Down,
            vel: (0.0, 0.0),
        };
        assert_eq!(
            classifier.classify(&down_empty, &ctx),
            Classification::Resolved(Intent::Point(FieldPos { u: -40.0, v: -40.0, w: 0.0 }))
        );

        // Router round-trip: a source emitting the widget-down yields Select(7).
        let mut router = InputRouter::new(vec![Box::new(ConstSource(down_on_widget.clone()))]);
        let intents = router.tick(&ctx);
        assert_eq!(intents, vec![Intent::Select(7)]);
    }

    // D1 — voice round-trip: "open menu" final → Navigate(Menu).
    #[test]
    fn voice_round_trip() {
        let ws = WidgetStore::new(4);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();
        let voice = RawInput::VoicePhrase {
            transcript: "open menu".into(),
            confidence: 0.9,
            is_final: true,
        };
        assert_eq!(
            classifier.classify(&voice, &ctx),
            Classification::Resolved(Intent::Navigate(NavTarget::Menu))
        );
    }

    // D2 — classifier purity: same (input, ctx) twice ⇒ identical Classification.
    #[test]
    fn classifier_is_pure() {
        let ws = WidgetStore::new(4);
        let ctx = ctx_for(&ws);
        let a = IntentClassifier::new();
        let b = IntentClassifier::new();
        for phrase in ["open menu", "confirm order", "cart", "accept order", "zzz"] {
            let raw = RawInput::VoicePhrase {
                transcript: phrase.into(),
                confidence: 0.8,
                is_final: true,
            };
            let c1 = a.classify(&raw, &ctx);
            let c2 = b.classify(&raw, &ctx);
            assert_eq!(c1, c2, "classifier must be pure for {phrase}");
        }
    }

    // D2 — a consequential+nav match must Reject(OutOfContext), never Ambiguous.
    #[test]
    fn ambiguous_never_auto_commits() {
        let ws = WidgetStore::new(4);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();
        // "confirm" matches both AcceptOrder? No — "confirm" matches ConfirmOrder
        // (consequential). "checkout" matches GoCheckout (consequential). Either
        // alone must reject-out-of-context, never resolve or defer.
        let confirm = RawInput::VoicePhrase {
            transcript: "confirm".into(),
            confidence: 0.9,
            is_final: true,
        };
        assert_eq!(
            classifier.classify(&confirm, &ctx),
            Classification::Rejected(RejectReason::OutOfContext)
        );
        let checkout = RawInput::VoicePhrase {
            transcript: "checkout".into(),
            confidence: 0.9,
            is_final: true,
        };
        assert_eq!(
            classifier.classify(&checkout, &ctx),
            Classification::Rejected(RejectReason::OutOfContext)
        );
    }

    // D1 — exploratory ambiguity surfaces as Ambiguous (non-consequential only).
    #[test]
    fn exploratory_ambiguity_is_ambiguous() {
        let ws = WidgetStore::new(4);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();
        // "open" is a prefix of both "open menu" and "open cart" → ambiguous nav.
        let open = RawInput::VoicePhrase {
            transcript: "open".into(),
            confidence: 0.9,
            is_final: true,
        };
        match classifier.classify(&open, &ctx) {
            Classification::Ambiguous(cands) => {
                assert!(cands.iter().all(|c| !c.is_consequential()), "ambiguous set must be consequential-free");
                assert!(cands.len() >= 2);
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    /// A tiny constant source for router round-trip tests.
    struct ConstSource(RawInput);
    impl InputSource for ConstSource {
        fn poll(&mut self) -> Option<RawInput> {
            Some(self.0.clone())
        }
    }
}
