// LANE W-PF2 ON SCREEN (2026-09-29), PURE: `money` and `t` are handed in,
// nothing is fetched -- node renders every piece (`pf2-view.test.mjs`).
//   prepsBlock      the recipes import's preview: which semi-finished
//                   products the file creates or updates (R1);
//   cookForm        the PRODUCTION ACT: how much the card makes from what
//                   went in, how much came out (weighed), the batch's date;
//   cookResult      what the act took off the shelf, the loss on cooking,
//                   and what the batch cost (R2);
//   shelfFact       a semi-finished product's shelf, once a batch was made.
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { ui, btn, pill, rowDiv } from './parts.js';

const esc = ui.esc;
const icon = name => `<i class="ti ti-${esc(name)}" aria-hidden="true"></i>`;
const pct = pm => (pm == null ? null : `${(pm / 10).toFixed(1)}%`);

/// A semi-finished product the file creates or updates: lines, yield, K.
export function prepRow(p){
  const lines = (p.lines || []).map(l => `${esc(l.name || l.item)} ${esc(String(l.qty))}`).join(', ');
  const k = p.k != null ? ` · K ${pct(p.k)}` : '';
  return rowDiv({ leading: icon('chef-hat'), title: p.name || p.id, sub: `<span class="mono">${esc(`${p.yield} ${p.unit || 'g'}${k}`)}</span><span class="muted">${lines}</span>`,
    trailing: pill(p.new ? 'ok' : 'info', { key: p.new ? 'bulkNew' : 'bulkPrepUpdate' }), data: { bkPrep: p.id } });
}

/// The import's semi-finished products, or nothing (a flat file has none).
export function prepsBlock(preps){
  if (!Array.isArray(preps) || !preps.length) return '';
  return `<p class="eyebrow mt-3"><span data-t="bulkPreps"></span> · ${preps.length}</p><p class="muted small" data-t="bulkPrepsHint"></p>
    <div class="rows" data-bk-preps="${preps.length}">${preps.map(prepRow).join('')}</div>`;
}

/// "On the shelf: 2050 g", for a product a batch was made of; else the words
/// that say every sale takes the raw items.
export function shelfFact(p, t){
  if (!p || !p.counted) return `<p class="hint" data-pf-shelf="none">${esc(t('pf_notStocked'))}</p>`;
  return `<p class="mono" data-pf-shelf="${esc(String(p.available ?? p.onHand ?? 0))}">${esc(t('pf_onShelf'))}: <b>${esc(String(p.onHand ?? 0))} ${esc(p.unit || 'g')}</b></p>
    <p class="hint">${esc(t('pf_stockFirst'))}</p>`;
}

/// The production act's form, for `p` (its yield is the default amount).
export function cookForm(p, { field, input }){
  return `<p class="eyebrow" data-t="pf_cook"></p><h2>${esc(p.name || p.id)}</h2><p class="sheet-hint" data-t="pf_cookHint"></p>
    ${field({ id: 'ck-qty', key: 'pf_cookQty', hintKey: 'pf_cookQtyHint', inputmode: 'numeric', autocomplete: 'off', value: p.yield ?? '', tour: 'pf.cookQty' })}
    ${field({ id: 'ck-out', key: 'pf_cookOut', hintKey: 'pf_cookOutHint', inputmode: 'numeric', autocomplete: 'off', value: '', tour: 'pf.cookOut' })}
    ${input({ id: 'ck-exp', type: 'date', key: 'inv_expiry' })}
    <p class="mono" id="ckLive"></p>
    <div class="btn-row">${btn({ id: 'ckSave', variant: 'primary', icon: 'check', key: 'pf_cook', tour: 'pf.cookSave' })}</div><div id="ckOut"></div>`;
}

/// What the typed numbers say before saving: the card's K against the weighed one.
export function cookLive(qty, out, gross1, yield1){
  const q = Number(qty), o = out === '' || out == null ? q : Number(out);
  if (!(q > 0) || !(o > 0) || !(yield1 > 0) || !(gross1 > 0)) return null;
  const gross = Math.round(gross1 * q / yield1);
  return { gross, loss: gross - o, pm: Math.floor(o * 1000 / gross) };
}

/// The act as the hub answered it: inputs, loss on cooking, cost.
export function cookResult(r, { money, t }){
  const rows = (r.lines || []).map(l => `<tr><td>${esc(l.name || l.item)}</td><td>${esc(String(l.qty))} ${esc(l.unit || '')}</td></tr>`).join('');
  const loss = r.lossG != null ? `${r.lossG} g (${pct(r.yieldPm)} / ${pct(r.cardPm)})` : '-';
  return `<p class="ok" data-ck-done="${esc(r.act || '')}">${esc(t('pf_cookDone'))}: ${esc(String(r.out))} · ${esc(t('pf_cookValue'))} ${r.value != null ? money(r.value) : '-'}</p>
    <p class="mono">${esc(t('pf_cookLoss'))}: <b>${esc(loss)}</b></p>
    <p class="ui-label">${esc(t('pf_cookDrawn'))}</p><div class="kt-wrap"><table class="kt"><tbody>${rows}</tbody></table></div>`;
}
