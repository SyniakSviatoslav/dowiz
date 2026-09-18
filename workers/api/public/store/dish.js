// The dish, in full: photograph, what is in it, what it does to the day.
//
// The three questions a price cannot answer -- what is in it, how much of it
// there is, what it does to the day -- each get a line ONLY when the venue has
// answered. Ingredients come from the venue's own list; weight, calories,
// protein, fat and carbohydrates print when declared and are absent when not,
// because "0 g" is a claim and "not declared" is the truth. Allergens are the
// one line that must never be silent: three states, three treatments.
//
// The add control is a snap. The price is an integer written as text and the
// quantity is a number; neither ever tweens.

import { state, addLine, lineUnit, moneyEl, money, on, allergenName } from '/store/state.js';
import { t, tagName } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, fallbackArt, toast } from '/store/ui.js';
import { seaEvent } from '/store/sea.js';

let onAdded = null;
export function onDishAdded(fn){ onAdded = fn; }

function allergenLine(p){
  if (!Array.isArray(p.allergens))
    return `<p class="allerg unknown">${icon('help-circle')}<span data-t="notDeclared"></span></p>`;
  if (!p.allergens.length)
    return `<p class="allerg none">${icon('check')}<span data-t="noneOf14"></span></p>`;
  return `<p class="allerg">${icon('alert-circle')}<span>${p.allergens.map(c => esc(allergenName(c))).join(', ')}</span></p>`;
}

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

function factsMarkup(p){
  const n = p.nutrition || {};
  const kcal = p.calories ?? n.kcal;
  const facts = [];
  if (Number.isFinite(p.cookingMin)) facts.push([icon('clock'), 'prep', `${p.cookingMin} ${t('etaMin')}`]);
  if (Number.isFinite(p.weightG)) facts.push([icon('bowl'), 'weight', `${p.weightG} g`]);
  if (Number.isFinite(kcal)) facts.push([icon('flame'), 'kcal', `${kcal}`]);
  if (Number.isFinite(n.protein)) facts.push(['', 'protein', `${n.protein} g`]);
  if (Number.isFinite(n.fat)) facts.push(['', 'fat', `${n.fat} g`]);
  if (Number.isFinite(n.carbs)) facts.push(['', 'carbs', `${n.carbs} g`]);
  if (!facts.length) return '';
  return `<div class="facts">${facts.map(([ic, key, val]) => `<div class="fact">
    <span class="fact-v">${val}</span><span class="fact-k">${ic}<span data-t="${key}"></span></span></div>`).join('')}</div>`;
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
        <button type="button" class="dback" id="dback" data-t-attr="aria-label:back">${icon('chevron-left')}</button>
        <span class="dprice-pill">${moneyEl(p.price)}</span>
      </div>
      <div class="dbody">
        ${tags.length ? `<div class="dtags">${tags.map(tg => `<span class="dtag" data-t-tag="${esc(tg)}">${esc(tagName(tg))}</span>`).join('')}</div>` : ''}
        <h2 class="dname">${esc(p.name)}</h2>
        ${p.description && !ingredients.length ? `<p class="muted ddesc">${esc(p.description)}</p>` : ''}
        ${factsMarkup(p)}
        ${ingredients.length ? `<h3 class="dsec" data-t="ingredients"></h3>
          <ul class="ings">${ingredients.map(i => `<li>${esc(i)}</li>`).join('')}</ul>` : ''}
        ${allergenLine(p)}
        ${groups.map(groupMarkup).join('')}
        <p id="derr" class="err" hidden></p>
        <button class="btn btn-ghost mb-2" id="dar" hidden>${icon('cube-3d-sphere')}<span data-t="onTable"></span></button>
        <p id="darNote" class="geo" hidden></p>
      </div>
      <div class="dfoot">
        <span class="qty"><button type="button" id="dm" aria-label="−">−</button><span id="dq">1</span><button type="button" id="dp" aria-label="+">+</button></span>
        <button class="btn dadd" id="dadd"><span data-t="add"></span><span class="money" id="dprice" data-money="${p.price | 0}">${money(p.price)}</span></button>
      </div>
    </div>`, { name: 'dish' });
  $('#dback').onclick = closeSheet;
  bindAr(p);
  let q = 1;

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
  $('#dm').onclick = () => { q = Math.max(1, q - 1); $('#dq').textContent = q; repriceAndCheck(); };
  $('#dp').onclick = () => { q = Math.min(99, q + 1); $('#dq').textContent = q; repriceAndCheck(); };
  $('#dadd').onclick = () => {
    const mods = chosen().map(el => el.value);
    addLine(p.id, mods, q);
    seaEvent('order_created', 24);
    toast(`${p.name} · ${q}`);
    onAdded?.(p, q);
    closeSheet();
  };
  repriceAndCheck();
}

/// Straight from the card, one of it, when the dish has no choices to make. A
/// dish with options opens the sheet instead, because "add" without choosing
/// would add a dish the kitchen cannot make.
export function quickAdd(p){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  if (groups.some(g => (g.min | 0) >= 1)) return openDish(p);
  addLine(p.id, [], 1);
  seaEvent('order_created', 18);
  onAdded?.(p, 1);
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
