// KITCHEN NUMBERS, DRAWN: the hub's answer (`GET /api/owner/analytics/kitchen`)
// in, markup out. PURE -- `money` and `t` are handed in, nothing is fetched,
// no clock is read -- so every table renders in node (`kitchen-view.test.mjs`).
//
// ASCII QUOTES ONLY in this file.

import * as C from './ingredients-calc.js';
import { ui, empty } from './parts.js';

/// A dish whose food cost passes this (per mille of its price) draws its bar
/// in the warning colour; an ingredient under this many days of cover, too.
export const FOOD_COST_WARN_PM = 400;
export const COVER_WARN_DAYS = 3;

const esc = ui.esc;
const th = keys => `<thead><tr>${keys.map(x => `<th data-t="${x}"></th>`).join('')}</tr></thead>`;
const tbl = (head, rows, foot = '') => `<div class="kt-wrap"><table class="kt">${th(head)}<tbody>${rows}</tbody>${foot}</table></div>`;
const cell = (v, cls = '') => `<td class="${cls}">${v == null ? '-' : v}</td>`;
const bar = (w, warn) => `<span class="kt-bar"><i class="${warn ? 'warn' : ''}" data-w="${w}"></i></span>`;
const sign = v => (v < 0 ? 'neg' : v > 0 ? 'pos' : '');

function totals(r, money){
  const x = r.totals || {};
  const stat = (v, word, cls = 'money') => `<div class="stat ${cls}"><b>${v}</b><small data-t="${word}"></small></div>`;
  return `<div class="stats kt-tot">${stat(money(x.revenue || 0), 'ka_revenue')}${stat(money(x.cogs || 0), 'ka_cogs')}${stat(C.pct(x.foodCostPm) || '-', 'ka_foodCost', '')}${stat(money(x.margin || 0), 'ka_margin')}
    ${stat(money(x.wasteValue || 0), 'ka_waste')}${stat(money(x.driftValue || 0), 'ka_drift')}${stat(money(x.receivedValue || 0), 'ka_received')}${stat(x.orders || 0, 'ka_orders', '')}</div>
    ${[['undated', 'ka_undated'], ['unmodelled', 'ka_unmodelled'], ['uncosted', 'ka_uncosted']].filter(([k2]) => x[k2]).map(([k2, w]) => `<p class="hint">${x[k2]} <span data-t="${w}"></span></p>`).join('')}`;
}

function byDay(r, m){
  const days = r.byDay || [];
  const w = C.bars(days.map(d => d.revenue));
  const rows = days.map((d, i) => `<tr><td>${esc(d.day)}${bar(w[i])}</td>${cell(d.orders)}${cell(m(d.revenue))}${cell(m(d.cogs))}${cell(C.pct(d.foodCostPm) || '-')}${cell(m(d.waste))}${cell(m(d.received))}</tr>`).join('');
  return tbl(['ka_day', 'ka_orders', 'ka_revenue', 'ka_cogs', 'ka_foodCost', 'ka_waste', 'ka_received'], rows);
}

function byDish(r, m){
  const dishes = r.dishes || [];
  const w = C.bars(dishes.map(d => d.revenue));
  const rows = dishes.map((d, i) => `<tr><td>${esc(d.name)}${d.hasRecipe ? '' : ` <small data-t="inv_noStockLink"></small>`}${bar(w[i], (d.foodCostPm || 0) > FOOD_COST_WARN_PM)}</td>${cell(d.sold)}${cell(m(d.revenue))}${cell(m(d.portionCost))}
    ${cell(m(d.marginPortion), sign(d.marginPortion))}${cell(m(d.margin), sign(d.margin))}${cell(C.pct(d.foodCostPm) || '-')}</tr>`).join('');
  return tbl(['ka_dish', 'ka_sold', 'ka_revenue', 'ka_portionCost', 'ka_marginPortion', 'ka_margin', 'ka_foodCost'], rows);
}

function byIngredient(r, m){
  const list = r.ingredients || [];
  const w = C.bars(list.map(i => i.cost || 0));
  const rows = list.map((i, n) => `<tr><td>${esc(i.name)}${bar(w[n])}</td>${cell(`${i.used} ${esc(i.unit || '')}`)}${cell(i.drawn)}${cell(i.wasted)}${cell(i.drift, sign(i.drift))}
    ${cell(m(i.cost))}${cell(i.cleanLossG ? i.cleanLossG + ' g' : '')}${cell(i.cookLossG ? i.cookLossG + ' g' : '')}${cell(i.daysCover, i.daysCover != null && i.daysCover < COVER_WARN_DAYS ? 'neg' : '')}${cell(i.reorder != null ? `${i.reorder} ${esc(i.unit || '')}` : '')}</tr>`).join('');
  return `<p class="hint" data-t="ka_usedHint"></p>` + tbl(['ka_ingredient', 'ka_used', 'ka_drawn', 'inv_waste', 'ka_drift', 'ka_cogs', 'ka_cleanLoss', 'ka_cookLoss', 'ka_cover', 'ka_reorder'], rows);
}

function waste(r, m, t){
  const list = r.waste || [];
  if (!list.length) return `<p class="hint" data-t="none"></p>`;
  const w = C.bars(list.map(x => x.value));
  return tbl(['reason', 'inv_lines', 'inv_value'], list.map((x, i) => `<tr><td>${esc(t(x.reason))}${bar(w[i], true)}</td>${cell(x.rows)}${cell(m(x.value))}</tr>`).join(''));
}

function yields(r, t){
  const list = r.yields || [];
  if (!list.length) return `<p class="hint" data-t="none"></p>`;
  return tbl(['ka_day', 'ka_ingredient', 'inv_stage', 'inv_qtyIn', 'inv_qtyOut', 'inv_measured', 'inv_expected'], list.map(y => `<tr><td>${esc(y.day)}</td><td>${esc(y.name || y.item)}</td>
    <td>${esc(t('inv_stage_' + y.stage))}</td>${cell(y.qty)}${cell(y.out)}${cell(C.pct(y.measuredPm), y.diffPm < 0 ? 'neg' : '')}${cell(C.pct(y.expectedPm))}</tr>`).join(''));
}

function prices(r, money){
  const list = r.prices || [];
  if (!list.length) return `<p class="hint" data-t="none"></p>`;
  return tbl(['ka_ingredient', 'inv_prices', 'ka_change'], list.map(p => `<tr><td>${esc(p.name)}</td>
    <td>${(p.points || []).map(x => `${esc(x.day)}: ${x.perBasis != null ? money(x.perBasis) : '-'}${x.supplier ? ' (' + esc(x.supplier) + ')' : ''}`).join('<br>')}</td>${cell(C.pct(p.changePm) || '-', sign(p.changePm || 0) === 'pos' ? 'neg' : '')}</tr>`).join(''));
}

/// The whole report. `fmt` is `{ money, t }`.
export function draw(r, { money, t }){
  const m = v => (v == null ? null : money(v));
  const sec = (word, html) => `<p class="eyebrow mt-3" data-t="${word}"></p>${html}`;
  if (!(r.byDay || []).some(d => d.orders) && !(r.ingredients || []).length) return totals(r, money) + empty('chart-bar', { key: 'ka_none' });
  return totals(r, money) + sec('ka_byDay', byDay(r, m)) + sec('ka_byDish', byDish(r, m)) + sec('ka_ingredients', byIngredient(r, m)) + sec('ka_waste', waste(r, m, t))
    + sec('ka_yields', yields(r, t)) + sec('ka_prices', prices(r, money));
}
