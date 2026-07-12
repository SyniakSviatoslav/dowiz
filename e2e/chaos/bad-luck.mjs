// "Bad Luck" — a chaos monkey. It moves through the staging UI with NO purpose:
// random clicks, weird inputs, rapid double-clicks, random navigation/back/reload.
// It hunts the non-ordinary: uncaught exceptions, console errors, 5xx, XSS reflection,
// blank/error-boundary states — recording the action trail that triggered each.
import { chromium } from '@playwright/test';
import fs from 'node:fs';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const ART = 'e2e/chaos/artifacts';
const OWN = fs.readFileSync('e2e/chaos/.owner_tok', 'utf8').trim();
const COUR = fs.readFileSync('e2e/chaos/.courier_tok', 'utf8').trim();
fs.mkdirSync(ART, { recursive: true });

// Deterministic-ish PRNG so a run is reproducible (Date not used).
let seed = Number(process.env.CHAOS_SEED || 1337);
const rnd = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
const pick = (a) => a[Math.floor(rnd() * a.length)];
const chance = (p) => rnd() < p;

const WEIRD = [
  '🎲💥🔥👻', 'a'.repeat(9000), '<script>window.__xss=1</script>', '"><img src=x onerror=window.__xss2=1>',
// eslint-disable-next-line local/no-raw-sql -- dynamic query
  "'; DROP TABLE orders;--", '-99999999', '0', '   ', '٩(◕‿◕)۶', '‮RTLtext', 'NaN', '${7*7}',
  '999999999999999999999999', '../../etc/passwd', '%00', 'null', 'undefined', '+355600000000', '🍣'.repeat(200),
];
const SESSIONS = [
  { name: 'storefront', url: '/s/demo', token: null },
  { name: 'checkout', url: '/s/demo/checkout', token: null },
  { name: 'owner-admin', url: '/admin', token: OWN },
  { name: 'owner-menu', url: '/admin/menu', token: OWN },
  { name: 'courier', url: '/courier', token: COUR },
  { name: 'root', url: '/', token: null },
];
// Occasionally jump to a malformed deep link mid-session.
const WILD_ROUTES = ['/s/demo/order/not-a-uuid', '/admin/menu/%FF', '/s/%00/menu', '/courier/delivery/0',
  '/admin/settings?x=<script>', '/s/demo?lang=zz', '/admin/../../etc', '/s/demo/order/' + 'x'.repeat(500)];

const findings = [];
const seen = new Set();
function record(kind, detail, ctx) {
  const sig = `${kind}::${String(detail).slice(0, 140)}`;
  if (seen.has(sig)) { findings.find(f => f.sig === sig).count++; return null; }
  seen.add(sig);
  const f = { sig, kind, detail: String(detail).slice(0, 600), url: ctx.url, trail: ctx.trail.slice(-6), count: 1 };
  findings.push(f);
  return f;
}

const ACTIONS = ['click', 'click', 'click', 'type', 'type', 'key', 'nav-back', 'reload', 'scroll', 'rapid', 'wild', 'resize'];

async function chaosSession(browser, sess, steps) {
  const ctx = await browser.newContext({ viewport: { width: chance(0.5) ? 390 : 1280, height: chance(0.5) ? 844 : 800 } });
  const page = await ctx.newPage();
  const trail = [];
  const cur = { url: sess.url, trail };

  page.on('pageerror', (e) => { const f = record('PAGEERROR', e.message, { url: page.url(), trail }); if (f) page.screenshot({ path: `${ART}/err-${findings.length}.png` }).catch(() => {}); });
  page.on('console', (m) => { if (m.type() === 'error') { const t = m.text(); if (!/favicon|net::ERR|Failed to load resource|status of 4|websocket|Manifest|preload/i.test(t)) record('CONSOLE', t, { url: page.url(), trail }); } });
  page.on('response', (r) => { if (r.status() >= 500) record('HTTP5XX', `${r.status()} ${r.request().method()} ${r.url().replace(BASE, '')}`, { url: page.url(), trail }); });
  page.on('dialog', async (d) => { record('DIALOG', `${d.type()}: ${d.message()}`.slice(0, 120), { url: page.url(), trail }); await d.dismiss().catch(() => {}); });

  if (sess.token) await page.addInitScript((t) => localStorage.setItem('dos_access_token', t), sess.token);
  try { await page.goto(`${BASE}${sess.url}`, { waitUntil: 'domcontentloaded', timeout: 20000 }); } catch (e) { record('NAV', `goto ${sess.url}: ${e.message}`, cur); }
  await page.waitForTimeout(800);

  for (let i = 0; i < steps; i++) {
    const act = pick(ACTIONS);
    try {
      if (act === 'click' || act === 'rapid') {
        const els = await page.locator('button:visible, a:visible, [role="button"]:visible, input[type="checkbox"]:visible, [data-testid]:visible').all();
        if (els.length) {
          const el = pick(els);
          const n = act === 'rapid' ? 4 : 1;
          for (let k = 0; k < n; k++) await el.click({ timeout: 1500, force: chance(0.3), noWaitAfter: true }).catch(() => {});
          trail.push(`${act}(${(await el.innerText().catch(() => '') || (await el.getAttribute('data-testid').catch(() => '')) || 'el').slice(0, 24)})`);
        }
      } else if (act === 'type') {
        const inputs = await page.locator('input:visible, textarea:visible').all();
        if (inputs.length) { const inp = pick(inputs); const v = pick(WEIRD); await inp.fill(v, { timeout: 1500 }).catch(() => {}); await inp.press('Enter', { timeout: 800 }).catch(() => {}); trail.push(`type(${v.slice(0, 16)})`); }
      } else if (act === 'key') { await page.keyboard.press(pick(['Enter', 'Escape', 'Tab', 'Backspace', 'ArrowDown'])); trail.push('key'); }
      else if (act === 'nav-back') { if (chance(0.5)) await page.goBack({ timeout: 4000 }).catch(() => {}); else await page.goForward({ timeout: 4000 }).catch(() => {}); trail.push('nav'); }
      else if (act === 'reload') { await page.reload({ timeout: 10000, waitUntil: 'domcontentloaded' }).catch(() => {}); trail.push('reload'); }
      else if (act === 'scroll') { await page.mouse.wheel(0, (rnd() - 0.5) * 4000).catch(() => {}); trail.push('scroll'); }
      else if (act === 'resize') { await page.setViewportSize({ width: 320 + Math.floor(rnd() * 1100), height: 600 + Math.floor(rnd() * 400) }).catch(() => {}); trail.push('resize'); }
      else if (act === 'wild') { const w = pick(WILD_ROUTES); await page.goto(`${BASE}${w}`, { waitUntil: 'domcontentloaded', timeout: 12000 }).catch((e) => record('NAV', `wild ${w}: ${e.message}`, { url: page.url(), trail })); trail.push(`wild(${w.slice(0, 30)})`); }
      await page.waitForTimeout(120 + Math.floor(rnd() * 250));
      // Detect blank / error-boundary state after the action.
      const txt = (await page.locator('body').innerText({ timeout: 1500 }).catch(() => 'x')).trim();
      if (txt.length < 2 && !page.url().includes('blank')) record('BLANK', `near-empty body at ${page.url().replace(BASE, '')}`, { url: page.url(), trail });
      if (/something went wrong|unexpected error|application error|cannot read propert|undefined is not/i.test(txt)) record('ERROR-UI', txt.slice(0, 140), { url: page.url(), trail });
      // XSS reflection canary.
      if (await page.evaluate(() => window.__xss || window.__xss2).catch(() => false)) record('XSS', `reflected script executed at ${page.url().replace(BASE, '')}`, { url: page.url(), trail });
    } catch (e) { record('MONKEY', `${act}: ${e.message}`, { url: page.url(), trail }); }
  }
  await ctx.close().catch(() => {});
}

const browser = await chromium.launch();
const STEPS = Number(process.env.CHAOS_STEPS || 45);
const ROUNDS = Number(process.env.CHAOS_ROUNDS || 1);
let total = 0;
for (let r = 0; r < ROUNDS; r++) for (const s of SESSIONS) { await chaosSession(browser, s, STEPS); total += STEPS; process.stdout.write('.'); }
await browser.close();

console.log(`\n\n=== Bad Luck report — ${total} chaotic actions across ${SESSIONS.length * ROUNDS} sessions ===`);
const order = ['PAGEERROR', 'XSS', 'ERROR-UI', 'BLANK', 'HTTP5XX', 'CONSOLE', 'DIALOG', 'NAV', 'MONKEY'];
findings.sort((a, b) => order.indexOf(a.kind) - order.indexOf(b.kind));
if (!findings.length) console.log('No anomalies captured.');
for (const f of findings) console.log(`\n[${f.kind} x${f.count}] ${f.detail}\n   url: ${f.url.replace(BASE, '')}\n   trail: ${f.trail.join(' → ')}`);
fs.writeFileSync(`${ART}/report.json`, JSON.stringify(findings, null, 2));
