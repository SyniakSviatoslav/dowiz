// WHAT THE ASSISTANT CAN SEE (W-WIRE, orphan `GET /api/owner/graph`): the
// same facts the assistant is shown for a question -- dishes, orders,
// customers' kinds of things and how they relate -- searched by a word. An
// owner who doubts an answer can check its sources here. Read-only.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, esc, icon, t, api, busy } from '/admin/core.js';
import { btn, field, rowDiv, empty } from '/admin/parts.js';
import { q, fail, open, paint } from '/admin/wire-core.js';

/// One found fact as a row's sub-line: its relations, in words.
export const relations = n => (n.related || []).slice(0, 6).map(r => `${r.direction === 'to' ? '→' : '←'} ${r.how} ${r.label}`).join(' · ');

export function mount(host){
  if (!host) return;
  host.innerHTML = `<div class="grid2">${field({ id: 'w-gq', key: 'w_graphQ', type: 'search', autocomplete: 'off' })}${btn({ id: 'wGo', variant: 'primary', icon: 'search', key: 'w_look' })}</div><div id="wFound"></div>`;
  paint(host);
  const go = async () => {
    const word = $('#w-gq').value.trim();
    if (!word) return;
    let d;
    try { d = await busy($('#wGo'), () => api(`/owner/graph${q()}&q=${encodeURIComponent(word)}&limit=20`)); } catch (e) { return fail(e); }
    const rows = (d.found || []).map(n => rowDiv({ leading: icon('sparkles'), title: `${n.label}`, sub: `${esc(n.kind)} · ${esc(relations(n))}` }));
    $('#wFound').innerHTML = `<p class="muted small">${esc(d.nodes)} ${esc(t('w_facts'))} · ${esc(d.relations)} ${esc(t('w_links'))}</p>`
      + (rows.length ? `<div class="rows" role="list">${rows.join('')}</div>` : empty('search-off', { key: 'w_nothingFound' }));
    paint($('#wFound'));
  };
  $('#wGo').onclick = go;
  $('#w-gq').onkeydown = e => { if (e.key === 'Enter') go(); };
}

export const openAssistSources = () => mount(open('w_assistSources', 'w_assistSourcesHint', 'graph'));
