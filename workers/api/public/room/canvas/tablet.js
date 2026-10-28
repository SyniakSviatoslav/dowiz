// THE KITCHEN TABLET'S LOCAL STOCK LOG (sheaf S2, R-BEBOPDB G.5): the venue's stock log as it was
// at the last sync, kept on the device (IndexedDB), with the tablet's own movements appended while
// it is offline; one sync sends only those, and replays the hub's answer onto the hub's own tip.
//
// Every rule is Rust's: `tablet.wasm` is crates/bebop-wasm built with `--features tablet`, a C
// surface over dowiz_hub::glue::tablet -- the crate the venue's object glues with. This file moves
// bytes: into the module, into IndexedDB, onto the wire (POST /api/staff/stock/glue).
//
// The contract of the wire is services/operations/stock/glue.rs. A 409 means the hub no longer
// holds this tablet's base (the owner reset the shelf): the tablet starts over from the hub's
// whole log and carries its own records on top, so nothing written here is lost.
//
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

export const ZERO = '0'.repeat(64);
const DB = 'dowiz.tablet';
const STORE = 'stalk';

/// The tablet's stalk in IndexedDB: one row per venue and context, {image: Uint8Array, base: hex}.
export function idbStore(key) {
  const open = () => new Promise((ok, no) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE);
    r.onsuccess = () => ok(r.result);
    r.onerror = () => no(r.error);
  });
  const tx = async (mode, f) => {
    const db = await open();
    return new Promise((ok, no) => {
      const t = db.transaction(STORE, mode);
      const req = f(t.objectStore(STORE));
      t.oncomplete = () => { db.close(); ok(req && req.result); };
      t.onerror = () => { db.close(); no(t.error); };
    });
  };
  return {
    get: () => tx('readonly', s => s.get(key)),
    set: v => tx('readwrite', s => s.put(v, key)),
  };
}

/// The module's C surface, with the copy-in / read-back the ABI asks for (crates/bebop-wasm
/// src/tablet.rs header). Every call answers {status, bytes}; a status >= 40 is a refusal whose
/// bytes are its words.
export function surface(instance) {
  const ex = instance.exports;
  const enc = new TextEncoder();
  const put = b => {
    if (!b.length) return [0, 0];
    const p = ex.bw_alloc(b.length);
    new Uint8Array(ex.memory.buffer, p, b.length).set(b);
    return [p, b.length];
  };
  const flat = a => a.flatMap(x => (Array.isArray(x) ? x : [x]));
  const run = (f, args) => {
    const held = args.map(a => (a instanceof Uint8Array ? put(a) : typeof a === 'string' ? put(enc.encode(a)) : a));
    const out = ex.bw_alloc(8);
    const status = f(...flat(held), out);
    const [p, n] = new Uint32Array(ex.memory.buffer, out, 2);
    const bytes = n ? new Uint8Array(ex.memory.buffer, p, n).slice() : new Uint8Array(0);
    if (n) ex.bw_free(p, n);
    ex.bw_free(out, 8);
    for (const h of held) if (Array.isArray(h) && h[1]) ex.bw_free(h[0], h[1]);
    return { status, bytes };
  };
  const words = r => new TextDecoder().decode(r.bytes);
  const must = (r, ok = [0]) => {
    if (!ok.includes(r.status)) { const e = new Error(words(r)); e.code = r.status; throw e; }
    return r;
  };
  return {
    /// One movement in the codec's form; answers {image, provisional}.
    record(image, ctx, rec, nowMs) {
      const r = must(run(ex.bw_tab_record, [image, ctx, rec, BigInt(Math.trunc(nowMs))]), [0, 1]);
      return { image: r.bytes, provisional: r.status === 1 };
    },
    /// The request body (without the venue): {ctx, base, tip, tail}.
    tail(image, ctx, base) { return JSON.parse(words(must(run(ex.bw_tab_tail, [image, ctx, base])))); },
    /// The tablet's shelf: [[item, on_hand], ...], sorted.
    items(image) { return JSON.parse(words(must(run(ex.bw_tab_items, [image])))); },
    /// The hub's reply replayed: the new image.
    rebase(image, base, fromZero, sent, reply) {
      return must(run(ex.bw_tab_rebase, [image, base, fromZero ? 1 : 0, sent, JSON.stringify(reply)])).bytes;
    },
  };
}

/// A tablet: `mod` is `surface(...)`, `store` an `idbStore`, `post(path, body)` the room's api.
export function tablet({ mod, store, post, loc, ctx = 'kitchen', now = () => Date.now() }) {
  const load = async () => (await store.get()) || { image: new Uint8Array(0), base: ZERO };
  return {
    /// Record one movement offline. Answers whether it was a sale past the tablet's rights.
    async record(rec) {
      const s = await load();
      const r = mod.record(s.image, ctx, rec, now());
      await store.set({ image: r.image, base: s.base });
      return r.provisional;
    },
    /// The shelf the offline stock sheet steps through (empty before the first sync).
    async items() {
      const s = await load();
      return s.image.length ? mod.items(s.image) : [];
    },
    /// How many records wait for the next sync.
    async pending() {
      const s = await load();
      return s.image.length ? mod.tail(s.image, ctx, s.base).tail.length : 0;
    },
    /// ONE SYNC: the tail out, the hub's answer replayed. Answers the hub's reply (its
    /// `conflicts` and `held` are what the board shows as decisions after its next read).
    async sync() {
      const s = await load();
      const body = s.image.length ? mod.tail(s.image, ctx, s.base) : { ctx, base: ZERO, tip: ZERO, tail: [] };
      let reply, fromZero = false, sent = body.tail.length;
      try {
        reply = await post('/staff/stock/glue', { location_id: loc, ...body });
      } catch (e) {
        if (e.status !== 409) throw e;
        reply = await post('/staff/stock/glue', { location_id: loc, ctx, base: ZERO, tip: ZERO, tail: [] });
        fromZero = true; sent = 0;
      }
      const image = mod.rebase(s.image, s.base, fromZero, sent, reply);
      await store.set({ image, base: reply.tip });
      return reply;
    },
  };
}
