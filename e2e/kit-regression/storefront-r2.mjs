// Storefront-R2 gate (BN2) -- how many WORKER requests a cold browse costs.
//
// THE NUMBER THIS MEASURES. Before BN2 a cold visit to a venue's storefront was
// about 22 Worker requests (the root, the menu, the i18n, the manifest, every
// photo through /media/*). With the venue's object publishing to R2 and the
// shell reading from the CDN (store/shell.js), the Worker answers ONE request
// per browse: `/` (run_worker_first, the host decides which front door). This
// gate counts that in a real browser, with `/api/*` BLOCKED at the proxy, so
// what it proves is the fail-closed property too: when the Worker's API is
// unreachable -- a daily cap tripped, a 503 -- the menu still paints.
//
// WHAT COUNTS AS A WORKER REQUEST. The paths production answers from the
// Worker and not from the asset layer: `/`, `/api/*`, `/media/*` and
// `/manifest.webmanifest`. Everything else under public/ is a static asset
// (free, unlimited, served without the Worker), and the CDN is another origin.
//
// NOTHING LIVE IS TOUCHED. The page is `workers/api/public/` served from disk
// on loopback (as outbox.mjs does) and the CDN is a route handler serving a
// fixture generation -- the same objects `hubdo/publish.rs` writes, in the
// shape `hubdo/publish/tests.rs` pins. `LIVE=https://qa-durres.dowiz.org`
// adds one read-only browse of a real venue and prints ITS count beside the
// local one: that is the "from about 22" half of the acceptance, measured
// rather than remembered. ZERO NEW DEPENDENCIES (the `playwright` library
// already in the tree). ASCII QUOTES ONLY.
//
//     node e2e/kit-regression/storefront-r2.mjs
//     LIVE=https://qa-durres.dowiz.org node e2e/kit-regression/storefront-r2.mjs

import { chromium } from 'playwright';
import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const PUBLIC = fileURLToPath(new URL('../../workers/api/public/', import.meta.url));
const SLUG = 'dubin';
const CDN = 'https://cdn.dowiz.org';
const BASE = `${CDN}/v/${SLUG}/`;
const TYPES = {
  '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2',
  '.json': 'application/json; charset=utf-8', '.webmanifest': 'application/manifest+json', '.webp': 'image/webp', '.jpg': 'image/jpeg',
};
/// What production answers from the Worker rather than from the asset layer.
const isWorkerPath = p => p === '/' || p === '/index.html' || p.startsWith('/api/') || p.startsWith('/media/') || p === '/manifest.webmanifest';

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `\n      ${detail}` : ''}`);
}

// ── the fixture generation: what the object publishes ───────────────────────
const WEEK = Array.from({ length: 7 }, () => [{ open: 0, close: 1440 }]);
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhQGAWjR9awAAAABJRU5ErkJggg==', 'base64');
const product = (id, name, price, cat, extra = {}) => ({
  id, name, description: null, price, available: true, ingredients: null, sortOrder: 1, unavailableNote: null,
  imageUrl: null, imageUrlSmall: null, allergens: [], modifierGroups: null, sizeCm: null, cookingMin: null, tags: [], weightG: null,
  nutrition: null, nutritionDerived: null, taste: null, station: null, calories: null, categoryId: cat, ...extra,
});
const location = {
  id: 'loc_1', name: 'Dubin & Sushi', slug: SLUG, phone: '+355', address: null, closesAt: null, deliveryEta: '30-40',
  deliveryFee: 200, freeDeliveryThreshold: null, minOrder: 1000, currencyCode: 'ALL', menuVersion: 7,
  supportedLocales: ['sq', 'en'], defaultLocale: 'sq', tz: 'Europe/Tirane', theme: null, logoUrl: '/media/logo.png', stage: null,
  lat: null, lng: null, google: null, hours: WEEK, pickup: false, hasDeliveryZones: false, deliveryZones: [], ownerStatus: 'open',
  deliveryPaused: false, features: {}, telegramBot: null, payments: { cash: true, card: false, applePay: false, googlePay: false, crypto: [] },
};
const fragment = {
  categories: [
    { id: 'c_soup', name: 'Supa', sortOrder: 1, products: [product('p_s', 'Miso', 400, 'c_soup')] },
    { id: 'c_rolls', name: 'Rolls', sortOrder: 2, products: [
      product('p_a', 'Nigiri', 700, 'c_rolls', { imageUrl: '/media/aaa.jpg' }),
      product('p_b', 'Maki', 900, 'c_rolls', { imageUrl: '/media/bbb.jpg', imageUrlSmall: '/media/bbbs.jpg' }),
    ] },
  ],
  location, stripePublishableKey: null, warnings: [],
};
const words = { words: { p_b: { name: 'Maki roll' }, c_rolls: { name: 'Rolls EN' } }, warnings: [] };
const media = ['aaa.jpg', 'bbb.jpg', 'bbbs.jpg', 'logo.png'];
const manifest = {
  v: 1, slug: SLUG, gens: { catalog: 3, i18n: 2, settings: 1 }, default: 'sq', locales: ['sq', 'en'],
  fragment: 'f1.json', words: { en: 'w1.json' }, blocks: { menu_prices: 'p1.dwb', names: 'n1.dwb' }, media: 'm1.json',
};
const OBJECTS = {
  'manifest.json': [JSON.stringify(manifest), 'application/json', 'public, max-age=30'],
  'f1.json': [JSON.stringify(fragment), 'application/json', 'public, max-age=31536000, immutable'],
  'w1.json': [JSON.stringify(words), 'application/json', 'public, max-age=31536000, immutable'],
  'm1.json': [JSON.stringify(media), 'application/json', 'public, max-age=31536000, immutable'],
  'm/aaa.jpg': [PNG, 'image/png', 'public, max-age=31536000, immutable'],
  'm/bbb.jpg': [PNG, 'image/png', 'public, max-age=31536000, immutable'],
  'm/bbbs.jpg': [PNG, 'image/png', 'public, max-age=31536000, immutable'],
  'm/logo.png': [PNG, 'image/png', 'public, max-age=31536000, immutable'],
};

/// The hub's own menu answer (the fallback path), with the clock as the Worker adds it.
const hubMenu = () => JSON.stringify({ ...fragment, location: { ...location, status: 'open', nextOpen: null, closedReason: null } });

// ── the page, from disk, answering the Worker's paths as the Worker would ───
async function serve() {
  const srv = http.createServer(async (req, res) => {
    const url = new URL(req.url, 'http://x');
    const p = url.pathname;
    if (p === '/' || p === '/index.html') return send(res, await readFile(path.join(PUBLIC, 'store/index.html')), TYPES['.html']);
    if (p === '/manifest.webmanifest') return send(res, JSON.stringify({ name: 'Dubin & Sushi', start_url: '/', icons: [] }), TYPES['.webmanifest']);
    if (p === `/api/public/locations/${SLUG}/menu`) return send(res, hubMenu(), TYPES['.json']);
    if (p.startsWith('/api/')) { res.writeHead(404); return res.end('{}'); }
    if (p.startsWith('/media/')) return send(res, PNG, 'image/png');
    try {
      const file = path.normalize(path.join(PUBLIC, p));
      if (!file.startsWith(PUBLIC)) throw new Error('outside');
      send(res, await readFile(file), TYPES[path.extname(file)] || 'application/octet-stream');
    } catch { res.writeHead(404); res.end(); }
  });
  await new Promise(r => srv.listen(0, '127.0.0.1', r));
  return { origin: `http://127.0.0.1:${srv.address().port}`, close: () => new Promise(r => srv.close(r)) };
}
function send(res, body, type) {
  res.writeHead(200, { 'content-type': type, 'cache-control': 'no-store' });
  res.end(body);
}

/// One cold browse. Returns what was asked of whom, and what painted.
async function browse(browser, origin, { cdnUp, blockApi, locale = 'sq', live = false }) {
  // Service workers are BLOCKED here so every request is counted at the wire; what sw.js adds on a second visit is its own gate.
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, serviceWorkers: 'block', locale });
  const page = await ctx.newPage();
  const asked = { worker: [], asset: 0, cdn: [], blocked: [], other: [] };
  const consoleErrors = [];
  page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text().slice(0, 160)); });
  page.on('pageerror', e => consoleErrors.push('PAGE ERROR: ' + String(e.message).slice(0, 160)));
  page.on('request', r => {
    const u = new URL(r.url());
    if (u.origin === origin) {
      if (isWorkerPath(u.pathname)) asked.worker.push(u.pathname + u.search);
      else asked.asset++;
    } else if (u.origin === CDN) asked.cdn.push(u.pathname);
    else asked.other.push(u.href.slice(0, 120));
  });
  if (!live) {
    await page.route(`${CDN}/**`, route => {
      const key = new URL(route.request().url()).pathname.replace(`/v/${SLUG}/`, '');
      const o = cdnUp ? OBJECTS[key] : null;
      if (!o) return route.fulfill({ status: 404, headers: { 'access-control-allow-origin': '*' }, body: '' });
      return route.fulfill({ status: 200, headers: { 'content-type': o[1], 'cache-control': o[2], 'access-control-allow-origin': '*' }, body: o[0] });
    });
  }
  if (blockApi) {
    await page.route(`${origin}/api/**`, route => { asked.blocked.push(new URL(route.request().url()).pathname); route.abort('failed'); });
  }
  const q = live ? '' : `/?s=${SLUG}&cdn=${encodeURIComponent(CDN)}`;
  await page.goto(origin + q, { waitUntil: 'load' });
  let cards = 0, brand = '', firstImg = '';
  try {
    await page.waitForSelector('.card', { timeout: 25_000 });
    // Let the lazy images and anything the first paint schedules ask for what they need.
    await page.waitForTimeout(1500);
    cards = await page.locator('.card').count();
    brand = (await page.locator('#brandName').textContent()) || '';
    firstImg = (await page.locator('.card img').first().getAttribute('src').catch(() => '')) || '';
  } catch {
    await page.waitForTimeout(1500);
  }
  await ctx.close();
  return { asked, cards, brand: brand.trim(), firstImg, consoleErrors };
}

export async function run() {
  const srv = await serve();
  const browser = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'] });
  try {
    // 1. THE ACCEPTANCE: CDN up, /api/* blocked -> the menu paints, the Worker answered one request.
    const a = await browse(browser, srv.origin, { cdnUp: true, blockApi: true });
    check('cold browse with /api/* blocked: the menu paints from the CDN', a.cards === 3 && a.brand === 'Dubin & Sushi',
      `cards=${a.cards} brand=${JSON.stringify(a.brand)} blocked=${JSON.stringify(a.asked.blocked)} errors=${JSON.stringify(a.consoleErrors)}`);
    const workerN = a.asked.worker.length;
    check(`Worker requests per cold browse <= 1 (measured ${workerN})`, workerN <= 1,
      `worker=${JSON.stringify(a.asked.worker)} cdn=${a.asked.cdn.length} assets=${a.asked.asset} other=${JSON.stringify(a.asked.other)}`);
    check('the photos are read from the CDN, not /media/', a.firstImg.startsWith(BASE + 'm/'), `first card img src=${a.firstImg}`);
    check('every CDN read is the root or a content-addressed object', a.asked.cdn.every(p => p === `/v/${SLUG}/manifest.json` || /^\/v\/dubin\/(m\/)?[A-Za-z0-9.-]+\.(json|dwb|png|jpg|webp)$/.test(p)),
      JSON.stringify(a.asked.cdn));
    check('no page error while reading the published menu', a.consoleErrors.length === 0, JSON.stringify(a.consoleErrors));

    // 2. ANOTHER LANGUAGE: the words object is read too; the fragment is the same object.
    const en = await browse(browser, srv.origin, { cdnUp: true, blockApi: true, locale: 'en' });
    check('a customer in another language reads the words object (one more immutable read)', en.cards === 3 && en.asked.cdn.includes(`/v/${SLUG}/w1.json`),
      `cdn=${JSON.stringify(en.asked.cdn)}`);

    // 3. THE FALLBACK: no manifest on the CDN -> the hub's route, as today; the Worker answered two.
    const b = await browse(browser, srv.origin, { cdnUp: false, blockApi: false });
    check('without a published manifest the menu still paints, through the Worker', b.cards === 3 && b.asked.worker.some(p => p.startsWith(`/api/public/locations/${SLUG}/menu`)),
      `worker=${JSON.stringify(b.asked.worker)} cdn=${JSON.stringify(b.asked.cdn)}`);
    check('the fallback asked the CDN once (the root) before the hub', b.asked.cdn.length === 1 && b.asked.cdn[0] === `/v/${SLUG}/manifest.json`, JSON.stringify(b.asked.cdn));

    // 4. THE CONTROL: CDN down AND /api/* blocked -> this is the day a cap trips without BN2: nothing paints.
    const c = await browse(browser, srv.origin, { cdnUp: false, blockApi: true });
    check('CONTROL -- no CDN and no API paints no menu (what a tripped cap meant before BN2)', c.cards === 0, `cards=${c.cards}`);

    console.log(`\nSUMMARY local: worker_requests_per_cold_browse=${workerN} (${JSON.stringify(a.asked.worker)}) cdn_reads=${a.asked.cdn.length} static_assets=${a.asked.asset} fallback_worker_requests=${b.asked.worker.length}`);

    // 5. LIVE (read-only, optional): the venue as deployed today, for the "from about 22".
    const live = process.env.LIVE;
    if (live) {
      const l = await browse(browser, live, { cdnUp: true, blockApi: false, live: true });
      console.log(`SUMMARY live ${live}: worker_requests_per_cold_browse=${l.asked.worker.length} cards=${l.cards} cdn_reads=${l.asked.cdn.length} static_assets=${l.asked.asset}`);
      console.log(`      worker paths: ${JSON.stringify(l.asked.worker)}`);
      if (l.asked.cdn.length) console.log(`      cdn paths: ${JSON.stringify(l.asked.cdn)}`);
    }
  } finally {
    await browser.close();
    await srv.close();
  }
  const failed = results.filter(r => !r.ok).length;
  console.log(failed ? `\nstorefront-r2: ${failed} failure(s)` : '\nstorefront-r2: everything passes');
  return failed;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  process.exit(await run());
}
