// The offers box under the phone field (§3.2 of
// BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22).
//
// UNTICKED, EVERY TIME. Recital 32: a pre-ticked box is not consent, so the
// tick is never remembered on the device -- a box ticked from memory is a
// pre-ticked box. Placing the order is not consent either; the box is
// separate, and nothing is sent unless it is ticked.
//
// THE SENTENCE IS THE HUB'S. `/api/public/consent/wordings` serves the exact
// bytes each id was computed over, so what the diner reads is what the id
// proves they read. A language with no sentence shows no box, and a checkout
// with no phone shows no box: there is nothing to consent to.

import { state, API } from '/store/state.js';
import { lang } from '/store/i18n.js';
import { $, esc } from '/store/ui.js';

let wordings = null;
async function load(){
  if (wordings) return wordings;
  try {
    const r = await fetch(`${API}/public/consent/wordings`);
    wordings = r.ok ? ((await r.json()).wordings || []) : [];
  } catch { wordings = []; }
  return wordings;
}
const shown = () => (wordings || []).find(w => w.lang === lang) || null;

/// The venue's privacy notice (P8), linked wherever a person hands over data:
/// beside every consent prompt, the booking form and the menu's foot.
export const privacyLink = () =>
  `<p class="privacy-link"><a href="/privacy?lang=${esc(lang)}" target="_blank" rel="noopener" data-t="privacy"></a></p>`;

export const consentMarkup = () =>
  `<label class="consent" id="offersBox" hidden><input type="checkbox" id="f-offers"><span id="f-offers-text"></span></label>${privacyLink()}`;

/// Fill the sentence and follow the phone field. Called once per render.
export async function wireConsent(){
  const box = $('#offersBox'), phone = $('#f-phone');
  if (!box || !phone) return;
  await load();
  const w = shown();
  if (!w || !$('#offersBox')) return;
  $('#f-offers-text').innerHTML = esc(w.text.replace('{venue}', state.loc?.name || ''));
  const follow = () => {
    const has = phone.value.trim() !== '';
    box.hidden = !has;
    if (!has) $('#f-offers').checked = false;
  };
  phone.addEventListener('input', follow);
  follow();
}

/// What the order body carries: the tick and the sentence's id, only when
/// ticked beside a phone number. Otherwise nothing at all.
export function consentBody(phone){
  const w = shown(), f = $('#f-offers');
  if (!phone || !w || !f || !f.checked || $('#offersBox')?.hidden) return {};
  return { consent: { marketing_whatsapp: true, wording: w.id } };
}
