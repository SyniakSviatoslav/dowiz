// B11: two owner-console loops (workers/api/public/admin).
//  (a) admin/orders.js:93,99  `S.orders.filter(o => !liveOrders().includes(o))` -- liveOrders()
//      (a filter over all orders) is rebuilt INSIDE the filter callback, then a linear includes:
//      O(n^2) with n array allocations per render/keystroke. REPLACEMENT: one Set of live orders.
//  (b) admin/app.js:316-322  for each other language x category x product: `S.products.find(x => x.id === p.id)`
//      REPLACEMENT: a Map id -> product built once.
// Equivalence: identical arrays / identical translations objects. Median of >= 7 runs.
const LIVE = new Set(['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']);
const STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'PAID', 'CANCELLED', 'DELIVERED', 'PAID'];
function orders(n) {
  return Array.from({ length: n }, (_, i) => ({ id: `o_${i}`, status: STATUSES[i % STATUSES.length], total: 1500 + (i % 13) * 250, contact: { name: `Guest ${i}`, phone: '+355690000000' }, items: [{ name: 'Roll 1' }, { name: 'Roll 2' }] }));
}
function median(reps, f) { const v = []; for (let i = 0; i < reps; i++) { const t = process.hrtime.bigint(); f(); v.push(Number(process.hrtime.bigint() - t)); } v.sort((a, b) => a - b); return v[v.length >> 1]; }
const reps = Number(process.argv[2] || 7);
console.log(`JSBENCH node ${process.version} reps=${reps}`);

for (const n of [100, 300, 1000, 3000]) {
  const S = { orders: orders(n) };
  const liveOrders = () => S.orders.filter(o => LIVE.has(o.status));
  const historyCurrent = () => S.orders.filter(o => !liveOrders().includes(o));
  const historyReplace = () => { const live = new Set(liveOrders()); return S.orders.filter(o => !live.has(o)); };
  const a = historyCurrent(), b = historyReplace();
  if (a.length !== b.length || !a.every((o, i) => o === b[i])) throw new Error('B11a not equivalent');
  const cur = median(reps, historyCurrent), rep = median(reps, historyReplace);
  console.log(`B11a EQUIV ok n=${n} (${a.length} history, ${n - a.length} live): CURRENT ${(cur / 1e3).toFixed(0)} us | REPLACE ${(rep / 1e3).toFixed(0)} us | ${(cur / Math.max(rep, 1)).toFixed(1)}x`);
}

const LANGS = ['sq', 'en', 'uk', 'ru'];
for (const dishes of [165, 500]) {
  const cats = 12;
  const mk = (l) => Array.from({ length: cats }, (_, c) => ({ id: `c_${c}`, name: `Cat ${c} ${l}`, products: Array.from({ length: Math.ceil(dishes / cats) }, (_, k) => ({ id: `p_${c * Math.ceil(dishes / cats) + k}`, name: `Roll ${k} ${l}`, description: `Desc ${k} ${l}` })).filter(p => Number(p.id.slice(2)) < dishes) }));
  const own = mk('sq');
  const rest = LANGS.filter(l => l !== 'sq').map(mk);
  const fresh = () => own.flatMap(c => c.products.map(p => ({ ...p, categoryId: c.id, translations: {} })));
  const current = () => { const P = fresh(); for (const [i, r] of rest.entries()) for (const c of r) for (const p of c.products) { const mine = P.find(x => x.id === p.id); if (mine) mine.translations[LANGS.filter(l => l !== 'sq')[i]] = { name: p.name, description: p.description || '' }; } return P; };
  const replace = () => { const P = fresh(); const byId = new Map(P.map(x => [x.id, x])); for (const [i, r] of rest.entries()) for (const c of r) for (const p of c.products) { const mine = byId.get(p.id); if (mine) mine.translations[LANGS.filter(l => l !== 'sq')[i]] = { name: p.name, description: p.description || '' }; } return P; };
  const a = JSON.stringify(current()), b = JSON.stringify(replace());
  if (a !== b) throw new Error('B11b not equivalent');
  const base = median(reps, fresh);
  const cur = median(reps, current), rep = median(reps, replace);
  console.log(`B11b EQUIV ok dishes=${dishes}: CURRENT ${(cur / 1e3).toFixed(0)} us | REPLACE ${(rep / 1e3).toFixed(0)} us (flatMap base ${(base / 1e3).toFixed(0)} us) | loop-only ${((cur - base) / Math.max(rep - base, 1)).toFixed(1)}x`);
}
