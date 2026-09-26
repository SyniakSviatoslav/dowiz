// THE DOOR (W-WIRE row 10): a guest shows the pass the kit issued for their
// booking; the person at the door types its code (or pastes what a scanner
// read) and the hub says whether it is this venue's, for now, and for whom.
// `POST /api/public/locations/:slug/pass/verify` answers 200 with `ok:false`
// for a bad pass, so a wrong code is a clear "no", never a broken screen.
//
// PURE RENDER (`renderPass`, `said`), the wire in `bindPass` through `c.api`.
import { ui, k, act, backBar } from './parts.js';
import './pass-i18n.js';

export const verifyPath = slug => `/public/locations/${encodeURIComponent(slug)}/pass/verify`;

/// What the hub answered, as one line in the reader's words.
export function said(d, t){
  if (!d) return '';
  if (!d.ok) return `${t('passNo')}: ${d.why || ''}`;
  const hh = String(Math.floor((Number(d.slotMin) || 0) / 60) % 24).padStart(2, '0');
  const mm = String((Number(d.slotMin) || 0) % 60).padStart(2, '0');
  return `${t('passYes')} · ${t('passParty').replace('{n}', d.party ?? '?')} · ${hh}:${mm}`;
}

export function renderPass(c){
  return `${backBar()}<h2>${ui.esc(c.t('passTitle'))}</h2><p class="muted">${ui.esc(c.t('passHint'))}</p>
    ${ui.field({ id: 'passCode', label: k('passCode'), autocomplete: 'off', attrs: { data: { tour: 'pass.code' } } })}
    ${ui.button({ variant: 'primary', icon: 'ticket', label: k('passCheck'), attrs: act('verifyPass', {}, 'pass.check') })}
    <p class="lead" id="passOut" role="status"></p>`;
}

export function bindPass(c, root, back){
  root.onclick = async ev => {
    const b = ev.target.closest('[data-act]');
    if (!b) return;
    if (b.dataset.act === 'back') return back();
    if (b.dataset.act !== 'verifyPass') return;
    const code = root.querySelector('#passCode').value.trim();
    const out = root.querySelector('#passOut');
    if (!code) { out.textContent = c.t('passNeedCode'); return; }
    try {
      const d = await c.api(verifyPath(c.S.slug), { method: 'POST', body: { code } });
      out.textContent = said(d, c.t);
    } catch (e) { out.textContent = e.message || c.t('error'); }
  };
}
