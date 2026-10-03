// The published menu, KEPT ON THE DEVICE (BN3, menu half).
//
// The venue's object publishes its menu to the CDN as content-addressed objects
// named `<k64>.json` / `<k64>.dwb` -- the k64 is the first 16 hex of the
// object's sha256 (`hubdo/menu.rs::k64`) -- plus one mutable root,
// `manifest.json`. This module keeps both in IndexedDB so a return visit reads
// the menu with no request when nothing changed, and from the device when the
// network is gone; a new generation costs only the objects whose name changed.
//
// MENU ONLY, NEVER AN ORDER (roadmap AX7, "shred before replicate"). An order
// carries personal data, and once it sits in a device's IndexedDB only
// crypto-shredding (DG10w) can make it forgettable. Nothing here is told about
// orders, customers or baskets: the objects are what every visitor of the
// venue's storefront is served anyway.
//
// NEVER TRUST A STORED BLOB. Every read recomputes the sha256 of the bytes it
// got and refuses a mismatch: the object is deleted, counted, and read from the
// network again. A name that is not a k64 is never stored -- nothing could
// check it.
//
// WHO OWNS WHAT. This store owns the manifest and the k64 objects (the bytes
// the shell parses). The service worker (`/sw.js`) owns the shell and the
// photos (`m/<name>`, which an <img> asks for and cannot be read from here);
// it no longer caches the manifest or the k64 objects, so nothing is kept twice.
//
// BOUNDED: `BUDGET` bytes per venue. Each object remembers the root sequence
// that last used it; `prune` drops the least recently used first and never one
// the current root names.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

/// The IndexedDB name: a `dowiz.*` key, so tools/gates/personal-data.sh holds it to a BROWSER row (it holds nothing personal).
const DB = 'dowiz.blocks.v1';
const OBJECTS = 'objects';
const ROOTS = 'roots';
/// Bytes kept per venue (the 165-dish fragment is ~0.1 MB).
export const BUDGET = 1024 * 1024;
/// The root's own `max-age` (`hubdo/publish.rs::ROOT_CACHE`): inside it, no request at all.
export const FRESH_MS = 30_000;
const K64 = /^([0-9a-f]{16})\.(json|dwb)$/;

/// The k64 a key names, or null when the key is not content-addressed.
export function k64Key(key) {
  const m = K64.exec(String(key));
  return m ? m[1] : null;
}

/// The first 16 hex of sha256(bytes).
export async function k64Of(bytes, subtle = globalThis.crypto && globalThis.crypto.subtle) {
  const d = new Uint8Array(await subtle.digest('SHA-256', bytes));
  return Array.from(d.subarray(0, 8), b => b.toString(16).padStart(2, '0')).join('');
}

/// Whether `bytes` are what `key` names. A key that names nothing is never verified.
export async function verified(key, bytes, subtle) {
  const want = k64Key(key);
  return want !== null && (await k64Of(bytes, subtle)) === want;
}

const asBuffer = bytes => {
  const u = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  return u.buffer.slice(u.byteOffset, u.byteOffset + u.byteLength);
};
const isBytes = v => v && typeof v.byteLength === 'number' && typeof v.slice === 'function';

// ── backends: IndexedDB, and a Map for node ─────────────────────────────────
/// The same four calls over two Maps; tests reach `raw` to tamper with a blob.
export function memoryBackend() {
  const raw = { [OBJECTS]: new Map(), [ROOTS]: new Map() };
  return {
    raw,
    get: async (s, k) => raw[s].get(k),
    put: async (s, k, v) => { raw[s].set(k, v); },
    del: async (s, k) => { raw[s].delete(k); },
    list: async (s, prefix) => [...raw[s]].filter(([k]) => k.startsWith(prefix)).map(([key, value]) => ({ key, value })),
  };
}

const done = req => new Promise((ok, no) => { req.onsuccess = () => ok(req.result); req.onerror = () => no(req.error); });

/// IndexedDB, or null when the browser has none or refuses it (a private window may).
export async function idbBackend(idb = globalThis.indexedDB) {
  if (!idb) return null;
  const open = idb.open(DB, 1);
  open.onupgradeneeded = () => {
    for (const s of [OBJECTS, ROOTS]) if (!open.result.objectStoreNames.contains(s)) open.result.createObjectStore(s);
  };
  const db = await done(open);
  const run = (s, mode, f) => done(f(db.transaction(s, mode).objectStore(s)));
  return {
    get: (s, k) => run(s, 'readonly', st => st.get(k)),
    put: (s, k, v) => run(s, 'readwrite', st => st.put(v, k)),
    del: (s, k) => run(s, 'readwrite', st => st.delete(k)),
    list: (s, prefix) => new Promise((ok, no) => {
      const out = [];
      const req = db.transaction(s, 'readonly').objectStore(s).openCursor(IDBKeyRange.bound(prefix, prefix + '\uffff'));
      req.onerror = () => no(req.error);
      req.onsuccess = () => {
        const c = req.result;
        if (!c) return ok(out);
        out.push({ key: c.key, value: c.value });
        c.continue();
      };
    }),
  };
}

// ── the store ───────────────────────────────────────────────────────────────
/// The verified store over `backend`. `stats` counts what happened, for the console and the gate.
export function blockStore(backend, { subtle = globalThis.crypto && globalThis.crypto.subtle, budget = BUDGET } = {}) {
  const id = (slug, key) => `${slug}/${key}`;
  const stats = { hits: 0, refused: 0, stored: 0, evicted: 0 };
  return {
    stats,
    /// The last root stored for `slug`: `{ text, at, seq }`, or null.
    async root(slug) {
      const r = await backend.get(ROOTS, slug);
      return r && typeof r.text === 'string' && Number.isFinite(r.at) && Number.isInteger(r.seq) ? r : null;
    },
    /// Store a root read at `at`; the sequence moves only when the text did.
    async putRoot(slug, text, at) {
      const prev = await this.root(slug);
      const seq = !prev ? 1 : prev.text === text ? prev.seq : prev.seq + 1;
      await backend.put(ROOTS, slug, { text, at, seq });
      return seq;
    },
    /// The bytes of `key`, VERIFIED, or null. A blob that is not its name is deleted.
    async get(slug, key, seq = 0) {
      if (!k64Key(key)) return null;
      const v = await backend.get(OBJECTS, id(slug, key));
      if (!v || !isBytes(v.bytes)) return null;
      if (!(await verified(key, v.bytes, subtle))) {
        stats.refused++;
        console.warn(`blocks: the device copy of ${key} is not what its name says; refused and read again`);
        await backend.del(OBJECTS, id(slug, key));
        return null;
      }
      stats.hits++;
      if (seq > (v.seen || 0)) await backend.put(OBJECTS, id(slug, key), { ...v, seen: seq });
      return new Uint8Array(v.bytes);
    },
    /// Keep `bytes` under `key` when they ARE its name; answers whether it was kept.
    async put(slug, key, bytes, seq = 0) {
      if (!(await verified(key, bytes, subtle))) return false;
      const buf = asBuffer(bytes);
      await backend.put(OBJECTS, id(slug, key), { bytes: buf, size: buf.byteLength, seen: seq });
      stats.stored++;
      return true;
    },
    /// Drop the least recently used objects of `slug` until it fits `budget`; never one in `keep`.
    async prune(slug, keep = new Set()) {
      const all = await backend.list(OBJECTS, `${slug}/`);
      let total = all.reduce((n, o) => n + (o.value && o.value.size || 0), 0);
      const old = all.filter(o => !keep.has(o.key.slice(slug.length + 1))).sort((a, b) => (a.value.seen || 0) - (b.value.seen || 0));
      let dropped = 0;
      for (const o of old) {
        if (total <= budget) break;
        await backend.del(OBJECTS, o.key);
        total -= o.value.size || 0;
        dropped++;
      }
      stats.evicted += dropped;
      return { total, dropped };
    },
  };
}

let opened = null;
/// The device's store, opened once per page; null when there is no IndexedDB or no
/// `crypto.subtle` (an insecure context): without a way to verify, nothing is kept.
export function openBlocks() {
  if (!opened) {
    opened = (async () => {
      if (!(globalThis.crypto && globalThis.crypto.subtle)) return null;
      const b = await idbBackend();
      return b ? blockStore(b) : null;
    })().catch(e => {
      console.warn('blocks: no device copy of the menu:', e && e.message ? e.message : e);
      return null;
    });
  }
  return opened;
}
