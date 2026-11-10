// THE THREE SHEETS THAT TOUCH MANY INGREDIENTS AT ONCE: a count of the whole
// shelf (one stocktake session, one decision), a delivery with every line of
// its invoice, and the write-offs with what they cost.
//
//   count     POST /api/owner/stock/count     {lines:[{item, observed}]}
//   delivery  POST /api/owner/stock/received  one per quantity and price, the paper on each
//             POST /api/owner/supplies/bulk   once, first, for the positions typed new
//   waste     GET  /api/owner/stock/waste     every binned thing, valued
//
// Whole grams throughout (1,5 kg is read as 1500 g); money in minor units.

import { $, $$, esc, t, api, post, toast, sheet, busy, money, day, clock, icon, retranslate, hydrate, S } from '/admin/core.js';
import { decimalsOf } from '/lib/money.js';
import { ui, k as key, btn, field, input, chips, press, pill, empty, loading, rowDiv } from '/admin/parts.js';
import * as C from '/admin/ingredients-calc.js';
import { openMove, pickSupply } from '/admin/ingredients.js';
import * as F from '/admin/supply-resolve-field.js';
import * as R from '/admin/supply-resolve.js';

const fail = e => toast(String(e.message || e));
const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '-');
const nameOf = (ctx, id) => (ctx.data?.supplies || []).find(s => s.id === id)?.name || id;
const byShelf = list => list.slice().sort((a, b) => String(a.category || '').localeCompare(String(b.category || '')) || String(a.name).localeCompare(String(b.name)));

/// The whole shelf, counted: every line one decision, one session id.
export function openCount(ctx){
  const all = byShelf(ctx.data?.supplies || []);
  const sessions = (ctx.data?.sessions || []).map(s => `<tr><td>${esc(when(s.at))}</td><td>${s.lines}</td><td class="${s.value < 0 ? 'neg' : ''}">${money(s.value)}</td></tr>`).join('');
  sheet(`<p class="eyebrow" data-t="inv_count"></p><h2 data-t="inv_count"></h2><p class="hint" data-t="inv_countHint"></p>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'cQ', type: 'search', label: key('search'), placeholder: key('search'), attrs: {} })}</div>
    <div id="cList">${all.map(s => `<div class="inv-count" data-row="${esc(s.id)}"><span class="t"><b>${esc(s.name)}</b>
      <small><span data-t="inv_onHand"></span> ${C.showMilli(C.levelMilli(s, 'onHand'))} ${esc(s.unit)} · ${esc(s.category || '')} <span data-drift="${esc(s.id)}"></span></small></span>
      ${ui.inputRow({ label: s.name, attrs: { placeholder: C.showMilli(C.levelMilli(s, 'onHand')), inputmode: 'decimal', autocomplete: 'off', data: { ci: s.id } } })}</div>`).join('')}</div>
    <div class="btn-row">${btn({ id: 'cGo', variant: 'primary', icon: 'check', key: 'save' })}</div>
    <div id="cOut"></div>
    ${sessions ? `<p class="eyebrow mt-3" data-t="inv_sessions"></p><div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_day"></th><th data-t="inv_lines"></th><th data-t="inv_value"></th></tr></thead><tbody>${sessions}</tbody></table></div>` : ''}`, { name: 'count' });
  const sup = id => all.find(s => s.id === id);
  $('#cQ').oninput = () => { const has = R.finder($('#cQ').value); for (const r of $$('[data-row]', $('#cList'))) r.hidden = !has(sup(r.dataset.row)); };
  for (const inp of $$('[data-ci]', $('#cList'))) inp.oninput = () => {
    const s = sup(inp.dataset.ci), v = C.milli(inp.value, s.unit), el = $(`[data-drift="${CSS.escape(s.id)}"]`);
    const d = v == null ? 0 : v - C.levelMilli(s, 'onHand');
    el.textContent = v == null ? '' : `· ${t('inv_drift')} ${d > 0 ? '+' : ''}${C.showMilli(d)}`;
  };
  $('#cGo').onclick = async () => {
    const lines = [];
    for (const inp of $$('[data-ci]', $('#cList'))) {
      if (!inp.value.trim()) continue;
      // W-QTYHUB H3: a count keeps its fraction (0,75 kg, 2,5 sheets), sent as text.
      const v = C.milli(inp.value, sup(inp.dataset.ci).unit);
      if (v == null && C.whyNot(inp.value, sup(inp.dataset.ci).unit) === 'tooFine') { inp.focus(); return toast(`${t('inv_wholeUnits')} (${sup(inp.dataset.ci).unit})`); }
      if (v == null || v < 0) { inp.focus(); return toast(t('required')); }
      lines.push({ item: inp.dataset.ci, observed: C.showMilli(v) });
    }
    if (!lines.length) return toast(t('required'));
    try {
      const r = await busy($('#cGo'), () => post('/owner/stock/count', { lines }));
      $('#cOut').innerHTML = `<p class="ok">${esc(t('inv_countSaved'))} · ${lines.length} ${esc(t('inv_lines'))} · ${money(r.value || 0)}</p>
        <div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_ingredient"></th><th data-t="inv_expected"></th><th data-t="inv_counted"></th><th data-t="inv_drift"></th><th data-t="inv_value"></th></tr></thead>
        <tbody>${(r.lines || []).map(l => `<tr><td>${esc(nameOf(ctx, l.item))}</td><td>${l.expected}</td><td>${l.observed}</td><td class="${l.drift < 0 ? 'neg' : l.drift > 0 ? 'pos' : ''}">${l.drift > 0 ? '+' : ''}${l.drift}</td><td>${l.value != null ? money(l.value) : '-'}</td></tr>`).join('')}</tbody></table></div>`;
      retranslate($('#cOut'));
      ctx.reload();
    } catch (e) { fail(e); }
  };
}

/// One quantity and price under a position; `k` is its place on the line (the
/// lot code of every pair after the first is told apart by it).
const pairMarkup = k => `<div class="grid2 pair" data-dp="${k}"><div>${field({ key: 'inv_qty', inputmode: 'decimal', autocomplete: 'off', attrs: { data: { dlq: '1' } } })}</div>
    <div>${field({ key: 'inv_price', inputmode: 'numeric', autocomplete: 'off', attrs: { data: { dlp: '1' } } })}</div><p data-dperr hidden></p></div>`;
/// An error on a line or a pair, or none.
const say = (el, msg) => { el.hidden = !msg; el.innerHTML = msg ? pill('bad', { label: msg }) : ''; };

/// A delivery with every line of its paper: one supplier, one invoice number.
/// A position is TYPED (`supply-resolve-field.js`): a known name is its id, a
/// similar one is a question, an unknown one is a new position, created before
/// anything is received. One position takes several prices: each quantity and
/// price is its own record and its own lot.
export function openDelivery(ctx){
  const sups = ctx.data?.suppliers || [];
  const line = i => `<div class="group" data-dl="${i}">${F.markup('d' + i)}
      <div data-dps>${pairMarkup(1)}</div>
      <div class="btn-row">${btn({ variant: 'ghost', icon: 'plus', key: 'inv_morePrice', data: { dmore: String(i) } })}</div>
      <div class="grid2 pair"><div>${field({ key: 'inv_lot', autocomplete: 'off', attrs: { data: { dll: '1' } } })}</div><div>${input({ type: 'date', key: 'inv_expiry', attrs: { data: { dle: '1' } } })}</div></div>
      <p data-dlerr hidden></p></div>`;
  sheet(`<p class="eyebrow" data-t="inv_delivery"></p><h2 data-t="inv_delivery"></h2><p class="hint" data-t="inv_deliveryHint"></p>
    <div class="grid2"><div>${field({ id: 'd-sup', key: 'inv_supplier', autocomplete: 'off', attrs: { list: 'dSupList' } })}<datalist id="dSupList">${sups.map(s => `<option value="${esc(s)}">`).join('')}</datalist></div>
      <div>${field({ id: 'd-doc', key: 'inv_doc', autocomplete: 'off' })}</div></div>
    <p class="ui-label" data-t="inv_price"></p>${chips({ id: 'dMode', values: [{ value: 'per', label: `${t('inv_perKg')} / ${t('inv_perL')} / ${t('inv_perPiece')}` }, { value: 'total', key: 'inv_total' }], value: 'per', attr: 'dmode' })}
    <div id="dLines"></div>
    <div class="btn-row">${btn({ id: 'dAdd', icon: 'plus', key: 'inv_addLine' })}${btn({ id: 'dGo', variant: 'primary', icon: 'check', key: 'inv_saveAll' })}</div>
    <div id="dOut"></div>`, { name: 'delivery' });
  let n = 0, mode = 'per', saving = false;
  const lines = [];
  const add = () => {
    const i = n++;
    $('#dLines').insertAdjacentHTML('beforeend', line(i)); retranslate($('#dLines'));
    const el = $$('[data-dl]', $('#dLines')).find(x => x.dataset.dl === String(i));
    const L = { el, f: F.bind($('[data-sr]', el), { supplies: () => ctx.data?.supplies }), pairs: 1 };
    $('[data-dmore]', el).onclick = () => { $('[data-dps]', el).insertAdjacentHTML('beforeend', pairMarkup(++L.pairs)); retranslate(el); };
    lines.push(L);
  };
  add();
  for (const b of $$('[data-dmode]', $('#sheetIn'))) b.onclick = () => { mode = b.dataset.dmode; press($$('[data-dmode]', $('#sheetIn')), b); };
  $('#dAdd').onclick = add;
  $('#dGo').onclick = async () => {
    if (saving) return;
    const rows = lines.map(L => ({ L, got: L.f.settle() })).filter(x => x.got.state !== 'empty');
    if (!rows.length) return toast(t('required'));
    // A question left open, or a new position without its unit and type: nothing is sent.
    const stuck = rows.find(x => x.got.state === 'ask' || x.got.state === 'incomplete');
    if (stuck) { stuck.L.f.focus(); return toast(t(stuck.got.state === 'ask' ? 'sr_answer' : 'sr_fillNew')); }
    const paper = { supplier: $('#d-sup').value.trim(), doc: $('#d-doc').value.trim() };
    const out = [];
    let created = false;
    saving = true;
    try {
      await busy($('#dGo'), async () => {
        // NEW POSITIONS FIRST, in one call: a receipt needs the id it lands on.
        const news = rows.filter(x => x.got.state === 'new');
        const made = news.length ? await F.createAll(news.map(x => x.got), ctx) : null;
        created = !!made;
        for (const x of news) {
          const sup = made.get(R.fold(x.got.name));
          if (sup && !sup.retired) { x.L.f.pick(sup); x.got = { state: 'ok', sup }; say($('[data-dlerr]', x.L.el), ''); }
          else { x.got = null; say($('[data-dlerr]', x.L.el), t(sup ? 'sr_retired' : 'sr_noId')); }
        }
        for (const x of rows) {
          if (!x.got) continue;
          const sup = x.got.sup, el = x.L.el;
          const lot = $('[data-dll]', el).value.trim(), expiry = $('[data-dle]', el).value.trim();
          for (const pr of $$('[data-dp]', el)) {
            const typedQty = $('[data-dlq]', pr).value.trim(), typedPrice = $('[data-dlp]', pr).value.trim();
            // An "another price" left blank is not a line of the paper.
            if (!typedQty && !typedPrice && $$('[data-dp]', el).length > 1) { pr.remove(); continue; }
            const m = C.milli(typedQty, sup.unit), price = C.priceMinor(typedPrice, decimalsOf(S.venue?.currencyCode || 'ALL'));
            if (!m || m <= 0) { say($('[data-dperr]', pr), `${t('inv_lineFailed')}: ${t('required')}`); continue; }
            // A price that does not read is refused here: sent on, it would be a delivery with no price.
            if (typedPrice && price == null) { say($('[data-dperr]', pr), `${t('inv_lineFailed')}: ${t('inv_badPrice')}`); continue; }
            const qty = C.showMilli(m), k = Number(pr.dataset.dp);
            const body = { item: sup.id, qty, ...C.priceBody(sup.unit, price, mode) };
            // One label, several prices: each price is its own lot, so each gets its own code.
            for (const [kk, v] of [['supplier', paper.supplier], ['doc', paper.doc], ['lot', lot && k > 1 ? `${lot}/${k}` : lot], ['expiry', expiry]]) if (v) body[kk] = v;
            let r;
            try { r = await post('/owner/stock/received', body); }
            catch (e) { say($('[data-dperr]', pr), `${t('inv_lineFailed')}: ${e.message || e}`); continue; }
            // Written: it leaves the form before anything else can fail, so a retry cannot write it twice.
            pr.remove();
            const v = (r?.lines || [])[0]?.value;
            out.push([sup.name, `${qty} ${sup.unit}${v != null ? ' · ' + money(v) : ''}`]);
          }
          if (!$$('[data-dp]', el).length) { el.remove(); lines.splice(lines.indexOf(x.L), 1); }
        }
      });
    } catch (e) { fail(e); }
    finally { saving = false; }
    $('#dOut').innerHTML = `<div class="rows">${out.map(([a, b]) => rowDiv({ title: a, sub: esc(b) })).join('')}</div>`;
    // Anything written (a receipt, or only the new positions) is on the list behind the sheet at once.
    if (out.length || created) ctx.reload();
  };
}

/// Every write-off, valued, with its reason, signer and lot; and the button
/// that records a new one.
export async function openWaste(ctx){
  sheet(`<p class="eyebrow" data-t="inv_waste"></p><h2 data-t="inv_waste"></h2><p class="hint" data-t="inv_wasteHint"></p>
    <div class="btn-row">${btn({ id: 'wNew', variant: 'primary', icon: 'trash', key: 'inv_writeOff' })}</div><div id="wOut">${loading(3)}</div>`, { name: 'waste' });
  $('#wNew').onclick = () => pickSupply(ctx, 'inv_writeOff', s => openMove(s, 'wasted', ctx));
  let r;
  try { r = await api('/owner/stock/waste'); } catch (e) { $('#wOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const tot = r.totals || {}, byValue = tot.valueByReason || {}, stockQty = (tot.byReason || {}).stock || {};
  const reasons = [...new Set([...Object.keys(byValue), ...Object.keys(stockQty)])];
  const heights = C.bars(reasons.map(x => byValue[x] || 0));
  const rows = (r.rows || []).slice().reverse();
  $('#wOut').innerHTML = `<div class="stats strip"><div class="stat money"><b>${money(tot.value || 0)}</b><small data-t="inv_value"></small></div>
      <div class="stat"><b>${rows.length}</b><small data-t="inv_lines"></small></div><div class="stat"><b>${tot.unvalued || 0}</b><small data-t="inv_unvalued"></small></div></div>
    <p class="eyebrow" data-t="inv_byReason"></p>
    ${reasons.length ? `<div class="kt-wrap"><table class="kt"><tbody>${reasons.map((x, i) => `<tr><td>${esc(t(x))}<span class="kt-bar"><i data-w="${heights[i]}"></i></span></td><td>${money(byValue[x] || 0)}</td></tr>`).join('')}</tbody></table></div>` : `<p class="hint" data-t="none"></p>`}
    <p class="eyebrow" data-t="inv_moves"></p>
    ${rows.length ? `<div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_day"></th><th data-t="ka_ingredient"></th><th data-t="inv_qty"></th><th data-t="inv_value"></th><th data-t="reason"></th><th data-t="inv_lot"></th></tr></thead>
      <tbody>${rows.map(w => `<tr><td>${esc(when(w.at))}</td><td>${esc(w.source === 'void' ? w.item : nameOf(ctx, w.item))}</td><td>${w.qty}</td><td>${w.value != null ? money(w.value) : '-'}</td><td>${esc(t(w.reason))}</td><td>${esc(w.lot || '')}</td></tr>`).join('')}</tbody></table></div>`
      : empty('trash', { key: 'none' })}`;
  retranslate($('#wOut')); hydrate($('#wOut'));
}
