// WHAT ONE ORDER COSTS, MEASURED RATHER THAN ESTIMATED.
//
// The hub-cost blueprint established a chain of numbers — 28 KB per delivered
// order, then 16.6, then 3.8, then ≈1.2 — and every one of them was computed
// from the format rather than observed on a wire. This counts what three real
// browsers and one real order actually send and receive, and what the hub's own
// gauges say afterwards.
//
// FOUR BUDGETS, and each is a number a bill is made of:
//   requests   — Cloudflare bills Workers per request, so this is the line item
//   bytes      — what a customer on a phone plan pays for
//   latency    — p50/p95 per route, from the browser's own timing
//   cells      — the venue's image growth, which is the D1 row that gets
//                rewritten and read on every poll
//
// IT MEASURES A FULL DELIVERY, placement to DELIVERED, because a partial order
// understates the poll traffic that dominates the bill: polling was 65 % of
// every request this platform served before the sockets landed.
import { chromium, devices } from 'playwright';
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const OUT = process.env.OUT || '/tmp/dowiz-evals';
fs.mkdirSync(OUT, { recursive: true });
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const j = async (p, o = {}) => {
  const r = await fetch(`${HOST}${p}`, o);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};
const slug = new URL(HOST).hostname.split('.')[0];
const login = await j('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }) });
const OT = login.body?.access_token;
const VENUE = (await j(`/api/public/locations/${slug}/menu`)).body?.location?.id;
const own = (path, body, method = 'POST') => j(path, { method,
  headers: { authorization: `Bearer ${OT}`, 'content-type': 'application/json' },
  ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
const cl = await j('/api/courier/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ phone: creds.QA_COURIER_PHONE, password: creds.QA_COURIER_PASSWORD }) });
const CT = cl.body?.jwt;
if (!OT || !CT || !VENUE) { console.log('cannot sign in'); process.exit(1); }

/// Every request a page makes, with its bytes and its wall time.
const meter = (page, who, into) => {
  const started = new Map();
  page.on('request', r => started.set(r, Date.now()));
  page.on('response', async r => {
    const req = r.request();
    const at = started.get(req) ?? Date.now();
    // ENCODED BYTES, WHICH IS WHAT A PHONE PLAN PAYS FOR.
    //
    // The first version read `content-length` and fell back to the decoded
    // body. A compressed response usually carries no `content-length`, so it
    // fell back every time and reported the DECOMPRESSED size: the menu read
    // 86 KB when the wire carried 12. `request().sizes()` gives the transfer
    // sizes the browser actually saw, headers included.
    let bytes = 0;
    let decoded = 0;
    try {
      const sz = await req.sizes();
      bytes = (sz.responseBodySize || 0) + (sz.responseHeadersSize || 0)
            + (sz.requestBodySize || 0) + (sz.requestHeadersSize || 0);
    } catch {}
    try { decoded = (await r.body()).length; } catch {}
    if (!bytes) bytes = decoded;
    const u = new URL(r.url());
    into.push({
      who,
      // Routes are grouped by SHAPE, not by URL: `/api/order/<uuid>` is one
      // route with many ids, and counting them separately hides the poll.
      route: u.pathname.replace(/\/[0-9a-f]{8}-[0-9a-f-]{27,}/gi, '/:id')
                       .replace(/\/[0-9a-f]{32,}/gi, '/:hash'),
      api: u.pathname.startsWith('/api/'),
      status: r.status(),
      bytes,
      decoded,
      ms: Date.now() - at,
      socket: false,
    });
  });
  page.on('websocket', ws => {
    into.push({ who, route: '/api/live (socket open)', api: true, status: 101, bytes: 0, ms: 0, socket: true });
    ws.on('framereceived', f => into.push({ who, route: '/api/live (frame in)', api: true, status: 0,
      bytes: typeof f.payload === 'string' ? f.payload.length : (f.payload?.length || 0), ms: 0, socket: true }));
    ws.on('framesent', f => into.push({ who, route: '/api/live (frame out)', api: true, status: 0,
      bytes: typeof f.payload === 'string' ? f.payload.length : (f.payload?.length || 0), ms: 0, socket: true }));
  });
};

const gauges = async () => {
  const h = await own('/api/owner/health', undefined, 'GET');
  return h.body?.images || {};
};
const before = await gauges();

const log = [];
const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const tap = async (p, sel) => {
  const el = typeof sel === 'string' ? await p.$(sel) : sel;
  if (!el) return false;
  try { await el.scrollIntoViewIfNeeded({ timeout: 3000 }).catch(() => {}); await el.click({ timeout: 8000 }); return true; }
  catch { try { await el.dispatchEvent('click'); return true; } catch { return false; } }
};

// ── the customer ────────────────────────────────────────────────────────────
const cust = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
const c = await cust.newPage(); meter(c, 'customer', log);
let placed = null;
c.on('response', async r => {
  if (/\/orders$/.test(r.url()) && r.request().method() === 'POST' && r.status() < 400) {
    try { placed = await r.json(); } catch {}
  }
});
const t0 = Date.now();
await c.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
const domReady = Date.now() - t0;
await c.waitForTimeout(6000);
const firstPaint = await c.evaluate(() => {
  const p = performance.getEntriesByType('paint').find(x => x.name === 'first-contentful-paint');
  const nav = performance.getEntriesByType('navigation')[0];
  return { fcp: p ? Math.round(p.startTime) : null, load: nav ? Math.round(nav.loadEventEnd) : null,
           transfer: nav ? nav.transferSize : null };
});
for (let i = 0; i < 4; i++) { const l = await c.$('#insLater'); if (!l) break; await tap(c, l); await c.waitForTimeout(400); }
const menuDone = log.filter(x => x.route.includes('/menu')).reduce((n, x) => n + x.bytes, 0);

const dish = await c.$('[data-add]');
if (dish) { await tap(c, dish); await c.waitForTimeout(1200); }
await tap(c, '#cartPill'); await c.waitForTimeout(1400);
await tap(c, '#toCheckout'); await c.waitForTimeout(2500);
for (const [sel, val] of [['#f-name', 'QA eval'], ['#f-phone', '+355690000009'],
                          ['#f-street', 'Rruga Taulantia'], ['#f-house', '12'],
                          ['#f-note', 'QA — не готувати, це перевірка']]) {
  const el = await c.$(sel); if (el) await el.fill(val).catch(() => {});
}
await tap(c, '#place, #f-place'); await c.waitForTimeout(6000);
const ORDER = placed?.id;
if (!ORDER) { console.log('no order placed — cannot measure'); await b.close(); process.exit(1); }

// ── the console ─────────────────────────────────────────────────────────────
const oc = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
const o = await oc.newPage(); meter(o, 'console', log);
await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await o.waitForSelector('#e', { timeout: 40000 });
await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD);
await tap(o, '#go'); await o.waitForTimeout(6000);

// ── the courier ─────────────────────────────────────────────────────────────
const kc = await b.newContext({ ...devices['Pixel 7'], serviceWorkers: 'block',
  permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.4450 } });
const k = await kc.newPage(); meter(k, 'courier', log);
k.on('dialog', d => d.accept());
await k.goto(`${HOST}/courier/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await k.waitForSelector('#em', { timeout: 40000 });
await k.fill('#em', creds.QA_COURIER_PHONE); await k.fill('#pw', creds.QA_COURIER_PASSWORD);
await tap(k, '#go'); await k.waitForTimeout(4500);
const sh = await k.$('#openShift'); if (sh) { await tap(k, sh); await k.waitForTimeout(3000); }

// ── the order is walked, with all three watching ────────────────────────────
const marks = {};
const at = (label) => { marks[label] = Date.now(); };
at('placed');
for (const [act, label] of [['confirm', 'confirmed'], ['preparing', 'preparing'], ['ready', 'ready']]) {
  await own(`/api/owner/orders/${ORDER}/action`, { action: act, location_id: VENUE });
  at(label);
  await o.waitForTimeout(4000);
}
const card = await k.waitForSelector(`[data-sel="${ORDER}"], #takeOffer`, { timeout: 45000 }).catch(() => null);
if (card) { await tap(k, card); await k.waitForTimeout(700); }
const take = await k.$('#take') || await k.$('#takeOffer');
if (take) { await tap(k, take); await k.waitForTimeout(3000); }
const pick = await k.$('#pick'); if (pick) { await tap(k, pick); await k.waitForTimeout(3000); }
at('in_delivery');
// A scooter moving: four fixes, which is what a short run looks like.
for (const [lat, lng] of [[41.3200, 19.4460], [41.3180, 19.4475], [41.3160, 19.4490], [41.3140, 19.4505]]) {
  await kc.setGeolocation({ latitude: lat, longitude: lng });
  await k.waitForTimeout(2500);
}
const dn = await k.$('#done'); if (dn) { await tap(k, dn); await k.waitForTimeout(1800); }
const gt = await k.$('#got'); if (gt) { await tap(k, '#confirm'); await k.waitForTimeout(3500); }
at('delivered');
await c.waitForTimeout(6000);   // let every surface settle so the poll is counted
await b.close();

// ── the report ──────────────────────────────────────────────────────────────
const after = await gauges();
const api = log.filter(x => x.api);
const sum = (rows, f) => rows.reduce((n, x) => n + (f(x) || 0), 0);
const pct = (rows, p) => {
  const v = rows.map(x => x.ms).filter(n => n > 0).sort((a, b) => a - b);
  return v.length ? v[Math.min(v.length - 1, Math.floor(v.length * p))] : 0;
};

const byRoute = new Map();
for (const r of api) {
  const k2 = r.route;
  const e = byRoute.get(k2) || { n: 0, bytes: 0, decoded: 0, ms: [] };
  e.n += 1; e.bytes += r.bytes; e.decoded += r.decoded || 0; if (r.ms > 0) e.ms.push(r.ms);
  byRoute.set(k2, e);
}
const rows = [...byRoute.entries()].sort((a, b) => b[1].n - a[1].n);

const report = {
  order: ORDER,
  at: new Date().toISOString(),
  requests: { total: log.length, api: api.length, byWho: {
    customer: log.filter(x => x.who === 'customer').length,
    console: log.filter(x => x.who === 'console').length,
    courier: log.filter(x => x.who === 'courier').length,
  } },
  bytes: { total: sum(log, x => x.bytes), api: sum(api, x => x.bytes),
           apiDecoded: sum(api, x => x.decoded),
           menu: menuDone, sockets: sum(log.filter(x => x.socket), x => x.bytes) },
  latency: { p50: pct(api, 0.5), p95: pct(api, 0.95), worst: Math.max(0, ...api.map(x => x.ms)) },
  firstPaint: { ...firstPaint, domReady },
  lifecycleMs: Object.fromEntries(Object.entries(marks).map(([k2, v], i, a) =>
    [k2, i === 0 ? 0 : v - a[0][1]])),
  image: { before: before.log, after: after.log,
           cellsPerOrder: (after.log?.usedCells ?? 0) - (before.log?.usedCells ?? 0) },
  topRoutes: rows.slice(0, 14).map(([route, e]) => ({
    route, n: e.n, bytes: e.bytes, decoded: e.decoded,
    p50: e.ms.length ? e.ms.sort((x, y) => x - y)[Math.floor(e.ms.length / 2)] : 0,
  })),
};

const pad = (s, n) => String(s).padEnd(n);
console.log(`\n══ ONE DELIVERED ORDER, MEASURED ══ ${report.at}\n`);
console.log(`requests   ${report.requests.total} total, ${report.requests.api} to the API`);
console.log(`           customer ${report.requests.byWho.customer} · console ${report.requests.byWho.console} · courier ${report.requests.byWho.courier}`);
console.log(`bytes      ${(report.bytes.total / 1024).toFixed(1)} KB on the wire · ${(report.bytes.api / 1024).toFixed(1)} KB API (${(report.bytes.apiDecoded / 1024).toFixed(1)} KB decoded) · ${report.bytes.sockets} B sockets`);
console.log(`latency    p50 ${report.latency.p50} ms · p95 ${report.latency.p95} ms · worst ${report.latency.worst} ms`);
console.log(`first paint ${report.firstPaint.fcp} ms · load ${report.firstPaint.load} ms · transfer ${report.firstPaint.transfer} B`);
console.log(`image      ${report.image.before?.usedCells} -> ${report.image.after?.usedCells} cells (+${report.image.cellsPerOrder} for this order)`);
console.log(`\n${pad('route', 44)}${pad('n', 4)}${pad('wire', 9)}${pad('decoded', 10)}p50`);
for (const r of report.topRoutes) console.log(`${pad(r.route, 44)}${pad(r.n, 4)}${pad(r.bytes, 9)}${pad(r.decoded || 0, 10)}${r.p50} ms`);

const file = `${OUT}/traffic-${Date.now()}.json`;
fs.writeFileSync(file, JSON.stringify(report, null, 1));
console.log(`\nfull report: ${file}`);

// ── budgets ─────────────────────────────────────────────────────────────────
// A measurement that cannot fail is a dashboard, not a gate. These are set from
// the first run and are meant to be tightened, never loosened without a note.
const BUDGET = {
  apiRequests: Number(process.env.BUDGET_REQUESTS || 120),
  apiKB: Number(process.env.BUDGET_KB || 400),
  p95: Number(process.env.BUDGET_P95 || 1500),
  cells: Number(process.env.BUDGET_CELLS || 60),
};
const over = [];
if (report.requests.api > BUDGET.apiRequests) over.push(`API requests ${report.requests.api} > ${BUDGET.apiRequests}`);
if (report.bytes.api / 1024 > BUDGET.apiKB) over.push(`API bytes ${(report.bytes.api / 1024).toFixed(1)} KB > ${BUDGET.apiKB} KB`);
if (report.latency.p95 > BUDGET.p95) over.push(`p95 ${report.latency.p95} ms > ${BUDGET.p95} ms`);
if (report.image.cellsPerOrder > BUDGET.cells) over.push(`image grew ${report.image.cellsPerOrder} cells > ${BUDGET.cells}`);
console.log(`\n${over.length ? 'OVER BUDGET:\n  ' + over.join('\n  ') : 'WITHIN BUDGET'}`);
process.exit(over.length ? 1 : 0);
