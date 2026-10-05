// STORAGES (P12, W-STORE): where each ingredient is -- kitchen, bar, freezer,
// or a room the owner names -- with a storage filter, a Move sheet, a count of
// one storage, and the storage cards. Every number is a field of
// `GET /api/owner/stock` (`byStore` per supply, `storages`); every save is one
// `POST /api/owner/stock/:kind` -- `moved`, `count` with `store`, `storage`.
// A transfer never adds or removes stock: the hub refuses more than is there.

import { $, $$, esc, t, api, post, sheet, closeSheet, busy, toast } from '/admin/core.js';
import { btn, field, select, chips, press, empty, loading, pill } from '/admin/parts.js';
import * as L from './stock-storages-logic.js';

const fail = e => toast(String(e.message || e));

export async function open(store = 'all'){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="sto_tool"></h2><p class="sheet-hint" data-t="sto_hint"></p>
    <div id="stoBody">${loading(3)}</div>`, { name: 'storages' });
  let stock;
  try { stock = await api('/owner/stock'); } catch (e) { $('#stoBody').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const all = stock.storages || [], live = L.open(all), sups = stock.supplies || [];
  const rows = L.rowsFor(sups, store);
  const named = id => L.nameOf(all.find(s => s.id === id) || { id }, t);
  const list = rows.length ? `<div class="kt-wrap"><table class="kt"><thead><tr><th data-t="sto_item"></th><th data-t="inv_qty"></th><th data-t="sto_where"></th></tr></thead><tbody>
    ${rows.map(r => `<tr><td>${esc(r.name)}</td><td class="mono">${r.qty} ${esc(r.unit)}</td><td class="small">${esc(r.by.map(([s, q]) => `${named(s)} ${q}`).join(' · '))}
      <br><span class="muted">${esc(t('sto_home'))} ${esc(named(r.home))}</span></td></tr>`).join('')}</tbody></table></div>` : empty('box', { key: 'sto_empty' });
  const cur = all.find(s => s.id === store);
  $('#stoBody').innerHTML = `${chips({ id: 'stoFilter', values: [{ value: 'all', key: 'sto_all' }, ...all.map(s => ({ value: s.id, label: L.nameOf(s, t) + (s.archived ? ' · ' + t('sto_archived') : '') }))], value: store, attr: 'sto', labelKey: 'sto_storage', tour: 'store.filter' })}
    ${list}
    <div class="btn-row">${btn({ id: 'stoMove', variant: 'primary', icon: 'arrows-sort', key: 'sto_move', tour: 'store.move' })}
      ${cur && !cur.archived ? btn({ id: 'stoCount', icon: 'check', key: 'sto_count', tour: 'store.count' }) : ''}</div>
    <p class="ui-label mt-3" data-t="sto_recv"></p>
    ${chips({ id: 'stoRecv', values: [{ value: '', key: 'sto_home' }, ...live.map(s => ({ value: s.id, label: L.nameOf(s, t) }))], value: L.recvStore(), attr: 'recv', tour: 'store.recv' })}
    <p class="hint" data-t="sto_recvHint"></p>
    ${cur ? `<div class="btn-row">${btn({ id: 'stoRename', variant: 'ghost', icon: 'note', key: 'sto_rename', tour: 'store.rename' })}
      ${cur.default ? '' : btn({ id: 'stoArchive', variant: 'ghost', icon: cur.archived ? 'history' : 'box', key: cur.archived ? 'sto_unarchive' : 'sto_archive', tour: 'store.archive' })}</div>
      <p class="hint" data-t="sto_archiveHint"></p>` : ''}
    <div class="btn-row">${btn({ id: 'stoAdd', variant: 'ghost', icon: 'plus', key: 'sto_add', tour: 'store.add' })}</div>`;
  for (const b of $$('[data-sto]', $('#sheetIn'))) b.onclick = () => open(b.dataset.sto);
  for (const b of $$('[data-recv]', $('#sheetIn'))) b.onclick = () => { L.setRecvStore(b.dataset.recv); press($$('[data-recv]', $('#sheetIn')), b); };
  $('#stoMove').onclick = () => openMove(stock, store === 'all' ? '' : store);
  const cnt = $('#stoCount'); if (cnt) cnt.onclick = () => openCount(stock, cur);
  $('#stoAdd').onclick = () => editCard(null);
  const ren = $('#stoRename'); if (ren) ren.onclick = () => editCard(cur);
  const arc = $('#stoArchive'); if (arc) arc.onclick = () => saveCard({ id: cur.id, name: cur.name || named(cur.id), archived: !cur.archived }, arc, cur.id);
}

/// The Move sheet: ingredient, quantity, from, to.
export function openMove(stock, from = ''){
  const sups = (stock.supplies || []).filter(s => s.kind !== 'prep'), live = L.open(stock.storages);
  const opts = live.map(s => ({ value: s.id, label: L.nameOf(s, t) }));
  sheet(`<p class="eyebrow" data-t="sto_tool"></p><h2 data-t="sto_move"></h2><p class="sheet-hint" data-t="sto_moveHint"></p>
    ${select({ id: 'mvItem', key: 'sto_item', value: '', options: [{ value: '', key: 'inv_pick' }, ...sups.map(s => ({ value: s.id, label: s.name || s.id }))], tour: 'store.moveItem' })}
    <div class="inv-big">${field({ id: 'mvQty', key: 'inv_qty', hintKey: 'inv_qtyHint', inputmode: 'decimal', autocomplete: 'off', tour: 'store.moveQty' })}</div>
    <p class="ui-label" data-t="sto_from"></p>${chips({ values: opts, value: from, attr: 'mvf', tour: 'store.moveFrom' })}
    <p class="ui-label" data-t="sto_to"></p>${chips({ values: opts, value: '', attr: 'mvt', tour: 'store.moveTo' })}
    <p class="hint" id="mvHave"></p>
    <div class="btn-row">${btn({ id: 'mvGo', variant: 'primary', icon: 'check', key: 'save', tour: 'store.moveSave' })}</div>`, { name: 'storeMove' });
  let f = from, to = '';
  const have = () => { const s = sups.find(x => x.id === $('#mvItem').value); $('#mvHave').textContent = s && f ? `${L.nameOf(live.find(x => x.id === f), t)}: ${(s.byStore?.stores || {})[f] || 0} ${s.unit}` : ''; };
  for (const b of $$('[data-mvf]', $('#sheetIn'))) b.onclick = () => { f = b.dataset.mvf; press($$('[data-mvf]', $('#sheetIn')), b); have(); };
  for (const b of $$('[data-mvt]', $('#sheetIn'))) b.onclick = () => { to = b.dataset.mvt; press($$('[data-mvt]', $('#sheetIn')), b); };
  $('#mvItem').onchange = have;
  $('#mvGo').onclick = async () => {
    const item = $('#mvItem').value, unit = sups.find(s => s.id === item)?.unit || 'g';
    const r = L.moveBody(item, $('#mvQty').value, unit, f, to);
    if (r.error) return toast(t(r.error));
    try { await busy($('#mvGo'), () => post('/owner/stock/moved', r.body)); toast(t('sto_moved')); open(to); } catch (e) { fail(e); }
  };
}

/// A count of ONE storage: only what is in it, plus anything typed.
function openCount(stock, cur){
  const sups = (stock.supplies || []).filter(s => s.kind !== 'prep');
  sheet(`<p class="eyebrow" data-t="sto_tool"></p><h2>${esc(L.nameOf(cur, t))}</h2><p class="sheet-hint" data-t="sto_countHint"></p>
    <div class="rows">${sups.map(s => `<div class="row">${field({ id: 'sc-' + s.id, label: `${s.name || s.id} (${(s.byStore?.stores || {})[cur.id] || 0} ${s.unit})`, inputmode: 'decimal', autocomplete: 'off', attrs: { data: { sc: s.id } } })}</div>`).join('')}</div>
    <div class="btn-row">${btn({ id: 'scGo', variant: 'primary', icon: 'check', key: 'inv_saveAll', tour: 'store.countSave' })}</div>`, { name: 'storeCount' });
  $('#scGo').onclick = async () => {
    const entries = $$('[data-sc]', $('#sheetIn')).map(i => [i.dataset.sc, i.value]);
    const r = L.countBody(entries, cur.id, id => sups.find(s => s.id === id)?.unit || 'g');
    if (r.error) return toast(r.error === 'required' ? t('required') : `${r.error}: ${t('required')}`);
    try { const out = await busy($('#scGo'), () => post('/owner/stock/count', r.body)); toast(`${t('inv_countSaved')} · ${(out.lines || []).length} ${t('inv_lines')}`); open(cur.id); } catch (e) { fail(e); }
  };
}

/// Name a new storage, or rename one.
function editCard(cur){
  sheet(`<p class="eyebrow" data-t="sto_tool"></p><h2 data-t="${cur ? 'sto_rename' : 'sto_add'}"></h2>
    ${field({ id: 'stName', key: 'sto_name', value: cur ? L.nameOf(cur, t) : '', autocomplete: 'off', tour: 'store.name' })}
    ${cur?.archived ? pill('warn', { key: 'sto_archived' }) : ''}
    <div class="btn-row">${btn({ id: 'stSave', variant: 'primary', icon: 'check', key: 'save', tour: 'store.save' })}</div>`, { name: 'storeCard' });
  $('#stSave').onclick = () => {
    const name = $('#stName').value.trim(); if (!name) return toast(t('required'));
    saveCard({ ...(cur ? { id: cur.id, archived: !!cur.archived } : {}), name }, $('#stSave'), cur?.id);
  };
}

async function saveCard(card, el, back){
  try { const r = await busy(el, () => post('/owner/stock/storage', { card })); toast(t('saved')); open(back || r.id || 'all'); } catch (e) { fail(e); }
}

export { closeSheet };
