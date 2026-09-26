// QA HUB SETUP -- qa-durres.dowiz.org, the venue walks may WRITE to (operator 2026-09-26).
//   HOST=https://qa-durres.dowiz.org LOC=qa-durres node e2e/walk/qa-setup.mjs
// Idempotent: every step reads first and writes only what is missing.
//
// WHY /api/bootstrap AND NOT /api/platform/hubs. The card says POST /api/platform/hubs with the owner
// credentials; that route answers 404 to the owner because the owner is not in `platform_admins`
// (platform.rs admin_only), and no route writes that table. The bootstrap route is the other door,
// behind BOOTSTRAP_SECRET from the same credentials file.
//
// THE FIRST BOOTSTRAP LANDS IN THE WRONG OBJECT (defect handed back): `bootstrap::seed` resolves its
// place from the Host BEFORE it writes the registry pointer, so for a slug the registry does not know
// yet `Place::of_slug` answers UNNAMED_VENUE ("hub") and the catalogue goes there. The registry row IS
// written, so a second identical call reaches the venue's own object. This script therefore calls it
// until the storefront answers 200, at most three times, and prints each answer.
import { api, creds, step, end, save } from './_lib.mjs';

const LOC = process.env.LOC || 'qa-durres';
const location = {
  id: LOC, slug: LOC, name: 'QA Durres', phone: '+355690000000', address: 'QA test venue -- not a restaurant',
  status: 'open', closes_at: null, delivery_eta: '30-45', delivery_fee: 0, free_delivery_threshold: null,
  min_order: 0, currency_code: 'ALL', menu_version: 1, supported_locales: '["sq","en","uk"]',
  default_locale: 'en', delivery_paused: 0,
};
// [id, name, price lek, allergens, recipe [[supply, grams]]]
const menu = [
  { id: 'qa-rolls', name: 'QA Rolls', dishes: [
    ['qa-salmon-roll', 'QA Salmon roll', 900, ['fish', 'soy'], [['qa-salmon', 80], ['qa-rice', 120]]],
    ['qa-tuna-roll', 'QA Tuna roll', 1000, ['fish'], [['qa-tuna', 80], ['qa-rice', 120]]],
    ['qa-veg-roll', 'QA Veg roll', 700, ['sesame'], [['qa-rice', 150]]]] },
  { id: 'qa-drinks', name: 'QA Drinks', dishes: [
    ['qa-water', 'QA Water', 150, [], []], ['qa-tea', 'QA Green tea', 250, [], []]] },
];
// [id, name, unit, opening stock]
const supplies = [['qa-salmon', 'QA Salmon', 'g', 5000], ['qa-tuna', 'QA Tuna', 'g', 3000], ['qa-rice', 'QA Sushi rice', 'g', 20000]];

try {
  let pub = await api(`/api/public/locations/${LOC}/menu`);
  for (let i = 0; i < 3 && pub.status !== 200; i++) {
    const r = await api('/api/bootstrap', { method: 'POST', headers: { 'x-dowiz-bootstrap': creds.BOOTSTRAP_SECRET }, body: { location, owner: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, name: 'QA owner' } } });
    step(null, `bootstrap try ${i + 1}`, `${r.status} ${JSON.stringify(r.body).replace(/"(jwt|token|access_token)":"[^"]+"/g, '"$1":"..."').slice(0, 200)}`);
    pub = await api(`/api/public/locations/${LOC}/menu`);
  }
  step(pub.status === 200 && pub.body?.location?.slug === LOC, 'the QA storefront menu answers for its own slug', `${pub.status} slug=${pub.body?.location?.slug}`);

  const login = await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, location_id: LOC } });
  const tok = login.body?.access_token;
  step(login.status === 200 && !!tok, 'owner signs in to the QA hub', `${login.status}`);
  const cats = await api(`/api/owner/categories?location_id=${LOC}`, { token: tok });
  const have = new Set((Array.isArray(cats.body) ? cats.body : cats.body?.categories || []).map(c => c.id));
  const prods = await api(`/api/owner/products?location_id=${LOC}`, { token: tok });
  const haveP = new Set((Array.isArray(prods.body) ? prods.body : prods.body?.products || []).map(p => p.id));
  const stock0 = await api(`/api/owner/stock?location_id=${LOC}`, { token: tok });
  const stockText = JSON.stringify(stock0.body);
  for (const [id, name, unit, opening] of supplies) {
    const r = await api(`/api/owner/supplies?location_id=${LOC}`, { method: 'POST', token: tok, body: { id, name, unit, kind: 'food_ingredient', lowAt: 500 } });
    step(r.status === 200, `supply ${id} saved`, `${r.status} ${JSON.stringify(r.body).slice(0, 160)}`);
    if (stockText.includes(`"${id}"`) && /"onHand":[1-9]/.test(stockText)) continue;
    const m = await api(`/api/owner/stock/received?location_id=${LOC}`, { method: 'POST', token: tok, body: { item: id, qty: opening } });
    step(m.status === 200, `opening stock ${id} = ${opening} ${unit}`, `${m.status} ${JSON.stringify(m.body).slice(0, 160)}`);
  }
  for (const [ci, c] of menu.entries()) {
    if (!have.has(c.id)) {
      const r = await api('/api/owner/categories', { method: 'POST', token: tok, body: { location_id: LOC, id: c.id, name: c.name, sort_order: ci } });
      step(r.status === 200, `category ${c.id} created`, `${r.status} ${JSON.stringify(r.body).slice(0, 160)}`);
    }
    for (const [id, name, price, allergens, bom] of c.dishes) {
      if (!haveP.has(id)) {
        // A dish goes on sale only once its allergens are declared (409 otherwise): add it off sale first.
        const r = await api('/api/owner/products', { method: 'POST', token: tok, body: { location_id: LOC, category_id: c.id, id, name, price, available: false } });
        step(r.status === 200, `dish ${id} created`, `${r.status} ${JSON.stringify(r.body).slice(0, 160)}`);
      }
      const r2 = await api(`/api/owner/products/${id}`, { method: 'POST', token: tok, body: { location_id: LOC, allergens, available: true, bom: bom.map(([supply, qty]) => ({ supply, qty })) } });
      step(r2.status === 200, `dish ${id} declared, recipe set, on sale`, `${r2.status} ${JSON.stringify(r2.body).slice(0, 160)}`);
    }
  }
  // `?fresh=1`: the public menu is edge-cached, and the cache answers the menu from before the writes.
  pub = await api(`/api/public/locations/${LOC}/menu?fresh=1`);
  const dishes = (pub.body?.categories || []).flatMap(c => c.products || []);
  step(dishes.length >= 5, 'the storefront lists the QA dishes', `${dishes.length} dishes: ${dishes.map(d => d.id).join(',')}`);
  save({ qaHub: LOC, qaDishes: dishes.map(d => d.id) });
} catch (e) {
  step(false, 'qa-setup threw', e.stack?.slice(0, 400));
}
await end(null);
