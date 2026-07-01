#!/usr/bin/env node
// demo-builder loop — turn a prospect restaurant into a POLISHED, claimable demo storefront at /s/:slug
// that mirrors the /s/demo (Dubin & Sushi) quality bar, then PROVE the preview actually renders.
//
// It WRAPS the shipped raw provisioner (scripts/acquisition-bulk-provision.mjs + the /internal acquisition
// pipeline) and ADDS the three "make it actually look good" layers the raw provisioner lacks:
//   Layer 1 — MENU QUALITY: normalize a menu (AI-extract from website · operator JSON · hand-authored) into
//             the menu_draft shape and QUALITY-GATE it (≥N categories, ≥M items, real descriptions, sane
//             integer prices). A low-quality menu is rejected BEFORE provisioning (mirrors the state
//             machine's LOW_QUALITY exit) — never stand up a thin, embarrassing demo.
//   Layer 2 — BRANDING: derive a coherent, cuisine-appropriate, AA-contrast palette triple
//             (primary/bg/text) for location_themes (a pizzeria ≠ the sushi demo's dark-teal/gold).
//   Layer 3 — VISUAL ACCEPTANCE GATE: after provisioning, load /s/:slug (mobile + desktop) and assert the
//             preview RENDERS — menu cards visible, honest preview banner + claim CTA present, ZERO console
//             errors, and NEVER-ORDERABLE (no add/cart affordance, noindex). Artifact-backed. NO fake-green:
//             the gate asserts rendered DOM content, not an HTTP 200. A storefront the API marks "verified"
//             but that renders EMPTY / ERRORED / ORDERABLE is classified needs-review, never certified.
//
// Safety (encoded, not assumed):
//   * PREVIEW-ONLY by default — provisions the shadow + runs the visual gate but does NOT mint/send any
//     claim invite unless --send-invite is passed (outreach is an explicit, off-by-default operator act).
//   * NEVER-ORDERABLE — the pipeline writes status='closed' + published_at NULL (B3); the visual gate
//     independently ASSERTS the rendered page has no order affordance (defense-in-depth, verified not assumed).
//   * Allergens are write-stripped pre-claim by the shipped provisioner (C2) — untouched here.
//   * fail-closed — a missing PROVISION_OPS_SECRET means the /internal surface 404s; the loop FAILS LOUDLY
//     (exit 2 on missing config; a per-item OPS_AUTH_404 → needs-review, never a silent no-op).
//   * idempotent/resumable per prospect (place_id/slug) — reads CURRENT state first, jumps to the right stage.
//
// Card: loops/demo-builder.yaml · Report: loops/reports/demo-builder-0.1.md · Memory: loops/memory/demo-builder.md
//
// Usage:
//   PROVISION_BASE_URL=https://dowiz-staging.fly.dev PROVISION_OPS_SECRET=*** \
//     node scripts/demo-builder.mjs ./prospects.json            # preview-only (default, no outreach)
//   … node scripts/demo-builder.mjs ./prospects.json --send-invite   # ALSO mint the claim token (explicit)
//
// Prospect row: { place_id, slug, name, website_url? , menu? , cuisine?, phone?, invited_contact? }
//   - website_url present & no menu → Layer 1 uses the shipped AI extract (H4 verdict = quality gate).
//   - menu present (array of {name, category, price, description?}) → Layer 1 quality-gates it client-side;
//     persistence of a non-AI draft is the documented operator DB seam (see loops/memory/demo-builder.md).
//   - cuisine (e.g. "pizzeria", "sushi", "burger", "cafe") seeds Layer 2's palette.

import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

// ── quality thresholds (env-overridable; sane defaults mirror the /s/demo bar) ──────────────────────────
const MIN_CATEGORIES = Number(process.env.DEMO_MIN_CATEGORIES ?? 2);
const MIN_ITEMS = Number(process.env.DEMO_MIN_ITEMS ?? 6);
const MIN_DESC_RATIO = Number(process.env.DEMO_MIN_DESC_RATIO ?? 0.5); // ≥50% of items carry a real description
const MIN_RENDERED_ITEMS = Number(process.env.DEMO_MIN_RENDERED_ITEMS ?? 3); // the visual gate's item floor
const PRICE_MIN = Number(process.env.DEMO_PRICE_MIN ?? 1); // integer minor units, > 0
const PRICE_MAX = Number(process.env.DEMO_PRICE_MAX ?? 5_000_000);

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const isUuid = (v) => typeof v === 'string' && UUID_RE.test(v);
const isNonEmptyStr = (v) => typeof v === 'string' && v.trim().length > 0;

const NEED_EXTRACT = new Set(['SOURCED', 'PLACE_INGESTED', 'MENU_EXTRACTED']);
const EXIT_STATES = new Set(['MENU_NOT_FOUND', 'LOW_QUALITY', 'MANUAL_REVIEW', 'DISQUALIFIED', 'ABANDONED']);

// ════════════════════════════════════════════════════════════════════════════════════════════════════════
// LAYER 1 — menu normalize + quality gate (PURE; exported for the anti-cheat dry-run / unit proof).
// ════════════════════════════════════════════════════════════════════════════════════════════════════════

/** Normalize a flat operator/hand-authored menu (array of {name, category, price, description?, bom?}) into
 *  the menu_draft shape: {categories:[{name, sort_order, products:[{name, price(int minor), description?,
 *  sort_order, attributes?}]}]}. Prices already in integer minor units (the operator's responsibility). */
export function normalizeMenu(items) {
  if (!Array.isArray(items)) throw new Error('menu must be an array of items');
  const byCat = new Map();
  for (const it of items) {
    const cat = isNonEmptyStr(it?.category) ? it.category.trim() : 'Menu';
    if (!byCat.has(cat)) byCat.set(cat, []);
    byCat.get(cat).push(it);
  }
  const categories = [...byCat.entries()].map(([name, prods], ci) => ({
    name,
    sort_order: ci,
    products: prods.map((p, pi) => ({
      name: String(p?.name ?? '').trim(),
      price: p?.price,
      description: isNonEmptyStr(p?.description) ? p.description.trim() : undefined,
      sort_order: pi,
      // Rich attributes from the brand-ingest layer. bom is the allergen surface (stripped on the shadow
      // preview by read_preview_menu's `attributes - 'bom'`); ingredients/image_url/description_sq are
      // display-only keys that SURVIVE that strip, so real photos, ingredient badges and the bilingual
      // (EN + original) description render pre-claim without touching the allergen gate.
      ...(() => {
        const a = {};
        if (p?.bom) a.bom = p.bom;
        if (Array.isArray(p?.ingredients) && p.ingredients.length) a.ingredients = p.ingredients;
        if (isNonEmptyStr(p?.image_url)) a.image_url = p.image_url.trim();
        if (isNonEmptyStr(p?.description_sq)) a.description_sq = p.description_sq.trim();
        return Object.keys(a).length ? { attributes: a } : {};
      })(),
    })),
  }));
  return { categories };
}

/** Quality gate: returns { ok, reasons[], stats }. A FAIL means "do NOT provision — needs-review:LOW_QUALITY".
 *  Enforces: ≥MIN_CATEGORIES categories, ≥MIN_ITEMS products, ≥MIN_DESC_RATIO with real descriptions, EVERY
 *  price an integer in [PRICE_MIN, PRICE_MAX], no blank product names. This is the "don't ship a thin demo" gate. */
export function gateMenu(draft) {
  const reasons = [];
  const cats = Array.isArray(draft?.categories) ? draft.categories : [];
  const products = cats.flatMap((c) => (Array.isArray(c?.products) ? c.products : []));
  const nCats = cats.filter((c) => (c?.products?.length ?? 0) > 0).length;
  const nItems = products.length;
  const withDesc = products.filter((p) => isNonEmptyStr(p?.description)).length;
  const descRatio = nItems > 0 ? withDesc / nItems : 0;

  if (nCats < MIN_CATEGORIES) reasons.push(`too few categories (${nCats} < ${MIN_CATEGORIES})`);
  if (nItems < MIN_ITEMS) reasons.push(`too few items (${nItems} < ${MIN_ITEMS})`);
  if (descRatio < MIN_DESC_RATIO) reasons.push(`too few descriptions (${(descRatio * 100) | 0}% < ${MIN_DESC_RATIO * 100}%)`);
  for (const p of products) {
    if (!isNonEmptyStr(p?.name)) { reasons.push('a product has a blank name'); break; }
  }
  for (const p of products) {
    if (!Number.isInteger(p?.price) || p.price < PRICE_MIN || p.price > PRICE_MAX) {
      reasons.push(`insane/non-integer price for "${p?.name ?? '?'}" (${p?.price})`);
      break;
    }
  }
  return { ok: reasons.length === 0, reasons, stats: { nCats, nItems, descRatio: +descRatio.toFixed(2) } };
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════
// LAYER 2 — coherent, cuisine-appropriate, AA-contrast palette triple (PURE; ported contrast math from
// packages/ui/src/theme/palette.ts so the server-side seed matches the storefront's derivePalette).
// ════════════════════════════════════════════════════════════════════════════════════════════════════════

function parseColor(input) {
  if (!input) return null;
  const hex = String(input).trim().replace(/^#/, '');
  if (/^[0-9a-fA-F]{3}$/.test(hex)) return { r: parseInt(hex[0] + hex[0], 16), g: parseInt(hex[1] + hex[1], 16), b: parseInt(hex[2] + hex[2], 16) };
  if (/^[0-9a-fA-F]{6}$/.test(hex)) return { r: parseInt(hex.slice(0, 2), 16), g: parseInt(hex.slice(2, 4), 16), b: parseInt(hex.slice(4, 6), 16) };
  return null;
}
const toHex = ({ r, g, b }) => '#' + [r, g, b].map((n) => Math.min(255, Math.max(0, Math.round(n))).toString(16).padStart(2, '0')).join('');
function luminance({ r, g, b }) {
  const ch = (c) => { const s = c / 255; return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4); };
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
}
export function contrastRatio(a, b) {
  const la = luminance(a), lb = luminance(b), hi = Math.max(la, lb), lo = Math.min(la, lb);
  return (hi + 0.05) / (lo + 0.05);
}
const WHITE = { r: 255, g: 255, b: 255 }, BLACK = { r: 17, g: 17, b: 17 };
const mix = (a, b, t) => ({ r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t });
const readableOn = (bg) => (contrastRatio(WHITE, bg) >= contrastRatio(BLACK, bg) ? WHITE : BLACK);
function ensureContrast(text, bg, min) {
  if (contrastRatio(text, bg) >= min) return text;
  const pole = readableOn(bg);
  let t = 0.15, cand = mix(text, pole, t);
  while (contrastRatio(cand, bg) < min && t < 1) { t += 0.15; cand = mix(text, pole, t); }
  return cand;
}

// Cuisine → tasteful (primary, bg) seeds. A pizzeria reads warm-tomato on cream; NOT the sushi demo's dark
// teal/gold. Unknown cuisine falls back to the dowiz food-orange on warm paper.
const CUISINE_SEEDS = {
  pizza: { primary: '#c1352b', bg: '#fbf6ee' }, pizzeria: { primary: '#c1352b', bg: '#fbf6ee' },
  italian: { primary: '#3f7d4f', bg: '#fbf6ee' },
  sushi: { primary: '#c8a24a', bg: '#14201f' }, japanese: { primary: '#c8a24a', bg: '#14201f' },
  burger: { primary: '#e08a1e', bg: '#201d1a' }, grill: { primary: '#d2691e', bg: '#1e1c1a' }, american: { primary: '#e08a1e', bg: '#201d1a' },
  cafe: { primary: '#8a5a2b', bg: '#f6efe4' }, coffee: { primary: '#8a5a2b', bg: '#f6efe4' }, bakery: { primary: '#b5762e', bg: '#f8f2e7' },
  kebab: { primary: '#b5471f', bg: '#faf4e8' }, turkish: { primary: '#b5471f', bg: '#faf4e8' }, mediterranean: { primary: '#2f7d76', bg: '#f7f3ea' },
  seafood: { primary: '#1f6f8b', bg: '#f2f6f7' }, vegan: { primary: '#4a8b3b', bg: '#f4f8f0' },
  dessert: { primary: '#c65b7c', bg: '#fbf1f4' }, indian: { primary: '#d17a1e', bg: '#fbf4e8' },
};

/** Derive a coherent {primary_color, bg_color, text_color} for location_themes from cuisine (+ optional
 *  brand seeds). GUARANTEE: contrastRatio(text, bg) ≥ 4.5 (AA). The storefront's derivePalette expands
 *  these three into the full token set. */
export function derivePaletteTriple({ cuisine, primary, bg } = {}) {
  const seed = CUISINE_SEEDS[String(cuisine ?? '').toLowerCase().trim()] ?? { primary: '#ea4f16', bg: '#faf8f4' };
  const primaryRgb = parseColor(primary) ?? parseColor(seed.primary);
  const bgRgb = parseColor(bg) ?? parseColor(seed.bg);
  const text = ensureContrast(readableOn(bgRgb), bgRgb, 4.5);
  return { primary_color: toHex(primaryRgb), bg_color: toHex(bgRgb), text_color: toHex(text) };
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════
// LAYER 3 — visual acceptance gate. assertPreviewDom is PURE + shared by both the live Playwright path and
// the hermetic dry-run probe, so the gate LOGIC (assert rendered DOM, not HTTP 200) is what gets certified.
// ════════════════════════════════════════════════════════════════════════════════════════════════════════

/** Assert a /s/:slug preview render is demo-quality. Input: { html (serialized DOM), consoleErrors (count),
 *  robots (x-robots-tag or meta) }. Returns { pass, reasons[] }. Cheat-resistant: a 200 that renders empty /
 *  errored / ORDERABLE fails here even though the HTTP status is fine. */
export function assertPreviewDom({ html = '', consoleErrors = 0, robots = '' } = {}) {
  const reasons = [];
  const lc = html.toLowerCase();
  // rendered menu items — React shell emits data-testid="menu-item"; the bot/static preview emits class="item".
  const itemCount = (html.match(/data-testid="menu-item"/g) || []).length
    || (html.match(/class="item"/g) || []).length;
  if (itemCount < MIN_RENDERED_ITEMS) reasons.push(`only ${itemCount} menu items rendered (< ${MIN_RENDERED_ITEMS})`);
  // honest preview banner present (testid OR the verbatim static-preview label).
  if (!/venue-preview-banner/.test(html) && !/not a live store/.test(lc)) reasons.push('honest preview banner missing');
  // claim CTA present.
  if (!/preview-claim-cta/.test(html) && !/claim this preview|is this your restaurant/.test(lc)) reasons.push('claim CTA missing');
  // zero console errors (the "renders clean" bar).
  if (consoleErrors > 0) reasons.push(`${consoleErrors} console error(s) on the page`);
  // NEVER-ORDERABLE (B3) — verified in the DOM, not assumed: no add/cart affordance, no order buttons.
  if (/menu-item-add/.test(html)) reasons.push('order affordance present (menu-item-add) — B3 never-orderable violated');
  if (/data-testid="cart-open"/.test(html)) reasons.push('cart FAB present — B3 never-orderable violated');
  if (/add to cart|checkout/.test(lc)) reasons.push('add-to-cart/checkout button present — B3 violated');
  // noindex (a demo preview must never be indexable).
  if (!/noindex/.test(String(robots).toLowerCase()) && !/noindex/.test(lc)) reasons.push('missing noindex');
  return { pass: reasons.length === 0, reasons, itemCount };
}

// Run the visual gate. Two backends, same assertPreviewDom logic:
//   * DEMO_BUILDER_VISUAL_MODE=probe  → hermetic HTTP probe (dry-run): fetch the storefront with a mobile UA
//     then a desktop UA, read the served HTML + x-fake-console-errors header + x-robots-tag.
//   * default (live)                  → spawn the real Playwright spec (mobile + desktop projects, real
//     console-error capture) and gate on exit 0 + the artifact it writes.
async function runVisualGate({ storefrontBase, slug }) {
  const mode = process.env.DEMO_BUILDER_VISUAL_MODE;
  if (mode === 'probe') {
    const viewports = [
      { name: 'mobile', ua: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1' },
      { name: 'desktop', ua: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) AppleWebKit/537.36 Chrome/120 Safari/537.36' },
    ];
    const results = {};
    let pass = true;
    for (const v of viewports) {
      let html = '', robots = '', consoleErrors = 0;
      try {
        const res = await fetch(`${storefrontBase}/s/${slug}`, { headers: { 'user-agent': v.ua } });
        html = await res.text();
        robots = res.headers.get('x-robots-tag') || '';
        consoleErrors = Number(res.headers.get('x-fake-console-errors') || 0); // stand-in for real console errors
      } catch (e) {
        results[v.name] = { pass: false, reasons: [`fetch failed: ${String(e?.message || e)}`] };
        pass = false;
        continue;
      }
      const r = assertPreviewDom({ html, consoleErrors, robots });
      results[v.name] = r;
      if (!r.pass) pass = false;
    }
    return { pass, viewports: results, mode: 'probe' };
  }

  // live: spawn the real Playwright visual spec against the deployed storefront (mobile + desktop).
  const artifact = resolve(`e2e/artifacts/demo-builder-visual-${slug}.json`);
  const run = spawnSync(
    'pnpm',
    ['exec', 'playwright', 'test', 'e2e/tests/demo-builder-visual.spec.ts', '--project=mobile', '--project=desktop', '--reporter=list'],
    { env: { ...process.env, VITE_BASE_URL: storefrontBase, PROVISION_VERIFY_SLUG: slug }, encoding: 'utf8' },
  );
  let viewports = null;
  try { viewports = JSON.parse(readFileSync(artifact, 'utf8')); } catch { /* spec may not have written it */ }
  return { pass: run.status === 0, viewports, mode: 'playwright', exitCode: run.status };
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════
// pipeline plumbing (reused from the certified acquisition-bulk-provision loop — field-gated, not status-gated).
// ════════════════════════════════════════════════════════════════════════════════════════════════════════

function loadConfig() {
  const argv = process.argv.slice(2);
  const flags = new Set(argv.filter((a) => a.startsWith('--')));
  const file = argv.find((a) => !a.startsWith('--'));
  const baseUrl = process.env.PROVISION_BASE_URL;
  const secret = process.env.PROVISION_OPS_SECRET;
  const missing = [];
  if (!baseUrl) missing.push('PROVISION_BASE_URL');
  if (!secret) missing.push('PROVISION_OPS_SECRET');
  if (!file) missing.push('<prospects.json> (argv)');
  if (missing.length) {
    console.error(`FATAL: missing required config: ${missing.join(', ')}`); // never echoes the secret value
    process.exit(2);
  }
  return {
    baseUrl: baseUrl.replace(/\/+$/, ''),
    storefrontBase: (process.env.STOREFRONT_BASE_URL || baseUrl).replace(/\/+$/, ''),
    secret,
    file: resolve(file),
    sendInvite: flags.has('--send-invite'), // outreach is OFF by default (preview-only)
  };
}

function parseInput(file) {
  const parsed = JSON.parse(readFileSync(file, 'utf8'));
  if (!Array.isArray(parsed)) throw new Error('input must be a JSON array of prospect rows');
  return parsed.map((r, i) => ({ ...r, _row: i + 1 }));
}

function makePost(baseUrl, secret) {
  return async function post(path, body) {
    let res;
    try {
      res = await fetch(`${baseUrl}${path}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-provision-ops-secret': secret },
        body: JSON.stringify(body),
      });
    } catch (e) {
      return { status: 0, json: { error: 'NETWORK', message: String(e?.message || e) } };
    }
    let json = {};
    try { json = await res.json(); } catch { json = {}; }
    return { status: res.status, json };
  };
}

async function processItem(item, ctx) {
  const { post, cfg } = ctx;
  const out = { row: item._row, place_id: item.place_id, name: item.name, slug: item.slug, warnings: [] };
  const fail = (reason, state) => ({ ...out, outcome: 'needs-review', reason, state });
  const skip = (reason, state) => ({ ...out, outcome: 'skipped-already-done', reason, state });

  for (const k of ['place_id', 'name', 'slug']) {
    if (!isNonEmptyStr(item[k])) return fail(`MALFORMED_ROW:missing ${k}`);
  }
  if (!isNonEmptyStr(item.website_url) && !Array.isArray(item.menu)) {
    return fail('MALFORMED_ROW:need website_url (AI extract) or menu[] (operator/authored)');
  }

  // LAYER 1 (pre-flight for operator/authored menus): quality-gate BEFORE any provisioning.
  if (Array.isArray(item.menu)) {
    let draft;
    try { draft = normalizeMenu(item.menu); } catch (e) { return fail(`MENU_NORMALIZE:${String(e?.message || e)}`); }
    const g = gateMenu(draft);
    out.menu_stats = g.stats;
    if (!g.ok) return fail(`LOW_QUALITY:${g.reasons.join('; ')}`); // never provision a thin demo
    out.menu_draft = draft; // persisted via the operator DB seam (see memory) — not an API endpoint
  }

  // LAYER 2: derive the palette triple (persisted to location_themes via the operator DB seam post-spine).
  out.palette = derivePaletteTriple({ cuisine: item.cuisine, primary: item.primary, bg: item.bg });

  // STAGE 0 — idempotent create / resume read.
  const c = await post('/internal/acquisition', { place_id: item.place_id });
  if (c.status === 404) return fail('OPS_AUTH_404 (secret rejected / surface disabled)');
  if (!isUuid(c.json?.id) || !isNonEmptyStr(c.json?.state)) return fail(`CREATE_FAILED:${c.json?.error || c.status}`);
  const id = c.json.id;
  out.source_id = id;
  let state = c.json.state;

  if (state === 'CLAIMED') return skip('already-claimed', state);
  if (state === 'CLAIM_OFFERED') return skip('already-invited (active invite exists)', state);
  if (EXIT_STATES.has(state)) return fail(`EXIT_STATE:${state}`, state);

  // STAGE 1 — extract (website mode). The AI H4 verdict IS the menu-quality gate for website menus.
  if (NEED_EXTRACT.has(state)) {
    if (!isNonEmptyStr(item.website_url)) {
      // operator/authored menu present but the source is not yet ENRICHED → needs the DB enrich seam first.
      return fail('NEEDS_ENRICH_SEAM: operator menu not yet persisted to ENRICHED (see memory: DB enrich seam)', state);
    }
    const ex = await post('/internal/acquisition/extract', { acquisition_source_id: id, website_url: item.website_url });
    if (ex.status === 503) return fail('EXTRACTION_UNAVAILABLE', state);
    const exState = ex.json?.state;
    if (exState !== 'ENRICHED') return fail(`EXTRACT:${exState || ex.json?.error || 'FAILED'}`, exState); // LOW_QUALITY/MENU_NOT_FOUND → needs-review
    state = 'ENRICHED';
  }

  const re = await post('/internal/acquisition', { place_id: item.place_id });
  state = re.json?.state || state;
  if (!['ENRICHED', 'PROVISIONED', 'VERIFIED'].includes(state)) return fail(`UNEXPECTED_STATE:${state}`, state);

  // STAGE 2 — mint + spine (ENRICHED→PROVISIONED). Field-gated (real FKs, not a 201).
  if (state === 'ENRICHED') {
    const mint = await post('/internal/acquisition/provision/mint', { acquisition_source_id: id });
    if (mint.status !== 201 || !isNonEmptyStr(mint.json?.token)) return fail(`MINT:${mint.json?.error || mint.status}`, state);
    const spine = await post('/internal/acquisition/provision/spine', {
      acquisition_source_id: id, token: mint.json.token, name: item.name, slug: item.slug, phone: item.phone,
    });
    if (spine.status !== 201 || !isUuid(spine.json?.org_id) || !isUuid(spine.json?.location_id)) {
      return fail(`SPINE:${spine.json?.error || spine.status}`, state);
    }
    out.org_id = spine.json.org_id;
    out.location_id = spine.json.location_id;
    state = 'PROVISIONED';
    // Layer 2 persistence directive: the operator writes the palette to location_themes for this location
    // (see memory: DB theme seam). Recorded so a live run knows exactly which row to theme.
    out.theme_directive = { location_id: spine.json.location_id, ...out.palette };
  }

  // STAGE 3 — API verify (PROVISIONED→VERIFIED). Field-gated on verified===true (empty shadow → 409).
  if (state === 'PROVISIONED') {
    const v = await post('/internal/acquisition/claim/verify', { acquisition_source_id: id });
    if (v.json?.verified !== true) return fail(`NOT_VERIFIABLE:${v.json?.error || v.status}`, state);
    state = 'VERIFIED';
  }

  // LAYER 3 — VISUAL ACCEPTANCE GATE. The differentiator: even though the API said "verified", the rendered
  // /s/:slug must ACTUALLY look like a demo (items visible, banner+CTA, clean console, never-orderable).
  const vg = await runVisualGate({ storefrontBase: cfg.storefrontBase, slug: item.slug });
  out.visual_gate = vg;
  if (!vg.pass) {
    const why = vg.viewports
      ? Object.entries(vg.viewports).map(([vp, r]) => `${vp}:${(r.reasons || []).join(',') || (r.pass ? 'ok' : 'fail')}`).join(' | ')
      : `exit ${vg.exitCode}`;
    return fail(`VISUAL_GATE_FAILED: ${why}`, state);
  }

  // At this point the demo storefront is CERTIFIED-PREVIEW: provisioned, themed, and visually accepted.
  // Outreach is a SEPARATE, explicit act — preview-only by default.
  if (!cfg.sendInvite) {
    const previewUrl = `${cfg.storefrontBase}/s/${item.slug}`;
    return { ...out, outcome: 'certified-preview', state: 'VERIFIED', preview_url: previewUrl };
  }

  // STAGE 4 (opt-in) — mint the claim invite. Even here we only MINT the token; delivering it is a human step.
  const cm = await post('/internal/acquisition/claim/mint', {
    acquisition_source_id: id, invited_contact: item.invited_contact, base_url: cfg.storefrontBase,
  });
  if (cm.status !== 201 || !isNonEmptyStr(cm.json?.token)) return fail(`CLAIM_MINT:${cm.json?.error || cm.status}`, state);
  if (!item.invited_contact) out.warnings.push('no invited_contact → claim link is decline-only (CONTACT_REQUIRED on web accept)');
  const claimUrl = `${cfg.storefrontBase}/claim#token=${cm.json.token}`;
  return { ...out, outcome: 'invited', state: 'CLAIM_OFFERED', preview_url: `${cfg.storefrontBase}/s/${item.slug}`, claim_url: claimUrl, decline_url: claimUrl };
}

async function main() {
  const cfg = loadConfig();
  const items = parseInput(cfg.file);
  const post = makePost(cfg.baseUrl, cfg.secret);
  const ctx = { post, cfg };

  const results = [];
  for (const item of items) {
    try { results.push(await processItem(item, ctx)); }
    catch (e) { results.push({ row: item._row, place_id: item.place_id, name: item.name, outcome: 'needs-review', reason: `EXCEPTION:${String(e?.message || e)}` }); }
  }

  const count = (o) => results.filter((r) => r.outcome === o).length;
  const summary = {
    attempted: results.length,
    certified_preview: count('certified-preview'),
    invited: count('invited'),
    needs_review: count('needs-review'),
    skipped_already_done: count('skipped-already-done'),
    send_invite: cfg.sendInvite,
  };

  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const runDir = resolve('loops/runs');
  mkdirSync(runDir, { recursive: true });
  const runFile = `${runDir}/demo-builder-${stamp}.json`;
  writeFileSync(runFile, JSON.stringify({ at: new Date().toISOString(), base_url: cfg.baseUrl, storefront_base: cfg.storefrontBase, summary, results }, null, 2));

  console.log('\n=== demo-builder — run report ===');
  console.log(`pipeline: ${cfg.baseUrl}   storefront: ${cfg.storefrontBase}   (secret: [redacted])`);
  console.log(`outreach: ${cfg.sendInvite ? 'ENABLED (--send-invite)' : 'preview-only (default)'}`);
  for (const r of results) {
    const tag = { 'certified-preview': 'CERTIFIED', invited: 'INVITED', 'needs-review': 'NEEDS-REVIEW', 'skipped-already-done': 'SKIPPED' }[r.outcome] || r.outcome;
    let line = `  [${tag}] row#${r.row} ${r.name || r.place_id} (${r.slug || r.place_id})`;
    if (r.state) line += ` state=${r.state}`;
    if (r.menu_stats) line += ` menu=${r.menu_stats.nItems}items/${r.menu_stats.nCats}cats`;
    if (r.palette) line += ` theme=${r.palette.primary_color}`;
    if (r.reason) line += ` — ${r.reason}`;
    console.log(line);
    if (r.preview_url) console.log(`           preview: ${r.preview_url}`);
    if (r.claim_url) console.log(`           claim/decline: ${r.claim_url}`);
    for (const w of r.warnings || []) console.log(`           ⚠ ${w}`);
  }
  console.log('\nsummary:', JSON.stringify(summary));
  console.log(`run artifact: ${runFile}`);

  // exit 3 iff any needs-review (a CI/cron caller detects partial failure); else 0.
  process.exit(summary.needs_review > 0 ? 3 : 0);
}

// Only run when invoked directly — the pure Layer-1/2/3 functions are imported by the anti-cheat dry-run.
if (process.argv[1] && resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)) {
  main().catch((e) => { console.error('FATAL:', e?.message || e); process.exit(1); });
}
