// START STOCK (W-STOCK P1): the shelf in an hour. Two steps on one sheet:
//
//   1. the sushi starter pack -- tick what the kitchen buys, check it (a dry
//      run), then Apply: `POST /api/owner/supplies/import`, the same importer
//      and writers as a spreadsheet, with stable ids so a second run updates;
//   2. a recipe skeleton for every dish of the menu that has none, from its
//      name -- grams EMPTY until the owner types them; only a dish whose every
//      line has a quantity is sent, dry run first: `POST /api/owner/recipes/import`.
//
// Every number the pack brings is a default to check, and says so.

import { $, $$, esc, t, api, sheet, busy, retranslate, toast, lang } from '/admin/core.js';
import { ui, btn, check, pill, rowDiv, empty, loading } from '/admin/parts.js';
import { rerender } from '/admin/app.js';
import { PACK, CATS } from '/admin/start-stock-pack.js';
import * as L from '/admin/start-stock-logic.js';

const fail = e => toast(String(e.message || e));
const nameOf = id => { const p = PACK.find(x => x.id === id); return p ? (p.n[lang] || p.n.en) : id; };
const unitOf = id => PACK.find(p => p.id === id)?.unit || 'g';
const csvPost = (path, csv) => api(path, { method: 'POST', body: csv, headers: { 'content-type': 'text/csv' } });
const loadFail = e => empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true });

/// One pack line: its name, what is assumed about it, and whether the venue has it.
function packRow(p, had){
  const facts = [p.unit, p.clean && p.clean !== 1000 ? `${t('inv_cleanPct')} ${p.clean / 10}` : '', p.cook && p.cook !== 1000 ? `${t('inv_cookPct')} ${p.cook / 10}` : '',
    p.nut ? `${p.nut[0]} kcal` : '', p.pack ? L.packName(p) : ''].filter(Boolean).join(' · ');
  return `<div class="ss-row">${check({ id: `ss-${p.id}`, label: p.n[lang] || p.n.en, checked: !had, data: { ss: p.id } })}
    <p class="hint mono">${esc(facts)} ${had ? pill('ok', { key: 'ss_have' }) : pill('warn', { key: 'ss_default' })}</p></div>`;
}

/// What an import answered: how many, its warnings, what was written.
function report(r, applied, word, n){
  const warn = r.warnings || [];
  return `<p class="mono"><span data-t="${word}"></span>: ${n ?? 0} · <span data-t="warnings"></span>: ${warn.length}</p>
    ${warn.map(w => `<p class="hint">${esc(w)}</p>`).join('')}
    ${applied ? `<p class="ok"><span data-t="ss_wrote"></span>: ${r.written ?? 0}</p>` : ''}`;
}

export async function open(){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="ss_start"></h2><p class="sheet-hint" data-t="ss_hint"></p>
    <h3 data-t="ss_step1"></h3><div id="ssPack">${loading(3)}</div>
    <div class="btn-row">${btn({ id: 'ssAll', variant: 'ghost', icon: 'check', key: 'ss_all' })}${btn({ id: 'ssNone', variant: 'ghost', icon: 'x', key: 'ss_none' })}</div>
    <div class="btn-row">${btn({ id: 'ssDry', icon: 'eye', key: 'dryRun' })}${btn({ id: 'ssApply', variant: 'primary', icon: 'check', key: 'applyImport', disabled: true })}</div>
    <div id="ssOut"></div>
    <h3 class="mt-3" data-t="ss_step2"></h3><p class="hint" data-t="ss_gramsHint"></p><div id="ssDishes">${loading(2)}</div>
    <div class="btn-row">${btn({ id: 'srDry', icon: 'eye', key: 'dryRun' })}${btn({ id: 'srApply', variant: 'primary', icon: 'check', key: 'applyImport', disabled: true })}</div>
    <div id="srOut"></div>`, { name: 'startStock' });
  let stock;
  try { stock = await api('/owner/stock'); } catch (e) { $('#ssPack').innerHTML = loadFail(e); return; }
  const { had } = L.split(stock.supplies);
  $('#ssPack').innerHTML = Object.keys(CATS).map(c => {
    const rows = PACK.filter(p => p.cat === c);
    return rows.length ? `<section class="group"><p class="eyebrow">${esc(CATS[c][lang] || CATS[c].en)}</p>${rows.map(p => packRow(p, had.includes(p.id))).join('')}</section>` : '';
  }).join('');
  retranslate($('#ssPack'));
  const chosen = () => $$('[data-ss]', $('#ssPack')).filter(i => i.checked).map(i => i.dataset.ss);
  const tick = on => { for (const i of $$('[data-ss]', $('#ssPack'))) i.checked = on; $('#ssApply').disabled = true; };
  $('#ssAll').onclick = () => tick(true);
  $('#ssNone').onclick = () => tick(false);
  $('#ssPack').onchange = () => { $('#ssApply').disabled = true; };
  const supplies = async apply => {
    const ids = chosen();
    if (!ids.length) return toast(t('required'));
    try {
      const r = await busy($(apply ? '#ssApply' : '#ssDry'), () => csvPost(`/owner/supplies/import${apply ? '?apply=1' : ''}`, L.suppliesCsv(ids, lang)));
      $('#ssOut').innerHTML = report(r, apply, 'bulkSupplyN', r.supplies);
      retranslate($('#ssOut'));
      // Apply follows a dry run of the same choice, never the first tap.
      $('#ssApply').disabled = apply || !r.supplies;
      if (apply && r.written) { await dishes(); rerender(); }
    } catch (e) { fail(e); }
  };
  $('#ssDry').onclick = () => supplies(false);
  $('#ssApply').onclick = () => supplies(true);
  $('#srDry').onclick = () => recipes(false);
  $('#srApply').onclick = () => recipes(true);
  await dishes();
}

/// Step 2's state: the drafts (lines and what was typed), what the venue
/// stocks, the dishes nothing matched.
const S = { drafts: [], known: new Set(), unmatched: [] };

async function dishes(){
  const host = $('#ssDishes'); if (!host) return;
  try {
    const [list, have] = await Promise.all([api('/owner/products'), api('/owner/stock')]);
    const s = L.skeletons(Array.isArray(list) ? list : []);
    Object.assign(S, { drafts: s.drafts, unmatched: s.unmatched, known: new Set((have.supplies || []).map(x => x.id)) });
  } catch (e) { host.innerHTML = loadFail(e); return; }
  paint(host);
  host.oninput = e => {
    const at = e.target.dataset?.dq; if (!at) return;
    const [di, li] = at.split(':').map(Number), line = S.drafts[di].lines[li];
    line.typed = e.target.value;
    line.qty = L.amount(line.typed, unitOf(line.supply));
    e.target.classList.toggle('bad', line.typed.trim() !== '' && line.qty == null);
    counted();
  };
  host.onclick = e => {
    const rm = e.target.closest('[data-drm]'); if (!rm) return;
    const [di, li] = rm.dataset.drm.split(':').map(Number);
    S.drafts[di].lines.splice(li, 1);
    paint(host);
  };
}

/// The drafts as they stand, what was typed kept.
function paint(host){
  const head = S.drafts.length ? `<p class="mono"><span id="ssReady">0</span> / ${S.drafts.length} <span data-t="ss_readyN"></span> · ${S.drafts.length} <span data-t="ss_dishes"></span></p>` : empty('bowl-chopsticks', { key: 'ss_noDishes' });
  const draft = (d, di) => `<section class="group"><p class="eyebrow">${esc(d.name)}</p><div class="rows">${d.lines.map((l, li) => rowDiv({
    title: nameOf(l.supply), sub: S.known.has(l.supply) ? '' : '<span class="bad" data-t="ss_needSupplies"></span>',
    trailing: ui.inputRow({ label: nameOf(l.supply), attrs: { placeholder: unitOf(l.supply), value: l.typed ?? '', inputmode: 'decimal', autocomplete: 'off', data: { dq: `${di}:${li}` } } }) +
      btn({ variant: 'ghost', icon: 'x', key: 'ss_remove', data: { drm: `${di}:${li}` } }) })).join('')}</div></section>`;
  const missed = S.unmatched.length ? `<details class="fold"><summary>${S.unmatched.length} · <span data-t="ss_unmatched"></span></summary>${S.unmatched.map(u => `<p class="hint">${esc(u.name)}</p>`).join('')}</details>` : '';
  host.innerHTML = head + S.drafts.map(draft).join('') + missed;
  retranslate(host);
  counted();
}

function counted(){
  const el = $('#ssReady'); if (el) el.textContent = String(S.drafts.filter(L.ready).length);
  const a = $('#srApply'); if (a) a.disabled = true;
}

/// Send the READY drafts: a dry run, then Apply.
async function recipes(apply){
  const ready = S.drafts.filter(L.ready);
  if (!ready.length) return toast(t('required'));
  try {
    const r = await busy($(apply ? '#srApply' : '#srDry'), () => csvPost(`/owner/recipes/import${apply ? '?apply=1' : ''}`, L.recipesCsv(ready)));
    $('#srOut').innerHTML = report(r, apply, 'bulkRecipeN', r.recipes);
    retranslate($('#srOut'));
    $('#srApply').disabled = apply || !r.recipes;
    if (apply && r.written) { await dishes(); rerender(); }
  } catch (e) { fail(e); }
}
