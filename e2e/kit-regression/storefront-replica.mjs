// Storefront-replica gate (BN3, menu half) -- what a RETURN visit costs.
//
// BN2 took the menu read off the Worker (storefront-r2.mjs counts that). BN3
// keeps the published root and its content-addressed objects on the device
// (`lib/blocks.js`, used by `store/shell.js`), so a return visit should cost:
//   * inside the root's 30 s ............ ZERO requests for the menu
//   * after it, nothing changed ......... the root, and no object
//   * a new generation .................. the root + only the names that moved
//   * a tampered device copy ............ refused, and that one object re-read
//   * offline ........................... the menu, from the device
// and a CONTROL: a device that never visited, offline, paints nothing.
//
// Measured at the wire: service workers are BLOCKED, so what is counted is the
// IndexedDB layer alone (the SW owns the shell and the photos and has its own
// gates). `/api/*` is aborted in every visit, so a menu that paints did not
// come from the hub. The page is `workers/api/public/` from disk on loopback and
// the CDN a route handler serving two fixture generations under REAL k64 names
// (sha256 of the bytes, as `hubdo/publish.rs` writes them). Nothing live.
// Warm time-to-menu is the first `.card` in the DOM after navigation start,
// printed plain and with the CPU throttled 4x (CDP), never claimed beyond that.
//
//     node e2e/kit-regression/storefront-replica.mjs
// ZERO NEW DEPENDENCIES. ASCII QUOTES ONLY.

import { chromium } from 'playwright';
import http from 'node:http';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const PUBLIC = fileURLToPath(new URL('../../workers/api/public/', import.meta.url));
const SLUG = 'dubin';
const CDN = 'https://cdn.dowiz.org';
const TYPES = { '.js': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.html': 'text/html; charset=utf-8', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' };
const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `\n      ${detail}` : ''}`);
}

// ── two generations, named by content ───────────────────────────────────────
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhQGAWjR9awAAAABJRU5ErkJggg==', 'base64');
const WEEK = Array.from({ length: 7 }, () => [{ open: 0, close: 1440 }]);
const product = (id, name, price, cat, extra = {}) => ({ id, name, description: null, price, available: true, ingredients: null, sortOrder: 1,
  unavailableNote: null, imageUrl: null, imageUrlSmall: null, allergens: [], modifierGroups: null, tags: [], categoryId: cat, ...extra });
const location = { id: 'loc_1', name: 'Dubin & Sushi', slug: SLUG, phone: '+355', deliveryFee: 200, minOrder: 1000, currencyCode: 'ALL',
  supportedLocales: ['sq', 'en'], defaultLocale: 'sq', tz: 'Europe/Tirane', logoUrl: '/media/logo.png', hours: WEEK, ownerStatus: 'open',
  deliveryPaused: false, features: {}, payments: { cash: true, card: false, applePay: false, googlePay: false, crypto: [] } };
const k64 = b => createHash('sha256').update(b).digest('hex').slice(0, 16);
function generation(makiPrice) {
  const fragment = JSON.stringify({ categories: [
    { id: 'c_soup', name: 'Supa', sortOrder: 1, products: [product('p_s', 'Miso', 400, 'c_soup')] },
    { id: 'c_rolls', name: 'Rolls', sortOrder: 2, products: [product('p_a', 'Nigiri', 700, 'c_rolls', { imageUrl: '/media/aaa.jpg' }),
      product('p_b', 'Maki', makiPrice, 'c_rolls', { imageUrl: '/media/bbb.jpg' })] },
  ], location, stripePublishableKey: null, warnings: [] });
  const words = JSON.stringify({ words: { p_b: { name: 'Maki roll' } }, warnings: [] });
  const media = JSON.stringify(['aaa.jpg', 'bbb.jpg', 'logo.png']);
  const key = s => `${k64(Buffer.from(s))}.json`;
  const root = JSON.stringify({ v: 1, slug: SLUG, default: 'sq', locales: ['sq', 'en'], fragment: key(fragment), words: { en: key(words) }, blocks: {}, media: key(media) });
  return { root, fragmentKey: key(fragment), objects: { [key(fragment)]: fragment, [key(words)]: words, [key(media)]: media } };
}
const GENS = [generation(900), generation(950)];

// ── the page from disk; /api/* is aborted by the browse, not answered here ──
async function serve() {
  const srv = http.createServer(async (req, res) => {
    const p = new URL(req.url, 'http://x').pathname;
    const file = p === '/' ? path.join(PUBLIC, 'store/index.html') : path.normalize(path.join(PUBLIC, p));
    try {
      if (!file.startsWith(PUBLIC)) throw new Error('outside');
      const body = p.startsWith('/media/') ? PNG : await readFile(file);
      res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream', 'cache-control': 'no-store' });
      res.end(body);
    } catch { res.writeHead(404); res.end(); }
  });
  await new Promise(r => srv.listen(0, '127.0.0.1', r));
  return { origin: `http://127.0.0.1:${srv.address().port}`, close: () => new Promise(r => srv.close(r)) };
}

/// The CDN as a switchboard: which generation it publishes and whether it is reachable.
function cdnRoute(ctx, cdn) {
  return ctx.route(`${CDN}/**`, route => {
    const key = new URL(route.request().url()).pathname.replace(`/v/${SLUG}/`, '');
    cdn.asked.push(key);
    if (!cdn.online) return route.abort('internetdisconnected');
    const g = GENS[cdn.gen];
    const cors = { 'access-control-allow-origin': '*' };
    if (key === 'manifest.json') return route.fulfill({ status: 200, headers: { ...cors, 'content-type': 'application/json', 'cache-control': 'public, max-age=30' }, body: g.root });
    if (key.startsWith('m/')) return route.fulfill({ status: 200, headers: { ...cors, 'content-type': 'image/png', 'cache-control': 'public, max-age=31536000, immutable' }, body: PNG });
    if (g.objects[key]) return route.fulfill({ status: 200, headers: { ...cors, 'content-type': 'application/json', 'cache-control': 'public, max-age=31536000, immutable' }, body: g.objects[key] });
    return route.fulfill({ status: 404, headers: cors, body: '' });
  });
}

/// The time the first `.card` entered the DOM, after navigation start.
const FIRST_CARD = () => {
  new MutationObserver((_, o) => { if (document.querySelector('.card')) { window.__firstCard = performance.now(); o.disconnect(); } })
    .observe(document, { childList: true, subtree: true });
};

/// One visit in `ctx`. Returns the CDN keys asked, the cards painted, the time to the first card and the console warnings.
async function visit(ctx, srv, cdn, { throttle = 1 } = {}) {
  cdn.asked = [];
  const page = await ctx.newPage();
  const warns = [];
  page.on('console', m => { if (m.type() === 'warning' || m.type() === 'error') warns.push(m.text().slice(0, 140)); });
  if (throttle > 1) await (await ctx.newCDPSession(page)).send('Emulation.setCPUThrottlingRate', { rate: throttle });
  await page.goto(`${srv.origin}/?s=${SLUG}&cdn=${encodeURIComponent(CDN)}`, { waitUntil: 'load' });
  let cards = 0, ms = null, prices = '', read = null;
  try {
    await page.waitForSelector('.card', { timeout: 20_000 });
    await page.waitForTimeout(500);
    cards = await page.locator('.card').count();
    ms = await page.evaluate(() => window.__firstCard);
    read = await page.evaluate(() => { const e = performance.getEntriesByName('dowiz:menu-read')[0]; return e ? e.duration : null; });
    prices = (await page.locator('.card-price').allInnerTexts()).join(' | ');
  } catch { /* cards stays 0: the check below says so */ }
  await page.close();
  const objects = cdn.asked.filter(k => k !== 'manifest.json' && !k.startsWith('m/'));
  return { asked: [...cdn.asked], objects, roots: cdn.asked.filter(k => k === 'manifest.json').length, photos: cdn.asked.filter(k => k.startsWith('m/')).length, cards, ms, read, prices, warns };
}

/// Edit the device copy from inside the page: age the root, or flip one byte of an object.
async function onDevice(ctx, srv, what, arg) {
  const page = await ctx.newPage();
  await page.goto(`${srv.origin}/lib/tokens.css`);
  const out = await page.evaluate(async ([what, arg, slug]) => {
    const db = await new Promise((ok, no) => { const r = indexedDB.open('dowiz.blocks.v1', 1); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); });
    const root = what === 'age' || what === 'fresh';
    const store = root ? 'roots' : 'objects';
    const key = root ? slug : `${slug}/${arg}`;
    const os = () => db.transaction(store, 'readwrite').objectStore(store);
    const v = await new Promise(ok => { const r = os().get(key); r.onsuccess = () => ok(r.result); });
    if (!v) return 'absent';
    if (what === 'age') v.at -= arg; else if (what === 'fresh') v.at = Date.now(); else new Uint8Array(v.bytes)[5] ^= 1;
    await new Promise(ok => { const r = os().put(v, key); r.onsuccess = ok; });
    return 'done';
  }, [what, arg, SLUG]);
  await page.close();
  return out;
}

export async function run() {
  const srv = await serve();
  const browser = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'] });
  const newCtx = async () => {
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, serviceWorkers: 'block', locale: 'sq' });
    await ctx.addInitScript(FIRST_CARD);
    await ctx.route(`${srv.origin}/api/**`, r => r.abort('failed'));
    return ctx;
  };
  try {
    const cdn = { gen: 0, online: true, asked: [] };
    const ctx = await newCtx();
    await cdnRoute(ctx, cdn);
    const cold = await visit(ctx, srv, cdn);
    check('cold visit paints from the CDN (/api aborted)', cold.cards === 3, `asked=${JSON.stringify(cold.asked)} warns=${JSON.stringify(cold.warns)}`);
    await onDevice(ctx, srv, 'fresh');
    const warm = await visit(ctx, srv, cdn);
    check(`return visit inside max-age: 0 CDN requests for the menu (root ${warm.roots}, objects ${warm.objects.length})`, warm.cards === 3 && warm.roots === 0 && warm.objects.length === 0, JSON.stringify(warm.asked));
    await onDevice(ctx, srv, 'fresh');
    const warmT = await visit(ctx, srv, cdn, { throttle: 4 });
    check(`warm visit at CPU/4 inside max-age: 0 CDN requests for the menu (asked ${JSON.stringify(warmT.asked)})`, warmT.cards === 3 && warmT.roots === 0 && warmT.objects.length === 0);
    check(`aged root ${await onDevice(ctx, srv, 'age', 60_000)}`, true);
    const same = await visit(ctx, srv, cdn);
    check(`return visit after max-age, unchanged root: the root only, 0 objects (asked ${JSON.stringify(same.asked)})`, same.cards === 3 && same.roots === 1 && same.objects.length === 0);
    cdn.gen = 1;
    await onDevice(ctx, srv, 'age', 60_000);
    const moved = await visit(ctx, srv, cdn);
    check('a new generation fetches only the changed k64', moved.cards === 3 && JSON.stringify(moved.objects) === JSON.stringify([GENS[1].fragmentKey]) && /950/.test(moved.prices),
      `objects=${JSON.stringify(moved.objects)} want=${JSON.stringify([GENS[1].fragmentKey])} prices=${JSON.stringify(moved.prices)}`);
    check(`tampered the device copy of the fragment: ${await onDevice(ctx, srv, 'flip', GENS[1].fragmentKey)}`, true);
    await onDevice(ctx, srv, 'age', 60_000);
    const tampered = await visit(ctx, srv, cdn);
    check('a tampered device blob is refused and re-read (that object only)', tampered.cards === 3 && JSON.stringify(tampered.objects) === JSON.stringify([GENS[1].fragmentKey]) && tampered.warns.some(w => /not what its name says/.test(w)),
      `objects=${JSON.stringify(tampered.objects)} warns=${JSON.stringify(tampered.warns)}`);
    cdn.online = false;
    await onDevice(ctx, srv, 'age', 3_600_000);
    const offline = await visit(ctx, srv, cdn);
    check('OFFLINE (CDN unreachable, /api aborted): the menu paints from the device', offline.cards === 3 && /950/.test(offline.prices), `cards=${offline.cards} prices=${JSON.stringify(offline.prices)} asked=${JSON.stringify(offline.asked)} warns=${JSON.stringify(offline.warns)}`);
    const offT = await visit(ctx, srv, cdn, { throttle: 4 });
    await ctx.close();

    const control = await newCtx();
    const dark = { gen: 1, online: false, asked: [] };
    await cdnRoute(control, dark);
    const none = await visit(control, srv, dark);
    check('CONTROL -- a device that never visited, offline, paints no menu', none.cards === 0, `cards=${none.cards}`);
    await control.close();

    const r = v => (v == null ? 'n/a' : `${Math.round(v)} ms`);
    console.log(`\nSUMMARY requests to the CDN per visit (root / k64 objects / photos):`);
    for (const [name, v] of [['cold', cold], ['warm <30 s', warm], ['warm, unchanged root', same], ['new generation', moved], ['tampered blob', tampered], ['offline (attempted)', offline]]) {
      console.log(`      ${name.padEnd(22)} ${v.roots} / ${v.objects.length} / ${v.photos}`);
    }
    console.log(`SUMMARY time to first menu card: cold ${r(cold.ms)}, warm ${r(warm.ms)}, warm CPU/4 ${r(warmT.ms)}, offline ${r(offline.ms)}, offline CPU/4 ${r(offT.ms)}`);
    console.log(`SUMMARY the menu read alone (store/shell.js fromCdn: root + objects + sha256 verify + assemble): cold ${r(cold.read)}, warm ${r(warm.read)}, warm CPU/4 ${r(warmT.read)}, unchanged root ${r(same.read)}, offline ${r(offline.read)}, offline CPU/4 ${r(offT.read)}`);
    console.log('      (the shell itself is read from loopback with no-store on every visit: no service worker here)');
  } finally {
    await browser.close();
    await srv.close();
  }
  const failed = results.filter(x => !x.ok).length;
  console.log(failed ? `\nstorefront-replica: ${failed} failure(s)` : '\nstorefront-replica: everything passes');
  return failed;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  process.exit(await run());
}
