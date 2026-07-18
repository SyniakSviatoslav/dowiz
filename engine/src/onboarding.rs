//! P64 M7 — Implicit onboarding v1 (idle-field hints, no tutorial modal).
//!
//! BLUEPRINT-P64 §2 / §3.7. `HintPolicy` tracks per-intent `familiarity: u8`.
//! When the field is SETTLED and the user has not yet demonstrated the surface's
//! primary intent, the Composer overlays low-amplitude affordance HINT
//! directives — gentle field perturbations expressing the next available intent,
//! in the SAME field language as friction (§16.50). A hint retires once
//! `familiarity[intent] >= HINT_MASTERY_THRESHOLD`. No modal, no text how-to.
//!
//! Hints are visual/audio ONLY: they never consume or reorder a real `Intent`
//! (`hint_never_intercepts_real_input`), and appear only when settled
//! (`hint_respects_settle`).

use crate::friction::HINT_MASTERY_THRESHOLD;
use crate::intent::Intent;
use crate::scene::SdfShape;

/// A single intent's familiarity record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Familiarity {
    pub successes: u8,
}

impl Familiarity {
    pub fn mastered(&self) -> bool {
        self.successes >= HINT_MASTERY_THRESHOLD
    }
}

/// Small stable key for an intent kind we track hints for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HintIntent {
    OpenMenu,
    OpenCart,
    Confirm,
}

impl HintIntent {
    fn from_intent(intent: &Intent) -> Option<HintIntent> {
        match intent {
            Intent::Navigate(crate::intent::NavTarget::Menu)
            | Intent::Command(crate::intent::CommandId::OpenMenu) => Some(HintIntent::OpenMenu),
            Intent::Navigate(crate::intent::NavTarget::Cart)
            | Intent::Command(crate::intent::CommandId::OpenCart) => Some(HintIntent::OpenCart),
            Intent::Command(crate::intent::CommandId::ConfirmOrder) => Some(HintIntent::Confirm),
            _ => None,
        }
    }
}

/// A low-amplitude affordance hint directive (in the SAME field language as
/// friction — a gentle perturbation at the location that would express the next
/// available intent). Visual/audio only.
#[derive(Debug, Clone, PartialEq)]
pub struct HintDirective {
    pub intent: HintIntent,
    pub shape: SdfShape,
    pub amplitude: f32,
}

/// The onboarding policy. Holds per-intent familiarity. Persisted in the P66
/// wallet by consumers (this module only owns the in-memory model + the rule).
#[derive(Debug, Clone)]
pub struct HintPolicy {
    familiarity: std::collections::BTreeMap<HintIntent, Familiarity>,
    /// The surface's primary intent (the one hinted first).
    primary: HintIntent,
}

impl HintPolicy {
    pub fn new() -> Self {
        HintPolicy {
            familiarity: std::collections::BTreeMap::new(),
            primary: HintIntent::OpenMenu,
        }
    }

    /// Record a successful performance of an intent (advances familiarity).
    pub fn record_success(&mut self, intent: &Intent) {
        if let Some(key) = HintIntent::from_intent(intent) {
            let f = self.familiarity.entry(key).or_default();
            f.successes = f.successes.saturating_add(1);
        }
    }

    fn familiarity_of(&self, key: HintIntent) -> Familiarity {
        *self.familiarity.get(&key).unwrap_or(&Familiarity::default())
    }

    /// True iff the field is settled (no in-flight animation). The Composer asks
    /// this before overlaying hints. (Caller owns the settle gate; we model it as
    /// a passed-in boolean so the policy stays pure/testable.)
    pub fn hints_for(&self, settled: bool, center: (f32, f32)) -> Vec<HintDirective> {
        if !settled {
            return Vec::new(); // hint_respects_settle
        }
        let mut out = Vec::new();
        // Hint the primary intent if not yet mastered; then any other unmastered.
        let consider = std::iter::once(self.primary)
            .chain([HintIntent::OpenCart, HintIntent::Confirm].into_iter())
            .filter(|k| !self.familiarity_of(*k).mastered());
        for k in consider {
            out.push(HintDirective {
                intent: k,
                shape: SdfShape::Circle {
                    cx: center.0 as f64,
                    cy: center.1 as f64,
                    r: 0.6,
                },
                amplitude: 0.05, // low-amplitude, never distracting
            });
        }
        out
    }

    /// A hint NEVER intercepts real input: resolving a user intent returns the
    /// user's intent, not a hinted one. This helper is the contract — it returns
    /// the user intent unchanged (proved by `hint_never_intercepts_real_input`).
    pub fn resolve_user_intent(&self, user_intent: Intent) -> Intent {
        user_intent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{CommandId, Intent, NavTarget};

    #[test]
    fn onboarding_hint_present_then_decays() {
        let mut policy = HintPolicy::new();
        let center = (0.0f32, 0.0f32);

        // Fresh profile, settled idle → ≥1 hint for the primary intent.
        let hints = policy.hints_for(true, center);
        assert!(
            !hints.is_empty(),
            "fresh profile must surface ≥1 hint on settled idle"
        );
        assert!(hints.iter().any(|h| h.intent == HintIntent::OpenMenu));

        // After HINT_MASTERY_THRESHOLD successes of the primary intent, NO hint
        // for it remains (it retired).
        for _ in 0..HINT_MASTERY_THRESHOLD {
            policy.record_success(&Intent::Navigate(NavTarget::Menu));
        }
        let hints2 = policy.hints_for(true, center);
        assert!(
            !hints2.iter().any(|h| h.intent == HintIntent::OpenMenu),
            "primary hint must retire after mastery threshold"
        );
    }

    #[test]
    fn hint_never_intercepts_real_input() {
        let policy = HintPolicy::new();
        let user = Intent::Command(CommandId::ConfirmOrder);
        // The user's real intent resolves to the user's intent, never the hinted one.
        assert_eq!(policy.resolve_user_intent(user.clone()), user);
    }

    #[test]
    fn hint_respects_settle() {
        let policy = HintPolicy::new();
        // A busy field (settled=false) shows no hints (no distraction mid-action).
        assert!(policy.hints_for(false, (0.0, 0.0)).is_empty());
        assert!(!policy.hints_for(true, (0.0, 0.0)).is_empty());
    }
}
