// Anti-cheat dry-run / certification harness for the demo-builder loop (M9).
//
// Two halves:
//   PART A — PURE unit proof of the three added quality layers (imported directly from the orchestrator):
//     Layer 1 gateMenu (blocks thin menus), Layer 2 derivePaletteTriple (AA-contrast, cuisine-aware),
//     Layer 3 assertPreviewDom (rejects empty/errored/orderable/noindexless renders — no-fake-green).
//   PART B — END-TO-END pipeline proof: run the REAL orchestrator (scripts/demo-builder.mjs) as a child
//     process against a state-machine-faithful mock (pipeline) + a configurable fake storefront (visual
//     gate), on crafted fixtures, and assert the loop:
//       1. CERTIFIES only a prospect whose menu is quality AND whose /s/:slug ACTUALLY renders demo-quality.
//       2. records NEEDS-REVIEW for every broken fixture and NEVER aborts the run.
//       3. NEVER provisions a low-quality / menu-not-found source (no spine).
//       4. FAILS the visual gate on an EMPTY / ERRORED / ORDERABLE / NOINDEXLESS 200-render — the loop's
//          differentiator over the raw provisioner: a source the API marks verified:true but that renders
//          broken is NEEDS-REVIEW, not certified. A "200 == pass" loop would falsely certify these.
//       5. is PREVIEW-ONLY by default: mints ZERO claim tokens unless --send-invite.
//       6. is IDEMPOTENT: a re-run skips already-done work.
//       7. FAILS CLOSED: a wrong ops secret (404) → needs-review, secret never printed.
//
// Run: node tools/demo-builder/dry-run.mjs   (exit 0 = CERTIFIED gates green)

import { spawn } from 'node:child_process';
import { writeFileSync, readFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startMock } from './mock-internal.mjs';
import { startFakeStorefront } from './fake-storefront.mjs';
import { gateMenu, normalizeMenu, derivePaletteTriple, contrastRatio, assertPreviewDom, fontPairingForCuisine } from '../../scripts/demo-builder.mjs';
// Leaf module (no React) — safe to import in node. The storefront's font allowlist is the source of truth;
// this pins the demo-builder's provision-time font seeds to it so a drift can't ship an unloadable font.
import { FONT_ALLOWLIST } from '../../packages/ui/dist/theme/fonts.js';

const ORCH = new URL('../../scripts/demo-builder.mjs', import.meta.url).pathname;
const SECRET = 'dry-run-secret-xyz';
const tmp = mkdtempSync(join(tmpdir(), 'demo-builder-dryrun-'));

let failures = 0;
const assert = (cond, msg) => { console.log(`  ${cond ? 'PASS' : 'FAIL'} — ${msg}`); if (!cond) failures++; };
const parseColor = (h) => ({ r: parseInt(h.slice(1, 3), 16), g: parseInt(h.slice(3, 5), 16), b: parseInt(h.slice(5, 7), 16) });

function runOrchestrator({ pipelineUrl, storefrontUrl, secret, fixturePath, sendInvite = false }) {
  return new Promise((resolveP) => {
    const args = [ORCH, fixturePath];
    if (sendInvite) args.push('--send-invite');
    const child = spawn(process.execPath, args, {
      env: {
        ...process.env,
        PROVISION_BASE_URL: pipelineUrl,
        STOREFRONT_BASE_URL: storefrontUrl,
        PROVISION_OPS_SECRET: secret,
        DEMO_BUILDER_VISUAL_MODE: 'probe', // hermetic HTTP probe into the fake storefront (no Playwright)
      },
    });
    let stdout = '';
    child.stdout.on('data', (d) => (stdout += d));
    child.stderr.on('data', (d) => (stdout += d));
    child.on('close', (code) => {
      const m = /run artifact: (.+)/.exec(stdout);
      const report = m ? JSON.parse(readFileSync(m[1].trim(), 'utf8')) : null;
      resolveP({ code, stdout, report, leaked: stdout.includes(secret) });
    });
  });
}

const fixture = (name, rows) => { const p = join(tmp, `${name}.json`); writeFileSync(p, JSON.stringify(rows, null, 2)); return p; };

async function main() {
  console.log('=== demo-builder loop · anti-cheat dry-run ===\n');

  // ─────────────────────────────────────────────────────────────────────────────────────────────────────
  console.log('PART A — pure proof of the three added quality layers:\n');

  // Layer 1 — gateMenu blocks a thin demo, passes a rich one.
  const thin = normalizeMenu([{ name: 'X', category: 'A', price: 100 }, { name: 'Y', category: 'A', price: 200 }]);
  assert(gateMenu(thin).ok === false, 'L1 gateMenu REJECTS a thin menu (1 cat / 2 items / no descriptions)');
  const rich = normalizeMenu([
    { name: 'Margherita', category: 'Pizza', price: 58000, description: 'Fior di latte, tomato, basil.' },
    { name: 'Diavola', category: 'Pizza', price: 68000, description: 'Spicy salami, chilli.' },
    { name: 'Quattro Formaggi', category: 'Pizza', price: 72000, description: 'Four cheese.' },
    { name: 'Bruschetta', category: 'Antipasti', price: 45000, description: 'Grilled sourdough, tomato.' },
    { name: 'Burrata', category: 'Antipasti', price: 62000, description: 'Creamy burrata, rocket.' },
    { name: 'Tiramisu', category: 'Dolci', price: 40000, description: 'Mascarpone, espresso.' },
    { name: 'Gelato', category: 'Dolci', price: 30000, description: 'Three scoops.' },
  ]);
  assert(gateMenu(rich).ok === true, 'L1 gateMenu PASSES a rich demo menu (3 cats / 7 items / descriptions)');
  const badPrice = normalizeMenu([
    { name: 'A', category: 'C1', price: 5.5, description: 'd' }, { name: 'B', category: 'C1', price: 100, description: 'd' },
    { name: 'C', category: 'C2', price: 100, description: 'd' }, { name: 'D', category: 'C2', price: 100, description: 'd' },
    { name: 'E', category: 'C3', price: 100, description: 'd' }, { name: 'F', category: 'C3', price: 100, description: 'd' },
  ]);
  assert(gateMenu(badPrice).ok === false && gateMenu(badPrice).reasons.some((r) => /price/.test(r)), 'L1 gateMenu REJECTS a non-integer price (money-integrity)');

  // Layer 2 — derivePaletteTriple is coherent, cuisine-aware, AA-contrast.
  const pizza = derivePaletteTriple({ cuisine: 'pizzeria' });
  const sushi = derivePaletteTriple({ cuisine: 'sushi' });
  assert(pizza.bg_color !== sushi.bg_color && pizza.primary_color !== sushi.primary_color, 'L2 a pizzeria palette differs from the sushi demo (not a single hard-coded theme)');
  for (const [cuisine, t] of [['pizzeria', pizza], ['sushi', sushi], ['burger', derivePaletteTriple({ cuisine: 'burger' })], ['cafe', derivePaletteTriple({ cuisine: 'cafe' })], ['unknown', derivePaletteTriple({ cuisine: 'zzz' })]]) {
    const cr = contrastRatio(parseColor(t.text_color), parseColor(t.bg_color));
    assert(cr >= 4.5, `L2 ${cuisine} text/bg contrast is AA (${cr.toFixed(2)} ≥ 4.5)`);
  }

  // Layer 3 — assertPreviewDom: PASS the good render, FAIL each broken one (no-fake-green).
  const goodDom = `<div data-testid="venue-preview-banner"></div>
    <div data-testid="menu-item"></div><div data-testid="menu-item"></div><div data-testid="menu-item"></div>
    <div data-testid="preview-claim-cta"></div>`;
  assert(assertPreviewDom({ html: goodDom, consoleErrors: 0, robots: 'noindex' }).pass === true, 'L3 PASSES a demo-quality render (items + banner + CTA + noindex, no order affordance)');
  assert(assertPreviewDom({ html: '<div data-testid="venue-preview-banner"></div><p>No menu items.</p><div data-testid="preview-claim-cta"></div>', robots: 'noindex' }).pass === false, 'L3 FAILS an EMPTY render (0 items) despite everything else present');
  assert(assertPreviewDom({ html: goodDom, consoleErrors: 1, robots: 'noindex' }).pass === false, 'L3 FAILS on a console error');
  assert(assertPreviewDom({ html: goodDom + '<button data-testid="menu-item-add">+</button>', robots: 'noindex' }).pass === false, 'L3 FAILS an ORDERABLE render (B3 never-orderable enforced in the DOM)');
  assert(assertPreviewDom({ html: goodDom, robots: '' }).pass === false, 'L3 FAILS a render missing noindex');

  // Layer 2b — font seed sync: every cuisine's font pairing must reference a real storefront allowlist id,
  // else the storefront silently falls back (an unloadable font). Guards the demo-builder↔ui duplication.
  const ALLOWED = new Set(Object.keys(FONT_ALLOWLIST));
  for (const cuisine of ['italian', 'pizzeria', 'sushi', 'burger', 'cafe', 'kebab', 'mediterranean', 'indian', 'vegan', 'seafood', 'grill', 'dessert', 'unknown-xyz', '']) {
    const { heading_font, body_font } = fontPairingForCuisine(cuisine);
    assert(ALLOWED.has(heading_font) && ALLOWED.has(body_font), `L2b ${cuisine || 'default'} font pairing (${heading_font}/${body_font}) ∈ storefront allowlist`);
  }

  // ─────────────────────────────────────────────────────────────────────────────────────────────────────
  console.log('\nPART B — end-to-end pipeline proof (real orchestrator × faithful mock × fake storefront):\n');

  // Scenario A — mixed batch: 1 happy (good menu + good render), + 4 broken. Preview-only (default).
  console.log('Scenario A — mixed batch (certify only the good demo; never-provision the bad menus):');
  const mockA = await startMock({ secret: SECRET });
  const sfA = await startFakeStorefront({ variants: { alpha: 'good' } }); // others default to good; broken ones never reach the gate
  const rowsA = [
    { place_id: 'place-goodmenu-alpha', slug: 'alpha', name: 'Alpha Pizzeria', website_url: 'https://a.example', cuisine: 'pizzeria', invited_contact: 'a@x.com' },
    { place_id: 'place-lowquality-beta', slug: 'beta', name: 'Beta', website_url: 'https://b.example', cuisine: 'burger' },
    { place_id: 'place-menunotfound-gamma', slug: 'gamma', name: 'Gamma', website_url: 'https://c.example', cuisine: 'cafe' },
    { place_id: 'place-emptymenu-delta', slug: 'delta', name: 'Delta', website_url: 'https://d.example', cuisine: 'sushi' },
    { place_id: 'place-malformed-eps', slug: '', name: 'Eps', website_url: 'https://e.example' },
  ];
  const a = await runOrchestrator({ pipelineUrl: mockA.baseUrl, storefrontUrl: sfA.baseUrl, secret: SECRET, fixturePath: fixture('A', rowsA) });
  assert(a.report?.summary.attempted === 5, `all 5 processed (no abort) — got ${a.report?.summary.attempted}`);
  assert(a.report?.summary.certified_preview === 1, `exactly 1 certified-preview (the good pizzeria) — got ${a.report?.summary.certified_preview}`);
  assert(a.report?.summary.invited === 0, 'preview-only by default: 0 claim invites minted');
  assert(a.report?.summary.needs_review === 4, `4 needs-review (lowquality/menunotfound/emptymenu/malformed) — got ${a.report?.summary.needs_review}`);
  const alpha = a.report?.results.find((r) => r.slug === 'alpha');
  assert(alpha?.outcome === 'certified-preview' && /\/s\/alpha$/.test(alpha?.preview_url || ''), 'alpha certified with a preview URL');
  assert(alpha?.palette?.primary_color && alpha?.theme_directive?.location_id, 'alpha carries a derived palette + a location_themes directive');
  const beta = mockA.sources.get('place-lowquality-beta');
  assert(beta?.org_id === null, 'NEVER provisioned the LOW_QUALITY source (org_id still null)');
  const gamma = mockA.sources.get('place-menunotfound-gamma');
  assert(gamma?.org_id === null, 'NEVER provisioned the MENU_NOT_FOUND source');
  assert(a.code === 3, `exit 3 signals partial failure — got ${a.code}`);
  assert(a.leaked === false, 'ops secret never printed');
  await sfA.close();

  // Scenario B — the visual-gate differentiator: API says verified:true, but the storefront renders BROKEN.
  // A raw provisioner (API-only) would call these DONE; demo-builder catches them at the visual gate.
  console.log('\nScenario B — visual gate catches broken renders the API marks verified (no-fake-green):');
  for (const variant of ['empty', 'error', 'orderable', 'noindexless']) {
    const mockB = await startMock({ secret: SECRET });
    const sfB = await startFakeStorefront({ variants: { [`veg-${variant}`]: variant } });
    const rowsB = [{ place_id: `place-goodmenu-${variant}`, slug: `veg-${variant}`, name: `Venue ${variant}`, website_url: 'https://x.example', cuisine: 'pizzeria' }];
    const bRes = await runOrchestrator({ pipelineUrl: mockB.baseUrl, storefrontUrl: sfB.baseUrl, secret: SECRET, fixturePath: fixture(`B-${variant}`, rowsB) });
    assert(bRes.report?.summary.certified_preview === 0, `[${variant}] NOT certified (0) despite API verified:true`);
    assert(/VISUAL_GATE_FAILED/.test(bRes.report?.results[0]?.reason || ''), `[${variant}] classified NEEDS-REVIEW:VISUAL_GATE_FAILED`);
    const src = mockB.sources.get(`place-goodmenu-${variant}`);
    assert(src?.state === 'VERIFIED', `[${variant}] source did reach API-VERIFIED (proving the gate — not the API — is what blocked it)`);
    await sfB.close();
    await mockB.close();
  }

  // Scenario C — opt-in outreach: --send-invite mints a real claim token (still preview-safe until then).
  console.log('\nScenario C — outreach is opt-in (--send-invite mints a real claim token):');
  const mockC = await startMock({ secret: SECRET });
  const sfC = await startFakeStorefront({ variants: { zeta: 'good' } });
  const rowsC = [{ place_id: 'place-goodmenu-zeta', slug: 'zeta', name: 'Zeta', website_url: 'https://z.example', cuisine: 'italian', invited_contact: 'z@x.com' }];
  const cRes = await runOrchestrator({ pipelineUrl: mockC.baseUrl, storefrontUrl: sfC.baseUrl, secret: SECRET, fixturePath: fixture('C', rowsC), sendInvite: true });
  assert(cRes.report?.summary.invited === 1, 'with --send-invite: exactly 1 invited');
  assert(/\/claim#token=/.test(cRes.report?.results[0]?.claim_url || ''), 'invited row carries a real fragment claim URL');
  await sfC.close(); await mockC.close();

  // Scenario D — idempotent re-run against the SAME mock+storefront: nothing re-provisioned/re-certified.
  console.log('\nScenario D — idempotent re-run (no double-provision):');
  const mockD = await startMock({ secret: SECRET });
  const sfD = await startFakeStorefront({ variants: { omega: 'good' } });
  const rowsD = [{ place_id: 'place-goodmenu-omega', slug: 'omega', name: 'Omega', website_url: 'https://o.example', cuisine: 'pizzeria', invited_contact: 'o@x.com' }];
  const d1 = await runOrchestrator({ pipelineUrl: mockD.baseUrl, storefrontUrl: sfD.baseUrl, secret: SECRET, fixturePath: fixture('D', rowsD), sendInvite: true });
  assert(d1.report?.summary.invited === 1, 'first run invites the good demo');
  const d2 = await runOrchestrator({ pipelineUrl: mockD.baseUrl, storefrontUrl: sfD.baseUrl, secret: SECRET, fixturePath: fixture('D', rowsD), sendInvite: true });
  assert(d2.report?.summary.invited === 0 && d2.report?.summary.skipped_already_done === 1, 're-run skips already-invited (no re-mint)');
  await sfD.close(); await mockD.close();

  // Scenario E — fail-closed on a wrong ops secret (surface 404s).
  console.log('\nScenario E — fail-closed: wrong ops secret (surface 404s):');
  const mockE = await startMock({ secret: SECRET });
  const sfE = await startFakeStorefront({ variants: { theta: 'good' } });
  const eRes = await runOrchestrator({ pipelineUrl: mockE.baseUrl, storefrontUrl: sfE.baseUrl, secret: 'WRONG-SECRET', fixturePath: fixture('E', [{ place_id: 'place-goodmenu-theta', slug: 'theta', name: 'Theta', website_url: 'https://t.example', cuisine: 'cafe' }]) });
  assert(eRes.report?.summary.certified_preview === 0 && eRes.report?.summary.invited === 0, 'zero faked successes when the secret is rejected');
  assert(/OPS_AUTH_404/.test(eRes.report?.results[0]?.reason || ''), 'classified NEEDS-REVIEW:OPS_AUTH_404 (fail-closed, not crash)');
  assert(eRes.leaked === false, 'wrong secret not echoed');
  await sfE.close(); await mockE.close();
  await mockA.close();

  console.log(`\n=== ${failures === 0 ? 'ALL GATES GREEN — anti-cheat dry-run PASS' : `${failures} ASSERTION(S) FAILED — RED`} ===`);
  process.exit(failures === 0 ? 0 : 1);
}

main().catch((e) => { console.error('HARNESS FATAL:', e); process.exit(1); });
