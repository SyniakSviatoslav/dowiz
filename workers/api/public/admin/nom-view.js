// THE NOMENCLATURE, DRAWN (lane W-NOM, 2026-09-28): a row in select mode, the
// bar under a list being selected, the packs editor. PURE: `t` is handed in,
// nothing is fetched, no DOM -- node renders every piece (`nom-view.test.mjs`).
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { ui, btn, rowBtn } from './parts.js';

const esc = ui.esc;
const icon = name => `<i class="ti ti-${esc(name)}" aria-hidden="true"></i>`;

/// A row in select mode: a tick and the row's words; a tap toggles it.
/// `sub` is markup the caller escaped.
export function pickRow(id, title, sub, on){
  return rowBtn({ cls: on ? 'nom-on' : '', leading: icon(on ? 'circle-check' : 'plus'), title, sub, data: { pick: id },
    attrs: { 'aria-pressed': String(!!on) } });
}

/// The bar under a list in select mode: how many, all shown, delete, done.
export function barMarkup(sel, t){
  const n = sel.ids.size;
  return `<div class="nom-bar" role="region"><span class="mono"><b>${n}</b> ${esc(t('nom_selected'))}</span>
    ${btn({ id: 'nomAll', variant: 'ghost', icon: 'check', key: 'nom_all' })}
    ${btn({ id: 'nomDel', variant: 'danger', icon: 'trash', key: 'nom_deleteSel', disabled: !n })}
    ${btn({ id: 'nomDone', variant: 'ghost', icon: 'x', key: 'nom_selectDone' })}</div>`;
}

/// One pack row: its name and what it holds, in the supply's base unit.
export const packRow = (p = {}) => `<div class="nom-pack">${ui.inputRow({ label: { t: 'nom_packName' }, placeholder: { t: 'nom_packName' },
  attrs: { value: p.name || '', data: { pkn: '1' } } })}${ui.inputRow({ label: { t: 'nom_packQty' }, placeholder: { t: 'nom_packQty' },
  attrs: { value: p.qty ?? '', inputmode: 'decimal', data: { pkq: '1' } } })}</div>`;

/// The packs editor: every pack, and one empty row to type the next.
export function packsMarkup(packs){
  return `<p class="eyebrow mt-3" data-t="nom_packs"></p><p class="hint" data-t="nom_packsHint"></p>
    <div id="nomPacks">${(packs || []).map(packRow).join('')}${packRow()}</div>${btn({ id: 'nomPackAdd', variant: 'ghost', icon: 'plus', key: 'nom_packAdd' })}`;
}

/// The chips a delivery offers for a supply bought in packs: one tap, one pack.
export function packChips(sup){
  const packs = sup?.packs || [];
  if (!packs.length) return '';
  return `<div class="chips" role="group">${packs.map(p => ui.chip({ as: 'button', icon: 'package', label: `+ ${p.name} (${p.qty} ${sup.unit || ''})`,
    attrs: { data: { pack: String(p.qty) } } })).join('')}</div>`;
}
