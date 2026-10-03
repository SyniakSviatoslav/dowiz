// The watcher's pure folds: what changed this tick, and what /status says. No I/O, so the
// tests in ../test/ drive them with plain objects.

/// /healthz goes red when the last tick is older than this: three missed 5-minute ticks.
export const STALE_MS = 15 * 60 * 1000;

/// What changed between the stored state and this tick's results. Pure.
/// A target never seen before that is up is a baseline, not news; one that is down IS news.
export function transitions(prev, results) {
  const out = [];
  for (const r of results) {
    const p = prev[r.name];
    const was = p ? p.status : null;
    if (was === r.status) continue;
    if (was === null && r.status === 'up') continue;
    out.push({ name: r.name, url: r.url, from: was, to: r.status, detail: r.detail });
  }
  return out;
}

/// The /status document. Pure over what the object holds.
export function statusDoc(state, env, nowMs) {
  const tickAt = state.tick ? state.tick.atMs : null;
  const latest = (state.tick && state.tick.latest) || {};
  const targetsOut = {};
  for (const [name, t] of Object.entries(state.targets)) {
    const l = latest[name] || {};
    targetsOut[name] = {
      status: t.status,
      detail: l.detail || t.detail,
      url: t.url,
      lastCheckMs: tickAt,
      sinceMs: t.since,
      forMinutes: Math.floor((nowMs - t.since) / 60000),
      latencyMs: l.ms == null ? null : l.ms,
    };
  }
  const notUp = Object.entries(targetsOut).filter(([, t]) => t.status !== 'up').map(([n]) => n);
  const stale = tickAt == null || nowMs - tickAt > STALE_MS;
  return {
    ok: !stale && notUp.length === 0 && Object.keys(targetsOut).length > 0,
    stale,
    lastTickMs: tickAt,
    tickAgeSeconds: tickAt == null ? null : Math.floor((nowMs - tickAt) / 1000),
    notUp,
    targets: targetsOut,
    lastMail: state.mail,
    watcher: { version: env.WATCH_VERSION, commit: env.WATCH_COMMIT, staleAfterMs: STALE_MS },
  };
}
