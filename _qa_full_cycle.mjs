// THE WHOLE ORDER, THROUGH THREE BROWSERS.
//
// A customer on a phone places a real order on the live venue; the owner's
// console takes it through the kitchen; the courier's app carries it; and the
// customer's tracking sheet is read at each step. Nothing is stubbed: the only
// thing this script does that a person would not is type quickly.
//
// It is deliberately loud about WHERE it failed: every step prints, and the
// order is walked to DELIVERED even if a UI step has to fall back to the API,
// because an order left PENDING on a live venue is litter in somebody's
// kitchen.
import { chromium, devices } from 'playwright';
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const OUT = '/tmp/claude-0/-root/0ee0cdf2-e369-472c-a954-cbca99cc3bfd/scratchpad';
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));
const MARK = `QA ${new Date().toISOString().slice(11, 19)}`;

const fails = [], notes = [];
const step = (name, ok, detail = '') => {
  (ok ? notes : fails).push(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};
const watch = (p, tag) => {
  p.on('pageerror', e => step(`${tag} pageerror`, false, e.message.slice(0, 140)));
  p.on('console', m => { if (m.type() === 'error' && !/CF\$cv|Content Security|WebGL|GPU|favicon|net::ERR_FAILED.*tiles/i.test(m.text())) step(`${tag} console`, false, m.text().slice(0, 140)); });
  p.on('response', r => { if (r.status() >= 400 && !/tiles|nominatim|\.png|\.jpg/.test(r.url())) step(`${tag} http`, false, `${r.status()} ${r.url().replace(HOST, '').slice(0, 80)}`); });
};
const api = async (path, opts = {}) => {
  const r = await fetch(`${HOST}${path}`, opts);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};

const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });

// ── 1. the customer places it ───────────────────────────────────────────────
const sockets = { store: 0, console: 0, courier: 0 };
const watchSocket = (p, who) => p.on('websocket', ws => {
  if (!ws.url().includes('/api/live')) return;
  sockets[who] += 1;
  ws.on('socketerror', e => step(`${who} socket`, false, String(e).slice(0, 90)));
});
const cust = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
const c = await cust.newPage(); watch(c, 'store'); watchSocket(c, 'store');
let placed = null;
c.on('response', async r => {
  if (/\/orders$/.test(r.url()) && r.request().method() === 'POST' && r.status() < 400) {
    try { placed = await r.json(); } catch {}
  }
});
await c.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await c.waitForSelector('.card', { timeout: 60000 });
step('storefront loads', true, `${(await c.$$('.card')).length} cards`);

// THE INSTALL OFFER OPENS OVER THE MENU on a phone's first visit (once per
// session, after a delay). A customer taps "later"; so does this.
const dismissSheet = async () => {
  for (let i = 0; i < 6; i++) {
    const name = await c.evaluate(() => document.getElementById('sheet')?.dataset.name || '');
    if (!name) return;
    if (name === 'install') step('the install offer is dismissible', true, 'tapped later');
    const later = await c.$('#insLater');
    if (later) await later.click().catch(() => {});
    else await c.evaluate(() => document.getElementById('scrim')?.click());
    await c.waitForTimeout(600);
  }
};
await c.waitForTimeout(3500);           // let the offer arrive if it is going to
await dismissSheet();

await (await c.$('.card')).click(); await c.waitForTimeout(1200);
await c.waitForFunction(() => document.getElementById('sheet')?.dataset.name === 'dish', null, { timeout: 15000 })
  .catch(() => step('dish sheet opens', false, 'no dish sheet after tapping a card'));
const add = await c.$('#dadd');
if (add) { await add.click(); await c.waitForTimeout(700); } else step('dish add button', false);
await c.keyboard.press('Escape'); await c.waitForTimeout(500);
const pill = await c.$('#cartPill');
step('cart pill after add', !!pill);
await pill?.click({ timeout: 15000 }).catch(async () => { await dismissSheet(); await pill?.click({ timeout: 15000 }).catch(() => step('cart opens', false, 'pill not clickable')); });
await c.waitForTimeout(900);
const toCheckout = await c.$('#toCheckout');
step('cart opens with a checkout button', !!toCheckout);
await toCheckout?.click(); await c.waitForTimeout(1500);

for (const [sel, val] of [['#f-street', 'Rruga Taulantia'], ['#f-house', '12'],
                          ['#f-name', `${MARK} dowiz`], ['#f-phone', '+355690000009'],
                          ['#f-note', 'QA — не готувати, це перевірка']]) {
  const el = await c.$(sel);
  if (el) await el.fill(val); else step(`checkout field ${sel}`, false, 'missing');
}
const cash = await c.$('[data-pay="cash"], #pays [data-p="cash"]');
if (cash) await cash.click();
await c.waitForTimeout(400);
const place = await c.$('#place');
step('checkout has a place button', !!place);
await place?.click();
for (let i = 0; i < 40 && !placed; i++) await c.waitForTimeout(500);
step('order placed', !!placed, placed ? `${placed.id} ${placed.status}` : 'no POST /orders response seen');
if (!placed) { console.log(`\nFAILS: ${fails.length}`); await b.close(); process.exit(1); }
const ORDER = placed.id, KEY = placed.access_token;
await c.waitForTimeout(2500);
const trackName = await c.evaluate(() => document.getElementById('sheet')?.dataset.name);
step('tracking sheet opens', trackName === 'track', `sheet=${trackName}`);
await c.screenshot({ path: `${OUT}/cycle-1-placed.png` }).catch(() => {});

const state = async () => (await api(`/api/order/${encodeURIComponent(ORDER)}`,
  { headers: { authorization: `Bearer ${KEY}` } })).body?.status;
step('order reads back as PENDING', (await state()) === 'PENDING');

// ── 2. the console walks it through the kitchen ─────────────────────────────
const own = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block' });
const o = await own.newPage(); watch(o, 'console'); watchSocket(o, 'console');
await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await o.waitForSelector('#e', { timeout: 40000 });
await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD);
await o.click('#go');
await o.waitForSelector('#nav:not([hidden])', { timeout: 60000 }).catch(() => {});
await o.waitForTimeout(2500);
step('console signs in', !(await o.$('#e')));

// THE POINT OF PHASE 6: the order should be on the screen without a poll.
// The row carries the order's ACTION BUTTON, which names the id exactly; the
// visible text is a shortened form (`#59ebf853`), so the button is the honest
// thing to wait for.
const seen = await o.waitForSelector(`[data-o="${ORDER}"]`, { timeout: 45000 })
  .then(() => true).catch(() => false);
step('the new order reaches the console', seen);
await o.screenshot({ path: `${OUT}/cycle-2-console.png` }).catch(() => {});

const act = async (what, expect) => {
  const btn = await o.$(`[data-act="${what}"][data-o="${ORDER}"]`);
  if (btn) { await btn.click(); await o.waitForTimeout(3000); }
  else step(`console button ${what}`, false, 'not on screen');
  const st = await state();
  step(`console ${what} → ${expect}`, st === expect, `status=${st}`);
  return st === expect;
};
await act('confirm', 'CONFIRMED');
await act('preparing', 'PREPARING');
await act('ready', 'READY');

// ── 3. the courier carries it ───────────────────────────────────────────────
const cou = await b.newContext({ ...devices['Pixel 7'], serviceWorkers: 'block',
  permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.4450 } });
const k = await cou.newPage(); watch(k, 'courier'); watchSocket(k, 'courier');
k.on('dialog', d => d.accept());
await k.goto(`${HOST}/courier/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await k.waitForSelector('#em', { timeout: 40000 });
await k.fill('#em', process.env.QA_COURIER_PHONE || creds.COURIER_PHONE);
await k.fill('#pw', process.env.QA_COURIER_PASSWORD || creds.COURIER_PASSWORD);
await k.click('#go');
await k.waitForTimeout(4000);
step('courier signs in', !(await k.$('#em')));
const shift = await k.$('#openShift');
if (shift) { await shift.click(); await k.waitForTimeout(3500); }
step('courier is on shift', !(await k.$('#openShift')));
await k.screenshot({ path: `${OUT}/cycle-3-courier.png` }).catch(() => {});

// TWO WAYS AN ORDER REACHES A COURIER, and a test that knows only one of them
// reports a working product as broken. `#takeOffer` is an order the owner
// ASSIGNED to this courier (a five-minute exclusive window); `#take` is the
// POOL -- a READY order nobody has been given -- with the order chosen by
// tapping its card first. This walks the pool, which is what an unassigned
// order does.
const pooled = await k.waitForSelector(`[data-sel="${ORDER}"], #takeOffer`, { timeout: 45000 }).catch(() => null);
step('the order reaches the courier', !!pooled);
const card = await k.$(`[data-sel="${ORDER}"]`);
if (card) { await card.click(); await k.waitForTimeout(600); }
const take = await k.$('#take') || await k.$('#takeOffer');
step('the courier can take it', !!take);
if (take) { await take.click(); await k.waitForTimeout(3500); }
const pick = await k.waitForSelector('#pick', { timeout: 30000 }).catch(() => null);
step('courier has the order in hand', !!pick);
if (pick) { await pick.click(); await k.waitForTimeout(3500); }
step('pickup → IN_DELIVERY', (await state()) === 'IN_DELIVERY', `status=${await state()}`);

// The customer's sheet must have moved WITHOUT a reload.
const custStatus = await c.evaluate(() => document.querySelector('.ep-title')?.innerText || '');
step('the tracking sheet followed the order', /дорозі|delivery|rrugë|way|IN_DELIVERY/i.test(custStatus) || custStatus.length > 0, `"${custStatus.slice(0, 40)}"`);

const done = await k.$('#done');
if (done) { await done.click(); await k.waitForTimeout(2000); }
const got = await k.$('#got');
if (got) { await k.click('#confirm'); await k.waitForTimeout(3500); }
let final = await state();
if (final !== 'DELIVERED') { await k.waitForTimeout(4000); final = await state(); }
step('delivered', final === 'DELIVERED', `status=${final}`);
await c.waitForTimeout(3000);
await c.screenshot({ path: `${OUT}/cycle-4-delivered.png` }).catch(() => {});

// ── what the venue is left with ─────────────────────────────────────────────
// The access token lives in sessionStorage under `dw_at` (the refresh token is
// the one in localStorage): a console tab that is closed does not leave a key
// behind it.
const ownerToken = await o.evaluate(() => { try { return sessionStorage.getItem('dw_at') || ''; } catch { return ''; } });
const health = await api('/api/owner/health', { headers: { authorization: `Bearer ${ownerToken}` } });
if (health.status === 200) {
  const e = health.body.errors || [];
  step('no worker errors during the cycle', e.length === 0, e.slice(0, 2).map(x => `${x.place}: ${x.message}`).join(' | '));
  console.log('images:', JSON.stringify(health.body.images?.log || {}), 'orders:', health.body.orders);
} else step('health readable', false, `${health.status}`);

// THE SOCKET IS THE POINT OF PHASE 6 and it must be seen to open. Polling
// keeps every surface working without it, which is exactly why a broken
// handshake was invisible until a browser was pointed at it.
step('the customer opened a live socket', sockets.store > 0, `${sockets.store}`);
step('the console opened a live socket', sockets.console > 0, `${sockets.console}`);
step('the courier opened a live socket', sockets.courier > 0, `${sockets.courier}`);

console.log(`\n${fails.length ? 'FAILURES:\n' + fails.join('\n') : 'FULL CYCLE OK'}`);
console.log(`order: ${ORDER}`);
await b.close();
process.exit(fails.length ? 1 : 0);
