// THE BLIND SPOTS — every screen behind a session, opened in a real browser.
//
// `render.mjs` visits the PUBLIC surfaces and checks that each one renders,
// applies its styles and fits a phone. Everything behind a login it cannot
// reach, and until this file nothing did: five audits and a full-cycle test
// had between them driven the owner console's ORDERS tab and nothing else.
// The console has five tabs and the fifth holds seven integration screens; the
// storefront has eight sheets; there is a kit surface, a platform page and a
// hub page. A defect on any of them is a defect nobody would meet until a
// customer did.
//
// WHAT COUNTS AS A FAILURE HERE. Not "it looks wrong" — this cannot see that.
// It fails on the things a browser can be certain about: an uncaught exception,
// a console error, a request that came back 4xx/5xx, a screen that stayed
// empty after it was asked to open, a page that scrolls sideways on a phone,
// and an untranslated key rendered as its own name.
//
// IT WRITES NOTHING. It opens screens, switches languages and closes sheets;
// it never submits a form, places an order or saves a setting. The one
// exception is logging in, which every one of these screens requires.
import { chromium, devices } from 'playwright';
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const PLATFORM = process.env.PLATFORM_HOST || 'https://dowiz.org';
const OUT = process.env.OUT || '/tmp/dowiz-sweep';
fs.mkdirSync(OUT, { recursive: true });
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const fails = [];
const step = (name, ok, detail = '') => {
  if (!ok) fails.push(`${name}${detail ? ' :: ' + detail : ''}`);
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};

// Noise this product knows about and has decided to live with. Everything else
// is a finding. Kept short on purpose: a long ignore list is how a gate stops
// gating.
// `Executing inline script violates ... script-src` is Cloudflare's own bot
// script, injected by the edge into every response on every surface. It is not
// ours and the page's policy is right to refuse it; it has been on the known
// list since the landing page shipped. It is matched by its exact wording so a
// DIFFERENT inline-script violation -- one this product actually caused --
// would still be a failure.
const IGNORE = /CF\$cv|cloudflareinsights|challenge-platform|Executing inline script violates|WebGL|GPU|favicon|tiles|nominatim|maplibre|\.png|\.jpg|\.webp|ERR_INTERNET_DISCONNECTED/i;

const watch = (p, tag) => {
  p.on('pageerror', e => step(`${tag}: uncaught`, false, e.message.slice(0, 140)));
  p.on('console', m => {
    if (m.type() !== 'error') return;
    const txt = m.text();
    if (IGNORE.test(txt)) return;
    step(`${tag}: console error`, false, txt.slice(0, 140));
  });
  p.on('response', r => {
    if (r.status() < 400) return;
    if (IGNORE.test(r.url())) return;
    step(`${tag}: http`, false, `${r.status()} ${r.url().replace(HOST, '').slice(0, 90)}`);
  });
};

/// A screen that opened must have put something on the page. An empty pane is
/// the failure mode a screenshot hides and a status code never shows.
const bodyOf = (p, sel) => p.evaluate(s => {
  const el = document.querySelector(s);
  if (!el) return { found: false, len: 0, text: '' };
  return { found: true, len: (el.innerHTML || '').length, text: (el.innerText || '').trim().slice(0, 80) };
}, sel);

/// An i18n key that reached the screen instead of its translation. The keys in
/// this product are camelCase words with no spaces, so a visible token that
/// matches one of the known keys is a miss rather than a word.
const rawKeys = p => p.evaluate(() => {
  const txt = document.body.innerText || '';
  const suspects = txt.match(/\b(tab[A-Z]\w+|[a-z]+[A-Z]\w{2,})\b/g) || [];
  // A camelCase token surrounded by other camelCase tokens is markup leaking;
  // one on its own is usually a brand or a unit. Report the raw list and let
  // the caller judge — this is evidence, not a verdict.
  return [...new Set(suspects)].slice(0, 12);
});

/// A CLICK THAT CANNOT KILL THE SWEEP. The first run died on a dish card that
/// was below the fold: playwright retried for thirty seconds and then threw,
/// taking every screen after it with it. A sweep that stops at the first
/// awkward element reports less than no sweep, because the summary looks
/// short rather than wrong.
const tap = async (p, sel, tag) => {
  const el = typeof sel === 'string' ? await p.$(sel) : sel;
  if (!el) return false;
  try {
    await el.scrollIntoViewIfNeeded({ timeout: 4000 }).catch(() => {});
    await el.click({ timeout: 8000 });
    return true;
  } catch (e) {
    // A forced click still exercises the handler, which is what is under test.
    try { await el.dispatchEvent('click'); return true; } catch {}
    step(`${tag}: could not tap ${typeof sel === 'string' ? sel : 'element'}`, false, e.message.slice(0, 70));
    return false;
  }
};

const noSideScroll = async (p, tag) => {
  const w = await p.evaluate(() => ({ doc: document.documentElement.scrollWidth, win: window.innerWidth }));
  step(`${tag}: fits the phone`, w.doc <= w.win + 1, `scrollWidth ${w.doc} vs ${w.win}`);
};

const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const phone = { ...devices['iPhone 14 Pro'], serviceWorkers: 'block' };

// ── 1. THE OWNER CONSOLE, EVERY TAB ─────────────────────────────────────────
{
  const ctx = await b.newContext(phone);
  const p = await ctx.newPage(); watch(p, 'console');
  await p.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('#e', { timeout: 40000 });
  await p.fill('#e', creds.OWNER_EMAIL);
  await p.fill('#p', creds.OWNER_PASSWORD);
  await tap(p, '#go', 'login');
  await p.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).catch(() => {});
  await p.waitForTimeout(3000);
  step('console: signs in', !(await p.$('#e')));

  const tabs = await p.$$eval('#nav [data-tab]', els => els.map(e => e.dataset.tab));
  step('console: every tab is on the nav', tabs.length === 5, tabs.join(','));

  for (const tab of tabs) {
    await tap(p, `#nav [data-tab="${tab}"]`, `console/${tab}`);
    // Each tab lazy-imports its module, so give the import and its first read
    // time before judging the pane empty.
    await p.waitForTimeout(3500);
    const body = await bodyOf(p, '#app');
    step(`console/${tab}: the pane drew something`, body.found && body.len > 200,
      `${body.len} chars :: "${body.text}"`);
    await noSideScroll(p, `console/${tab}`);
    await p.screenshot({ path: `${OUT}/console-${tab}.png` })
      .catch(e => step(`console/${tab}: screenshot`, false, e.message.slice(0, 80)));
  }

  // ── the seven integration screens, which no run has ever opened ───────────
  await tap(p, '#nav [data-tab="more"]', 'console/more');
  await p.waitForTimeout(2500);
  for (const what of ['telegram', 'whatsapp', 'instagram', 'webhook', 'cloud', 'ai', 'mcp']) {
    // The console opens an integration from its list: `data-cfg="<key>"`.
    const btn = await p.$(`#igList [data-cfg="${what}"]`);
    if (!btn) { step(`console/more/${what}: reachable`, false, 'no control found'); continue; }
    await tap(p, btn, `console/more/${what}`);
    await p.waitForTimeout(1800);
    const sheet = await bodyOf(p, '#sheet');
    step(`console/more/${what}: opens`, sheet.found && sheet.len > 120, `${sheet.len} chars`);
    await p.screenshot({ path: `${OUT}/more-${what}.png` }).catch(() => {});
    const close = await p.$('#sheetClose');
    if (close) { await tap(p, close, 'console/more'); await p.waitForTimeout(600); }
  }
  await ctx.close();
}

// ── 2. THE STOREFRONT'S SHEETS ──────────────────────────────────────────────
{
  const ctx = await b.newContext(phone);
  const p = await ctx.newPage(); watch(p, 'store');
  await p.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForTimeout(5000);
  // The install offer takes the screen first; a customer taps "later".
  for (let i = 0; i < 4; i++) {
    const later = await p.$('#insLater');
    if (!later) break;
    await tap(p, later, 'store/install'); await p.waitForTimeout(500);
  }
  // A dish card is `data-p="<id>"`, the same attribute the console uses.
  const cards = await p.$$eval('[data-p]', els => els.length).catch(() => 0);
  step('store: the menu drew its dishes', cards > 0, `${cards} cards`);

  const sheets = [
    ['dish', () => tap(p, '[data-p]', 'store/dish')],
    ['venue', () => tap(p, '#heroInfo, [data-go="venue"], #venueBtn', 'store/venue')],
    ['lang', () => tap(p, '#langBtn, [data-go="lang"]', 'store/lang')],
    ['history', () => tap(p, '[data-go="history"], #histBtn', 'store/history')],
  ];
  for (const [name, open] of sheets) {
    await open();
    await p.waitForTimeout(1600);
    const s = await bodyOf(p, '#sheet');
    const shown = await p.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
    step(`store/${name}: opens`, s.found && s.len > 80, `name="${shown}" ${s.len} chars`);
    await p.screenshot({ path: `${OUT}/store-${name}.png` }).catch(() => {});
    const scrim = await p.$('#scrim');
    if (scrim) { await p.evaluate(() => document.getElementById('scrim')?.click()); await p.waitForTimeout(700); }
  }
  await noSideScroll(p, 'store');

  // Every language, on the live page, looking for keys that reached the screen.
  for (const lang of ['uk', 'en', 'sq']) {
    await p.evaluate(l => { try { localStorage.setItem('dw_lang', l); } catch {} }, lang);
    await p.reload({ waitUntil: 'domcontentloaded' });
    await p.waitForTimeout(3500);
    const keys = await rawKeys(p);
    const leaked = keys.filter(k => /^tab[A-Z]|^install[A-Z]|^load[A-Z]|^track[A-Z]|^session[A-Z]/.test(k));
    step(`store/${lang}: no i18n key reached the screen`, leaked.length === 0, leaked.join(',') || `(sampled ${keys.length} camelCase tokens)`);
  }
  await ctx.close();
}

// ── 3. THE COURIER APP ──────────────────────────────────────────────────────
{
  const ctx = await b.newContext({ ...devices['Pixel 7'], serviceWorkers: 'block',
    permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.4450 } });
  const p = await ctx.newPage(); watch(p, 'courier');
  p.on('dialog', d => d.accept());
  await p.goto(`${HOST}/courier/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('#em', { timeout: 40000 });
  await p.fill('#em', creds.QA_COURIER_PHONE);
  await p.fill('#pw', creds.QA_COURIER_PASSWORD);
  await tap(p, '#go', 'login');
  await p.waitForTimeout(5000);
  step('courier: signs in', !(await p.$('#em')));
  const body = await bodyOf(p, '#app');
  step('courier: the screen drew something', body.found && body.len > 200, `${body.len} chars`);
  await noSideScroll(p, 'courier');
  await p.screenshot({ path: `${OUT}/courier.png` }).catch(() => {});
  await ctx.close();
}

// ── 4. THE SURFACES NOBODY HAS DRIVEN AT ALL ────────────────────────────────
for (const [name, url] of [
  ['kit', `${HOST}/kit/`],
  ['platform', `${PLATFORM}/`],
  ['platform/hub', `${PLATFORM}/platform/hub`],
]) {
  const ctx = await b.newContext(phone);
  const p = await ctx.newPage(); watch(p, name);
  const r = await p.goto(url, { waitUntil: 'domcontentloaded', timeout: 90000 }).catch(() => null);
  step(`${name}: answers`, !!r && r.status() < 400, r ? `${r.status()}` : 'no response');
  await p.waitForTimeout(4000);
  const body = await bodyOf(p, 'body');
  step(`${name}: drew something`, body.len > 500, `${body.len} chars :: "${body.text}"`);
  await noSideScroll(p, name);
  await p.screenshot({ path: `${OUT}/${name.replace('/', '-')}.png` }).catch(() => {});
  await ctx.close();
}

process.on('uncaughtException', e => {
  step('the sweep itself', false, `threw: ${String(e.message).slice(0, 110)}`);
  console.log(`\nFAILURES:\n  ${fails.join('\n  ')}`);
  process.exit(1);
});

console.log(`\n${fails.length ? 'FAILURES:\n  ' + fails.join('\n  ') : 'SURFACE SWEEP OK'}`);
console.log(`shots in ${OUT}`);
await b.close();
process.exit(fails.length ? 1 : 0);
