// THE OFFLINE CASH SALE, decided on the tablet (lane W-OFFSALE, row OF3;
// operator decision D8, 2026-10-03). PURE: no DOM, no fetch, no storage, no
// clock -- `room/sell.js` passes the instant in.
//
// THE PRICE IS THE PRICER'S RULE, NOT A SECOND ONE. The Worker prices a line
// with `services::ordering::pricing::price_basket`; a dish sold here has no
// options chosen, so its unit is the catalogue `price`, and the same refusals
// apply in the same order: quantity 1..99, unknown, `available` not true,
// price missing or negative, a required option group. ONE FIXTURE pins both
// sides -- `services/orders/offline_sale/parity.json` -- read by the Rust
// test and by `offline-sale.test.mjs`; a rule changed on one side alone turns
// one of them red. The server re-prices at the sync and KEEPS what the guest
// paid, flagging a dish that moved (`offline_sale::rules::envelope`).
//
// MONEY IS INTEGER MINOR UNITS: every product and every sum is checked with
// `Number.isSafeInteger`, never rounded, never divided.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES 11).

export const MAX_QTY = 99;
export const MAX_LINES = 50;
/// The order id the sale becomes on the server (`offline_sale::PREFIX`).
export const PREFIX = 'offline:';
/// 48 h, Law 87/2019 art. 29 (`fiscal::queue::DEADLINE_MS`).
export const DEADLINE_MS = 48 * 3600 * 1000;

/// The menu read's categories as one map, id -> product.
export function productsOf(categories) {
  const m = {};
  for (const c of categories || []) for (const p of c.products || []) if (p && p.id) m[p.id] = p;
  return m;
}

/// `dowiz_hub::modifiers`: a group with an id, at least one option with an id,
/// and `min >= 1` must be chosen; a sale with no options chosen is refused.
function needsOptions(p) {
  const gs = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  return gs.some(g => g && g.id && Array.isArray(g.options) && g.options.some(o => o && o.id) && Number(g.min) >= 1);
}

/// Price `lines` ([[product_id, quantity], ...]) against `products`.
/// `{ ok: true, units, total, lines }` or `{ ok: false, refusal, product_id }`,
/// with the refusal words of `offline_sale::rules::refusal_kind`.
export function priceOffline(products, lines) {
  const units = [], out = [];
  let total = 0;
  // The server's bound (`offline_sale::rules::MAX_LINES`), said here rather than refused there.
  if (lines.length > MAX_LINES) return { ok: false, refusal: 'quantity', product_id: null };
  for (const [id, qty] of lines) {
    if (!Number.isSafeInteger(qty) || qty < 1 || qty > MAX_QTY) return { ok: false, refusal: 'quantity', product_id: id };
    // OWN keys only: `constructor` is not a dish on any menu.
    const p = Object.prototype.hasOwnProperty.call(products, id) ? products[id] : null;
    if (!p) return { ok: false, refusal: 'unknown', product_id: id };
    if (p.available !== true) return { ok: false, refusal: 'off_sale', product_id: id };
    if (!Number.isSafeInteger(p.price) || p.price < 0) return { ok: false, refusal: 'no_price', product_id: id };
    if (needsOptions(p)) return { ok: false, refusal: 'options', product_id: id };
    const gross = p.price * qty;
    total += gross;
    if (!Number.isSafeInteger(gross) || !Number.isSafeInteger(total)) return { ok: false, refusal: 'quantity', product_id: id };
    units.push(p.price);
    out.push({ product_id: id, quantity: qty, unit_price: p.price, name: [...String(p.name || id)].slice(0, 120).join('') });
  }
  return { ok: true, units, total, lines: out };
}

/// The basket the picker holds ({id: qty}) as priced lines, in a stable order.
export const basketLines = basket => Object.entries(basket || {}).filter(([, q]) => q > 0).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));

/// `POST /api/staff/offline_sales` -- `offline_sale::SaleIn`, exactly its fields.
export function saleBody({ loc, key, soldAt, currency, priced, menuVersion }) {
  const b = { location_id: loc, sale_key: key, sold_at_ms: soldAt, currency, method: 'cash',
    lines: priced.lines.map(l => ({ product_id: l.product_id, quantity: l.quantity, unit_price: l.unit_price, name: l.name })),
    total: priced.total };
  if (Number.isSafeInteger(menuVersion)) b.menu_version = menuVersion;
  return b;
}

/// The fiscal deadline the receipt and the journal show.
export const deadlineOf = soldAt => soldAt + DEADLINE_MS;

/// THE RECEIPT, as lines of text (`renderReceipt` draws them; `print()` prints
/// them). The legal sentence is ALWAYS in Albanian -- the receipt is a fiscal
/// paper in Albania -- with the reader's own language under it when it is not
/// Albanian. `w` is `{ title, total, cash, ref, noNivf, noNivfSq, when }`,
/// `fmt(amount)` draws money (`logic.money`), `at(ms)` draws an instant.
export function receiptLines(sale, w, fmt, at) {
  const rows = [w.title, at(sale.sold_at_ms), ''];
  for (const l of sale.lines) rows.push(`${l.quantity} x ${l.name}  ${fmt(l.quantity * l.unit_price)}`);
  rows.push('', `${w.total}  ${fmt(sale.total)}`, w.cash, '', w.noNivfSq);
  if (w.noNivf && w.noNivf !== w.noNivfSq) rows.push(w.noNivf);
  rows.push(`${w.ref} ${String(sale.sale_key).slice(0, 8)}`);
  return rows;
}
