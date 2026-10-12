// The invite send's pure parts (W-INVITE), tested by invite-send-view.test.mjs.
// The hub sends the link on the venue's own rail (services/courier/invite_send.rs)
// and answers a delivery state; this turns both ways into what the owner reads.
//
// ASCII QUOTES ONLY in this file.

const VIA = { whatsapp: 'WhatsApp', telegram: 'Telegram' };
const TONE = { queued: 'info', retrying: 'warn', parked: 'warn', sent: 'ok', failed: 'bad', unknown: 'warn', noRail: 'warn', noLink: 'bad', error: 'bad' };

/// The rails the owner may choose, from the list's `rails`. Empty = none set up. PURE.
export function railOptions(rails){
  const r = rails || {};
  const out = [];
  if (r.whatsapp === true) out.push({ value: 'whatsapp', key: 'invs_wa' });
  if (r.telegram === true) out.push({ value: 'telegram', key: 'invs_tg' });
  return out.length ? [{ value: '', key: 'invs_self' }, ...out] : [];
}

/// The invite body as hiring.rs takes it: `send` and `telegram` only when asked. PURE.
export function inviteBody(v){
  const b = { phone: (v.phone || '').trim(), name: (v.name || '').trim() };
  if (v.send) {
    b.send = v.send;
    b.lang = v.lang || 'sq';
    if (v.send === 'telegram') b.telegram = (v.telegram || '').trim();
  }
  return b;
}

/// `[tone, words]` for a delivery, or null when nothing was sent. `t` translates. PURE.
export function deliveryLine(d, t){
  if (!d || !d.state) return null;
  const words = t('invs_' + d.state);
  const via = VIA[d.via] || '';
  if (d.state === 'sent') return [TONE.sent, `${words} ${via}`.trim()];
  return [TONE[d.state] || 'warn', via && d.state !== 'noRail' ? `${via}: ${words}` : words];
}
