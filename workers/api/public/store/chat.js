// THE CHAT WITH THE COURIER, on the tracking sheet (W-URGENT 2026-10-02).
//
// Nobody needs a phone number to say "third floor, the door on the left".
// The thread opens when a courier takes the order, closes when the order is
// over, stays readable for thirty days and then goes with the order's
// personal data. The venue's owner may read it for a dispute; the kitchen
// never sees it. The hub decides all of that (`services/orders/chat.rs`);
// this file draws the lines and sends one.
//
// NOTHING IS KEPT ON THE DEVICE: what the customer has already seen lives in
// a Map for the life of the page, so no browser-storage key is added to the
// personal-data registry for a badge.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { API } from '/store/state.js';
import { t, intlLocale } from '/store/i18n.js';
import { $, esc, toast } from '/store/ui.js';
import { ui, k } from '/store/parts.js';

/// The hub's bound (`chat::store::MAX_CHARS`), said on the field too.
export const CHAT_MAX = 500;

/// The last instant the customer has seen, per order; memory only.
const seen = new Map();

/// The placeholder the tracking sheet draws; filled by `mountChat`. Nothing at
/// all before a courier is on the order: there is nobody to talk to yet.
export const chatMarkup = order => order?.id && order.courier_id ? `<section class="credits-wrap" id="chatBox" hidden></section>` : '';

const time = ms => new Date(ms).toLocaleTimeString(intlLocale(), { hour: '2-digit', minute: '2-digit' });
const clientId = () => `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;

/// Fill the box with the order's own token. Silent on failure: the order is
/// the page, the chat is a panel on it; the next poll tries again.
export async function mountChat(order, tok){
  const box = $('#chatBox');
  if (!box || !tok) return;
  let d;
  try {
    const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/chat?since=0`, { headers: { authorization: 'Bearer ' + tok } });
    if (!r.ok) return;
    d = await r.json();
  } catch { return; }
  draw(box, order, tok, d);
}

function draw(box, order, tok, d){
  const ms = d.messages || [];
  const before = seen.get(order.id) || 0;
  const unread = ms.filter(m => m.from === 'COURIER' && m.atMs > before).length;
  const rows = ms.map(m => ui.row({ title: m.text, sub: `${m.from === 'CUSTOMER' ? t('chatYou') : t('chatCourier')} · ${time(m.atMs)}` }));
  box.hidden = false;
  box.innerHTML = `<p class="eyebrow"><span data-t="chatTitle"></span>${unread ? ` ${ui.badge({ label: String(unread), tone: 'accent' })}` : ''}</p>
    ${rows.length ? ui.list(rows, { inset: true, id: 'chatList' }) : `<p class="muted small" data-t="chatEmpty"></p>`}
    ${d.state === 'open'
      ? ui.inputRow({ id: 'chatText', label: k('chatPlaceholder'), placeholder: k('chatPlaceholder'), enterkeyhint: 'send', attrs: { maxlength: CHAT_MAX },
          action: ui.iconButton({ id: 'chatSend', icon: 'send', ariaLabel: k('chatSend') }) })
      : `<p class="muted small" data-t="${d.state === 'closed' ? 'chatClosed' : 'chatWaiting'}"></p>`}`;
  for (const el of box.querySelectorAll('[data-t]')) el.textContent = t(el.dataset.t);
  // Drawn on an open sheet is seen.
  seen.set(order.id, Math.max(before, ...ms.map(m => m.atMs || 0)));
  const send = async () => {
    const field = $('#chatText');
    const text = (field?.value || '').trim();
    if (!text) return;
    const go = $('#chatSend');
    if (go) go.disabled = true;
    try {
      const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/chat`, {
        method: 'POST', headers: { 'content-type': 'application/json', authorization: 'Bearer ' + tok },
        body: JSON.stringify({ text, clientId: clientId() }) });
      if (!r.ok) throw new Error((await r.text().catch(() => '')) || ('HTTP ' + r.status));
      await mountChat(order, tok);
    } catch (e) {
      toast(esc(String(e.message || e)));
      if (go) go.disabled = false;
    }
  };
  const go = $('#chatSend'); if (go) go.onclick = send;
  const field = $('#chatText'); if (field) field.onkeydown = ev => { if (ev.key === 'Enter') { ev.preventDefault(); send(); } };
}
