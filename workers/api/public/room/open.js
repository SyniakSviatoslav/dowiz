// OPENING A TABLE (2026-09-24). The room app could change, move and take
// money on a round, but nothing in it -- or anywhere else -- could START one:
// the storefront has no table ordering, so on a live venue the room was
// always empty and the tip at payment, the wallet tender and every void or
// comp were out of a waiter's reach. A round in the room is the storefront's
// one placement path with a room token (`services/orders/room/placer.rs`):
// `dine_in`, a table, and the hub mints the sitting and signs the round.
//
// The picker is `menu.js`'s own, drawn under a table field.
import { renderAdd, bindAdd } from './menu.js';
import { ui, k } from './parts.js';

/// A table as the ticket prints it: trimmed, bounded, never empty.
export const TABLE_MAX = 24;
export function tableOk(s) {
  const v = String(s ?? '').trim();
  return v.length > 0 && v.length <= TABLE_MAX;
}

/// The placement body: intents only, never a price (`handlers.rs` re-prices).
/// NO PAYMENT METHOD: a round is paid at the table, later, maybe by several
/// people in several ways; the method is each payment's (`/lib/paid-with.js`),
/// never a guess stamped at placement.
export function placeBody(table, ops) {
  return {
    items: ops.map(o => ({ product_id: o.product_id, quantity: o.quantity, modifier_ids: [] })),
    contact: { name: '', phone: '' },
    fulfilment: { kind: 'dine_in', table: String(table).trim() },
  };
}

export function renderOpen(c) {
  const { S } = c;
  const table = ui.field({ id: 'openTable', name: 'table', label: k('tableName'), value: S.openTable || '', maxlength: TABLE_MAX,
    autocomplete: 'off', required: true, cls: 'table-field', attrs: { data: { in: 'table', tour: 'open.table' } } });
  // The table goes under the picker's heading, above its search.
  return renderAdd(c, 'openTable', table);
}

export function bindOpen(c, root, back) {
  const { S, t } = c;
  const place = async (_c, _round, ops) => {
    if (!tableOk(S.openTable)) { c.toast(t('needTable')); return false; }
    try {
      const o = await c.api(`/public/locations/${encodeURIComponent(S.slug)}/orders`, { method: 'POST', body: placeBody(S.openTable, ops) });
      S.openTable = '';
      c.toast(t('saved'));
      await c.reload();
      // Straight onto the new round: it is the one the waiter is standing at.
      if (o?.sitting_id) { S.sittingId = o.sitting_id; S.roundId = o.id; }
      return true;
    } catch (e) { c.toast(e.message || t('error')); return false; }
  };
  bindAdd(c, root, null, place);
  const search = root.oninput;
  root.oninput = ev => {
    if (ev.target.dataset.in === 'table') { S.openTable = ev.target.value; return; }
    return search(ev);
  };
  const pick = root.onclick;
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (b && b.dataset.act === 'back') { S.basket = {}; return back(); }
    return pick(ev);
  };
}
