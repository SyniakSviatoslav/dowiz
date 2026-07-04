//! S3 catalog/admin CRUD — theme GET/PUT. Ports `apps/api/src/routes/owner/themes.ts`'s two
//! non-media ops:
//!   - `GET  /api/owner/locations/:locationId/theme` (themes.ts:17-43)
//!   - `PUT  /api/owner/locations/:locationId/theme` (themes.ts:46-116)
//!
//! **Deferred, NOT built here:** `POST .../theme/logo` (themes.ts:119-149) — S4 media-upload work
//! (sharp resize/webp + R2 `storage.put`), a different surface (census row #86). See
//! `docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md` rows #84-86.
//!
//! Also ports `apps/api/src/lib/theme-renderer.ts` (`renderTheme`/`ALLOWED_FONTS`/the color-math
//! helpers) as pure, DB-free Rust functions below — op 2's response is entirely a function of
//! this rendering logic, so it is the most heavily unit-tested part of this file.
//!
//! ## In-transaction membership re-check (S3 breaker finding C1+H4)
//! Per `routes/owner/mod.rs`'s module doc: the extractor-level `require_location_access` call
//! (still run first, as a cheap fast-path) is NOT the security boundary by itself — every
//! `with_user`-seated transaction below ALSO calls
//! [`crate::routes::owner::assert_active_owner_membership`] as its first statement, on the SAME
//! connection/GUC as the op's real SQL. A `false` result is treated as `Ok(None)` up through
//! [`ThemesRepo::get_theme`]/[`ThemesRepo::update_and_render`] — the handler maps that to 404,
//! identically to a genuinely-missing theme row (existence-hiding, matching
//! `require_location_access`'s own owner-branch 404).
//!
//! ## CARRY-DIVERGENCE: op 1 (GET) now goes through `with_user`, unlike the TS
//! The TS GET (`themes.ts:23`) uses a raw `db.connect()` — no GUC seated at all. This is a
//! sanctioned fix-in-port (S3 breaker finding C2, pending council RESOLVE), not a carry: routing
//! GET through `with_user` + `assert_active_owner_membership` like every other S3 op is strictly
//! safer under NOBYPASSRLS and costs nothing today (BYPASSRLS still masks any GUC gap). Separately,
//! `theme_versions`' own RLS policy (`theme_versions_owner_write`, migration
//! `1790000000084...ts:33-41`) keys off a PostgREST-style GUC (`request.jwt.claim.sub`) that this
//! Rust build does not seat either — noted, not addressed here (also true of every other S3 write,
//! not specific to themes).
//!
//! ## Schema-gap finding (NOT in the original build brief — verified against live migrations)
//! `location_themes`' REAL columns (verified against `packages/db/migrations/1780310075801_branding.ts`,
//! `.../1780338982030_theme_versions.ts`, `.../1790000000022...ts`, `.../1790000000037...ts`,
//! `.../1790000000084_location-themes-fonts.ts`) are: `id, location_id, logo_url, primary_color,
//! css_hash, updated_at, bg_color, text_color, frame_ancestors, font_url, google_rating,
//! google_review_count, google_maps_url, google_place_id, social_instagram, social_facebook,
//! heading_font, body_font`. There is **no `secondary_color` and no `font_family` column** —
//! despite both being accepted by `themes.ts`'s Zod body schema (`themes.ts:51-52`) and read back
//! by `renderTheme`'s call site (`themes.ts:88-89`). A PUT body that includes either key hits
//! TS's dynamic `SET ${f} = $n` (`themes.ts:69`) against a column that does not exist, throwing a
//! Postgres `undefined_column` error the route's catch-all (`themes.ts:110-112`) turns into an
//! unhandled 500 — confirmed no E2E spec (`flow-ui-admin-branding.spec.ts`, the row's cited proof
//! spec) ever exercises this PUT op with either field (it only exercises the logo POST and the
//! public GET), consistent with this being live, never-hit, broken functionality. Ported here as:
//! [`OwnerThemeRow`] carries `secondary_color`/`font_family` fields (per the build brief's own
//! "safe minimum set" fallback) but they are `#[sqlx(default)]` (never populated by a real SELECT,
//! always absent on the wire — `#[serde(skip_serializing_if)]`), and [`PgThemesRepo::update_and_render`]
//! /`fake::FakeThemesRepo::update_and_render` both fail the request (mapped to 500 Internal) the
//! moment either key is PRESENT in the update body — reproducing the real crash without literally
//! emitting doomed SQL this sandbox can't run against a live Postgres anyway. Flagged prominently
//! per the build brief's "any TS ambiguity judgment-called" ask; not something migrations can fix
//! from this file (`packages/db/migrations/` is on the "do not touch" list).
//!
//! ## Quirks carried verbatim (see also module doc above)
//! - **No rowcount check** (`themes.ts:66-79`): the UPDATE never checks whether it touched any
//!   rows; the subsequent `SELECT * FROM location_themes WHERE location_id = $1` is trusted to
//!   return a row. If the location has no `location_themes` row at all, TS's `theme.primary_color`
//!   throws on `undefined` → 500. Ported via `.fetch_one(...)` (not `.fetch_optional`), whose
//!   `RowNotFound` propagates as a genuine `RepoError` → the handler's 500 Internal path — same
//!   observable behavior, no invented 404.
//! - **Version can outrun storage** (`themes.ts:96-109`): `INSERT ... ON CONFLICT (location_id,
//!   css_hash) DO NOTHING` silently dedupes an identical-CSS re-render, but the response's
//!   `version` is still `currentVersion + 1` regardless of whether a new row was actually stored.
//!   Carried as-is (see `fake::update_and_render_dedup_reports_a_version_ahead_of_actual_storage`).
//! - **`frame_ancestors` has no origin-format validation** (owner-route census row #85) despite
//!   feeding the CSP header elsewhere — not added here; Node doesn't validate it either.
//! - **`adjust_color`'s channel-order oddity** (`theme-renderer.ts:52`): recombines as
//!   `g | (b << 8) | (r << 16)`, NOT the `r | (g<<8) | (b<<16)`-equivalent (i.e. standard
//!   `#RRGGBB`) you'd expect — this swaps the rendered green and blue channels relative to what a
//!   "lighten/darken and rebuild #RRGGBB" helper should produce. Ported bug-for-bug in
//!   [`adjust_color`] below (see its doc + `adjust_color_reproduces_the_green_blue_channel_swap`).
//!   **This looks like a genuine latent bug in the original TS**, flagged here as a candidate for
//!   a future 🔴-gated fix — not fixed in this port per the build brief's carry-verbatim mandate.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;
use tower_http::request_id::RequestId;
use uuid::Uuid;

use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;

// ── theme-renderer.ts port: fonts, hex validation, color math, render_theme ──────────────────

/// `theme-renderer.ts:3`.
pub const ALLOWED_FONTS: [&str; 6] = [
    "Inter",
    "Roboto",
    "Source Sans 3",
    "Lato",
    "Open Sans",
    "system-ui",
];

/// `FontFamily` (`theme-renderer.ts:4`) — a closed set, strictly deserialized so an invalid string
/// fails at the `Json` extractor (400), the same net effect as Zod's `z.enum(ALLOWED_FONTS)`
/// rejecting at the Fastify schema layer (exact error body differs; both are 400s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeFont {
    Inter,
    Roboto,
    SourceSans3,
    Lato,
    OpenSans,
    SystemUi,
}

impl ThemeFont {
    pub const fn as_str(self) -> &'static str {
        match self {
            ThemeFont::Inter => "Inter",
            ThemeFont::Roboto => "Roboto",
            ThemeFont::SourceSans3 => "Source Sans 3",
            ThemeFont::Lato => "Lato",
            ThemeFont::OpenSans => "Open Sans",
            ThemeFont::SystemUi => "system-ui",
        }
    }
}

impl<'de> Deserialize<'de> for ThemeFont {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Inter" => Ok(ThemeFont::Inter),
            "Roboto" => Ok(ThemeFont::Roboto),
            "Source Sans 3" => Ok(ThemeFont::SourceSans3),
            "Lato" => Ok(ThemeFont::Lato),
            "Open Sans" => Ok(ThemeFont::OpenSans),
            "system-ui" => Ok(ThemeFont::SystemUi),
            other => Err(DeError::custom(format!(
                "invalid font_family {other:?}: must be one of {ALLOWED_FONTS:?}"
            ))),
        }
    }
}

/// A `#rrggbb` hex color string, validated on deserialize (`themes.ts:50` etc.'s
/// `z.string().regex(/^#[0-9a-fA-F]{6}$/)`). A manual check, not the `regex` crate — the crate
/// isn't already a workspace dependency, and a 6-hex-digit check is trivial without one (per the
/// build brief's "no new deps unless truly unavoidable" rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexColor(pub String);

fn is_valid_hex_color(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit)
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if is_valid_hex_color(&s) {
            Ok(HexColor(s))
        } else {
            Err(DeError::custom(format!(
                "invalid hex color {s:?}: must match ^#[0-9a-fA-F]{{6}}$"
            )))
        }
    }
}

/// The serde "double option" idiom: distinguishes a JSON key that is ABSENT (`#[serde(default)]`
/// leaves the field `None`) from a key PRESENT with value `null` (`Some(None)`) from a key present
/// with a real value (`Some(Some(v))`). Required to carry `themes.ts:67-75`'s "only update fields
/// actually present in `Object.keys(updates)`" quirk — an explicit `null` clears a column, an
/// absent key leaves it untouched, and a naive single `Option<T>` cannot tell those apart.
fn deserialize_some<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// `ThemeInput` (`theme-renderer.ts:6-13`) — the pure rendering input. `secondary_color`/
/// `font_family` are always `None` in production (see module doc's schema-gap finding); kept as
/// real fields here so [`render_theme`] itself stays a faithful, independently-testable port of
/// the TS function regardless of that DB reality.
#[derive(Debug, Clone, Default)]
pub struct ThemeInput {
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub font_family: Option<String>,
    pub bg_color: Option<String>,
    pub text_color: Option<String>,
    pub logo_url: Option<String>,
}

/// `ThemeRendered` (`theme-renderer.ts:15-20`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeRendered {
    pub css: String,
    pub css_hash: String,
    pub version: i32,
    pub warnings: Vec<String>,
}

/// `getLuminance` (`theme-renderer.ts:22-33`) — WCAG-style relative luminance from a `#rrggbb`
/// hex string. A malformed hex parses to `0` (mirrors JS: `parseInt` on a bad slice yields `NaN`,
/// and `NaN >> 16` etc. coerce to `0` under JS's ToInt32 semantics — so a bad hex behaves as pure
/// black on both stacks, not a panic).
pub fn get_luminance(hex: &str) -> f64 {
    let rgb = hex
        .get(1..)
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .unwrap_or(0);
    let r = f64::from((rgb >> 16) & 0xff);
    let g = f64::from((rgb >> 8) & 0xff);
    let b = f64::from(rgb & 0xff);
    let channel = |v: f64| {
        let v = v / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    channel(r) * 0.2126 + channel(g) * 0.7152 + channel(b) * 0.0722
}

/// `getContrastRatio` (`theme-renderer.ts:35-41`).
pub fn get_contrast_ratio(hex1: &str, hex2: &str) -> f64 {
    let lum1 = get_luminance(hex1);
    let lum2 = get_luminance(hex2);
    let brightest = lum1.max(lum2);
    let darkest = lum1.min(lum2);
    (brightest + 0.05) / (darkest + 0.05)
}

/// `adjustColor` (`theme-renderer.ts:44-53`) — lighten/darken by `amount` per channel, clamped to
/// `0..=255`. **Ported bug-for-bug**: the recombination is `g | (b << 8) | (r << 16)`
/// (`theme-renderer.ts:52`), NOT the `r`-high/`g`-mid/`b`-low order you'd expect for a `#RRGGBB`
/// rebuild — this swaps the rendered green and blue channels. Confirmed by hand in
/// `adjust_color_reproduces_the_green_blue_channel_swap` below. Flagged as a likely genuine latent
/// bug in the source TS (candidate for a future 🔴-gated fix) but carried verbatim here per the
/// build brief — do NOT "fix" this arithmetic.
pub fn adjust_color(hex: &str, amount: i32) -> String {
    let col = hex
        .get(1..)
        .and_then(|h| i32::from_str_radix(h, 16).ok())
        .unwrap_or(0);
    let r = ((col >> 16) + amount).clamp(0, 255);
    let g = (((col >> 8) & 0x00ff) + amount).clamp(0, 255);
    let b = ((col & 0x0000ff) + amount).clamp(0, 255);
    let combined = g | (b << 8) | (r << 16);
    format!("#{combined:06x}")
}

/// `undefined`/empty-string treated as "no value" — mirrors JS's `input.x || DEFAULT` truthy-OR
/// (`theme-renderer.ts:58-59,65,72`), under which `""` is just as falsy as `null`/`undefined`.
fn nonempty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|s| !s.is_empty())
}

/// Narrows a `theme_versions.version` (real column type `bigint`) down to the `i32` this pure
/// rendering layer and the trait's `(String, i32)`/`ThemeUpdates` shapes use — saturates rather
/// than an `as` truncation (workspace `clippy::as_conversions = "warn"`, promoted to deny under
/// `-D warnings`). A real deployment overflowing `i32::MAX` versions for one location is not a
/// scenario this port needs to handle gracefully beyond "don't silently wrap".
fn version_i32(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// `crypto.createHash('sha256').update(css).digest('hex').slice(0, 16)` (`theme-renderer.ts:125`).
fn css_hash_of(css: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(css.as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}

/// `.replace(/\s+/g, ' ').trim()` (`theme-renderer.ts:123`) — collapses any run of whitespace
/// (space/tab/newline/etc.) to a single space, matching JS's `\s+` character class exactly
/// (`char::is_whitespace` covers the same Unicode whitespace set `\s` does in practice for the
/// ASCII-only content this function ever actually produces).
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !in_ws {
                out.push(' ');
            }
            in_ws = true;
        } else {
            out.push(c);
            in_ws = false;
        }
    }
    out
}

/// `renderTheme` (`theme-renderer.ts:55-133`) — the pure CSS-rendering core op 2 depends on. Same
/// defaults, same `LOW_CONTRAST_PRIMARY` warning threshold, same Google-Fonts `@import` gate, same
/// minification, same `cssHash` algorithm (sha256 hex, first 16 chars), same `version =
/// current_version + 1`.
pub fn render_theme(input: &ThemeInput, current_version: i32) -> ThemeRendered {
    let mut warnings = Vec::new();

    let primary = nonempty(&input.primary_color)
        .unwrap_or("#e63946")
        .to_string();
    let secondary = nonempty(&input.secondary_color)
        .unwrap_or("#457b9d")
        .to_string();
    let bg = nonempty(&input.bg_color).unwrap_or("#ffffff").to_string();

    let bg_lum = get_luminance(&bg);
    let is_dark_bg = bg_lum < 0.5;
    let text = nonempty(&input.text_color)
        .map(str::to_string)
        .unwrap_or_else(|| {
            if is_dark_bg {
                "#ffffff".to_string()
            } else {
                "#212529".to_string()
            }
        });

    let primary_contrast = get_contrast_ratio(&primary, &bg);
    if primary_contrast < 4.5 {
        warnings.push("LOW_CONTRAST_PRIMARY".to_string());
    }

    let font = nonempty(&input.font_family)
        .unwrap_or("system-ui")
        .to_string();
    let font_face = if font != "system-ui" {
        let font_safe = font.replace(' ', "+");
        format!(
            "\n      @import url('https://fonts.googleapis.com/css2?family={font_safe}:wght@400;700&display=swap&subset=latin-ext');\n    "
        )
    } else {
        String::new()
    };

    let logo_block = match nonempty(&input.logo_url) {
        Some(url) => {
            format!("\n    .brand-logo {{\n      background-image: url('{url}');\n    }}\n    ")
        }
        None => String::new(),
    };

    let btn_text_color = if get_luminance(&primary) > 0.5 {
        "#000"
    } else {
        "#fff"
    };
    let primary_hover = adjust_color(&primary, -20);

    let raw_css = format!(
        "
    {font_face}
    :root {{
      --brand-primary: {primary};
      --brand-primary-hover: {primary_hover};
      --brand-secondary: {secondary};
      --brand-bg: {bg};
      --brand-text: {text};
      --brand-font: '{font}', system-ui, sans-serif;
      --brand-radius: 8px;
    }}

    body {{
      background-color: var(--brand-bg);
      color: var(--brand-text);
      font-family: var(--brand-font);
    }}

    .btn-primary {{
      background-color: var(--brand-primary);
      color: {btn_text_color};
      border: none;
      border-radius: var(--brand-radius);
      padding: 0.5rem 1rem;
      cursor: pointer;
    }}
    .btn-primary:hover {{
      background-color: var(--brand-primary-hover);
    }}

    .text-primary {{
      color: var(--brand-primary);
    }}

    {logo_block}
  "
    );

    let css = collapse_whitespace(&raw_css).trim().to_string();
    let css_hash = css_hash_of(&css);

    ThemeRendered {
        css,
        css_hash,
        version: current_version + 1,
        warnings,
    }
}

// ── DTOs ───────────────────────────────────────────────────────────────────────────────────

/// `location_themes` row projection (see module doc's schema-gap finding for why
/// `secondary_color`/`font_family` are `#[sqlx(default)]` — they are not real columns, so a real
/// `SELECT` never populates them, and `#[serde(skip_serializing_if)]` keeps them off the wire
/// exactly like a genuine Postgres `SELECT *` would (the key is simply absent, not `null`).
#[derive(Debug, Clone, sqlx::FromRow, Serialize, utoipa::ToSchema)]
pub struct OwnerThemeRow {
    pub location_id: Uuid,
    pub primary_color: Option<String>,
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
    pub bg_color: Option<String>,
    pub text_color: Option<String>,
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    pub logo_url: Option<String>,
    pub frame_ancestors: Vec<String>,
    /// `#[schema(value_type = String)]`: `utoipa` isn't built with the `chrono` feature in this
    /// workspace (`Cargo.toml` is on the "do not touch" list) — the wire shape is already an
    /// RFC3339 string via chrono's `serde` feature, this just tells the OpenAPI schema generator
    /// to describe it as one instead of requiring `chrono::DateTime<Utc>: ToSchema`.
    #[schema(value_type = String)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// PUT body (`themes.ts:49-56`'s `.strict()` schema). Every color field is `.nullable().optional()`
/// → the serde "double option" (see [`deserialize_some`]); `font_family` likewise but enum-typed;
/// `frame_ancestors` is `.optional()` only (not nullable) → a plain `Option<Vec<String>>` already
/// captures "absent vs present" correctly.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemeRequest {
    // `#[schema(value_type = ...)]`: sidesteps needing `ToSchema` on the double-Option
    // (`Option<Option<T>>`) shape or on `HexColor`/`ThemeFont` themselves — the wire type is
    // just an optional/nullable string either way.
    #[serde(default, deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<String>, nullable)]
    pub primary_color: Option<Option<HexColor>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<String>, nullable)]
    pub secondary_color: Option<Option<HexColor>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<String>, nullable)]
    pub font_family: Option<Option<ThemeFont>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<String>, nullable)]
    pub bg_color: Option<Option<HexColor>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<String>, nullable)]
    pub text_color: Option<Option<HexColor>>,
    #[serde(default)]
    pub frame_ancestors: Option<Vec<String>>,
}

/// The repo-facing update payload — `UpdateThemeRequest` with its validated newtypes unwrapped to
/// plain `String`s (the repo layer doesn't need to re-validate; that already happened at the
/// `Json` extractor boundary).
#[derive(Debug, Clone, Default)]
pub struct ThemeUpdates {
    pub primary_color: Option<Option<String>>,
    pub secondary_color: Option<Option<String>>,
    pub font_family: Option<Option<String>>,
    pub bg_color: Option<Option<String>>,
    pub text_color: Option<Option<String>>,
    pub frame_ancestors: Option<Vec<String>>,
}

impl From<UpdateThemeRequest> for ThemeUpdates {
    fn from(body: UpdateThemeRequest) -> Self {
        ThemeUpdates {
            primary_color: body.primary_color.map(|inner| inner.map(|h| h.0)),
            secondary_color: body.secondary_color.map(|inner| inner.map(|h| h.0)),
            font_family: body
                .font_family
                .map(|inner| inner.map(|f| f.as_str().to_string())),
            bg_color: body.bg_color.map(|inner| inner.map(|h| h.0)),
            text_color: body.text_color.map(|inner| inner.map(|h| h.0)),
            frame_ancestors: body.frame_ancestors,
        }
    }
}

/// Op 2's success payload (`themes.ts:105-109`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedThemeResult {
    pub css_hash: String,
    pub version: i32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetThemeResponse {
    pub theme: OwnerThemeRow,
    #[serde(rename = "cssHash")]
    pub css_hash: Option<String>,
    pub version: i32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UpdateThemeResponse {
    #[serde(rename = "cssHash")]
    pub css_hash: String,
    pub version: i32,
    pub warnings: Vec<String>,
}

// ── Repo trait + Pg/Fake impls ────────────────────────────────────────────────────────────────

/// Every method takes `owner_user_id` (in addition to the sketch in the build brief, which only
/// listed it on `update_and_render`) — required by the in-transaction membership re-check (module
/// doc): each method opens its OWN `with_user`-seated transaction, so each needs the seating value.
/// `Ok(None)` from `get_theme`/`update_and_render` covers BOTH "membership check failed" and (for
/// `get_theme`) "no theme row" — both are existence-hiding 404s at the handler, so collapsing them
/// is intentional, not a loss of information the caller needs. `update_and_render` returning
/// `Ok(None)` is a narrower deviation from the build brief's literal `Result<RenderedThemeResult,
/// _>` sketch, made to carry that same existence-hiding 404 distinctly from the TS "blows up on a
/// missing row"/schema-gap 500 paths (both of which are genuine `Err`s below).
#[async_trait::async_trait]
pub trait ThemesRepo: Send + Sync {
    async fn get_theme(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<OwnerThemeRow>, RepoError>;

    /// `(css_hash, version)` of the latest `theme_versions` row (`themes.ts:29-33`).
    async fn latest_version(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<(String, i32)>, RepoError>;

    async fn update_and_render(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        updates: ThemeUpdates,
    ) -> Result<Option<RenderedThemeResult>, RepoError>;
}

#[derive(Clone)]
pub struct ThemesState {
    pub auth: crate::auth::AuthState,
    pub repo: std::sync::Arc<dyn ThemesRepo>,
}

/// Collapses `db::with_user`'s transaction-lifecycle error into the plain `RepoError` every
/// `ThemesRepo` method returns — the closure's own `sqlx::Error`s (membership-check query,
/// `SELECT`/`UPDATE`/`INSERT`) surface via `TenantTxnError::Work`; `Begin`/`SetTenant`/`Commit`
/// are connection/transaction-lifecycle failures, not query failures, but collapse the same way
/// since `RepoError` has no room to distinguish them and callers only ever map any `Err` to a
/// generic 500 anyway.
fn map_txn_err(e: crate::db::TenantTxnError) -> RepoError {
    use crate::db::TenantTxnError as E;
    match e {
        E::Begin(err) | E::SetTenant(err) | E::Work(err) | E::Commit(err) => RepoError(err),
        E::WorkThenRollbackFailed { work, .. } => RepoError(work),
    }
}

pub struct PgThemesRepo {
    pool: sqlx::PgPool,
}

impl PgThemesRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgThemesRepo { pool }
    }
}

#[async_trait::async_trait]
impl ThemesRepo for PgThemesRepo {
    /// CARRY-DIVERGENCE: the old TS `themes.ts` GET (`themes.ts:23`) used a raw `db.connect()`
    /// with no GUC seated at all. This is a sanctioned fix-in-port (S3 breaker finding C2, pending
    /// council RESOLVE), not something to carry into the Rust port — routed through `with_user` +
    /// `assert_active_owner_membership` like every other S3 op. Separately, `theme_versions`' own
    /// RLS policy (if any) keys on a PostgREST GUC family (`request.jwt.claim.sub`) this Rust
    /// build does not seat — noted, not addressed here.
    async fn get_theme(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<OwnerThemeRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !crate::routes::owner::assert_active_owner_membership(
                    txn,
                    owner_user_id,
                    location_id,
                )
                .await?
                {
                    return Ok(None);
                }
                let row: Option<OwnerThemeRow> = sqlx::query_as(
                    "SELECT location_id, primary_color, bg_color, text_color, logo_url, \
                     frame_ancestors, updated_at
                       FROM location_themes WHERE location_id = $1",
                )
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn latest_version(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<(String, i32)>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !crate::routes::owner::assert_active_owner_membership(
                    txn,
                    owner_user_id,
                    location_id,
                )
                .await?
                {
                    return Ok(None);
                }
                // `version` is `bigint` (migration 1780338982030) — decode as i64, narrow to i32
                // only at the trait boundary; binding/decoding an i32 directly against a real
                // int8 column would mismatch sqlx's runtime type check.
                let row: Option<(String, i64)> = sqlx::query_as(
                    "SELECT css_hash, version FROM theme_versions
                      WHERE location_id = $1 ORDER BY version DESC LIMIT 1",
                )
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row.map(|(hash, v)| (hash, version_i32(v))))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn update_and_render(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        updates: ThemeUpdates,
    ) -> Result<Option<RenderedThemeResult>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !crate::routes::owner::assert_active_owner_membership(
                    txn,
                    owner_user_id,
                    location_id,
                )
                .await?
                {
                    return Ok(None);
                }

                if updates.secondary_color.is_some() || updates.font_family.is_some() {
                    // CARRY (verified schema gap — see module doc): `location_themes` has no
                    // secondary_color/font_family column. themes.ts's dynamic `SET ${f} = $n`
                    // (themes.ts:69) would issue `UPDATE location_themes SET secondary_color =
                    // $2 ...` against a column that does not exist, throwing a Postgres
                    // undefined_column error → caught by themes.ts:110-112's catch-all → 500.
                    // Reproduced as a genuine sqlx error rather than emitting the doomed SQL this
                    // sandbox has no live Postgres to run.
                    return Err(sqlx::Error::ColumnNotFound(
                        "secondary_color/font_family is not a real location_themes column \
                         (themes.ts's dynamic UPDATE would 500 here too — see themes.rs module doc)"
                            .to_string(),
                    ));
                }

                let ThemeUpdates {
                    primary_color,
                    bg_color,
                    text_color,
                    frame_ancestors,
                    ..
                } = updates;

                let mut qb =
                    sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE location_themes SET ");
                let mut any = false;
                if let Some(v) = primary_color {
                    qb.push("primary_color = ").push_bind(v);
                    any = true;
                }
                if let Some(v) = bg_color {
                    if any {
                        qb.push(", ");
                    }
                    qb.push("bg_color = ").push_bind(v);
                    any = true;
                }
                if let Some(v) = text_color {
                    if any {
                        qb.push(", ");
                    }
                    qb.push("text_color = ").push_bind(v);
                    any = true;
                }
                if let Some(v) = frame_ancestors {
                    if any {
                        qb.push(", ");
                    }
                    qb.push("frame_ancestors = ").push_bind(v);
                    any = true;
                }

                // `themes.ts:68`: `if (fields.length > 0)` — an update body with no recognized
                // fields at all skips the UPDATE entirely (still proceeds to render below).
                if any {
                    qb.push(", updated_at = now() WHERE location_id = ")
                        .push_bind(location_id);
                    qb.build().execute(&mut **txn).await?;
                }

                // CARRY (`themes.ts:78-79`): no rowcount check — `fetch_one` (not
                // `fetch_optional`) so a genuinely-missing row surfaces as `RowNotFound`, the
                // same observable "blows up" outcome as `theme.primary_color` on `undefined`.
                let row: OwnerThemeRow = sqlx::query_as(
                    "SELECT location_id, primary_color, bg_color, text_color, logo_url, \
                     frame_ancestors, updated_at
                       FROM location_themes WHERE location_id = $1",
                )
                .bind(location_id)
                .fetch_one(&mut **txn)
                .await?;

                let current_version: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(version), 0) FROM theme_versions WHERE location_id = $1",
                )
                .bind(location_id)
                .fetch_one(&mut **txn)
                .await?;

                let input = ThemeInput {
                    primary_color: row.primary_color,
                    secondary_color: None, // CARRY: not a real column, see module doc
                    font_family: None,     // CARRY: not a real column, see module doc
                    bg_color: row.bg_color,
                    text_color: row.text_color,
                    logo_url: row.logo_url,
                };
                let rendered = render_theme(&input, version_i32(current_version));

                // CARRY (`themes.ts:96-109`): `ON CONFLICT (location_id, css_hash) DO NOTHING`
                // dedupes an identical-CSS re-render — the response's `version` below is still
                // `current_version + 1` regardless of whether this INSERT actually stored a row.
                sqlx::query(
                    "INSERT INTO theme_versions (location_id, css_hash, css_body, version)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (location_id, css_hash) DO NOTHING",
                )
                .bind(location_id)
                .bind(&rendered.css_hash)
                .bind(&rendered.css)
                .bind(i64::from(rendered.version))
                .execute(&mut **txn)
                .await?;

                Ok(Some(RenderedThemeResult {
                    css_hash: rendered.css_hash,
                    version: rendered.version,
                    warnings: rendered.warnings,
                }))
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── Handlers ───────────────────────────────────────────────────────────────────────────────────

/// `GET /api/owner/locations/{locationId}/theme` — source: `themes.ts:17-43`.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/theme",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Theme row + latest rendered CSS pointer", body = GetThemeResponse),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn get_owner_theme(
    Extension(state): Extension<ThemesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    crate::routes::owner::require_location_access(
        &state.auth,
        &owner,
        location_id,
        &correlation_id,
    )
    .await?;

    let theme = state
        .repo
        .get_theme(owner.user_id, location_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                domain::ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    let Some(theme) = theme else {
        return Err(ApiError::new(
            domain::ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    };

    let version = state
        .repo
        .latest_version(owner.user_id, location_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                domain::ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    let (css_hash, version_num) = match version {
        Some((hash, v)) => (Some(hash), v),
        None => (None, 0),
    };

    Ok(Json(GetThemeResponse {
        theme,
        css_hash,
        version: version_num,
    }))
}

/// `PUT /api/owner/locations/{locationId}/theme` — source: `themes.ts:46-116`. No explicit error
/// path in the TS beyond the rollback+rethrow catch-all (`themes.ts:110-115`) — see module doc for
/// exactly which failures map to 404 (membership) vs 500 (missing row / schema-gap fields).
#[utoipa::path(
    put,
    path = "/api/owner/locations/{locationId}/theme",
    params(("locationId" = Uuid, Path)),
    request_body = UpdateThemeRequest,
    responses(
        (status = 200, description = "Updated + re-rendered CSS pointer", body = UpdateThemeResponse),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn put_owner_theme(
    Extension(state): Extension<ThemesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<UpdateThemeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    crate::routes::owner::require_location_access(
        &state.auth,
        &owner,
        location_id,
        &correlation_id,
    )
    .await?;

    let updates = ThemeUpdates::from(body);

    let result = state
        .repo
        .update_and_render(owner.user_id, location_id, updates)
        .await
        .map_err(|_err| {
            ApiError::new(
                domain::ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    let Some(rendered) = result else {
        return Err(ApiError::new(
            domain::ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    };

    Ok(Json(UpdateThemeResponse {
        css_hash: rendered.css_hash,
        version: rendered.version,
        warnings: rendered.warnings,
    }))
}

// ── Fake repo (cfg(test)) ────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    //! `FakeThemesRepo` — in-memory, `Mutex<HashMap<...>>`-backed, mirroring `crate::repo::fake`'s
    //! `FakeRepo` / `crate::auth::repo::fake`'s `FakeAuthRepo` style. Deliberately does NOT model
    //! the membership check (`assert_active_owner_membership`) — that invariant is proven at the
    //! extractor/`require_location_access` layer (using `FakeAuthRepo`) in every handler test
    //! below; this fake trusts its caller, same as `FakeRepo`'s S1 methods trust theirs.
    use std::collections::HashMap;
    use std::sync::Mutex;

    use uuid::Uuid;

    use super::{
        OwnerThemeRow, RenderedThemeResult, ThemeInput, ThemeUpdates, ThemesRepo, render_theme,
    };
    use crate::repo::RepoError;

    #[derive(Debug, Clone, Default)]
    pub struct StoredTheme {
        pub primary_color: Option<String>,
        pub bg_color: Option<String>,
        pub text_color: Option<String>,
        pub logo_url: Option<String>,
        pub frame_ancestors: Vec<String>,
    }

    #[derive(Default)]
    pub struct FakeThemesRepo {
        pub themes: Mutex<HashMap<Uuid, StoredTheme>>,
        /// `location_id -> [(css_hash, version)]` — the versions ACTUALLY stored (post
        /// ON-CONFLICT dedup), distinct from what a response REPORTS (see
        /// `update_and_render_dedup_reports_a_version_ahead_of_actual_storage`).
        pub versions: Mutex<HashMap<Uuid, Vec<(String, i32)>>>,
    }

    #[async_trait::async_trait]
    impl ThemesRepo for FakeThemesRepo {
        async fn get_theme(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<OwnerThemeRow>, RepoError> {
            let themes = self.themes.lock().unwrap();
            Ok(themes.get(&location_id).cloned().map(|row| OwnerThemeRow {
                location_id,
                primary_color: row.primary_color,
                secondary_color: None,
                bg_color: row.bg_color,
                text_color: row.text_color,
                font_family: None,
                logo_url: row.logo_url,
                frame_ancestors: row.frame_ancestors,
                updated_at: chrono::Utc::now(),
            }))
        }

        async fn latest_version(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<(String, i32)>, RepoError> {
            let versions = self.versions.lock().unwrap();
            Ok(versions
                .get(&location_id)
                .and_then(|v| v.iter().max_by_key(|(_, ver)| *ver).cloned()))
        }

        async fn update_and_render(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            updates: ThemeUpdates,
        ) -> Result<Option<RenderedThemeResult>, RepoError> {
            if updates.secondary_color.is_some() || updates.font_family.is_some() {
                // Mirrors PgThemesRepo's schema-gap short-circuit (see themes.rs module doc) —
                // proven end-to-end via the fake in
                // `put_owner_theme_500_when_secondary_color_is_present`.
                return Err(RepoError(sqlx::Error::ColumnNotFound(
                    "secondary_color/font_family is not a real location_themes column".to_string(),
                )));
            }

            {
                let mut themes = self.themes.lock().unwrap();
                if let Some(row) = themes.get_mut(&location_id) {
                    if let Some(v) = updates.primary_color {
                        row.primary_color = v;
                    }
                    if let Some(v) = updates.bg_color {
                        row.bg_color = v;
                    }
                    if let Some(v) = updates.text_color {
                        row.text_color = v;
                    }
                    if let Some(v) = updates.frame_ancestors {
                        row.frame_ancestors = v;
                    }
                }
                // If absent: TS's UPDATE silently affects 0 rows (no insert) — matching the
                // no-rowcount-check carry below.
            }

            let row = self.themes.lock().unwrap().get(&location_id).cloned();
            let Some(row) = row else {
                // CARRY (themes.ts:79): `theme.primary_color` on `undefined` — genuinely missing
                // row, mirrors PgThemesRepo's `fetch_one` -> RowNotFound path.
                return Err(RepoError(sqlx::Error::RowNotFound));
            };

            let current_version = {
                let versions = self.versions.lock().unwrap();
                versions
                    .get(&location_id)
                    .and_then(|v| v.iter().map(|(_, ver)| *ver).max())
                    .unwrap_or(0)
            };

            let input = ThemeInput {
                primary_color: row.primary_color,
                secondary_color: None,
                font_family: None,
                bg_color: row.bg_color,
                text_color: row.text_color,
                logo_url: row.logo_url,
            };
            let rendered = render_theme(&input, current_version);

            {
                let mut versions = self.versions.lock().unwrap();
                let entry = versions.entry(location_id).or_default();
                if !entry.iter().any(|(h, _)| h == &rendered.css_hash) {
                    entry.push((rendered.css_hash.clone(), rendered.version));
                }
                // else: ON CONFLICT (location_id, css_hash) DO NOTHING — no new row stored, but
                // the returned `rendered.version` below still reports current_version + 1.
            }

            Ok(Some(RenderedThemeResult {
                css_hash: rendered.css_hash,
                version: rendered.version,
                warnings: rendered.warnings,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{FakeThemesRepo, StoredTheme};
    use super::*;
    use crate::auth::AuthState;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::{Arc, Mutex};

    // ── pure color-math ──────────────────────────────────────────────────────────────────────

    #[test]
    fn get_luminance_of_black_is_zero() {
        assert_eq!(get_luminance("#000000"), 0.0);
    }

    #[test]
    fn get_luminance_of_white_is_one() {
        assert!((get_luminance("#ffffff") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn get_luminance_of_a_midtone_matches_hand_computation() {
        // #808080: each channel v=128/255≈0.5020 > 0.03928, so channel = ((v+0.055)/1.055)^2.4.
        let v: f64 = 128.0 / 255.0;
        let channel = ((v + 0.055) / 1.055).powf(2.4);
        let expected = channel * (0.2126 + 0.7152 + 0.0722); // same value on all 3 channels
        assert!((get_luminance("#808080") - expected).abs() < 1e-9);
    }

    #[test]
    fn get_contrast_ratio_black_vs_white_is_21() {
        assert!((get_contrast_ratio("#000000", "#ffffff") - 21.0).abs() < 1e-6);
    }

    #[test]
    fn get_contrast_ratio_is_order_independent() {
        assert_eq!(
            get_contrast_ratio("#123456", "#abcdef"),
            get_contrast_ratio("#abcdef", "#123456")
        );
    }

    #[test]
    fn adjust_color_lightens_and_clamps_at_255() {
        assert_eq!(adjust_color("#ffffff", 20), "#ffffff");
    }

    #[test]
    fn adjust_color_darkens_and_clamps_at_0() {
        assert_eq!(adjust_color("#000000", -20), "#000000");
    }

    #[test]
    fn adjust_color_reproduces_the_green_blue_channel_swap() {
        // Hand-computed per theme-renderer.ts:44-53's ACTUAL (buggy) recombination order:
        // r=0x01,g=0x02,b=0x03, amount=0 -> combined = g | (b<<8) | (r<<16)
        //   = 0x02 | (0x03<<8) | (0x01<<16) = 0x010302.
        // A "correct" #RRGGBB rebuild would have produced "#010203" (unchanged, amount=0) —
        // the green and blue channels are visibly swapped in the actual output.
        assert_eq!(adjust_color("#010203", 0), "#010302");
    }

    // ── render_theme ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn render_theme_applies_documented_defaults() {
        let rendered = render_theme(&ThemeInput::default(), 0);
        assert!(rendered.css.contains("#e63946"), "primary default");
        assert!(rendered.css.contains("#457b9d"), "secondary default");
        assert!(rendered.css.contains("#ffffff"), "bg default");
        assert_eq!(rendered.version, 1);
    }

    #[test]
    fn render_theme_empty_string_input_falls_back_like_js_truthy_or() {
        let with_empty = render_theme(
            &ThemeInput {
                primary_color: Some(String::new()),
                ..Default::default()
            },
            0,
        );
        let with_none = render_theme(&ThemeInput::default(), 0);
        assert_eq!(with_empty.css_hash, with_none.css_hash);
    }

    #[test]
    fn render_theme_low_contrast_primary_warning_fires() {
        let rendered = render_theme(
            &ThemeInput {
                primary_color: Some("#e0e0e0".to_string()),
                bg_color: Some("#ffffff".to_string()),
                ..Default::default()
            },
            0,
        );
        assert!(
            rendered
                .warnings
                .contains(&"LOW_CONTRAST_PRIMARY".to_string())
        );
    }

    #[test]
    fn render_theme_high_contrast_primary_no_warning() {
        let rendered = render_theme(
            &ThemeInput {
                primary_color: Some("#000000".to_string()),
                bg_color: Some("#ffffff".to_string()),
                ..Default::default()
            },
            0,
        );
        assert!(rendered.warnings.is_empty());
    }

    #[test]
    fn render_theme_css_hash_is_deterministic() {
        let a = render_theme(&ThemeInput::default(), 3);
        let b = render_theme(&ThemeInput::default(), 3);
        assert_eq!(a.css_hash, b.css_hash);
        assert_eq!(a.css_hash.len(), 16);
    }

    #[test]
    fn render_theme_css_hash_differs_for_different_input() {
        let a = render_theme(&ThemeInput::default(), 0);
        let b = render_theme(
            &ThemeInput {
                primary_color: Some("#123456".to_string()),
                ..Default::default()
            },
            0,
        );
        assert_ne!(a.css_hash, b.css_hash);
    }

    #[test]
    fn render_theme_version_is_current_plus_one() {
        assert_eq!(render_theme(&ThemeInput::default(), 5).version, 6);
    }

    #[test]
    fn render_theme_google_font_import_for_non_system_ui_font() {
        let rendered = render_theme(
            &ThemeInput {
                font_family: Some("Source Sans 3".to_string()),
                ..Default::default()
            },
            0,
        );
        assert!(rendered.css.contains("fonts.googleapis.com"));
        assert!(rendered.css.contains("family=Source+Sans+3"));
    }

    #[test]
    fn render_theme_no_font_import_for_system_ui_default() {
        let rendered = render_theme(&ThemeInput::default(), 0);
        assert!(!rendered.css.contains("fonts.googleapis.com"));
    }

    // ── DTO validation ───────────────────────────────────────────────────────────────────────

    #[test]
    fn update_theme_request_rejects_unknown_field() {
        let json = serde_json::json!({ "primary_color": "#ffffff", "nope": true });
        assert!(serde_json::from_value::<UpdateThemeRequest>(json).is_err());
    }

    #[test]
    fn update_theme_request_rejects_invalid_hex_color() {
        let json = serde_json::json!({ "primary_color": "red" });
        assert!(serde_json::from_value::<UpdateThemeRequest>(json).is_err());
    }

    #[test]
    fn update_theme_request_rejects_invalid_font_family() {
        let json = serde_json::json!({ "font_family": "Comic Sans" });
        assert!(serde_json::from_value::<UpdateThemeRequest>(json).is_err());
    }

    #[test]
    fn update_theme_request_distinguishes_absent_present_null_and_present_value() {
        let absent: UpdateThemeRequest = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(absent.primary_color, None);

        let explicit_null: UpdateThemeRequest =
            serde_json::from_value(serde_json::json!({ "primary_color": null })).unwrap();
        assert_eq!(explicit_null.primary_color, Some(None));

        let present: UpdateThemeRequest =
            serde_json::from_value(serde_json::json!({ "primary_color": "#ffffff" })).unwrap();
        assert_eq!(
            present.primary_color,
            Some(Some(HexColor("#ffffff".to_string())))
        );
    }

    // ── handlers (FakeThemesRepo + FakeAuthRepo) ────────────────────────────────────────────

    fn test_state(
        user_id: Uuid,
        active_location: Option<Uuid>,
        repo: FakeThemesRepo,
    ) -> ThemesState {
        let auth_repo = Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new(
                active_location
                    .map(|loc| [(user_id, vec![loc])].into_iter().collect())
                    .unwrap_or_default(),
            ),
            ..Default::default()
        });
        ThemesState {
            auth: AuthState::test_state(auth_repo),
            repo: Arc::new(repo),
        }
    }

    fn owner_ext(user_id: Uuid) -> OwnerClaimsExt {
        OwnerClaimsExt(OwnerClaims::new(user_id, None))
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    fn empty_update() -> UpdateThemeRequest {
        UpdateThemeRequest {
            primary_color: None,
            secondary_color: None,
            font_family: None,
            bg_color: None,
            text_color: None,
            frame_ancestors: None,
        }
    }

    #[tokio::test]
    async fn get_owner_theme_200_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes.lock().unwrap().insert(
            loc,
            StoredTheme {
                primary_color: Some("#111111".to_string()),
                frame_ancestors: vec!["self".to_string()],
                ..Default::default()
            },
        );
        repo.versions
            .lock()
            .unwrap()
            .insert(loc, vec![("abc0123456789def".to_string(), 1)]);
        let state = test_state(user_id, Some(loc), repo);

        let response = get_owner_theme(
            Extension(state),
            owner_ext(user_id),
            Path(loc),
            request_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["theme"]["primary_color"], "#111111");
        assert_eq!(body["cssHash"], "abc0123456789def");
        assert_eq!(body["version"], 1);
        assert!(
            body["theme"].get("secondary_color").is_none(),
            "phantom field must never appear on the wire"
        );
    }

    #[tokio::test]
    async fn get_owner_theme_404_when_no_theme_row() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(user_id, Some(loc), FakeThemesRepo::default());
        let err = crate::error::expect_err(
            get_owner_theme(
                Extension(state),
                owner_ext(user_id),
                Path(loc),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, domain::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_owner_theme_404_cross_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes
            .lock()
            .unwrap()
            .insert(theirs, StoredTheme::default());
        let state = test_state(user_id, Some(mine), repo);
        let err = crate::error::expect_err(
            get_owner_theme(
                Extension(state),
                owner_ext(user_id),
                Path(theirs),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, domain::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn put_owner_theme_200_only_updates_present_fields() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes.lock().unwrap().insert(
            loc,
            StoredTheme {
                primary_color: Some("#111111".to_string()),
                bg_color: Some("#222222".to_string()),
                text_color: None,
                logo_url: None,
                frame_ancestors: vec!["self".to_string()],
            },
        );
        let state = test_state(user_id, Some(loc), repo);
        let mut body = empty_update();
        body.text_color = Some(Some(HexColor("#333333".to_string())));

        let response = put_owner_theme(
            Extension(state.clone()),
            owner_ext(user_id),
            Path(loc),
            request_id(),
            Json(body),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let after = state.repo.get_theme(user_id, loc).await.unwrap().unwrap();
        assert_eq!(after.primary_color.as_deref(), Some("#111111"), "untouched");
        assert_eq!(after.bg_color.as_deref(), Some("#222222"), "untouched");
        assert_eq!(after.text_color.as_deref(), Some("#333333"), "updated");
    }

    #[tokio::test]
    async fn put_owner_theme_200_response_has_css_hash_version_and_warnings() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes
            .lock()
            .unwrap()
            .insert(loc, StoredTheme::default());
        let state = test_state(user_id, Some(loc), repo);

        let response = put_owner_theme(
            Extension(state),
            owner_ext(user_id),
            Path(loc),
            request_id(),
            Json(empty_update()),
        )
        .await
        .unwrap()
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: UpdateThemeResponse_ = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.css_hash.len(), 16);
        assert_eq!(body.version, 1);
    }

    // Local mirror of the wire shape (avoids depending on UpdateThemeResponse's Serialize-only
    // shape for round-tripping in a test — this crate has no Deserialize on that response type).
    #[derive(Debug, serde::Deserialize)]
    struct UpdateThemeResponse_ {
        #[serde(rename = "cssHash")]
        css_hash: String,
        version: i32,
        #[allow(dead_code)]
        warnings: Vec<String>,
    }

    #[tokio::test]
    async fn put_owner_theme_404_cross_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes
            .lock()
            .unwrap()
            .insert(theirs, StoredTheme::default());
        let state = test_state(user_id, Some(mine), repo);
        let err = crate::error::expect_err(
            put_owner_theme(
                Extension(state),
                owner_ext(user_id),
                Path(theirs),
                request_id(),
                Json(empty_update()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, domain::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn put_owner_theme_500_when_secondary_color_is_present() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes
            .lock()
            .unwrap()
            .insert(loc, StoredTheme::default());
        let state = test_state(user_id, Some(loc), repo);
        let mut body = empty_update();
        body.secondary_color = Some(Some(HexColor("#456789".to_string())));

        let err = crate::error::expect_err(
            put_owner_theme(
                Extension(state),
                owner_ext(user_id),
                Path(loc),
                request_id(),
                Json(body),
            )
            .await,
        );
        assert_eq!(err.envelope.code, domain::ErrorCode::Internal);
    }

    #[tokio::test]
    async fn put_owner_theme_500_when_theme_row_missing() {
        // Membership passes (extractor-level), but no `location_themes` row exists at all —
        // CARRY: themes.ts's no-rowcount-check bug (themes.ts:78-79), not a 404.
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(user_id, Some(loc), FakeThemesRepo::default());
        let err = crate::error::expect_err(
            put_owner_theme(
                Extension(state),
                owner_ext(user_id),
                Path(loc),
                request_id(),
                Json(empty_update()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, domain::ErrorCode::Internal);
    }

    #[tokio::test]
    async fn update_and_render_dedup_reports_a_version_ahead_of_actual_storage() {
        // CARRY (themes.ts:96-109): an identical-CSS re-render dedupes at the storage layer (ON
        // CONFLICT DO NOTHING) but the reported `version` still increments past what's stored.
        let loc = Uuid::new_v4();
        let repo = FakeThemesRepo::default();
        repo.themes
            .lock()
            .unwrap()
            .insert(loc, StoredTheme::default());
        let user_id = Uuid::new_v4();

        let first = repo
            .update_and_render(user_id, loc, ThemeUpdates::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.version, 1);

        let second = repo
            .update_and_render(user_id, loc, ThemeUpdates::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.version, 2, "response keeps incrementing");
        assert_eq!(
            second.css_hash, first.css_hash,
            "identical input -> identical CSS"
        );

        let stored = repo.versions.lock().unwrap().get(&loc).unwrap().clone();
        assert_eq!(
            stored,
            vec![(first.css_hash.clone(), 1)],
            "dedup: only ONE row actually stored, even though the response reported version 2"
        );
    }
}
