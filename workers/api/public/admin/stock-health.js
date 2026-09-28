// THE STOCK CHECK: the owner's view of the rebuild report (`GET /api/owner/health`
// -> `rebuild`, src/rebuild.rs). The hub refolds every order from its log and
// crosses it against the stock ledger; an ingredient still RESERVED for an order
// that is over is a "stranded hold" -- the shelf shows less than the kitchen
// has. Until this screen the report was JSON nobody saw.
//
// JOINED WITH THE STOCK ANSWER. The rebuild names only the orders
// (`rebuild.stranded`: ids; `rebuild.archived`: {order, archive, status} for the
// ones an archive holds -- OPTIONAL, older hubs do not send it). The
// ingredient and the grams come from `GET /api/owner/stock` -> `stranded`,
// which lists EVERY open reservation, live orders' included; only the orders
// the rebuild names are wrong.
//
// NO RELEASE BUTTON: the hub has no route that releases a stranded hold, and a
// stocktake keeps reservations (dowiz-hub stock.rs,
// `a_stocktake_resets_the_basis_and_keeps_promises`). The screen says so rather
// than offer an action that does nothing. "Check again" is the one action.
//
// PURE apart from `drawHealth`, whose `api` is handed in: node renders every
// state (stock-health.test.mjs). ASCII QUOTES ONLY in this file.

import { ui, btn, rowDiv, empty } from './parts.js';

const esc = ui.esc;
const icon = name => `<i class="ti ti-${esc(name)}" aria-hidden="true"></i>`;
const list = v => (Array.isArray(v) ? v : []);

/// What the two answers say together.
///   state: 'ok' | 'stranded' | 'error'
///   orders: [{ order, archive, status, holds: [{ item, name, qty, unit }] }]
///   live: open reservations that belong to live orders (correct holds)
export function findings(health, stock){
  const r = health && health.rebuild;
  if (!r || typeof r !== 'object') return { state: 'error', error: 'rebuild' };
  if (r.error) return { state: 'error', error: String(r.error) };
  const supplies = new Map(list(stock && stock.supplies).map(s => [s.id, s]));
  const holds = list(stock && stock.stranded);
  const ids = list(r.stranded).map(String);
  const archived = new Map(list(r.archived).map(a => [String(a.order), a]));
  const orders = ids.map(id => {
    const a = archived.get(id);
    return {
      order: id, archive: a ? String(a.archive || '') : '', status: a ? String(a.status || '') : '',
      holds: holds.filter(h => String(h.order) === id).map(h => {
        const s = supplies.get(h.item);
        return { item: h.item, name: (s && s.name) || h.item, qty: h.qty, unit: (s && s.unit) || 'g' };
      }),
    };
  });
  return {
    state: orders.length ? 'stranded' : 'ok', orders,
    live: holds.filter(h => !ids.includes(String(h.order))).length,
    checked: Number(r.orders) || 0, stale: list(r.stale).length,
  };
}

/// One stranded hold: what, how much, for which order, and where that order ended.
function holdRow(o, h, t, st){
  const where = o.archive
    ? ` · ${esc(t('sh_archive'))} <span class="mono">${esc(o.archive)}</span>${o.status ? ` · ${esc(t('sh_final'))}: ${esc(st(o.status))}` : ''}`
    : ` · ${esc(t('sh_notArchived'))}`;
  return rowDiv({
    cls: 'off', leading: icon('alert-triangle'),
    title: h ? `${h.name} · ${h.qty} ${h.unit}` : t('sh_noHolds'),
    // ONE span: the row's sub stacks its children, and this is one line of facts.
    sub: `<span>${esc(t('sh_order'))} <span class="mono">#${esc(o.order.slice(0, 8))}</span>${where}</span>`,
    data: { shOrder: o.order },
  });
}

/// The section's inside for a finding (`null` = not checked yet). `st` is the
/// console's status word (admin/i18n.js `st`: DELIVERED -> "Delivered").
export function markup(f, t, st = s => s){
  const again = btn({ id: 'shAgain', icon: 'refresh', label: t(f ? 'sh_again' : 'sh_run'), data: { shAgain: '1' } });
  const head = `<p class="eyebrow">${esc(t('sh_title'))}</p>`;
  if (!f) return `${head}<p class="muted small">${esc(t('sh_hint'))}</p><div class="btn-row">${again}</div>`;
  if (f.state === 'error') {
    return `${head}${empty('alert-triangle', { title: t('sh_error'), body: f.error, alert: true })}<div class="btn-row">${again}</div>`;
  }
  const facts = [`${f.checked} ${t('sh_checked')}`, `${f.live} ${t('sh_live')}`].join(' · ');
  const stale = f.stale ? `<p class="warn small">${esc(`${f.stale} ${t('sh_stale')}`)}</p>` : '';
  if (f.state === 'ok') {
    return `${head}<div class="rows">${rowDiv({ leading: icon('check'), title: t('sh_ok'), sub: esc(facts), data: { shState: 'ok' } })}</div>${stale}<div class="btn-row">${again}</div>`;
  }
  const rows = f.orders.flatMap(o => (o.holds.length ? o.holds.map(h => holdRow(o, h, t, st)) : [holdRow(o, null, t, st)])).join('');
  return `${head}<p class="warn small" data-sh-state="stranded">${esc(`${f.orders.length} ${t('sh_bad')}`)}</p>
    <p class="muted small">${esc(t('sh_badHint'))}</p><div class="rows">${rows}</div>
    <p class="hint mono">${esc(facts)}</p>${stale}<div class="btn-row">${again}</div>`;
}

/// Fill `el` with the check. `run` false draws the button only; the check
/// reads every image of the venue, so it runs when the owner asks. `stock` is
/// a getter: the stock screen's latest answer at the moment of the check.
export async function drawHealth(el, { api, stock, t, st, run = false }){
  if (!el) return;
  const wire = () => {
    const b = el.querySelector('[data-sh-again]');
    if (b) b.onclick = () => { b.disabled = true; return drawHealth(el, { api, stock, t, st, run: true }); };
  };
  if (!run) { el.innerHTML = markup(null, t); wire(); return; }
  let h;
  try { h = await api('/owner/health'); } catch (e) { h = { rebuild: { error: String((e && e.message) || e) } }; }
  el.innerHTML = markup(findings(h, stock()), t, st);
  wire();
}
