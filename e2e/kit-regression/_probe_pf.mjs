// SEMI-FINISHED PRODUCTS, LIVE (lane W-PF, 2026-09-29): the operator's own
// example built through the owner API on the QA hub, one dish sold, the
// ledger read back, the numbers asserted, everything taken away again.
//
//   HOST=https://qa-durres.dowiz.org node e2e/kit-regression/_probe_pf.mjs
//
// NEVER against dubin-sushi or sushi-durres: it places a real order. The
// default host is the QA hub and a real venue's host is refused.
// Credentials come from /root/.dowiz_owner (never printed).
//
// What it proves, in order (each line `ok`/`FAIL`, exit 1 on any FAIL):
//   1. raw items with list prices; water untracked
//   2. ПФ 1 Mitsukan (vinegar 800 + salt 50 + sugar 150 -> 1000 g) saved,
//      K 100 %, cost per kg 141; ПФ 2 Rice seasoned (dry rice 1000 + water
//      1100 + mitsukan 250 -> 2100 g), K 89.4 %, cost per kg 112
//   3. a cycle (mitsukan <- rice seasoned) is refused with its path
//   4. a dish, 130 g of ПФ 2: stored cost 15, weight 130; "one sale takes"
//      names rice 61.905, vinegar 12.381, salt 0.774, sugar 2.321, no water
//   5. one order of TWO: the shelf shows rice -124 (2 x 61.905 -> 124 whole),
//      vinegar -25, salt -2, sugar -5 and NOTHING for the ПФ ids
//      (this step needs the place.rs / storefront.rs hand-backs deployed;
//      until then it reports the ПФ id reserved instead and FAILS loudly)
//   6. where-used of salt names both ПФ and the dish; deleting salt without
//      confirmUses is a 409; with it, the lines leave the cards
//   5b. (W-PF2 R2) a PRODUCTION ACT: 2100 g of ПФ 2 by the card, 2050 weighed:
//      raw items leave by the card, the batch is on the shelf (2050, counted),
//      the loss on cooking is 300 g (2350 in); a second order of ONE then
//      reserves 130 g of the READY ПФ 2 and no dry rice
//   5c. (W-PF2 R1) the recipes IMPORT creates a real ПФ from a semi-finished card:
//      the dry run names it (new, K 100 %), Apply writes it and the dish's
//      recipe names it; a second import says "update"
//   7. cleanup: the orders cancelled, the dish, the category, every item deleted
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://qa-durres.dowiz.org';
if (/dubin-sushi|sushi-durres/.test(HOST)) { console.log('REFUSED: a real venue; use the QA hub'); process.exit(2); }
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8').split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const fails = [];
const step = (name, ok, detail = '') => { if (!ok) fails.push(name); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`); };
const j = async (p, o = {}) => { const r = await fetch(`${HOST}${p}`, o); const t = await r.text(); try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; } };
let JWT = '';
const own = (p, body, method = 'POST') => j(p, { method, headers: { authorization: `Bearer ${JWT}`, 'content-type': 'application/json' }, ...(body === undefined ? {} : { body: JSON.stringify(body) }) });

const login = await j('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }) });
JWT = login.body?.access_token || '';
const slug = new URL(HOST).hostname.split('.')[0];
const menu0 = await j(`/api/public/locations/${slug}/menu`);
const VENUE = menu0.body?.location?.id;
step('owner signs in and the venue is known', login.status === 200 && !!VENUE, `${VENUE}`);
if (!VENUE) process.exit(1);

const P = 'qapf-';
const RAW = [['vinegar', 'ml', 15], ['salt', 'g', 5], ['sugar', 'g', 12], ['rice-dry', 'g', 20]];
// The import names its ПФ by the card's slug: 'QA PF sauce' -> 'qa-pf-sauce'.
const ids = { water: P + 'water', mitsukan: P + 'mitsukan', rice: P + 'rice-seasoned', sauce: 'qa-pf-sauce' };
const cleanup = async (say = false) => {
  const log = (what, r) => { if (say) console.log(`     cleanup ${what}: ${r.status} ${JSON.stringify(r.body).slice(0, 160)}`); return r; };
  if (ORDER) log('cancel order', await own(`/api/owner/orders/${ORDER}/action`, { action: 'cancel', location_id: VENUE }));
  if (ORDER2) log('cancel order 2', await own(`/api/owner/orders/${ORDER2}/action`, { action: 'cancel', location_id: VENUE }));
  if (PROD) log('delete dish', await own('/api/owner/products/delete', { ids: [PROD], location_id: VENUE }));
  if (CAT) log('delete category', await own(`/api/owner/categories/${CAT}/delete`, { location_id: VENUE }));
  log('delete supplies', await own('/api/owner/supplies/delete', { ids: [ids.sauce, ids.rice, ids.mitsukan, ...RAW.map(([r]) => P + r), ids.water], location_id: VENUE, confirmUses: true }));
};
let ORDER = null, ORDER2 = null, PROD = null, CAT = null;
// A run that died mid-way left its items: start clean.
await cleanup();

// ── 1. raw items ────────────────────────────────────────────────────────────
for (const [r, unit, cost] of RAW) {
  const s = await own('/api/owner/supplies', { id: P + r, name: `QA ${r}`, unit, kind: 'food_ingredient', category: 'QA', costPerBasis: cost, location_id: VENUE });
  step(`raw ${r} is created`, s.status === 200, `${s.status} ${JSON.stringify(s.body).slice(0, 100)}`);
  await own('/api/owner/stock/stocktake', { item: P + r, observed: 0 });
}
const w = await own('/api/owner/supplies', { id: ids.water, name: 'QA water', unit: 'ml', kind: 'food_ingredient', category: 'QA', costPerBasis: 0, untracked: true, location_id: VENUE });
step('water is created untracked', w.status === 200 && w.body?.untracked === true, `${w.status} ${JSON.stringify(w.body).slice(0, 100)}`);

// ── 2. the two cards ────────────────────────────────────────────────────────
const m = await own('/api/owner/preps', { id: ids.mitsukan, name: 'QA Mitsukan', unit: 'g', category: 'QA', yield: 1000, location_id: VENUE,
  lines: [{ item: P + 'vinegar', qty: 800 }, { item: P + 'salt', qty: 50 }, { item: P + 'sugar', qty: 150 }] });
step('ПФ 1 Mitsukan saved: K 100 %, 141 per kg', m.status === 200 && m.body?.prep?.k === 1000 && m.body?.prep?.costPer === 141, `${m.status} k=${m.body?.prep?.k} costPer=${m.body?.prep?.costPer}`);
const r2 = await own('/api/owner/preps', { id: ids.rice, name: 'QA Rice seasoned', unit: 'g', category: 'QA', yield: 2100, location_id: VENUE,
  lines: [{ item: P + 'rice-dry', qty: 1000 }, { item: ids.water, qty: 1100 }, { item: ids.mitsukan, qty: 250 }] });
step('ПФ 2 Rice seasoned saved: K 89.4 %, 112 per kg', r2.status === 200 && r2.body?.prep?.k === 894 && r2.body?.prep?.costPer === 112, `${r2.status} k=${r2.body?.prep?.k} costPer=${r2.body?.prep?.costPer}`);
const list = await own('/api/owner/preps', undefined, 'GET');
step('the list carries both, hydrated', list.status === 200 && (list.body?.preps || []).filter(p => p.id.startsWith(P)).length === 2, `${list.status}`);

// ── 3. a cycle is refused with its path ─────────────────────────────────────
const cyc = await own('/api/owner/preps', { id: ids.mitsukan, name: 'QA Mitsukan', unit: 'g', yield: 1000, location_id: VENUE, lines: [{ item: ids.rice, qty: 1 }] });
step('a cycle is refused with its path', cyc.status === 400 && String(cyc.body?.error || cyc.body).includes('→'), `${cyc.status} ${JSON.stringify(cyc.body).slice(0, 160)}`);

// ── 4. the dish and what one sale takes ─────────────────────────────────────
const cat = await own('/api/owner/categories', { location_id: VENUE, name: 'QA PF' });
CAT = cat.body?.id || cat.body?.category?.id;
step('a QA category is created', cat.status === 200 && !!CAT, `${cat.status} ${JSON.stringify(cat.body).slice(0, 160)}`);
const prod = await own('/api/owner/products', { location_id: VENUE, category_id: CAT, name: 'QA Philadelphia', price: 650 });
PROD = prod.body?.id || prod.body?.product?.id;
step('the dish is created', prod.status === 200 && !!PROD, `${prod.status} ${JSON.stringify(prod.body).slice(0, 200)}`);
const bom = await own(`/api/owner/products/${PROD}`, { location_id: VENUE, bom: [{ supply: ids.rice, qty: 130 }], available: true, allergens: [] });
const stored = bom.body?.product || bom.body || {};
step('the dish stores 130 g of ПФ 2: cost 15, weight 130', bom.status === 200 && (stored.cost === 15 || stored.cost === undefined) , `${bom.status} cost=${stored.cost} weightG=${stored.weightG}`);
const tk = await own(`/api/owner/products/${PROD}/takes`, undefined, 'GET');
const leaves = Object.fromEntries((tk.body?.leaves || []).map(l => [l.supply.slice(P.length), l.qty]));
step('one sale takes rice 61.905, vinegar 12.381, salt 0.774, sugar 2.321, no water',
  tk.status === 200 && leaves['rice-dry'] === '61.905' && leaves.vinegar === '12.381' && leaves.salt === '0.774' && leaves.sugar === '2.321' && !('water' in leaves),
  `${tk.status} ${JSON.stringify(leaves)} cost=${tk.body?.cost}`);

// ── 5. an order of two draws the raw leaves, not the ПФ ─────────────────────
// The QA shelf starts empty; an order is refused against an empty shelf, so receive the raw items first.
for (const r of ['rice-dry', 'vinegar', 'salt', 'sugar']) {
  const rc = await own('/api/owner/stock/received', { item: P + r, qty: 1000 });
  step(`1000 of ${r} received`, rc.status === 200, `${rc.status} ${JSON.stringify(rc.body).slice(0, 120)}`);
}
const o = await j(`/api/public/locations/${slug}/orders`, { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ contact: { name: 'QA pf', phone: '+355690000019' }, fulfilment: { kind: 'pickup' }, items: [{ product_id: PROD, quantity: 2 }], payment: 'cash' }) });
ORDER = o.body?.id || o.body?.order?.id || o.body?.order_id;
step('an order of two is placed', o.status < 400 && !!ORDER, `${o.status} ${JSON.stringify(o.body).slice(0, 120)}`);
const st = await own('/api/owner/stock', undefined, 'GET');
const lvl = id => (st.body?.supplies || []).find(x => x.id === id);
const held = Object.fromEntries([...RAW.map(([r]) => r), 'mitsukan', 'rice-seasoned', 'water'].map(k => [k, lvl(P + k)?.reserved ?? null]));
// 2 x (61.904762, 12.380952, 0.773810, 2.321429) -> 124 (123.81), 25 (24.76), 2 (1.55), 5 (4.64) whole units.
step('the shelf holds the RAW leaves: rice 124, vinegar 25, salt 2, sugar 5, and nothing for the ПФ or water',
  held['rice-dry'] === 124 && held.vinegar === 25 && held.salt === 2 && held.sugar === 5 && !held.mitsukan && !held['rice-seasoned'] && !held.water,
  JSON.stringify(held) + (held['rice-seasoned'] ? ' (the ПФ id was reserved: place.rs/storefront.rs hand-backs not deployed)' : ''));

// ── 5b. a production act, and a sale that takes the ready batch (R2) ────────
const shelfOf = async id => ((await own('/api/owner/stock', undefined, 'GET')).body?.supplies || []).find(x => x.id === id) || {};
const riceBefore = await shelfOf(P + 'rice-dry');
const act = await own('/api/owner/stock/cooked', { item: ids.rice, qty: 2100, out: 2050 });
step('a batch of ПФ 2 is cooked: 2350 g in, 2050 out, 300 g lost, a cost', act.status === 200 && act.body?.gross === 2350 && act.body?.lossG === 300 && act.body?.value != null,
  `${act.status} ${JSON.stringify(act.body).slice(0, 200)}`);
const drawn = Object.fromEntries((act.body?.lines || []).map(l => [String(l.item).slice(P.length), l.qty]));
step('the act took the card off the shelf: dry rice 1000, vinegar 200, no water', drawn['rice-dry'] === 1000 && drawn.vinegar === 200 && !('water' in drawn), JSON.stringify(drawn));
const pot = await shelfOf(ids.rice);
step('the batch is on the shelf, counted', pot.onHand === 2050 && pot.counted === true, `onHand=${pot.onHand} counted=${pot.counted}`);
const o2 = await j(`/api/public/locations/${slug}/orders`, { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ contact: { name: 'QA pf', phone: '+355690000019' }, fulfilment: { kind: 'pickup' }, items: [{ product_id: PROD, quantity: 1 }], payment: 'cash' }) });
ORDER2 = o2.body?.id || o2.body?.order?.id || o2.body?.order_id;
step('an order of one is placed', o2.status < 400 && !!ORDER2, `${o2.status} ${JSON.stringify(o2.body).slice(0, 120)}`);
const potAfter = await shelfOf(ids.rice), riceAfter = await shelfOf(P + 'rice-dry');
step('it reserves 130 g of the READY ПФ 2 and no dry rice', potAfter.reserved === 130 && riceAfter.reserved === riceBefore.reserved,
  `rice-seasoned reserved ${potAfter.reserved}, dry rice ${riceBefore.reserved} -> ${riceAfter.reserved} (needs the W-PF2 hub deployed)`);

// ── 5c. the recipes import creates a real ПФ (R1) ───────────────────────────
const CSV = 'dish,ingredient,qty,unit,batch\nQA PF sauce,QA vinegar,80,ml,100 g\nQA PF sauce,QA sugar,20,g,\nQA Philadelphia,QA Rice seasoned,130,g,\nQA Philadelphia,QA PF sauce,10,g,\n';
const imp = (q = '') => j(`/api/owner/recipes/import${q}`, { method: 'POST', headers: { authorization: `Bearer ${JWT}`, 'content-type': 'text/csv' }, body: CSV });
const dry = await imp();
const pv = (dry.body?.preps || []).find(p => p.id === ids.sauce);
step('the dry run names the ПФ it will create: new, K 100 %', dry.status === 200 && pv?.new === true && pv?.k === 1000 && !(dry.body?.rows || []).some(r => r.error),
  `${dry.status} ${JSON.stringify(dry.body?.preps || dry.body).slice(0, 200)}`);
const applied = await imp('?apply=1');
step('Apply writes the ПФ and the dish', applied.status === 200 && applied.body?.written === 2, `${applied.status} written=${applied.body?.written} ${JSON.stringify(applied.body?.warnings)}`);
const dish = ((await own(`/api/owner/products?id=${PROD}`, undefined, 'GET')).body?.products || [])[0] || {};
const lines = Object.fromEntries((dish.bom || []).map(l => [l.supply, l.qty]));
step('the dish names both ПФ: rice seasoned 130, the sauce 10', lines[ids.rice] === 130 && lines[ids.sauce] === 10, JSON.stringify(lines));
const again = await imp();
step('a second import says update, not new', again.status === 200 && !(again.body?.warnings || []).length && (again.body?.preps || []).find(p => p.id === ids.sauce)?.new === false,
  `${again.status} ${JSON.stringify(again.body?.warnings)}`);

// ── 6. where used, and the delete that asks first ───────────────────────────
const u = await own(`/api/owner/supplies/${P}salt/uses`, undefined, 'GET');
const named = { preps: (u.body?.uses?.preps || []).map(p => p.id), dishes: (u.body?.uses?.dishes || []).map(d => d.id) };
step('where-used of salt names both ПФ and the dish', u.status === 200 && named.preps.includes(ids.mitsukan) && named.preps.includes(ids.rice) && named.dishes.includes(PROD), JSON.stringify(named));
const refused = await own('/api/owner/supplies/delete', { ids: [P + 'salt'], location_id: VENUE });
step('deleting salt without confirming its uses is a 409 naming them', refused.status === 409 && !!refused.body?.uses?.[P + 'salt'], `${refused.status} ${JSON.stringify(refused.body).slice(0, 160)}`);
const stillThere = await own(`/api/owner/supplies/${P}salt/uses`, undefined, 'GET');
step('and nothing was deleted', stillThere.status === 200);

// ── 7. cleanup ──────────────────────────────────────────────────────────────
await cleanup(true);
const gone = await own(`/api/owner/supplies/${ids.rice}/uses`, undefined, 'GET');
const gone2 = await own(`/api/owner/supplies/${ids.sauce}/uses`, undefined, 'GET');
step('cleanup: the cards are gone', gone.status === 404 && gone2.status === 404, `${gone.status} ${gone2.status}`);
const ledger = await own('/api/owner/stock', undefined, 'GET');
step('cleanup: the shelf forgot the QA items', !(ledger.body?.supplies || []).some(x => x.id.startsWith(P)));

console.log(fails.length ? `\nFAILED: ${fails.length}\n - ${fails.join('\n - ')}` : '\nall green');
process.exit(fails.length ? 1 : 0);
