// SEMI-FINISHED PRODUCTS, DRAWN (lane W-PF, 2026-09-29): a ПФ's row in the
// ingredients list, its card (lines with cost, yield, K, cost per kg, where
// used), the editor's line rows, and "one sale takes off the shelf" for a
// dish. PURE: `money` and `t` are handed in, nothing is fetched -- node
// renders every piece (`prep-view.test.mjs`).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { ui, btn, pill, rowBtn, rowDiv } from './parts.js';
import * as P from './prep-logic.js';
import { shelfFact } from './pf2-view.js';

const esc = ui.esc;
const icon = name => `<i class="ti ti-${esc(name)}" aria-hidden="true"></i>`;
// Every icon must be one /lib/icons.css draws: 'chef-hat' and 'scale' were not, and drew a grey square (W-VERIFY).
export const PREP_ICON = 'tools-kitchen-2';
const pct = pm => (pm == null ? null : `${Math.abs(pm) % 10 ? (pm / 10).toFixed(1) : pm / 10}%`);

/// One ПФ in the list: yield, K, cost per kg, how many dishes use it.
export function prepRowMarkup(p, { money, t, del = false }){
  const k = pct(p.k);
  const facts = [p.category, `${t('pf_yield')} ${p.yield ?? '-'} ${esc(p.unit || 'g')}`, k ? `K ${k}` : null,
    p.costPer != null ? `${money(p.costPer)} ${t(P.perWord(p.unit))}` : t('pf_costUnknown'),
    p.counted ? `${t('pf_onShelf')} ${p.onHand ?? 0} ${p.unit || 'g'}` : null].filter(Boolean).join(' · ');
  const n = (p.uses?.dishes || []).length, np = (p.uses?.preps || []).length;
  const used = n || np ? `${t('pf_usedIn')}: ${n} ${t('pf_dishes')}${np ? `, ${np} ${t('pf_preps')}` : ''}` : t('pf_usedNowhere');
  const card = rowBtn({ leading: icon(PREP_ICON), title: p.name || p.id, data: { s: p.id, prep: '1' },
    sub: `<span class="mono">${esc(facts)}</span><span class="mono">${esc(used)}</span>`,
    trailing: p.active === false ? pill('warn', { key: 'retireSupply' }) : pill('info', { key: 'kind_prep' }) });
  const gone = del ? btn({ variant: 'ghost', icon: 'x', key: 'nom_delete', data: { del: p.id } }) : '';
  return `<div class="inv-item">${card}<div class="inv-acts">${btn({ variant: 'ghost', icon: 'adjustments', key: 'edit', data: { pedit: p.id } })}${gone}</div></div>`;
}

/// The lines of a card, drawn as a table: item, gross, cost.
function linesTable(p, { money, t }){
  const rows = (p.lines || []).map(l => `<tr><td>${l.kind === P.KIND ? icon(PREP_ICON) + ' ' : ''}${esc(l.name || l.item)}${l.missing ? ` ${pill('bad', { key: 'none' })}` : ''}${l.untracked ? ` <small>(${esc(t('pf_untracked'))})</small>` : ''}</td>
    <td>${l.qty} ${esc(l.unit || '')}</td><td>${l.cost != null && !l.untracked ? money(l.cost) : '-'}</td></tr>`).join('');
  return `<div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_ingredient"></th><th data-t="pf_gross"></th><th data-t="pf_lineCost"></th></tr></thead><tbody>${rows}</tbody>
    <tfoot><tr><td data-t="pf_yield"></td><td>${p.yield ?? '-'} ${esc(p.unit || 'g')}</td><td>${p.batchCost != null ? money(p.batchCost) : '-'}</td></tr></tfoot></table></div>`;
}

/// Where a thing is used: two lists of rows, each dish with "one sale takes".
export function usesMarkup(uses, t){
  const preps = uses?.preps || [], dishes = uses?.dishes || [];
  if (!preps.length && !dishes.length) return `<p class="hint" data-t="pf_usedNowhere"></p>`;
  const rows = (list, kind) => list.map(x => rowDiv({ title: x.name || x.id, leading: icon(kind === 'prep' ? PREP_ICON : 'bowl-chopsticks'),
    trailing: kind === 'dish' ? btn({ variant: 'ghost', icon: 'receipt', key: 'pf_takes', tour: 'pf.takes', data: { takes: x.id, tname: x.name || x.id } }) : '' })).join('');
  return `${preps.length ? `<p class="ui-label">${esc(t('pf_preps'))} · ${preps.length}</p><div class="rows">${rows(preps, 'prep')}</div>` : ''}
    ${dishes.length ? `<p class="ui-label">${esc(t('pf_dishes'))} · ${dishes.length}</p><div class="rows">${rows(dishes, 'dish')}</div>` : ''}`;
}

/// The card: the numbers at the top, the lines, where used, edit / delete.
export function cardMarkup(p, { money, t, owner = true }){
  const k = pct(p.k);
  return `<p class="eyebrow" data-t="pf_title"></p><h2>${esc(p.name || p.id)}</h2>
    <div class="stats strip"><div class="stat"><b>${p.yield ?? '-'}</b><small>${esc(t('pf_yield'))}, ${esc(p.unit || 'g')}</small></div>
      <div class="stat"><b>${k || '—'}</b><small data-t="pf_k"></small></div>
      <div class="stat money"><b>${p.costPer != null ? money(p.costPer) : '—'}</b><small>${esc(t(P.perWord(p.unit)))}</small></div>
      <div class="stat money"><b>${p.batchCost != null ? money(p.batchCost) : '—'}</b><small data-t="pf_batchCost"></small></div></div>
    ${k ? '' : `<p class="hint">${esc(t('pf_k'))}: ${esc(t('pf_kUnknown'))}</p>`}${p.costPer == null ? `<p class="hint" data-t="pf_costUnknown"></p>` : ''}
    <p class="eyebrow mt-3" data-t="pf_lines"></p>${linesTable(p, { money, t })}
    <p class="eyebrow" data-t="pf_usedIn"></p>${usesMarkup(p.uses, t)}
    <p class="eyebrow mt-3" data-t="pf_onShelf"></p>${shelfFact(p, t)}
    <div class="btn-row">${btn({ id: 'pfCook', variant: 'primary', icon: 'flame', key: 'pf_cook', tour: 'pf.cook' })}${btn({ id: 'pfEdit', variant: 'secondary', icon: 'adjustments', key: 'edit' })}${btn({ id: 'pfRetire', variant: 'ghost', icon: 'box', key: 'retireSupply' })}${owner ? btn({ id: 'pfDelete', variant: 'danger', icon: 'trash', key: 'nom_delete' }) : ''}</div>`;
}

/// One editor line: the item's name, its quantity field in ITS unit, remove.
export function editorLine(l, i){
  return `<div class="pf-line" data-pl="${i}"><span class="t">${l.sup?.kind === P.KIND ? icon(PREP_ICON) + ' ' : ''}<b>${esc(l.sup?.name || l.item)}</b><small data-plc="${i}"></small></span>
    ${ui.inputRow({ label: l.sup?.name || l.item, attrs: { value: l.qty == null ? '' : String(l.qty), inputmode: 'decimal', autocomplete: 'off', data: { plq: i } } })}
    <span class="mono">${esc(l.sup?.unit || '')}</span>${btn({ variant: 'ghost', icon: 'x', ariaKey: 'remove', data: { plx: i } })}</div>`;
}

/// The live numbers under the editor.
export function liveMarkup({ k, batch, per, unit }, { money, t }){
  return `<span>${esc(t('pf_k'))}: <b>${pct(k) || esc(t('pf_kUnknown'))}</b></span> · <span>${esc(t('pf_batchCost'))}: <b>${batch != null ? money(batch) : '—'}</b></span>` +
    (per != null ? ` · <span><b>${money(per)}</b> ${esc(t(P.perWord(unit)))}</span>` : '');
}

/// What one sale takes off the shelf: a leaf per row, three decimals.
export function takesMarkup(tk, { money, t }){
  if (tk?.refused) return `<p class="hint">${esc(t('pf_refused'))}: ${esc(tk.refused)}</p>`;
  if (!tk?.lines) return `<p class="hint" data-t="pf_takesNone"></p>`;
  const rows = (tk.leaves || []).map(l => `<tr><td>${esc(l.name || l.supply)}</td><td>${esc(P.qty3(l.uq))} ${esc(l.unit || 'g')}</td><td>${l.cost != null ? money(l.cost) : '-'}</td></tr>`).join('');
  return `<p class="hint" data-t="pf_takesHint"></p><div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_ingredient"></th><th data-t="inv_qty"></th><th data-t="pf_lineCost"></th></tr></thead>
    <tbody>${rows}</tbody><tfoot><tr><td data-t="pf_takesTotal"></td><td></td><td>${tk.cost != null ? money(tk.cost) : '-'}</td></tr></tfoot></table></div>`;
}
