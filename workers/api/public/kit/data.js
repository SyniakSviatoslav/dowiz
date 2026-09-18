// dowiz kit — the venue's real data.
//
// The kit's frames ship with the designer's content (ItaliaCrisp Pizza, $12.00,
// New York). That content is the SPECIFICATION, and it stays in each screen as
// the fallback, because a screen that renders nothing when the API is down is
// the bug this whole session started with.
//
// What this module does is ask the same endpoint the storefront already uses
// and hand back records in the shape the screens already draw. When there is no
// venue -- the kit opened on a host that names none, or the request failed --
// it says so and the screen keeps its own content.

import * as Money from '/lib/money.js';
import { productOf as basketProduct } from '/kit/basket.js';

const API = '/api';

// WHICH VENUE THIS IS. Identical rule to the storefront (public/app.js), on
// purpose: two answers to "which venue" is how a customer ends up looking at
// one hub's menu under another hub's name.
export const SLUG = (() => {
  const explicit = new URLSearchParams(location.search).get('s');
  if (explicit) return explicit;
  const host = location.hostname.toLowerCase();
  if (host.endsWith('.workers.dev') || host === 'localhost') return '';
  const labels = host.split('.');
  if (labels.length > 2 && labels[0] !== 'www') return labels[0];
  return '';
})();

// One request per language, kept for the life of the page. The menu is the same
// answer for every screen that needs it, and three screens asking separately is
// three chances for them to disagree about the price.
const cache = new Map();

export function menu(lang = 'uk'){
  if (!SLUG) return Promise.resolve(null);
  const key = `${SLUG}:${lang}`;
  if (!cache.has(key)) cache.set(key, load(SLUG, lang).catch(() => null));
  return cache.get(key);
}

async function load(slug, lang){
  const r = await fetch(
    `${API}/public/locations/${encodeURIComponent(slug)}/menu?locale=${encodeURIComponent(lang)}`);
  if (!r.ok) throw new Error(`menu ${r.status}`);
  const body = await r.json();
  if (!body || !Array.isArray(body.categories)) throw new Error('menu: no categories');
  return body;
}

// ── Shapes the screens already draw ────────────────────────────────────────
// Money arrives as integer minor units with a currency beside it. The kit's
// frames print dollars; a venue in Durrës is charged in lekë. Neither this
// module nor a screen decides an amount -- the server's number is formatted and
// nothing else.
/// THE FOURTH COPY OF THE MONEY RULE, DELETED.
///
/// This divided every amount by a hundred and asked Intl for two decimals, so a
/// venue trading in LEK -- which has no minor unit at all -- had 1500 lek drawn
/// as `$15.00`: off by a factor of a hundred AND in the wrong currency, because
/// the fallback code was `USD`. `lib/money.js` already holds this rule for the
/// storefront, the owner console and the courier app, and the comment at the
/// top of that file records that the last time this rule existed three times,
/// two of the copies were wrong. It now exists once.
export function formatMoney(minor, currency){
  return Money.format(Number(minor || 0), currency || 'ALL');
}

export function dishes(body, limit = 12){
  if (!body) return null;
  const currency = body.location?.currencyCode;
  return (body.categories || [])
    .flatMap(c => (c.products || []).map(p => ({ product: p, category: c })))
    .filter(({ product }) => product.available !== false)
    .slice(0, limit)
    .map(({ product, category }) => ({
      id: product.id,
      name: product.name,
      kind: category.name,
      description: product.description || '',
      imageUrl: product.imageUrl || null,
      // WHAT IS IN THE DISH. Passed through exactly as the hub sent it, null
      // included: `null` is "the venue has not declared this" and `0`/`[]` are
      // claims the venue made. A screen must be able to tell them apart, so
      // nothing here is defaulted.
      ingredients: product.ingredients ?? null,
      weightG: product.weightG ?? null,
      nutrition: product.nutrition ?? null,
      calories: product.calories ?? (product.nutrition && product.nutrition.kcal) ?? null,
      allergens: product.allergens ?? null,
      available: product.available !== false,
      unavailableNote: product.unavailableNote ?? null,
      sizeCm: product.sizeCm ?? null,
      cookingMin: product.cookingMin ?? null,
      categoryId: category.id,
      price: formatMoney(product.price, currency),
      priceMinor: product.price,
      // The kit's cards show a rating, a time and a distance. dowiz has none of
      // those per dish and does not invent them: trust here is a signed
      // capability, never a score (DECISIONS.md). The card leaves them out
      // rather than printing a number nobody measured.
      mins: body.location?.deliveryEta || null,
      veg: !(product.allergens || []).some(a => /fish|milk|egg|molluscs|crustacean/.test(a)),
    }));
}

// ── The whole catalogue, grouped the way the venue groups it ───────────────
//
// `dishes()` answers "give me a rail of N"; a catalogue screen needs every
// dish, in the venue's own categories, INCLUDING the sold-out ones. Sold-out
// dishes stay visible with the venue's reason rather than vanishing: a customer
// who cannot find yesterday's dish assumes the app is broken, and the old
// service kept them on the page deliberately (see products.unavailable_note).
export function catalogue(body){
  if (!body) return null;
  const currency = body.location?.currencyCode;
  const cats = (body.categories || []).map(c => ({
    id: c.id,
    name: c.name,
    sortOrder: c.sortOrder ?? 0,
    items: (c.products || []).map(p => ({
      id: p.id,
      name: p.name,
      kind: c.name,
      categoryId: c.id,
      description: p.description || '',
      imageUrl: p.imageUrl || null,
      price: formatMoney(p.price, currency),
      priceMinor: p.price,
      currency,
      available: p.available !== false,
      unavailableNote: p.unavailableNote ?? null,
      allergens: p.allergens ?? null,
      ingredients: p.ingredients ?? null,
      weightG: p.weightG ?? null,
      nutrition: p.nutrition ?? null,
      calories: p.calories ?? (p.nutrition && p.nutrition.kcal) ?? null,
      sizeCm: p.sizeCm ?? null,
      cookingMin: p.cookingMin ?? null,
      modifierGroups: p.modifierGroups ?? null,
      sortOrder: p.sortOrder ?? 0,
    })),
  })).filter(c => c.items.length);
  return { categories: cats, items: cats.flatMap(c => c.items), currency,
           venue: venue(body) };
}

/// One dish by id, across every category.
export function dishById(body, id){
  const cat = catalogue(body);
  return cat ? cat.items.find(i => i.id === id) || null : null;
}

export function venue(body){
  if (!body?.location) return null;
  const l = body.location;
  return {
    name: l.name,
    address: l.address || '',
    slug: l.slug,
    currency: l.currencyCode,
    eta: l.deliveryEta,
    phone: l.phone,
    open: l.status === 'open',
    minOrder: l.minOrder,
    deliveryFee: l.deliveryFee,
    freeOver: l.freeDeliveryThreshold,
  };
}

// ── The domains that now have a kernel behind them ─────────────────────────
//
// Reservations, threads and the wallet journal. Each of these calls a Worker
// route whose LAW is in the kernel (`dowiz_kernel::reservation`, `::thread`,
// `::ledger_account`, `::pass`) — the screen asks, it never decides.
//
// Every one returns `null` rather than throwing when there is no venue or the
// request fails, because the frame's own content is the fallback and a screen
// that throws is a screen that renders nothing.

async function call(path, init){
  if (!SLUG) return null;
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}${path}`, init);
    const text = await r.text();
    let body = null;
    try { body = text ? JSON.parse(text) : null; } catch { /* not json */ }
    if (!r.ok) return { error: (body && body.error) || text || `HTTP ${r.status}`, status: r.status };
    return body;
  } catch (e) {
    return { error: String(e && e.message || e), status: 0 };
  }
}

const json = body => ({
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify(body),
});

// A stable key per attempt, so a retried request does not book a second table
// or move money twice. The server treats it as the idempotency key.
export const requestId = () => {
  try {
    return crypto.randomUUID();
  } catch {
    return `r${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;
  }
};

export const reservations = {
  list: user => call(`/reservations?user=${encodeURIComponent(user)}`),
  detail: id => call(`/reservations/${encodeURIComponent(id)}`),
  create: body => call('/reservations', json(body)),
  act: (id, body) => call(`/reservations/${encodeURIComponent(id)}/action`, json(body)),
  pass: id => call(`/reservations/${encodeURIComponent(id)}/pass`),
  verifyPass: code => call('/pass/verify', json({ code })),
};

export const threads = {
  messages: (id, after = 0) => call(`/threads/${encodeURIComponent(id)}?after=${after}`),
  send: (id, body) => call(`/threads/${encodeURIComponent(id)}/messages`, json(body)),
};

export const wallet = {
  balance: user => call(`/wallet?user=${encodeURIComponent(user)}`),
  statement: user => call(`/wallet/statement?user=${encodeURIComponent(user)}`),
  topUp: body => call('/wallet/topup', json(body)),
};

// ── THE ORDER ──
//
// The one transaction this whole product exists for, and the kit could not make
// it: every checkout screen ended at a design frame, so a customer could fill a
// basket, read a total and press "Place Order" without anything being ordered.
//
// This is the SAME endpoint the storefront has always used. Nothing about the
// order is decided here: the hub re-derives every price from its own catalogue
// (`place_order_priced`), so a tampered `unitPrice` changes nothing except that
// the hub refuses. What travels is what the customer chose.
export const orders = {
  place: body => call('/orders', json(body)),
};
//
// THERE IS NO `orders.detail` HERE, and the one that used to be was a 404.
//
// It called `/api/public/locations/:slug/orders/:id`, a route that does not
// exist -- every public route the hub has is in `workers/api/src/lib.rs`, and
// reading one order is not among them. The real route is `/api/order/:id`,
// outside the location prefix, and it refuses an id on its own: an order id
// appears in a URL and a screenshot, and behind it sit a name, a phone and a
// street address. The key is minted with the order and returned exactly once.
//
// That is why reading an order back belongs to `kit/orders.js`, which is where
// the key is kept, and not to this module.

/// Build the order payload from a basket and a contact.
///
/// Kept beside the call rather than in a screen, because two screens building
/// this shape slightly differently is how one of them starts sending a field
/// the hub silently ignores.
export function orderPayload({ lines, contact, mode, address, note, payment = 'cash', locale = 'sq' }){
  return {
    items: (lines || []).map(l => ({
      // THE DISH, NOT THE LINE. `l.id` is `productId:sizeId` so that one basket
      // can hold two sizes of the same dish; the hub looks `product_id` up in
      // the venue's catalogue and refuses anything it does not find.
      product_id: basketProduct(l),
      modifier_ids: l.modifierIds || [],
      quantity: l.qty,
      unit_price: l.cents,
    })),
    contact: { name: contact?.name || '', phone: contact?.phone || '' },
    fulfilment: {
      kind: mode === 'pickup' ? 'pickup' : 'delivery',
      ...(mode === 'pickup' ? {} : { address: { line: address || '', note: note || null } }),
    },
    payment,
    locale,
  };
}

// The delivery estimate. Built by `dowiz_kernel::eta` from the dishes' own
// cooking times, the orders already in the kitchen and the distance — not from
// a string the venue typed once.
export const eta = {
  quote: body => call('/eta', json(body)),
};
