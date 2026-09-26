// KITCHEN NUMBERS (card I7): for a range of the venue's days, by DAY and by
// DISH -- consumption per ingredient (by recipes, and as recorded), cost of
// goods and food cost, margin per dish, waste by reason and value, raw ->
// cooked losses, measured yields against the defaults, supplier prices, days
// of cover and what to order. Tables first, thin bars inside them.
//
// Every number is the hub's (`GET /api/owner/analytics/kitchen`); this file
// only draws. The day boundaries are the venue's, decided by the server: the
// range picker sends dates as typed and draws the dates the server answers.

import '/admin/ingredients-i18n.js';
import { $, $$, esc, t, api, sheet, money, retranslate, hydrate, repaintMoney } from '/admin/core.js';
import { btn, input, chips, press, empty, loading } from '/admin/parts.js';
import { draw } from '/admin/kitchen-view.js';

const range = { from: '', to: '', days: 7 };

/// The screens' own stylesheet, loaded once (the CSP allows no inline style).
export function ensureCss(){
  if (document.getElementById('invCss')) return;
  const l = document.createElement('link');
  l.id = 'invCss'; l.rel = 'stylesheet'; l.href = '/admin/ingredients.css';
  document.head.append(l);
}

export function openKitchen(){
  ensureCss();
  sheet(`<p class="eyebrow" data-t="inv_numbers"></p><h2 data-t="ka_title"></h2><p class="hint" data-t="ka_hint"></p>
    ${chips({ id: 'kaQuick', values: [{ value: 7, key: 'ka_7' }, { value: 30, key: 'ka_30' }], value: range.from ? '' : range.days, attr: 'kd' })}
    <div class="kt-range">${input({ id: 'ka-from', type: 'date', key: 'ka_from', value: range.from })}${input({ id: 'ka-to', type: 'date', key: 'ka_to', value: range.to })}</div>
    <div class="btn-row">${btn({ id: 'kaGo', variant: 'primary', icon: 'chart-bar', key: 'ka_show' })}</div>
    <div id="kaOut">${loading(4)}</div>`, { name: 'kitchen' });
  for (const b of $$('[data-kd]', $('#sheetIn'))) b.onclick = () => { range.days = Number(b.dataset.kd); range.from = ''; range.to = ''; press($$('[data-kd]', $('#sheetIn')), b); load(); };
  $('#kaGo').onclick = () => { range.from = $('#ka-from').value; range.to = $('#ka-to').value; press($$('[data-kd]', $('#sheetIn')), null); load(); };
  load();
}

async function load(){
  const q = range.from ? `from=${encodeURIComponent(range.from)}&to=${encodeURIComponent(range.to)}` : `days=${range.days}${range.to ? '&to=' + encodeURIComponent(range.to) : ''}`;
  $('#kaOut').innerHTML = loading(4);
  let r;
  try { r = await api(`/owner/analytics/kitchen?${q}`); } catch (e) { $('#kaOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  $('#ka-from').value = r.from; $('#ka-to').value = r.to;
  $('#kaOut').innerHTML = draw(r, { money, t });
  retranslate($('#kaOut')); hydrate($('#kaOut')); repaintMoney($('#kaOut'));
}
