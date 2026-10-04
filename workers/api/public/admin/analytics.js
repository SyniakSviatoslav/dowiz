// The owner's numbers (W-HIST P2c, P3): any period (7 / 30 / 90 / 365 days or
// a custom range), the previous period and the same weekday beside it, each
// dish's trend, best and quietest hours, weekday by hour, the channel mix,
// the average check, repeat customers as counts, and the menu-engineering
// matrix. GET /api/owner/analytics (analytics.owner.v2), GET
// /api/owner/analytics/kitchen (its `menu`), POST /api/owner/analytics/history.
//
// EVERY NUMBER OPENS ITS RECORDS: a day, a dish, an hour or a total leads to
// the orders it was added up from (`?trace=`), and an archived day says
// whether it still matches its archive byte for byte. That is the edge over
// a dashboard: a number the owner can follow back to the orders.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, icon, t, api, post, toast, sheet, money, day, clock, busy, hydrate, retranslate } from '/admin/core.js';
import { intlLocale } from '/admin/i18n.js';
import { ui, btn, chips, loading, rowBtn, rowDiv, empty, pill, input } from '/admin/parts.js';
import '/admin/analytics-i18n.js';
import * as L from '/admin/analytics-logic.js';
import { weekTopCard } from '/admin/menu-flags.js';

const view = { p: { days: L.WINDOWS[0] }, a: null };
const fail = e => toast(String(e.message || e));
const head = (title = 'analytics', hint = 'ap_h_analytics') => `<p class="eyebrow" data-t="analytics"></p><h2 data-t="${title}"></h2><p class="sheet-hint" data-t="${hint}"></p>`;
const dayMs = s => L.utcDayMs(s, 12);
const showDay = s => esc(day(dayMs(s)));
const weekdayName = i => new Date(Date.UTC(2024, 0, 1 + i)).toLocaleDateString(intlLocale(), { weekday: 'short', timeZone: 'UTC' });
const hh = h => String(h).padStart(2, '0') + ':00';
const signed = n => (n > 0 ? '+' : '') + money(n);
const dishName = id => {
  const a = view.a || {};
  const d = [...((a.trend && a.trend.dishes) || []), ...(a.topProducts || [])].find(x => x.id === id);
  return d ? (typeof d.name === 'string' ? d.name : id) : id;
};

function paint(){
  const root = $('#sheetIn'); retranslate(root); hydrate(root);
  // The heat grid is drawn through the CSSOM: a style attribute would meet the CSP.
  for (const g of $$('[data-heat]', root)) { g.style.display = 'grid'; g.style.gridTemplateColumns = 'repeat(24, 1fr)'; g.style.gap = '2px'; }
  for (const c of $$('[data-l]', root)) { c.style.height = '12px'; c.style.borderRadius = '2px'; c.style.background = 'currentColor'; c.style.opacity = String(0.08 + 0.23 * Number(c.dataset.l)); }
}

/// The analytics sheet for period `p` ({ days } or { from, to }).
export async function open(p = view.p){
  view.p = p;
  const sel = p.from ? 'custom' : p.days;
  const wins = [...L.WINDOWS.map(w => ({ value: w, key: { 7: 'week', 30: 'month', 90: 'anQuarter', 365: 'anYear' }[w] })), { value: 'custom', key: 'anCustom' }];
  sheet(`${head()}${chips({ values: wins, value: sel, attr: 'win', tour: 'analytics.window' })}
    <div id="anRange" class="grid2"${p.from ? '' : ' hidden'}>${input({ id: 'an-from', type: 'date', key: 'anFrom', value: p.from || '' })}${input({ id: 'an-to', type: 'date', key: 'anTo', value: p.to || '' })}
      <div class="btn-row">${btn({ id: 'anGo', icon: 'check', key: 'anApply' })}</div></div>
    <div id="anBody">${loading(3)}</div>`, { name: 'analytics', keepScroll: true });
  for (const b of $$('[data-win]', $('#sheetIn'))) b.onclick = () => (b.dataset.win === 'custom' ? ($('#anRange').hidden = false) : open({ days: Number(b.dataset.win) }));
  $('#anGo').onclick = () => {
    const from = $('#an-from').value, to = $('#an-to').value, bad = L.badRange(from, to);
    return bad ? toast(t(bad)) : open({ from, to });
  };
  let a; try { a = await api(`/owner/analytics?${L.query(p)}`); } catch (e) { return fail(e); }
  view.a = a;
  $('#anBody').innerHTML = `<div id="anExplain"></div>${body(a)}`;
  paint();
  bind(a);
  matrix(p, a);
  // "What the numbers say" (W-AI): templates from this answer; AI only rewords on the owner's tap.
  import('/admin/ai.js').then(m => m.explainCard($('#anExplain'), 'analytics', (p.days || 7) >= 30 ? 30 : 7)).catch(() => {});
}

function stat(key, value, delta, extra = ''){
  const d = delta ? `<small class="${L.tone(delta.n)}">${esc(delta.text)}</small>` : '';
  return `<div class="stat"><small data-t="${key}"></small><b>${value}</b>${d}${extra}</div>`;
}

function body(a){
  const c = a.compare || {}, pv = c.prev || {}, wd = c.weekday || {}, dpm = pv.deltaPm || {}, dd = pv.delta || {};
  const byDay = a.byDay || [], maxRev = Math.max(1, ...byDay.map(d => d.revenue || 0));
  const byHour = a.byHour || [], maxH = Math.max(1, ...byHour);
  const grid = a.byWeekdayHour || [], maxG = Math.max(1, ...grid.flat());
  const hours = a.hours || {}, rep = a.repeat || {}, hist = a.history || {};
  const archived = byDay.filter(d => d.archived > 0).length;
  const hourRow = h => rowBtn({ leading: icon('clock'), title: hh(h.hour), data: { hour: h.hour }, sub: `<span class="mono">${esc(h.orders)} · ${esc(money(h.revenue))}</span>`, trailing: icon('chevron-right', 'chev') });
  const trend = ((a.trend && a.trend.dishes) || []).map(d => rowBtn({ leading: `<svg class="spark" viewBox="0 0 80 20" width="80" height="20" aria-hidden="true"><polyline points="${L.spark(d.series)}" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>`,
    title: typeof d.name === 'string' ? d.name : d.id, data: { dish: d.id },
    sub: `<span class="mono">${esc(d.quantity)} · ${esc(money(d.revenue))} · <span data-t="anBefore"></span> ${esc(d.prevQuantity)}</span>`,
    trailing: `<small class="${L.tone(d.quantity - d.prevQuantity)}">${esc(L.pct(d.prevQuantity ? Math.trunc(((d.quantity - d.prevQuantity) * 1000) / d.prevQuantity) : null))}</small>` })).join('');
  return `
    <p class="muted small" data-t="anTapRecords"></p>
    <div class="stats">${stat('orders7', esc(a.orders ?? 0), { n: dd.orders, text: L.pct(dpm.orders) })}${stat('revenue7', esc(money(a.revenue || 0)), { n: dd.revenue, text: L.pct(dpm.revenue) })}
      ${stat('avgCheck', esc(money(a.averageOrder || 0)), { n: dd.averageOrder, text: dd.averageOrder ? signed(dd.averageOrder) : '' })}${stat('rejected', esc(a.rejected ?? 0), null)}</div>
    ${ui.button({ variant: 'ghost', icon: 'chevron-right', label: { t: 'anDays' }, cls: 'mt-3', attrs: { data: { 'days-list': '1' } } })}
    ${weekTopCard(a.weekTop)}
    <p class="muted small"><span data-t="anVsPrev"></span>: ${esc(pv.from || '')} - ${esc(pv.to || '')} · ${esc(pv.orders ?? 0)} · ${esc(money(pv.revenue || 0))}</p>
    <section class="group mt-3"><p class="eyebrow" data-t="anWeekday"></p><div class="rows">
      ${rowBtn({ leading: icon('clock'), title: `${weekdayName(wd.weekday || 0)} ${showDay(wd.day || a.to)}`, data: { day: wd.day || a.to }, sub: '<span data-t="anLastDay"></span>', trailing: `<b class="mono">${esc(wd.orders ?? 0)} · ${esc(money(wd.revenue || 0))}</b>` })}
      ${rowBtn({ leading: icon('clock'), title: showDay((wd.lastWeek || {}).day || a.to), data: { day: (wd.lastWeek || {}).day || a.to }, sub: '<span data-t="anLastWeek"></span>', trailing: `<b class="mono">${esc((wd.lastWeek || {}).orders ?? 0)} · ${esc(money((wd.lastWeek || {}).revenue || 0))}</b>` })}
      ${rowDiv({ leading: icon('chart-bar'), title: { t: 'anAvg4' }, trailing: `<b class="mono">${esc((wd.average || {}).orders ?? 0)} · ${esc(money((wd.average || {}).revenue || 0))}</b>` })}</div></section>
    <p class="eyebrow mt-3" data-t="byDay"></p><div class="bars" role="img" aria-label="${esc(t('byDay'))}">${byDay.map(d => `<i data-day="${esc(d.day)}" data-h="${Math.round(100 * (d.revenue || 0) / maxRev)}" title="${showDay(d.day)} · ${esc(money(d.revenue || 0))}"></i>`).join('')}</div>
    <div class="axis"><span>${byDay.length ? showDay(byDay[0].day) : ''}</span><span>${byDay.length ? showDay(byDay[byDay.length - 1].day) : ''}</span></div>
    <section class="group mt-3"><p class="eyebrow" data-t="anBestHours"></p><div class="rows">${(hours.best || []).map(hourRow).join('')}</div>
      <p class="eyebrow" data-t="anWorstHours"></p><div class="rows">${(hours.worst || []).map(hourRow).join('')}</div></section>
    <p class="eyebrow mt-3" data-t="byHour"></p><div class="bars" role="img" aria-label="${esc(t('byHour'))}">${byHour.map((n, h) => `<i data-hour="${h}" class="${n === maxH ? 'hi' : ''}" data-h="${Math.round(100 * n / maxH)}" title="${hh(h)} · ${n}"></i>`).join('')}</div><div class="axis"><span>00</span><span>12</span><span>23</span></div>
    <p class="eyebrow mt-3" data-t="anHeat"></p><div>${grid.map((r, i) => `<p class="muted small mono">${esc(weekdayName(i))}</p><div data-heat="1">${r.map((n, h) => `<i data-hour="${h}" data-l="${L.heat(n, maxG)}" title="${esc(weekdayName(i))} ${hh(h)} · ${n}"></i>`).join('')}</div>`).join('')}</div>
    <section class="group mt-3"><p class="eyebrow" data-t="anChannels"></p><div class="rows">${(a.channels || []).map(c => rowDiv({ leading: icon('scroll'), title: c.channel,
      sub: `<span class="mono">${esc(c.orders)} · ${esc(L.pct(c.sharePm))} <span data-t="anShare"></span></span>`, trailing: ui.amount(money(c.revenue || 0)) })).join('')}
      ${rowDiv({ leading: icon('bike'), title: { t: 'delivery' }, trailing: `<b class="mono">${esc(a.delivery ?? 0)}</b>` })}${rowDiv({ leading: icon('walk'), title: { t: 'pickup' }, trailing: `<b class="mono">${esc(a.pickup ?? 0)}</b>` })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="anTrend"></p><div class="rows">${trend || empty('bowl-chopsticks', { key: 'anMenuEmpty' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="anRepeat"></p><p class="muted small" data-t="anRepeatHint"></p><div class="rows">
      ${rowDiv({ leading: icon('user'), title: { t: 'anCustomers' }, sub: `<span class="mono">${esc(rep.from || '')} - ${esc(rep.to || '')}</span>`, trailing: `<b class="mono">${esc(rep.customers ?? 0)}</b>` })}
      ${rowDiv({ leading: icon('user-plus'), title: { t: 'anCameBack' }, trailing: `<b class="mono">${esc(rep.repeatCustomers ?? 0)}</b>` })}
      ${rowDiv({ leading: icon('receipt'), title: { t: 'anRepeatShare' }, trailing: `<b class="mono">${esc(rep.repeatOrders ?? 0)} / ${esc(rep.orders ?? 0)} · ${esc(L.pct(rep.sharePm))}</b>` })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="anMenu"></p><p class="muted small" data-t="anMenuHint"></p><div id="anMenu">${loading(2)}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="anHistory"></p><p class="muted small" data-t="anHistoryHint"></p><div class="rows">
      ${rowDiv({ leading: icon('cloud-upload'), title: { t: 'anArchivedDays' }, trailing: `<b class="mono">${esc(archived)}</b>` })}
      ${rowDiv({ leading: icon('check'), title: { t: 'anFolded' }, trailing: `<b class="mono">${esc(hist.folded ?? 0)}</b>` })}
      ${(hist.pending || []).length ? rowDiv({ leading: icon('alert-triangle'), title: { t: 'anPending' }, trailing: pill('warn', { label: String(hist.pending.length) }) }) : ''}
      ${hist.error ? rowDiv({ leading: icon('alert-triangle'), title: { t: 'anHistoryError' }, sub: esc(hist.error) }) : ''}</div>
      <p class="muted small" id="anCheck"></p>
      <div class="btn-row">${btn({ id: 'anFold', icon: 'cloud-upload', key: 'anFold' })}${btn({ id: 'anVerify', icon: 'check', key: 'anVerify' })}${btn({ id: 'anCsv', icon: 'download', key: 'anExport' })}</div></section>`;
}

function bind(a){
  const root = $('#anBody');
  root.onclick = e => {
    const el = e.target.closest('[data-day],[data-dish],[data-hour],[data-days-list]');
    if (!el) return;
    if (el.dataset.day) return trace(el.dataset.day, {});
    if (el.dataset.dish) return days({ dish: el.dataset.dish });
    if (el.dataset.hour != null && el.dataset.hour !== '') return days({ hour: Number(el.dataset.hour) });
    return days({});
  };
  $('#anFold').onclick = async () => {
    try { const r = await busy($('#anFold'), () => post('/owner/analytics/history', {})); toast(`${t('anFolded')}: ${r.folded ?? 0}`); open(view.p); } catch (e) { fail(e); }
  };
  $('#anVerify').onclick = async () => {
    try { const r = await busy($('#anVerify'), () => api('/owner/analytics?v=2&verify=1')); traceSheet(r, {}); } catch (e) { $('#anCheck').textContent = String(e.message || e); }
  };
  $('#anCsv').onclick = () => {
    const blob = new Blob(['\uFEFF' + L.csv(a)], { type: 'text/csv;charset=utf-8' });
    const x = document.createElement('a'); x.href = URL.createObjectURL(blob); x.download = `dowiz-analytics-${a.from}-${a.to}.csv`; x.click(); requestAnimationFrame(() => URL.revokeObjectURL(x.href));
  };
}

/// The period's days that had orders, newest first; each opens its records,
/// filtered to a dish or an hour when one was tapped.
function days(f){
  const list = ((view.a && view.a.byDay) || []).filter(d => d.orders > 0).slice().reverse();
  const what = f.dish ? dishName(f.dish) : f.hour != null ? hh(f.hour) : '';
  sheet(`${head('anRecords', 'anRecordsHint')}${what ? `<p class="mono">${esc(what)}</p>` : ''}
    <div class="rows">${list.map(d => rowBtn({ leading: icon('clock'), title: showDay(d.day), data: { day: d.day },
      sub: `<span class="mono">${esc(d.orders)} · ${esc(money(d.revenue || 0))}${d.archived ? ` · ${esc(t('anFromArchive'))} ${esc(d.archived)}` : ''}</span>`, trailing: icon('chevron-right', 'chev') })).join('') || empty('clock', { key: 'anNoRecords' })}</div>
    <div class="btn-row">${btn({ id: 'anBack', icon: 'arrow-left', key: 'analytics' })}</div>`, { name: 'analyticsDays' });
  paint();
  for (const b of $$('[data-day]', $('#sheetIn'))) b.onclick = () => trace(b.dataset.day, f);
  $('#anBack').onclick = () => open(view.p);
}

/// One day's records and the archive's check of it.
async function trace(d, f){
  sheet(`${head('anRecords', 'anRecordsHint')}<div id="anTrace">${loading(3)}</div>`, { name: 'analyticsTrace' });
  let r; try { r = await api(`/owner/analytics?v=2&trace=${encodeURIComponent(d)}`); } catch (e) { return fail(e); }
  traceSheet(r, f);
}

function traceSheet(r, f){
  const ar = r.archived || {}, recs = L.keep(r.records, f);
  const check = ar.stored ? rowDiv({ leading: icon(ar.equal ? 'circle-check' : 'alert-triangle'), title: { t: ar.equal ? 'anVerified' : 'anMismatch' }, sub: `<span class="mono">${esc((ar.src || []).join(', '))}</span>` }) : '';
  sheet(`${head('anRecords', 'anRecordsHint')}<p class="mono">${showDay(r.day)}${f.dish ? ' · ' + esc(dishName(f.dish)) : ''}${f.hour != null ? ' · ' + hh(f.hour) : ''}</p>
    <div class="rows">${check}${recs.map(o => rowDiv({ leading: icon('receipt'), title: `${clock(o.at)} · ${o.channel} · ${o.kind}`,
      sub: `<span class="mono">${esc(String(o.id).slice(0, 8))} · ${esc(o.status)} · ${esc(o.from === 'hot' ? t('anFromHot') : t('anFromArchive') + ' ' + o.from)}<br>${(o.items || []).map(i => `${esc(i.quantity)} x ${esc(dishName(i.id))}`).join(', ')}</span>`,
      trailing: ui.amount(money(o.took || 0)) })).join('') || empty('receipt', { key: 'anNoRecords' })}</div>
    <div class="btn-row">${btn({ id: 'anBack', icon: 'arrow-left', key: 'analytics' })}</div>`, { name: 'analyticsTrace' });
  paint();
  $('#anBack').onclick = () => open(view.p);
}

/// The menu-engineering matrix, over the kitchen window of the same period.
async function matrix(p, a){
  const host = $('#anMenu'); if (!host) return;
  const kq = L.kitchenQuery(p, a);
  let k; try { k = await api(`/owner/analytics/kitchen?${kq.query}`); } catch (e) { host.innerHTML = `<p class="warn small">${esc(e.message || e)}</p>`; return; }
  const m = k.menu || {}, g = L.byQuadrant(m);
  if (!(m.dishes || []).length) { host.innerHTML = empty('bowl-chopsticks', { key: 'anMenuEmpty' }); paint(); return; }
  const chip = d => rowBtn({ leading: icon('bowl-chopsticks'), title: d.name || d.id, data: { mm: d.id },
    sub: `<span class="mono">${esc(d.sold)} · ${esc(L.pct(d.mixPm))}${d.costUnknown ? '' : ` · <span data-t="mmMargin"></span> ${esc(money(d.marginPortion || 0))} · <span data-t="mmBasis_${esc(d.costBasis)}"></span>`}</span><span class="ui-row-sub" data-advice="${esc(d.id)}" hidden></span>`,
    trailing: icon('chevron-down', 'chev') });
  host.innerHTML = `${kq.clipped ? '<p class="muted small" data-t="anMenuClipped"></p>' : ''}
    ${m.averageMarginPortion != null ? `<p class="muted small"><span data-t="mmAvg"></span>: ${esc(money(m.averageMarginPortion))}</p>` : ''}
    ${[...L.QUADRANTS, 'unknown'].filter(q => g[q].length).map(q => `<p class="eyebrow mt-3" data-t="mm${q[0].toUpperCase() + q.slice(1)}"></p><div class="rows">${g[q].map(chip).join('')}</div>`).join('')}`;
  paint();
  for (const b of $$('[data-mm]', host)) b.onclick = () => {
    const d = (m.dishes || []).find(x => x.id === b.dataset.mm), out = $(`[data-advice="${CSS.escape(b.dataset.mm)}"]`, host);
    if (!d || !out) return;
    const s = L.advice(d, money);
    out.textContent = L.fill(t(s.key), s.params);
    out.hidden = !out.hidden;
  };
}
