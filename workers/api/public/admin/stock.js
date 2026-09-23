// The shelf -- what the kitchen has, as a fold over what happened to it.
//
// A level is never a number somebody edits: it is received, wasted, counted,
// and reserved or consumed by orders. The screen shows each supply with its
// available quantity and a bar against its low mark, and a tap opens the
// three honest movements. A supply the venue tracks is a supply an order is
// refused for when it runs out -- which is the whole point.

import { $, $$, esc, icon, t, api, post, withLoc, toast, sheet, closeSheet, busy, money, switchEl, confirm, retranslate, hydrate } from '/admin/core.js';
import { lang } from '/admin/i18n.js';
import { rerender } from '/admin/app.js';

/// The bar reads full at this many times the low mark.
const FULL_AT_LOW_MULTIPLE = 4;
/// The movements an owner can make by hand.
const MOVES = [['received', 'received', 'download'], ['wasted', 'wasted', 'trash'], ['stocktake', 'counted', 'check']];
/// The reasons a kitchen writes off stock, as the hub's vocabulary spells them.
const WASTE_REASONS = ['spoiled', 'dropped', 'unsold', 'returned', 'staff_meal'];

let stock = null;

/// The four kinds a supply can be, with the old console's icons.
export const KINDS = [['food_ingredient', 'meat'], ['condiment', 'bottle'], ['packaging', 'box'], ['utensil', 'tool']];
export const UNITS = ['g', 'ml', 'unit'];
const isFood = k => k === 'food_ingredient' || k === 'condiment';
/// Nutrition and cost are per 100 g/ml, per ONE piece.
export const basisOf = u => u === 'unit' ? 1 : 100;
const view = { kind: 'all', q: '', sort: 'name' };
const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabStock"></p><h1 data-t="supplies"></h1></div>
    <button type="button" class="act pri" id="addSupply">${icon('plus')}<span data-t="addSupply"></span></button></div>
    <p class="screen-hint" data-t="stockScreenHint"></p>
    <label class="srch">${icon('search')}<input id="sq" type="search" value="${esc(view.q)}" data-t-attr="placeholder:search"></label>
    <div class="chips filters"><button type="button" class="chip ${view.kind === 'all' ? 'on' : ''}" data-k="all"><span data-t="all"></span></button>
      ${KINDS.map(([k, ic]) => `<button type="button" class="chip ${view.kind === k ? 'on' : ''}" data-k="${k}">${icon(ic)}<span data-t="kind_${k}"></span></button>`).join('')}
      <select class="chip sortsel" id="sSort">${['name', 'category', 'low'].map(k => `<option value="${k}" ${view.sort === k ? 'selected' : ''}>${esc(t('ssort_' + k))}</option>`).join('')}</select></div>
    <div id="stockList"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>`;
  $('#addSupply', host).onclick = () => openSupply(null);
  $('#sSort', host).onchange = e => { view.sort = e.target.value; rerender(); };
  $('#sq', host).oninput = e => { view.q = e.target.value; drawList(host); };
  try { stock = await api('/owner/stock'); } catch (e) { $('#stockList', host).innerHTML = `<div class="empty">${icon('alert-triangle')}<b>${esc(t('loadFail'))}</b><span class="muted small">${esc(e.message || e)}</span></div>`; return; }
  drawList(host);
  host.onclick = e => {
    const k = e.target.closest('[data-k]'); if (k) { view.kind = k.dataset.k; return rerender(); }
    const r = e.target.closest('[data-s]'); if (r) openMove(r.dataset.s);
  };
}

function drawList(host){
  const all = stock?.supplies || [];
  const lowN = all.filter(s => s.low || (s.available || 0) <= 0).length;
  const h1 = $('.screen-h', host); if (h1 && lowN && !$('.pill.warn', h1)) h1.insertAdjacentHTML('beforeend', `<span class="pill warn">${lowN} <span data-t="low"></span></span>`);
  const q = norm(view.q).trim();
  let list = all.filter(s => (view.kind === 'all' || s.kind === view.kind) && (!q || norm(`${s.name} ${s.category} ${s.id}`).includes(q)));
  if (view.sort === 'name') list = [...list].sort((a, b) => String(a.name).localeCompare(String(b.name), lang));
  else if (view.sort === 'category') list = [...list].sort((a, b) => String(a.category).localeCompare(String(b.category), lang) || String(a.name).localeCompare(String(b.name), lang));
  else list = [...list].sort((a, b) => Number(!!b.low) - Number(!!a.low) || (a.available || 0) - (b.available || 0));
  const counts = KINDS.map(([k]) => all.filter(s => s.kind === k).length);
  const row = sup => {
    const low = sup.lowAt || 0;
    const pct = low ? Math.min(100, Math.round(100 * (sup.available || 0) / (low * FULL_AT_LOW_MULTIPLE))) : 100;
    const state = (sup.available || 0) <= 0 ? 'bad' : sup.low ? 'warn' : 'ok';
    const ic = (KINDS.find(([k]) => k === sup.kind) || KINDS[0])[1];
    const per = `/${basisOf(sup.unit) === 1 ? '' : '100'}${sup.unit}`;
    const facts = [sup.category, isFood(sup.kind) && sup.kcalPer100 != null ? `${sup.kcalPer100} kcal${per}` : '', sup.costPerBasis != null ? `${money(sup.costPerBasis)}${per}` : ''].filter(Boolean).join(' · ');
    return `<button type="button" class="rowc" data-s="${esc(sup.id)}">${icon(ic)}
      <span class="t"><b>${esc(sup.name || sup.id)}${isFood(sup.kind) && !sup.nutritionConfirmed ? ` <span class="pill warn" data-t="unconfirmed"></span>` : ''}</b><small class="mono">${esc(facts)}</small>
        <small class="mono">${sup.available ?? 0} ${esc(sup.unit || '')}${sup.reserved ? ` · ${t('reserved')} ${sup.reserved}` : ''}${low ? ` · ${t('minLevel')} ${low}` : ''}</small>
        <span class="gauge"><i class="${state === 'ok' ? '' : state}" data-w="${pct}"></i></span></span>
      <span class="pill ${state}" data-t="${state === 'bad' ? 'out' : state === 'warn' ? 'low' : 'onSale'}"></span></button>`; };
  let html = '';
  if (view.sort === 'category') {
    const groups = [...new Set(list.map(s => s.category || ''))];
    html = groups.map(g => `<section class="group"><p class="eyebrow">${esc(g || t('none'))}</p><div class="rows">${list.filter(s => (s.category || '') === g).map(row).join('')}</div></section>`).join('');
  } else html = `<div class="rows">${list.map(row).join('')}</div>`;
  $('#stockList', host).innerHTML = (all.length ? `<p class="hint mono">${KINDS.map(([k], i) => `${counts[i]} ${t('kind_' + k).toLowerCase()}`).join(' · ')}</p>` : '') +
    (list.length ? html : `<div class="empty">${icon('bento')}<b data-t="${all.length ? 'none' : 'noStock'}"></b><span class="muted small" data-t="stockHint"></span></div>`) +
    (stock.stranded?.length ? `<section class="group mt-3"><p class="eyebrow" data-t="stranded"></p><p class="muted small" data-t="strandedHint"></p><div class="rows">${stock.stranded.map(s => `<div class="rowc off">${icon('alert-triangle')}<span class="t"><b>${esc(s.item)} × ${s.qty}</b><small class="mono">#${esc(String(s.order).slice(0, 8))}</small></span></div>`).join('')}</div></section>` : '');
  retranslate($('#stockList', host)); hydrate($('#stockList', host));
}

/// The supply as the old service knew it, plus a cost and a weight per piece.
function openSupply(sup){
  const kind = sup?.kind || KINDS[0][0], unit = sup?.unit || 'g';
  const cats = [...new Set((stock?.supplies || []).map(s => s.category).filter(Boolean))];
  sheet(`<p class="eyebrow" data-t="supplies"></p><h2 data-t="${sup ? 'edit' : 'addSupply'}"></h2>
    <label for="s-name" data-t="name"></label><input id="s-name" value="${esc(sup?.name || '')}" autocomplete="off">
    <label for="s-id" data-t="ingredient"></label><input id="s-id" value="${esc(sup?.id || '')}" ${sup ? 'readonly' : ''} placeholder="salmon"><p class="hint" data-t="supplyIdHint"></p>
    <label data-t="kind"></label><div class="seg" id="sKind">${KINDS.map(([k, ic]) => `<button type="button" class="seg-b ${k === kind ? 'on' : ''}" data-kind="${k}">${icon(ic)}<span data-t="kind_${k}"></span></button>`).join('')}</div>
    <div class="grid2"><div><label for="s-cat" data-t="category"></label><input id="s-cat" list="catList" value="${esc(sup?.category || '')}" autocomplete="off"><datalist id="catList">${cats.map(c => `<option value="${esc(c)}">`).join('')}</datalist></div>
      <div><label for="s-unit" data-t="unit"></label><select id="s-unit">${UNITS.map(u => `<option value="${u}" ${u === unit ? 'selected' : ''}>${u}</option>`).join('')}</select></div></div>
    <div id="sFood" ${isFood(kind) ? '' : 'hidden'}>
      <p class="eyebrow mt-3"><span data-t="nutritionPer"></span> <span id="sBasis">${basisOf(unit) === 1 ? '1' : '100'} ${esc(unit)}</span></p>
      <div class="grid2"><div><label for="s-kcal" data-t="kcal"></label><input id="s-kcal" inputmode="decimal" value="${sup?.kcalPer100 ?? ''}"></div><div><label for="s-prot" data-t="protein"></label><input id="s-prot" inputmode="decimal" value="${sup?.proteinPer100 ?? ''}"></div></div>
      <div class="grid2"><div><label for="s-fat" data-t="fat"></label><input id="s-fat" inputmode="decimal" value="${sup?.fatPer100 ?? ''}"></div><div><label for="s-carb" data-t="carbs"></label><input id="s-carb" inputmode="decimal" value="${sup?.carbsPer100 ?? ''}"></div></div>
      ${switchEl('s-conf', !!sup?.nutritionConfirmed, 'nutritionConfirmed', 'nutritionConfirmedHint')}
    </div>
    <div class="grid2"><div><label for="s-cost"><span data-t="costPer"></span> <span class="sBasis">${basisOf(unit) === 1 ? '1' : '100'} ${esc(unit)}</span></label><input id="s-cost" inputmode="numeric" value="${sup?.costPerBasis ?? ''}"></div>
      <div id="sWeight" ${unit === 'unit' ? '' : 'hidden'}><label for="s-wpu" data-t="weightPerUnit"></label><input id="s-wpu" inputmode="decimal" value="${sup?.weightPerUnit ?? ''}"></div></div>
    <label for="s-low" data-t="minLevel"></label><input id="s-low" inputmode="numeric" value="${sup?.lowAt ?? ''}">
    <div class="btn-row"><button class="btn" id="sSave">${icon('check')}<span data-t="save"></span></button></div>
    ${sup ? `<div class="btn-row"><button class="btn danger" id="sRetire">${icon('trash')}<span data-t="retireSupply"></span></button></div>` : ''}`, { name: 'supply' });
  let k = kind;
  for (const b of $$('[data-kind]', $('#sheetIn'))) b.onclick = () => { k = b.dataset.kind; for (const x of $$('[data-kind]', $('#sheetIn'))) x.classList.toggle('on', x === b); $('#sFood').hidden = !isFood(k); };
  $('#s-unit').onchange = e => { const u = e.target.value; $('#sWeight').hidden = u !== 'unit'; for (const el of $$('#sBasis, .sBasis')) el.textContent = `${basisOf(u) === 1 ? '1' : '100'} ${u}`; };
  if (!sup) $('#s-name').oninput = e => { $('#s-id').value = e.target.value.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '').replace(/[^a-z0-9а-яіїєґ]+/gi, '-').replace(/^-|-$/g, ''); };
  const num = v => { const s = String(v ?? '').trim(); if (!s) return null; const n = Number(s.replace(',', '.')); return Number.isFinite(n) ? n : null; };
  $('#sSave').onclick = async () => {
    const id = $('#s-id').value.trim().toLowerCase().replace(/\s+/g, '-'); if (!id) return toast(t('required'));
    const body = { id, name: $('#s-name').value.trim() || id, unit: $('#s-unit').value, kind: k, category: $('#s-cat').value.trim() };
    const low = num($('#s-low').value); if (low != null) body.lowAt = Math.round(low);
    if (isFood(k)) { for (const [f, key] of [['s-kcal', 'kcalPer100'], ['s-prot', 'proteinPer100'], ['s-fat', 'fatPer100'], ['s-carb', 'carbsPer100']]) { const v = num($('#' + f).value); if (v != null) body[key] = v; } body.nutritionConfirmed = $('#s-conf').checked; }
    const cost = num($('#s-cost').value); if (cost != null) body.costPerBasis = Math.round(cost);
    const wpu = num($('#s-wpu').value); if (wpu != null) body.weightPerUnit = wpu;
    try { await busy($('#sSave'), () => post('/owner/supplies', body)); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
  const ret = $('#sRetire'); if (ret) ret.onclick = async () => { const ok = await confirm(t('retireSupply'), t('retireSupplyHint'), { danger: true }); if (!ok) return openSupply(sup); try { await post(`/owner/supplies/${encodeURIComponent(sup.id)}/retire`, withLoc()); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); } };
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
