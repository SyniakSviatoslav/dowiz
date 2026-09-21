// DOES AN ORDER ACTUALLY DRAW THE STOCK DOWN?
//
// `dowiz_hub::stock` implements a full deterministic ledger -- Received,
// Reserved, Consumed, Released, Wasted, Stocktake, folded rather than counted,
// with an `OutOfStock` refusal that IS the automatic 86. The Worker calls it
// from three places: a reservation when an order is placed, a settle when the
// kitchen starts or the order dies, and the owner's three manual movements.
//
// None of that had ever run on a live venue: `GET /api/owner/stock` answered
// `{"supplies": []}`, so every order this platform has taken reserved nothing,
// consumed nothing, and could not have been refused for stock it did not have.
// A ledger with no supplies in it is indistinguishable from a ledger that does
// not work, which is the whole reason for this file.
//
// It builds its own ingredient and its own dish, orders them, and takes both
// away again, so it can be run against a real venue without touching the menu
// anyone sees for longer than it takes to run.
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const SUPPLY = 'qa-stock-salmon';
const PER_DISH = 200;      // grams of the supply in one dish
const RECEIVED = 1000;     // grams put on the shelf

const fails = [];
const step = (name, ok, detail = '') => {
  if (!ok) fails.push(name);
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};

const j = async (p, o = {}) => {
  const r = await fetch(`${HOST}${p}`, o);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};
let JWT = '';
const own = (p, body, method = 'POST') => j(p, {
  method,
  headers: { authorization: `Bearer ${JWT}`, 'content-type': 'application/json' },
  ...(body === undefined ? {} : { body: JSON.stringify(body) }),
});

// ── who we are, and which venue this host is ────────────────────────────────
const login = await j('/api/auth/login', {
  method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }),
});
JWT = login.body?.access_token || '';
const slug = new URL(HOST).hostname.split('.')[0];
const menu0 = await j(`/api/public/locations/${slug}/menu`);
const VENUE = menu0.body?.location?.id;
step('owner signs in and the venue is known', login.status === 200 && !!VENUE, `${VENUE}`);
if (!VENUE) { console.log('cannot continue'); process.exit(1); }

const level = async (item = SUPPLY) => {
  const s = await own('/api/owner/stock', undefined, 'GET');
  const row = (s.body?.supplies || []).find(x => x.id === item);
  return row ? { onHand: row.onHand, reserved: row.reserved, available: row.available } : null;
};

// ── 1. an ingredient, and a delivery of it ──────────────────────────────────
const sup = await own('/api/owner/supplies', {
  id: SUPPLY, name: 'QA Salmon', unit: 'g', kind: 'food_ingredient',
  category: 'QA', kcalPer100: 208, proteinPer100: 20, fatPer100: 13, carbsPer100: 0,
  costPerBasis: 1900, lowAt: 100, active: true, locationId: VENUE,
});
step('the ingredient is created', sup.status === 200, `${sup.status} ${JSON.stringify(sup.body).slice(0, 120)}`);

// A RETIRED SUPPLY KEEPS ITS LEDGER HISTORY, by design -- retiring is not
// deleting, and the events that moved it stay folded. So re-creating this
// ingredient under the same id resurrects whatever the last run left on the
// shelf: the second run of this file read 1600 g after delivering 1000 g, and
// blamed the reservation arithmetic for it. A stocktake is the one event that
// SETS a level rather than moving it, so the run starts from a counted zero.
await own(`/api/owner/stock/stocktake`, { item: SUPPLY, observed: 0 });
const recv = await own(`/api/owner/stock/received`, { item: SUPPLY, qty: RECEIVED });
step('a delivery is recorded', recv.status === 200, `${recv.status} ${JSON.stringify(recv.body).slice(0, 100)}`);

let L = await level();
step('the shelf holds what was delivered', L && L.onHand === RECEIVED && L.available === RECEIVED,
  JSON.stringify(L));

// ── 2. a dish made of it ────────────────────────────────────────────────────
const cat = await own('/api/owner/categories', { location_id: VENUE, name: 'QA Stock' });
const CAT = cat.body?.id || cat.body?.category?.id;
step('a category exists to hold it', cat.status === 200 && !!CAT, `${cat.status} ${CAT}`);

const prod = await own('/api/owner/products', {
  location_id: VENUE, category_id: CAT, name: 'QA Stock Dish', price: 100, available: true,
});
const PROD = prod.body?.id || prod.body?.product?.id;
step('the dish exists', prod.status === 200 && !!PROD, `${prod.status} ${PROD}`);

const bom = await own(`/api/owner/products/${PROD}`, {
  location_id: VENUE,
  bom: [{ supply: SUPPLY, qty: PER_DISH }],
});
step('the dish has a recipe', bom.status === 200, `${bom.status} ${JSON.stringify(bom.body).slice(0, 140)}`);

// ── 2b. and the menu can be asked whether a dish has one ────────────────────
//
// The public menu never carries `bom` -- a recipe is the venue's business --
// so counting recipes by looking for it reads zero on a venue where every
// dish has one. What it does carry is what the recipe DERIVES: `weightG` and
// `nutritionDerived` exist only when a bom does. This step is here to prove
// that instrument fires, so the coverage number it produces elsewhere means
// something. `?fresh=1` because the public menu is cached for 30 s + 300 s
// stale-while-revalidate, and a dish saved a second ago is otherwise absent.
{
  const m = await j(`/api/public/locations/${slug}/menu?fresh=1`);
  const dish = (m.body?.categories || []).flatMap(c => c.products || []).find(x => x.id === PROD);
  step('a dish with a recipe is visible AS having one',
    !!dish && dish.nutritionDerived === true && dish.weightG === PER_DISH,
    dish ? `nutritionDerived=${dish.nutritionDerived} weightG=${dish.weightG} kcal=${dish.nutrition?.kcal ?? '?'}`
         : 'the dish is not on the menu');
}

// ── 3. an order reserves, and does not yet consume ──────────────────────────
const place = async (qty) => j(`/api/public/locations/${slug}/orders`, {
  method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    contact: { name: 'QA stock', phone: '+355690000009' },
    fulfilment: { kind: 'delivery', address: { line: 'Rruga Taulantia 12', note: 'QA — не готувати, це перевірка' } },
    items: [{ product_id: PROD, quantity: qty }],
    payment: 'cash',
  }),
});

const o1 = await place(2);
const ORDER1 = o1.body?.id || o1.body?.order?.id;
step('an order is placed', o1.status < 400 && !!ORDER1, `${o1.status} ${JSON.stringify(o1.body).slice(0, 140)}`);

L = await level();
step('placing RESERVES, it does not consume',
  L && L.onHand === RECEIVED && L.reserved === 2 * PER_DISH && L.available === RECEIVED - 2 * PER_DISH,
  JSON.stringify(L));

// ── 4. the kitchen starting is what consumes ────────────────────────────────
await own(`/api/owner/orders/${ORDER1}/action`, { action: 'confirm', location_id: VENUE });
await own(`/api/owner/orders/${ORDER1}/action`, { action: 'preparing', location_id: VENUE });
L = await level();
step('preparing CONSUMES: it leaves the shelf and the reservation clears',
  L && L.onHand === RECEIVED - 2 * PER_DISH && L.reserved === 0,
  JSON.stringify(L));

// ── 5. a cancelled order gives its reservation back ─────────────────────────
const o2 = await place(1);
const ORDER2 = o2.body?.id || o2.body?.order?.id;
const beforeCancel = await level();
await own(`/api/owner/orders/${ORDER2}/action`, { action: 'cancel', location_id: VENUE });
const afterCancel = await level();
step('a cancelled order RELEASES what it held',
  beforeCancel && afterCancel &&
  beforeCancel.reserved === PER_DISH && afterCancel.reserved === 0 &&
  afterCancel.onHand === beforeCancel.onHand,
  `${JSON.stringify(beforeCancel)} -> ${JSON.stringify(afterCancel)}`);

// ── 6. THE AUTOMATIC 86 ─────────────────────────────────────────────────────
// §4's I1: an order reserving more than is available is REFUSED. That refusal
// is the stop-listing -- nothing has to notice a level hit zero and go and
// flip a boolean, which is the version that races the next order and oversells.
const avail = (await level())?.available ?? 0;
const tooMany = Math.floor(avail / PER_DISH) + 1;
const over = await place(tooMany);
step('an order for more than is on the shelf is REFUSED',
  over.status >= 400,
  `asked for ${tooMany} (${tooMany * PER_DISH} g of ${avail} g) -> ${over.status} ${JSON.stringify(over.body).slice(0, 120)}`);
const overId = over.body?.id || over.body?.order?.id;

// ── 7. put the venue back ───────────────────────────────────────────────────
for (const id of [ORDER1, overId].filter(Boolean)) {
  await own(`/api/owner/orders/${id}/action`, { action: 'cancel', location_id: VENUE });
}
if (PROD) await own(`/api/owner/products/${PROD}/delete`, { location_id: VENUE });
if (CAT) await own(`/api/owner/categories/${CAT}/delete`, { location_id: VENUE });
await own(`/api/owner/stock/stocktake`, { item: SUPPLY, observed: 0 });
const retire = await own(`/api/owner/supplies/${SUPPLY}/retire`, { location_id: VENUE });
const leftovers = await own('/api/owner/stock', undefined, 'GET');
const still = (leftovers.body?.supplies || []).find(x => x.id === SUPPLY);
step('the venue is left as it was found', !still,
  `retire ${retire.status}; ${still ? 'supply still listed' : 'supply gone'}; stranded=${JSON.stringify(leftovers.body?.stranded || [])}`);

console.log(`\n${fails.length ? 'FAILURES:\n  ' + fails.join('\n  ') : 'STOCK CYCLE OK'}`);
process.exit(fails.length ? 1 : 0);
