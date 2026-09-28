// THE NOMENCLATURE ON SCREEN (lane W-NOM, 2026-09-28): adding many
// ingredients from one list, and DELETING FOR GOOD -- ingredients and dishes,
// one row at a time (a red Delete that arms on the first tap and fires on the
// second) or many at once (Select, tick, "Delete selected").
//
// "максимально просто": the list is one box, one name per line; the unit and
// group are big chips and one field under it; what will be added is shown
// before anything is sent. Deleting names what it deletes and what it does
// (recipes lose the line, the shelf forgets it, history stays, no write-off).
//
// Every call is one of: POST /api/owner/supplies/bulk, POST
// /api/owner/supplies/delete, POST /api/owner/products/delete.
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import './nom-i18n.js';
import { $, $$, esc, icon, t, post, withLoc, toast, sheet, closeSheet, busy, confirm, retranslate } from '/admin/core.js';
import { btn, field, chips, press } from '/admin/parts.js';
import { KINDS } from '/admin/ingredients-view.js';
import * as N from '/admin/nom-logic.js';
import { pickRow, barMarkup as bar, packRow, packsMarkup, packChips } from '/admin/nom-view.js';

export { pickRow, packsMarkup, packChips };
/// The bar, in the console's words.
export const barMarkup = sel => bar(sel, t);

export { N };
const fail = e => toast(String(e.message || e));

/// The styles of the bar and the pack rows, once.
export function ensureNomCss(){
  if (document.getElementById('nomCss')) return;
  const l = document.createElement('link');
  l.id = 'nomCss'; l.rel = 'stylesheet'; l.href = '/admin/nom.css';
  document.head.append(l);
}

/// A selection: on or off, and the ids ticked.
export const selection = () => ({ on: false, ids: new Set() });

/// Wire the bar: `shown()` answers the ids on screen; `del(ids)` deletes them;
/// `redraw()` paints again.
export function bindBar(root, sel, { shown, del, redraw }){
  const all = $('#nomAll', root), go = $('#nomDel', root), done = $('#nomDone', root);
  if (all) all.onclick = () => { sel.ids = N.toggleAll(sel.ids, shown()); redraw(); };
  if (done) done.onclick = () => { sel.on = false; sel.ids = new Set(); redraw(); };
  if (go) go.onclick = () => del([...sel.ids], go);
}

/// The two-tap Delete on a row: `el` is the button; the first tap arms it
/// (red, "tap again"), the second within five seconds runs `fire`.
let armed = null;
const was = new WeakMap();
export function tapDelete(el, id, fire){
  const r = N.tap(armed, id, Date.now());
  armed = r.arm;
  for (const b of $$('.nom-armed')) if (b !== el) disarm(b);
  if (r.fire) { disarm(el); return fire(); }
  if (!was.has(el)) was.set(el, [el.className, el.innerHTML]);
  el.className = el.className.replace('ui-btn--ghost', 'ui-btn--danger') + ' nom-armed';
  el.innerHTML = `${icon('trash')}<span>${esc(t('nom_deleteArmed'))}</span>`;
  setTimeout(() => { if (armed?.id === id) armed = null; disarm(el); }, N.ARM_MS);
}
function disarm(b){
  const w = was.get(b);
  if (!w) return;
  [b.className, b.innerHTML] = w;
  was.delete(b);
}

/// Delete ingredients for good. `names` label the confirmation of a list.
export async function deleteSupplies(ids, el, { names = [], ask = true } = {}){
  if (!ids.length) return null;
  if (ask) {
    const shown = names.slice(0, 6).join(', ') + (names.length > 6 ? ' …' : '');
    const ok = await confirm(`${t('nom_deleteSel')}: ${ids.length}`, shown || ids.join(', '), { danger: true, hint: t('nom_deleteHint') });
    if (!ok) return null;
  }
  try {
    const r = await busy(el, () => post('/owner/supplies/delete', withLoc({ ids })));
    toast(N.deletedLine(r, t));
    return r;
  } catch (e) { fail(e); return null; }
}

/// Delete dishes for good (many at once).
export async function deleteDishes(ids, el, names = []){
  if (!ids.length) return null;
  const shown = names.slice(0, 6).join(', ') + (names.length > 6 ? ' …' : '');
  const ok = await confirm(`${t('nom_deleteSel')}: ${ids.length}`, shown, { danger: true, hint: t('nom_deleteDishesHint') });
  if (!ok) return null;
  try {
    const r = await busy(el, () => post('/owner/products/delete', withLoc({ ids })));
    toast(N.deletedLine(r, t));
    return r;
  } catch (e) { fail(e); return null; }
}

/// ADD MANY: one box, one name per line; the unit, group and kind for all.
export function openQuickAdd(ctx){
  const cats = [...new Set((ctx.data?.supplies || []).map(s => s.category).filter(Boolean))];
  const have = new Set((ctx.data?.supplies || []).map(s => String(s.name || '').toLowerCase()));
  sheet(`<p class="eyebrow" data-t="supplies"></p><h2 data-t="nom_addMany"></h2><p class="sheet-hint" data-t="nom_addManyHint"></p>
    ${field({ id: 'qa-lines', key: 'nom_lines', rows: 8, autocomplete: 'off', placeholder: 'Salmon\nRice; kg; Dry\nSoy sauce; l' })}
    <p class="ui-label" data-t="nom_unitAll"></p>${chips({ id: 'qaUnit', values: N.UNITS.map(u => (u === 'unit' ? { value: u, key: 'nom_pcs' } : { value: u, label: u })), value: 'g', attr: 'qu' })}
    <p class="ui-label" data-t="nom_kindAll"></p>${chips({ id: 'qaKind', values: KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic })), value: KINDS[0][0], attr: 'qk' })}
    ${field({ id: 'qa-cat', key: 'nom_groupAll', autocomplete: 'off', attrs: { list: 'qaCats' } })}<datalist id="qaCats">${cats.map(c => `<option value="${esc(c)}">`).join('')}</datalist>
    <div id="qaPreview" class="nom-preview"></div>
    <div class="btn-row">${btn({ id: 'qaGo', variant: 'primary', icon: 'plus', key: 'nom_add' })}</div>`, { name: 'quickAdd' });
  let unit = 'g', kind = KINDS[0][0];
  const items = () => N.parseLines($('#qa-lines').value, { unit, category: $('#qa-cat').value.trim(), kind });
  const draw = () => {
    const list = items(), fresh = list.filter(i => !have.has(i.name.toLowerCase())), old = list.length - fresh.length;
    $('#qaPreview').innerHTML = list.length ? `<p class="mono"><b>${fresh.length}</b> ${esc(t('nom_willAdd'))}${old ? ` · ${old} ${esc(t('nom_exists'))}` : ''}</p>
      <p class="hint">${fresh.slice(0, 12).map(i => esc(`${i.name} (${i.unit}${i.category ? ', ' + i.category : ''})`)).join(' · ')}${fresh.length > 12 ? ' …' : ''}</p>` : '';
  };
  for (const b of $$('[data-qu]', $('#sheetIn'))) b.onclick = () => { unit = b.dataset.qu; press($$('[data-qu]', $('#sheetIn')), b); draw(); };
  for (const b of $$('[data-qk]', $('#sheetIn'))) b.onclick = () => { kind = b.dataset.qk; press($$('[data-qk]', $('#sheetIn')), b); draw(); };
  $('#qa-lines').oninput = draw; $('#qa-cat').oninput = draw;
  $('#qaGo').onclick = async () => {
    const list = items(), bad = N.listProblem(list);
    if (bad) return toast(t(bad));
    try {
      const r = await busy($('#qaGo'), () => post('/owner/supplies/bulk', withLoc({ items: list })));
      toast(`${t('nom_added')}: ${(r.added || []).length}${(r.existing || []).length ? ` · ${r.existing.length} ${t('nom_exists')}` : ''}`);
      closeSheet(); ctx.reload();
    } catch (e) { fail(e); }
  };
  draw();
}

/// Bind the packs editor; answers a reader of `{ packs } | { error }`.
export function bindPacks(){
  const add = $('#nomPackAdd');
  if (add) add.onclick = () => { $('#nomPacks').insertAdjacentHTML('beforeend', packRow()); retranslate($('#nomPacks')); };
  return unit => N.packsOf($$('.nom-pack', $('#sheetIn')).map(r => ({ name: $('[data-pkn]', r)?.value, qty: $('[data-pkq]', r)?.value })), unit);
}
