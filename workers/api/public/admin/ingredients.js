// ONE INGREDIENT: its card, its form, and the movements a person records on
// it -- a delivery with its paper (price, supplier, invoice, lot, date), a
// write-off with its reason and lot, a count, and prep weighed before and
// after. Every number drawn here is a field of `GET /api/owner/stock` (one
// fold of the log); every save is one `POST /api/owner/stock/:kind`.
//
// "максимально просто": big number fields in whole grams (1,5 kg is read as
// 1500 g), chips instead of menus, the four actions as big buttons on the card.

import { $, $$, esc, icon, t, post, withLoc, toast, sheet, closeSheet, busy, money, switchEl, confirm, day, clock } from '/admin/core.js';
import { ui, k as key, btn, field, input, select, chips, press, pill, rowBtn, rowDiv } from '/admin/parts.js';
import * as C from '/admin/ingredients-calc.js';
import { KINDS, cardMarkup } from '/admin/ingredients-view.js';

export { KINDS };
const isFood = k => k === 'food_ingredient' || k === 'condiment';
/// The movements an owner makes by hand, their words and icons.
const MOVES = [['received', 'received', 'download'], ['wasted', 'wasted', 'trash'], ['stocktake', 'counted', 'check']];
/// The reasons a kitchen writes off stock, as the hub spells them.
export const WASTE_REASONS = ['spoiled', 'dropped', 'unsold', 'returned', 'staff_meal'];
const fail = e => toast(String(e.message || e));
const perWord = u => t(u === 'unit' ? 'inv_perPiece' : u === 'ml' ? 'inv_perL' : 'inv_perKg');
const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '-');

/// The card: levels, the four actions, losses, lots, prices, yields, movements.
export function openCard(sup, ctx){
  if (!sup) return;
  sheet(cardMarkup(sup, { money, t, when, warnDays: ctx.data?.expiryWarnDays }), { name: 'supplyCard' });
  for (const b of $$('[data-cact]', $('#sheetIn'))) b.onclick = () => (b.dataset.cact === 'prep' ? openPrep(sup, ctx) : openMove(sup, b.dataset.cact, ctx));
  for (const b of $$('[data-adopt]', $('#sheetIn'))) b.onclick = () => adoptYield(sup, b.dataset.adopt, Number(b.dataset.pm), b, ctx);
  $('#cEdit').onclick = () => openSupply(sup, ctx);
}

/// "Use the measured yield as the default": one catalogue write on the supply.
async function adoptYield(sup, stage, pm, el, ctx){
  try {
    await busy(el, () => post('/owner/supplies', withLoc({ id: sup.id, [stage === 'clean' ? 'cleanPm' : 'cookPm']: pm })));
    toast(t('inv_adopted')); closeSheet(); ctx.reload();
  } catch (e) { fail(e); }
}

/// The supply as the old service knew it, plus its losses and shelf life.
export function openSupply(sup, ctx){
  const kind = sup?.kind || KINDS[0][0], unit = sup?.unit || 'g';
  const cats = [...new Set((ctx.data?.supplies || []).map(s => s.category).filter(Boolean))];
  const pctOf = pm => (pm == null ? '' : String(pm / 10));
  sheet(`<p class="eyebrow" data-t="supplies"></p><h2 data-t="${sup ? 'edit' : 'addSupply'}"></h2>
    ${field({ id: 's-name', key: 'name', value: sup?.name || '', autocomplete: 'off', tour: 'supply.name' })}
    ${field({ id: 's-id', key: 'ingredient', value: sup?.id || '', placeholder: 'salmon', hintKey: 'supplyIdHint', attrs: { readonly: !!sup }, tour: 'supply.id' })}
    <p class="ui-label" data-t="kind"></p>${chips({ id: 'sKind', values: KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic })), value: kind, attr: 'kind', labelKey: 'kind', tour: 'supply.kind' })}
    <div class="grid2"><div>${field({ id: 's-cat', key: 'category', value: sup?.category || '', autocomplete: 'off', attrs: { list: 'catList' }, tour: 'supply.category' })}<datalist id="catList">${cats.map(c => `<option value="${esc(c)}">`).join('')}</datalist></div>
      <div>${select({ id: 's-unit', key: 'unit', value: unit, options: ['g', 'ml', 'unit'].map(u => ({ value: u, label: u })), tour: 'supply.unit' })}</div></div>
    <div id="sFood" ${isFood(kind) ? '' : 'hidden'}>
      <p class="eyebrow mt-3"><span data-t="nutritionPer"></span> <span id="sBasis">${C.basisOf(unit) === 1 ? '1' : '100'} ${esc(unit)}</span></p>
      <div class="grid2"><div>${field({ id: 's-kcal', key: 'kcal', inputmode: 'decimal', value: sup?.kcalPer100 ?? '', tour: 'supply.kcal' })}</div><div>${field({ id: 's-prot', key: 'protein', inputmode: 'decimal', value: sup?.proteinPer100 ?? '' })}</div></div>
      <div class="grid2"><div>${field({ id: 's-fat', key: 'fat', inputmode: 'decimal', value: sup?.fatPer100 ?? '' })}</div><div>${field({ id: 's-carb', key: 'carbs', inputmode: 'decimal', value: sup?.carbsPer100 ?? '' })}</div></div>
      <p class="ui-label" data-t="inv_nutritionBasis"></p>${chips({ id: 'sNb', values: [{ value: 'raw', key: 'inv_basis_raw' }, { value: 'cooked', key: 'inv_basis_cooked' }], value: sup?.nutritionBasis || 'raw', attr: 'nb' })}
      ${switchEl('s-conf', !!sup?.nutritionConfirmed, 'nutritionConfirmed', 'nutritionConfirmedHint', 'supply.confirmed')}
    </div>
    <p class="eyebrow mt-3" data-t="inv_losses"></p><p class="hint" data-t="inv_lossesHint"></p>
    <div class="grid2"><div>${field({ id: 's-clean', key: 'inv_cleanPct', inputmode: 'decimal', value: pctOf(sup?.cleanPm), placeholder: '100' })}</div>
      <div>${field({ id: 's-cook', key: 'inv_cookPct', inputmode: 'decimal', value: pctOf(sup?.cookPm), placeholder: '100' })}</div></div>
    <div class="grid2"><div>${field({ id: 's-cost', key: 'costPer', hint: `${C.basisOf(unit) === 1 ? '1' : '100'} ${unit}`, inputmode: 'numeric', value: sup?.costPerBasis ?? '', tour: 'supply.cost' })}</div>
      <div id="sWeight" ${unit === 'unit' ? '' : 'hidden'}>${field({ id: 's-wpu', key: 'weightPerUnit', inputmode: 'decimal', value: sup?.weightPerUnit ?? '' })}</div></div>
    <div class="grid2"><div>${field({ id: 's-low', key: 'minLevel', inputmode: 'numeric', value: sup?.lowAt ?? '', tour: 'supply.low' })}</div>
      <div>${field({ id: 's-shelf', key: 'inv_shelfDays', inputmode: 'numeric', value: sup?.shelfDays ?? '' })}</div></div>
    <div class="btn-row">${btn({ id: 'sSave', variant: 'primary', icon: 'check', key: 'save', tour: 'supply.save' })}</div>
    ${sup ? `<div class="btn-row">${btn({ id: 'sRetire', variant: 'danger', icon: 'trash', key: 'retireSupply', tour: 'supply.retire' })}</div>` : ''}`, { name: 'supply' });
  let k = kind, nb = sup?.nutritionBasis || 'raw';
  for (const b of $$('[data-kind]', $('#sheetIn'))) b.onclick = () => { k = b.dataset.kind; press($$('[data-kind]', $('#sheetIn')), b); $('#sFood').hidden = !isFood(k); };
  for (const b of $$('[data-nb]', $('#sheetIn'))) b.onclick = () => { nb = b.dataset.nb; press($$('[data-nb]', $('#sheetIn')), b); };
  $('#s-unit').onchange = e => { const u = e.target.value; $('#sWeight').hidden = u !== 'unit'; for (const el of $$('#sBasis, #s-cost-hint')) el.textContent = `${C.basisOf(u) === 1 ? '1' : '100'} ${u}`; };
  if (!sup) $('#s-name').oninput = e => { $('#s-id').value = e.target.value.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '').replace(/[^a-z0-9а-яіїєґ]+/gi, '-').replace(/^-|-$/g, ''); };
  const num = v => { const s = String(v ?? '').trim(); if (!s) return null; const n = Number(s.replace(',', '.')); return Number.isFinite(n) ? n : null; };
  $('#sSave').onclick = async () => {
    const id = $('#s-id').value.trim().toLowerCase().replace(/\s+/g, '-'); if (!id) return toast(t('required'));
    const body = { id, name: $('#s-name').value.trim() || id, unit: $('#s-unit').value, kind: k, category: $('#s-cat').value.trim() };
    const low = num($('#s-low').value); if (low != null) body.lowAt = Math.round(low);
    if (isFood(k)) { for (const [f, kk] of [['s-kcal', 'kcalPer100'], ['s-prot', 'proteinPer100'], ['s-fat', 'fatPer100'], ['s-carb', 'carbsPer100']]) { const v = num($('#' + f).value); if (v != null) body[kk] = v; } body.nutritionConfirmed = $('#s-conf').checked; body.nutritionBasis = nb; }
    const cost = num($('#s-cost').value); if (cost != null) body.costPerBasis = Math.round(cost);
    const wpu = num($('#s-wpu').value); if (wpu != null) body.weightPerUnit = wpu;
    for (const [f, kk] of [['s-clean', 'cleanPm'], ['s-cook', 'cookPm']]) { const raw = $('#' + f).value.trim(); if (!raw) continue; const pm = C.pmOfPct(raw); if (pm == null) return toast(t('required')); body[kk] = pm; }
    const shelf = num($('#s-shelf').value); if (shelf != null) body.shelfDays = Math.round(shelf);
    try { await busy($('#sSave'), () => post('/owner/supplies', body)); toast(t('saved')); closeSheet(); ctx.reload(); } catch (e) { fail(e); }
  };
  const ret = $('#sRetire'); if (ret) ret.onclick = async () => { const ok = await confirm(t('retireSupply'), t('retireSupplyHint'), { danger: true }); if (!ok) return openSupply(sup, ctx); try { await post(`/owner/supplies/${encodeURIComponent(sup.id)}/retire`, withLoc()); toast(t('saved')); closeSheet(); ctx.reload(); } catch (e) { fail(e); } };
}

/// A delivery, a write-off or a count on one ingredient.
export function openMove(sup, first, ctx){
  if (!sup) return;
  const sups = ctx.data?.suppliers || [];
  sheet(`<p class="eyebrow" data-t="move"></p><h2>${esc(sup.name || sup.id)}</h2>
    <p class="mono muted">${esc(t('inv_onHand'))} ${sup.onHand ?? 0} · ${esc(t('inv_available'))} ${sup.available ?? 0} ${esc(sup.unit || '')}</p>
    <div class="chips" id="kind" role="group">${MOVES.map(([k, word, ic]) => ui.chip({ as: 'button', selected: k === first, icon: ic, label: key(word), attrs: { data: { k, tour: 'stock.' + k } } })).join('')}</div>
    <div class="inv-big">${field({ id: 'm-qty', key: 'inv_qty', hintKey: 'inv_qtyHint', inputmode: 'decimal', autocomplete: 'off', tour: 'move.qty' })}</div>
    <div id="mRecv">
      <p class="ui-label" data-t="inv_price"></p>${chips({ id: 'mMode', values: [{ value: 'per', label: perWord(sup.unit) }, { value: 'total', key: 'inv_total' }], value: 'per', attr: 'mode' })}
      ${field({ id: 'm-price', key: 'inv_price', inputmode: 'numeric', autocomplete: 'off' })}
      <div class="grid2"><div>${field({ id: 'm-sup', key: 'inv_supplier', value: sup.supplier || '', autocomplete: 'off', attrs: { list: 'supList' } })}<datalist id="supList">${sups.map(s => `<option value="${esc(s)}">`).join('')}</datalist></div>
        <div>${field({ id: 'm-doc', key: 'inv_doc', autocomplete: 'off' })}</div></div>
      <div class="grid2"><div>${field({ id: 'm-lot', key: 'inv_lot', autocomplete: 'off' })}</div><div>${input({ id: 'm-exp', type: 'date', key: 'inv_expiry' })}</div></div>
    </div>
    <div id="mReasons"><p class="ui-label" data-t="reason"></p>${chips({ values: WASTE_REASONS.map(r => ({ value: r, key: r })), attr: 'r', labelKey: 'reason', tour: 'move.wasteReason' })}
      ${(sup.lots || []).length > 1 ? `<p class="ui-label" data-t="inv_lot"></p>${chips({ values: sup.lots.map(l => ({ value: l.code, label: `${l.code} · ${l.left}${l.expiry ? ' · ' + l.expiry : ''}` })), attr: 'lot' })}` : ''}
      ${field({ id: 'm-reason', key: 'reason', tour: 'move.reason', attrs: { readonly: true } })}</div>
    <p class="hint" id="mOut"></p>
    <div class="btn-row">${btn({ id: 'mEdit', variant: 'ghost', icon: 'tools-kitchen-2', key: 'edit', tour: 'move.edit' })}${btn({ id: 'mGo', variant: 'primary', icon: 'check', key: 'save', tour: 'move.save' })}</div>`, { name: 'move' });
  let kind = first, mode = 'per', lot = null;
  const show = () => { $('#mRecv').hidden = kind !== 'received'; $('#mReasons').hidden = kind !== 'wasted'; };
  show();
  for (const b of $$('[data-k]', $('#sheetIn'))) b.onclick = () => { kind = b.dataset.k; press($$('[data-k]', $('#sheetIn')), b); show(); };
  for (const b of $$('[data-mode]', $('#sheetIn'))) b.onclick = () => { mode = b.dataset.mode; press($$('[data-mode]', $('#sheetIn')), b); };
  for (const b of $$('[data-r]', $('#sheetIn'))) b.onclick = () => { $('#m-reason').value = b.dataset.r; press($$('[data-r]', $('#sheetIn')), b); };
  for (const b of $$('[data-lot]', $('#sheetIn'))) b.onclick = () => { lot = lot === b.dataset.lot ? null : b.dataset.lot; press($$('[data-lot]', $('#sheetIn')), lot ? b : null); };
  $('#mEdit').onclick = () => openSupply(sup, ctx);
  $('#mGo').onclick = async () => {
    const qty = C.amount($('#m-qty').value, sup.unit);
    if (qty == null || qty < 0 || (kind !== 'stocktake' && qty === 0)) return toast(t('required'));
    let body = kind === 'stocktake' ? { item: sup.id, observed: qty } : { item: sup.id, qty };
    if (kind === 'wasted') { body.reason = $('#m-reason').value.trim(); if (lot) body.lot = lot; }
    if (kind === 'received') {
      const price = C.amount($('#m-price').value, 'unit');
      body = { ...body, ...C.priceBody(sup.unit, price, mode) };
      for (const [f, kk] of [['m-sup', 'supplier'], ['m-doc', 'doc'], ['m-lot', 'lot'], ['m-exp', 'expiry']]) { const v = $('#' + f).value.trim(); if (v) body[kk] = v; }
    }
    try {
      const r = await busy($('#mGo'), () => post(`/owner/stock/${kind}`, body));
      const line = (r.lines || [])[0] || {};
      toast([t('saved'), line.value != null ? money(line.value) : '', line.drift != null ? `${t('inv_drift')} ${line.drift > 0 ? '+' : ''}${line.drift}` : ''].filter(Boolean).join(' · '));
      closeSheet(); ctx.reload();
    } catch (e) { fail(e); }
  };
}

/// Choose an ingredient, then `then(sup)`.
export function pickSupply(ctx, titleKey, then){
  const all = (ctx.data?.supplies || []).slice().sort((a, b) => String(a.name).localeCompare(String(b.name)));
  sheet(`<p class="eyebrow" data-t="${titleKey}"></p><h2 data-t="inv_pick"></h2>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'pkQ', type: 'search', label: key('search'), placeholder: key('search'), attrs: {} })}</div>
    <div class="rows" id="pkList"></div>`, { name: 'pick' });
  const draw = () => {
    const q = $('#pkQ').value.toLowerCase().trim();
    $('#pkList').innerHTML = all.filter(s => !q || `${s.name} ${s.category}`.toLowerCase().includes(q)).slice(0, 80)
      .map(s => rowBtn({ title: s.name, sub: esc(`${s.category || ''} · ${s.onHand ?? 0} ${s.unit}`), data: { pk: s.id } })).join('');
  };
  draw();
  $('#pkQ').oninput = draw;
  $('#pkList').onclick = e => { const r = e.target.closest('[data-pk]'); if (r) then(all.find(s => s.id === r.dataset.pk)); };
}

/// Prep: raw onto the board, cleaned or cooked off it. A measurement, or into
/// another ingredient. The measured yield can become the default in one tap.
export function openPrep(sup, ctx){
  if (!sup) return pickSupply(ctx, 'inv_prep', s => openPrep(s, ctx));
  const others = (ctx.data?.supplies || []).filter(s => s.id !== sup.id);
  sheet(`<p class="eyebrow" data-t="inv_prep"></p><h2>${esc(sup.name || sup.id)}</h2><p class="hint" data-t="inv_prepHint"></p>
    <p class="ui-label" data-t="inv_stage"></p>${chips({ id: 'pStage', values: [{ value: 'clean', key: 'inv_stage_clean' }, { value: 'cook', key: 'inv_stage_cook' }], value: 'clean', attr: 'st' })}
    <div class="grid2 inv-big"><div>${field({ id: 'p-in', key: 'inv_qtyIn', inputmode: 'decimal', autocomplete: 'off' })}</div><div>${field({ id: 'p-out', key: 'inv_qtyOut', inputmode: 'decimal', autocomplete: 'off' })}</div></div>
    ${select({ id: 'p-into', key: 'inv_into', value: '', options: [{ value: '', key: 'inv_intoSame' }, ...others.map(s => ({ value: s.id, label: s.name }))] })}
    <p class="mono" id="pYield"></p>
    <div class="btn-row">${btn({ id: 'pGo', variant: 'primary', icon: 'check', key: 'save' })}</div>
    <div id="pAdopt"></div>`, { name: 'prep' });
  let stage = 'clean';
  const expected = () => C.pmOf(sup, stage === 'clean' ? 'cleanPm' : 'cookPm', stage === 'clean' ? C.CLEAN_MAX : C.COOK_MAX);
  const live = () => {
    const pm = C.yieldPm(C.amount($('#p-in').value, sup.unit), C.amount($('#p-out').value, sup.unit));
    $('#pYield').textContent = `${t('inv_measured')} ${pm == null ? '-' : C.pct(pm)} · ${t('inv_expected')} ${C.pct(expected())}`;
  };
  for (const b of $$('[data-st]', $('#sheetIn'))) b.onclick = () => { stage = b.dataset.st; press($$('[data-st]', $('#sheetIn')), b); live(); };
  $('#p-in').oninput = live; $('#p-out').oninput = live; live();
  $('#pGo').onclick = async () => {
    const qty = C.amount($('#p-in').value, sup.unit), out = C.amount($('#p-out').value, sup.unit);
    if (!qty || qty <= 0 || out == null || out < 0) return toast(t('required'));
    const body = { item: sup.id, qty, out, stage, ...($('#p-into').value ? { into: $('#p-into').value } : {}) };
    try {
      const r = await busy($('#pGo'), () => post('/owner/stock/produced', body));
      const pm = (r.lines || [])[0]?.yieldPm;
      toast(t('saved'));
      $('#pAdopt').innerHTML = pm == null ? '' : rowDiv({ title: `${t('inv_measured')} ${C.pct(pm)} · ${t('inv_expected')} ${C.pct(expected())}`,
        trailing: btn({ id: 'pAdoptGo', icon: 'check', key: 'inv_adopt' }) });
      const a = $('#pAdoptGo'); if (a) a.onclick = () => adoptYield(sup, stage, pm, a, ctx);
    } catch (e) { fail(e); }
  };
}
