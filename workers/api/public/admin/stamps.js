// THE STAMP CARD's three settings, on the promo codes sheet (C5). The card
// itself is a fold over the orders in the hub (`services/loyalty/stamps.rs`);
// what an owner sets here is whether it runs, how many stamps fill it, and
// what a full card takes off the next order. The hub refuses a value out of
// range (`stamps::validate`) and its words are shown as they come.
//
// ASCII QUOTES ONLY in this file: a typographic quote is a syntax error that
// takes the whole console down.

import { $, esc, icon, t, api, post, toast, busy, switchEl, retranslate } from '/admin/core.js';

import { stampValues } from '/lib/stamps.js';

const KEYS = { on: 'loyalty.stamps.enabled', n: 'loyalty.stamps.n', reward: 'loyalty.stamps.reward_minor' };

/// Draw the block into `host` and bind its save.
export async function mount(host){
  if (!host) return;
  let s = { values: {} }; try { s = await api('/owner/settings'); } catch {}
  const v = s.values || {};
  host.innerHTML = `<p class="eyebrow mt-3" data-t="stampCard"></p><p class="muted small" data-t="stampHint"></p>
    ${switchEl('st-on', v[KEYS.on] === '1', 'stampOn')}
    <div class="grid2"><div><label for="st-n" data-t="stampN"></label><input id="st-n" inputmode="numeric" value="${esc(v[KEYS.n] || '10')}"></div>
      <div><label for="st-reward" data-t="stampReward"></label><input id="st-reward" inputmode="numeric" value="${esc(v[KEYS.reward] || '')}"></div></div>
    <div class="btn-row"><button class="btn" id="stSave">${icon('check')}<span data-t="save"></span></button></div>`;
  retranslate(host);
  $('#stSave').onclick = async () => {
    const vals = stampValues($('#st-on').checked, $('#st-n').value, $('#st-reward').value);
    try {
      await busy($('#stSave'), async () => { for (const [key, value] of vals) await post('/owner/settings', { key, value }); });
      toast(t('saved'));
    } catch (e) { toast(String(e.message || e)); }
  };
}
