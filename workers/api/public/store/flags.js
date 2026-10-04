// What a card and a dish may CLAIM, and the guest's own allergen filter (W-MR0, 2026-10-04).
//
// EVERY FLAG IS A CLAIM THE DATA SUPPORTS (docs/research/2026-10-03-dynamic-menu-and-resilience.md
// §6 row 1). Two of them live here:
//   * "MOST ORDERED THIS WEEK: N" -- a real count from the venue's last seven days, taken orders
//     only, TEST orders left out, shown only from the hub's threshold and always with its number
//     (`GET /api/public/locations/<slug>/menu/week`, workers/api/src/services/analytics/week_top.rs).
//     The owner's own `popular` tag is a different claim and now says so: "Venue's pick".
//   * THE ALLERGEN FILTER, when the venue has it on (`feature.allergen_filter`): fourteen chips
//     the GUEST taps; what they hide is decided in `store/avoid.js` (only an explicit choice
//     hides, and it hides the undeclared too). A choice saved on this device (`dw_avoid`) never
//     leaves it.

import { state, SLUG, API, on, ALLERGENS, allergenName, loadAvoid, saveAvoid, findProduct } from '/store/state.js';
import { t, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet } from '/store/ui.js';
import { declaration, hiddenCounts, hiddenBecause } from '/store/avoid.js';
import { ui } from '/store/parts.js';

// ── most ordered this week ──────────────────────────────────────────────────
/// Ask once per menu build; a failure leaves no badge (an absent claim is a true one) and says so
/// in the console, never on the card.
export async function paintWeek(root = document){
  let w;
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/menu/week`);
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    w = await r.json();
  } catch (e) { console.warn('menu/week: no badge this time:', String(e?.message || e)); return 0; }
  let n = 0;
  for (const d of Array.isArray(w?.dishes) ? w.dishes : []) {
    if (!Number.isInteger(d?.n) || d.n < (w.threshold | 0)) continue;
    for (const media of $$(`.card[data-p="${CSS.escape(String(d.id))}"] .card-media`, root)) {
      if ($('.card-flag.week', media)) continue;
      media.insertAdjacentHTML('beforeend', `<span class="card-flag week">${icon('flame')}<span data-t="weekTop"></span> ${d.n | 0}</span>`);
      n += 1;
    }
  }
  retranslate(root);
  return n;
}

// ── the guest's allergen filter ────────────────────────────────────────────
/// The guest's saved choice, but only where the venue offers the filter: a choice saved at one
/// venue must not hide dishes at a venue whose page has no chip to undo it.
export function initAvoid(){ state.avoid = on('allergen_filter') ? loadAvoid() : []; }

/// The button in the filter row, when the venue offers the filter.
export function avoidButton(){
  if (!on('allergen_filter')) return '';
  const n = (state.avoid || []).length;
  return ui.chip({ as: 'button', selected: !!n, icon: 'alert-triangle', label: n ? `${t('avoid')} ${n}` : { t: 'avoid' },
    cls: ui.cx('tag', n && 'on'), attrs: { id: 'avoidOpen', data: { tour: 'menu.avoid' } } });
}

/// The sheet: fourteen chips, what they hid, and one tap to show everything again.
export function openAvoid(onChange){
  const draw = () => {
    const avoid = state.avoid || [];
    const hid = hiddenCounts([...(state.products?.values?.() || [])], avoid);
    $('#avoidCount').innerHTML = avoid.length
      ? `${hid.contains} ${esc(t('avoidOn'))}${hid.undeclared ? `, ${hid.undeclared} ${esc(t('avoidUnknown'))}` : ''}`
      : '';
  };
  const avoid = state.avoid || [];
  sheet(`<p class="eyebrow" data-t="avoid"></p><h2 data-t="avoidHint"></h2>
    <div class="avoid-in" id="avoidBox">${ALLERGENS.map(([code]) => ui.chip({ as: 'button', selected: avoid.includes(code), label: allergenName(code),
       cls: ui.cx('chip', avoid.includes(code) && 'on'), attrs: { data: { avoid: code, tour: 'avoid.chip' } } })).join('')}</div>
    <p class="muted small mt-2" id="avoidCount" aria-live="polite"></p>
    ${ui.button({ variant: 'ghost', label: { t: 'clearAvoid' }, id: 'avoidClear', cls: 'linky', attrs: { data: { tour: 'avoid.clear' } } })}`, { name: 'avoid' });
  for (const b of $$('[data-avoid]', $('#sheetIn'))) b.onclick = () => {
    const code = b.dataset.avoid;
    state.avoid = state.avoid.includes(code) ? state.avoid.filter(c => c !== code) : [...state.avoid, code];
    saveAvoid();
    const onn = state.avoid.includes(code);
    b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn));
    onChange?.(); draw();
  };
  $('#avoidClear').onclick = () => {
    state.avoid = []; saveAvoid();
    for (const b of $$('[data-avoid]', $('#sheetIn'))) { b.classList.remove('on'); b.setAttribute('aria-pressed', 'false'); }
    onChange?.(); draw();
  };
  retranslate($('#sheetIn'));
  draw();
}

/// The dish sheet's allergen line, in its three states, when the venue offers the filter. The
/// one line that must never be silent: "not declared" is printed, never left blank.
export function allergenLine(p){
  if (!on('allergen_filter')) return '';
  const d = declaration(p);
  if (d.kind === 'undeclared') return `<p class="allergen-line warn">${icon('alert-triangle')}<span data-t="notDeclared"></span></p>`;
  if (d.kind === 'none') return `<p class="allergen-line">${icon('check')}<span data-t="noneOf14"></span></p>`;
  return `<p class="allergen-line">${icon('alert-triangle')}<span data-t="avoid"></span>: ${d.codes.map(c => esc(allergenName(c))).join(', ')}</p>`;
}

/// Does the guest's own choice hide this card (`avoid.js`)? A product the page cannot find is
/// left to the other filters.
export const avoidHides = el => { const p = findProduct(el.dataset.p); return !!p && !!hiddenBecause(p, state.avoid); };
