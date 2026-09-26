// THE WAITER, BY VOICE (2026-09-25). The mic in the room's header.
//
// THE HUB DECIDES WHAT WAS MEANT (`services/engagement/voice.rs`): this file
// hears (`/lib/voice.js`), posts the words with the round being built on this
// phone, and does what the answer says. Three kinds of answer:
//   * NOW -- nothing a guest would notice changes: a table opened on this
//     phone, a dish put in the round being built, the room at a glance.
//   * A PROPOSAL -- a write. Its read-back is shown in a sheet; only the
//     Confirm button sends the token back, and the confirmation's OWN
//     instruction (not what this phone remembers) is run through the SAME
//     route and outbox a tap uses, so voice reaches exactly the checks a tap
//     reaches and a round that moved meanwhile is refused by `base_seq`.
//   * A REFUSAL -- said, with what was heard, so a person learns the words.
//
// What a waiter's role cannot do is refused at the hub before it is proposed
// and refused again by the route; nothing here decides a right.
//
// PURE above the line (`draftOf`, `applyNow`, `planOf`), tested in node.
import { placeBody } from './open.js';
import { money } from './logic.js';
import { ui } from './parts.js';
import { create, speak, supported, tagFor } from '../lib/voice.js';

/// The round being built on this phone: the Open view's table and basket.
export function draftOf(S) {
  if (S.view !== 'open') return null;
  const items = Object.entries(S.basket || {}).filter(([, q]) => q > 0).map(([product_id, quantity]) => ({ product_id, quantity }));
  return { table: String(S.openTable || ''), items };
}

/// An answer that runs at once. Returns the line to show, or null.
export function applyNow(S, r, t) {
  if (r.action === 'open') {
    Object.assign(S, { view: 'open', openTable: r.table, basket: {} });
    return t('voiceOpened').replace('{t}', r.table) + (r.guests ? ' · ' + t('voiceGuests').replace('{n}', r.guests) : '');
  }
  if (r.action === 'draft_add') {
    if (S.view !== 'open' || String(S.openTable || '') !== r.table) Object.assign(S, { view: 'open', openTable: r.table, basket: {} });
    S.basket = S.basket || {};
    S.basket[r.productId] = (S.basket[r.productId] || 0) + Number(r.quantity || 1);
    return `${r.quantity} × ${r.name}`;
  }
  if (r.action === 'status') return t('voiceStatus').replace('{open}', r.open).replace('{waiting}', r.waiting);
  return null;
}

/// A confirmed instruction as the request a tap would make.
/// `{ via: 'write', path, body, tag }` goes through the outbox;
/// `{ via: 'place', path, body }` is the Open view's placement.
export function planOf(done, S) {
  const a = done && done.args;
  if (!a) return null;
  if (done.verb === 'add') {
    return { via: 'write', tag: 'amend:' + a.orderId, path: `/staff/orders/${encodeURIComponent(a.orderId)}/amend`,
      body: { location_id: S.loc, base_seq: a.baseSeq, ops: [{ op: 'add', product_id: a.productId, modifier_ids: [], quantity: a.quantity }] } };
  }
  if (done.verb === 'pay') {
    return { via: 'write', tag: 'pay:' + a.orderId, path: `/staff/orders/${encodeURIComponent(a.orderId)}/pay`,
      body: { location_id: S.loc, amount: a.amount, method: a.method, base_seq: a.baseSeq } };
  }
  if (done.verb === 'place') {
    return { via: 'place', path: `/public/locations/${encodeURIComponent(S.slug)}/orders`,
      body: placeBody(a.table, a.items.map(i => ({ op: 'add', product_id: i.product_id, quantity: i.quantity, modifier_ids: [] }))) };
  }
  return null;
}

/// The read-back as shown: the hub's words, and the amount for a payment.
export function readbackOf(r, S, locale) {
  return r.verb === 'pay' && Number.isInteger(r.amount) ? `${r.readback} · ${money(r.amount, S.currency, locale)}` : r.readback;
}

// ─────────────────────────────────────────────── the DOM half ──────────────

async function run(c, done) {
  const { S, t } = c;
  const p = planOf(done, S);
  if (!p) return c.toast(t('error'));
  try {
    if (p.via === 'place') {
      const o = await c.api(p.path, { method: 'POST', body: p.body });
      Object.assign(S, { openTable: '', basket: {} });
      await c.reload();
      if (o?.sitting_id) Object.assign(S, { sittingId: o.sitting_id, roundId: o.id, view: 'round' });
      c.toast(t('saved'));
    } else {
      const r = await c.write(p.path, p.body, p.tag);
      c.toast(t(r.landed ? 'saved' : r.queued ? 'queuedSaved' : 'queueNoStore'));
      c.reload();
    }
  } catch (e) { c.toast(e.message || t('error')); }
  c.render();
}

function propose(c, r) {
  const { S, t } = c;
  const actions = ui.button({ variant: 'ghost', label: { t: 'cancel' }, attrs: { 'data-ui-close': 'no', data: { tour: 'voice.cancel' } } })
    + ui.button({ variant: 'primary', icon: 'check', label: { t: 'voiceConfirm' }, attrs: { 'data-ui-close': 'yes', data: { tour: 'voice.confirm' } } });
  const line = readbackOf(r, S, c.locale());
  speak(line + '?', tagFor(c.lang()));
  ui.openSheet({ title: { t: 'voice' }, body: `<p class="ui-sheet-text" data-tour="voice.readback">${ui.esc(line)}</p>`, actions, closeLabel: t('cancel'),
    onClose: async v => {
      if (v !== 'yes') return;
      try {
        const done = await c.api('/voice', { method: 'POST', body: { confirm: r.token, lang: c.lang() } });
        if (!done?.understood) return c.toast(done?.say || t('error'));
        await run(c, done);
      } catch (e) { c.toast(e.message || t('error')); }
    } });
}

async function heard(c, res) {
  const { S, t } = c;
  const r = await c.api('/voice', { method: 'POST', body: { transcript: res.transcript, confidence: res.confidence, is_final: true, lang: c.lang(), draft: draftOf(S) } });
  if (!r?.understood) { c.toast(`${r?.say || t('error')}${r?.heard ? ' · «' + r.heard + '»' : ''}`); return; }
  if (r.needsConfirmation) return propose(c, r);
  const line = applyNow(S, r, t);
  if (line) { c.toast(line); speak(line, tagFor(c.lang())); }
  c.render();
}

/// The header's mic, from the design system. `aria-pressed` says it is listening.
export const micButton = () => ui.iconButton({ id: 'voiceBtn', icon: 'microphone', variant: 'plain', ariaLabel: { t: 'voice' }, pressed: false,
  attrs: { data: { tour: 'hud.voice' } } });

/// Put the mic in the header, before `before`. Nothing is drawn where the
/// browser cannot recognise speech: a button that does nothing is worse.
export function mountVoice(c, header, before) {
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
      lang: tagFor(c.lang()),
      onResult: res => { if (res.isFinal) heard(c, res).catch(e => c.toast(e.message || c.t('error'))); },
      onError: err => { idle(); c.toast(c.t(err === 'microphone-denied' ? 'voiceDenied' : err === 'network' ? 'voiceOffline' : 'error')); },
      onEnd: idle,
    });
    if (!rec) return;
    btn.setAttribute('aria-pressed', 'true');
    try { rec.start(); } catch { idle(); }
  };
}
