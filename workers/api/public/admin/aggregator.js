// An aggregator order typed in from the platform's tablet (BLUEPRINT-
// OPERATIONAL-BLIND-SPOTS §2.9, no partner API): which platform, its order
// number, the dishes at the prices the platform charged, its discount, and
// the total it shows. The hub refuses a total that is not lines - discount
// (conservation law 3); the form says so live, before anyone presses send.
//
// ONE IDEMPOTENCY KEY PER OPENING of this sheet, so a double tap or a retry
// after a lost answer is the same entry; the platform's number is the second
// guard, on the hub (entering it twice returns the first order).

import { $, $$, esc, icon, t, S, api, withLoc, toast, sheet, closeSheet, money, busy } from '/admin/core.js';
import { newKey } from '/lib/outbox.js';
import { loadOrders, rerender } from '/admin/app.js';

const PLATFORMS = ['wolt', 'glovo', 'baboon'];
const QTY_MAX = 99;

/// Whole minor units from an input, or null. Money here is the hub's integer.
const whole = v => { const s = String(v ?? '').trim(); return /^[0-9]+$/.test(s) ? parseInt(s, 10) : null; };

export function openAggregator(){
  const key = newKey();
  const lines = [];
  const dish = id => S.products.find(p => p.id === id);
  const options = S.products.map(p => `<option value="${esc(p.id)}">${esc(p.name)}</option>`).join('');
  sheet(`<h2 data-t="aggTitle"></h2>
    <label for="ag-ch" data-t="aggPlatform"></label>
    <select id="ag-ch">${PLATFORMS.map(p => `<option value="${p}">${p[0].toUpperCase() + p.slice(1)}</option>`).join('')}</select>
    <label for="ag-x" data-t="aggNumber"></label>
    <input id="ag-x" maxlength="64" autocomplete="off" pattern="[A-Za-z0-9_-]+">
    <p class="eyebrow mt-3" data-t="items"></p>
    <div id="ag-lines"></div>
    <div class="btn-row"><select id="ag-dish">${options}</select>
      <button class="btn ghost" id="ag-add" type="button">${icon('plus')}<span data-t="aggAdd"></span></button></div>
    <label for="ag-disc" data-t="discount"></label>
    <input id="ag-disc" inputmode="numeric" value="0">
    <label for="ag-total" data-t="total"></label>
    <input id="ag-total" inputmode="numeric">
    <p id="ag-check" class="muted" role="status"></p>
    <div class="btn-row"><button class="btn" id="ag-go" type="button" disabled>${icon('check')}<span data-t="aggEnter"></span></button></div>`,
    { name: 'aggregator' });

  const draw = () => {
    $('#ag-lines').innerHTML = lines.map((l, i) => `<div class="line">
      <span class="n">${esc(dish(l.product_id)?.name || l.product_id)}</span>
      <input class="q" data-q="${i}" inputmode="numeric" value="${l.quantity}" aria-label="${esc(t('aggQty'))}">
      <input class="p" data-p="${i}" inputmode="numeric" value="${l.unit_price}" aria-label="${esc(t('aggPrice'))}">
      <button class="act danger" data-rm="${i}" type="button" aria-label="${esc(t('aggRemove'))}">${icon('x')}</button></div>`).join('');
    check();
  };
  // THE LAW, LIVE: what the hub will check, said before the tap.
  const check = () => {
    const ok = lines.length > 0 && lines.every(l => l.quantity >= 1 && l.quantity <= QTY_MAX && l.unit_price != null);
    const sum = lines.reduce((n, l) => n + (l.quantity || 0) * (l.unit_price || 0), 0);
    const disc = whole($('#ag-disc').value), total = whole($('#ag-total').value);
    const x = $('#ag-x').value.trim();
    const adds = ok && disc != null && disc <= sum && total === sum - disc;
    $('#ag-check').textContent = `${t('aggLines')} ${money(sum)} - ${t('discount')} ${money(disc ?? 0)} = ${money(sum - (disc ?? 0))}`
      + (total == null ? '' : adds ? ` · ${t('aggAddsUp')}` : ` · ${t('aggNotTotal')} ${money(total)}`);
    $('#ag-check').className = total != null && !adds ? 'err' : 'muted';
    $('#ag-go').disabled = !(adds && /^[A-Za-z0-9_-]{1,64}$/.test(x));
  };

  $('#ag-add').onclick = () => {
    const p = dish($('#ag-dish').value); if (!p) return;
    lines.push({ product_id: p.id, quantity: 1, unit_price: p.price ?? 0 });
    draw();
  };
  $('#sheetIn').oninput = e => {
    const q = e.target.dataset?.q, p = e.target.dataset?.p;
    if (q != null) lines[q].quantity = whole(e.target.value) ?? 0;
    if (p != null) lines[p].unit_price = whole(e.target.value);
    check();
  };
  $('#sheetIn').onclick = e => {
    const rm = e.target.closest('[data-rm]'); if (!rm) return;
    lines.splice(Number(rm.dataset.rm), 1); draw();
  };
  $('#ag-go').onclick = async () => {
    const body = withLoc({
      channel: $('#ag-ch').value, external_id: $('#ag-x').value.trim(),
      lines: lines.map(l => ({ product_id: l.product_id, quantity: l.quantity, unit_price: l.unit_price })),
      discount: whole($('#ag-disc').value) ?? 0, total: whole($('#ag-total').value),
    });
    try {
      const r = await busy($('#ag-go'), () => api('/staff/orders/aggregator',
        { method: 'POST', body, headers: { 'idempotency-key': key } }));
      toast(t(r?.existing ? 'aggAlready' : 'saved'));
      closeSheet(); await loadOrders(); await rerender();
    } catch (e) { toast(String(e.message || e)); }
  };
  draw();
}
