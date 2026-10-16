// AN OPTION'S RECIPE, the pure half (R13, W-LOST): the section the dish sheet
// shows under its own recipe, and what a save sends. The hub's answer is
// `GET /api/owner/products/:id/option-bom` (catalog.modifier_bom.v1). PURE,
// so node tests it (`option-bom-logic.test.mjs`); `option-bom.js` owns the DOM.
//
// ASCII QUOTES ONLY in this file.

import { ui, btn, select, field } from './parts.js';
import { parseMilli, wireQty } from './qty-adapter.js';

const esc = ui.esc;

/// The route, for one dish.
export const optionBomPath = id => `/owner/products/${encodeURIComponent(id)}/option-bom`;

/// One recipe line: a supply picker and a quantity in its unit (up to 3 decimals).
export function lineRow(supplies, line = {}, opt = '', i = 0){
  const options = [{ value: '', label: '-' }, ...supplies.map(s => ({ value: s.id, label: `${s.name} (${s.unit})` }))];
  return `<div class="grid2 pair" data-obline="${esc(opt)}">
    <div>${select({ id: `ob-s-${esc(opt)}-${i}`, key: 'ob_supply', value: line.supply || '', options, data: { obsupply: '1' } })}</div>
    <div>${field({ id: `ob-q-${esc(opt)}-${i}`, key: 'ob_qty', inputmode: 'decimal', value: line.qty ?? '', data: { obqty: '1' } })}</div>
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
/// the pickers. An empty row (no supply, no quantity) is skipped. A quantity
/// that does not read, or is 0, is refused with `ob_badQty`; a quantity typed
/// with no ingredient is refused with `ob_noSupply`. Either names the rows
/// (`rows`: their indexes) -- never dropped, never rounded (TZ 2026-10-16).
/// The kitchen's forms are read ("0,5 г", "1/2", "1,5 кг") through parseMilli
/// against the supply's unit (`supplies`). The wire is the exact text (W-QTYHUB H5)
/// or the integer when whole.
export function collect(rows, supplies = []){
  const bom = [], badQty = [], noSupply = [];
  rows.forEach((r, i) => {
    const supply = String(r.supply || '').trim(), raw = String(r.qty ?? '').trim();
    if (!supply && !raw) return;
    if (!supply) { noSupply.push(i); return; }
    const unit = (supplies || []).find(s => s.id === supply)?.unit || 'g';
    const milli = parseMilli(raw, unit);
    if (milli == null || milli < 1) { badQty.push(i); return; }
    bom.push({ supply, qty: wireQty(milli) });
  });
  if (badQty.length) return { error: 'ob_badQty', rows: badQty };
  if (noSupply.length) return { error: 'ob_noSupply', rows: noSupply };
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
