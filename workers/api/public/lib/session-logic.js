// BACKGROUND RENEWAL (W-SESSION, operator TZ 7, 2026-10-12). Every app's
// access token is short (a day, a shift) and is renewed here BEFORE it ends,
// while the page is open, so a person mid-edit never meets the 401 the old
// console answered with "session over". Pure: the clock, the timers, the
// visibility and the renewal call are injected, so the tests prove the rule.
//
// NOTHING HERE TRUSTS THE TOKEN. `claimsOf` reads the payload for its expiry
// and role the way a wristwatch is read: to know when to ask again. The hub
// re-derives authority on every request.

/// Renew when this much of the token's life is left: two hours, which is
/// twenty-four ticks of five minutes -- enough that a tab asleep for a while
/// (a laptop lid) still wakes before the token ends.
export const RENEW_AHEAD_MS = 2 * 60 * 60 * 1000;
/// How often the renewer looks.
export const RENEW_EVERY_MS = 5 * 60 * 1000;

function b64url(s) {
  try {
    const pad = s.length % 4 ? '='.repeat(4 - (s.length % 4)) : '';
    return atob(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
  } catch { return null; }
}
/// The payload of a JWT, or null for anything that is not one.
export function claimsOf(jwt) {
  if (typeof jwt !== 'string') return null;
  const parts = jwt.split('.');
  if (parts.length !== 3) return null;
  const raw = b64url(parts[1]);
  if (raw == null) return null;
  try {
    const v = JSON.parse(decodeURIComponent(escape(raw)));
    return v && typeof v === 'object' && !Array.isArray(v) ? v : null;
  } catch { return null; }
}
/// The expiry in ms (the hub writes `exp` in ms), or null.
export function expOf(jwt) {
  const c = claimsOf(jwt);
  return c && Number.isFinite(c.exp) ? c.exp : null;
}
export const roleOf = jwt => claimsOf(jwt)?.role ?? null;

/// Is it time to ask for a new token? An expired one is overdue, not exempt.
export function dueToRenew(jwt, nowMs, ahead = RENEW_AHEAD_MS) {
  const exp = expOf(jwt);
  return exp != null && exp - nowMs <= ahead;
}

/// The renewer. `renew()` is the app's own call (owner refresh family, courier
/// refresh token, staff bearer renewal); it is asked once at a time and only
/// while the page is visible.
export function createRenewer({ token, renew, now = () => Date.now(), visible = () => true,
  every = RENEW_EVERY_MS, ahead = RENEW_AHEAD_MS,
  setInterval: setI = globalThis.setInterval, clearInterval: clearI = globalThis.clearInterval } = {}) {
  let timer = null, inflight = null;
  async function tick() {
    if (inflight) return inflight;
    if (!visible() || !dueToRenew(token(), now(), ahead)) return false;
    inflight = (async () => { try { return !!(await renew()); } catch { return false; } finally { inflight = null; } })();
    return inflight;
  }
  return {
    tick,
    start() { if (timer == null) timer = setI(tick, every); return this; },
    stop() { if (timer != null) clearI(timer); timer = null; },
  };
}
