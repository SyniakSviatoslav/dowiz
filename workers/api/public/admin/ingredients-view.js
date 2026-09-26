// INGREDIENTS & STOCK, DRAWN: one ingredient's row, its card, the alarm
// chips and the dishes that reduce no stock -- the hub's answer in, markup
// out. PURE: `money`, `t` and `when` are handed in, nothing is fetched and no
// clock is read, so every piece renders in node (`ingredients-view.test.mjs`).
//
// ASCII QUOTES ONLY in this file.

import * as C from './ingredients-calc.js';
import { ui, btn, pill, rowBtn, rowDiv } from './parts.js';

const esc = ui.esc;
const icon = name => `<i class="ti ti-${esc(name)}" aria-hidden="true"></i>`;

/// The kinds a supply can be, with their icons.
export const KINDS = [['food_ingredient', 'meat'], ['condiment', 'bottle'], ['packaging', 'box'], ['utensil', 'tool'], ['resale', 'beer']];
/// The bar reads full at this many times the low mark.
export const FULL_AT_LOW_MULTIPLE = 4;
/// The alarms, as filter chips.
export const FLAGS = ['needsCount', 'low', 'expiring', 'noRecipe'];
/// The four actions under an ingredient and on its card.
export const ACTIONS = [['received', 'download', 'inv_delivery'], ['wasted', 'trash', 'inv_writeOff'], ['stocktake', 'check', 'inv_count'], ['prep', 'tools-kitchen-2', 'inv_prep']];

/// Does `sup` pass the alarm filter `flag`?
export function matches(sup, flag){
  if (flag === 'needsCount') return C.levelState(sup) === 'needsCount';
  if (flag === 'low') return ['low', 'out'].includes(C.levelState(sup));
  if (flag === 'expiring') return (sup.expiring || 0) > 0;
  return true;
}

/// The alarm chips: how many need a count, are low, expire, reduce no stock.
export function alertsMarkup(all, noRecipe, flag, t){
  const n = { needsCount: all.filter(s => matches(s, 'needsCount')).length, low: all.filter(s => matches(s, 'low')).length,
    expiring: all.filter(s => matches(s, 'expiring')).length, noRecipe: (noRecipe || []).length };
  const word = { needsCount: 'inv_needsCount', low: 'low', expiring: 'inv_expiring', noRecipe: 'inv_noStockLink' };
  const tone = { needsCount: 'info', low: 'warning', expiring: 'warning', noRecipe: 'neutral' };
  return `<div class="chips filters" role="group">${FLAGS.filter(f => n[f]).map(f => ui.chip({ as: 'button', selected: flag === f, tone: tone[f],
    label: `${n[f]} ${t(word[f])}`, attrs: { data: { flag: f } } })).join('')}</div>`;
}

/// One ingredient: its facts, its levels, its state and its four actions.
export function rowMarkup(sup, { money, t, warnDays }){
  const low = sup.lowAt || 0, state = C.levelState(sup);
  const pct = low ? Math.min(100, Math.round(100 * Math.max(0, sup.available || 0) / (low * FULL_AT_LOW_MULTIPLE))) : 100;
  const ic = (KINDS.find(([k]) => k === sup.kind) || KINDS[0])[1];
  const per = `/${C.basisOf(sup.unit) === 1 ? '' : '100'}${sup.unit}`;
  const price = C.priceOf(sup);
  const soon = (sup.lots || []).find(l => l.daysLeft != null);
  const facts = [sup.category, price.perBasis != null ? `${money(price.perBasis)}${per}${price.from === 'wac' ? ' ' + t('inv_wac') : ''}` : ''].filter(Boolean).join(' · ');
  const levels = `${t('inv_onHand')} ${sup.onHand ?? 0} · ${t('reserved')} ${sup.reserved ?? 0} · ${t('inv_available')} ${sup.available ?? 0} ${sup.unit || ''}`;
  const pills = [
    state === 'needsCount' ? pill('info', { key: 'inv_needsCount' }) : '',
    state === 'out' ? pill('bad', { key: 'out' }) : state === 'low' ? pill('warn', { key: 'low' }) : state === 'ok' ? pill('ok', { key: 'onSale' }) : '',
    soon ? pill(C.expiryTone(soon.daysLeft, warnDays) || 'ok', { label: `${t('inv_expiry')} ${soon.expiry}` }) : '',
  ].join('');
  const card = rowBtn({ leading: icon(ic), title: sup.name || sup.id, data: { s: sup.id }, tour: 'stock.supply',
    sub: `<span class="mono">${esc(facts)}</span><span class="mono">${esc(levels)}${low ? ` · ${esc(t('minLevel'))} ${low}` : ''}</span>
      ${sup.counted ? `<span class="gauge"><i class="${state === 'ok' ? '' : state === 'low' ? 'warn' : 'bad'}" data-w="${pct}"></i></span>` : ''}`,
    trailing: pills });
  const acts = ACTIONS.map(([a, ic2, word]) => btn({ variant: 'ghost', icon: ic2, key: word, data: { act: a, s: sup.id } })).join('');
  return `<div class="inv-item">${card}<div class="inv-acts">${acts}</div></div>`;
}

/// The dishes that take nothing off the shelf, by category, with the one tap
/// (per dish, or per category) that links them to what they are sold as.
export function noRecipeMarkup(noRecipe, t){
  const groups = C.byCategory(noRecipe);
  if (!groups.length) return '';
  return `<section class="group mt-3" id="invNoRecipe"><p class="eyebrow"><span data-t="inv_noRecipe"></span> · ${noRecipe.length}</p>
    <p class="muted small" data-t="inv_noRecipeHint"></p>
    ${groups.map(([cat, ds]) => `<div class="rows">${rowDiv({ title: cat || t('none'), cls: 'cat-h',
      trailing: btn({ icon: 'beer', key: 'inv_asIsAll', data: { asiscat: cat } }) })}
      ${ds.map(d => rowDiv({ title: d.name, trailing: btn({ variant: 'ghost', icon: 'beer', key: 'inv_asIs', data: { asis: d.id } }) })).join('')}</div>`).join('')}
  </section>`;
}

/// The card: levels, the four actions, losses (with the measured yields and
/// "use as default"), lots with their dates, price history, movements.
export function cardMarkup(sup, { money, t, when, warnDays }){
  const state = C.levelState(sup), price = C.priceOf(sup), per = `/${C.basisOf(sup.unit) === 1 ? '' : '100'}${sup.unit}`;
  const w = C.weights(sup, C.basisOf(sup.unit) === 1 ? 1 : 1000);
  const lots = (sup.lots || []).map(l => `<tr><td>${esc(l.code)}</td><td>${l.left} ${esc(sup.unit)}</td><td>${esc(l.expiry || '-')}</td>
      <td>${l.daysLeft == null ? '' : pill(C.expiryTone(l.daysLeft, warnDays), { label: `${l.daysLeft} ${t('inv_daysLeft')}` })}</td><td>${esc(l.supplier || '')}</td></tr>`).join('');
  const prices = (sup.prices || []).slice().reverse().map(p => `<tr><td>${esc(when(p.at))}</td><td>${p.perBasis != null ? money(p.perBasis) : '-'}${esc(per)}</td><td>${p.qty} ${esc(sup.unit)}</td><td>${esc(p.supplier || '')}</td><td>${esc(p.doc || '')}</td></tr>`).join('');
  const moves = (sup.moves || []).slice().reverse().map(m => `<tr><td>${esc(when(m.at))}</td><td>${esc(t('inv_mv_' + m.kind))}${m.reason ? ' · ' + esc(t(m.reason)) : ''}</td>
      <td>${m.kind === 'stocktake' ? `${m.qty} (${m.drift > 0 ? '+' : ''}${m.drift})` : m.qty}</td><td>${m.value != null ? money(m.value) : ''}</td><td>${esc(m.lot || '')}</td></tr>`).join('');
  const adopt = (stage, pm) => pm == null ? '' : `<span class="mono">${esc(t('inv_measured'))} ${C.pct(pm)}</span> ${btn({ icon: 'check', key: 'inv_adopt', data: { adopt: stage, pm } })}`;
  const table = (head, body) => body ? `<div class="kt-wrap"><table class="kt"><thead><tr>${head.map(h => `<th data-t="${h}"></th>`).join('')}</tr></thead><tbody>${body}</tbody></table></div>` : `<p class="hint" data-t="none"></p>`;
  return `<p class="eyebrow" data-t="inv_card"></p><h2>${esc(sup.name || sup.id)}</h2>
    <p>${state === 'needsCount' ? pill('info', { key: 'inv_needsCount' }) : ''}${sup.expired ? pill('bad', { key: 'inv_expired' }) : sup.expiring ? pill('warn', { key: 'inv_expiring' }) : ''}</p>
    ${state === 'needsCount' ? `<p class="hint" data-t="inv_needsCountHint"></p>` : ''}
    <div class="stats strip"><div class="stat"><b>${sup.onHand ?? 0}</b><small data-t="inv_onHand"></small></div><div class="stat"><b>${sup.reserved ?? 0}</b><small data-t="reserved"></small></div>
      <div class="stat"><b>${sup.available ?? 0}</b><small data-t="inv_available"></small></div><div class="stat money"><b>${price.perBasis != null ? money(price.perBasis) : '-'}</b><small>${esc(t(price.from === 'wac' ? 'inv_avgPrice' : 'inv_listPrice'))}${esc(per)}</small></div></div>
    <div class="btn-row">${ACTIONS
      .map(([a, ic, word]) => btn({ icon: ic, key: word, data: { cact: a } })).join('')}</div>
    <p class="eyebrow mt-3" data-t="inv_losses"></p>
    <p class="mono">${esc(t('inv_gross'))} ${w.gross ?? '-'} g → ${esc(t('inv_net'))} ${w.net ?? '-'} g → ${esc(t('inv_out'))} ${w.out ?? '-'} g · ${esc(t('inv_loss'))} ${C.pct(C.lossPm(w))}</p>
    <p class="hint">${esc(t('inv_stage_clean'))}: ${C.pct(w.cleanPm)} ${adopt('clean', sup.measuredCleanPm)}</p>
    <p class="hint">${esc(t('inv_stage_cook'))}: ${C.pct(w.cookPm)} ${adopt('cook', sup.measuredCookPm)}</p>
    <p class="eyebrow mt-3" data-t="inv_lots"></p>${table(['inv_lot', 'inv_qty', 'inv_expiry', 'inv_daysLeft', 'inv_supplier'], lots)}
    <p class="eyebrow" data-t="inv_prices"></p>${table(['ka_day', 'inv_price', 'inv_qty', 'inv_supplier', 'inv_doc'], prices)}
    <p class="eyebrow" data-t="inv_moves"></p>${table(['ka_day', 'move', 'inv_qty', 'inv_value', 'inv_lot'], moves)}
    <div class="btn-row">${btn({ id: 'cEdit', variant: 'ghost', icon: 'adjustments', key: 'edit' })}</div>`;
}
