//! Which parts of the product this venue has switched on.
//!
//! WHY A REGISTRY AND NOT MORE SETTINGS KEYS. There were two flags --
//! `ai.enabled` and `social.enabled` -- each invented where it was needed, each
//! read by one call site, neither listed anywhere. That is not a flag system;
//! it is two flags. The difference shows up the moment somebody asks "what can
//! I turn off?", because the answer lived only in whoever remembered.
//!
//! A flag here is a DECLARED thing: it has a key, a name a restaurateur
//! recognises, a sentence about what turning it off costs, and a default. The
//! surfaces read the list rather than a hard-coded set, so a flag added here
//! appears in the console with no front-end change.
//!
//! THREE RULES.
//!
//! 1. A FLAG NEVER GUARDS CORRECTNESS. Money, the order machine, the allergen
//!    gate and the entitlement checks have no switches. A flag that could turn
//!    off a refusal is a flag that will be turned off during a rush by somebody
//!    who wants the refusal to stop.
//! 2. OFF IS ALWAYS SAFE. Every default is chosen so that a venue which touches
//!    nothing gets a working service, and every flag can be turned off without
//!    stranding data: an order placed with a tip keeps its tip after tipping is
//!    switched off, because the flag governs the CONTROL, not the record.
//! 3. IT SAYS WHAT IT COSTS. "Off" with no explanation is a switch nobody dares
//!    touch, which is the same as not having one.

pub struct Feature {
    pub key: &'static str,
    /// What the owner sees. Not the internal name.
    pub label: &'static str,
    /// What turning it off actually does, in one sentence.
    pub hint: &'static str,
    pub default_on: bool,
    /// Which surface it changes, so the console can group them.
    pub surface: &'static str,
}

pub const FEATURES: &[Feature] = &[
    Feature {
        key: "feature.tips",
        label: "Чайові кур'єру",
        hint: "Прибирає вибір чайових на касі. Уже залишені чайові нікуди не зникають.",
        default_on: true,
        surface: "storefront",
    },
    Feature {
        key: "feature.promo",
        label: "Промокоди",
        hint: "Ховає поле коду на касі. Створені коди лишаються, але ввести їх ніде.",
        default_on: true,
        surface: "storefront",
    },
    Feature {
        key: "feature.feedback",
        label: "Відгук після замовлення",
        hint: "Прибирає поле «як вам?» зі сторінки замовлення.",
        default_on: true,
        surface: "storefront",
    },
    Feature {
        key: "feature.allergen_filter",
        label: "Фільтр за алергенами",
        hint: "Ховає фільтр у меню. Самі заяви про алергени лишаються на стравах — \
               їх приховати не можна.",
        default_on: true,
        surface: "storefront",
    },
    Feature {
        key: "feature.ar",
        label: "Страва на столі (AR)",
        hint: "Прибирає кнопку перегляду страви в реальному розмірі. \
               Вона й так з'являється лише там, де вказано розмір.",
        default_on: true,
        surface: "storefront",
    },
    Feature {
        key: "feature.sea",
        label: "Рухливе тло",
        hint: "Вимикає анімоване тло. Варте того на старих телефонах: це єдина \
               частина сторінки, що постійно малюється.",
        default_on: true,
        surface: "storefront",
    },
    Feature {
        key: "feature.voice",
        label: "Голосові команди",
        hint: "Прибирає мікрофон з панелей. Голос і так нічого не робить сам — \
               кожну дію ще треба підтвердити.",
        default_on: false,
        surface: "staff",
    },
    Feature {
        key: "ai.enabled",
        label: "AI-помічник",
        hint: "Поки вимкнено, нікуди нічого не надсилається.",
        default_on: false,
        surface: "staff",
    },
    Feature {
        key: "social.enabled",
        label: "Чернетки постів",
        hint: "dowiz пропонує пости про справжні зміни в меню. \
               Без вашого схвалення не публікується нічого.",
        default_on: false,
        surface: "staff",
    },
];

pub fn get(key: &str) -> Option<&'static Feature> {
    FEATURES.iter().find(|f| f.key == key)
}

/// Is this feature on for this venue?
///
/// AN UNKNOWN KEY IS OFF. A typo in a call site must not switch something on
/// that the owner never saw a control for.
pub fn is_on(settings: &crate::settings::Settings, key: &str) -> bool {
    let Some(f) = get(key) else { return false };
    match settings.get(key) {
        Some(v) => v.trim() == "1" || v.trim().eq_ignore_ascii_case("true"),
        None => f.default_on,
    }
}

/// Every flag and its current state, for the console and for the surfaces.
pub fn all(settings: &crate::settings::Settings) -> Vec<(&'static Feature, bool)> {
    FEATURES.iter().map(|f| (f, is_on(settings, f.key))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn a_venue_that_touches_nothing_gets_the_defaults() {
        let s = Settings::create().unwrap();
        assert!(is_on(&s, "feature.tips"));
        assert!(is_on(&s, "feature.promo"));
        // The two that send data somewhere are off until asked for.
        assert!(!is_on(&s, "ai.enabled"));
        assert!(!is_on(&s, "social.enabled"));
        assert!(!is_on(&s, "feature.voice"));
    }

    /// A typo must not switch something on that the owner never saw a control
    /// for. The safe answer to "is this unknown thing enabled" is no.
    #[test]
    fn an_unknown_flag_is_off() {
        let s = Settings::create().unwrap();
        assert!(!is_on(&s, "feature.tipz"));
        assert!(!is_on(&s, ""));
        assert!(get("feature.tipz").is_none());
    }

    #[test]
    fn a_flag_can_be_switched_both_ways_and_survives_a_reload() {
        let mut s = Settings::create().unwrap();
        s.set("feature.tips", "0");
        s.set("ai.enabled", "1");
        let bytes = s.to_bytes().unwrap();
        let back = Settings::load(&bytes).unwrap();
        assert!(!is_on(&back, "feature.tips"));
        assert!(is_on(&back, "ai.enabled"));
    }

    #[test]
    fn every_flag_is_declared_once_and_explains_itself() {
        let mut keys: Vec<&str> = FEATURES.iter().map(|f| f.key).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n, "a duplicate key means two controls for one thing");
        for f in FEATURES {
            assert!(!f.label.is_empty(), "{} has no label", f.key);
            // A switch with no consequence written down is one nobody dares
            // touch, which is the same as not having one.
            assert!(f.hint.chars().count() > 20, "{} does not say what it costs", f.key);
            assert!(matches!(f.surface, "storefront" | "staff"), "{} surface", f.key);
        }
    }

    /// The rule that keeps this from becoming a way to switch off a refusal.
    #[test]
    fn no_flag_guards_correctness() {
        for f in FEATURES {
            let k = f.key;
            for banned in ["money", "price", "allergen_gate", "auth", "entitle", "refus", "stock"] {
                assert!(!k.contains(banned), "{k} sounds like it guards correctness");
            }
        }
    }
}
