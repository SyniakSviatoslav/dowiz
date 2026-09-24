// The dish, in full: photograph, what is in it, what it does to the day.
//
// The three questions a price cannot answer -- what is in it, how much of it
// there is, what it does to the day -- each get a line ONLY when the venue has
// answered. Ingredients come from the venue's own list, in the customer's
// language when the venue has one; calories, protein, fat and carbohydrates
// print when declared and are absent when not, because "0 g" is a claim and
// "not declared" is the truth. A figure the venue has not measured but the
// menu carries as an estimate is printed with "≈" in front of it and a line
// under the block saying so. Allergens are the one line that must never be
// silent: three states, three treatments.
//
// The add control is a snap. The price is an integer written as text and the
// quantity is a number; neither ever tweens.

import { state, addLine, lineUnit, moneyEl, money, on } from '/store/state.js';
import { t, tagName } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, fallbackArt, toast } from '/store/ui.js';
import { seaEvent } from '/store/sea.js';
import { ui, k, ghost, stepper } from '/store/parts.js';

/// The Sea's answer to a dish being added: a small pulse from the sheet, a
/// smaller one from a card. Numbers are particle counts, not milliseconds.
const SEA_PULSE_SHEET = 24;
const SEA_PULSE_CARD = 18;
/// The quantity control's bounds. Ninety-nine is what the order route accepts.
const QTY_MIN = 1;
const QTY_MAX = 99;
/// The prefix on every figure the venue has not measured.
const APPROX = '≈';
/// A touch has weight: the phone answers an add and a choice with a tap of
/// its own, where it can (Android). Milliseconds of vibration.
const HAPTIC_ADD_MS = 12;
const HAPTIC_TAP_MS = 6;
export const haptic = ms => { try { navigator.vibrate?.(ms); } catch {} };

let onAdded = null;
export function onDishAdded(fn){ onAdded = fn; }
/// Where a control is on the page, for the sparks to rise from.
const centreOf = el => { const r = el?.getBoundingClientRect?.(); return r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : undefined; };

function groupMarkup(g){
  const single = g.max === 1;
  const need = g.min >= 1 ? `<span class="req" data-t="required"></span>` : g.max > 0 ? `<span class="req">${g.max}</span>` : '';
  return `<fieldset class="mgroup" data-g="${esc(g.id)}" data-min="${g.min|0}" data-max="${g.max|0}">
    <legend>${esc(g.name)} ${need}</legend>
    ${(g.options || []).map(o => `
      <label class="mopt ${o.available === false ? 'off' : ''}">
        <input type="${single ? 'radio' : 'checkbox'}" name="mg-${esc(g.id)}" value="${esc(o.id)}"
               data-delta="${o.priceDelta | 0}" ${o.available === false ? 'disabled' : ''}>
        <span>${esc(o.name)}</span>
        ${o.priceDelta ? `<span class="mdelta">${o.priceDelta > 0 ? '+' : '−'}${moneyEl(Math.abs(o.priceDelta))}</span>` : ''}
      </label>`).join('')}
  </fieldset>`;
}

/// The nutrition block: calories first, because that is the question, then
/// the three macros, then the served weight, then the kitchen time.
function factsMarkup(p){
  const n = p.nutrition || {};
  const approx = !!n.approx;
  const mark = approx ? APPROX + ' ' : '';
  const kcal = p.calories ?? n.kcal;
  const facts = [];
  if (Number.isFinite(kcal)) facts.push([icon('flame'), 'kcal', `${mark}${kcal}`]);
  if (Number.isFinite(n.protein)) facts.push(['', 'protein', `${mark}${n.protein} g`]);
  if (Number.isFinite(n.fat)) facts.push(['', 'fat', `${mark}${n.fat} g`]);
  if (Number.isFinite(n.carbs)) facts.push(['', 'carbs', `${mark}${n.carbs} g`]);
  if (Number.isFinite(p.weightG)) facts.push([icon('bowl'), 'weight', `${mark}${p.weightG} g`]);
  if (Number.isFinite(p.cookingMin)) facts.push([icon('clock'), 'prep', `${p.cookingMin} ${t('etaMin')}`]);
  const taste = tasteMarkup(p);
  if (!facts.length) return taste;
  return `<h3 class="dsec" data-t="nutrition"></h3>
    <div class="facts">${facts.map(([ic, key, val]) => `<div class="fact">
      <span class="fact-v">${val}</span><span class="fact-k">${ic}<span data-t="${key}"></span></span></div>`).join('')}</div>
    ${approx ? `<p class="fact-note muted" data-t="approx"></p>` : ''}${taste}`;
}

/// The kitchen's taste profile: five axes, three levels, drawn as filled dots.
const TASTE_AXES = ['spicy', 'sweet', 'salty', 'sour', 'richness'];
const TASTE_ICON = { spicy: 'pepper', sweet: 'candy', salty: 'salt', sour: 'lemon-2', richness: 'flame' };
const TASTE_LEVELS = 3;
function tasteMarkup(p){
  const tz = p.taste && typeof p.taste === 'object' ? p.taste : null;
  const axes = tz ? TASTE_AXES.filter(a => Number(tz[a]) >= 1) : [];
  if (!axes.length) return '';
  return `<h3 class="dsec" data-t="taste"></h3><div class="taste-row-s">${axes.map(a => `<span class="taste-s">${icon(TASTE_ICON[a])}<span data-t="taste_${a}"></span><i class="tdots" aria-hidden="true">${Array.from({ length: TASTE_LEVELS }, (_, i) => `<b class="${i < Number(tz[a]) ? 'on' : ''}"></b>`).join('')}</i></span>`).join('')}</div>`;
}

export function openDish(p){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  const tags = Array.isArray(p.tags) ? p.tags : [];
  const ingredients = Array.isArray(p.ingredients) ? p.ingredients.filter(Boolean) : [];
  sheet(`
    <div class="dsheet">
      <div class="dhero">${p.imageUrl
        ? `<img src="${esc(p.imageUrl)}" alt="${esc(p.name)}" decoding="async" data-fb="${esc(p.name)}">`
        : fallbackArt(p.name)}
        ${ui.iconButton({ id: 'dback', icon: 'chevron-left', ariaLabel: k('back'), cls: 'dback', attrs: { data: { tour: 'dish.back' } } })}
        <span class="dprice-pill">${moneyEl(p.price)}</span>
      </div>
      <div class="dbody">
        ${tags.length ? `<div class="dtags">${tags.map(tg => `<span class="dtag" data-t-tag="${esc(tg)}">${esc(tagName(tg))}</span>`).join('')}</div>` : ''}
        <h2 class="dname">${esc(p.name)}</h2>
        ${p.description && !ingredients.length ? `<p class="muted ddesc">${esc(p.description)}</p>` : ''}
        ${ingredients.length ? `<h3 class="dsec" data-t="ingredients"></h3>
          <ul class="ings">${ingredients.map(i => `<li>${esc(i)}</li>`).join('')}</ul>` : ''}
        ${factsMarkup(p)}
        ${groups.map(groupMarkup).join('')}
        <p id="derr" class="err" hidden></p>
        ${ghost({ id: 'dar', cls: 'mb-2', icon: 'cube-3d-sphere', label: k('onTable'), attrs: { hidden: true }, tour: 'dish.onTable' })}
        <p id="darNote" class="geo" hidden></p>
      </div>
      <div class="dfoot">
        ${stepper({ value: 1, valueId: 'dq', minus: { id: 'dm' }, plus: { id: 'dp' }, tour: 'dish.qty' })}
        <button class="btn dadd" id="dadd" data-tour="dish.add"><span data-t="add"></span><span class="money" id="dprice" data-money="${p.price | 0}">${money(p.price)}</span></button>
      </div>
    </div>`, { name: 'dish' });
  for (const [i, li] of $$('.ings li', $('#sheetIn')).entries()) li.style.setProperty('--i', String(i));
  haptic(HAPTIC_TAP_MS);
  $('#dback').onclick = closeSheet;
  bindAr(p);
  let q = QTY_MIN;

  const chosen = () => $$('.mgroup input:checked', $('#sheetIn'));
  const repriceAndCheck = () => {
    const picked = chosen();
    const delta = picked.reduce((s, el) => s + (parseInt(el.dataset.delta, 10) || 0), 0);
    const unit = Math.max(0, p.price + delta) * q;
    const pr = $('#dprice'); pr.dataset.money = String(unit); pr.textContent = money(unit);
    let problem = null;
    for (const fs of $$('.mgroup', $('#sheetIn'))) {
      const min = parseInt(fs.dataset.min, 10) || 0, max = parseInt(fs.dataset.max, 10) || 0;
      const n = fs.querySelectorAll('input:checked').length;
      const name = fs.querySelector('legend')?.firstChild?.textContent?.trim() || '';
      if (n === 0 && min >= 1) { problem = `${t('required')}: ${name}`; break; }
      if (n > 0 && n < min)   { problem = `${name}: ${min}`; break; }
      if (max > 0 && n > max) { problem = `${name}: ${max}`; break; }
    }
    const err = $('#derr'), add = $('#dadd');
    err.hidden = !problem; if (problem) err.textContent = problem;
    add.disabled = Boolean(problem);
  };
  $('#sheetIn').addEventListener('change', e => { if (e.target.closest('.mgroup')) repriceAndCheck(); });
  $('#dm').onclick = () => { q = Math.max(QTY_MIN, q - 1); $('#dq').textContent = q; repriceAndCheck(); };
  $('#dp').onclick = () => { q = Math.min(QTY_MAX, q + 1); $('#dq').textContent = q; repriceAndCheck(); };
  $('#dadd').onclick = () => {
    const mods = chosen().map(el => el.value);
    addLine(p.id, mods, q);
    haptic(HAPTIC_ADD_MS);
    seaEvent('order_created', SEA_PULSE_SHEET, centreOf($('#dadd')));
    toast(`${p.name} · ${q}`);
    onAdded?.(p, q);
    closeSheet();
  };
  repriceAndCheck();
}

/// Straight from the card, one of it, when the dish has no choices to make. A
/// dish with options opens the sheet instead, because "add" without choosing
/// would add a dish the kitchen cannot make.
export function quickAdd(p, el){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  if (groups.some(g => (g.min | 0) >= 1)) return openDish(p);
  addLine(p.id, [], 1);
  haptic(HAPTIC_ADD_MS);
  seaEvent('order_created', SEA_PULSE_CARD, centreOf(el));
  onAdded?.(p, 1, el);
}

// "How big is it, actually?" -- the AR button, on devices that can, for dishes
// the venue has measured. Imported on demand.
async function bindAr(p){
  const b = document.getElementById('dar');
  if (!b || !on('ar') || !p.imageUrl || !(p.sizeCm > 0)) return;
  let ar;
  try { ar = await import('/lib/ar.js'); } catch { return; }
  if (!await ar.supported()) return;
  b.hidden = false;
  b.onclick = async () => {
    const note = document.getElementById('darNote');
    b.disabled = true;
    try {
      await ar.show({
        imageUrl: p.imageUrl, sizeCm: p.sizeCm, overlay: document.getElementById('sheet'),
        onStatus: st => { if (!note) return; note.hidden = false; note.className = 'geo';
          note.textContent = st === 'ready' ? t('arTap') : st === 'placed' ? `${p.name} · ${p.sizeCm} cm` : t('arScan'); },
        onEnd: () => { b.disabled = false; if (note) note.hidden = true; },
      });
    } catch {
      b.disabled = false;
      if (note) { note.hidden = false; note.className = 'geo bad'; note.textContent = t('arFail'); }
    }
  };
}

export { lineUnit };
