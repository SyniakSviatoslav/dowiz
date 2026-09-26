// CUSTOMER MESSAGES, the venue's side (W-WIRE row 1). A customer writes about
// an order from the kit's chat; until this screen nobody at the venue could
// read it. Every thread is listed with what the venue has not read (the
// kernel's number, `GET /api/owner/threads`); tapping one shows the
// conversation, marks it read, and a reply goes back as the VENUE.
// Staff who work orders may read; only the owner replies (`party.rs`).
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, icon, t, api, post, busy, clock, day, S } from '/admin/core.js';
import { btn, field, rowBtn, rowDiv, pill, empty, loading } from '/admin/parts.js';
import { q, venuePath, fail, open, paint } from '/admin/wire-core.js';
import { readThrough, requestId } from '/admin/wire-logic.js';

/// How many messages the venue has not read, for a badge on the nav entry.
export async function unread(){
  try { return (await api('/owner/threads' + q())).unread || 0; } catch { return 0; }
}

const orderOf = id => (S.orders || []).find(o => o.id === id);
const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '');

/// The list, into `host`.
export async function mount(host){
  if (!host) return;
  host.innerHTML = loading(3);
  let d;
  try { d = await api('/owner/threads' + q()); } catch (e) { host.innerHTML = empty('alert-triangle', { title: String(e.message || e), alert: true }); return; }
  const rows = (d.threads || []).map(th => {
    const o = orderOf(th.id);
    const sub = th.error ? esc(th.error) : `${esc(th.last ? th.last.body : '')}<br><span class="muted small">${esc(when(th.atMs))}${o ? ' · ' + esc(t('w_order')) + ' ' + esc(o.status || '') : ''}</span>`;
    return rowBtn({ data: { thread: th.id }, leading: icon('message-2'), title: '#' + th.id.slice(0, 8), sub,
      trailing: th.unread ? pill('warn', { label: String(th.unread) }) : '' });
  });
  host.innerHTML = rows.length ? `<div class="rows" role="list">${rows.join('')}</div>` : empty('message-2', { key: 'w_noThreads', bodyKey: 'w_noThreadsHint' });
  paint(host);
  for (const b of $$('[data-thread]', host)) b.onclick = () => openThread(b.dataset.thread);
}

/// One conversation as a sheet: the messages, a reply box, read on open.
export async function openThread(id){
  const host = open('w_messages', null, 'thread');
  host.innerHTML = loading(2);
  let d;
  try { d = await api(venuePath(`/threads/${encodeURIComponent(id)}`)); } catch (e) { return fail(e); }
  const msgs = (d.messages || []).filter(m => m.kind === 'TEXT');
  host.innerHTML = `<p class="eyebrow">#${esc(id.slice(0, 8))}</p>
    <div class="rows" role="list">${msgs.map(m => rowDiv({ leading: icon(m.from === 'VENUE' ? 'building' : 'user'),
      title: { t: m.from === 'VENUE' ? 'w_fromVenue' : 'w_fromCustomer' }, sub: `${esc(m.body)}<br><span class="muted small">${esc(when(m.sentAtMs))}</span>` })).join('')
      || empty('message-2', { key: 'w_noMessages' })}</div>
    ${field({ id: 'w-reply', key: 'w_reply', rows: 3, maxlength: 1000 })}
    <div class="btn-row">${btn({ id: 'wBack', variant: 'ghost', icon: 'arrow-left', key: 'w_allThreads' })}${btn({ id: 'wSend', variant: 'primary', icon: 'send', key: 'w_send' })}</div>`;
  paint(host);
  $('#wBack').onclick = () => mount(open('w_messages', 'w_messagesHint', 'threads'));
  const through = readThrough(d.messages);
  if (through && (d.unread || {}).VENUE) {
    post(venuePath(`/threads/${encodeURIComponent(id)}/messages`), { from: 'VENUE', kind: 'READ', readThrough: through, clientId: requestId('rd', through, id) }).catch(() => {});
  }
  $('#wSend').onclick = async () => {
    const body = $('#w-reply').value.trim();
    if (!body) return;
    try {
      await busy($('#wSend'), () => post(venuePath(`/threads/${encodeURIComponent(id)}/messages`), { from: 'VENUE', kind: 'TEXT', body, clientId: requestId('tx', Date.now(), id) }));
      openThread(id);
    } catch (e) { fail(e); }
  };
}

/// The whole screen as a sheet (the More tile).
export const openMessages = () => mount(open('w_messages', 'w_messagesHint', 'threads'));
