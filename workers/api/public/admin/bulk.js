// Supplies and recipes in bulk (F1): a CSV, a dry run that shows what would
// change and every warning by row, then Apply. The same shape as the menu's
// import (menu.js openImport); the Worker reads the file with the hub's parser
// and writes through the console's own writers, one object turn per file.

import { $, esc, icon, t, api, sheet, busy, money, switchEl, retranslate } from '/admin/core.js';

const ROUTE = { supplies: '/owner/supplies/import', recipes: '/owner/recipes/import' };

/// `kind` is 'supplies' or 'recipes'; `done` runs after an Apply that wrote something.
export function openBulk(kind, done){
  sheet(`<p class="eyebrow" data-t="importCsv"></p><h2 data-t="${kind === 'supplies' ? 'importSupplies' : 'importRecipes'}"></h2>
    <p class="muted small" data-t="${kind === 'supplies' ? 'bulkSuppliesHint' : 'bulkRecipesHint'}"></p>
    <input type="file" id="bkFile" accept=".csv,text/csv">
    ${kind === 'supplies' ? switchEl('bkRetire', false, 'bulkRetireMissing', 'bulkRetireHint') : ''}
    <div class="btn-row"><button class="btn ghost" id="bkDry" disabled>${icon('eye')}<span data-t="dryRun"></span></button><button class="btn" id="bkApply" disabled>${icon('check')}<span data-t="applyImport"></span></button></div>
    <div id="bkOut"></div>`, { name: 'bulk' });
  let csv = null;
  $('#bkFile').onchange = async e => {
    const f = e.target.files?.[0]; csv = f ? await f.text() : null;
    $('#bkDry').disabled = !csv; $('#bkApply').disabled = true; $('#bkOut').innerHTML = '';
  };
  const run = async apply => {
    const retire = kind === 'supplies' && $('#bkRetire')?.checked;
    const q = apply ? `?apply=1${retire ? '&retire=1' : ''}` : '';
    try {
      const r = await busy($(apply ? '#bkApply' : '#bkDry'), () => api(ROUTE[kind] + q, { method: 'POST', body: csv, headers: { 'content-type': 'text/csv' } }));
      $('#bkOut').innerHTML = report(kind, r, apply);
      retranslate($('#bkOut'));
      // Apply follows a dry run of the same file, never the first click.
      $('#bkApply').disabled = apply || !((r.supplies || 0) + (r.recipes || 0));
      if (apply && r.written) done?.();
    } catch (err) { $('#bkOut').innerHTML = `<p class="hint bad">${esc(String(err.message || err))}</p>`; }
  };
  $('#bkDry').onclick = () => run(false);
  $('#bkApply').onclick = () => run(true);
}

const fold = (n, key, list) => list.length ? `<details class="fold"><summary>${n} · <span data-t="${key}"></span></summary>${list.map(w => `<p class="hint">${esc(w)}</p>`).join('')}</details>` : '';
const per = u => u === 'unit' ? '' : '/100' + u;

function supplyRow(s){
  const facts = [s.unit, s.kcalPer100 != null ? `${s.kcalPer100} kcal${per(s.unit)}` : '', s.costPerBasis != null ? `${money(s.costPerBasis)}${per(s.unit)}` : '', s.lowAt ? `min ${s.lowAt} ${s.unit}` : '', s.supplier || ''].filter(Boolean).join(' · ');
  return `<div class="rowc">${icon('bottle')}<span class="t"><b>${esc(s.name)}</b><small class="mono">${esc(facts)}</small></span>${s.new ? `<span class="pill ok" data-t="bulkNew"></span>` : ''}</div>`;
}

function recipeRow(r){
  const side = x => `${x.lines} <span data-t="bulkLines"></span>${x.kcal != null ? ` · ${esc(String(x.kcal))} kcal` : ''}${x.weightG != null ? ` · ${esc(String(x.weightG))} g` : ''}${x.cost != null ? ` · ${money(x.cost)}` : ''}`;
  const lines = (r.bom || []).map(l => `${esc(l.name || l.supply)} ${l.qty} ${esc(l.unit || '')}`).join(', ');
  return `<div class="rowc">${icon('bowl-chopsticks')}<span class="t"><b>${esc(r.dish)}</b>
    <small class="mono"><span data-t="bulkNow"></span>: ${side(r.before)}</small>
    <small class="mono"><span data-t="bulkThen"></span>: ${side(r.after)}</small>
    <small class="muted">${lines}</small>${r.error ? `<small class="bad">${esc(r.error)}</small>` : ''}</span></div>`;
}

function report(kind, r, applied){
  const rows = r.rows || [];
  const n = kind === 'supplies' ? r.supplies : r.recipes;
  return `<div class="stats mt-3"><div class="stat"><small data-t="${kind === 'supplies' ? 'bulkSupplyN' : 'bulkRecipeN'}"></small><b>${n ?? 0}</b></div>
      <div class="stat"><small data-t="warnings"></small><b>${(r.warnings || []).length}</b></div></div>
    ${fold((r.warnings || []).length, 'warnings', r.warnings || [])}
    ${fold((r.withoutRecipe || []).length, 'bulkWithout', r.withoutRecipe || [])}
    ${fold((r.notInFile || []).length, 'notInFile', r.notInFile || [])}
    ${fold((r.flattened || []).length, 'bulkFlattened', r.flattened || [])}
    ${applied ? `<p class="ok"><span data-t="bulkWritten"></span>: ${r.written ?? 0}${r.retired ? ` · ${r.retired} <span data-t="retired"></span>` : ''}</p>` : ''}
    <div class="rows mt-2">${rows.map(kind === 'supplies' ? supplyRow : recipeRow).join('')}</div>`;
}
