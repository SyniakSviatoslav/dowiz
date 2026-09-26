// THE REGRESSION RULES, IN ONE PLACE (BLUEPRINT-OPTIMIZATION-AND-EVALS-2026-09-24 §B.3).
//
// A measurement that cannot fail is a dashboard (§B.1.2). Every indicator names one of
// these rules, and the rule — not the collector — decides whether the number is a breach.
//
//   ratchet  value <= baseline; a LOWER value rewrites the baseline in the same run and says so
//   plus25   value <= baseline * 1.25 (anything with a clock in it)
//   exact    value == baseline (counts that must not move without a dated note)
//   zero     value == 0
//   max      value <= limit (a fixed ceiling named in the catalogue, e.g. 800 per mille)
//   min      value >= limit (e.g. 20 of 20 gates green, coverage >= 80)
//   floor    value >= baseline; a HIGHER value raises the baseline (tests, coverage)
//   trend    recorded, never fails
//
// UNVERIFIED is not a value. An indicator whose value is null carries `unverified: <why>` and
// is never judged green: it is listed apart, so a missing number cannot pass as a good one.

export const RULES = ['ratchet', 'plus25', 'exact', 'zero', 'max', 'min', 'floor', 'trend'];

/** One indicator, the shape every collector returns. */
export function ind(id, value, unit, rule, source, extra = {}) {
  if (!RULES.includes(rule)) throw new Error(`unknown rule ${rule} for ${id}`);
  return { id, value, unit, rule, source, ...extra };
}

/** An indicator that could not be measured, and why. Never judged. */
export function unverified(id, unit, rule, source, why) {
  return ind(id, null, unit, rule, source, { unverified: why });
}

/**
 * Judge one indicator against its baseline.
 * Returns { status: 'ok'|'breach'|'new'|'unverified'|'improved', next } where `next` is the
 * baseline value to store (undefined = leave the stored one alone).
 */
export function judge(i, baseline) {
  if (i.value === null || i.value === undefined) return { status: 'unverified' };
  const v = Number(i.value);
  const lim = i.limit;
  switch (i.rule) {
    case 'trend':
      return { status: 'ok' };
    case 'zero':
      return { status: v === 0 ? 'ok' : 'breach', why: v === 0 ? '' : `${v} != 0` };
    case 'max':
      return v <= lim ? { status: 'ok' } : { status: 'breach', why: `${v} > limit ${lim}` };
    case 'min':
      return v >= lim ? { status: 'ok' } : { status: 'breach', why: `${v} < limit ${lim}` };
    default:
      break;
  }
  if (baseline === undefined || baseline === null || baseline === '') return { status: 'new', next: v };
  const b = Number(baseline);
  switch (i.rule) {
    case 'ratchet':
      if (v > b) return { status: 'breach', why: `${v} > baseline ${b}` };
      return v < b ? { status: 'improved', next: v, why: `lowered ${b} -> ${v}` } : { status: 'ok' };
    case 'floor':
      if (v < b) return { status: 'breach', why: `${v} < baseline ${b}` };
      return v > b ? { status: 'improved', next: v, why: `raised ${b} -> ${v}` } : { status: 'ok' };
    case 'plus25':
      return v <= b * 1.25 ? { status: 'ok' } : { status: 'breach', why: `${v} > ${b} x 1.25` };
    default: // exact
      return v === b ? { status: 'ok' } : { status: 'breach', why: `${v} != baseline ${b}` };
  }
}

/** `key=value` lines, `#` comments — the shape of tools/gates/*.baseline. */
export function parseBaseline(text) {
  const out = {};
  for (const line of String(text).split('\n')) {
    const t = line.trim();
    if (!t || t.startsWith('#')) continue;
    const eq = t.indexOf('=');
    if (eq < 1) continue;
    out[t.slice(0, eq)] = t.slice(eq + 1);
  }
  return out;
}

export function formatBaseline(obj, header) {
  const keys = Object.keys(obj).sort();
  return `# ${header}\n` + keys.map(k => `${k}=${obj[k]}`).join('\n') + '\n';
}

/** Which baseline file an indicator lives in: its id's first segment (`wasm.raw` -> wasm). */
export const groupOf = id => id.split('.')[0];

/** Median of a list of numbers (three runs for anything with a clock, §B.1.4). */
export function median(xs) {
  const v = xs.filter(Number.isFinite).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = Math.floor(v.length / 2);
  return v.length % 2 ? v[m] : Math.round((v[m - 1] + v[m]) / 2);
}

/** Nearest-rank percentile. */
export function pct(xs, p) {
  const v = xs.filter(Number.isFinite).sort((a, b) => a - b);
  if (!v.length) return null;
  return v[Math.min(v.length - 1, Math.max(0, Math.ceil(v.length * p) - 1))];
}
