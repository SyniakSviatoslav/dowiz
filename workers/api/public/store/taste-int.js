// PURE. The integer arithmetic the phone ranks dishes with (W-TASTE, 2026-10-05).
//
// The line-for-line twin of crates/dowiz-hub/src/rank.rs: the same Q16 half-life table built by
// the same Q32 chain (BigInt here, u128 there), the same per-mille cosine, the same truncation.
// Every value stays an integer below 2^53, so a JavaScript number holds it exactly and `+`, `*`
// and `Math.trunc(a / b)` agree with Rust's i64 to the last unit. taste-int.test.mjs and
// rank/tests.rs hold the two equal on one shared fixture (crates/dowiz-hub/fixtures/rank/strip.json).
//
// No float decides anything here: the one Math.sqrt is a GUESS that integer comparisons correct.

/// One portion, as an integer weight (the server's UNIT).
export const UNIT = 1000;
export const HALF_LIFE_DAYS = 60;
/// round(2^32 * 2^(-1/60)): one day of the half-life as a Q32 factor (rank.rs STEP).
export const STEP = 4245635389n;
/// A weight is clamped to this magnitude before it is faded, so |w| * 2^16 stays below 2^53.
export const W_MAX = 2 ** 37 - 1;

/// HALF[r] = round(2^16 * 2^(-r/60)), r = 0..59, from the Q32 chain.
export const HALF = (() => {
  const out = [];
  let q32 = 1n << 32n;
  for (let r = 0; r < 60; r++) {
    out.push(Number((q32 + (1n << 15n)) >> 16n));
    q32 = (q32 * STEP + (1n << 31n)) >> 32n;
  }
  return out;
})();

/// `w` after `days` of the half-life, rounded half away from zero; days <= 0 leaves it.
export function fade(w, days){
  if (days <= 0) return w;
  const s = 16 + Math.floor(days / HALF_LIFE_DAYS);
  if (s > 52) return 0;
  const m = Math.min(Math.abs(w), W_MAX) * HALF[days % HALF_LIFE_DAYS];
  const q = Math.floor((m + 2 ** (s - 1)) / 2 ** s);
  return w < 0 ? -q : q;
}

/// floor(sqrt(n)) for 0 <= n < 2^53, exactly.
export function isqrt(n){
  if (!(n > 0)) return 0;
  let r = Math.floor(Math.sqrt(n));
  while (r * r > n) r -= 1;
  while ((r + 1) * (r + 1) <= n) r += 1;
  return r;
}

/// A vector scaled to per mille of its strongest key (truncated); keys at 0 or below dropped.
export function perMille(v){
  const vals = Object.values(v || {});
  const top = Math.min(vals.length ? Math.max(...vals) : 0, W_MAX);
  const out = {};
  if (!(top > 0)) return out;
  for (const [k, w] of Object.entries(v)) {
    if (!(w > 0)) continue;
    const x = Math.min(Math.trunc(Math.min(w, W_MAX) * 1000 / top), 1000);
    if (x > 0) out[k] = x;
  }
  return out;
}

/// Cosine of two sparse vectors in per mille, truncated toward zero; 0 when either is empty.
export function cosPm(a, b){
  let dot = 0, na = 0, nb = 0;
  for (const [k, x] of Object.entries(a || {})) { na += x * x; if (b && Object.hasOwn(b, k)) dot += x * b[k]; }
  for (const y of Object.values(b || {})) nb += y * y;
  return na && nb ? Math.trunc(dot * 1000 / isqrt(na * nb)) : 0;
}

/// Byte order of two ids (Rust's `str` order for every id below U+10000).
export const byId = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
