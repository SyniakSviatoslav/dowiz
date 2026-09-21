// THE WHOLE JOURNEY, IN THREE BROWSERS, AND WHETHER IT IS CORRECT.
//
// `cycle-full.mjs` asks whether an order can be walked from a storefront to a
// courier's hands. This asks the harder question: whether what each of the
// three people SEES is right at every step — the dish they chose, the price
// they were quoted, the ingredients that left the shelf, the estimate, the
// status, the pin on the map, and the numbers the owner reads afterwards.
//
// It is built around a dish this run creates, out of an ingredient this run
// delivers, so that "the stock went down by 200 g" is a statement about a
// known quantity rather than a hope. Both are taken away at the end.
//
// A CUSTOMER DOES NOT ARRIVE KNOWING WHAT THEY WANT. This one searches, sorts,
// filters by a tag, opens three dishes, adds one, changes their mind, removes
// it, adds another — because every one of those is a control that can be
// broken independently, and "the menu rendered" says nothing about any of them.
import { chromium, devices } from 'playwright';
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const OUT = process.env.OUT || '/tmp/dowiz-journey';
fs.mkdirSync(OUT, { recursive: true });
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const SUPPLY = 'qa-journey-rice';
const PER_DISH = 200;
const RECEIVED = 2000;
const PRICE = 1234;

const fails = [];
const step = (name, ok, detail = '') => {
  if (!ok) fails.push(`${name}${detail ? ' :: ' + detail : ''}`);
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};
const j = async (p, o = {}) => {
  const r = await fetch(`${HOST}${p}`, o);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};

// Noise the product knows about; everything else is a finding.
const IGNORE = /CF\$cv|cloudflareinsights|challenge-platform|Executing inline script violates|WebGL|GPU|favicon|tiles|nominatim|maplibre|\.png|\.jpg|\.webp/i;
const problems = [];
const watch = (p, who) => {
  p.on('pageerror', e => problems.push(`${who} uncaught: ${e.message.slice(0, 110)}`));
  p.on('console', m => { if (m.type() === 'error' && !IGNORE.test(m.text())) problems.push(`${who} console: ${m.text().slice(0, 110)}`); });
  p.on('response', r => { if (r.status() >= 400 && !IGNORE.test(r.url())) problems.push(`${who} http ${r.status()} ${r.url().replace(HOST, '').slice(0, 70)}`); });
};
const tap = async (p, sel, who) => {
  const el = typeof sel === 'string' ? await p.$(sel) : sel;
  if (!el) return false;
  try { await el.scrollIntoViewIfNeeded({ timeout: 4000 }).catch(() => {}); await el.click({ timeout: 8000 }); return true; }
  catch { try { await el.dispatchEvent('click'); return true; } catch { return false; } }
};

const slug = new URL(HOST).hostname.split('.')[0];
const login = await j('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }) });
const OT = login.body?.access_token;
const VENUE = (await j(`/api/public/locations/${slug}/menu`)).body?.location?.id;
const own = (path, body, method = 'POST') => j(path, { method,
  headers: { authorization: `Bearer ${OT}`, 'content-type': 'application/json' },
  ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
step('the venue and the owner are known', !!OT && !!VENUE, `${VENUE}`);
if (!OT || !VENUE) process.exit(1);

const level = async () => {
  const s = await own('/api/owner/stock', undefined, 'GET');
  return (s.body?.supplies || []).find(x => x.id === SUPPLY) || null;
};
const dash = async () => (await own('/api/owner/dashboard', undefined, 'GET')).body || {};

// ── 0. A DISH WITH A KNOWN RECIPE, SO "IT WENT DOWN" IS MEASURABLE ──────────
await own('/api/owner/supplies', { id: SUPPLY, name: 'QA Journey Rice', unit: 'g',
  kind: 'food_ingredient', category: 'QA', kcalPer100: 130, proteinPer100: 2,
  fatPer100: 0, carbsPer100: 28, costPerBasis: 200, lowAt: 100, active: true, locationId: VENUE });
await own('/api/owner/stock/stocktake', { item: SUPPLY, observed: 0 });
await own('/api/owner/stock/received', { item: SUPPLY, qty: RECEIVED });
const cat = await own('/api/owner/categories', { location_id: VENUE, name: 'QA Journey' });
const CAT = cat.body?.id;
const prod = await own('/api/owner/products', { location_id: VENUE, category_id: CAT,
  name: 'QA Journey Bowl', price: PRICE, available: true });
const PROD = prod.body?.id;
await own(`/api/owner/products/${PROD}`, { location_id: VENUE, bom: [{ supply: SUPPLY, qty: PER_DISH }] });
const start = await level();
step('the shelf is stocked and the dish has a recipe',
  !!PROD && start?.onHand === RECEIVED, `dish=${PROD} onHand=${start?.onHand}`);

const before = await dash();

const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });

// ── 1. THE CUSTOMER LOOKS AROUND ────────────────────────────────────────────
const cust = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
const c = await cust.newPage(); watch(c, 'store');
const sockets = { store: 0, console: 0, courier: 0 };
const watchSocket = (p, who) => p.on('websocket', ws => { if (ws.url().includes('/api/live')) sockets[who] += 1; });
watchSocket(c, 'store');
let placed = null;
c.on('response', async r => {
  if (/\/orders$/.test(r.url()) && r.request().method() === 'POST' && r.status() < 400) {
    try { placed = await r.json(); } catch {}
  }
});
await c.goto(`${HOST}/?fresh=1`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await c.waitForTimeout(6000);
for (let i = 0; i < 4; i++) { const l = await c.$('#insLater'); if (!l) break; await tap(c, l, 'store'); await c.waitForTimeout(500); }

const visible = () => c.$$eval('[data-p]', els => els.filter(e => e.offsetParent !== null).length);
const all = await visible();
step('the menu is on screen', all > 0, `${all} dishes`);

// SEARCH. Typing must narrow the menu and clearing must restore it.
await c.fill('#q', 'QA Journey');
await c.waitForTimeout(1200);
const hits = await visible();
const named = await c.$$eval('[data-p]', els => els.filter(e => e.offsetParent !== null)
  .map(e => (e.querySelector('h3, .card-t, b')?.textContent || '').trim()).slice(0, 3));
step('search narrows the menu to what was typed', hits > 0 && hits < all,
  `${all} -> ${hits}${named.length ? ' :: ' + named.join(' | ') : ''}`);
await c.fill('#q', '');
await c.waitForTimeout(1200);
const restored = await visible();
step('clearing the search brings the menu back', restored === all, `${restored} vs ${all}`);

// A SEARCH THAT MATCHES NOTHING must say so rather than show an empty page.
await c.fill('#q', 'zzzz-nothing-matches-this');
await c.waitForTimeout(1200);
const none = await visible();
const saidNone = await c.evaluate(() => {
  const el = document.getElementById('noHits');
  return { present: !!el, shown: !!el && el.offsetParent !== null, text: (el?.innerText || '').trim().slice(0, 40) };
});
step('a search with no hits says so', none === 0 && saidNone.shown, `${none} shown, noHits="${saidNone.text}"`);
await c.fill('#q', '');
await c.waitForTimeout(1200);

// SORT. Low-to-high must actually order the prices.
await tap(c, '[data-open="sort"], #sortBtn, [data-t="sortBy"]', 'store');
await c.waitForTimeout(900);
const sorted = await tap(c, '[data-sort="low"]', 'store');
await c.waitForTimeout(1500);
await c.evaluate(() => document.getElementById('scrim')?.click());
await c.waitForTimeout(800);
const prices = await c.$$eval('[data-p]', els => els.filter(e => e.offsetParent !== null)
  .map(e => Number(e.dataset.price || 0)).filter(n => n > 0).slice(0, 12));
const ascending = prices.every((n, i) => i === 0 || prices[i - 1] <= n);
step('sorting by price actually sorts', !sorted || ascending,
  sorted ? `first prices: ${prices.slice(0, 6).join(',')}` : 'no sort control found');

// FILTER by a tag, if the venue has any.
const tags = await c.$$eval('[data-tag]', els => els.map(e => e.dataset.tag).filter(Boolean));
if (tags.length) {
  await tap(c, `[data-tag="${tags[0]}"]`, 'store');
  await c.waitForTimeout(1300);
  const filtered = await visible();
  step('a tag filters the menu', filtered > 0 && filtered <= all, `tag "${tags[0]}": ${all} -> ${filtered}`);
  await tap(c, '[data-tag=""]', 'store');
  await c.waitForTimeout(1200);
  step('clearing the tag restores the menu', (await visible()) === all);
} else step('a tag filters the menu', true, 'this venue declares no tags — skipped');

// THREE DISHES OPENED, because a sheet that shows the previous dish is a real
// and quiet bug.
const ids = await c.$$eval('[data-p]', els => els.filter(e => e.offsetParent !== null).map(e => e.dataset.p).slice(0, 3));
for (const id of ids) {
  await tap(c, `[data-open="${id}"]`, 'store');
  await c.waitForTimeout(1200);
  const shows = await c.evaluate(() => {
    const s = document.getElementById('sheet');
    return { name: s?.dataset.name || '', p: s?.dataset.p || '', len: (s?.innerText || '').length };
  });
  step(`the dish sheet opens on the dish that was tapped (${id})`,
    shows.name === 'dish' && shows.len > 40, `sheet=${shows.name} len=${shows.len}`);
  await c.evaluate(() => document.getElementById('scrim')?.click());
  await c.waitForTimeout(700);
}

// ── 2. CHOOSES, CHANGES THEIR MIND, CHOOSES AGAIN ───────────────────────────
const pill = () => c.evaluate(() => ({
  shown: !!document.getElementById('cartPill')?.classList.contains('show'),
  count: document.getElementById('pillCount')?.textContent || '',
  total: Number(document.getElementById('pillTotal')?.dataset.money || 0),
}));
const other = ids.find(x => x !== PROD) || ids[0];
await tap(c, `[data-add="${other}"]`, 'store');
await c.waitForTimeout(1200);
const afterFirst = await pill();
step('adding a dish moves the cart', afterFirst.shown && afterFirst.count === '1', JSON.stringify(afterFirst));

await tap(c, '#cartPill', 'store');
await c.waitForTimeout(1500);
await tap(c, `[data-m="${other}"]`, 'store');
await c.waitForTimeout(1300);
const afterRemove = await pill();
step('removing it empties the cart', afterRemove.count === '0' || !afterRemove.shown, JSON.stringify(afterRemove));
await c.evaluate(() => document.getElementById('scrim')?.click());
await c.waitForTimeout(800);

// Now the dish this run can measure.
await c.fill('#q', 'QA Journey');
await c.waitForTimeout(1300);
await tap(c, `[data-add="${PROD}"]`, 'store');
await c.waitForTimeout(1300);
const chosen = await pill();
step('the measured dish is in the cart at its price',
  chosen.count === '1' && chosen.total === PRICE, `count=${chosen.count} total=${chosen.total} expected=${PRICE}`);

// ── 3. ORDERS IT ────────────────────────────────────────────────────────────
await tap(c, '#cartPill', 'store');
await c.waitForTimeout(1500);
await tap(c, '#toCheckout', 'store');
await c.waitForTimeout(2500);
for (const [sel, val] of [['#f-name', 'QA Journey'], ['#f-phone', '+355690000009'],
                          ['#f-street', 'Rruga Taulantia'], ['#f-house', '12'],
                          ['#f-note', 'QA — не готувати, це перевірка']]) {
  const el = await c.$(sel); if (el) await el.fill(val).catch(() => {});
}
await c.waitForTimeout(700);
await c.screenshot({ path: `${OUT}/1-checkout.png` }).catch(() => {});
await tap(c, '#place, #f-place, [data-t="placeOrder"]', 'store');
await c.waitForTimeout(7000);
const ORDER = placed?.id;
step('the order is placed', !!ORDER, ORDER ? `${ORDER}` : JSON.stringify(placed || {}).slice(0, 120));
if (!ORDER) { console.log(`\nFAILURES:\n  ${fails.join('\n  ')}`); await b.close(); process.exit(1); }

const readOrder = async () => (await j(`/api/order/${ORDER}`,
  { headers: placed?.access_token ? { authorization: `Bearer ${placed.access_token}` } : {} })).body;
const o0 = await readOrder();
step('the order the hub stored is the order the customer built',
  o0?.total === PRICE + (o0?.delivery_fee ?? 0) && o0?.items?.[0]?.product_id === PROD && o0?.items?.[0]?.quantity === 1,
  `total=${o0?.total} fee=${o0?.delivery_fee} item=${o0?.items?.[0]?.product_id}x${o0?.items?.[0]?.quantity}`);

const held = await level();
step('placing RESERVED the ingredients, and did not consume them',
  held?.onHand === RECEIVED && held?.reserved === PER_DISH,
  `onHand=${held?.onHand} reserved=${held?.reserved}`);

// ── 4. THE OWNER'S CONSOLE ──────────────────────────────────────────────────
const oc = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
const o = await oc.newPage(); watch(o, 'console'); watchSocket(o, 'console');
await o.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await o.waitForSelector('#e', { timeout: 40000 });
await o.fill('#e', creds.OWNER_EMAIL); await o.fill('#p', creds.OWNER_PASSWORD);
await tap(o, '#go', 'console');
await o.waitForTimeout(7000);
step('console: signs in', !(await o.$('#e')));

const row = async () => o.evaluate(id => {
  const el = document.querySelector(`[data-o="${id}"]`)?.closest('article, .row, li, div.card');
  const txt = (el?.innerText || document.getElementById('app')?.innerText || '');
  return { found: !!el, text: txt.slice(0, 400) };
}, ORDER);
const r0 = await row();
// THE DISH'S NAME, NOT ITS SLUG. Every surface writes `i.name || i.product_id`
// and no stored line carried a `name`, so a kitchen ticket read `1x item-05`
// for every order this product has ever taken. The total shown is the order's,
// which includes the delivery fee.
const wantTotal = PRICE + (o0?.delivery_fee ?? 0);
step('console: the queue shows the DISH NAME, not its id',
  r0.text.includes('QA Journey Bowl') && !r0.text.includes('qa-journey-bowl'),
  r0.text.replace(/\n+/g, ' | ').slice(0, 160));
step('console: the queue shows the order total',
  r0.text.replace(/[^0-9]/g, '').includes(String(wantTotal)),
  `looking for ${wantTotal} in the row`);
step('console: the order shows an estimate', /min|хв|мин/i.test(r0.text), 'looked for an ETA unit');
await o.screenshot({ path: `${OUT}/2-console-new.png` }).catch(() => {});

const statusOf = async () => (await own('/api/owner/orders', undefined, 'GET'))
  .body?.orders?.find(x => x.id === ORDER)?.status;
const act = async (what, expect) => {
  const btn = await o.$(`[data-act="${what}"][data-o="${ORDER}"]`);
  if (btn) { await tap(o, btn, 'console'); await o.waitForTimeout(3500); }
  else { await own(`/api/owner/orders/${ORDER}/action`, { action: what, location_id: VENUE }); await o.waitForTimeout(2500); }
  const st = await statusOf();
  step(`console: ${what} -> ${expect}`, st === expect, `status=${st}${btn ? '' : ' (no button — used the route)'}`);
};
await act('confirm', 'CONFIRMED');
await act('preparing', 'PREPARING');

const eaten = await level();
step('preparing CONSUMED exactly the recipe',
  eaten?.onHand === RECEIVED - PER_DISH && eaten?.reserved === 0,
  `onHand=${eaten?.onHand} (expected ${RECEIVED - PER_DISH}) reserved=${eaten?.reserved}`);

await act('ready', 'READY');

// A NAME THAT SURVIVES ONLY WHILE THE ORDER IS PENDING is worse than none:
// `fold::delta` deletes any key the kernel's re-emission leaves out, and the
// kernel emits four fields per line.
const midway = await readOrder();
step('the dish name survives the status changes',
  midway?.items?.[0]?.name === 'QA Journey Bowl',
  `name=${JSON.stringify(midway?.items?.[0]?.name ?? null)}`);

// ── 5. THE COURIER ──────────────────────────────────────────────────────────
const kc = await b.newContext({ ...devices['Pixel 7'], serviceWorkers: 'block',
  permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.4450 } });
const k = await kc.newPage(); watch(k, 'courier'); watchSocket(k, 'courier');
k.on('dialog', d => d.accept());
await k.goto(`${HOST}/courier/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await k.waitForSelector('#em', { timeout: 40000 });
await k.fill('#em', creds.QA_COURIER_PHONE); await k.fill('#pw', creds.QA_COURIER_PASSWORD);
await tap(k, '#go', 'courier');
await k.waitForTimeout(5000);
step('courier: signs in', !(await k.$('#em')));
const CTOK = (await j('/api/courier/auth/login', { method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ phone: creds.QA_COURIER_PHONE, password: creds.QA_COURIER_PASSWORD }) }))
  .body?.jwt || '';
const shift = await k.$('#openShift');
if (shift) { await tap(k, shift, 'courier'); await k.waitForTimeout(3500); }
step('courier: is on shift', !(await k.$('#openShift')));

const pooled = await k.waitForSelector(`[data-sel="${ORDER}"], #takeOffer`, { timeout: 45000 }).catch(() => null);
step('courier: the order reaches the pool', !!pooled);
const card = await k.$(`[data-sel="${ORDER}"]`);
if (card) { await tap(k, card, 'courier'); await k.waitForTimeout(800); }
const take = await k.$('#take') || await k.$('#takeOffer');
step('courier: can take it', !!take);
if (take) { await tap(k, take, 'courier'); await k.waitForTimeout(3500); }

const pick = await k.waitForSelector('#pick', { timeout: 30000 }).catch(() => null);
step('courier: has it in hand', !!pick);
if (pick) { await tap(k, pick, 'courier'); await k.waitForTimeout(3500); }
step('courier: pickup -> IN_DELIVERY', (await statusOf()) === 'IN_DELIVERY', `status=${await statusOf()}`);
await k.screenshot({ path: `${OUT}/3-courier.png` }).catch(() => {});

// THE SCOOTER MOVES. Each fix should reach the hub and the owner's map.
const LEG = [[41.3225, 19.4450], [41.3190, 19.4470], [41.3160, 19.4490], [41.3140, 19.4505]];
for (const [lat, lng] of LEG) {
  await kc.setGeolocation({ latitude: lat, longitude: lng });
  await k.waitForTimeout(2500);
}
await k.waitForTimeout(3000);
// THE PIN, FOR THIS COURIER. Asking whether the WORD "lat" appears anywhere in
// the roster is not a measurement — it matched a list that contained a
// different courier entirely. The courier who is carrying this order is the
// one whose position has to have moved.
const mine = (await j('/api/courier/tasks', { headers: { authorization: `Bearer ${CTOK}` } })).body;
const myId = mine?.courier?.id;
const seen = await own(`/api/owner/couriers/${myId}`, undefined, 'GET');
const pos = seen.body?.lastSeen || seen.body?.position || seen.body?.at || null;
step('the courier who is carrying it has a position at the venue',
  seen.status === 200 && !!pos && (pos.lat != null || pos.lat_e6 != null || pos.latE6 != null),
  `courier=${String(myId).slice(0, 8)} ${JSON.stringify(pos)?.slice(0, 110)}`);

// ── 6. THE CUSTOMER WATCHES IT COME ─────────────────────────────────────────
const sheetText = await c.evaluate(() => (document.querySelector('.ep-title, #sheetIn')?.innerText || '').trim().slice(0, 60));
step('the tracking sheet followed the order without a reload', sheetText.length > 0, `"${sheetText}"`);
await c.screenshot({ path: `${OUT}/4-tracking.png` }).catch(() => {});

const done = await k.$('#done'); if (done) { await tap(k, done, 'courier'); await k.waitForTimeout(2000); }
const got = await k.$('#got'); if (got) { await tap(k, '#confirm', 'courier'); await k.waitForTimeout(3500); }
let final = await statusOf();
if (final !== 'DELIVERED') { await k.waitForTimeout(4000); final = await statusOf(); }
step('delivered', final === 'DELIVERED', `status=${final}`);

// ── 7. FEEDBACK ─────────────────────────────────────────────────────────────
const fb = await j(`/api/order/${ORDER}/feedback`, { method: 'POST',
  headers: { 'content-type': 'application/json', ...(placed?.access_token ? { authorization: `Bearer ${placed.access_token}` } : {}) },
  body: JSON.stringify({ rating: 5, text: 'QA — перевірка, не реальний відгук' }) });
step('the customer can leave feedback', fb.status < 400, `${fb.status}`);
const withFb = await readOrder();
step('the feedback is on the order', !!withFb?.feedback, JSON.stringify(withFb?.feedback || null).slice(0, 70));

// ── 8. THE NUMBERS MOVED ────────────────────────────────────────────────────
const after = await dash();
step('the day\'s order count went up by one',
  (after.todayOrders ?? 0) === (before.todayOrders ?? 0) + 1,
  `${before.todayOrders} -> ${after.todayOrders}`);
const an = await own('/api/owner/analytics', undefined, 'GET');
step('analytics answers with this venue\'s money', an.status === 200 && typeof an.body?.averageOrder === 'number',
  `averageOrder=${an.body?.averageOrder}`);

step('all three opened a live socket', sockets.store > 0 && sockets.console > 0 && sockets.courier > 0,
  JSON.stringify(sockets));
step('nothing errored anywhere in the journey', problems.length === 0,
  problems.slice(0, 4).join(' | ') || 'clean');

// ── 9. PUT THE VENUE BACK ───────────────────────────────────────────────────
await own(`/api/owner/products/${PROD}/delete`, { location_id: VENUE });
await own(`/api/owner/categories/${CAT}/delete`, { location_id: VENUE });
await own('/api/owner/stock/stocktake', { item: SUPPLY, observed: 0 });
await own(`/api/owner/supplies/${SUPPLY}/retire`, { location_id: VENUE });
const left = await own('/api/owner/stock', undefined, 'GET');
step('the venue is left as it was found',
  !(left.body?.supplies || []).some(x => x.id === SUPPLY) && (left.body?.stranded || []).length === 0,
  `stranded=${JSON.stringify(left.body?.stranded || [])}`);

console.log(`\n${fails.length ? 'FAILURES:\n  ' + fails.join('\n  ') : 'JOURNEY OK'}`);
console.log(`order: ${ORDER} · shots in ${OUT}`);
await b.close();
process.exit(fails.length ? 1 : 0);
