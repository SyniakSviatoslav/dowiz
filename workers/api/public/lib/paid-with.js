// HOW AN ORDER WAS PAID, as the console shows it (2026-09-24 live walk: every
// room round said "cash", because the placement stamped `payment: 'cash'`).
//
// A delivery or pickup order chooses its method at checkout: `payment` is the
// truth. A round at a table (`fulfilment.kind === 'dine_in'`) is paid AT THE
// TABLE, later, possibly by several people in several ways: the truth is the
// `payments[]` the room's pay command wrote (`method` on each), and a round no
// one has paid yet has no method at all -- whatever its `payment` field says.
// PURE: an order in, the distinct methods out, in the order they were taken.
export function paidWith(o) {
  if (!o || typeof o !== 'object') return [];
  const taken = [];
  for (const p of Array.isArray(o.payments) ? o.payments : []) {
    const m = p && typeof p.method === 'string' ? p.method : '';
    if (m && !taken.includes(m)) taken.push(m);
  }
  if (taken.length) return taken;
  if (o.fulfilment && o.fulfilment.kind === 'dine_in') return [];
  return typeof o.payment === 'string' && o.payment ? [o.payment] : [];
}

/// The icon for the first method: cash, crypto, else a card.
export function paidIcon(methods) {
  const m = methods[0];
  return m === 'cash' ? 'cash' : m === 'crypto' ? 'currency-bitcoin' : 'credit-card';
}
