// The data processing agreement between the venue and dowiz (P9 of
// BLUEPRINT-GDPR-AND-MCP-2026-09-24): read it, see which version this venue
// accepted, accept the current one. The text is served at /dpa in sq/en/uk;
// the acceptance lands on the venue's record in the platform registry.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, esc, icon, t, api, post, withLoc, toast, sheet, store, busy, day } from '/admin/core.js';
import { retranslate, lang } from '/admin/i18n.js';
import { btn, check, pill, loading, rowDiv } from '/admin/parts.js';

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String(e.message || e));

function body(d){
  const tone = d.current ? ['ok', 'dpaCurrent'] : d.accepted ? ['warn', 'dpaOld'] : ['bad', 'dpaMissing'];
  const acc = d.accepted
    ? `<span class="mono">${esc(d.accepted.version)}${d.accepted.atMs ? ' · ' + esc(day(d.accepted.atMs)) : ''}</span>` : '';
  return `
    <div class="rows">${rowDiv({ leading: icon('shield-check'), title: d.version, sub: acc, trailing: pill(tone[0], { key: tone[1] }), tour: 'dpa.state' })}</div>
    <div class="btn-row">${btn({ href: '/dpa?lang=' + encodeURIComponent(lang), target: '_blank', variant: 'ghost', icon: 'external-link', key: 'dpaRead', tour: 'dpa.read' })}</div>
    ${d.current ? '' : `${check({ id: 'dpaAgree', key: 'dpaAgree', tour: 'dpa.agree' })}
    <div class="btn-row">${btn({ id: 'dpaGo', variant: 'primary', icon: 'check', key: 'dpaAccept', disabled: true, tour: 'dpa.accept' })}</div>`}`;
}

export async function open(){
  sheet(`<p class="eyebrow" data-t="settings"></p><h2 data-t="dpa"></h2><p class="muted small" data-t="dpaHint"></p>
    <div id="dpaBody">${loading()}</div>`, { name: 'dpa' });
  let d;
  try { d = await api('/owner/dpa' + q()); } catch (e) { return fail(e); }
  paint(d);
}

function paint(d){
  $('#dpaBody').innerHTML = body(d);
  retranslate($('#sheetIn'));
  const agree = $('#dpaAgree'), go = $('#dpaGo');
  if (!agree || !go) return;
  agree.onchange = () => { go.disabled = !agree.checked; };
  go.onclick = async () => {
    try {
      const r = await busy(go, () => post('/owner/dpa/accept', withLoc({ version: d.version })));
      toast(t('dpaAccepted'));
      paint(r);
    } catch (e) { fail(e); }
  };
}
