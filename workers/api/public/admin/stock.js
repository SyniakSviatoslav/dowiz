// The shelf -- what the kitchen has, as a fold over what happened to it.
//
// A level is never a number somebody edits: it is received, wasted, counted,
// and reserved or consumed by orders. The screen shows each supply with its
// available quantity and a bar against its low mark, and a tap opens the
// three honest movements. A supply the venue tracks is a supply an order is
// refused for when it runs out -- which is the whole point.

import { $, $$, esc, icon, t, api, post, withLoc, toast, sheet, closeSheet, busy } from '/admin/core.js';
import { rerender } from '/admin/app.js';

/// The bar reads full at this many times the low mark.
const FULL_AT_LOW_MULTIPLE = 4;
/// The movements an owner can make by hand.
const MOVES = [['received', 'received', 'download'], ['wasted', 'wasted', 'trash'], ['stocktake', 'counted', 'check']];
/// The reasons a kitchen writes off stock, as the hub's vocabulary spells them.
const WASTE_REASONS = ['spoiled', 'dropped', 'unsold'];

let stock = null;

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabStock"></p><h1 data-t="supplies"></h1></div>
    <button type="button" class="act" id="addSupply">${icon('plus')}<span data-t="addSupply"></span></button></div>
    <p class="screen-hint" data-t="stockScreenHint"></p>
    <div id="stockList"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>`;
  $('#addSupply', host).onclick = () => openSupply(null);
  try { stock = await api('/owner/stock'); } catch (e) { $('#stockList', host).innerHTML = `<div class="empty">${icon('alert-triangle')}<b>${esc(t('loadFail'))}</b><span class="muted small">${esc(e.message || e)}</span></div>`; return; }
  const list = stock.supplies || [];
  const lowN = list.filter(s => s.low || (s.available || 0) <= 0).length;
  const h1 = $('.screen-h', host); if (h1 && lowN) h1.insertAdjacentHTML('beforeend', `<span class="pill warn">${lowN} <span data-t="low"></span></span>`);
  $('#stockList', host).innerHTML = list.length ? `<div class="rows">${list.map(sup => {
    const low = sup.lowAt || 0;
    const pct = low ? Math.min(100, Math.round(100 * (sup.available || 0) / (low * FULL_AT_LOW_MULTIPLE))) : 100;
    const state = (sup.available || 0) <= 0 ? 'bad' : sup.low ? 'warn' : 'ok';
    return `<button type="button" class="rowc" data-s="${esc(sup.id)}">${icon('bento')}
      <span class="t"><b>${esc(sup.name || sup.id)}</b><small class="mono">${sup.available ?? 0} ${esc(sup.unit || '')}${sup.reserved ? ` · ${t('reserved')} ${sup.reserved}` : ''}${sup.onHand != null ? ` · ${t('onShelf')} ${sup.onHand}` : ''}${low ? ` · ${t('minLevel')} ${low}` : ''}</small>
        <span class="gauge"><i class="${state === 'ok' ? '' : state}" data-w="${pct}"></i></span></span>
      <span class="pill ${state}" data-t="${state === 'bad' ? 'out' : state === 'warn' ? 'low' : 'onSale'}"></span></button>`; }).join('')}</div>
    ${stock.stranded?.length ? `<section class="group mt-3"><p class="eyebrow" data-t="stranded"></p><p class="muted small" data-t="strandedHint"></p><div class="rows">${stock.stranded.map(s => `<div class="rowc off">${icon('alert-triangle')}<span class="t"><b>${esc(s.item)} × ${s.qty}</b><small class="mono">#${esc(String(s.order).slice(0, 8))}</small></span></div>`).join('')}</div></section>` : ''}`
    : `<div class="empty">${icon('bento')}<b data-t="noStock"></b><span class="muted small" data-t="stockHint"></span></div>`;
  host.onclick = e => { const r = e.target.closest('[data-s]'); if (r) openMove(r.dataset.s); };
}

function openSupply(sup){
  sheet(`<p class="eyebrow" data-t="supplies"></p><h2 data-t="${sup ? 'edit' : 'addSupply'}"></h2>
    <label for="s-id" data-t="ingredient"></label><input id="s-id" value="${esc(sup?.id || '')}" ${sup ? 'readonly' : ''} placeholder="salmon">
    <label for="s-name" data-t="name"></label><input id="s-name" value="${esc(sup?.name || '')}">
    <div class="grid2"><div><label for="s-unit" data-t="unit"></label><input id="s-unit" value="${esc(sup?.unit || 'g')}"></div>
      <div><label for="s-low" data-t="minLevel"></label><input id="s-low" inputmode="numeric" value="${sup?.lowAt ?? ''}"></div></div>
    <div class="btn-row"><button class="btn" id="sSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'supply' });
  $('#sSave').onclick = async () => {
    const id = $('#s-id').value.trim().toLowerCase().replace(/\s+/g, '-'); if (!id) return toast(t('required'));
    const body = { id, name: $('#s-name').value.trim() || id, unit: $('#s-unit').value.trim() || 'g' };
    const low = Number($('#s-low').value); if (Number.isFinite(low) && $('#s-low').value.trim()) body.lowAt = low;
    try { await busy($('#sSave'), () => post('/owner/supplies', body)); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
}

function openMove(id){
  const sup = (stock?.supplies || []).find(s => s.id === id); if (!sup) return;
  sheet(`<p class="eyebrow" data-t="move"></p><h2>${esc(sup.name || sup.id)}</h2>
    <p class="mono muted">${sup.available ?? 0} ${esc(sup.unit || '')}</p>
    <div class="seg" id="kind">${MOVES.map(([k, key, ic], i) => `<button type="button" class="seg-b ${i === 0 ? 'on' : ''}" data-k="${k}">${icon(ic)}<span data-t="${key}"></span></button>`).join('')}</div>
    <label for="m-qty" data-t="level"></label><input id="m-qty" inputmode="decimal">
    <div id="mReasons" hidden><label data-t="reason"></label><div class="chips">${WASTE_REASONS.map(r => `<button type="button" class="chip" data-r="${r}" data-t="${r}"></button>`).join('')}</div></div>
    <label for="m-reason" data-t="reason"></label><input id="m-reason">
    <div class="btn-row"><button class="btn ghost" id="mEdit">${icon('tools-kitchen-2')}<span data-t="edit"></span></button><button class="btn" id="mGo">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'move' });
  let kind = MOVES[0][0];
  for (const b of $$('[data-k]', $('#sheetIn'))) b.onclick = () => { kind = b.dataset.k; for (const x of $$('[data-k]', $('#sheetIn'))) x.classList.toggle('on', x === b); $('#mReasons').hidden = kind !== 'wasted'; };
  for (const b of $$('[data-r]', $('#sheetIn'))) b.onclick = () => { $('#m-reason').value = b.dataset.r; for (const x of $$('[data-r]', $('#sheetIn'))) x.classList.toggle('on', x === b); };
  $('#mEdit').onclick = () => openSupply(sup);
  $('#mGo').onclick = async () => {
    const qty = Number($('#m-qty').value); if (!Number.isFinite(qty)) return toast(t('required'));
    const body = kind === 'stocktake' ? { item: id, observed: qty } : { item: id, qty };
    const reason = $('#m-reason').value.trim(); if (reason) body.reason = reason;
    try { await busy($('#mGo'), () => post(`/owner/stock/${kind}`, body)); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
}
