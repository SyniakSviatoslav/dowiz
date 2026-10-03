// THE OFFLINE CASH SALE's three screens (W-OFFSALE, row OF3): pick dishes
// from the menu this tablet saved, confirm the cash, print the receipt.
//
// THE SALE IS DECIDED HERE, ON THE TABLET (`offline-sale.js`, the pricer's
// rule pinned by one fixture), kept in this tablet's journal (`sales-db.js`)
// BEFORE the receipt is shown, and sent under the key minted at the tap:
// now when the server answers, through the outbox when the network does not
// carry it. The server records it once (`offline:<key>`) with its own time
// and queues its fiscal document with 48 h from the sale.
//
// Offered only while the room is offline (`app.js`): online, a sale is a table
// opened and paid, as before. CASH ONLY: a card needs the network.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES 11).
import './sale-i18n.js';
import { priceOffline, productsOf, basketLines, saleBody, receiptLines } from './offline-sale.js';
import { renderAdd, bindAdd } from './menu.js';
import { money, ageOf } from './logic.js';
import { ui, k, act, backBar, amt } from './parts.js';
import { API, api, OUT } from './net.js';
import { newKey } from '../lib/outbox.js';
import { keepSale, allSales, markSale, pruneSynced } from './sales-db.js';

export const PATH = '/staff/offline_sales';
/// The outbox tag of a sale: `sale:<key>`.
export const TAG = 'sale:';
/// A synced sale stays in the journal this long, then the server alone has it.
const KEEP_SYNCED_MS = 2 * 24 * 3600 * 1000;

const ageText = (t, ms) => { const a = ageOf(ms); return t('age' + a.unit.toUpperCase()).replace('{n}', a.n); };

// ── 1. the picker ──────────────────────────────────────────────────────────

export function renderSell(c) {
  const { S, t } = c;
  if (!S.menu || !S.currency) return `${backBar()}${ui.emptyState({ icon: 'plug-connected-x', title: k('saleNoMenu'), alert: true })}`;
  const lead = `${ui.alert({ tone: 'warning', icon: 'cash', label: t('cashOnly') })}
    ${ui.para(t('sellHint') + (S.menuAt ? ' ' + t('menuCachedAt').replace('{age}', ageText(t, Date.now() - S.menuAt)) : ''), { hint: true })}
    ${S.unsentSales ? ui.para(t('saleUnsent').replace('{n}', S.unsentSales), { hint: true }) : ''}`;
  return renderAdd(c, 'sellTitle', lead, 'sellN');
}

export function bindSell(c, root) {
  const { S, t } = c;
  bindAdd(c, root, null, async (_c, _round, ops) => {
    const priced = priceOffline(productsOf(S.menu), basketLines(Object.fromEntries(ops.map(o => [o.product_id, o.quantity]))));
    if (!priced.ok) { c.toast(t('refusal_' + priced.refusal)); return false; }
    // The server records a sale that took money (`rules::check`): nothing to sell is said here, not refused there.
    if (!(priced.total > 0)) { c.toast(t('saleZero')); return false; }
    S.sellDraft = priced; S.view = 'sellConfirm';
    return false;
  });
  const pick = root.onclick;
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (b && b.dataset.act === 'back') { S.basket = {}; S.view = 'room'; return c.render(); }
    return pick(ev);
  };
}

// ── 2. the confirmation ────────────────────────────────────────────────────

export function renderSellConfirm(c) {
  const { S, t } = c;
  const p = S.sellDraft, loc = c.locale();
  if (!p) { S.view = 'sell'; return renderSell(c); }
  const rows = p.lines.map(l => `<li class="sale-line"><span>${l.quantity} × ${ui.esc(l.name)}</span>${amt(money(l.quantity * l.unit_price, S.currency, loc))}</li>`).join('');
  return `${backBar()}
    <h2>${ui.esc(t('sellConfirm'))}</h2>
    <ul class="sale-lines" data-tour="sale.lines">${rows}</ul>
    ${ui.stat({ label: k('saleTotal'), value: amt(money(p.total, S.currency, loc), { size: 'xl', strong: true }), emphasis: true, attrs: { data: { tour: 'sale.total' } } })}
    ${ui.alert({ tone: 'warning', icon: 'cash', label: t('cashOnly') })}
    ${ui.button({ variant: 'primary', size: 'lg', block: true, icon: 'cash', label: k('cashTaken'), attrs: act('sellCommit', {}, 'sale.commit') })}`;
}

export function bindSellConfirm(c, root) {
  const { S, t } = c;
  root.onclick = async ev => {
    const b = ev.target.closest('[data-act]');
    if (!b) return;
    if (b.dataset.act === 'back') { S.view = 'sell'; return c.render(); }
    if (b.dataset.act !== 'sellCommit' || b.disabled) return;
    b.disabled = true;
    const body = saleBody({ loc: S.loc, key: newKey(), soldAt: Date.now(), currency: S.currency, priced: S.sellDraft, menuVersion: S.menuVersion });
    // KEPT BEFORE THE RECEIPT: a sale this tablet cannot remember is not made.
    if (!(await keepSale({ ...body, status: 'queued', said: '' }))) { b.disabled = false; return c.toast(t('saleNoStore')); }
    Object.assign(S, { lastSale: body, sellDraft: null, basket: {}, view: 'receipt' });
    c.render();
    await send(c, body);
  };
}

// ── 3. the receipt ─────────────────────────────────────────────────────────

const receiptWords = (c) => ({ title: c.S.venueName || 'dowiz', total: c.t('saleTotal'), cash: c.t('saleCash'), ref: c.t('saleRef'),
  noNivf: c.t('saleNoNivf'), noNivfSq: c.t('saleNoNivfSq') });

export function renderReceipt(c) {
  const { S, t } = c;
  const s = S.lastSale;
  if (!s) { S.view = 'room'; return ''; }
  const loc = c.locale();
  const at = ms => new Date(ms).toLocaleString(loc);
  const lines = receiptLines(s, receiptWords(c), a => money(a, s.currency, loc), at);
  return `<h2>${ui.esc(t('receipt'))}</h2>
    <pre class="receipt" data-tour="sale.receipt">${ui.esc(lines.join('\n'))}</pre>
    <div class="acts">${ui.button({ icon: 'receipt', label: k('print'), attrs: act('salePrint', {}, 'sale.print') })}
      ${ui.button({ variant: 'primary', icon: 'check', label: k('done'), attrs: act('saleDone', {}, 'sale.done') })}</div>`;
}

export function bindReceipt(c, root) {
  const { S } = c;
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (!b) return;
    if (b.dataset.act === 'salePrint') { try { window.print(); } catch {} return; }
    if (b.dataset.act === 'saleDone') { S.lastSale = null; S.view = 'room'; return c.render(); }
  };
}

// ── the wire ───────────────────────────────────────────────────────────────

/// A status the server may answer differently later: queue, do not give up.
const transient = s => s === 401 || s === 408 || s === 425 || s === 429 || s >= 500;

/// Send one sale under ITS key: now, or through the outbox.
export async function send(c, body) {
  const { t } = c;
  try {
    await api(PATH, { method: 'POST', body, headers: { 'idempotency-key': body.sale_key } });
    await markSale(body.sale_key, 'synced');
    c.toast(t('saleSynced'));
  } catch (e) {
    if (e.offline || transient(e.status)) {
      // THE JOURNAL ALREADY HOLDS IT (kept before the receipt). An outbox that is
      // full is not a lost sale: `resync` queues it again when one drains.
      await OUT.queue(API + PATH, { body: JSON.stringify(body), tag: TAG + body.sale_key, key: body.sale_key });
      c.toast(t('saleQueued'));
    } else {
      await markSale(body.sale_key, 'refused', String(e.message || e.status));
      c.toast(t('saleRefused'));
    }
  }
  await count(c);
}

/// The journal's unsent sales, counted onto `S.unsentSales`.
export async function count(c) {
  c.S.unsentSales = (await allSales()).filter(s => s.status === 'queued' && s.location_id === c.S.loc).length;
  return c.S.unsentSales;
}

/// QUEUE AGAIN what the outbox lost (signed out, cleared, a 401 drop) under
/// the SAME key, and forget synced sales older than two days. Run at start
/// and after a sign-in.
export async function resync(c) {
  await pruneSynced(Date.now() - KEEP_SYNCED_MS);
  const waiting = new Set((await OUT.pending()).map(e => e.key));
  for (const s of await allSales()) {
    if (s.status !== 'queued' || s.location_id !== c.S.loc || waiting.has(s.sale_key)) continue;
    const { status: _s, said: _w, ...body } = s;
    await OUT.queue(API + PATH, { body: JSON.stringify(body), tag: TAG + s.sale_key, key: s.sale_key });
  }
  await count(c);
}

/// The outbox told the screen: a sale landed, or the server answered no.
export async function settled(c, entry, ok, status, detail) {
  if (!entry?.tag?.startsWith(TAG)) return false;
  const key = entry.tag.slice(TAG.length);
  if (ok) await markSale(key, 'synced');
  else if (!transient(status)) await markSale(key, 'refused', String(detail || status));
  // A slot came free: a sale the full outbox could not take goes in now.
  if (ok) await resync(c); else await count(c);
  c.toast(c.t(ok ? 'saleSynced' : transient(status) ? 'saleQueued' : 'saleRefused'));
  return true;
}
