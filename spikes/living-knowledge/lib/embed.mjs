// embed.mjs — DETERMINISTIC, zero-dependency text embedding + cosine.
//
// Why not a neural embedding: the living-knowledge layer must be reproducible, cacheable, and
// gate-able (§0·GP / Verified-by-Math). A hashed signed-random-projection bag-of-words gives a fixed
// vector for a fixed string with NO network, NO model, NO randomness — so `trace(query, band)` is
// bit-identical across runs (the determinism proof depends on this). A real sentence embedding is a
// drop-in swap behind this same interface (embed(text) -> Float array) once one is licensed/offline.
//
// Signed random projection: each token contributes to D dims; a second hash picks the sign, which
// halves systematic collision bias (a token and its collision partner cancel in expectation).

export const DIM = 256;

// FNV-1a 32-bit — deterministic, fast, no deps.
function fnv1a(str, seed = 0x811c9dc5) {
  let h = seed >>> 0;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

export function tokenize(text) {
  return (text || '')
    .toLowerCase()
    .split(/[^a-z0-9_]+/)
    .filter((t) => t.length >= 2 && t.length <= 40);
}

// Bag-of-words → signed-random-projection vector, L2-normalized. Deterministic.
// With an `idf` map (token -> inverse-document-frequency weight, built by the store over the corpus),
// each token's contribution is scaled by its IDF. This is the fix for "everything documents": a token
// that appears in nearly every file (loop, gate, the) carries ~0 weight, so a long index file no
// longer matches every query; a distinctive token (subagent, helixdb, falsifiable) dominates. Query
// tokens absent from the corpus get weight 0 → a nonsense query yields a ~zero vector (cosine ~0),
// which is exactly what makes the "real query scores higher than nonsense" assertion falsifiable.
// idf omitted (store not finalized) → raw BoW (weight 1).
export function embed(text, idf = null) {
  const v = new Float64Array(DIM);
  for (const t of tokenize(text)) {
    const wgt = idf ? (idf.get(t) ?? 0) : 1;
    if (wgt <= 0) continue;
    const dim = fnv1a(t) % DIM;
    const sign = (fnv1a(t, 0x9e3779b1) & 1) ? 1 : -1;
    v[dim] += sign * wgt;
  }
  // L2 normalize (a zero vector stays zero → cosine 0, handled below).
  let norm = 0;
  for (let i = 0; i < DIM; i++) norm += v[i] * v[i];
  norm = Math.sqrt(norm);
  if (norm > 0) for (let i = 0; i < DIM; i++) v[i] /= norm;
  return v;
}

export function cosine(a, b) {
  let dot = 0;
  for (let i = 0; i < a.length; i++) dot += a[i] * b[i];
  return dot; // both L2-normalized → dot == cosine
}
