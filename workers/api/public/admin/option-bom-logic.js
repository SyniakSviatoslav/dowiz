// AN OPTION'S RECIPE, the pure half (R13, W-LOST): the section the dish sheet
// shows under its own recipe, and what a save sends. The hub's answer is
// `GET /api/owner/products/:id/option-bom` (catalog.modifier_bom.v1). PURE,
// so node tests it (`option-bom-logic.test.mjs`); `option-bom.js` owns the DOM.
//
// ASCII QUOTES ONLY in this file.

import { ui, btn, select, field } from './parts.js';

const esc = ui.esc;

/// The route, for one dish.
export const optionBomPath = id => `/owner/products/${encodeURIComponent(id)}/option-bom`;

/// One recipe line: a supply picker and a whole quantity in its unit.
export function lineRow(supplies, line = {}, opt = '', i = 0){
  const options = [{ value: '', label: '-' }, ...supplies.map(s => ({ value: s.id, label: `${s.name} (${s.unit})` }))];
  return `<div class="grid2 pair" data-obline="${esc(opt)}">
    <div>${select({ id: `ob-s-${esc(opt)}-${i}`, key: 'ob_supply', value: line.supply || '', options, data: { obsupply: '1' } })}</div>
    <div>${field({ id: `ob-q-${esc(opt)}-${i}`, key: 'ob_qty', inputmode: 'numeric', value: line.qty ?? '', data: { obqty: '1' } })}</div>
  </div>`;
}

/// The whole section from the hub's answer.
export function section(v){
  const head = `<p class="ui-label" data-t="ob_title"></p><p class="hint" data-t="ob_hint"></p>`;
  const opts = (v && v.options) || [];
  if (!opts.length) return `<div id="obBox" data-tour="dish.optionBom">${head}<p class="hint" data-t="ob_none"></p></div>`;
  const sup = (v && v.supplies) || [];
  const blocks = opts.map(o => {
    const lines = (o.bom && o.bom.length ? o.bom : [{}]).map((l, i) => lineRow(sup, l, o.id, i)).join('');
    return `<div class="group" data-ob="${esc(o.id)}"><p class="eyebrow">${esc(o.groupName || o.group || '')}</p><p><b>${esc(o.name)}</b></p>
      <div data-oblines="${esc(o.id)}">${lines}</div>
      <div class="btn-row">${btn({ icon: 'plus', key: 'ob_add', data: { obadd: o.id } })}${btn({ variant: 'primary', icon: 'check', key: 'ob_save', data: { obsave: o.id }, tour: 'dish.optionBomSave' })}</div></div>`;
  }).join('');
  return `<div id="obBox" data-tour="dish.optionBom">${head}${blocks}</div>`;
}

/// The lines typed for one option: `rows` are `{ supply, qty }` as read from
/// the pickers. A row with no supply is skipped; a quantity that is not a
/// positive whole number is refused (`{ error }`), never rounded.
export function collect(rows){
  const bom = [];
  for (const r of rows) {
    const supply = String(r.supply || '').trim();
    if (!supply) continue;
    const q = String(r.qty ?? '').trim();
    if (!/^[0-9]+$/.test(q) || Number(q) <= 0) return { error: 'ob_badQty' };
    bom.push({ supply, qty: Number(q) });
  }
  return { bom };
}

/// What a save sends.
export const bodyOf = (loc, option, bom) => ({ location_id: loc, option, bom });

/// The dish the open sheet is for: the row last tapped when it names a dish
/// with this title, else the one dish with this title.
export function dishOf(products, lastId, title){
  const named = (products || []).filter(p => p.name === title);
  if (lastId && named.some(p => p.id === lastId)) return lastId;
  return named.length === 1 ? named[0].id : null;
}
