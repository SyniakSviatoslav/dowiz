// THE TILL TAB (W-EBILLS-TAB, operator 2026-11-03): every sale ebills.al rang
// up in the last 30 days, as the hub imported it, and the numbers over them --
// revenue by day, orders by hour and weekday, payment split, top till lines,
// the names no dish answers to (counted in revenue, written off nowhere).
// GET /api/owner/ebills/sales; a tap on a row opens the sale.
//
// The link's settings and the code mapping stay in ebills.js; this sheet
// only reads. Every number is the hub's (`ebills/sales.rs`): the screen draws.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, icon, t, api, toast, sheet, money, moneyEl, day, clock, ago, store, hydrate, retranslate } from '/admin/core.js';
import { intlLocale } from '/admin/i18n.js';
import { ui, k, btn, chips, field, loading, rowBtn, rowDiv, empty, pill } from '/admin/parts.js';
import '/admin/ebills-sales-i18n.js';
import * as L from '/admin/ebills-sales-logic.js';
import { sidesSection } from '/admin/analytics-sides.js'; // kitchen vs drinks (W-DRINKS-KITCHEN), the pane's own section
import { REFUSED } from '/lib/vocab.js'; // the kernel's own took-no-money set, as orders.js reads it

const view = { d: null, mode: 'list', q: '', day: null };
const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="analytics"></p><h2 data-t="es_title"></h2><p class="sheet-hint" data-t="es_hint"></p>`;
const hh = h => String(h).padStart(2, '0');
const stat = (key, value, cls = '') => `<div class="stat ${cls}"><small data-t="${key}"></small><b>${value}</b></div>`;
const sum = s => `<b class="mono">${esc(s.orders)} · ${esc(money(s.revenue || 0))}</b>`;
/// The local day a row belongs to: the last `by_day` boundary at or before it.
const dayOf = ms => { const d = (view.d?.by_day || []).filter(x => x.at_ms <= ms).pop(); return d ? d.day : null; };
const showDay = s => { const d = (view.d?.by_day || []).find(x => x.day === s); return d ? esc(day(d.at_ms + 43_200_000)) : esc(s); };

export async function open(){
  sheet(`${head}<div id="esBody">${loading(3)}</div>`, { name: 'ebillsSales', keepScroll: true });
  try { view.d = await api('/owner/ebills/sales' + q()); } catch (e) { return fail(e); }
  paint();
}

function paint(){
  const d = view.d || {};
  const modes = [{ value: 'list', key: 'es_list' }, { value: 'numbers', key: 'es_numbers' }];
  $('#esBody').innerHTML = `
    <div class="stats">${stat('es_orders30', esc(d.count ?? 0))}${stat('es_revenue30', esc(money(d.revenue || 0)))}
      ${stat('es_avgCheck', esc(money(d.average_check || 0)))}${stat('es_unmatchedShare', esc(L.pct(d.unmatched?.share_ppm ?? 0)), d.unmatched?.lines ? 'warn' : '')}</div>
    ${chips({ values: modes, value: view.mode, attr: 'mode' })}
    <div id="esPane" class="mt-3">${view.mode === 'list' ? list(d) : numbers(d)}</div>`;
  const root = $('#esBody'); retranslate(root); hydrate(root);
  for (const b of $$('[data-mode]', root)) b.onclick = () => { view.mode = b.dataset.mode; paint(); };
  wire(root);
}

function numbers(d){
  const byDay = d.by_day || [], hDay = L.heights(byDay), byHour = d.by_hour || [], hHour = L.heights(byHour, 'orders');
  const names = L.weekdays(intlLocale());
  const pay = Object.entries(d.by_payment || {});
  const kinds = d.by_kind || {};
  const m = d.matched || {}, u = d.unmatched || {};
  return `
    ${sidesSection(d.sides, showDay)}
    <p class="eyebrow mt-3" data-t="es_byDay"></p><div class="bars" role="img" aria-label="${esc(t('es_byDay'))}">${byDay.map((x, i) => `<i data-day="${esc(x.day)}" data-h="${hDay[i]}" title="${showDay(x.day)} · ${esc(x.orders)} · ${esc(money(x.revenue || 0))}"></i>`).join('')}</div>
    <div class="axis"><span>${showDay(byDay[0]?.day)}</span><span>${showDay(byDay[byDay.length - 1]?.day)}</span></div>
    <p class="eyebrow mt-3" data-t="es_byHour"></p><div class="bars" role="img" aria-label="${esc(t('es_byHour'))}">${byHour.map((x, h) => `<i data-h="${hHour[h]}" class="${hHour[h] === 100 && x.orders ? 'hi' : ''}" title="${hh(h)}:00 · ${esc(x.orders)} · ${esc(money(x.revenue || 0))}"></i>`).join('')}</div>
    <div class="axis"><span>00</span><span>12</span><span>23</span></div>
    <section class="group mt-3"><p class="eyebrow" data-t="es_byWeekday"></p><div class="rows">${(d.by_weekday || []).map((x, i) => rowDiv({ leading: icon('clock'), title: names[i], trailing: sum(x) })).join('')}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="es_payment"></p><div class="rows">${pay.map(([p, x]) => rowDiv({ leading: icon(L.payKey(p) === 'card' ? 'credit-card' : 'coin-hole'), title: L.payKey(p) ? { t: 'es_' + L.payKey(p) } : p, trailing: sum(x) })).join('') || empty('receipt', { key: 'es_none' })}
      ${rowDiv({ leading: icon('tools-kitchen-2'), title: { t: 'es_dineIn' }, trailing: `<b class="mono">${esc(kinds.dine_in || 0)}</b>` })}
      ${rowDiv({ leading: icon('walk'), title: { t: 'es_counter' }, trailing: `<b class="mono">${esc(kinds.pickup || 0)}</b>` })}
      ${rowDiv({ leading: icon('clock-hour-4'), title: { t: 'es_unpaid' }, trailing: `<b class="mono">${esc(d.unpaid_courses || 0)}</b>` })}
      ${rowDiv({ leading: icon('x'), title: { t: 'es_notTaken' }, trailing: `<b class="mono">${esc(d.not_taken || 0)}</b>` })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="es_topItems"></p><p class="muted small" data-t="es_noWriteOff"></p><div class="rows">${(d.items || []).slice(0, 12).map(x => rowDiv({ leading: icon(x.product_id ? 'bowl-chopsticks' : 'plug-connected-x'), title: x.name || x.code,
      sub: `<span class="mono">${esc(x.code)}</span> · ${pill(x.product_id ? 'ok' : 'warn', { key: x.product_id ? (x.matched_by === 'name' ? 'es_matchedName' : 'es_matchedMap') : 'es_open' })}`,
      trailing: `<b class="mono">${esc(x.quantity)} · ${esc(money(x.revenue || 0))}</b>` })).join('') || empty('receipt', { key: 'es_none' })}</div>
      <p class="muted small mt-3"><span data-t="es_matchedMap"></span>: ${esc(m.map?.lines || 0)} <span data-t="es_lines"></span> · <span data-t="es_matchedName"></span>: ${esc(m.name?.lines || 0)} · <span data-t="es_open"></span>: ${esc(u.lines || 0)}</p></section>
    <section class="group mt-3"><p class="eyebrow" data-t="es_unmatched"></p><p class="muted small" data-t="es_unmatchedHint"></p><div class="rows">${(u.names || []).map(x => rowDiv({ leading: icon('plug-connected-x'), title: x.name || x.code,
      sub: `<span class="mono">${esc(x.code)} · ${esc(x.quantity)} ${esc(t('es_portions'))}</span>`, trailing: `<b class="mono">${esc(money(x.revenue || 0))}</b>` })).join('') || `<p class="muted small" data-t="eb_allMatched"></p>`}</div>
      <div class="btn-row">${btn({ id: 'esLink', variant: 'ghost', icon: 'receipt', key: 'es_openLink' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="es_tables"></p><div class="rows">${(d.tables || []).slice(0, 10).map(x => rowDiv({ leading: icon('tools-kitchen-2'), title: `${t('es_table')} ${esc(x.table)}`, trailing: sum(x) })).join('') || empty('receipt', { key: 'es_none' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="es_operators"></p><div class="rows">${(d.operators || []).map(x => rowDiv({ leading: icon('user'), title: `${t('es_operator')} #${esc(x.operator_id)}`, trailing: sum(x) })).join('') || empty('receipt', { key: 'es_none' })}</div></section>`;
}

const lineOf = it => `${esc(it.quantity)}× ${esc(it.name || it.code)}`;

function list(d){
  const all = d.orders || [];
  const rows = L.search(L.ofDay(all, view.day, dayOf), view.q);
  const days = [{ value: '', key: 'es_all' }, ...(d.by_day || []).filter(x => x.orders).slice(-7).reverse().map(x => ({ value: x.day, label: day(x.at_ms + 43_200_000) }))];
  return `
    ${field({ id: 'esQ', type: 'search', key: 'es_search', value: view.q })}
    ${chips({ values: days, value: view.day || '', attr: 'day' })}
    <div class="rows mt-3">${rows.map(r => rowBtn({ cls: 'es-row', data: { sale: r.id }, leading: icon(r.kind === 'dine_in' ? 'tools-kitchen-2' : 'walk'),
      title: `${esc(clock(r.at_ms))} · ${r.table != null ? `${esc(t('es_table'))} ${esc(r.table)}` : esc(t('es_counter'))}`,
      sub: `${esc(day(r.at_ms))} · <span class="mono">#${esc(r.sale_id)}</span> · ${(r.items || []).slice(0, 3).map(lineOf).join(', ')}${(r.items || []).length > 3 ? ' …' : ''}`,
      trailing: `${moneyEl(r.total)}${r.void_of || REFUSED.has(r.status) ? pill('bad', { key: 'es_notTaken' }) : r.kind === 'dine_in' && r.payment_status !== 'paid' ? pill('', { key: 'es_unpaidCourse' }) : ''}` })).join('')
      || empty('receipt', { key: 'es_none' })}</div>
    <div class="btn-row">${btn({ id: 'esCsv', variant: 'ghost', icon: 'download', key: 'es_export' })}</div>`;
}

function wire(root){
  const qi = $('#esQ', root);
  if (qi) qi.oninput = () => { view.q = qi.value; const pane = $('#esPane'); pane.innerHTML = list(view.d); retranslate(pane); hydrate(pane); wire(pane); $('#esQ').focus(); $('#esQ').setSelectionRange(view.q.length, view.q.length); };
  for (const b of $$('[data-day]', root)) b.onclick = () => { view.day = b.dataset.day || null; view.mode = 'list'; paint(); };
  for (const b of $$('.es-row', root)) b.onclick = () => detail(b.dataset.sale);
  const csv = $('#esCsv', root);
  if (csv) csv.onclick = () => {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([L.csv(L.search(L.ofDay(view.d.orders || [], view.day, dayOf), view.q))], { type: 'text/csv' }));
    a.download = `ebills-sales-${store.loc || 'venue'}.csv`; a.click(); URL.revokeObjectURL(a.href);
  };
  const link = $('#esLink', root);
  if (link) link.onclick = async () => (await import('/admin/ebills.js')).open();
}

/// One sale, as the till fiscalised it and the hub keeps it.
function detail(id){
  const r = (view.d?.orders || []).find(x => x.id === id); if (!r) return;
  const paid = r.kind !== 'dine_in' || r.payment_status === 'paid';
  sheet(`
    <p class="eyebrow">${esc(t('es_sale'))} #${esc(r.sale_id)} · ${esc(clock(r.at_ms))} · ${esc(day(r.at_ms))}</p>
    <h2>${r.table != null ? `${esc(t('es_table'))} ${esc(r.table)}` : esc(t('es_counter'))}</h2>
    <div class="fact">${icon('receipt')}<span class="v"><span class="k" data-t="es_invoice"></span><span class="mono">${esc(r.inv_ord_num ?? '—')}</span>${r.fic ? ` · <span class="k" data-t="es_fic"></span><span class="mono">${esc(r.fic)}</span>` : ''}</span></div>
    <div class="fact">${icon(L.payKey(r.payment) === 'card' ? 'credit-card' : 'coin-hole')}<span class="v"><span class="k" data-t="es_payment"></span>${L.payKey(r.payment) ? `<span data-t="es_${L.payKey(r.payment)}"></span>` : esc(r.payment || '—')} · ${paid ? `<span data-t="es_paidByBill"></span>` : `<span data-t="es_unpaidCourse"></span>`}</span></div>
    ${r.operator_id != null ? `<div class="fact">${icon('user')}<span class="v"><span class="k" data-t="es_operator"></span>#${esc(r.operator_id)}</span></div>` : ''}
    ${r.changed ? `<div class="fact">${icon('alert-triangle')}<span class="v" data-t="es_changed"></span></div>` : ''}
    ${r.void_of ? `<div class="fact">${icon('x')}<span class="v"><span class="k" data-t="es_void"></span><span class="mono">${esc(r.void_of)}</span></span></div>` : ''}
    <p class="eyebrow mt-3" data-t="items"></p>
    ${(r.items || []).map(it => `<div class="line"><span class="q">${esc(it.quantity)}×</span><span class="n">${esc(it.name || it.code)}<small><span class="mono">${esc(it.code)}</span> · ${pill(it.product_id ? 'ok' : 'warn', { key: { map: 'es_matchedMap', name: 'es_matchedName', open: 'es_open' }[L.matchOf(it)] })} · ${pill(it.side ? '' : 'warn', { key: 'an_' + (it.side || 'unclassified') })}${it.till_category ? ` <span class="mono">${esc(it.till_category)}</span>` : ''}${it.discount_pct ? ` · −${esc(it.discount_pct)}%` : ''}</small></span>${moneyEl(it.quantity * it.unit_price)}</div>`).join('')}
    <div class="line total"><span class="n" data-t="total"></span>${moneyEl(r.total ?? 0)}</div>
    <p class="muted small mt-3">${esc(ago(r.at_ms))} · <span data-t-st="${esc(r.status || '')}"></span></p>
    <div class="btn-row">${btn({ id: 'esBack', variant: 'ghost', icon: 'chevron-left', key: 'es_list' })}</div>`, { name: 'ebillsSale' });
  $('#esBack').onclick = () => { sheet(`${head}<div id="esBody"></div>`, { name: 'ebillsSales' }); paint(); };
}
