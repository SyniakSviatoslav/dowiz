// eval.mjs — Verified-by-Math ground-truth evaluation of the living-knowledge activation layer.
//
// Proves (with numbers, against a hand-derived oracle, incl. expected-MISS queries so a spurious
// 100% is impossible):
//   1. WORKS   — activation retrieval finds the oracle files.
//   2. MATH    — precision/recall@k vs a pure-vector baseline; determinism = bit-identical reruns.
//   3. FALSIFIABLE — the eval ASSERTS (exit 1) that (a) reruns are identical, (b) activation recall ≥
//                    baseline recall, (c) real queries score higher than nonsense queries. Any of
//                    these can go RED — this is not a vanity-green report.
//
// Baseline = pure vector top-k (vectorTopK) — the "semantic search" a plain embedding store gives.
// Activation = spreading activation over structure+why bands (seeds + referenced neighbours), which
// should LIFT recall by reaching answer files that are referenced-by (not lexically similar to) the
// query. Backend-agnostic (runs identically on MemoryStore or the HelixStore mirror).
import { MemoryStore } from './lib/store.mjs';
import { buildStore } from './ingest.mjs';
import { activate, trace, analyzeLayers, BANDS } from './lib/activate.mjs';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const K = 5;
const HERE = dirname(fileURLToPath(import.meta.url));

// Hand-derived oracle. `want` files MUST be in the ingested corpus. `miss:true` = no answer exists.
const ORACLE = [
  { q: 'which bash hook blocks mutations of protected paths like migrations and env', want: ['.claude/hooks/guard-bash.sh'] },
  { q: 'checker for 0-tool-use degenerate subagent return echo of injected context', want: ['.claude/hooks/subagent-return-guard.sh'] },
  { q: 'verified by math falsifiable proof rule no false positive metrics', want: ['docs/operating-model/verified-by-math.md', 'scripts/guardrail-falsifiable-proof.mjs'] },
  { q: 'spreading activation bands helixdb living knowledge store', want: ['docs/operating-model/living-knowledge-helixdb-arc.md'] },
  { q: 'certified loop must have its report artifact parity registry', want: ['scripts/guardrail-loop-registry-parity.mjs', 'loops/registry.md'] },
  { q: 'knowledge as circuits red-line pattern registry runner', want: ['scripts/run-circuits.mjs', 'docs/operating-model/circuits/registry.json'] },
  { q: 'fable off model routing agent dispatch gate deny', want: ['.claude/hooks/agent-dispatch-gate.sh'] },
  { q: 'run all governance armaments before commit suite', want: ['scripts/run-armaments.sh'] },
  // expected MISS — no such thing in the harness corpus (falsifiability floor).
  { q: 'kubernetes helm chart ingress controller autoscaling', want: [], miss: true },
  { q: 'react usestate css keyframes flexbox animation component', want: [], miss: true },
];

// activation retrieval: union of structure+why bands, ranked by max activation, top-k.
function activationTopK(store, query, k) {
  const bands = ['code-structure', 'why'];
  const best = new Map();
  let top1 = 0;
  for (const b of bands) for (const { id, a } of activate(store, query, b).ranked) {
    best.set(id, Math.max(best.get(id) || 0, a));
  }
  const ranked = [...best.entries()].sort((x, y) => (y[1] - x[1]) || (x[0] < y[0] ? -1 : 1));
  top1 = ranked.length ? ranked[0][1] : 0;
  return { ids: ranked.slice(0, k).map(([id]) => id), top1 };
}

const recall = (got, want) => (want.length ? want.filter((w) => got.includes(w)).length / want.length : 1);
const precision = (got, want) => (got.length ? got.filter((w) => want.includes(w)).length / Math.min(got.length, K) : 0);

function main() {
  const store = new MemoryStore();
  const info = buildStore(store);

  let baseR = 0, actR = 0, baseP = 0, actP = 0, nHit = 0;
  let hitTop1 = 0, missTop1 = 0, nMiss = 0;
  const rows = [];
  for (const t of ORACLE) {
    const base = store.vectorTopK(t.q, K).map((x) => x.id);
    const act = activationTopK(store, t.q, K);
    if (t.miss) { missTop1 += act.top1; nMiss++; rows.push({ q: t.q, miss: true, baseTop: base[0], actTop1: Number(act.top1.toFixed(3)) }); continue; }
    nHit++;
    const rb = recall(base, t.want), ra = recall(act.ids, t.want);
    baseR += rb; actR += ra; baseP += precision(base, t.want); actP += precision(act.ids, t.want); hitTop1 += act.top1;
    rows.push({ q: t.q, want: t.want, baseRecall: rb, actRecall: ra, actIds: act.ids.slice(0, 3) });
  }
  const R = (x) => Number((x / nHit).toFixed(3));
  const baseRecall = R(baseR), actRecall = R(actR), basePrec = R(baseP), actPrec = R(actP);
  const hitMeanTop1 = hitTop1 / nHit, missMeanTop1 = missTop1 / nMiss;

  // determinism: two independent activation passes must be byte-identical.
  const d1 = JSON.stringify(trace(store, ORACLE[0].q, Object.keys(BANDS)).perBand);
  const d2 = JSON.stringify(trace(store, ORACLE[0].q, Object.keys(BANDS)).perBand);
  const deterministic = d1 === d2;

  const layers = analyzeLayers(store);

  // telemetry snapshot for the probe/report.
  const telem = trace(store, ORACLE[2].q).telems;
  mkdirSync(join(HERE, 'out'), { recursive: true });
  writeFileSync(join(HERE, 'out', 'eval-telemetry.jsonl'), telem.map((t) => JSON.stringify(t)).join('\n') + '\n');
  writeFileSync(join(HERE, 'out', 'eval-results.json'), JSON.stringify({
    corpus: info, k: K, baseRecall, actRecall, basePrec, actPrec, hitMeanTop1: Number(hitMeanTop1.toFixed(3)),
    missMeanTop1: Number(missMeanTop1.toFixed(3)), deterministic, layers: { nodeCount: layers.nodeCount, edgeCount: layers.edgeCount, islands: layers.islands.length, disconnectedLayerPairs: layers.disconnectedLayerPairs },
  }, null, 2) + '\n');

  // ── report ──
  console.log(`\n=== living-knowledge eval (corpus: ${info.files} files, ${info.edges} edges, K=${K}) ===`);
  for (const r of rows) {
    if (r.miss) console.log(`  MISS  "${r.q.slice(0, 44)}"  activation top-1=${r.actTop1}`);
    else console.log(`  hit   recall base=${r.baseRecall.toFixed(2)} act=${r.actRecall.toFixed(2)}  "${r.q.slice(0, 40)}"  → ${r.actIds.join(', ')}`);
  }
  console.log(`\n  recall@${K}:    baseline=${baseRecall}  activation=${actRecall}   (Δ ${(actRecall - baseRecall).toFixed(3)})`);
  console.log(`  precision@${K}: baseline=${basePrec}  activation=${actPrec}`);
  console.log(`  confidence (mean top-1 activation): real-query=${hitMeanTop1.toFixed(3)}  nonsense-query=${missMeanTop1.toFixed(3)}`);
  console.log(`  determinism (identical reruns): ${deterministic}`);
  console.log(`\n  cross-layer analysis (brain-in-brain): ${layers.nodeCount} nodes / ${layers.edgeCount} edges`);
  console.log(`    islands (no cross-layer edge): ${layers.islands.length}`);
  console.log(`    disconnected layer pairs (zero connecting edges): ${layers.disconnectedLayerPairs.map((p) => p.join('↔')).join(', ') || 'none'}`);

  // ── falsifiable GO/NO-GO assertions (each CAN go red) ──
  const checks = [
    ['reruns are bit-identical (determinism)', deterministic],
    ['activation recall ≥ baseline recall', actRecall >= baseRecall],
    ['real queries score higher than nonsense queries', hitMeanTop1 > missMeanTop1],
    ['activation recall is non-trivial (≥ 0.5)', actRecall >= 0.5],
  ];
  let ok = true;
  console.log('');
  for (const [name, pass] of checks) { console.log(`  ${pass ? '✓' : '✗'} ${name}`); if (!pass) ok = false; }
  console.log(`\n  VERDICT: ${ok ? 'GO' : 'NO-GO'} — activation ${actRecall >= baseRecall ? 'matches/beats' : 'is worse than'} pure-vector baseline, deterministic, falsifiable.\n`);
  if (!ok) process.exit(1);
}

main();
