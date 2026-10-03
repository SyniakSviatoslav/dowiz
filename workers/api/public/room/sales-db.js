// THE TABLET'S OWN RECORD OF ITS OFFLINE SALES (W-OFFSALE), in IndexedDB.
//
// WHY A SECOND STORE BESIDE THE OUTBOX. The outbox is a TRANSPORT: it forgets
// what it sent, and it is cleared on sign-out (`net.js`, `OUT.forget`) so the
// next person does not replay the last one's taps. A cash sale is not a tap:
// the money is in the drawer. This journal is what the tablet knows it sold,
// keyed by the sale's key, until the server has it (`status: 'synced'`); a
// sale the outbox lost (signed out, 401, cleared) is queued again from here
// under its SAME key, which the server answers once (`offline:<key>`).
//
// IT REFUSES RATHER THAN DEGRADES, like the outbox: a browser that will not
// store answers `false` and the screen tells the seller not to sell.
const DB = 'dowiz.room.sales';
const STORE = 'sales';

function open() {
  return new Promise((resolve, reject) => {
    let req;
    try { req = indexedDB.open(DB, 1); } catch (e) { reject(e); return; }
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE, { keyPath: 'sale_key' });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
    req.onblocked = () => reject(new Error('sales: database blocked by another tab'));
  });
}

let dbp = null;
const db = () => (dbp ||= open().catch(e => { dbp = null; throw e; }));

/// One transaction; resolves on `complete` (a `put` that aborted stored nothing).
async function tx(mode, fn) {
  const d = await db();
  return new Promise((resolve, reject) => {
    const t = d.transaction(STORE, mode);
    let out;
    t.oncomplete = () => resolve(out);
    t.onerror = () => reject(t.error);
    t.onabort = () => reject(t.error || new Error('sales: transaction aborted'));
    const r = fn(t.objectStore(STORE));
    if (r) r.onsuccess = () => { out = r.result; };
  });
}

/// Keep a sale (the body sent, plus `status` and `loc`). `true` when stored.
export async function keepSale(sale) {
  try { await tx('readwrite', s => s.put(sale)); return true; } catch { return false; }
}

/// Every sale this tablet holds, oldest first.
export async function allSales() {
  try { return ((await tx('readonly', s => s.getAll())) || []).sort((a, b) => a.sold_at_ms - b.sold_at_ms); } catch { return []; }
}

/// Mark one sale (`synced` / `refused`), with what the server said.
export async function markSale(key, status, said = '') {
  try {
    const d = await db();
    const cur = await new Promise((resolve, reject) => {
      const r = d.transaction(STORE, 'readonly').objectStore(STORE).get(key);
      r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error);
    });
    if (!cur) return false;
    await tx('readwrite', s => s.put({ ...cur, status, said }));
    return true;
  } catch { return false; }
}

/// Drop SYNCED sales sold before `beforeMs` (the server holds them); an
/// unsynced or refused sale is never dropped here.
export async function pruneSynced(beforeMs) {
  try {
    const old = (await allSales()).filter(s => s.status === 'synced' && s.sold_at_ms < beforeMs);
    for (const s of old) await tx('readwrite', st => st.delete(s.sale_key));
    return old.length;
  } catch { return 0; }
}
