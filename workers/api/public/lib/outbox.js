// The tap that survives the basement.
//
// WHAT IT IS FOR. Reads already survive a dead signal on one surface: the
// console draws `replica.js` before it asks anything. WRITES did not survive
// anywhere. `public/kit/sw.js` says "`/api/` IS NEVER TOUCHED" and
// `public/sw.js` says "NETWORK FIRST, ALWAYS", both correctly -- a cached
// price is worse than no price -- and the courier app had an offline panel
// with a Retry button and nothing that queued a tap. A courier in a lift, a
// basement or a tram taps "picked up", `fetch` rejects, a toast says so, and
// the tap is gone. The kitchen is still waiting for a courier who left.
//
// WHY THIS NEEDS NO CRDT, which is worth stating because the obvious reflex is
// to reach for one. A courier's actions on ONE order are a SEQUENCE, not a set
// of concurrent edits: accept, then picked up, then delivered. The server's FSM
// refuses an illegal edge as an error rather than a silent no-op, and an
// `Idempotency-Key` makes the replay of the same tap the same answer. A queue
// that drains IN ORDER, replays idempotently, and believes the refusal is a
// deterministic merge already -- built out of what exists rather than out of a
// new vocabulary that has to be kept honest.
//
// THE KEY IS MINTED AT TAP TIME, NOT AT DRAIN TIME, and that is the whole
// point. A key generated as the request goes out is a new key on every attempt,
// which is exactly the duplicate the header exists to prevent; a key minted
// when the courier's thumb came off the glass is the identity of THAT TAP, and
// it stays the same however many times the phone finds and loses a signal.
//
// WHAT IT NEVER DOES. It never caches a read, and it holds nothing but the
// requests this browser has not managed to send. It never retries a refusal:
// a queued "delivered" replayed after the owner cancelled comes back 409 from
// the FSM, and a client that retried that would hammer production until the
// phone died. 409 is an ANSWER -- it is dropped and reported, and the courier
// is told the order changed while they were away.
//
// STORAGE CAN FAIL AND THAT IS NORMAL -- a private window, blocked site data,
// a full quota. Unlike `replica.js`, which degrades to "no replica" and carries
// on, a queue that cannot store REFUSES: `queue()` answers `{ ok: false }` so
// the caller can tell the courier the tap was not saved. Silently accepting a
// tap into nothing is the defect this file exists to close, not a fallback.

const DB_NAME = 'dowiz.outbox';
const DB_VERSION = 1;
const STORE = 'queue';

/// HOW MANY TAPS MAY WAIT. This is a HYPOTHESIS about a shift, not a
/// measurement, and it is written down as one: this surface shows a courier
/// ONE job at a time and a job takes at most three taps (accept, picked up,
/// delivered), so 64 is about twenty jobs' worth of offline work -- longer
/// than any tunnel, shorter than a queue that takes minutes to drain over a
/// re-found 3G connection. What IS verified is the property that matters: the
/// queue is finite and says so at the boundary (see the test).
export const MAX_QUEUE = 64;

/// WHICH END GIVES WAY WHEN IT IS FULL, and why not the orphan's end. The
/// orphan (`offline/OfflineQueue.ts`, typed against a deleted app and imported
/// by nothing) evicted the OLDEST entry to make room. Here that would delete a
/// "picked up" the courier was told was saved, leaving the "delivered" behind
/// it to be refused by the FSM as an illegal edge -- two taps lost, one of them
/// silently. This queue refuses the NEW tap instead, at the moment of the tap,
/// while the courier is still looking at the screen and can be told.

/// Statuses that mean "ask again later". 409 is deliberately NOT here: see
/// `retryAfterOf` for the one 409 that is, and why it is distinguishable.
const RETRY_STATUS = new Set([408, 425, 429, 500, 502, 503, 504]);
/// How many times a transient answer is honoured before the entry is dropped
/// and reported. A server that has answered 5xx six times over the backoff
/// below has been failing for four minutes; a courier needs to know.
const MAX_TRIES = 6;
/// Backoff between drains, indexed by the head entry's try count. Never faster
/// than two seconds: a phone that keeps flapping between one bar and none must
/// not spend its battery on the retry.
const BACKOFF_MS = [2_000, 5_000, 15_000, 30_000, 60_000, 120_000];

/// The identity of one tap. `randomUUID` is present on every browser this
/// product supports over HTTPS; the fallback exists because it is absent on
/// plain HTTP, which is how a developer's phone reaches a laptop on the LAN,
/// and a key that throws there would take the whole tap with it.
export function newKey(){
  try { return crypto.randomUUID(); }
  catch { return `k-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`; }
}

/// The one 409 that is not a refusal. `idempotency.rs` rule 4: a retry that
/// arrives while the FIRST copy of this same key is still running is answered
/// 409 WITH a `Retry-After`, precisely so it is not mistaken for rule 3's
/// "same key, different body". The FSM's illegal-transition 409 carries no
/// such header. Without this test, a client doing the right thing -- one tap,
/// one key, a retry after a timeout -- would drop its own tap as "the order
/// changed" the one time the header did its job.
function retryAfterOf(res){
  let raw = null;
  try { raw = res.headers?.get('retry-after'); } catch { return null; }
  if (raw == null || raw === '') return null;
  const s = Number(raw);
  return Number.isFinite(s) && s >= 0 ? Math.min(s, 120) * 1000 : 1000;
}

function open(name = DB_NAME){
  return new Promise((resolve, reject) => {
    let req;
    // `indexedDB` itself THROWS on access in a blocked-cookies third-party
    // context rather than being undefined, so the guard has to be a try, not
    // an `in` check.
    try { req = indexedDB.open(name, DB_VERSION); }
    catch (e) { reject(e); return; }
    req.onupgradeneeded = () => {
      const db = req.result;
      // `seq` IS THE ORDER. An auto-incrementing primary key means a cursor in
      // its natural direction is insertion order, which is the property the
      // whole design rests on -- a "delivered" must never overtake the
      // "picked up" in front of it, or the FSM refuses work that was done.
      if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE, { keyPath: 'seq', autoIncrement: true });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
    req.onblocked = () => reject(new Error('outbox: database blocked by another tab'));
  });
}

const tx = (db, mode, fn) => new Promise((resolve, reject) => {
  let t;
  try { t = db.transaction(STORE, mode); } catch (e) { reject(e); return; }
  // THE TRANSACTION'S `complete` IS THE ANSWER, not the request's `success`.
  // A `put` whose success fired and whose transaction then aborted (quota) has
  // not stored anything, and resolving on success would have reported that tap
  // as queued. `done` carries the value out; `complete` is what resolves.
  let out;
  t.oncomplete = () => resolve(out);
  t.onerror = () => reject(t.error);
  t.onabort = () => reject(t.error || new Error('outbox: transaction aborted'));
  try { fn(t.objectStore(STORE), v => { out = v; }); } catch (e) { try { t.abort(); } catch {} reject(e); }
});

const asPromise = req => new Promise((resolve, reject) => {
  req.onsuccess = () => resolve(req.result);
  req.onerror = () => reject(req.error);
});

/// An outbox over one origin's `/api`.
///
/// `authorize()` is asked for headers AT DRAIN TIME, not at tap time: a courier
/// whose token was refreshed while the phone was in a pocket must not replay a
/// tap under the token that has since expired. It may return `null` to mean
/// "there is no session right now", which pauses the drain rather than burning
/// the entry's tries against a 401.
// `name`: ONE QUEUE PER APP. The courier and the room run on the same origin;
// sharing one database would let one app's drain replay the other's taps under
// its own token, and one app's sign-out would clear the other's queue.
export function createOutbox({ name = DB_NAME, authorize = () => ({}), onChange = () => {}, onSent = () => {}, onDropped = () => {}, fetchImpl, now = () => Date.now() } = {}){
  const doFetch = fetchImpl || ((...a) => fetch(...a));
  let dbp = null;
  let draining = false;
  /// A tap queued WHILE a drain is in flight must not wait for the next
  /// `online` event that may never come. The re-entrant call cannot run the
  /// loop (one sender, or two tabs' worth of duplicates), so it leaves this
  /// mark and the finishing drain runs again.
  let rerun = false;
  let timer = null;
  let stopped = false;

  const db = () => (dbp ||= open(name).catch(e => { dbp = null; throw e; }));

  async function all(){
    const d = await db();
    return tx(d, 'readonly', (store, done) => { asPromise(store.getAll()).then(done); });
  }
  async function count(){
    const d = await db();
    return tx(d, 'readonly', (store, done) => { asPromise(store.count()).then(done); });
  }
  async function head(){
    const d = await db();
    const rows = await tx(d, 'readonly', (store, done) => { asPromise(store.getAll(null, 1)).then(done); });
    return rows[0] || null;
  }
  async function remove(seq){
    const d = await db();
    return tx(d, 'readwrite', store => { store.delete(seq); });
  }
  async function bump(entry){
    const d = await db();
    return tx(d, 'readwrite', store => { store.put(entry); });
  }

  /// Everything still waiting, oldest first, for a surface to draw.
  async function pending(){
    try { return await all(); } catch { return []; }
  }

  function announce(){ pending().then(rows => { try { onChange(rows); } catch {} }); }

  /// Put a tap in the queue. The key is minted at TAP time and travels with the
  /// entry for the rest of its life.
  ///
  /// A CALLER MAY SUPPLY THE KEY, and the courier surface does. It mints one
  /// before its FIRST, direct attempt and hands the same one here if that
  /// attempt died on the network -- because a request whose response was lost
  /// may well have landed, and queueing it under a fresh key is the duplicate
  /// order the header exists to prevent, arriving by the back door.
  ///
  /// Answers `{ ok, key, seq }` or `{ ok: false, reason }` -- `'full'` when the
  /// bound is reached, `'nostore'` when this browser will not keep a queue at
  /// all. Both are for the caller to SAY, not to swallow.
  async function queue(route, { method = 'POST', body = null, tag = null, key = newKey() } = {}){
    const entry = { route, method, body, key, queued_at: now(), tries: 0, tag, last_status: 0, last_error: '' };
    try {
      if (await count() >= MAX_QUEUE) return { ok: false, reason: 'full' };
      const d = await db();
      const seq = await tx(d, 'readwrite', (store, done) => { asPromise(store.add(entry)).then(done); });
      announce();
      // Try immediately. A tap made while the signal is merely SLOW should not
      // wait for an `online` event that will never fire, because the browser
      // never thought it was offline.
      schedule(0);
      return { ok: true, key, seq };
    } catch (e) {
      return { ok: false, reason: 'nostore', error: String(e?.message || e) };
    }
  }

  function schedule(ms){
    if (stopped) return;
    clearTimeout(timer);
    timer = setTimeout(() => { drain().catch(() => {}); }, ms);
  }

  /// Send what is waiting, in order, until something says wait.
  ///
  /// IT STOPS AT THE FIRST ENTRY IT CANNOT RESOLVE rather than skipping it.
  /// Draining past a stuck "picked up" would deliver "delivered" to an FSM that
  /// has never seen the pickup, and the correct refusal that follows would
  /// destroy a delivery that really happened.
  async function drain(){
    if (stopped) return;
    if (draining) { rerun = true; return; }
    draining = true;
    rerun = false;
    try {
      for (;;) {
        let entry;
        try { entry = await head(); } catch { return; }
        if (!entry) return;

        const auth = await Promise.resolve().then(() => authorize()).catch(() => null);
        // No session: pause, do not spend a try. The courier will sign in
        // again and the tap is still theirs until then.
        if (!auth) { schedule(BACKOFF_MS[0]); return; }

        let res;
        try {
          res = await doFetch(entry.route, {
            method: entry.method,
            headers: { 'content-type': 'application/json', 'idempotency-key': entry.key, ...auth },
            body: entry.body,
          });
        } catch (e) {
          // The network, not the server. Nothing has been decided about this
          // tap, so it keeps its place and its key.
          entry.last_error = String(e?.message || e);
          try { await bump(entry); } catch {}
          schedule(BACKOFF_MS[Math.min(entry.tries, BACKOFF_MS.length - 1)]);
          return;
        }

        if (res.ok || res.status === 204) {
          let payload = null;
          try { payload = res.status === 204 ? null : await res.clone().json(); } catch {}
          try { await remove(entry.seq); } catch {}
          announce();
          try { onSent(entry, payload, res.status); } catch {}
          continue;
        }

        const after = res.status === 409 ? retryAfterOf(res) : null;
        if (after != null) {
          // Rule 4: the first copy of this exact key is still running. This is
          // not a refusal and must not consume a try.
          schedule(after);
          return;
        }

        if (RETRY_STATUS.has(res.status)) {
          entry.tries += 1;
          entry.last_status = res.status;
          if (entry.tries >= MAX_TRIES) {
            try { await remove(entry.seq); } catch {}
            announce();
            try { onDropped(entry, 'server', res.status, ''); } catch {}
            continue;
          }
          try { await bump(entry); } catch {}
          schedule(BACKOFF_MS[Math.min(entry.tries, BACKOFF_MS.length - 1)]);
          return;
        }

        // THE SERVER ANSWERED AND SAID NO. 409 from the FSM (the order moved
        // on without this courier), 403 (it is not theirs any more), 404 (it
        // is gone), 400/422 (this tap can never be accepted). Repeating any of
        // them produces the same answer for ever, so the entry is dropped and
        // the surface is told which one it was. 401 is here too, deliberately:
        // a tap replayed after a DIFFERENT courier signs in on the same phone
        // would put one person's delivery under another's name.
        let detail = '';
        try { detail = (await res.clone().text()).slice(0, 200); } catch {}
        try { await remove(entry.seq); } catch {}
        announce();
        try { onDropped(entry, res.status === 409 ? 'changed' : 'refused', res.status, detail); } catch {}
      }
    } finally {
      draining = false;
      if (rerun) { rerun = false; schedule(0); }
    }
  }

  const onOnline = () => schedule(0);
  const onVisible = () => { if (document.visibilityState === 'visible') schedule(0); };

  /// Begin listening and try once. Called on load: a queue that only drained on
  /// the `online` EVENT would sit for ever on a phone that was already online
  /// when the app opened -- which is every reopen after the tunnel.
  function start(){
    stopped = false;
    addEventListener('online', onOnline);
    document.addEventListener('visibilitychange', onVisible);
    announce();
    schedule(0);
  }

  function stop(){
    stopped = true;
    clearTimeout(timer);
    removeEventListener('online', onOnline);
    document.removeEventListener('visibilitychange', onVisible);
  }

  /// Throw the queue away. Used when the session ends, for the same reason
  /// `replica.js` forgets its copy: the next person at this screen is not
  /// entitled to the last one's unsent work, and replaying it under their
  /// token would file it under their name.
  async function forget(){
    try {
      const d = await db();
      await tx(d, 'readwrite', store => { store.clear(); });
      announce();
    } catch {}
  }

  return { queue, drain, pending, start, stop, forget, get draining(){ return draining; } };
}
