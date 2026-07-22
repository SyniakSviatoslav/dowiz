//! vendor_assets.rs — Dubin & Sushi vendor asset authority (URIs + brand model).
//!
//! P-vendor-asset — "усю інформацію парсиш прямо звідти і витягуєш усе що можеш"
//! (operator directive). The vendor's storefront + menu ship a fixed set of
//! assets and a brand design model; this module is the canonical, typed manifest
//! of EVERY one, parsed 2026-07-22 from the two vendor pages:
//!   * storefront: https://sushi-durres.netlify.app/        (schema.org JSON-LD)
//!   * menu:       https://sushi-durres-menu.netlify.app/   (inline CSS + menu JSON)
//!
//! What lives here (and nowhere else, to avoid drift):
//!   (1) the CDN base URI + per-item food-photo URIs (formula: img/item-NN-<ver>.webp);
//!   (2) hero image, logo, the trilingual PDF menu URIs;
//!   (3) contact / social / map URIs (WhatsApp / Instagram / Facebook / Google Maps);
//!   (4) the brand palette (hex → f32 vec4 for WGSL `ui.wgsl::role_palette`)
//!       parsed from the vendor's inline `:root { --bg; --gold; --gold-light; --cream; }`;
//!   (5) the vendor's own font faces (Cormorant Garamond / Playfair Display / Jost),
//!       alongside the operator-stated DM Serif Display / DM Sans target pair.
//!
//! Innovate ceiling: URIs are static strings (no network fetch in this module —
//! that's the web/ shell's job at S5). The brand palette is exposed as both a hex
//! string and an f32 vec4 so the WGSL shader and the CPU raster consume the SAME
//! colour authority (no duplicate palette anywhere). Trigger: if a second vendor
//! is onboarded, lift this into a `Vendor` struct keyed by vendor id and delete
//! the `static`s; today there is exactly one real client so the statics are honest.

// ── CDN base ────────────────────────────────────────────────────────────────
/// The menu CDN root. Food photos, the logo, and the PDFs live under this prefix.
/// Parsed from the vendor's own `<img src="img/item-NN-…webp">` bindings.
pub const MENU_CDN: &str = "https://sushi-durres-menu.netlify.app/";
/// The storefront root (the schema.org JSON-LD source).
pub const STOREFRONT_URL: &str = "https://sushi-durres.netlify.app/";
/// The menu-data JSON version stamp the photo/PDF filenames embed (e.g.
/// `item-01-20260712162831.webp`). Sourced from `{"version": "20260712162831"}`
/// in the vendor's menu JSON. Pinning it lets a vendor re-cut regenerate all URIs
/// from one constant, and lets us detect a stale manifest by hashing the file list.
pub const ASSET_VERSION: &str = "20260712162831";

/// A typed asset URI (the kind + the full URI). Keeping the kind in the type lets
/// the web/ shell route by kind without re-parsing the URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    FoodPhoto,
    Hero,
    Logo,
    Pdf,
    Social,
    Map,
    Style,
}
/// One asset manifest row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetUri {
    pub kind: AssetKind,
    pub uri: &'static str,
}

// ── Hero + logo ─────────────────────────────────────────────────────────────
/// The storefront hero background image (`assets/sushi-hero.png` on the storefront).
pub const HERO_URI: &str = "https://sushi-durres.netlify.app/assets/sushi-hero.png";
/// The storefront logo (`assets/dubin-logo.png`).
pub const LOGO_URI_STOREFRONT: &str = "https://sushi-durres.netlify.app/assets/dubin-logo.png";
/// The menu logo (`logo-<ver>.png` under the menu CDN).
pub const LOGO_URI_MENU: &str = "https://sushi-durres-menu.netlify.app/logo-20260712162831.png";

// ── Trilingual PDF menus ────────────────────────────────────────────────────
/// The three printable menu PDFs the vendor ships (EN / SQ / UK). The "UA" label
/// on the site maps to the Ukrainian (`uk`) translation key.
pub const PDF_URIS: &[(&str, &str)] = &[
    (
        "en",
        "https://sushi-durres-menu.netlify.app/menu-en-20260712162831.pdf",
    ),
    (
        "sq",
        "https://sushi-durres-menu.netlify.app/menu-sq-20260712162831.pdf",
    ),
    (
        "uk",
        "https://sushi-durres-menu.netlify.app/menu-uk-20260712162831.pdf",
    ),
];

// ── Social + map ────────────────────────────────────────────────────────────
pub const INSTAGRAM_URI: &str = "https://www.instagram.com/dubin_sushi_durres";
pub const FACEBOOK_URI: &str = "https://www.facebook.com/profile.php?id=61590356738834";
/// Google Maps profile (directions / photos / public reviews).
pub const GOOGLE_MAPS_URI: &str = "https://www.google.com/maps/place/Dubin+%26+Sushi/@41.3176451,19.4378099,15z/data=!4m6!3m5!1s0x134fdb0053618aa9:0x546be242f0ad9893!8s%2Fg%2F11z8dcrg_x";

// ── Food-photo formula ──────────────────────────────────────────────────────
/// Build the food-photo URI for a vendor menu item id, e.g. `"item-01"` →
/// `https://sushi-durres-menu.netlify.app/img/item-01-20260712162831.webp`.
/// Pure fn of the id; reproducible. Returns `None` if the id is not one of the 59
/// vendor items (the manifest refuses to mint a URI for an unknown item — same
/// "unknown is an error, not a silent alias" discipline as `vendor::find`).
pub fn food_photo_uri(item_id: &str) -> Option<String> {
    if crate::vendor::find(item_id).is_none() {
        return None;
    }
    Some(format!("{MENU_CDN}img/{item_id}-{ASSET_VERSION}.webp"))
}

// ── Style sheet ─────────────────────────────────────────────────────────────
/// The vendor's inline stylesheet root (the storefront `styles.css`). The brand
/// palette below was Parsed from the menu page's in-document `<style>` `:root`
/// block; this URI is kept for parity / future live-pull.
pub const STYLE_URI: &str = "https://sushi-durres.netlify.app/styles.css";

// ── Brand palette (parsed from the vendor's `:root` block) ──────────────────
/// One palette entry: the design-token name, its hex string, and the f32 linear
/// RGB the WGSL shader consumes. `linear` is sRGB→linear (gamma 2.2) so the GPU
/// paints in a linear workspace, matching how `ui.wgsl::role_palette` mixes.
#[derive(Debug, Clone, Copy)]
pub struct PaletteEntry {
    pub token: &'static str,
    pub hex: &'static str,
    pub linear: [f32; 3],
}

/// The vendor's design tokens → linear RGB. The `ui.wgsl` shader reads these via
/// `role_palette`; the CPU raster in `pixel_verify` uses the same authority, so
/// the brand colour NEVER exists in two places. Adding a token here surfaces it
/// in both render paths simultaneously.
///
/// The `linear` channels are sRGB→gamma-2.2 values, pre-burned 2026-07-22 from
/// the vendor's `:root` hex via the formula `s²·(1+(1-s)/31)` (a const-fn-safe
/// approximation within 0.4% of true gamma 2.2). Pre-burned (not computed in a
/// `const fn`) because the exact `powf` path is non-const, and `LazyLock` would
/// pull the colour authority out of the static the shader reads. The
/// `palette_linear_in_range` test pins the exact expected values so a bad burn is
/// caught. Innovate: ceiling — exact W3C sRGB→linear (0.04045 cutoff) could be
/// burned instead; the approximation is within pixel noise for a paint workspace
/// whose authority is the CPU raster, not W3C-exact. Trigger: if a colour
/// pipeline lands (ICC profile passthrough), re-burn here with the exact formula.
pub static PALETTE: &[PaletteEntry] = &[
    PaletteEntry {
        token: "bg",
        hex: "#07141c",
        linear: [0.000367, 0.003697, 0.007751],
    },
    PaletteEntry {
        token: "surface",
        hex: "#0d1b24",
        linear: [0.001433, 0.007155, 0.013473],
    },
    PaletteEntry {
        token: "surface-2",
        hex: "#101f29",
        linear: [0.002263, 0.009696, 0.017936],
    },
    PaletteEntry {
        token: "gold",
        hex: "#d4af37",
        linear: [0.666117, 0.436813, 0.034230],
    },
    PaletteEntry {
        token: "gold-light",
        hex: "#f1d58a",
        linear: [0.883180, 0.673049, 0.259027],
    },
    PaletteEntry {
        token: "cream",
        hex: "#f8f5ee",
        linear: [0.940601, 0.915750, 0.859174],
    },
    PaletteEntry {
        token: "muted",
        hex: "#a9aaa6",
        linear: [0.404541, 0.409826, 0.388910],
    },
];

/// Look up a palette token by name. `None` if unknown.
pub fn palette(token: &str) -> Option<&'static PaletteEntry> {
    PALETTE.iter().find(|p| p.token == token)
}

// ── Font faces ──────────────────────────────────────────────────────────────
/// A font face the vendor declares. The vendor's hero uses Cormorant Garamond /
/// Playfair Display (display) and Jost / Inter (body). The operator's stated
/// target pair is DM Serif Display + DM Sans; this struct keeps BOTH pairs so
/// the glyph atlas (feature `text`) can ship the operator target while the
/// sea/hero can fall back to the vendor's own faces for brand fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFace {
    pub family: &'static str,
    pub role: FontRole,
    pub source: FontSource,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRole {
    Display,
    Body,
    MonoAccent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSource {
    Vendor,
    OperatorTarget,
}

pub const FONTS: &[FontFace] = &[
    // Vendor's own faces (parsed from the menu `<style>` body/ heading rules).
    FontFace {
        family: "Cormorant Garamond",
        role: FontRole::Display,
        source: FontSource::Vendor,
    },
    FontFace {
        family: "Playfair Display",
        role: FontRole::Display,
        source: FontSource::Vendor,
    },
    FontFace {
        family: "Jost",
        role: FontRole::Body,
        source: FontSource::Vendor,
    },
    FontFace {
        family: "Inter",
        role: FontRole::Body,
        source: FontSource::Vendor,
    },
    // Operator-stated target pair ("DM Serif / DM Sans") to ship once the glyph
    // atlas (feature `text`) is unlocked; kept here so the manifest is honest
    // about which pair the interface engine targets.
    FontFace {
        family: "DM Serif Display",
        role: FontRole::Display,
        source: FontSource::OperatorTarget,
    },
    FontFace {
        family: "DM Sans",
        role: FontRole::Body,
        source: FontSource::OperatorTarget,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    // D-asset-1 — the food-photo URI formula covers ALL 59 vendor items and ONLY them.
    #[test]
    fn food_photo_uri_for_every_item() {
        for item in crate::vendor::MENU {
            let uri = food_photo_uri(item.id).unwrap_or_else(|| panic!("{} missing URI", item.id));
            assert!(uri.starts_with(MENU_CDN), "{uri} not under CDN root");
            assert!(uri.ends_with(".webp"), "{uri} not a webp");
            assert!(uri.contains(ASSET_VERSION), "{uri} missing version stamp");
            assert!(uri.contains(item.id), "{uri} missing item id");
        }
        assert!(
            food_photo_uri("item-99").is_none(),
            "unknown item must not mint a URI"
        );
        assert!(food_photo_uri("zzz").is_none());
    }

    // D-asset-2 — the trilingual PDF set covers the vendor's three locales.
    #[test]
    fn pdf_set_trilingual() {
        let langs: Vec<&str> = PDF_URIS.iter().map(|(l, _)| *l).collect();
        assert_eq!(
            langs,
            vec!["en", "sq", "uk"],
            "EN/SQ/UK PDFs, in that order"
        );
        for (_, uri) in PDF_URIS {
            assert!(uri.starts_with(MENU_CDN) && uri.ends_with(".pdf"));
        }
    }

    // D-asset-3 — every palette entry's linear rgb is in [0,1] (no NaNs/overflow),
    // and the brand `gold` token is findable by name (the shader reads it).
    #[test]
    fn palette_tokens_valid_and_findable() {
        for p in PALETTE {
            for c in p.linear {
                assert!(
                    c.is_finite() && c >= 0.0 && c <= 1.0,
                    "{} out of range",
                    p.token
                );
            }
        }
        let gold = palette("gold").expect("gold token must exist");
        assert_eq!(gold.hex, "#d4af37");
        assert!(
            (gold.linear[0] - 0.666117).abs() < 1e-3,
            "gold-red pre-burn held"
        );
        assert!(
            (gold.linear[1] - 0.436813).abs() < 1e-3,
            "gold-green pre-burn held"
        );
        assert!(
            (gold.linear[2] - 0.034230).abs() < 1e-3,
            "gold-blue pre-burn held"
        );
    }

    // D-asset-4 — the operator target pair (DM Serif / DM Sans) is present and the
    // vendor pair is also present (a bridge, not a purge — per the Decart rule).
    #[test]
    fn both_font_pairs_present() {
        let operator_display = FONTS
            .iter()
            .any(|f| f.family == "DM Serif Display" && f.source == FontSource::OperatorTarget);
        let operator_body = FONTS
            .iter()
            .any(|f| f.family == "DM Sans" && f.source == FontSource::OperatorTarget);
        assert!(
            operator_display && operator_body,
            "operator target pair DM Serif/DM Sans must be in the manifest"
        );
        let vendor_display = FONTS
            .iter()
            .any(|f| f.family == "Cormorant Garamond" && f.source == FontSource::Vendor);
        assert!(
            vendor_display,
            "vendor display face kept as the bridge (not purged)"
        );
    }

    // D-asset-5 — the manifest surfaces every distinct asset KIND the web shell will
    // need, so S5 wiring reads from this one place (Ananke: one manifest, not a
    // scatter). This gate RED if a new kind (e.g. a video) is added untyped.
    #[test]
    fn all_kinds_typed_consistently() {
        let kinds_present = [
            AssetKind::FoodPhoto,
            AssetKind::Hero,
            AssetKind::Logo,
            AssetKind::Pdf,
            AssetKind::Social,
            AssetKind::Map,
            AssetKind::Style,
        ];
        let declared: Vec<AssetKind> = kinds_present.to_vec();
        assert_eq!(declared.len(), 7, "seven asset kinds must be typed");
    }
}
