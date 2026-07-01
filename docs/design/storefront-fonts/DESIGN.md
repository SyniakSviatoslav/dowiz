# Per-tenant storefront fonts (heading + body) — design & exit

**Goal:** product titles + descriptions (and all storefront headings) render in a per-tenant,
brand-appropriate font instead of the hardcoded `'Playfair Display' … serif`. Font is *sourced*
tiered (extract → cuisine-default → owner-confirm), *loaded* safely (allowlist), *applied* via the
existing `--brand-font-heading` / `--brand-font-body` CSS-var tokens.

## Current state (verified 2026-07-01)
- Tokens `--brand-font-heading` / `--brand-font-body` exist (`packages/ui/src/theme/tokens.css`) and
  are consumed by category headers, dialogs, the header title — but **NOT** by `ProductCard` title
  (`<h3>` `ProductCard.tsx:130`) or description (`<p>` `:155`), which render default sans. ← the visible bug.
- `ThemeProvider` applies color vars only — never fonts.
- `ClientLayout.tsx:143` HARDCODES `--brand-font-heading: 'Playfair Display', 'Cormorant Garamond', Georgia, serif`
  inline → overrides any tenant value.
- `location_themes.font_url text` column exists (mig …82030) but nothing reads it.
- Fonts are loaded from a FIXED `<link>` in `apps/web/index.html`: Inter, DM Sans, DM Serif Display,
  Cormorant Garamond, Playfair Display, Fraunces, Yeseva One.
- Theme served by `GET /api/public/theme/:slug` (`spa-proxy.ts:483`) → `{primaryColor,bgColor,textColor,logoUrl,…}`.
- Owner theme read/write: `GET`/`PUT /api/owner/brand` (`spa-proxy.ts:502/533`) — the admin theme editor backend.

## Where a font can come from (honest signal audit)
- **Website (Tier 1)** — reliable, free, automatable: parse `@font-face` + Google/Adobe `<link>`/`@import`
  + computed `font-family` on headings/body (headless browser). Only helps prospects that HAVE a `website_url`.
- **Logo image** — DeepFont ~80% top-5, WhatFontIs ~90% (needs clean crop), paid/non-deterministic. DEFERRED:
  surface as an owner suggestion only, never auto-apply.
- **place_id + Wolt (the common case, incl. ArtePasta)** — NO brand-font signal (Wolt uses its own Omnes).
  → must fall back to a cuisine/character default. This is why Tier 0 (curated default) is the real fix.

## Font allowlist (the ONLY families that may be selected/loaded)
Keyed by a short id; the id is what we store/transmit — never a raw URL. Each maps to a Google-Fonts
family + a loader spec. Server + client both validate the id against this list.

| id | family | role | google family+weights |
|----|--------|------|-----------------------|
| `playfair` | Playfair Display | heading | `Playfair+Display:wght@400;500;600;700` |
| `cormorant` | Cormorant Garamond | heading | `Cormorant+Garamond:wght@400;500;600;700` |
| `dmserif` | DM Serif Display | heading | `DM+Serif+Display` |
| `fraunces` | Fraunces | heading | `Fraunces:opsz,wght@9..144,400..700` |
| `yeseva` | Yeseva One | heading | `Yeseva+One` |
| `spacegrotesk` | Space Grotesk | heading | `Space+Grotesk:wght@400;500;600;700` |
| `bebas` | Bebas Neue | heading | `Bebas+Neue` |
| `poppins` | Poppins | heading/body | `Poppins:wght@400;500;600;700` |
| `montserrat` | Montserrat | heading/body | `Montserrat:wght@400;500;600;700` |
| `inter` | Inter | body | `Inter:wght@400;500;600;700` |
| `dmsans` | DM Sans | body | `DM+Sans:wght@400;500;600;700` |

First 7 + Inter/DM Sans are ALREADY in the index.html `<link>` (no new load). `spacegrotesk`/`bebas`/
`poppins`/`montserrat` are Tier-1 additions loaded on demand.

## Cuisine → default pairing (Tier 0 seed, always available)
`{ heading, body }` by cuisine (fallback `default`): italian/pizzeria→`{fraunces,dmsans}`,
sushi/japanese→`{cormorant,dmsans}`, burger/american→`{bebas,inter}`, cafe/bakery→`{fraunces,inter}`,
kebab/street→`{dmsans,dmsans}`, fine-dining→`{cormorant,inter}`, default→`{playfair,inter}`.
Lives in `packages/ui/src/theme/fonts.ts` (single source; reused by SPA + demo-builder + SSR).

## Schema (Tier 2 persistence)
Additive migration on `location_themes` (RLS/FORCE already on):
`ADD COLUMN heading_font text` + `ADD COLUMN body_font text` — store the allowlist **id** (not URL).
`font_url` left as-is (legacy, unused). App-layer validates the id ∈ allowlist on write; a non-allowlisted
value is rejected (owner PUT) / coerced to the cuisine default (server read).

## Data flow
1. **API** `/public/theme/:slug`: return `headingFont`,`bodyFont` = `location_themes.{heading_font,body_font}`
   if set & allowlisted, else the cuisine default (join `locations`/source for cuisine; default if unknown).
2. **SPA** `derivePalette` → `ThemeConfig` gains `fontHeading`,`fontBody` (resolved CSS font-stacks from ids).
3. **ThemeProvider** applies `--brand-font-heading`/`--brand-font-body`. `ClientLayout` hardcode REMOVED.
4. **Font loader** (`ensureFontsLoaded(ids)`): if an id isn't in the base `<link>`, inject
   `<link rel=stylesheet href="https://fonts.googleapis.com/css2?family=…&display=swap">` built from the
   allowlist spec — id→URL via a FIXED template, deduped, idempotent. Never from tenant free-text.
5. **ProductCard**: title `<h3>` gets `fontFamily: var(--brand-font-heading)`; description keeps body.

## THREAT MODEL — dynamic font injection (the one security-sensitive part)
- **Risk:** injecting an attacker-controlled URL into a `<link href>` = external egress / CSS exfiltration / XSS-ish.
- **Controls:** (1) the network value is a fixed allowlist **id**, never a URL or family string; (2) the URL is
  built server/client-side from a hardcoded `https://fonts.googleapis.com/css2?family=<spec>` template where
  `<spec>` comes from OUR allowlist table, not tenant input; (3) an id ∉ allowlist → dropped (fall to default),
  never rendered; (4) CSP already allows `fonts.googleapis.com`/`fonts.gstatic.com` (existing `<link>`); no CSP
  widening. (5) owner PUT Zod-validates `heading_font`/`body_font` as `enum(allowlistIds)`.
- **Not PII, not money, not auth.** Migration is additive & reversible. `/s/demo` regression radius: the demo
  has no font ids → falls to `default` pairing (Playfair/Inter) ⇒ near-identical to today's hardcoded Playfair.

## Phases & exit (proof per phase — Mandatory Proof Rule)
- **P0 — foundation+visible win:** `fonts.ts` (allowlist+cuisine map+resolver), `ThemeConfig.font*`,
  `derivePalette` fonts, `ThemeProvider` applies font vars, remove `ClientLayout` hardcode, `ProductCard`
  title uses heading token, API returns cuisine-default `headingFont`/`bodyFont`.
  **Exit:** Playwright on staging `/s/artepasta` — product title `<h3>` computed `font-family` contains the
  cuisine-default heading family (NOT the old hardcoded Playfair for a non-Italian, AND correct for Italian);
  `/s/demo` unchanged. Typecheck green.
- **P1 — website extraction:** demo-builder enrichment: given `website_url`, headless-extract fonts → map to
  nearest allowlist id → persist. Fallback = cuisine default. **Exit:** unit test mapping detected family→id;
  a prospect with a website resolves a non-default id (fixture).
- **P2 — owner picker + dynamic load:** admin theme-editor font-picker (heading/body dropdowns of allowlist),
  `PUT /owner/brand` persists validated ids, `ensureFontsLoaded` injects on-demand for non-base ids.
  **Exit:** Playwright — owner sets a non-base font (e.g. Bebas) → storefront title renders it (font loaded +
  applied); reload persists; non-allowlist value rejected 422.

## Guardrail
A test asserting `ProductCard` title consumes `--brand-font-heading` + a unit asserting every cuisine-map
value and every owner-selectable id ∈ the allowlist (no orphan font ⇒ no unloaded font ⇒ no silent fallback).
