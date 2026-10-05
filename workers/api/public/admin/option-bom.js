// AN OPTION'S RECIPE IN THE DISH SHEET (R13, W-LOST): "extra salmon" takes
// 20 g of salmon off the shelf with the roll. `menu-edit.js` starts `watch()`
// with one line; `admin/menu.js` is not touched (another lane owns it). The
// section appears above the sheet's Save row whenever a dish sheet is open,
// and saves on its own button through
// `POST /api/owner/products/:id/option-bom` (catalog.modifier_bom.v1).
//
// ASCII QUOTES ONLY in this file.

import '/admin/lost-sales-i18n.js';
import { $, $$, S, store, api, post, toast, t, busy, retranslate } from '/admin/core.js';
import { optionBomPath, section, lineRow, collect, bodyOf, dishOf } from '/admin/option-bom-logic.js';

let lastId = null;
let watching = false;
const views = new Map();

/// Start once: remember the dish row tapped, and fill every dish sheet opened.
export function watch(){
  if (watching || typeof document === 'undefined') return;
  watching = true;
  document.addEventListener('click', e => { const r = e.target.closest && e.target.closest('[data-p]'); if (r) lastId = r.dataset.p; }, true);
  new MutationObserver(() => mount()).observe(document.body, { childList: true, subtree: true });
}

function mount(){
  const sheet = $('#sheet');
  if (!sheet || sheet.dataset.name !== 'dish' || !$('#dSave') || $('#obBox') || $('#obWait')) return;
  const id = dishOf(S.products, lastId, ($('#sheetIn h2') || {}).textContent || '');
  if (!id) return;
  const host = document.createElement('div');
  host.id = 'obWait';
  $('#dSave').closest('.btn-row').before(host);
  load(host, id);
}

async function load(host, id){
  let v;
  try { v = await api(optionBomPath(id)); } catch { host.remove(); return; }
  if (!host.isConnected) return;
  views.set(id, v);
  host.outerHTML = section(v);
  const box = $('#obBox');
  if (!box) return;
  retranslate(box);
  for (const b of $$('[data-obadd]', box)) b.onclick = () => {
    const list = box.querySelector(`[data-oblines="${CSS.escape(b.dataset.obadd)}"]`);
    list.insertAdjacentHTML('beforeend', lineRow(v.supplies || [], {}, b.dataset.obadd, list.children.length));
    retranslate(list);
  };
  for (const b of $$('[data-obsave]', box)) b.onclick = () => save(box, id, b);
}

async function save(box, id, b){
  const opt = b.dataset.obsave;
  const rows = $$(`[data-obline="${CSS.escape(opt)}"]`, box).map(r => ({ supply: r.querySelector('[data-obsupply]')?.value, qty: r.querySelector('[data-obqty]')?.value }));
  const got = collect(rows);
  if (got.error) { toast(t(got.error)); return; }
  try { await busy(b, () => post(optionBomPath(id), bodyOf(store.loc, opt, got.bom))); toast(t('saved')); }
  catch (e) { toast(String(e.message || e)); }
}
