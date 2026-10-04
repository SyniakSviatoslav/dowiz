// The SMS box under the phone field (W-SMS): "text me about this order".
//
// UNTICKED, EVERY TIME, and SEPARATE from the offers box: placing an order is
// not consent, and order-status texts are not offers. Never remembered on the
// device -- a box ticked from memory is a pre-ticked box.
//
// SHOWN ONLY WHEN IT CAN BE KEPT: the venue has SMS on
// (`/api/public/locations/:slug/sms` -> on), the page's language has a
// sentence, and a phone is typed. The sentence is the HUB'S, with its id, so
// what the diner reads is what the filed consent proves they read.

import { state, API, SLUG } from '/store/state.js';
import { lang } from '/store/i18n.js';
import { $, esc } from '/store/ui.js';
import { consentBox } from '/store/consent.js';

let box = null;
async function load(){
  if (box) return box;
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/sms`);
    box = r.ok ? await r.json() : { on: false, wordings: [] };
  } catch { box = { on: false, wordings: [] }; }
  return box;
}

/// The sentence for this page's language, or null (no box). PURE.
export function sentence(b, l){
  if (!b || !b.on) return null;
  return (b.wordings || []).find(w => w.lang === l) || null;
}

export const smsMarkup = () => consentBox('smsBox', 'f-sms', ' data-tour="checkout.sms"');

/// Fill the sentence and follow the phone field. Called once per render.
export async function wireSms(){
  const el = $('#smsBox'), phone = $('#f-phone');
  if (!el || !phone) return;
  const w = sentence(await load(), lang);
  if (!w || !$('#smsBox')) return;
  $('#f-sms-text').innerHTML = esc(w.text.replace('{venue}', state.loc?.name || ''));
  const follow = () => {
    const has = phone.value.trim() !== '';
    el.hidden = !has;
    if (!has) $('#f-sms').checked = false;
  };
  phone.addEventListener('input', follow);
  follow();
}

/// What the order body carries: only when ticked beside a phone number.
export function smsBody(phone){
  const w = sentence(box, lang), f = $('#f-sms');
  if (!phone || !w || !f || !f.checked || $('#smsBox')?.hidden) return {};
  return { sms: { order_status: true, wording: w.id } };
}
