// The room app's wire: the session, reads, and writes through the outbox.
//
// THE COURIER'S SHAPE (`courier/app.js` `api` / `tapped`), for the same
// reasons: a read says which kind of failure it was (`e.offline` = the
// network never carried it, `e.status` = the server answered no), and every
// WRITE carries an `Idempotency-Key` minted AT THE TAP. A write the network
// never carried is queued in `lib/outbox.js` under THAT key -- a request
// whose answer was lost may have landed, and a fresh key would be the
// duplicate payment the header exists to prevent. A write the server
// ANSWERED is never queued: a 409 is a decision.
import { createOutbox, newKey } from '../lib/outbox.js';
import { safeGet, safeSet, safeDel } from '../store/storage.js';

export const API = '/api';
const SESSION_KEY = 'dw_room_session';
const LAST_KEY = 'dw_room_last';

/// The signed-in person: `{ jwt, staff: {id, locationId, role, caps, expiresMs} }`.
export const session = {
  get() {
    try {
      const s = JSON.parse(safeGet(SESSION_KEY) || 'null');
      return s && s.jwt && s.staff && !(s.staff.expiresMs < Date.now()) ? s : null;
    } catch { return null; }
  },
  set(s) { s ? safeSet(SESSION_KEY, JSON.stringify(s)) : safeDel(SESSION_KEY); },
};

/// The last room the server showed, with WHEN, so a reopen in a dead spot
/// draws something and says how old it is. The next person's session clears it.
export const lastRoom = {
  get(loc) { try { const v = JSON.parse(safeGet(LAST_KEY) || 'null'); return v && v.loc === loc ? v : null; } catch { return null; } },
  set(loc, sittings, at) { safeSet(LAST_KEY, JSON.stringify({ loc, sittings, at })); },
  clear() { safeDel(LAST_KEY); },
};

const auth = () => { const s = session.get(); return s ? { authorization: 'Bearer ' + s.jwt } : null; };

/// One request. Throws an Error with `offline` or `status` set.
export async function api(path, { method = 'GET', body, headers = {}, signedOut = () => {} } = {}) {
  let r;
  try {
    r = await fetch(API + path, {
      method,
      headers: { 'content-type': 'application/json', ...headers, ...(auth() || {}) },
      body: body == null ? undefined : JSON.stringify(body),
    });
  } catch (e) {
    const err = new Error(String(e?.message || e)); err.offline = true; throw err;
  }
  if (r.status === 401 && auth()) signedOut();
  if (!r.ok) {
    let m = 'HTTP ' + r.status;
    try { const txt = await r.text(); if (txt) { try { const d = JSON.parse(txt); m = d.error || d.message || m; } catch { m = txt.slice(0, 200); } } } catch {}
    const err = new Error(m); err.status = r.status; throw err;
  }
  return r.status === 204 ? null : r.json();
}

/// The queue. `hooks` is filled by the app (`onChange`, `onSent`, `onDropped`).
export const hooks = { onChange() {}, onSent() {}, onDropped() {} };
export const OUT = createOutbox({
  name: 'dowiz.room.outbox',
  // At DRAIN time: a token refreshed while the phone was in a pocket goes out.
  authorize: auth,
  onChange: rows => hooks.onChange(rows),
  onSent: (e, payload) => hooks.onSent(e, payload),
  onDropped: (e, reason, status, detail) => hooks.onDropped(e, reason, status, detail),
});

/// One write. `{landed: true, data}` when the server took it now,
/// `{landed: false, queued}` when the network did not carry it. A refusal
/// throws (the caller says what the server said).
export async function write(path, body, { tag = null, signedOut } = {}) {
  const key = newKey();
  try {
    const data = await api(path, { method: 'POST', body, headers: { 'idempotency-key': key }, signedOut });
    return { landed: true, data };
  } catch (e) {
    if (!e.offline) throw e;
    const q = await OUT.queue(API + path, { body: JSON.stringify(body), tag, key });
    return { landed: false, queued: q.ok, reason: q.reason };
  }
}
