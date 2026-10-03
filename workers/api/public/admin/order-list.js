// THE ORDER LIST (W-STOCK P5): what to buy, grouped by supplier, sent from
// the phone. Every number comes from `GET /api/owner/stock` `orderList`
// (`stock/order_list.rs`): the average daily use of the last two weeks, the
// supplier's lead time and delivery days, what is free and what is on its
// way, rounded up to the pack. The owner edits a quantity, taps Send: the
// phone's Share sheet (navigator.share), else the text is copied -- in the
// SUPPLIER's language -- and the order is marked as on its way:
//   POST /api/owner/stock/ordered  {card: {supplier, lines: [{item, qty}]}}
// so the next list subtracts it until the delivery is recorded.

import { $, $$, esc, t, api, post, sheet, busy, toast, S, lang } from '/admin/core.js';
import { T } from '/admin/i18n.js';
import { ui, btn, pill, rowDiv, empty, loading } from '/admin/parts.js';
import * as O from '/admin/order-list-logic.js';

const fail = e => toast(String(e.message || e));
let showAll = false;

/// A word in the supplier's language (theirs, else the owner's, else English).
const wordsIn = l => k => T[l]?.[k] ?? T[lang]?.[k] ?? T.en?.[k] ?? k;

function lineRow(l, gi, li){
  const facts = [`${t('ol_free')} ${l.available} ${l.unit}`, l.adu != null ? `${l.adu} ${l.unit} ${t('ol_adu')}` : '', `${t('ol_par')} ${l.par}`,
    l.onOrder ? `${t('ol_onOrder')} ${l.onOrder}` : '', l.pack ? `${l.pack.name}` : ''].filter(Boolean).join(' · ');
  return rowDiv({ title: l.name, sub: `<span class="mono">${esc(facts)}</span>${l.suggest == null ? ` ${pill('info', { key: 'ol_countFirst' })}` : ''}`,
    trailing: ui.inputRow({ label: l.name, attrs: { value: l.suggest ? String(l.suggest) : '', placeholder: l.unit, inputmode: 'numeric', autocomplete: 'off', data: { oq: `${gi}:${li}` } } }) });
}

export async function open(){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="ol_title"></h2><p class="sheet-hint" data-t="ol_hint"></p>
    <div class="btn-row">${btn({ id: 'olAll', variant: 'ghost', icon: 'eye', key: 'ol_showAll', pressed: showAll })}</div>
    <div id="olOut">${loading(3)}</div>`, { name: 'orderList' });
  $('#olAll').onclick = () => { showAll = !showAll; open(); };
  let stock;
  try { stock = await api('/owner/stock'); } catch (e) { $('#olOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const groups = (stock.orderList?.groups || []).map(g => ({ ...g, lines: g.lines.filter(l => showAll || l.suggest > 0 || l.onOrder > 0) })).filter(g => g.lines.length);
  if (!groups.length) { $('#olOut').innerHTML = empty('check', { key: 'ol_empty' }); return; }
  $('#olOut').innerHTML = groups.map((g, gi) => `<section class="group"><p class="eyebrow">${esc(g.supplier?.name || t('ol_noCard'))}${g.supplier?.phone ? ' · ' + esc(g.supplier.phone) : ''}</p>
    <div class="rows">${g.lines.map((l, li) => lineRow(l, gi, li)).join('')}</div>
    <div class="btn-row">${btn({ variant: 'primary', icon: 'send', key: 'ol_share', data: { og: String(gi) } })}${g.supplier ? '' : btn({ variant: 'ghost', icon: 'building', key: 'ol_makeCard', data: { stockx: 'suppliers' } })}</div></section>`).join('');
  $('#olOut').onclick = async e => {
    const b = e.target.closest('[data-og]'); if (!b) return;
    const gi = Number(b.dataset.og), g = groups[gi];
    const lines = g.lines.map((l, li) => ({ ...l, qty: Number.parseInt($(`[data-oq="${gi}:${li}"]`)?.value || '0', 10) || 0 }));
    const send = O.toSend(lines);
    if (!send.length) return toast(t('required'));
    const text = O.orderText({ venue: S.venue?.name, supplier: g.supplier?.name, lines }, wordsIn(g.supplier?.lang || lang));
    try {
      await busy(b, async () => {
        const how = await O.shareOrCopy(text, navigator);
        if (how === 'cancelled') return;
        if (how === 'failed') throw new Error(text);
        if (how === 'copied') toast(t('ol_copied'));
        // Sent: the list subtracts it until the delivery is recorded.
        if (g.supplier) { await post('/owner/stock/ordered', { card: { supplier: g.supplier.id, lines: send } }); toast(t('ol_sent')); open(); }
      });
    } catch (err) { fail(err); }
  };
  for (const x of $$('[data-oq]', $('#olOut'))) x.oninput = () => x.classList.toggle('bad', x.value.trim() !== '' && !/^\d+$/.test(x.value.trim()));
}
