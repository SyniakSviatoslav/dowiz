// Proactive Integrity Layer — deterministic, zero-token pre-flight checks.
//
// SHC (State Hash Checksum): canonical fnv1a64 hash of an object's state — same state,
// same hash, on every machine forever (key order never matters).
// IDR (Integrity Drift): expected-vs-actual delta = exact mismatched paths (authority)
// + a graded hypervector drift score (advisory magnitude, from predictionError).
// Gate: expected != actual → circuit-break BEFORE a lane/dispatch spends its ~17K floor.
// Corridor: a mismatch younger than the sync corridor is IN-FLIGHT, not corrupt —
// time-qualifying IDR is what separates WS-lag from real divergence.
//
// All checks run locally at $0. Advisory-vs-authority: the exact path diff decides;
// the hv drift score only grades HOW far apart the states are.

import { fnv1a64 } from './fnv.mjs';
import { predictionError } from './hv.mjs';

/** Canonical form: field-subset (optional) then recursively key-sorted JSON. */
export function canon(value, fields) {
  const pick =
    fields && !Array.isArray(value) && value !== null && typeof value === 'object'
      ? Object.fromEntries(fields.filter((f) => f in value).map((f) => [f, value[f]]))
      : value;
  return JSON.stringify(sortKeys(pick));
}

function sortKeys(v) {
  if (Array.isArray(v)) return v.map(sortKeys);
  if (v && typeof v === 'object') {
    return Object.fromEntries(
      Object.keys(v)
        .sort()
        .map((k) => [k, sortKeys(v[k])]),
    );
  }
  return v;
}

/** State Hash Checksum — deterministic hex hash of the canonical state. */
export function shc(value, fields) {
  return fnv1a64(canon(value, fields)).toString(16).padStart(16, '0');
}

/** Flat list of paths where two canonical states differ (the exact, deciding signal). */
export function diffPaths(expected, actual, base = '') {
  const e = sortKeys(expected);
  const a = sortKeys(actual);
  if (JSON.stringify(e) === JSON.stringify(a)) return [];
  const eObj = e && typeof e === 'object';
  const aObj = a && typeof a === 'object';
  if (!eObj || !aObj) return [base || '$'];
  const keys = new Set([...Object.keys(e), ...Object.keys(a)]);
  const out = [];
  for (const k of keys) {
    const p = base ? `${base}.${k}` : k;
    if (!(k in e) || !(k in a)) out.push(p);
    else out.push(...diffPaths(e[k], a[k], p));
  }
  return out;
}

/**
 * Integrity gate — the pre-flight circuit breaker.
 * @param {object} expected  state the caller believes is current (e.g. resolver model)
 * @param {object} actual    state read back from the source of truth (DB/API)
 * @param {object} [opts]    { fields, ageMs, corridorMs }
 *   ageMs/corridorMs: a mismatch with ageMs <= corridorMs is IN-FLIGHT (pass + warn),
 *   older is DIVERGED (fail). Default corridor 0 = strict.
 * @returns {{pass:boolean, inCorridor:boolean, shcExpected:string, shcActual:string,
 *            mismatches:string[], drift:number}}
 */
export function integrityGate(expected, actual, opts = {}) {
  const { fields, ageMs = 0, corridorMs = 0 } = opts;
  const shcExpected = shc(expected, fields);
  const shcActual = shc(actual, fields);
  if (shcExpected === shcActual) {
    return { pass: true, inCorridor: false, shcExpected, shcActual, mismatches: [], drift: 0 };
  }
  const mismatches = diffPaths(
    fields ? JSON.parse(canon(expected, fields)) : expected,
    fields ? JSON.parse(canon(actual, fields)) : actual,
  );
  const drift = predictionError(canon(expected, fields), canon(actual, fields));
  const inCorridor = corridorMs > 0 && ageMs <= corridorMs;
  return { pass: inCorridor, inCorridor, shcExpected, shcActual, mismatches, drift };
}
