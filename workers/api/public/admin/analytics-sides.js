// KITCHEN vs DRINKS (W-DRINKS-KITCHEN, operator 2026-11-04: "розділи
// аналітику на напої та кухню"): one section both owner screens draw -- the
// analytics pane over its period and the till tab over its thirty days --
// from the hub's `sides` answer (`services/analytics/split.rs`). The sushi
// business and the bar business as two sets of numbers: sales and share per
// side, the checks that had a kitchen line against the drinks-only checks
// (each with its own average), each side's top items, and the two by day.
// Lines no rule can place are shown as "unclassified" with their money, so
// nothing is silently put on a side. The hub did every sum; this file draws.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { esc, icon, t, money } from '/admin/core.js';
import { ui, rowDiv, empty } from '/admin/parts.js';
import '/admin/analytics-i18n.js';
import * as L from '/admin/analytics-logic.js';

const ICON = { kitchen: 'bowl-chopsticks', drinks: 'cup', unclassified: 'help-circle' };
const SIDES = ['kitchen', 'drinks', 'unclassified'];

const stat = (key, value, sub = '') => `<div class="stat"><small data-t="${key}"></small><b>${value}</b>${sub ? `<small>${sub}</small>` : ''}</div>`;

/// One side's top items (at most five), dearest first.
function top(side, s){
  const rows = (s.top || []).slice(0, 5).map(x => rowDiv({ leading: icon(ICON[side]), title: typeof x.name === 'string' && x.name ? x.name : String(x.id || ''),
    sub: `<span class="mono">${esc(x.quantity ?? 0)} <span data-t="anPortions"></span></span>`, trailing: ui.amount(money(x.sales || 0)) })).join('');
  return rows || empty(ICON[side], { key: 'anSideNone' });
}

/// A check group: how many, their average, and what they took.
const check = (key, side, c = {}) => rowDiv({ leading: icon(ICON[side]), title: { t: key },
  sub: `<span class="mono">${esc(c.orders ?? 0)} · <span data-t="avgCheck"></span> ${esc(money(c.averageCheck || 0))}</span>`, trailing: ui.amount(money(c.revenue || 0)) });

/// The bars of one side by day, against the tallest day of ANY side, so the
/// two rows read on one scale. `showDay` words a `YYYY-MM-DD` for the title.
function bars(side, byDay, heights, showDay){
  return `<p class="eyebrow mt-3"><span data-t="anSideDay"></span> · <span data-t="an_${side}"></span></p>
    <div class="bars" role="img" aria-label="${esc(t('an_' + side))}">${byDay.map((d, i) => `<i data-h="${heights[side][i]}" title="${showDay(d.day)} · ${esc(money(d[side] || 0))}"></i>`).join('')}</div>`;
}

/// The whole section, or nothing when the hub sent no split (an older hub).
/// `showDay` is the caller's day formatter (the pane and the tab word days differently).
export function sidesSection(sp, showDay = s => esc(String(s || ''))){
  if (!sp) return '';
  const o = sp.orders || {}, byDay = sp.byDay || [], heights = L.sideBars(byDay);
  const open = sp.unclassified || {};
  return `<section class="group mt-3"><p class="eyebrow" data-t="anSides"></p><p class="muted small" data-t="anSidesHint"></p>
    <div class="stats">${SIDES.map(s => stat('an_' + s, esc(money((sp[s] || {}).sales || 0)), esc(L.share((sp[s] || {}).sharePm)))).join('')}</div>
    <div class="rows">${check('anKitchenChecks', 'kitchen', o.kitchen)}${check('anDrinksOnlyChecks', 'drinks', o.drinksOnly)}
      ${(o.unclassified || {}).orders ? check('anUnclassifiedChecks', 'unclassified', o.unclassified) : ''}</div>
    <p class="muted small"><span data-t="anChecksFrom"></span>: ${showDay(o.from)} - ${showDay(o.to)}</p>
    ${byDay.length ? bars('kitchen', byDay, heights, showDay) + bars('drinks', byDay, heights, showDay) : ''}
    <div class="axis"><span>${byDay.length ? showDay(byDay[0].day) : ''}</span><span>${byDay.length ? showDay(byDay[byDay.length - 1].day) : ''}</span></div>
    <p class="eyebrow mt-3"><span data-t="anSideTop"></span> · <span data-t="an_kitchen"></span></p><div class="rows">${top('kitchen', sp.kitchen || {})}</div>
    <p class="eyebrow mt-3"><span data-t="anSideTop"></span> · <span data-t="an_drinks"></span></p><div class="rows">${top('drinks', sp.drinks || {})}</div>
    ${open.quantity ? `<p class="eyebrow mt-3"><span data-t="an_unclassified"></span></p><p class="muted small" data-t="anUnclassifiedHint"></p><div class="rows">${top('unclassified', open)}</div>` : ''}
  </section>`;
}
