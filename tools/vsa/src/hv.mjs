// Hypervector core (true VSA math) — bipolar {−1,+1}, D=8192, fully deterministic.
//
// These vectors NEVER travel to an LLM (8192 dims serialized ≈ thousands of tokens — the
// opposite of economy). They exist so that matching/recall/dedup runs LOCALLY in
// microseconds at ZERO token cost: the saved tokens are the whole LLM calls not made.
// hvFor(x) is a pure function of the string — "SEARCH_LOGISTICS" maps to the same vector
// on every machine forever, no registry (fnv seed → splitmix64 bit-stream).

import { fnv1a64, splitmix64 } from './fnv.mjs';

export const D = 8192;

const cache = new Map();

/** Deterministic atomic hypervector for a symbol. */
export function hvFor(symbol) {
  let v = cache.get(symbol);
  if (v) return v;
  v = new Int8Array(D);
  const rng = splitmix64(fnv1a64(symbol));
  for (let i = 0; i < D; i += 64) {
    let bits = rng.next().value;
    for (let j = 0; j < 64 && i + j < D; j++) {
      v[i + j] = bits & 1n ? 1 : -1;
      bits >>= 1n;
    }
  }
  if (cache.size < 4096) cache.set(symbol, v);
  return v;
}

/** bind — elementwise product (self-inverse: bind(bind(a,b),b) ≈ a). */
export function bind(a, b) {
  const out = new Int8Array(D);
  for (let i = 0; i < D; i++) out[i] = a[i] * b[i];
  return out;
}

/** bundle — majority sign of the sum (ties break positive, deterministically). */
export function bundle(vectors) {
  const sum = new Int32Array(D);
  for (const v of vectors) for (let i = 0; i < D; i++) sum[i] += v[i];
  const out = new Int8Array(D);
  for (let i = 0; i < D; i++) out[i] = sum[i] >= 0 ? 1 : -1;
  return out;
}

export function cosine(a, b) {
  let dot = 0;
  for (let i = 0; i < D; i++) dot += a[i] * b[i];
  return dot / D; // bipolar: |a|=|b|=√D
}

/** Text → hypervector: bundle of word unigrams + order-preserving bound bigrams. */
export function textHv(text) {
  const words = String(text).toLowerCase().match(/[a-z0-9_./-]+/g) || [];
  if (words.length === 0) return hvFor('∅');
  const parts = words.map(hvFor);
  for (let i = 0; i + 1 < words.length; i++) {
    parts.push(bind(hvFor(words[i]), hvFor('→' + words[i + 1])));
  }
  return bundle(parts);
}

/**
 * Rank corpus items by similarity to a query — the zero-token replacement for
 * "ask an LLM which lesson/memory/loop matches this task".
 */
export function match(query, items) {
  const q = textHv(query);
  return items
    .map((it) => ({ ...it, score: cosine(q, textHv(it.text)) }))
    .sort((a, b) => b.score - a.score);
}

/**
 * Prediction-error between two states (Active Inference signal): 1 − cos(pred, actual).
 * 0 ≈ perfect prediction; > threshold ⇒ surprise ⇒ escalate (doubt-escalation ladder).
 */
export function predictionError(predText, actualText) {
  return 1 - cosine(textHv(predText), textHv(actualText));
}
