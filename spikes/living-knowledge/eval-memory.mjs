// eval-memory.mjs — un-strand the living-knowledge recall engine over the AGENT MEMORY STORE.
//
// The proven recall engine (recall@5=1.0 on the harness corpus) never ingested the 179-file agent
// memory store, docs/design, or docs/adr — so "recall a past decision/plan/lesson by concept" was
// impossible (memory BREAK D/E/G). This wires the memory corpus into the SOVEREIGN, OFFLINE recall
// path: the engine's own LEXICAL fusion — Okapi BM25 over Porter-stemmed tokens ⊕ title-label overlap
// (lib/porter.mjs) — plus the FNV hash floor for comparison. NO model, NO network, deterministic.
// The full engine adds a third SEMANTIC signal (bge-small) that lifts recall further but needs a cache
// rebuild (LK_BUILD_CACHE=1, network) — an honestly-flagged follow-up. This file proves the WIRE + the
// sovereign lexical recall achievable TODAY, with a red case: the fusion must BEAT the hash floor.
//
// Invariants (each exits 1 on violation — a proof that cannot fail proves nothing):
//   I1 corpus-inclusion — the 179-file memory store IS in the corpus (RED without the collector).
//   I2 determinism      — identical searches are byte-identical (pure lexical scoring).
//   I3 falsifiability   — real queries reach a stronger BM25 passage than nonsense (expected-MISS floor).
//   I4 no-regression    — lexical fusion recall@5 > hash-floor recall@5 (the fusion earns its place).
//   I5 usable recall    — lexical recall@5 ≥ 0.6 over the hand-verified oracle.
import { readdirSync, readFileSync, statSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, basename, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { MemoryStore } from './lib/store.mjs';
import { stem } from './lib/porter.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = '/root/dowiz';
const MEM = '/root/.claude/projects/-root-dowiz/memory';
const ETEXT_SLICE = 8000;
const K = 5;

// ── collector: memory store (semantic + episodic) + design docs + ADR decision spine ──
function walkMd(absDir, idPrefix, recurse = true) {
  const out = [];
  let entries = [];
  try { entries = readdirSync(absDir, { withFileTypes: true }); } catch { return out; }
  for (const e of entries) {
    const abs = join(absDir, e.name);
    if (e.isDirectory()) { if (recurse) out.push(...walkMd(abs, `${idPrefix}/${e.name}`, true)); continue; }
    if (!e.name.endsWith('.md')) continue;
    let text = '';
    try { text = readFileSync(abs, 'utf8'); statSync(abs); } catch { continue; }
    out.push({ id: `${idPrefix}/${e.name}`, title: basename(e.name), text: text.slice(0, ETEXT_SLICE), layer: idPrefix.split('/')[0] });
  }
  return out;
}
export function collectMemoryCorpus() {
  const files = [
    ...walkMd(MEM, 'memory', false),
    ...walkMd(join(REPO, 'docs/design'), 'docs/design', true),
    ...walkMd(join(REPO, 'docs/adr'), 'docs/adr', false),
  ];
  const seen = new Set();
  return files.filter((f) => (seen.has(f.id) ? false : (seen.add(f.id), true)));
}

// ── lexical engine (engine's own BM25 ⊕ title signals, reused offline; retriever.mjs:33-89) ──
const STOP = new Set('the a an of to in and or for is are be with that this it as on at by from into no not any all before after your you our we they them these those what which who how when where than then so if'.split(' '));
const rawTok = (t) => (t || '').toLowerCase().split(/[^a-z0-9_]+/).filter((w) => w.length >= 2 && w.length <= 40);
const stemTok = (t) => rawTok(t).filter((w) => !STOP.has(w)).map(stem);
function summaryLine(text, fallback = '') {
  for (const raw of (text || '').split('\n').slice(0, 20)) {
    const l = raw.trim();
    if (!l || l.startsWith('#!') || l === '---') continue;
    const md = l.match(/^#{1,3}\s+(.*)/); if (md) return md[1].replace(/[#*`]/g, '').slice(0, 180);
    const yaml = l.match(/^(?:name|title|description|summary):\s*["']?(.+?)["']?$/i); if (yaml) return yaml[1].slice(0, 180);
    if (l.split(/\s+/).length >= 4 && !l.includes(':')) return l.slice(0, 180);
  }
  return fallback;
}
const BM25P = { k1: 1.5, b: 0.75 };
const minmax = (m) => { const vs = [...m.values()]; const lo = Math.min(...vs), hi = Math.max(...vs); const r = hi - lo || 1; const o = new Map(); for (const [k, v] of m) o.set(k, (v - lo) / r); return o; };

export function buildLexical(corpus) {
  const docs = corpus.map((f) => {
    const summary = summaryLine(f.text, f.title);
    return { id: f.id, toks: stemTok(`${f.title} ${f.title} ${summary} ${f.text}`), labelToks: new Set(stemTok(`${f.title} ${summary}`)) };
  });
  const N = docs.length;
  const avgdl = docs.reduce((s, d) => s + d.toks.length, 0) / (N || 1);
  const df = new Map();
  for (const d of docs) for (const t of new Set(d.toks)) df.set(t, (df.get(t) || 0) + 1);
  const idf = (t) => Math.log(1 + (N - (df.get(t) || 0) + 0.5) / ((df.get(t) || 0) + 0.5));
  const tf = new Map(docs.map((d) => { const m = new Map(); for (const t of d.toks) m.set(t, (m.get(t) || 0) + 1); return [d.id, m]; }));
  const bm25 = (qToks, d) => { let s = 0; const f = tf.get(d.id); for (const q of qToks) { const c = f.get(q) || 0; if (!c) continue; s += idf(q) * (c * (BM25P.k1 + 1)) / (c + BM25P.k1 * (1 - BM25P.b + BM25P.b * d.toks.length / avgdl)); } return s; };
  const titleMatch = (qToks, d) => { let hit = 0, tot = 0; for (const q of new Set(qToks)) { const w = idf(q); tot += w; if (d.labelToks.has(q)) hit += w; } return tot ? hit / tot : 0; };
  function rank(query, k = K) {
    const qToks = stemTok(query);
    const bm = new Map(), ti = new Map();
    for (const d of docs) { bm.set(d.id, bm25(qToks, d)); ti.set(d.id, titleMatch(qToks, d)); }
    const nb = minmax(bm), nt = minmax(ti);
    const fused = docs.map((d) => [d.id, 0.7 * nb.get(d.id) + 0.3 * nt.get(d.id)]);
    const ranked = fused.sort((a, b) => (b[1] - a[1]) || (a[0] < b[0] ? -1 : 1));
    return { ids: ranked.slice(0, k).map((x) => x[0]), ranked, bmTop: Math.max(0, ...[...bm.values()]) };
  }
  return { rank, size: N };
}

// ── wikilink/markdown-link graph over the corpus (deterministic, checkable edges) ──
export function buildGraph(corpus) {
  const ids = new Set(corpus.map((f) => f.id));
  const byBase = new Map(); // basename.md -> [ids]
  for (const f of corpus) { const b = f.id.split('/').pop(); if (!byBase.has(b)) byBase.set(b, []); byBase.get(b).push(f.id); }
  const resolveWiki = (slug) => (ids.has(`memory/${slug}.md`) ? `memory/${slug}.md` : (byBase.get(`${slug}.md`)?.length === 1 ? byBase.get(`${slug}.md`)[0] : null));
  const resolveMd = (target) => { const b = target.split('/').pop(); const c = byBase.get(b); return c && c.length === 1 ? c[0] : (ids.has(target) ? target : null); };
  const adj = new Map(corpus.map((f) => [f.id, new Set()]));
  const add = (a, b) => { if (a && b && a !== b && adj.has(a) && adj.has(b)) { adj.get(a).add(b); adj.get(b).add(a); } };
  for (const f of corpus) {
    for (const m of f.text.matchAll(/\[\[([a-z0-9][a-z0-9-]*)\]\]/gi)) add(f.id, resolveWiki(m[1]));
    for (const m of f.text.matchAll(/\]\(([^)]+?\.md)\)/gi)) add(f.id, resolveMd(m[1]));
  }
  const deg = new Map([...adj].map(([id, s]) => [id, s.size]));
  const edges = [...adj.values()].reduce((s, set) => s + set.size, 0) / 2;
  return { adj, deg, edges };
}

// seed(BM25⊕title) → 1-hop degree-normalized spreading activation (activate.mjs math, retain=1.0).
function diffuse(seedScore, graph, decay = 0.35) {
  const out = new Map(seedScore); // retain 1.0: keep baseline, add neighbour lift
  for (const [id, base] of seedScore) {
    if (base <= 0) continue;
    const dn = graph.deg.get(id) || 1;
    for (const nb of (graph.adj.get(id) || [])) {
      const dm = graph.deg.get(nb) || 1;
      out.set(nb, (out.get(nb) || 0) + (base * decay) / Math.sqrt(dn * dm));
    }
  }
  return out;
}

// ── hand-verified oracle: natural-language operator questions (paraphrased, NOT copied tokens) ──
export const ORACLE = [
  { q: 'the consolidated plan to store specs plans docs and memory as one graph and vector index', want: ['memory/knowledge-spine-arc-2026-07-14.md'] },
  { q: 'how the agent may modify its own harness machinery without lowering the safety floor', want: ['memory/governance-gate-topology-2026-07-14.md'] },
  { q: 'spectral graph analysis of the order lifecycle state machine cyclomatic complexity and cycles', want: ['memory/fsm-graph-analysis.md'] },
  { q: 'a change is only finished after commit staging deploy and end to end proof', want: ['memory/ship-discipline-rule.md'] },
  { q: 'test account login credentials and the demo location identifier', want: ['memory/test-owner-fixture-sushi-demo.md'] },
  { q: 'which model to pick for cheap work versus expensive money and authentication changes', want: ['memory/model-routing-policy-2026-07-03.md'] },
  { q: 'detecting when the agent is trapped orbiting without making progress a limit cycle', want: ['memory/markov-attractor-loop-signal-2026-07-13.md'] },
  { q: 'why the background job queue lives inside postgres rather than a dedicated message broker', want: ['docs/adr/0001-queue-in-postgres.md'] },
  { q: 'where the delivery fee amount is authoritatively decided as the single source of truth', want: ['docs/adr/0005-delivery-fee-source-of-truth.md'] },
  { q: 'blueprint for spec driven development with graph and vector retrieval over all documentation', want: ['docs/design/KNOWLEDGE-SPINE-BLUEPRINT-2026-07-14.md'] },
];
const MISS = [
  { q: 'kubernetes helm chart ingress controller autoscaling', want: [], miss: true },
  { q: 'react usestate flexbox css keyframes animation component', want: [], miss: true },
];
const recallAtK = (got, want) => (want.length ? want.filter((w) => got.includes(w)).length / want.length : 1);

function main() {
  const files = collectMemoryCorpus();
  const ids = new Set(files.map((f) => f.id));
  const orphaned = [...new Set(ORACLE.flatMap((o) => o.want))].filter((w) => !ids.has(w));
  if (orphaned.length) { console.error(`ORACLE ERROR — want not in corpus:\n  ${orphaned.join('\n  ')}`); process.exit(1); }
  const memCount = files.filter((f) => f.id.startsWith('memory/')).length;

  // hash floor (FNV BoW+IDF)
  const hstore = new MemoryStore();
  for (const f of files) hstore.addNode({ id: f.id, label: f.layer, title: f.title, text: f.text });
  hstore.finalize();
  let hRec = 0;
  for (const o of ORACLE) hRec += recallAtK(hstore.vectorTopK(o.q, K).map((x) => x.id), o.want);
  const hashRecall = Number((hRec / ORACLE.length).toFixed(3));

  // lexical fusion (BM25 ⊕ title) — the sovereign offline engine
  const lex = buildLexical(files);
  let lRec = 0; const rows = [];
  for (const o of ORACLE) {
    const r = lex.rank(o.q, K); const rr = recallAtK(r.ids, o.want); lRec += rr;
    const rank = lex.rank(o.q, 999).ranked.findIndex(([id]) => o.want.includes(id)) + 1;
    rows.push({ q: o.q.slice(0, 46), want: o.want[0], hit: rr === 1, rank, top1: r.ids[0] });
  }
  const lexRecall = Number((lRec / ORACLE.length).toFixed(3));

  // diffusion: BM25⊕title seed → 1-hop degree-normalized spreading activation over the wikilink graph
  const graph = buildGraph(files);
  let dRec = 0; const drows = [];
  for (const o of ORACLE) {
    const seed = new Map(lex.rank(o.q, 999999).ranked);
    const diff = [...diffuse(seed, graph).entries()].sort((a, b) => (b[1] - a[1]) || (a[0] < b[0] ? -1 : 1));
    const top = diff.slice(0, K).map((x) => x[0]); const rr = recallAtK(top, o.want); dRec += rr;
    drows.push({ want: o.want[0], hit: rr === 1, rank: diff.findIndex(([id]) => o.want.includes(id)) + 1, top1: top[0] });
  }
  const diffRecall = Number((dRec / ORACLE.length).toFixed(3));

  // falsifiability: raw BM25 top score, real vs nonsense
  const realBM = ORACLE.reduce((s, o) => s + lex.rank(o.q, 1).bmTop, 0) / ORACLE.length;
  const nonBM = MISS.reduce((s, o) => s + lex.rank(o.q, 1).bmTop, 0) / MISS.length;
  // determinism
  const d1 = JSON.stringify(lex.rank(ORACLE[0].q, K).ids), d2 = JSON.stringify(lex.rank(ORACLE[0].q, K).ids);

  const checks = [
    [`I1 corpus-inclusion — memory store ingested (${memCount} files ≥ 150)`, memCount >= 150],
    ['I2 determinism — identical search byte-identical', d1 === d2],
    [`I3 falsifiability — real BM25 top (${realBM.toFixed(2)}) > nonsense (${nonBM.toFixed(2)})`, realBM > nonBM],
    [`I4 no-regression — lexical recall@5 (${lexRecall}) > hash floor (${hashRecall})`, lexRecall > hashRecall],
    [`I5 usable recall — lexical recall@5 (${lexRecall}) ≥ 0.6`, lexRecall >= 0.6],
  ];

  console.log(`\n=== living-knowledge recall OVER THE AGENT MEMORY STORE (sovereign, offline, zero-dep) ===\n`);
  console.log(`  corpus: ${files.length} files  (memory ${memCount} · design ${files.filter(f=>f.id.startsWith('docs/design/')).length} · adr ${files.filter(f=>f.id.startsWith('docs/adr/')).length})`);
  console.log(`  wikilink graph: ${graph.edges} edges over ${files.length} nodes`);
  console.log(`  recall@${K}:  hash floor ${hashRecall}  →  lexical BM25⊕title ${lexRecall}  →  +diffusion ${diffRecall}`);
  console.log(`  falsifiability: real BM25 ${realBM.toFixed(2)} vs nonsense ${nonBM.toFixed(2)}  (separable ${realBM > nonBM})`);
  console.log(`  (semantic bge-small third signal → recall↑, needs cache rebuild: LK_BUILD_CACHE=1)\n`);
  for (let i = 0; i < rows.length; i++) { const r = rows[i], d = drows[i]; console.log(`  lex ${r.hit ? '✓' : '✗'}@${r.rank}  diff ${d.hit ? '✓' : '✗'}@${d.rank}  → ${r.want}${d.hit ? '' : `  (got ${d.top1})`}`); }
  console.log('');

  mkdirSync(join(HERE, 'out'), { recursive: true });
  writeFileSync(join(HERE, 'out', 'eval-memory-results.json'), JSON.stringify({
    corpus: files.length, memoryFiles: memCount, k: K, oracle: ORACLE.length, wikilinkEdges: graph.edges,
    lexicalRecall: lexRecall, diffusionRecall: diffRecall, hashRecall, realBM25Top: Number(realBM.toFixed(3)), nonsenseBM25Top: Number(nonBM.toFixed(3)),
    note: 'sovereign offline: hash→BM25⊕title→+wikilink-diffusion. Semantic bge-small upgrade → LK_BUILD_CACHE=1 (network).',
  }, null, 2) + '\n');

  let ok = true;
  for (const [name, pass] of checks) { console.log(`  ${pass ? '✓' : '✗'} ${name}`); if (!pass) ok = false; }
  console.log(`\n  VERDICT: ${ok ? 'GO' : 'NO-GO'} — memory store is recall-indexed (sovereign, offline, deterministic).\n`);
  if (!ok) process.exit(1);
}

// CLI: `node eval-memory.mjs` runs the eval; `node eval-memory.mjs "a question"` recalls over memory.
// Guarded so importing this module (for collectMemoryCorpus / ORACLE) does NOT run the eval.
const Q = process.argv.slice(2).join(' ').trim();
if (import.meta.url === `file://${process.argv[1]}`) if (Q) {
  const files = collectMemoryCorpus();
  const lex = buildLexical(files);
  const graph = buildGraph(files);
  const seed = new Map(lex.rank(Q, 999999).ranked);
  const ranked = [...diffuse(seed, graph).entries()].sort((a, b) => (b[1] - a[1]) || (a[0] < b[0] ? -1 : 1)).slice(0, 8);
  console.log(`\nrecall("${Q}") — top 8 of ${files.length} indexed (BM25⊕title + wikilink-diffusion, offline):\n`);
  ranked.forEach(([id, s], i) => console.log(`  ${i + 1}. ${id}   (${s.toFixed(3)})`));
  console.log('');
} else { main(); }
