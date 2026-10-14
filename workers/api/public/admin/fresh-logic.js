// AFTER A SAVE, EVERY CACHE IS STALE -- THE ONE RULE (lane W-TZFRESH,
// 2026-10-13, operator TZ 2026-10-12 item 6: "the page never needs F5").
//
// THE DEFECT. The console kept three answers for a minute each (the shelf,
// the dishes needing review, the categories), every one behind its own
// `_MS` constant and its own `forget*()` that one save remembered to call and
// the next did not: rename a supply on the stock screen, open a dish, and the
// recipe showed the old name and the old food cost for up to sixty seconds.
//
// THE RULE. One version number for the whole console. EVERY write that lands
// (`core.js api()`, the one fetch wrapper) bumps it; a cache made with
// `fresh.cache(read)` re-reads on its next ask after any bump it listens to
// (all writes by default, or the paths `stale` names), and still after its
// own `ttlMs` so a save made on ANOTHER device shows up too. A read that was
// in flight when the bump landed does not count as fresh: the bump arrived
// after it started, so the next ask reads again.
//
// Pure: no DOM, no network, the clock is injected (`fresh-logic.test.mjs`).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

export function createFresh(){
  let version = 1;
  const subs = new Set();
  const fresh = {
    /// The console's write count since boot (never the same twice after a save).
    version: () => version,
    /// A write landed at `path`: every listening cache forgets what it holds.
    bump(path = ''){
      version += 1;
      for (const f of [...subs]) { try { f(path, version); } catch { /* one listener's error is not another's */ } }
      return version;
    },
    /// Hear every bump; returns the unsubscribe.
    on(f){ subs.add(f); return () => subs.delete(f); },
    /// A value read once and kept until a write makes it stale (or `ttlMs`
    /// passes). `read(prev)` gets the previous value, so a reader may fall
    /// back on it when the hub does not answer. Concurrent asks share one read.
    cache(read, { ttlMs = 0, stale = () => true, now = () => Date.now() } = {}){
      let value, has = false, dirty = true, at = 0, pending = null;
      fresh.on(path => { if (stale(path)) dirty = true; });
      const get = () => {
        if (has && !dirty && !(ttlMs > 0 && now() - at > ttlMs)) return Promise.resolve(value);
        if (pending) return pending;
        pending = (async () => {
          dirty = false;   // this read answers every bump BEFORE it started; one DURING it sets dirty again
          try { value = await read(has ? value : undefined); has = true; at = now(); return value; }
          catch (e) { dirty = true; throw e; }
          finally { pending = null; }
        })();
        return pending;
      };
      /// What the cache holds right now (undefined before the first read), for synchronous readers.
      get.peek = () => value;
      get.forget = () => { dirty = true; };
      return get;
    },
  };
  return fresh;
}

/// The console's one version: `core.js api()` bumps it after every write that landed.
export const fresh = createFresh();
