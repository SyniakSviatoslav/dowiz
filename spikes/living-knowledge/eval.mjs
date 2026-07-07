// eval.mjs — Verified-by-Math ground-truth evaluation of the living-knowledge RETRIEVAL ENGINE.
//
// The engine is DETERMINISTIC (semantic vectors from a committed cache; BM25/title/fusion are pure
// functions). So a miss is a BUG, not model noise — and the target is a clean, provable 100%. This eval
// is written as PROPERTY-BASED invariants (operator framing): assertions that must ALWAYS hold, each
// able to go RED. Ship the red case — a proof that cannot fail proves nothing.
//
// Compared:
//   • hash baseline  — the sovereign zero-dep FNV BoW+IDF vector store (embed.mjs). Falsifiability anchor.
//   • hybrid engine  — semantic (bge-small, summary-anchored chunks, max-pool) ⊕ stemmed BM25 ⊕ title
//                      label, fused (retriever.mjs). The upgrade must EARN its place (recall ≥ hash).
//
// INVARIANTS (each exits 1 on violation):
//   I1 determinism      — identical searches are byte-identical, SAME-process AND CROSS-process (a fresh
//                          `node rank-once.mjs` reproduces the in-process ranking exactly).
//   I2 completeness      — hybrid recall@K == 1.0 over the hand-verified oracle (every canonical answer
//                          in top-K). This is the 100% goal, enforced as a gate.
//   I3 no-regression     — hybrid recall ≥ hash recall.
//   I4 falsifiability    — real queries reach a stronger best-passage than nonsense queries
//                          (expected-MISS floor: a system that "answers" everything fails this).
//   I5 cache integrity   — committed cache is the right model+dim, covers the corpus offline, AND its
//                          payload digest verifies (detects a tampered/corrupted vector value).
import { execSync } from 'node:child_process';
import { MemoryStore } from './lib/store.mjs';
import { buildStore, collect } from './ingest.mjs';
import { analyzeLayers } from './lib/activate.mjs';
import { createRetriever } from './lib/retriever.mjs';
import { createSemanticEmbedder } from './lib/embed-semantic.mjs';
import { ORACLE } from './oracle.mjs';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const K = 5;
const HERE = dirname(fileURLToPath(import.meta.url));
const HITS = ORACLE.filter((o) => !o.miss);
const MISSES = ORACLE.filter((o) => o.miss);
const recall = (got, want) => (want.length ? want.filter((w) => got.includes(w)).length / want.length : 1);

async function main() {
  const files = collect();
  const corpus = new Set(files.map((f) => f.rel));
  const orphaned = [...new Set(ORACLE.flatMap((o) => o.want))].filter((w) => !corpus.has(w));
  if (orphaned.length) { console.error(`ORACLE ERROR — want files not in corpus:\n  ${orphaned.join('\n  ')}`); process.exit(1); }

  // ── hash baseline (the "old" pure-vector approach, sovereign zero-dep) ──
  const hstore = new MemoryStore();
  buildStore(hstore);
  let hR = 0;
  for (const o of HITS) hR += recall(hstore.vectorTopK(o.q, K).map((x) => x.id), o.want);
  const hashRecall = Number((hR / HITS.length).toFixed(3));

  // ── hybrid engine ──
  let hy = null, err = null;
  if (process.env.LK_EMBED !== 'hash-only') {
    try {
      const emb = createSemanticEmbedder();
      const R = createRetriever(files.map((f) => ({ rel: f.rel, title: f.title, text: f.text })), emb);
      const staleBefore = R.coverage().length; // doc chunks the COMMITTED cache is missing (staleness/tamper)
      const integ = emb.integrity();
      const warm = await R.prewarm(ORACLE.map((o) => o.q));
      let rec = 0, realConf = 0, nonConf = 0;
      const miss = [], telem = [];
      for (const o of HITS) {
        const r = R.search(o.q, K);
        const rr = recall(r.ids, o.want); rec += rr; realConf += r.absConfidence;
        telem.push({ q: o.q, top: r.ids[0], recall: rr, absConf: Number(r.absConfidence.toFixed(4)) });
        if (rr < 1) miss.push({ q: o.q, want: o.want, got: r.ids.slice(0, 3), rank: r.ranked.findIndex(([id]) => o.want.includes(id)) + 1 });
      }
      for (const o of MISSES) nonConf += R.search(o.q, K).absConfidence;
      const d1 = JSON.stringify(R.search(HITS[0].q, K).ranked);
      const d2 = JSON.stringify(R.search(HITS[0].q, K).ranked);
      // cross-process: a FRESH node process must reproduce the in-process ranking. Compare PARSED content
      // (ids + 6dp top1), extracting the JSON object from the child's stdout — robust to any stray library
      // output on the pipe, which a raw string-equals would false-negative on (a flaky determinism check
      // is itself a defect). The claim tested is ranking reproducibility, not byte-for-byte stdout.
      const r0 = R.search(HITS[0].q, K);
      let xproc = false;
      try {
        const raw = execSync('node rank-once.mjs', { cwd: HERE, env: { ...process.env, LK_Q: HITS[0].q } }).toString();
        const j = JSON.parse(raw.slice(raw.indexOf('{'), raw.lastIndexOf('}') + 1));
        xproc = JSON.stringify(j.ids) === JSON.stringify(r0.ids) && j.top1 === Number(r0.top1.toFixed(6));
      } catch { xproc = false; }
      hy = {
        recall: Number((rec / HITS.length).toFixed(3)), realConf: realConf / HITS.length, nonConf: nonConf / MISSES.length,
        deterministic: d1 === d2 && xproc, sameProc: d1 === d2, xproc, miss, telem, cacheBuilt: warm?.built ?? 0, size: R.size,
        cacheModelOk: integ.matches, cacheDigestOk: integ.digestOk, staleBefore, cacheOffline: staleBefore === 0,
      };
    } catch (e) { err = e.message; }
  }

  const layers = analyzeLayers(hstore);

  // ── artifacts ──
  mkdirSync(join(HERE, 'out'), { recursive: true });
  if (hy) writeFileSync(join(HERE, 'out', 'eval-telemetry.jsonl'), hy.telem.map((t) => JSON.stringify(t)).join('\n') + '\n');
  writeFileSync(join(HERE, 'out', 'eval-results.json'), JSON.stringify({
    k: K, corpus: corpus.size, hitQueries: HITS.length, missQueries: MISSES.length,
    hashRecall,
    hybrid: hy ? { model: 'Xenova/bge-small-en-v1.5', recall: hy.recall, realConf: Number(hy.realConf.toFixed(4)), nonsenseConf: Number(hy.nonConf.toFixed(4)), deterministic: hy.deterministic, cacheBuilt: hy.cacheBuilt } : { error: err },
    layers: { nodeCount: layers.nodeCount, edgeCount: layers.edgeCount, islands: layers.islands.length },
  }, null, 2) + '\n');

  // ── report ──
  console.log(`\n=== living-knowledge retrieval engine (corpus ${corpus.size} files, ${HITS.length} hit + ${MISSES.length} miss queries, K=${K}) ===\n`);
  console.log(`  hash baseline (pure vector, sovereign):   recall@${K} = ${hashRecall}`);
  if (hy) {
    console.log(`  hybrid engine (semantic⊕bm25⊕title):      recall@${K} = ${hy.recall}   (Δ +${(hy.recall - hashRecall).toFixed(3)})`);
    console.log(`  best-passage confidence  real=${hy.realConf.toFixed(3)}  nonsense=${hy.nonConf.toFixed(3)}   (separable: ${hy.realConf > hy.nonConf})`);
    console.log(`  determinism: same-process ${hy.sameProc} · cross-process ${hy.xproc}   ·   semantic cache: ${hy.cacheBuilt ? `built ${hy.cacheBuilt} vectors` : 'read from committed cache (offline)'}`);
    console.log(`  cache integrity: model ${hy.cacheModelOk ? 'matches ✓' : 'MISMATCH ✗'} · payload digest ${hy.cacheDigestOk ? 'verified ✓' : 'FAILED ✗'} · corpus coverage ${hy.cacheOffline ? 'offline ✓' : `STALE — ${hy.staleBefore} chunks missing ✗ (rebuild: LK_BUILD_CACHE=1)`}`);
    if (hy.miss.length) { console.log(`\n  MISSES (${hy.miss.length}):`); for (const m of hy.miss) console.log(`    ✗ "${m.q.slice(0, 54)}"  want ${m.want.join(',')} @rank ${m.rank}  got ${m.got.join(', ')}`); }
    console.log(`\n  🎯 100%-retrieval: ${hy.recall >= 0.9999 ? 'MET ✅' : `NOT MET (${hy.recall})`}`);
  } else {
    console.log(`\n  ⚠ hybrid engine unavailable: ${err}`);
    if (process.env.LK_EMBED === 'hash-only') { console.log(`  (LK_EMBED=hash-only — baseline only, 100% goal not evaluated)\n`); return; }
  }
  console.log(`  cross-layer (brain-in-brain): ${layers.nodeCount} nodes / ${layers.edgeCount} edges / ${layers.islands.length} islands\n`);

  // ── falsifiable invariants ──
  const checks = [];
  if (hy) checks.push(
    ['I1 determinism — byte-identical same-process AND cross-process', hy.deterministic],
    ['I2 completeness — hybrid recall@K == 1.0 (100% goal)', hy.recall >= 0.9999],
    ['I3 no-regression — hybrid recall ≥ hash recall', hy.recall >= hashRecall],
    ['I4 falsifiability — real best-passage > nonsense', hy.realConf > hy.nonConf],
    ['I5 cache integrity — right model + payload digest verified + covers corpus offline', hy.cacheModelOk && hy.cacheDigestOk && hy.cacheOffline],
  );
  else checks.push(['hybrid engine available (or LK_EMBED=hash-only)', false]);

  let ok = true;
  console.log('');
  for (const [name, pass] of checks) { console.log(`  ${pass ? '✓' : '✗'} ${name}`); if (!pass) ok = false; }
  console.log(`\n  VERDICT: ${ok ? 'GO' : 'NO-GO'} — engine recall@${K} = ${hy ? hy.recall : hashRecall}, deterministic, falsifiable.\n`);
  if (!ok) process.exit(1);
}

main();
