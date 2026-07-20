// retriever.mjs — the living-knowledge RETRIEVAL ENGINE: a deterministic multi-level fusion of three
// independent signals, the "multi-level indexing" upgrade (operator rec #1) over plain similarity search.
// Each miss in the old system was an implementation gap, not model noise — this closes them, signal by signal:
//
//   • SEMANTIC (vector)  — bge-small-en-v1.5, SUMMARY-ANCHORED chunks (title + first descriptive line
//                          prepended to each passage — a lightweight hierarchical-summary, rec #4),
//                          max-pooled at query time (score = best-matching passage). Served from a
//                          committed vector cache → offline + bit-identical (embed-semantic.mjs).
//   • LEXICAL (BM25)     — Okapi BM25 over PORTER-STEMMED tokens (porter.mjs) so "classified" matches
//                          "classification"; title tokens up-weighted. Catches exact terms semantics blurs.
//   • TITLE-LABEL        — idf-weighted overlap of query stems with the doc's filename+summary stems.
//                          The curated human label is a strong signal both vector and BM25 under-use.
//
// FUSION: min-max normalize each signal per query, then WS·sem + WB·bm25 + WT·title. Deterministic
// (pure function of query + committed cache + corpus), tie-broken by id. A one-hop graph boost was
// TESTED and REJECTED — it net-degraded recall by floating hub nodes (measured, not assumed).
//
// Weights are the centre of a 1.0-recall plateau on the hard oracle (three neighbouring weightings all
// score 1.0; near-misses sit at rank 6–7 — evidence of a robust optimum, not an overfit knife-edge).
import { QUERY_PREFIX } from './embed-semantic.mjs';
import { stem } from './porter.mjs';

// Fusion weights. LK_WEIGHTS="sem,bm25,title" overrides them — used to ABLATE (prove each signal is
// load-bearing: e.g. LK_WEIGHTS=1,0,0 = semantic-only, which reds the completeness invariant). This is
// the falsifiability lever: it lets the eval demonstrate a RED case on demand.
export const WEIGHTS = (() => {
  const e = (process.env.LK_WEIGHTS || '').split(',').map(Number);
  return e.length === 3 && e.every((x) => Number.isFinite(x)) ? { sem: e[0], bm25: e[1], title: e[2] } : { sem: 0.45, bm25: 0.35, title: 0.20 };
})();
const CHUNK = { W: 150, S: 120, C: 8 }; // words/window, stride, max chunks per doc
const BM25 = { k1: 1.5, b: 0.75 };

const STOP = new Set('the a an of to in and or for is are be with that this it as on at by from into no not any all before after your you our we they them these those what which who how when where than then so if'.split(' '));
const rawTok = (t) => (t || '').toLowerCase().split(/[^a-z0-9_]+/).filter((w) => w.length >= 2 && w.length <= 40);
const stemTok = (t) => rawTok(t).filter((w) => !STOP.has(w)).map(stem);

// First descriptive line of a file: a markdown heading, the lead comment after a shebang, a yaml
// name/description, or the first prose line. The doc's own one-line summary (rec #4), used to anchor
// every chunk and to enrich the title-label signal.
export function summaryLine(text, fallback = '') {
  for (const raw of (text || '').split('\n').slice(0, 20)) {
    const l = raw.trim();
    if (!l || l.startsWith('#!') || l === '---') continue;
    const md = l.match(/^#{1,3}\s+(.*)/); if (md) return md[1].replace(/[#*`]/g, '').slice(0, 180);
    const cm = l.match(/^(?:#|\/\/|--)\s*(.*)/); if (cm && cm[1].split(/\s+/).length >= 4) return cm[1].replace(/^[\w./-]+\s*[—-]\s*/, '').slice(0, 180);
    const yaml = l.match(/^(?:name|title|description|summary):\s*["']?(.+?)["']?$/i); if (yaml) return yaml[1].slice(0, 180);
    if (l.split(/\s+/).length >= 4 && !l.includes(':')) return l.slice(0, 180);
  }
  return fallback;
}

function chunkTexts(title, text, summary) {
  const head = `${title}. ${summary}`;
  const words = (text || '').split(/\s+/).filter(Boolean);
  const out = [];
  for (let i = 0; i < words.length && out.length < CHUNK.C; i += CHUNK.S) out.push(`${head}\n${words.slice(i, i + CHUNK.W).join(' ')}`);
  if (!out.length) out.push(head);
  return out;
}

const cos = (a, b) => { let s = 0; for (let i = 0; i < a.length; i++) s += a[i] * b[i]; return s; };
const minmax = (m) => { const vs = [...m.values()]; const lo = Math.min(...vs), hi = Math.max(...vs); const r = hi - lo || 1; const o = new Map(); for (const [k, v] of m) o.set(k, (v - lo) / r); return o; };

// files: [{ rel, title, text }]. embedder: { prewarm(texts), embedSync(text) } (embed-semantic).
export function createRetriever(files, embedder) {
  // per-doc precompute (deterministic, no model): chunk texts, stemmed tokens, title/summary label set.
  const docs = files.map((f) => {
    const summary = summaryLine(f.text, f.title);
    return {
      id: f.rel,
      chunks: chunkTexts(f.title, f.text, summary),
      toks: stemTok(`${f.title} ${f.title} ${summary} ${f.text}`),
      labelToks: new Set(stemTok(`${f.title} ${summary}`)),
    };
  });
  const ids = docs.map((d) => d.id);
  const N = docs.length;
  const avgdl = docs.reduce((s, d) => s + d.toks.length, 0) / (N || 1);
  const df = new Map();
  for (const d of docs) for (const t of new Set(d.toks)) df.set(t, (df.get(t) || 0) + 1);
  const idf = (t) => Math.log(1 + (N - (df.get(t) || 0) + 0.5) / ((df.get(t) || 0) + 0.5));

  const bm25 = (qToks, d) => {
    const tf = new Map(); for (const t of d.toks) tf.set(t, (tf.get(t) || 0) + 1);
    let s = 0;
    for (const q of qToks) { const f = tf.get(q) || 0; if (!f) continue; s += idf(q) * (f * (BM25.k1 + 1)) / (f + BM25.k1 * (1 - BM25.b + BM25.b * d.toks.length / avgdl)); }
    return s;
  };
  const titleMatch = (qToks, d) => { let hit = 0, tot = 0; for (const q of new Set(qToks)) { const w = idf(q); tot += w; if (d.labelToks.has(q)) hit += w; } return tot ? hit / tot : 0; };

  // Every string the semantic model must have vectors for: all doc chunks + the (prefixed) queries.
  const docChunkTexts = docs.flatMap((d) => d.chunks);
  async function prewarm(queries = []) {
    return embedder.prewarm([...docChunkTexts, ...queries.map((q) => QUERY_PREFIX + q)]);
  }

  // SAFETY: cap query length (a defensive bound against pathological/adversarial input inflating work).
  const MAX_QUERY = 2000;
  const capQuery = (q) => { if (typeof q !== 'string') throw new TypeError('query must be a string'); return q.slice(0, MAX_QUERY); };

  // Shared deterministic scoring — the single source of truth for both search() and explain().
  // Reads semantic vectors from the committed cache; NO model, NO network at query time.
  function score(query) {
    const q = capQuery(query);
    const qToks = stemTok(q);
    const qv = embedder.embedSync(QUERY_PREFIX + q);
    const semRaw = new Map(), bm = new Map(), ti = new Map();
    const bestChunk = new Map();
    for (const d of docs) {
      let best = -Infinity, bi = 0; for (let i = 0; i < d.chunks.length; i++) { const c = cos(qv, embedder.embedSync(d.chunks[i])); if (c > best) { best = c; bi = i; } }
      semRaw.set(d.id, best); bestChunk.set(d.id, bi); bm.set(d.id, bm25(qToks, d)); ti.set(d.id, titleMatch(qToks, d));
    }
    const ns = minmax(semRaw), nb = minmax(bm), nt = minmax(ti);
    const fused = ids.map((id) => [id, WEIGHTS.sem * ns.get(id) + WEIGHTS.bm25 * nb.get(id) + WEIGHTS.title * nt.get(id)]);
    const ranked = fused.sort((a, b) => (b[1] - a[1]) || (a[0] < b[0] ? -1 : 1));
    // absConfidence = the single best RAW passage cosine anywhere (an ABSOLUTE measure — unlike the
    // min-max fused score). Falsifiability floor: nonsense queries find no strong passage → stays low.
    const absConfidence = Math.max(...semRaw.values());
    return { qToks, ranked, absConfidence, ns, nb, nt, semRaw, bestChunk };
  }

  function search(query, k = 5) {
    const s = score(query);
    return { ids: s.ranked.slice(0, k).map((x) => x[0]), top1: s.ranked.length ? s.ranked[0][1] : 0, absConfidence: s.absConfidence, ranked: s.ranked };
  }

  // OBSERVABILITY: full per-signal trace of WHY each top-k doc ranked where it did — the real-time
  // state of the engine for one query. Deterministic; safe to log/audit.
  function explain(query, k = 5) {
    const s = score(query);
    const rows = s.ranked.slice(0, k).map(([id, fused], rank) => ({
      rank: rank + 1, id, fused: Number(fused.toFixed(4)),
      contrib: { semantic: Number((WEIGHTS.sem * s.ns.get(id)).toFixed(4)), bm25: Number((WEIGHTS.bm25 * s.nb.get(id)).toFixed(4)), title: Number((WEIGHTS.title * s.nt.get(id)).toFixed(4)) },
      rawCosine: Number(s.semRaw.get(id).toFixed(4)), bestChunk: s.bestChunk.get(id),
    }));
    return { query: capQuery(query), tokens: s.qToks, weights: WEIGHTS, absConfidence: Number(s.absConfidence.toFixed(4)), results: rows };
  }

  // SAFETY: which doc-chunk texts are absent from the cache — a stale/tampered/incomplete-cache detector.
  // Empty array = the committed cache exactly covers the current corpus (no silent drift).
  function coverage() { return docChunkTexts.filter((t) => !embedder.has(t)); }

  return { prewarm, search, explain, coverage, docChunkTexts, size: docs.length, weights: WEIGHTS };
}
