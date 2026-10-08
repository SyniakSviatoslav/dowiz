// THE OBJECT'S COUNTERS ON THE OWNER'S HEALTH PAGE (AX0, plan §A0). `GET /api/owner/health` ->
// `counters`. A figure with a neutral pill, never a warning: nothing acts on it.
//
// ASCII QUOTES ONLY in this file.

import { icon } from '/admin/core.js';
import { T, LANGS, retranslate } from '/admin/i18n.js';
import { pill, rowDiv } from '/admin/parts.js';
import { WORDS, countersSub } from '/admin/counters-health-view.js';

for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// The health pane's row (more.js openHealth).
export function healthRow(host, counters){
  const sub = countersSub(counters);
  if (!host || !sub) return;
  host.insertAdjacentHTML('beforeend', `<div class="rows mt-3">${rowDiv({ leading: icon('radar-2'), title: { t: 'cnt_title' }, sub, trailing: pill('ok', { label: 'i' }), tour: 'health.counters' })}</div>`);
  retranslate(host);
}
