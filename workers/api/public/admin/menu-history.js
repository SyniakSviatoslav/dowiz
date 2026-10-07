// "Історія змін меню" (W-PITR): the newest edits of the menu and, on a dish's
// edit, "put back as before this change". Opened from the Menu screen's
// history button (`menu.js`). The markup is `menu-history-view.js` (pure),
// the words `menu-history-words.js`.
//
// ASCII QUOTES ONLY in this file.

import '/admin/menu-history-i18n.js';
import { esc, t, api, post, withLoc, toast, sheet, money, day, clock, store, retranslate, confirm } from '/admin/core.js';
import { loadVenue, rerender } from '/admin/app.js';
import { drawHistory } from '/admin/menu-history-view.js';

const LIMIT = 30;
const when = ms => `${day(ms)} ${clock(ms)}`;

export async function openHistory(){
  sheet(`<p class="muted" data-t="loading"></p>`, { name: 'menu-history' });
  let r;
  try { r = await api('/owner/menu/history?location_id=' + encodeURIComponent(store.loc) + '&limit=' + LIMIT); }
  catch (e) { return sheet(`<p class="error">${esc(String(e.message || e))}</p>`, { name: 'menu-history' }); }
  sheet(drawHistory(r, { money, when }), { name: 'menu-history' });
  retranslate(document.body);
  for (const b of document.querySelectorAll('[data-mh-seq]')) b.onclick = () => restore(Number(b.dataset.mhSeq), b.dataset.mhKey);
}

async function restore(seq, key){
  const ok = await confirm(t('mh_title'), t('mh_restoreQ'), { hint: t('mh_restoreHint') });
  if (!ok) return openHistory();
  try { await post('/owner/menu/history/restore', withLoc({ seq, key })); toast(t('mh_restored')); }
  catch (e) { toast(String(e.message || e), { error: true }); }
  await loadVenue(); rerender(); openHistory();
}
