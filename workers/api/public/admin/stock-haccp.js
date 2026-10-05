// RAW FISH & HACCP (P13, W-STORE): the in-house freezing record of a lot --
// how long, how cold, and which EU 853/2004 rule that meets (never a block on
// a sale) -- and the three CSV files an inspector asks for, over the venue's
// local days: lot -> orders, order -> lots, the freezing log.
//   POST /api/owner/stock/frozen  {item, lot, hours, tempC, store?}
//   GET  /api/owner/stock/haccp?kind=lots|orders|freezing&from=&to=  (owner only)
// A delivery the SUPPLIER already treated is recorded on the delivery itself
// (the Delivery sheet's "Supplier already froze it" field, `treated`).

import { $, $$, esc, t, api, post, sheet, busy, toast, API, store } from '/admin/core.js';
import { btn, field, input, select, chips, press, empty, loading, pill } from '/admin/parts.js';
import { me } from '/admin/app.js';
import * as L from './stock-storages-logic.js';

const fail = e => toast(String(e.message || e));
const TONE = { hc_rule20: 'ok', hc_rule35: 'ok', hc_ruleNone: 'warn' };

export async function open(){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="hc_tool"></h2><p class="sheet-hint" data-t="hc_hint"></p>
    <div id="hcBody">${loading(3)}</div>`, { name: 'haccp' });
  let stock;
  try { stock = await api('/owner/stock'); } catch (e) { $('#hcBody').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const sups = (stock.supplies || []).filter(s => s.kind !== 'prep');
  const live = L.open(stock.storages);
  const today = stock.today || new Date().toISOString().slice(0, 10);
  const owner = !me().staff;
  $('#hcBody').innerHTML = `<p class="eyebrow" data-t="hc_freeze"></p>
    ${select({ id: 'hcItem', key: 'sto_item', value: '', options: [{ value: '', key: 'inv_pick' }, ...sups.map(s => ({ value: s.id, label: s.name || s.id }))], tour: 'haccp.item' })}
    <div id="hcLots"></div>
    ${field({ id: 'hcLot', key: 'hc_lot', autocomplete: 'off', tour: 'haccp.lot' })}
    <div class="grid2 pair"><div>${field({ id: 'hcHours', key: 'hc_hours', inputmode: 'numeric', value: '24', tour: 'haccp.hours' })}</div>
      <div>${field({ id: 'hcTemp', key: 'hc_temp', inputmode: 'numeric', value: '-20', tour: 'haccp.temp' })}</div></div>
    <p class="ui-label" data-t="sto_storage"></p>${chips({ values: live.map(s => ({ value: s.id, label: L.nameOf(s, t) })), value: 'freezer', attr: 'hcs' })}
    <p id="hcRule"></p>
    <div class="btn-row">${btn({ id: 'hcGo', variant: 'primary', icon: 'check', key: 'save', tour: 'haccp.save' })}</div>
    <p class="eyebrow mt-3" data-t="hc_export"></p>
    ${owner ? `<div class="grid2 pair"><div>${input({ id: 'hcFrom', type: 'date', key: 'hc_from', value: L.daysBefore(today, 30), tour: 'haccp.from' })}</div>
      <div>${input({ id: 'hcTo', type: 'date', key: 'hc_to', value: today, tour: 'haccp.to' })}</div></div>
      <div class="btn-row">${['lots', 'orders', 'freezing'].map(k => btn({ icon: 'file-spreadsheet', key: 'hc_' + k, data: { hcx: k }, tour: 'haccp.' + k })).join('')}</div>`
      : `<p class="hint" data-t="hc_ownerOnly"></p>`}`;
  let where = 'freezer';
  const rule = () => { const k = L.ruleKey($('#hcHours').value.trim(), $('#hcTemp').value.trim()); $('#hcRule').innerHTML = pill(TONE[k], { key: k }); };
  const lots = () => {
    const s = sups.find(x => x.id === $('#hcItem').value);
    $('#hcLots').innerHTML = (s?.lots || []).length ? `<p class="ui-label" data-t="hc_lotPick"></p>${chips({ values: s.lots.map(l => ({ value: l.code, label: `${l.code} · ${l.left} ${s.unit}${l.expiry ? ' · ' + l.expiry : ''}` })), attr: 'hcl' })}` : '';
    for (const b of $$('[data-hcl]', $('#sheetIn'))) b.onclick = () => { $('#hcLot').value = b.dataset.hcl; press($$('[data-hcl]', $('#sheetIn')), b); };
  };
  rule();
  $('#hcItem').onchange = lots;
  $('#hcHours').oninput = rule; $('#hcTemp').oninput = rule;
  for (const b of $$('[data-hcs]', $('#sheetIn'))) b.onclick = () => { where = b.dataset.hcs; press($$('[data-hcs]', $('#sheetIn')), b); };
  $('#hcGo').onclick = async () => {
    const r = L.freezeBody($('#hcItem').value, $('#hcLot').value, $('#hcHours').value, $('#hcTemp').value, where);
    if (r.error) return toast(t(r.error));
    try { const out = await busy($('#hcGo'), () => post('/owner/stock/frozen', r.body)); toast(`${t('hc_saved')} · ${t(L.ruleKey(out.hours, out.tempC))}`); } catch (e) { fail(e); }
  };
  for (const b of $$('[data-hcx]', $('#sheetIn'))) b.onclick = () => download(b.dataset.hcx, $('#hcFrom').value, $('#hcTo').value, b);
}

/// The CSV, fetched with the session and saved as a file.
async function download(kind, from, to, el){
  try {
    await busy(el, async () => {
      const loc = store.loc ? `&location_id=${encodeURIComponent(store.loc)}` : '';
      const r = await fetch(API + L.exportPath(kind, from, to) + loc, { headers: store.t ? { authorization: 'Bearer ' + store.t } : {} });
      if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
      const blob = new Blob(['﻿' + await r.text()], { type: 'text/csv;charset=utf-8' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `haccp-${kind}-${from}-${to}.csv`; a.click();
      requestAnimationFrame(() => URL.revokeObjectURL(a.href));
    });
  } catch (e) { fail(e); }
}

export { esc };
