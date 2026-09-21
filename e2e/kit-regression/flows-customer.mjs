// THE CUSTOMER'S CYCLES, walked in a live session, including the ones that
// touch money.
//
// The full-cycle test walks ONE shape of order: cash, delivery, no promo, no
// options, taken from the pool. Everything else the product offers has never
// been walked at all — a code applied in the cart, a collection order, an
// order the owner ASSIGNS rather than pools, and the feedback a customer
// leaves afterwards. Each of those is a different path through the same
// kernel, and three of them touch the total.
//
// EVERY ORDER IT PLACES IS CLOSED. Cancel only works from PENDING (see
// order_machine.rs `allowed_next`), so anything walked further is taken to
// DELIVERED with the venue's own courier. A run that cannot close what it
// opened says so.
import { chromium, devices } from 'playwright';
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const OUT = process.env.OUT || '/tmp/dowiz-flows';
fs.mkdirSync(OUT, { recursive: true });
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

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

const slug = new URL(HOST).hostname.split('.')[0];
const menu0 = await j(`/api/public/locations/${slug}/menu`);
const VENUE = menu0.body?.location?.id;
const DISH = (menu0.body?.categories || []).flatMap(c => c.products || [])
  .find(p => p.available && p.price > 0);

const ol = await j('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }) });
const OT = ol.body?.access_token;
const cl = await j('/api/courier/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ phone: creds.QA_COURIER_PHONE, password: creds.QA_COURIER_PASSWORD }) });
const CT = cl.body?.jwt || cl.body?.access_token;
step('the three sessions open', !!VENUE && !!OT && !!CT && !!DISH,
  `venue=${VENUE} dish=${DISH?.id}@${DISH?.price} owner=${ol.status} courier=${cl.status}`);
if (!VENUE || !OT || !CT || !DISH) process.exit(1);

const own = (p, body) => j(p, { method: 'POST',
  headers: { authorization: `Bearer ${OT}`, 'content-type': 'application/json' },
  body: JSON.stringify(body ?? {}) });
const cour = (p, body) => j(p, { method: 'POST',
  headers: { authorization: `Bearer ${CT}`, 'content-type': 'application/json' },
  body: JSON.stringify(body ?? {}) });
// AN ORDER IS READ WITH THE LINK THE CUSTOMER WAS GIVEN. `/api/order/:id`
// answers 401 "this order needs the link you were given" to anyone else, and
// the placement response carries that token — so a test that forgets it reads
// `undefined` and blames the product. The owner's list is the fallback.
const asCustomer = tok => tok ? { authorization: `Bearer ${tok}` } : {};
const readOrder = (id, tok) => j(`/api/order/${id}`, { headers: asCustomer(tok) });
const statusOf = async (id, tok) => (await readOrder(id, tok)).body?.status
  ?? (await j('/api/owner/orders', { headers: { authorization: `Bearer ${OT}` } }))
      .body?.orders?.find(o => o.id === id)?.status;

const place = (body) => j(`/api/public/locations/${slug}/orders`, {
  method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    contact: { name: 'QA flows', phone: '+355690000009' },
    fulfilment: { kind: 'delivery', address: { line: 'Rruga Taulantia 12', note: 'QA — не готувати, це перевірка' } },
    items: [{ product_id: DISH.id, quantity: 1 }],
    payment: 'cash',
    ...body,
  }),
});

/// Close an order whatever state it is in, and say whether it worked.
const close = async (id, label) => {
  if (!id) return;
  const already = await statusOf(id);
  if (['DELIVERED', 'CANCELLED', 'REJECTED', 'PICKED_UP'].includes(already)) {
    step(`${label}: the order was closed`, true, `status=${already}`);
    return;
  }
  let r = await own(`/api/owner/orders/${id}/action`, { action: 'cancel', location_id: VENUE });
  if (r.status !== 200) {
    for (const a of ['confirm', 'preparing', 'ready']) {
      await own(`/api/owner/orders/${id}/action`, { action: a, location_id: VENUE });
    }
    await cour(`/api/courier/orders/${id}/accept`);
    await cour(`/api/courier/orders/${id}/pickup`);
    r = await cour(`/api/courier/orders/${id}/deliver`, { cash_collected: true });
  }
  const st = await statusOf(id);
  step(`${label}: the order was closed`, ['DELIVERED', 'CANCELLED', 'REJECTED', 'PICKED_UP'].includes(st),
    `status=${st}${r.status >= 400 ? ` last=${r.status}` : ''}`);
};

const flow = async (name, fn) => {
  console.log(`\n── ${name} ───────────────────────────────`);
  try { await fn(); } catch (e) { step(`${name}: threw`, false, String(e.message).slice(0, 140)); }
};

await cour('/api/courier/shift', { open: true });

// ── 1. A PROMO CODE, AND WHAT THE CUSTOMER IS ACTUALLY ASKED TO PAY ─────────
//
// The discount is redeemed inside the CAS closure that writes the order, so
// the stored total and the total the payment rail is handed came from two
// different variables. On cash this is invisible — `cash_due` reads the stored
// total — so it takes a deliberate comparison to see it at all.
await flow('a promo code', async () => {
  const CODE = `QA${Date.now().toString().slice(-6)}`;
  const mk = await own('/api/owner/promotions', { code: CODE, kind: 'percent', value: 10, maxUses: 5 });
  step('the venue mints a code', mk.status === 200, `${mk.status} ${JSON.stringify(mk.body).slice(0, 90)}`);

  const chk = await j('/api/promo/check', { method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ slug, code: CODE, items: [{ product_id: DISH.id, quantity: 2 }] }) });
  const quoted = chk.body?.total;
  step('the cart is quoted a discounted subtotal',
    chk.status === 200 && chk.body?.discount > 0 && quoted === chk.body.subtotal - chk.body.discount,
    `subtotal=${chk.body?.subtotal} discount=${chk.body?.discount} total=${quoted}`);

  const o = await place({ items: [{ product_id: DISH.id, quantity: 2 }], promo: CODE, promo_code: CODE });
  const id = o.body?.id;
  step('an order with the code is placed', o.status < 400 && !!id, `${o.status} ${JSON.stringify(o.body).slice(0, 90)}`);

  const stored = (await readOrder(id, o.body?.access_token)).body;
  const expect = (chk.body?.subtotal ?? 0) - (chk.body?.discount ?? 0) + (stored?.delivery_fee ?? 0);
  step('the STORED total carries the discount',
    stored?.total === expect,
    `stored=${stored?.total} expected=${expect} discount_on_order=${stored?.discount ?? 'none'}`);

  // THE COMPARISON THAT MATTERS. `payment_intent` only exists on a card order
  // and the card rail is off here, so what this can check is that the order
  // the customer is shown and the order the venue stored agree, and that the
  // discount was recorded at all — the rest is in the code review.
  step('the discount is recorded on the order', typeof stored?.discount === 'number' && stored.discount > 0,
    `discount=${stored?.discount}`);

  await close(id, 'promo order');
  await own(`/api/owner/promotions/${CODE}/delete`, {});
});

// ── 2. A COLLECTION ORDER ───────────────────────────────────────────────────
//
// `allowed_next` gives Ready → [InDelivery, PickedUp, Refunding], and no
// Worker route writes PICKED_UP. So a customer who chose to collect leaves an
// order that the owner has no action to end. This flow is here to hold that
// claim to a live venue rather than to a reading of the table.
await flow('a collection order', async () => {
  const o = await place({ fulfilment: { kind: 'pickup' } });
  const id = o.body?.id;
  step('a pickup order is placed', o.status < 400 && !!id, `${o.status} ${JSON.stringify(o.body).slice(0, 110)}`);
  if (!id) return;

  for (const a of ['confirm', 'preparing', 'ready']) {
    const r = await own(`/api/owner/orders/${id}/action`, { action: a, location_id: VENUE });
    step(`pickup: ${a}`, r.status === 200, `${r.status}`);
  }
  // THE STEP THAT CLOSES IT. Written first as an always-false assertion
  // recording that no such action existed; `collected` -> PICKED_UP now does,
  // and a test that still asserts the gap is a stale instrument.
  const coll = await own(`/api/owner/orders/${id}/action`, { action: 'collected', location_id: VENUE });
  const st = await statusOf(id);
  step('a collection order can be marked collected', coll.status === 200 && st === 'PICKED_UP',
    `${coll.status} status=${st} ${coll.status >= 400 ? JSON.stringify(coll.body).slice(0, 80) : ''}`);

  // And the courier pool should not be offering it as a delivery.
  const tasks = await j('/api/courier/tasks', { headers: { authorization: `Bearer ${CT}` } });
  const offered = JSON.stringify(tasks.body || {}).includes(id);
  step('a collection order is NOT offered to couriers as a delivery', !offered,
    offered ? 'it is in the courier pool' : 'not in the pool');

  await close(id, 'pickup order');
});

// ── 3. AN ORDER THE OWNER ASSIGNS ───────────────────────────────────────────
await flow('an assigned order', async () => {
  const o = await place({});
  const id = o.body?.id;
  if (!id) { step('assigned: placed', false, `${o.status}`); return; }
  for (const a of ['confirm', 'preparing', 'ready']) {
    await own(`/api/owner/orders/${id}/action`, { action: a, location_id: VENUE });
  }
  const me = (await j('/api/courier/tasks', { headers: { authorization: `Bearer ${CT}` } })).body;
  const myId = me?.courier?.id || cl.body?.courier?.id;
  const asg = await own(`/api/owner/orders/${id}/assign`, { location_id: VENUE, courier_id: myId });
  step('the owner can assign a courier', asg.status === 200, `${asg.status} ${JSON.stringify(asg.body).slice(0, 90)}`);

  const tasks = await j('/api/courier/tasks', { headers: { authorization: `Bearer ${CT}` } });
  const mine = JSON.stringify(tasks.body || {}).includes(id);
  step('the assigned order reaches that courier', mine, mine ? 'in their list' : 'not in their list');

  await close(id, 'assigned order');
});

// ── 4. FEEDBACK ON A DELIVERED ORDER ────────────────────────────────────────
await flow('feedback', async () => {
  const o = await place({});
  const id = o.body?.id;
  if (!id) { step('feedback: placed', false, `${o.status}`); return; }
  await close(id, 'feedback order');
  const fb = await j(`/api/order/${id}/feedback`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...asCustomer(o.body?.access_token) },
    body: JSON.stringify({ rating: 5, text: 'QA — перевірка, не реальний відгук' }),
  });
  step('a customer can leave feedback on a delivered order', fb.status < 400,
    `${fb.status} ${JSON.stringify(fb.body).slice(0, 90)}`);
  const after = (await readOrder(id, o.body?.access_token)).body;
  step('the feedback is on the order', !!after?.feedback, JSON.stringify(after?.feedback || null).slice(0, 80));
});

// ── 5. THE CUSTOMER'S OWN SCREEN, IN A BROWSER ──────────────────────────────
await flow('the storefront in a browser', async () => {
  const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
  const p = await ctx.newPage();
  const bad = [];
  p.on('pageerror', e => bad.push(`uncaught: ${e.message.slice(0, 90)}`));
  await p.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForTimeout(6000);
  for (let i = 0; i < 4; i++) {
    const later = await p.$('#insLater'); if (!later) break;
    await later.click().catch(() => {}); await p.waitForTimeout(500);
  }
  const cards = await p.$$eval('[data-p]', e => e.length).catch(() => 0);
  step('store: the menu is on screen', cards > 0, `${cards} dishes`);

  // Add one to the cart and open it — the path every order starts on.
  const add = await p.$('[data-add]');
  if (add) { await add.click().catch(() => {}); await p.waitForTimeout(1200); }
  // `#cartPill` gains the class `show` and a count; that is the whole signal.
  const pill = await p.evaluate(() => {
    const el = document.getElementById('cartPill');
    return { shown: !!el?.classList.contains('show'),
             count: document.getElementById('pillCount')?.textContent || '',
             total: document.getElementById('pillTotal')?.textContent || '' };
  });
  step('store: the cart pill answers an add', pill.shown && pill.count !== '0' && pill.count !== '',
    `show=${pill.shown} count="${pill.count}" total="${pill.total}"`);
  await p.screenshot({ path: `${OUT}/store-cart.png` }).catch(() => {});
  step('store: nothing threw while ordering', bad.length === 0, bad.slice(0, 2).join(' | '));
  await b.close();
});

await cour('/api/courier/shift', { open: false });

console.log(`\n${fails.length ? 'FAILURES:\n  ' + fails.join('\n  ') : 'CUSTOMER FLOWS OK'}`);
process.exit(fails.length ? 1 : 0);
