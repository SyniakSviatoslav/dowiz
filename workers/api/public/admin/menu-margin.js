// THE MENU'S MARGIN (operator 2026-11-23), the owner's alone: each dish row
// wears its food cost and margin per portion (today's recipe against today's
// price) or "cost unknown"; the dish sheet shows its last 30 days from every
// channel, eBills included, with the basis of each part of the cost, and
// names what an unknown cost waits on as buttons to where it is fixed; the
// till codes no dish answers to are listed under the menu, each opening the
// existing mapping sheet (`ebills.js`) -- there is no second one.
//
// ONE READ: `GET /owner/analytics/kitchen?days=30&v=2`, whose `menu` block the
// hub strips for staff (`kitchen.rs`); a staff console never asks. A read that
// fails leaves the list as it was: no badge rather than a wrong one.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, t, api, money, retranslate } from '/admin/core.js';
import { fresh } from '/admin/fresh-logic.js';
import { k, btn, pill, rowDiv } from '/admin/parts.js';
import '/admin/menu-margin-i18n.js';
import * as G from '/admin/menu-margin-logic.js';

/// The `menu` block of the kitchen answer, or null. Stale after any save
/// (a price typed on the stock screen) and after a minute anyway.
const menu = fresh.cache(async () => {
  try { return (await api('/owner/analytics/kitchen?days=30&v=2'))?.menu || null; } catch { return null; }
}, { ttlMs: 60_000 });

const badgeOf = r => {
  const b = G.badge(r);
  if (!b) return '';
  return b.key ? pill(b.tone, { key: b.key, cls: 'mg' }) : pill(b.tone, { label: `${G.pct(b.fc)} · ${money(b.margin)}`, cls: 'mg' });
};

/// The till codes no dish answers to, by revenue, each with its link.
function tillMarkup(m){
  const rows = G.unmatchedOf(m, 8);
  if (!rows.length) return '';
  return `<section class="group mt-3"><p class="eyebrow" data-t="mg_till"></p><p class="muted small" data-t="mg_tillHint"></p><div class="rows">${rows.map(u => rowDiv({
    data: { till: u.code }, title: u.name || u.code, sub: `<span class="mono">${esc(u.code)} · ${esc(u.portions)} ${esc(t('mg_portions'))} · ${esc(money(u.revenue || 0))}</span>`,
    trailing: btn({ cls: 'mg-link', variant: 'ghost', icon: 'check', key: 'mg_link' }) })).join('')}</div></section>`;
}

/// The list: a badge on every dish row, and the till codes under it.
export async function markMargins(host, staff){
  if (staff) return;
  const m = await menu();
  if (!m) return;
  const by = new Map((m.today || []).map(r => [r.id, r]));
  for (const r of $$('[data-p]', host)) {
    if (!$('.mg', r)) $('.ui-row-trail', r)?.insertAdjacentHTML('afterbegin', badgeOf(by.get(r.dataset.p)));
  }
  const box = $('#mTill', host);
  if (box) {
    box.innerHTML = tillMarkup(m);
    for (const b of $$('.mg-link', box)) b.onclick = async () => (await import('/admin/ebills.js')).open();
  }
  retranslate(host);
}

const value = ([, v, kind]) => (v == null ? `<span data-t="mg_unknownCost">${esc(t('mg_unknownCost'))}</span>` : esc(kind === 'money' ? money(v) : v));

/// The dish sheet's block: today's portion cost, what it waits on, the 30 days.
/// `fix.supply(id)` and `fix.recipe()` open where the owner fixes a cause.
export async function fillHistory(box, id, fix){
  if (!box) return;
  const m = await menu();
  // The sheet may have moved on while the answer came: never write into a gone one.
  if (!m || box.isConnected === false) return;
  const today = (m.today || []).find(r => r.id === id);
  const h = G.history((m.dishes || []).find(r => r.id === id));
  const miss = G.missingOf(today);
  const cost = today?.portionCost;
  box.innerHTML = `<p class="eyebrow mt-3" data-t="mg_title"></p><div class="rows">
    ${rowDiv({ title: k('mg_today'), trailing: cost != null ? `<b class="mono">${esc(money(cost))} · ${esc(G.pct(today.foodCostPm))}</b>` : badgeOf(today) })}
    ${(h?.facts || []).map(f => rowDiv({ title: k(f[0]), trailing: `<b class="mono">${value(f)}</b>` })).join('')}
    ${(h?.basis || []).map(([key, n, c]) => rowDiv({ title: k(key), trailing: `<span class="mono">${esc(n)} ${esc(t('mg_portions'))} · ${value([key, c, 'money'])}</span>` })).join('')}</div>
    ${h ? '' : `<p class="muted small" data-t="mg_notSold"></p>`}
    ${h?.attributed ? `<p class="muted small"><b class="mono">${esc(h.attributed)}</b> <span data-t="mg_attributed"></span></p>` : ''}
    ${miss.length ? `<p class="muted small" data-t="mg_missing"></p><div class="btn-row">${miss.map(x => x.kind === 'recipe'
      ? btn({ variant: 'ghost', icon: 'tools-kitchen-2', key: 'mg_addRecipe', data: { fixRecipe: '1' } })
      : btn({ variant: 'ghost', icon: 'coin', label: x.name || x.id, data: { fixSupply: x.id } })).join('')}</div>` : ''}`;
  for (const b of $$('[data-fix-supply]', box)) b.onclick = () => fix.supply(b.dataset.fixSupply);
  for (const b of $$('[data-fix-recipe]', box)) b.onclick = () => fix.recipe();
  retranslate(box);
}
