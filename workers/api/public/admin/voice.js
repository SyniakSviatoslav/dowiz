// THE OWNER, BY VOICE (2026-09-25). The mic in the console's header.
//
// The courier's shape (`courier/app.js` voice), for the owner: this file
// hears (`/lib/voice.js`) and posts the words to `POST /api/voice`; the HUB
// decides what they meant (services/engagement/voice.rs) -- accept, start,
// ready, reject or cancel an order; a dish off or back on sale; the venue
// open, busy or closed; "how many are waiting". A change is never made from
// the words: the hub's read-back is shown in a sheet, only Confirm sends the
// token back, and the confirmation's own instruction goes through the SAME
// route the console's button calls (`voice-plan.js`).
import { t, lang, api, post, store, toast } from '/admin/core.js';
import * as ui from '/lib/ui/index.js';
import { create, speak, supported, tagFor } from '/lib/voice.js';
import { planOf, needsReason, lineOf, assistPath } from '/admin/voice-plan.js';
import { principalOf } from '/admin/kitchen-logic.js';

/// The header's mic, from the design system. `aria-pressed` says it is listening.
export const micButton = () => ui.iconButton({ id: 'voiceBtn', icon: 'microphone', ariaLabel: { t: 'voice' }, pressed: false,
  attrs: { data: { tour: 'hud.voice' } } });

const say = line => { toast(line); speak(line, tagFor(lang)); };

async function run(done, reason, refresh) {
  const p = planOf(done, store.loc, reason);
  if (!p) return toast(t('voiceFailed'));
  try { await post(p.path, p.body); toast(t('saved')); navigator.vibrate?.(12); await refresh(); }
  catch (e) { toast(String(e.message || e)); }
}

function propose(r, refresh) {
  const reason = needsReason(r.verb)
    ? ui.field({ id: 'vReason', label: { t: 'reason' }, value: t('outOfStock'), attrs: { data: { tour: 'voice.reason' } } }) : '';
  const actions = ui.button({ variant: 'ghost', label: { t: 'cancel' }, attrs: { 'data-ui-close': 'no', data: { tour: 'voice.cancel' } } })
    + ui.button({ variant: needsReason(r.verb) ? 'danger' : 'primary', icon: 'check', label: { t: 'voiceConfirm' }, attrs: { 'data-ui-close': 'yes', data: { tour: 'voice.confirm' } } });
  speak(r.readback + '?', tagFor(lang));
  // The reason is read as it is typed: the sheet is gone by the time Confirm lands.
  let why = t('outOfStock');
  const s = ui.openSheet({ title: { t: 'voice' }, closeLabel: t('close'), actions,
    body: `<p class="ui-sheet-text" data-tour="voice.readback">${ui.esc(r.readback)}</p>${reason}`,
    onClose: async v => {
      if (v !== 'yes') return;
      try {
        const done = await post('/voice', { confirm: r.token, lang });
        if (!done?.understood) return toast(done?.say || t('voiceFailed'));
        await run(done, why, refresh);
      } catch (e) { toast(String(e.message || e)); }
    } });
  const input = s.el.querySelector('#vReason');
  if (input) input.oninput = () => { why = input.value; };
}

async function heard(res, refresh) {
  const r = await post('/voice', { transcript: res.transcript, confidence: res.confidence, is_final: true, lang });
  if (!r?.understood) return toast(`${r?.say || t('voiceFailed')}${r?.heard ? ' · «' + r.heard + '»' : ''}`);
  if (r.needsConfirmation) return propose(r, refresh);
  const line = lineOf(r, t);
  if (line) return say(line);
  if (r.action === 'ask') {
    // Not answered here: the assistant answers if the venue switched it on,
    // and says so plainly if not.
    toast(t('voiceAsking'));
    try { const d = await api(assistPath(principalOf(store.t).staff, store.loc), { method: 'POST', body: { question: r.question } }); say(d.answer); }
    catch (e) { toast(String(e.message || e)); }
  }
}

/// Put the mic in the header, before `before`. Nothing is drawn where the
/// browser cannot recognise speech: a button that does nothing is worse.
/// `refresh` re-reads what a confirmed action moved (orders, the menu, the venue).
export function mountVoice(header, before, refresh) {
  if (!header || !supported() || header.querySelector('#voiceBtn')) return;
  const tmp = document.createElement('div');
  tmp.innerHTML = micButton();
  const btn = tmp.firstElementChild;
  header.insertBefore(btn, before || null);
  let rec = null;
  const idle = () => { rec = null; btn.setAttribute('aria-pressed', 'false'); };
  btn.onclick = () => {
    if (rec) { rec.stop(); return; }
    rec = create({
      lang: tagFor(lang),
      onResult: res => { if (res.isFinal) heard(res, refresh).catch(e => toast(String(e.message || e))); },
      onError: err => { idle(); toast(t(err === 'microphone-denied' ? 'voiceDenied' : err === 'network' ? 'voiceOffline' : 'error')); },
      onEnd: idle,
    });
    if (!rec) return;
    btn.setAttribute('aria-pressed', 'true');
    try { rec.start(); } catch { idle(); }
  };
}
