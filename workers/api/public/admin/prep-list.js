// THE PREP LIST SHEET (W-PREP, P6): the day picker and the hub's answer,
// drawn by `prep-list-view.js`. Opened from the Stock screen's tile and from
// the card on the kitchen numbers. The kitchen and the owner read the same
// answer: it carries no money and no person.
//
// ASCII QUOTES ONLY in this file.

import '/admin/prep-list-i18n.js';
import { $, api, sheet, retranslate } from '/admin/core.js';
import { lang } from '/admin/i18n.js';
import { btn, input, empty, loading } from '/admin/parts.js';
import { draw, card } from '/admin/prep-list-view.js';
import { ensureCss } from '/admin/kitchen-analytics.js';

/// The route, with the day when one is picked.
export const prepPath = day => `/staff/kitchen/prep${day ? '?day=' + encodeURIComponent(day) : ''}`;

export function openPrepList(day = ''){
  ensureCss();
  sheet(`<p class="eyebrow" data-t="inv_numbers"></p><h2 data-t="pl_title"></h2><p class="hint" data-t="pl_hint"></p>
    <div class="kt-range">${input({ id: 'pl-day', type: 'date', key: 'pl_day', value: day, tour: 'prepList.day' })}</div>
    <div class="btn-row">${btn({ id: 'plGo', variant: 'primary', icon: 'note', key: 'pl_show' })}</div>
    <div id="plOut">${loading(4)}</div>`, { name: 'prepList' });
  retranslate($('#sheetIn'));
  $('#plGo').onclick = () => load($('#pl-day').value);
  load(day);
}

async function load(day){
  $('#plOut').innerHTML = loading(4);
  let r;
  try { r = await api(prepPath(day)); } catch (e) { $('#plOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  $('#pl-day').value = r.day || day;
  $('#plOut').innerHTML = draw(r, { lang });
  retranslate($('#plOut'));
}

/// The card on the kitchen numbers: today's forecast, or nothing when the
/// read fails (the numbers above it stay).
export async function fillCard(host){
  if (!host) return;
  try { host.innerHTML = card(await api(prepPath(''))); } catch { host.innerHTML = ''; return; }
  retranslate(host);
  const b = host.querySelector('[data-openprep]');
  if (b) b.onclick = () => openPrepList();
}
