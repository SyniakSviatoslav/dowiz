//! The five tokens a venue owner is allowed to touch.
//!
//! §8.1 of the interfaces plan: T1 is BRAND-OWNED and has exactly five members
//! -- accent, ink, paper, type-pair, radius. Everything else in the system is
//! derived from those or is dowiz's own and not the venue's to move.
//!
//! WHY FIVE AND NOT THIRTY. A colour picker per token is a way to produce an
//! unreadable storefront, slowly, with every step feeling reasonable. Five
//! choices cover what a restaurant actually wants to say about itself, and each
//! one is CORRECTED rather than obeyed: the accent moves until it clears AA on
//! white, the ink moves until it clears AA on the paper. What the owner gets is
//! their colour if it works and the nearest colour that does if it does not,
//! with the distance reported so the change is visible rather than silent.
//!
//! THE TYPE PAIR IS AN ID, NEVER A STRING. Two reasons, and the second is the
//! serious one. First, a font the venue names is a font the customer's phone
//! probably does not have, so it renders as whatever the fallback is and the
//! choice was theatre. Second, the storefront receives this theme from its hub
//! and writes it into a stylesheet: a free-text font-family is arbitrary text
//! reaching CSS, and `font-family: x; } body { display:none` is a defacement
//! that no amount of escaping in the wrong place will reliably stop. An id is
//! looked up in a table the CLIENT owns, so nothing the hub says can be
//! anything but one of the pairs that already shipped.

use crate::palette::{Rgb, Theme};

/// A type pairing. Every stack is system-resident or falls back to one that is:
/// no web font is fetched, because a storefront that waits on a font is a
/// storefront that shows nothing on a Durrës 3G connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypePair {
    pub id: &'static str,
    /// What the owner sees. Not a font name -- "Classic" tells a restaurateur
    /// more than "DM Serif Display / DM Sans" does.
    pub label: &'static str,
}

pub const TYPE_PAIRS: [TypePair; 4] = [
    TypePair { id: "classic", label: "Classic" },
    TypePair { id: "modern", label: "Modern" },
    TypePair { id: "warm", label: "Warm" },
    TypePair { id: "plain", label: "Plain" },
];

pub fn type_pair(id: &str) -> Option<TypePair> {
    TYPE_PAIRS.into_iter().find(|p| p.id == id)
}

/// Corner radius, in whole pixels, for the largest surface. Everything smaller
/// is derived from it.
///
/// ZERO IS A CHOICE, not an error: a venue that wants hard corners should be
/// able to have them. The ceiling is 20 because past it a card stops reading as
/// a card and a button starts reading as a pill by accident rather than on
/// purpose -- `--radius-btn` is already a pill and is dowiz's, not the venue's.
pub const RADIUS_MAX: i64 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brand {
    pub accent: Rgb,
    /// The text colour the venue wants. Corrected against the paper.
    pub ink: Rgb,
    /// The page's ground.
    pub paper: Rgb,
    pub type_pair: &'static str,
    pub radius: i64,
}

impl Brand {
    /// The shipped defaults: dowiz's own warm gold on near-white, classic pair,
    /// 8px. A venue that changes nothing still gets a complete, contrast-checked
    /// theme rather than a placeholder.
    pub fn shipped() -> Brand {
        Brand {
            accent: Rgb::from_hex("#d69a3d").unwrap_or(Rgb::new(214, 154, 61)),
            ink: Rgb::new(26, 26, 26),
            paper: Rgb::new(251, 250, 248),
            type_pair: "classic",
            radius: 8,
        }
    }

    /// Read a brand out of a stored theme record, falling back field by field.
    /// A record written before these tokens existed still loads -- it simply
    /// has an accent and nothing else, which is exactly what it had.
    pub fn parse(json: &str) -> Brand {
        use crate::minijson::{int_field, str_field};
        let d = Brand::shipped();
        Brand {
            accent: str_field(json, "seed").and_then(|s| Rgb::from_hex(&s)).unwrap_or(d.accent),
            ink: str_field(json, "ink").and_then(|s| Rgb::from_hex(&s)).unwrap_or(d.ink),
            paper: str_field(json, "paper").and_then(|s| Rgb::from_hex(&s)).unwrap_or(d.paper),
            type_pair: str_field(json, "typePair")
                .and_then(|s| type_pair(&s))
                .map(|p| p.id)
                .unwrap_or(d.type_pair),
            radius: int_field(json, "radius").map(|r| r.clamp(0, RADIUS_MAX)).unwrap_or(d.radius),
        }
    }

    /// The tokens, ready to store. Colours as hex, the pair as its id, the
    /// radius as a number -- nothing here is a CSS fragment, so nothing here
    /// can become one.
    pub fn to_json(&self) -> String {
        format!(
            r#""ink":"{}","paper":"{}","typePair":"{}","radius":{}"#,
            self.ink.hex(),
            self.paper.hex(),
            self.type_pair,
            self.radius
        )
    }

    pub fn theme(&self) -> Theme {
        Theme::from_brand(self.accent, self.ink, self.paper)
    }
}

/// A preset is a whole brand under one name. Hick's law, §8.1: these cover the
/// venues that do not want to make five decisions, and the five tokens stay
/// there for the ones that do.
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub accent: &'static str,
    pub ink: &'static str,
    pub paper: &'static str,
    pub type_pair: &'static str,
    pub radius: i64,
}

pub const PRESETS: [Preset; 6] = [
    Preset {
        id: "cosmo-noir",
        label: "Cosmo Noir",
        accent: "#d69a3d",
        ink: "#1a1a1a",
        paper: "#fbfaf8",
        type_pair: "classic",
        radius: 8,
    },
    Preset {
        id: "adriatic",
        label: "Adriatic",
        accent: "#1c6e8c",
        ink: "#10242b",
        paper: "#f4f8f9",
        type_pair: "modern",
        radius: 12,
    },
    Preset {
        id: "terracotta",
        label: "Terracotta",
        accent: "#b4552d",
        ink: "#2b1a12",
        paper: "#faf4ef",
        type_pair: "warm",
        radius: 16,
    },
    Preset {
        id: "olive",
        label: "Olive",
        accent: "#5f7a3a",
        ink: "#1e2416",
        paper: "#f7f8f2",
        type_pair: "warm",
        radius: 10,
    },
    Preset {
        id: "ink",
        label: "Ink",
        accent: "#2f2f33",
        ink: "#141416",
        paper: "#ffffff",
        type_pair: "plain",
        radius: 0,
    },
    Preset {
        id: "rose",
        label: "Rose",
        accent: "#a83254",
        ink: "#2a121a",
        paper: "#fdf5f7",
        type_pair: "modern",
        radius: 14,
    },
];

pub fn preset(id: &str) -> Option<Brand> {
    PRESETS.iter().find(|p| p.id == id).map(|p| Brand {
        accent: Rgb::from_hex(p.accent).unwrap_or(Rgb::new(214, 154, 61)),
        ink: Rgb::from_hex(p.ink).unwrap_or(Rgb::new(26, 26, 26)),
        paper: Rgb::from_hex(p.paper).unwrap_or(Rgb::new(255, 255, 255)),
        type_pair: p.type_pair,
        radius: p.radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every preset must survive its own derivation. A preset that ships a
    /// failing contrast pair is a venue the owner chose from a menu and cannot
    /// read.
    #[test]
    fn every_preset_passes_contrast_in_both_modes() {
        for p in PRESETS.iter() {
            let b = preset(p.id).unwrap_or_else(|| panic!("{} did not resolve", p.id));
            let t = b.theme();
            for (pair, got, want) in t.contrast_report() {
                assert!(got >= want, "{}: {pair} is {got:.2}:1, needs {want}:1", p.id);
            }
            assert!(type_pair(p.type_pair).is_some(), "{} names an unknown pair", p.id);
            assert!((0..=RADIUS_MAX).contains(&p.radius), "{} radius {}", p.id, p.radius);
        }
    }

    #[test]
    fn preset_ids_are_distinct() {
        let mut ids: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PRESETS.len());
    }

    /// A theme record written before these tokens existed still loads. It has
    /// an accent and nothing else, which is exactly what it had.
    #[test]
    fn an_old_record_still_loads() {
        let b = Brand::parse(r##"{"seed":"#1c6e8c","light":"x","dark":"y"}"##);
        assert_eq!(b.accent.hex(), "#1c6e8c");
        assert_eq!(b.type_pair, "classic");
        assert_eq!(b.radius, 8);
    }

    #[test]
    fn a_brand_round_trips_and_an_unknown_pair_falls_back() {
        let b = Brand {
            accent: Rgb::from_hex("#b4552d").unwrap(),
            ink: Rgb::new(20, 20, 24),
            paper: Rgb::new(255, 255, 255),
            type_pair: "warm",
            radius: 16,
        };
        let stored = format!(r#"{{"seed":"{}",{}}}"#, b.accent.hex(), b.to_json());
        assert_eq!(Brand::parse(&stored), b);

        // A pair nobody shipped is not a pair.
        let bad = Brand::parse(
            r##"{"seed":"#b4552d","typePair":"comic; }body{display:none"}"##,
        );
        assert_eq!(bad.type_pair, "classic");
    }

    /// Zero is a choice; anything past the ceiling is a typo or an attack on
    /// the layout, and is clamped rather than refused so a slider cannot break
    /// the page.
    #[test]
    fn the_radius_is_clamped_at_both_ends() {
        assert_eq!(Brand::parse(r#"{"radius":0}"#).radius, 0);
        assert_eq!(Brand::parse(r#"{"radius":999}"#).radius, RADIUS_MAX);
        assert_eq!(Brand::parse(r#"{"radius":-40}"#).radius, 0);
    }
}
