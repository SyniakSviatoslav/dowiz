// embed-semantic.mjs — OPTIONAL semantic embedder: the drop-in the hash baseline (embed.mjs) always
// anticipated ("a real sentence embedding is a drop-in swap behind this same interface once one is
// licensed/offline"). Same contract: text -> Float array, cosine over L2-normalized vectors.
//
// DETERMINISM (the whole reason this is admissible under §0·GP / Verified-by-Math):
//   • Model = Xenova/bge-small-en-v1.5 (MIT, 384-dim). Fixed weights, CPU ONNX inference →
//     bit-identical output for a fixed input in-process (proven: probe embedded the same string twice,
//     JSON-identical). Chosen over MiniLM by measured recall (bge-small won the model bake-off). bge is
//     an ASYMMETRIC model: QUERIES must be prefixed ("Represent this sentence for searching relevant
//     passages: "), passages left raw — the caller (retriever.mjs) applies the prefix; this module is
//     prefix-agnostic (it embeds whatever string it is handed and caches by its sha).
//   • We do NOT depend on live inference at eval time. prewarm() computes every needed vector ONCE and
//     writes them to out/semantic-cache.json (keyed by sha256 of the exact text, rounded to 6 dp). That
//     committed JSON is the deterministic artifact — the eval then reads vectors from disk with NO model
//     present, so reruns are byte-identical on ANY machine, offline. The neural net is a build-time tool;
//     the shipped engine is the cache. Sovereign: no network, no model, at run time.
//
// OPTIONAL DEP: @huggingface/transformers (Transformers.js). Only imported when prewarm() has a cache
// MISS to fill. If the cache already covers every text, the model is never loaded (pure offline read).
// Not declared in a package.json (that path is a protected governance zone); install it into this
// spike's gitignored node_modules to (re)build the cache. Missing dep + cache miss = a loud throw, not
// a silent degrade (a silent fallback to zeros would be a false-green — forbidden).
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
export const SEM_MODEL = 'Xenova/bge-small-en-v1.5';
export const SEM_DIM = 384;
export const QUERY_PREFIX = 'Represent this sentence for searching relevant passages: ';
const ROUND = 6; // decimal places persisted — determinism floor, cosine-robust.

const sha = (t) => createHash('sha256').update(t).digest('hex');
const round = (x) => Number(x.toFixed(ROUND));
// Integrity checksum over the vector PAYLOAD (sorted key→rounded-vector). The per-text sha256 KEYS
// address the text, not the stored vector — so a swapped/edited vector value would go undetected. This
// digest closes that: recomputing it over the committed vectors and comparing to the stored digest reds
// on any value corruption/tamper. Not a cryptographic signature (an editor could recompute it), but it
// catches accidental corruption and casual tampering of the committed artifact — the realistic threat.
const computeDigest = (vectors) => createHash('sha256').update(Object.keys(vectors).sort().map((k) => `${k}:${vectors[k].join(',')}`).join(';')).digest('hex');

export function createSemanticEmbedder(opts = {}) {
  const cachePath = opts.cachePath || process.env.LK_CACHE || join(HERE, '..', 'out', 'semantic-cache.json');
  let cache = { model: SEM_MODEL, dim: SEM_DIM, vectors: {} };
  // Integrity captured from the file AS LOADED (before any rebuild) — the security-relevant state:
  //   rawHeader      = the header actually on disk (a wrong-model file can't masquerade as correct)
  //   digestVerified = the committed payload digest recomputes to the stored one (detects a tampered/
  //                    corrupted VECTOR VALUE, which the sha256(text) keys do NOT cover).
  let rawHeader = { model: null, dim: null }, digestVerified = false;
  if (existsSync(cachePath)) {
    try {
      const loaded = JSON.parse(readFileSync(cachePath, 'utf8'));
      rawHeader = { model: loaded.model ?? null, dim: loaded.dim ?? null };
      if (loaded.model === SEM_MODEL && loaded.dim === SEM_DIM) cache = loaded;
      if (loaded.digest && loaded.vectors) digestVerified = computeDigest(loaded.vectors) === loaded.digest;
    } catch { /* corrupt cache → rebuild */ }
  }
  let extractor = null; // lazy — only if a miss must be filled
  let built = 0, hits = 0;

  async function ensureModel() {
    if (extractor) return extractor;
    let transformers;
    try {
      transformers = await import('@huggingface/transformers');
    } catch (e) {
      throw new Error(
        `semantic embedder: cache miss but @huggingface/transformers is not installed.\n` +
        `Install it into spikes/living-knowledge/node_modules (gitignored) to (re)build the cache, ` +
        `or run with LK_EMBED=hash. Underlying: ${e.message}`,
      );
    }
    // No surprise network: only an explicit cache (re)build (LK_BUILD_CACHE=1) may fetch the model;
    // otherwise use a locally-present model or fail loud. Keeps the query/eval path from silently
    // reaching out — the offline guarantee is enforced, not just documented.
    transformers.env.allowRemoteModels = process.env.LK_BUILD_CACHE === '1';
    extractor = await transformers.pipeline('feature-extraction', SEM_MODEL);
    return extractor;
  }

  // Fill the cache for every text (async, may load the model). Persists once at the end.
  async function prewarm(texts) {
    const missing = [...new Set(texts)].filter((t) => !cache.vectors[sha(t)]);
    if (missing.length) {
      const ex = await ensureModel();
      for (const t of missing) {
        const out = await ex(t, { pooling: 'mean', normalize: true });
        cache.vectors[sha(t)] = Array.from(out.data, round);
        built++;
      }
    }
    // canonical key order → deterministic file bytes; (re)seal the integrity digest. Persist when
    // anything changed OR the committed file lacks/mismatches its digest (self-heals an unsealed cache).
    const ordered = {};
    for (const k of Object.keys(cache.vectors).sort()) ordered[k] = cache.vectors[k];
    cache.vectors = ordered;
    const digest = computeDigest(ordered);
    if (missing.length || cache.digest !== digest) {
      cache.digest = digest;
      mkdirSync(dirname(cachePath), { recursive: true });
      writeFileSync(cachePath, JSON.stringify(cache) + '\n');
    }
    return { built, cached: texts.length - built };
  }

  // Synchronous cache read — the hot QUERY-TIME path. NO model, NO network, by construction: this
  // function only reads the in-memory cache and never calls ensureModel(). A cache miss is a LOUD throw
  // (never a silent zero) so a stale/tampered/incomplete cache can't produce a false-green ranking.
  function embedSync(text) {
    const v = cache.vectors[sha(text)];
    if (!v) throw new Error(`semantic embedder: text not prewarmed (sha ${sha(text).slice(0, 12)}…). Call prewarm() with every node+query text first, or rebuild the cache (LK_EMBED=semantic LK_BUILD_CACHE=1).`);
    if (v.length !== SEM_DIM) throw new Error(`semantic embedder: cache vector wrong dim (${v.length} != ${SEM_DIM}) — cache corrupt; rebuild.`);
    hits++;
    return Float64Array.from(v);
  }

  const has = (text) => Boolean(cache.vectors[sha(text)]); // membership without embedding (coverage checks)

  return {
    prewarm, embedSync, has, dim: SEM_DIM, model: SEM_MODEL,
    // integrity: the loaded cache's header must match the model/dim this build expects (a mismatched
    // cache is reset to empty on load, so a truthy match here means the committed artifact is the right one).
    integrity: () => ({ model: rawHeader.model, dim: rawHeader.dim, size: Object.keys(cache.vectors).length, matches: rawHeader.model === SEM_MODEL && rawHeader.dim === SEM_DIM, digestOk: digestVerified }),
    stats: () => ({ built, hits, size: Object.keys(cache.vectors).length }),
  };
}
