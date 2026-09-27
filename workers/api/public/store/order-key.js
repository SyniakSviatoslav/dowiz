// THE STOREFRONT'S Idempotency-Key, minted at the tap and kept until the answer.
//
// THE DEFECT (audit 2026-09-27, W-AUDIT F1). The Worker's idempotency layer
// was written for one scenario above all others -- its own header says so:
// "a customer on a weak connection whose response is lost, and whose app
// retries, gets a SECOND ORDER". `storefront::place` guards on the key. The
// storefront never sent one. `checkout.js` posted the basket with a content
// type and nothing else, so the second tap after "the order did not go
// through" was a second order: a second stock reservation, a second kitchen
// ticket, a second courier. The waiter's room, the courier app and the console
// all mint a key; the customer -- the one on the weak connection -- did not.
//
// THE RULE. One key per BASKET ATTEMPT: the same body sent again carries the
// same key, so the Worker answers the first call's whole response (rule 1 of
// `idempotency/mod.rs`) instead of cooking twice. A DIFFERENT body gets a new
// key, because the Worker refuses a reused key with a different body (rule 3,
// 409) and a customer who fixed a typo in the address must not be told "you
// already used this key". The key is forgotten once an order was answered, so
// the next basket is a new order. PURE: the state and the minting function
// are handed in (`order-key.test.mjs`). ASCII quotes only (rule 11).

/// The key to send with `body` (the exact JSON string that goes on the wire).
/// `mint` answers a fresh key; it is called once per new body. `null` when no
/// key can be minted, which is the behaviour before this file existed.
export function keyFor(keys, body, mint = defaultMint){
  if (keys.body === body && keys.key) return keys.key;
  const key = mint();
  if (!key) { keys.key = null; keys.body = null; return null; }
  keys.key = key;
  keys.body = body;
  return key;
}

/// The order was answered (placed, or refused with an answer): the next
/// basket is a new attempt with a new key.
export function answered(keys){
  keys.key = null;
  keys.body = null;
}

/// The Worker's status for this attempt, and what it means for the key. A
/// 409 is the Worker saying the FIRST call is still running (rule 4 of
/// `idempotency/mod.rs`, `Retry-After`): the key stays, so the next tap asks
/// for that call's answer instead of placing a second order. Any other
/// status is an answer, recorded under the key, and the key is spent.
export function settle(keys, status){
  if (status === 409) return false;
  answered(keys);
  return true;
}

/// A fresh key from the platform CSPRNG, or `null` where there is none --
/// never a weaker source, for the same reason the Worker refuses to mint an
/// order id without one.
export function defaultMint(){
  const c = globalThis.crypto;
  return c && typeof c.randomUUID === 'function' ? c.randomUUID() : null;
}
