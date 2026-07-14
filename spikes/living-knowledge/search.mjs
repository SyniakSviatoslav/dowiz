// search.mjs — OBSERVABILITY CLI: see the engine's real-time state for one query. Prints the ranked
// results and, for each, the per-signal contribution (semantic / BM25 / title) that produced the rank —
// so a human can audit WHY a document surfaced, not just THAT it did.
//
//   node search.mjs "which hook blocks writes to protected paths"        # top-5, full signal trace
//   node search.mjs -k 8 "loop that bisects git history to one commit"   # top-8
//
// Novel queries (not in the committed cache) are embedded on demand → needs @huggingface/transformers
// installed. Corpus chunk vectors come from the committed cache (offline).
import { collect } from './ingest.mjs';
import { createRetriever } from './lib/retriever.mjs';
import { createSemanticEmbedder } from './lib/embed-semantic.mjs';

const argv = process.argv.slice(2);
let k = 5; const parts = [];
for (let i = 0; i < argv.length; i++) { if (argv[i] === '-k') k = Number(argv[++i]) || 5; else parts.push(argv[i]); }
const query = parts.join(' ').trim();
if (!query) { console.error('usage: node search.mjs [-k N] "<query>"'); process.exit(2); }

const files = collect();
const emb = createSemanticEmbedder();
const R = createRetriever(files.map((f) => ({ rel: f.rel, title: f.title, text: f.text })), emb);

const stale = R.coverage().length;
if (stale) console.error(`⚠ committed cache is stale (${stale} corpus chunks missing) — rebuild: LK_BUILD_CACHE=1 node eval.mjs`);
try { await R.prewarm([query]); } // embeds the (novel) query; corpus chunks already cached
catch (e) { console.error(`✗ ${e.message}`); process.exit(1); }

const ex = R.explain(query, k);
const bar = (x) => '█'.repeat(Math.round(x / 0.02)); // 1 block ≈ 0.02
console.log(`\nquery      "${ex.query}"`);
console.log(`tokens     ${ex.tokens.join(' ')}`);
console.log(`weights    sem ${ex.weights.sem} · bm25 ${ex.weights.bm25} · title ${ex.weights.title}`);
console.log(`confidence ${ex.absConfidence} (best raw passage cosine; higher = more on-topic)\n`);
console.log(` #  fused   │ sem    bm25   title │ file`);
for (const r of ex.results) {
  const c = r.contrib;
  console.log(` ${String(r.rank).padStart(2)} ${r.fused.toFixed(3)}  │ ${c.semantic.toFixed(2)}   ${c.bm25.toFixed(2)}   ${c.title.toFixed(2)}  │ ${r.id}`);
  console.log(`    ${' '.repeat(6)} │ ${bar(c.semantic + c.bm25 + c.title)}`);
}
console.log('');
