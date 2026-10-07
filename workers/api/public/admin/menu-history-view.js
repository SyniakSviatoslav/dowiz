// THE MENU'S EDIT HISTORY, DRAWN (W-PITR): the answer of
// `GET /api/owner/menu/history` (contract feature-menu-history.json) in,
// markup out. PURE -- `money` and `when` are handed in, nothing is fetched --
// so it renders in node (`menu-history-view.test.mjs`). The words are `data-t`
// keys (`menu-history-words.js`); every value is the hub's.
//
// ASCII QUOTES ONLY in this file.

import { ui, btn } from './parts.js';

const esc = ui.esc;
const KINDS = new Set(['product', 'category', 'supply', 'promo', 'location']);

/// Which word says what the edit did.
export const change = e => e.removed ? 'mh_removed' : e.added ? 'mh_added' : 'mh_changed';

/// Who, as a reader can take it: the two marks the journal writes for itself
/// are words, a staff or owner id is shown as it is.
export function who(by){
  if (by === '?') return { key: 'mh_unseen' };
  if (by === 'baseline') return { key: 'mh_baseline' };
  return { text: by || '-' };
}

/// One row. `restore` is drawn only for an edit the hub says can be restored.
export function row(e, { money = n => String(n), when = ms => String(ms) } = {}){
  const kind = KINDS.has(e.kind) ? e.kind : 'product';
  const name = e.name || e.id || e.key;
  const w = who(e.by);
  const by = w.key ? `<span data-t="${w.key}"></span>` : esc(w.text);
  const price = Number.isInteger(e.price) ? ` · ${money(e.price)}` : '';
  const act = e.restorable ? btn({ variant: 'ghost', icon: 'history', key: 'mh_restore', data: { mhSeq: Number(e.seq), mhKey: e.key } }) : '';
  return `<div class="ui-row mh-row"><div class="ui-row-main"><b>${esc(String(name))}</b>${price}
    <small class="muted"><span data-t="mh_k_${kind}"></span> · <span data-t="${change(e)}"></span> · ${esc(when(e.at))} · <span data-t="mh_by"></span> ${by}</small></div>${act}</div>`;
}

/// The whole sheet body, or the empty sentence.
export function drawHistory(r, opts = {}){
  const list = (r && r.edits) || [];
  const head = `<p class="eyebrow" data-t="tabMenu"></p><h2 data-t="mh_title"></h2><p class="sheet-hint" data-t="mh_hint"></p>`;
  if (!list.length) return `${head}<p class="hint" data-t="mh_none"></p>`;
  return `${head}<div class="rows mh-list">${list.map(e => row(e, opts)).join('')}</div>`;
}
