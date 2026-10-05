// PURE. The dish's taste, texture and aroma as markup (W-SENSE row 2). Relative imports only, so
// node renders every piece for real (sense.test.mjs). Words are `data-t` keys of lib/sense-words.js,
// filled by the page's own retranslate(); figures are written as text beside every bar.
//
//   cardSense(p)        the grid card: a strip of the declared taste axes + at most two chips
//   sheetSense(p, g)    the dish sheet: labelled bars with "n/5", texture chips, aroma chips, and
//                       "You may like it: smoky, crispy" when the dish matches the guest's vector g
//   filterRow(ids, on)  the storefront's taste filter chips
//   moodRow(mood)       the session's mood chips
//   yearMarkup(months)  "your taste over the year": one row per month, its two strongest keys
// Nothing is drawn for a dish that declares nothing: no fake zeros.

import { chip } from '../lib/ui/chip.js';
import { esc } from '../lib/ui/core.js';
import { senseOf, vectorOf, cosine, because, TASTE_MAX, TAG_MAX, MOOD_IDS } from './sense.js';
import { wordKey } from '../lib/sense-words.js';

/// Icons that exist in lib/icons.css; a key with none gets a dot.
export const ICON = { 't:sweet': 'candy', 't:sour': 'lemon-2', 't:salty': 'salt', 't:bitter': 'teacup', 't:umami': 'soup', 't:spicy': 'pepper',
  'a:smoky': 'flame', 'a:citrus': 'lemon-2', 'a:herbal': 'leaf', 'a:floral': 'sakura', 'a:marine': 'fish', 'a:fruity': 'candy', 'x:crispy': 'sparkles' };
const ic = k => (ICON[k] ? `<i class="ti ti-${ICON[k]}" aria-hidden="true"></i>` : '');
/// A screen reader reads the figure; the bar is decoration.
const bar = (n, max) => `<i class="sx-bar" aria-hidden="true"><b style="--v:${(n / max).toFixed(2)}"></b></i>`;

const tagChips = (s, dim, pre, max = 99) => Object.entries(s[dim]).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, max)
  .map(([id, n]) => chip({ icon: ICON[`${pre}:${id}`] || null, dot: !ICON[`${pre}:${id}`], cls: 'sx-chip', label: { t: wordKey(`${pre}:${id}`) },
    attrs: { data: { sx: `${pre}:${id}`, n: String(n), max: String(TAG_MAX) } } })).join('');

/// The grid card's line, or ''.
export function cardSense(p){
  const s = senseOf(p); if (!s) return '';
  const axes = Object.entries(s.taste);
  const strip = axes.length ? `<span class="sx-mini" role="img" aria-label="${esc(axes.map(([id, n]) => `${id} ${n}/${TASTE_MAX}`).join(', '))}">${axes
    .map(([id, n]) => `<span class="sx-m" title="${esc(id)} ${n}/${TASTE_MAX}">${ic('t:' + id)}${bar(n, TASTE_MAX)}</span>`).join('')}</span>` : '';
  const chips = [...Object.entries(s.texture).map(([id, n]) => ['x', id, n]), ...Object.entries(s.aroma).map(([id, n]) => ['a', id, n])]
    .sort((a, b) => b[2] - a[2] || a[1].localeCompare(b[1])).slice(0, 2)
    .map(([pre, id]) => `<span class="sx-tag" data-t="${wordKey(`${pre}:${id}`)}"></span>`).join('');
  return strip || chips ? `<span class="card-sense">${strip}${chips}</span>` : '';
}

/// The dish sheet's block, or '' when nothing is declared. `guest` is the guest's sense vector.
export function sheetSense(p, guest = null){
  const s = senseOf(p); if (!s) return '';
  const axes = Object.entries(s.taste);
  const taste = axes.length ? `<h3 class="dsec" data-t="sx_taste"></h3><dl class="sx-axes">${axes.map(([id, n]) =>
    `<div class="sx-ax" data-sx="t:${id}"><dt>${ic('t:' + id)}<span data-t="${wordKey('t:' + id)}"></span></dt><dd>${bar(n, TASTE_MAX)}<span class="sx-n mono">${n}/${TASTE_MAX}</span></dd></div>`).join('')}</dl>` : '';
  const tex = Object.keys(s.texture).length ? `<h3 class="dsec" data-t="sx_texture"></h3><div class="sx-chips">${tagChips(s, 'texture', 'x')}</div>` : '';
  const aro = Object.keys(s.aroma).length ? `<h3 class="dsec" data-t="sx_aroma"></h3><div class="sx-chips">${tagChips(s, 'aroma', 'a')}</div>` : '';
  return `<section class="sx-sheet" data-tour="dish.sense">${taste}${tex}${aro}${mayLike(s, guest)}</section>`;
}

/// "You may like it: smoky, crispy" -- the dish's keys among the guest's two strongest, when the
/// dish is close to the guest's vector (cosine 0.5 or more).
export const MAY_LIKE_COS = 0.5;
function mayLike(s, guest){
  if (!guest || !Object.keys(guest).length) return '';
  const v = vectorOf(s);
  if (cosine(guest, v) < MAY_LIKE_COS) return '';
  const keys = because(guest, 3).filter(k => k in v).slice(0, 2);
  return keys.length ? `<p class="sx-like small"><span data-t="sx_mayLike"></span> ${keys.map(k => `<b data-t="${wordKey(k)}"></b>`).join(', ')}</p>` : '';
}

/// The filter chips. `ids` from sense.filterChips; `on` the active Set.
export function filterRow(ids, on = new Set()){
  if (!ids.length) return '';
  const lbl = id => (id === 'spicy' ? 'sx_spicy' : id === 'not-spicy' ? 'sx_notSpicy' : wordKey(id));
  const icn = id => (id === 'spicy' || id === 'not-spicy' ? 'pepper' : ICON[id] || null);
  return `<div class="tags sx-filters" id="sxFilters" role="group" data-tour="menu.senseFilter" data-t-attr="aria-label:sx_filters">${ids.map(id =>
    chip({ as: 'button', selected: on.has(id), icon: icn(id), dot: !icn(id), cls: `tag${on.has(id) ? ' on' : ''}`, label: { t: lbl(id) }, attrs: { data: { sxf: id } } })).join('')}</div>`;
}

/// The mood chips: one at most, tapped again to clear.
export function moodRow(mood = null){
  return `<div class="sx-mood" data-tour="menu.mood"><p class="small muted"><span data-t="sx_mood"></span></p><div class="fy-row">${MOOD_IDS.map(m =>
    chip({ as: 'button', selected: mood === m, cls: `fy${mood === m ? ' on' : ''}`, label: { t: 'sx_mood_' + m }, attrs: { data: { mood: m } } })).join('')}</div>
    <p class="small muted" data-t="sx_moodHint"></p></div>`;
}

/// "Because you often pick smoky + crispy", or ''.
export const becauseLine = keys => (keys && keys.length ? `<p class="sx-because small muted"><span data-t="sx_because"></span> ${keys.map(k => `<b data-t="${wordKey(k)}"></b>`).join(' + ')}</p>` : '');

/// "Your taste over the year": `months` = {"2026-10": {key: per mille}}, newest first.
export function yearMarkup(months){
  const rows = Object.entries(months || {}).sort((a, b) => b[0].localeCompare(a[0]));
  if (!rows.length) return '';
  return `<h3 class="dsec" data-t="sx_year"></h3><dl class="tr-list">${rows.map(([m, v]) =>
    `<dt class="mono">${esc(m)}</dt><dd>${because(v, 2).map(k => `<span data-t="${wordKey(k)}"></span>`).join(' + ') || '-'}</dd>`).join('')}</dl>`;
}
