// THE STAMP CARD's two pure rules for the browser (C5): what the customer's
// order page may draw, and the order in which the console saves the three
// settings. No imports, so `node --test lib/stamps.test.mjs` calls them for
// real. The count itself is the hub's fold (`services/loyalty/stamps.rs`).
//
// ASCII QUOTES ONLY in this file.

/// What the order page draws from `GET /api/order/:id/stamps`, or null when
/// the venue has no card. `have` is clamped to 0..n and nothing is invented:
/// no "almost there", no expiry, no level.
export function stampView(d){
  if (!d || !d.on || !(d.n > 0)) return null;
  const n = Math.floor(d.n);
  const have = Math.min(Math.max(Math.floor(d.have) || 0, 0), n);
  return { have, n, reward: Math.max(Math.floor(d.reward) || 0, 0), used: Math.max(Math.floor(d.used) || 0, 0) };
}

/// The console's three writes, in order. The switch goes LAST: a count or a
/// reward the hub refuses stops the save before the card is on.
export function stampValues(on, n, reward){
  return [['loyalty.stamps.n', String(n).trim()], ['loyalty.stamps.reward_minor', String(reward).trim()],
    ['loyalty.stamps.enabled', on ? '1' : '0']];
}
