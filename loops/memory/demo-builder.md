# Пам'ять петлі · demo-builder

«Без запису що сталося — немає покращення, і палиш токени на тих самих граблях.»

## Runbook (operator)

Preview-only (default — no outreach):
```
PROVISION_BASE_URL=https://dowiz-staging.fly.dev \
STOREFRONT_BASE_URL=https://dowiz-staging.fly.dev \
PROVISION_OPS_SECRET=*** \
node scripts/demo-builder.mjs ./prospects.json
```
Add `--send-invite` to ALSO mint the claim token (still preview-safe until then; delivery is a separate human act).

Prospect row: `{ place_id, slug, name, website_url? , menu[]? , cuisine?, phone?, invited_contact? }`.
- `website_url` present → Layer 1 uses the shipped AI extract (its H4 ENRICHED/LOW_QUALITY verdict is the menu-quality gate).
- `menu[]` present (array of `{name, category, price(int minor), description?, bom?}`) → Layer 1 quality-gates it CLIENT-SIDE before any provisioning.
- `cuisine` (pizzeria/sushi/burger/cafe/italian/kebab/…) seeds Layer 2's palette.

### The two operator DB seams (non-AI menu + palette persistence)
The shipped /internal surface has NO endpoint to (a) set a non-AI `menu_draft` or (b) write `location_themes`. Rather than
invent phantom endpoints (M6), demo-builder uses the SAME sanctioned operator DB seam the sibling acquisition loop certified:
1. **Enrich seam** (non-AI menu): `UPDATE acquisition_sources SET menu_draft = $draft WHERE id = $src;` then advance to ENRICHED
   (the loop's `normalizeMenu` + `gateMenu` produce/validate `$draft`; the spine reads `menu_draft` and writes products in-tx).
2. **Theme seam** (palette): after spine returns `location_id`, `INSERT INTO location_themes (location_id, primary_color, bg_color, text_color) VALUES (…)`
   from the run report's `theme_directive`. The storefront's `derivePalette` expands the triple into the full token set.
Both run with operator Postgres creds (staging: see MEMORY staging-db-access recipe). The loop RECORDS the exact directive per prospect.

## Brand-asset seams (added 2026-07-01 · ArtePasta real-identity build)
Real venue identity from Google-Maps/Wolt → storefront. Storefront support (mapped): logo ✅, colors ✅,
menu-item photos ✅ (all DB/R2 seams); hero=video-only (image needs a render change); per-tenant font ❌.
- **Menu (real)**: `tools/demo-builder/wolt-menu-extract.mjs` parses a Wolt venue page's inline item JSON
  (name/price÷100/description/image, no API key) → normalize (translate SQ→EN, categorise, ingredients).
- **Re-seed an already-provisioned shadow** (menu changed): `SELECT erase_shadow_tenant(loc,org)` — but DELETE
  `location_themes` FIRST (fn doesn't, FK blocks) — then reset the source `state='ENRICHED', menu_draft=NEW,
  org_id/location_id=NULL`, then loop pass 2 re-spines. migrations role = `postgres` (BYPASSRLS) so DB works;
  the ops `/provision/hard-delete` endpoint 500'd (API role can't run the DEFINER fn).
- **Logo seam**: sharp-crop the sign from a Maps photo → 512² webp → PUT to R2 `locations/{id}/logo.webp`
  (`@aws-sdk/client-s3`, ContentType `image/webp` — the R2 gotcha) → `UPDATE location_themes.logo_url =
  getImageUrl(key)` (`{APP_BASE}/images/{key}` when R2_PUBLIC_URL unset). Header renders it (ClientLayout).
- **Ingredient BADGES on a shadow — ✅ CLOSED (b3f75ed5, 2026-07-01)**: `read_preview_menu` does
  `attributes - 'bom'` (C2 allergen gate) so bom-derived badges vanish pre-claim. FIX SHIPPED: normalizeMenu
  emits display-only `attributes.ingredients` string[] + `image_url` + `description_sq` — all three SURVIVE
  the `- 'bom'` strip (only `bom` is removed). MenuPage: getImageUrl falls back to `attributes.image_url`
  (uploaded image_key still wins), ingredient block prefers `attributes.ingredients` else BOM, modal shows
  `attributes.description_sq` (`p[lang=sq]`) under the EN desc. C2 NOT weakened (bom still stripped; proven
  bom=0 on the live preview payload). Proof: `e2e/tests/storefront-brand-ingest.spec.ts` green mobile+desktop
  on staging v230 (/s/artepasta: 38 photos, 31 ingredient lists, 28 sq descriptions).
- **Hero — ✅ CLOSED (0c713c64)**: signage photo renders as a still `<img>` backdrop behind vendor-info.
- **Font (todo)**: storefront hardcodes serif; wire a per-tenant heading/body font (SHARED-render change,
  regress-radius on /s/demo, needs a web deploy).

## Уроки (lessons learned)
- 2026-07-01 — BUILD+CERTIFY. The loop's whole reason to exist is the gap the raw provisioner leaves: the raw provisioner
  stops at API `verified:true`, which only proves the preview endpoint served + has ≥1 item. That is NOT "looks like a demo."
  demo-builder adds three gates on top: (L1) menu-quality BEFORE provisioning, (L2) a coherent cuisine palette, (L3) a RENDER
  gate on mobile+desktop. L3 is the differentiator — the anti-cheat dry-run proves a source that is API-verified:true can still
  render EMPTY / ERRORED / ORDERABLE / NOINDEXLESS, and the loop MUST land it needs-review, never certified.
- 2026-07-01 — no-fake-green on a VISUAL gate is the sharp edge: the cheap cheat is "HTTP 200 = the storefront works." The gate
  asserts on the RENDERED DOM (assertPreviewDom): ≥3 `data-testid="menu-item"` cards, `venue-preview-banner`, `preview-claim-cta`,
  0 console errors, NO `menu-item-add`/`cart-open`/checkout, noindex. Live = real Playwright (captures real `console`/`pageerror`);
  dry-run = an HTTP probe feeding the SAME assertPreviewDom (a fake storefront serves the broken variants). Same logic, two backends.
- 2026-07-01 — quality-before-provision: gateMenu (≥2 cats, ≥6 items, ≥50% descriptions, integer prices in a sane band) runs BEFORE
  mint/spine. A thin menu → needs-review:LOW_QUALITY with `org_id` still null (proven in the dry-run) — you never stand up an
  embarrassing 2-item demo. For website menus the AI's own LOW_QUALITY exit is the gate; the visual gate's rendered-item floor is the
  independent second check.
- 2026-07-01 — Layer-2 palette reuses the SHIPPED contrast math (ported from packages/ui/src/theme/palette.ts): text is nudged until
  `contrastRatio(text, bg) ≥ 4.5` (AA) for EVERY cuisine seed — proven for pizzeria/sushi/burger/cafe/unknown. A pizzeria reads
  warm-tomato-on-cream, NOT the sushi demo's dark-teal/gold (asserted `pizza.bg !== sushi.bg`), so demos don't all look identical.
- 2026-07-01 — preview-only-by-default is an ethics gate, not a convenience: the default run provisions + certifies but mints ZERO
  claim tokens (proven `invited === 0` without the flag). `--send-invite` only MINTS the token; the Art-14 notice delivery to a real
  owner is STILL a separate human step. No unconsented outreach can slip out of a bulk run.
- 2026-07-01 — no phantom skills: resisted adding an `/internal/acquisition/enrich` or a theme endpoint. The non-AI menu + palette
  persistence is the documented operator DB seam (above), so every `execution_skill` in the card is a real shipped op or a real DB write.
  The dry-run's full pipeline runs the WEBSITE path (extract → mock), which needs no DB; the DB seams are documented, not mocked as prod.

## Історія прогонів (run history)
| дата | тригер/скоуп | результат | flaky? | нотатки |
|---|---|---|---|---|
| 2026-07-01 | tools/demo-builder/dry-run.mjs (anti-cheat, mock+fake-storefront) | GREEN 43/43 | no | CERTIFIED; PART A pure L1/L2/L3 (14) + PART B pipeline A–E (29) |
| 2026-07-01 | FIRST LIVE RUN — Eljo's Pizza (pizzeria, Durrës) → /s/eljos-pizza, staging, preview-only | CERTIFIED-PREVIEW | no | 14 items/5 cats, theme #c1352b/#fbf6ee, gate green mobile+desktop, 0 invites. source_id 76c7f09a…, location_id 79acd75e…. Menu hand-authored (RestaurantGuru had no menu — placeholder "upload menu" page). Both DB seams run from inside the staging container (pg via npm-i in /tmp; DATABASE_URL_MIGRATIONS). |
| 2026-07-01 | REUSE — ArtePasta (italian, Durrës) → /s/artepasta, staging, preview-only | CERTIFIED-PREVIEW | no | 16 items/6 cats, theme #3f7d4f/#fbf6ee, gate green (16 items ×2, 0 console err), 0 invites, ~11.5s total. CERTIFIED on FIRST pass-2 (gate fix held). Menu hand-authored (Wolt has it but API-locked/CSR; RG none). Metrics report: loops/reports/demo-builder-run-artepasta-2026-07-01.md. |
| 2026-07-01 | REAL-IDENTITY REBUILD — ArtePasta re-seeded with the Wolt-extracted menu (50 items/11 cats), red brand #e11b22, + brand-ingest DISPLAY attributes | SHIPPED (b3f75ed5→d738b316, staging v230) | no | Closed the ingredient-badge + hero todos. Live preview now: 38 real Wolt photos, 31 ingredient-badge lists, 28 Albanian (sq) descriptions, all pre-claim, still never-orderable + noindex + bom-stripped. Proof e2e/tests/storefront-brand-ingest.spec.ts green mobile+desktop. DB already re-seeded from a prior seam run; this session shipped only the render + plumbing + proof. |

## Live-run recipe (proven 2026-07-01, Eljo's)
Authored-menu prospect ⇒ two passes + two container-side DB seams:
1. Pass 1 (sandbox): `PROVISION_OPS_SECRET` fetched from the container via a marker (never printed), `node scripts/demo-builder.mjs prospects.json` → creates the acquisition_source, records `menu_draft`/`palette`, returns `NEEDS_ENRICH_SEAM`.
2. Enrich seam (container): `UPDATE acquisition_sources SET menu_draft=$1::jsonb, state='ENRICHED' WHERE id=$src` — acquisition_sources RLS policy is permissive `USING(true)`, so the migrations role writes it.
3. Pass 2: mint→spine (writes org owner_id NULL + location closed/published_at NULL + products) → verify → visual gate → CERTIFIED-PREVIEW.
4. Theme seam (container): upsert `location_themes (location_id, primary_color, bg_color, text_color)` from the run's `theme_directive`; storefront derives the full palette client-side (reload /s/:slug to see it).
- Ops secret + DB creds LIVE ONLY in the staging container — the loop script runs from the sandbox against the public `/internal` surface (secret-header gated); the SQL seams run in-box via `flyctl ssh`.

- 2026-07-01 — VISUAL-GATE FALSE-NEGATIVE FIX (red→green on a GOOD demo): `e2e/tests/demo-builder-visual.spec.ts` counted `menu-item` immediately after `category-nav` became visible, but the SPA paints the shell+nav FIRST and hydrates the item cards a beat later → itemCount 0 on a perfectly good preview (Eljo's failed the first pass 2 for exactly this). Fix: `await page.getByTestId('menu-item').first().waitFor({state:'visible', timeout:15000}).catch(()=>{})` before counting. NOT a weakening — a genuinely empty preview still fails (wait times out, count stays 0, the ≥MIN_RENDERED_ITEMS assertion rejects it). The anti-cheat dry-run (probe backend) was unaffected (it fetches, doesn't hydrate).
