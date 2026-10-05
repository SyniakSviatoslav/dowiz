// THE PREP LIST, DRAWN (W-PREP, P6): the hub's answer
// (`GET /api/staff/kitchen/prep`, kitchen.prep_forecast.v1) in, markup out.
// PURE -- nothing is fetched, no clock is read -- so every part renders in
// node (`prep-list-view.test.mjs`). The words are `data-t` keys
// (`prep-list-words.js`); the numbers are the hub's, never computed here.
//
// A number the hub has not got ("learning") is drawn as the word, never as
// a zero: an empty prep list and an unknown one look different on purpose.
//
// ASCII QUOTES ONLY in this file.

import { ui, empty, btn } from './parts.js';

const esc = ui.esc;
const w = k => `<span data-t="${k}"></span>`;
const th = keys => `<thead><tr>${keys.map(x => `<th data-t="${x}"></th>`).join('')}</tr></thead>`;
const tbl = (head, rows) => `<div class="kt-wrap"><table class="kt">${th(head)}<tbody>${rows}</tbody></table></div>`;
const cell = (v, cls = '') => `<td class="${cls}">${v == null ? '-' : v}</td>`;

/// A name as the menu stores it: text, or a map of languages.
export function nameOf(n, lang = 'en'){
  if (typeof n === 'string') return n;
  if (n && typeof n === 'object') return n[lang] || n.en || Object.values(n).find(x => typeof x === 'string') || '';
  return '';
}

/// A forecast number, or the word "learning".
export const num = e => (e && !e.learning && e.value != null ? String(e.value) : w('pl_learning'));

/// "usually off by +-n portions" when the hub measured it.
export function offBy(p){
  if (!p || p.offBy == null) return '';
  const naive = p.naiveOffBy != null ? ` <small>(${w('pl_naive')} &plusmn;${p.naiveOffBy})</small>` : '';
  return `<p class="hint" data-tour="prepList.error">${w('pl_offBy')} &plusmn;${p.offBy} ${w('pl_portions')}${naive}</p>`;
}

function summary(r){
  const stat = (v, word) => `<div class="stat"><b>${v}</b><small data-t="${word}"></small></div>`;
  return `<div class="stats kt-tot" data-tour="prepList.summary">${stat(num(r.orders), 'pl_orders')}${stat(num(r.portions), 'pl_portions')}${stat(r.portions?.weeks ?? 0, 'pl_weeks')}${stat(r.bookings?.covers ?? 0, 'pl_guests')}</div>${offBy(r.portions)}`;
}

function bands(r){
  if (r.hours && r.hours.known && !r.hours.open) return `<p class="hint">${w('pl_closed')}</p>`;
  const note = r.hours && !r.hours.known ? `<p class="hint">${w('pl_hoursUnknown')}</p>` : '';
  return note + tbl(['pl_bands', 'pl_orders'], (r.bands || []).map(b => `<tr><td>${esc(b.from)}-${esc(b.to)}</td>${cell(num(b))}</tr>`).join(''));
}

function dishes(r, lang){
  return tbl(['pl_dish', 'pl_portions', 'pl_offBy', 'pl_fromBookings'], (r.dishes || []).map(d => `<tr><td>${esc(nameOf(d.name, lang))}</td>${cell(num(d))}${cell(d.offBy != null ? '&plusmn;' + d.offBy : null)}${cell(d.fromBookings || null)}</tr>`).join(''));
}

const amount = x => `${x.qty} ${esc(x.unit || '')}`;

function preps(r, lang){
  const list = r.preps || [];
  if (!list.length || list.every(p => !p.make)) return `<p class="hint" data-tour="prepList.preps">${w('pl_nothingToMake')}</p>`;
  const rows = list.map(p => `<tr><td>${esc(nameOf(p.name, lang))}</td>${cell(`${p.need} ${esc(p.unit || '')}`)}${cell(p.onHand)}${cell(`<b>${p.make}</b>`, p.make > 0 ? 'neg' : '')}
    <td>${p.error ? `<small class="neg">${esc(p.error)}</small>` : (p.from || []).map(x => `${esc(nameOf(x.name, lang))} ${amount(x)}`).join('<br>')}</td></tr>`).join('');
  return `<div data-tour="prepList.preps">${tbl(['pl_prep', 'pl_need', 'pl_onHand', 'pl_make', 'pl_from'], rows)}</div>`;
}

function raw(r, lang){
  const list = r.raw || [];
  if (!list.length) return `<p class="hint" data-t="none"></p>`;
  return tbl(['pl_raw', 'inv_qty'], list.map(x => `<tr><td>${esc(nameOf(x.name, lang))}</td>${cell(amount(x))}</tr>`).join(''));
}

/// The whole list. `fmt` is `{ lang }`.
export function draw(r, { lang = 'en' } = {}){
  const sec = (word, html) => `<p class="eyebrow mt-3" data-t="${word}"></p>${html}`;
  const err = r.history && r.history.error ? `<p class="hint neg">${w('pl_historyError')}: ${esc(r.history.error)}</p>` : '';
  const head = summary(r) + err;
  if (!r.portions || r.portions.learning) {
    return head + empty('clock', { key: 'pl_learning', bodyKey: 'pl_learningHint' }) + sec('pl_bands', bands(r));
  }
  const unmodelled = (r.unmodelled || []).length ? `<p class="hint">${w('pl_unmodelled')}: ${r.unmodelled.map(esc).join(', ')}</p>` : '';
  return head + sec('pl_preps', preps(r, lang)) + sec('pl_bands', bands(r)) + sec('pl_dishes', dishes(r, lang)) + unmodelled
    + sec('pl_raw', raw(r, lang)) + `<p class="hint">${w('pl_limits')}</p>`;
}

/// THE CARD on the kitchen numbers (`kitchen-analytics.js`): today's
/// portions, the error, and the button that opens the list.
export function card(r){
  if (!r) return '';
  return `<div class="group"><p class="eyebrow" data-t="pl_card"></p>${summary(r)}
    ${btn({ icon: 'note', key: 'pl_open', data: { openprep: '1' } })}</div>`;
}
