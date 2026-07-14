// eval-memory-semantic.mjs — FULL semantic⊕bm25⊕title recall over the agent memory store (target 1.0).
//
// Same fusion the proven engine uses (retriever.mjs), now over the memory corpus. Builds the semantic
// cache with LK_BUILD_CACHE=1 — bge-small is already installed (@huggingface/transformers) and its ONNX
// model is already cached on disk, so this is OFFLINE, no npm, no network, no gate. After the build the
// committed cache serves recall offline forever (sovereign at runtime). This is the honest 1.0 path.
import { collectMemoryCorpus, ORACLE } from './eval-memory.mjs';
import { createRetriever } from './lib/retriever.mjs';
import { createSemanticEmbedder } from './lib/embed-semantic.mjs';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const K = 5;
const recall = (got, want) => (want.length ? want.filter((w) => got.includes(w)).length / want.length : 1);

async function main() {
  const files = collectMemoryCorpus();
  const emb = createSemanticEmbedder();
  const R = createRetriever(files.map((f) => ({ rel: f.id, title: f.title, text: f.text })), emb);
  const warm = await R.prewarm(ORACLE.map((o) => o.q)); // embeds all memory chunks + queries (builds cache if LK_BUILD_CACHE=1)
  const integ = emb.integrity();
  const stale = R.coverage().length;

  let rec = 0; const rows = []; const miss = [];
  for (const o of ORACLE) {
    const r = R.search(o.q, K); const rr = recall(r.ids, o.want); rec += rr;
    const rank = r.ranked.findIndex(([id]) => o.want.includes(id)) + 1;
    rows.push({ want: o.want[0], hit: rr === 1, rank, top1: r.ids[0] });
    if (rr < 1) miss.push({ q: o.q, want: o.want[0], rank, got: r.ids.slice(0, 3) });
  }
  const semRecall = Number((rec / ORACLE.length).toFixed(3));
  // determinism: a second search reproduces the ranking
  const d1 = JSON.stringify(R.search(ORACLE[0].q, K).ids), d2 = JSON.stringify(R.search(ORACLE[0].q, K).ids);

  console.log(`\n=== FULL semantic⊕bm25⊕title recall over the AGENT MEMORY STORE (${files.length} files) ===\n`);
  console.log(`  cache: ${warm?.built ? `built ${warm.built} vectors` : 'read from committed cache (offline)'} · model ${integ.matches ? '✓' : '✗'} · digest ${integ.digestOk ? '✓' : '✗'} · coverage ${stale === 0 ? 'offline ✓' : `STALE ${stale} chunks missing`}`);
  console.log(`  recall@${K} = ${semRecall}   (100% goal: ${semRecall >= 0.9999 ? 'MET ✅' : `NOT MET (${semRecall})`})   determinism ${d1 === d2}\n`);
  for (const r of rows) console.log(`  ${r.hit ? '✓' : '✗'} @${r.rank}  → ${r.want}${r.hit ? '' : `  (got ${r.top1})`}`);
  if (miss.length) { console.log(`\n  MISSES:`); for (const m of miss) console.log(`    ✗ "${m.q.slice(0, 52)}" want ${m.want} @rank ${m.rank}  got ${m.got.join(', ')}`); }

  mkdirSync(join(HERE, 'out'), { recursive: true });
  writeFileSync(join(HERE, 'out', 'eval-memory-semantic-results.json'), JSON.stringify({
    corpus: files.length, k: K, oracle: ORACLE.length, semanticRecall: semRecall,
    cacheBuilt: warm?.built ?? 0, cacheOffline: stale === 0, deterministic: d1 === d2,
    model: 'Xenova/bge-small-en-v1.5', modelOk: integ.matches, digestOk: integ.digestOk,
  }, null, 2) + '\n');

  const ok = semRecall >= 0.9999 && integ.matches && integ.digestOk && d1 === d2;
  console.log(`\n  VERDICT: ${ok ? 'GO — whole-system recall@5 = 1.0 (semantic⊕bm25⊕title, offline, deterministic)' : `NO-GO (recall ${semRecall})`}\n`);
  if (!ok) process.exit(1);
}
main();
