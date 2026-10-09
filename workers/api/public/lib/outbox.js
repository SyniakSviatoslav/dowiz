// THE TAP THAT SURVIVES THE BASEMENT (the offline write queue). Why and how: crates/dowiz-canvas/HOST-LIBS.md, outbox.js.

const DB_NAME = 'dowiz.outbox';
const DB_VERSION = 1;
const STORE = 'queue';

export const MAX_QUEUE = 64;

const RETRY_STATUS = new Set([408, 425, 429, 500, 502, 503, 504]);
const MAX_TRIES = 6;
const BACKOFF_MS = [2_000, 5_000, 15_000, 30_000, 60_000, 120_000];

export function newKey(){
  try { return crypto.randomUUID(); }
  catch { return `k-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`; }
}

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
    try { req = indexedDB.open(name, DB_VERSION); }
    catch (e) { reject(e); return; }
    req.onupgradeneeded = () => {
      const db = req.result;
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

export function createOutbox({ name = DB_NAME, authorize = () => ({}), onChange = () => {}, onSent = () => {}, onDropped = () => {}, fetchImpl, now = () => Date.now() } = {}){
  const doFetch = fetchImpl || ((...a) => fetch(...a));
  let dbp = null;
  let draining = false;
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

  async function pending(){
    try { return await all(); } catch { return []; }
  }

  function announce(){ pending().then(rows => { try { onChange(rows); } catch {} }); }

  async function queue(route, { method = 'POST', body = null, tag = null, key = newKey() } = {}){
    const entry = { route, method, body, key, queued_at: now(), tries: 0, tag, last_status: 0, last_error: '' };
    try {
      if (await count() >= MAX_QUEUE) return { ok: false, reason: 'full' };
      const d = await db();
      const seq = await tx(d, 'readwrite', (store, done) => { asPromise(store.add(entry)).then(done); });
      announce();
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
        if (!auth) { schedule(BACKOFF_MS[0]); return; }

        let res;
        try {
          res = await doFetch(entry.route, {
            method: entry.method,
            headers: { 'content-type': 'application/json', 'idempotency-key': entry.key, ...auth },
            body: entry.body,
          });
        } catch (e) {
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

  async function forget(){
    try {
      const d = await db();
      await tx(d, 'readwrite', store => { store.clear(); });
      announce();
    } catch {}
  }

  return { queue, drain, pending, start, stop, forget, get draining(){ return draining; } };
}
