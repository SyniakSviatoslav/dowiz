// activate.mjs — DETERMINISTIC spreading activation over the living-knowledge graph+vector store.
//
// Formula (arc spec): a(n,t+1) = clamp01( a(n,t)·retain + Σ_{m~n} a(m,t)·w(edge_type,band)·decay )
// seeded by vector-similarity(query, n); stop when Δactivation < ε or hop-budget reached; return
// nodes with a ≥ θ in canonical order (a desc, id asc). Deterministic given (query, band, params) ⇒
// reproducible, cacheable, gate-able. This is the retrieval dual of §7·A speculative decoding: a
// cheap activation pass PROPOSES the relevant subgraph; a ground-truth check (does the traced node
// contain the answer bytes) VERIFIES before use.
//
// BANDS = independent consumers tracing the SAME store on different signals: each = an edge-type
// weight profile + seed strategy + decay/retain/θ. A datum is "traceable by band b" iff b's activation
// reaches it (a ≥ θ). Multi-band trace = union/intersect of the activated subgraphs → an auditable
// coverage metric, and (across layer bands) a disconnected-edge detector ("brain inside the brain").
//
// Reverse-engineered from HelixDB (see ../helix-recon.md): a deterministic SCAN-ROW budget
// (nodesVisited cap), not just wall-clock, keeps behavior identical under load (HelixDB's
// slow_query_min_scan_rows pattern). Telemetry per trace (hops, nodesVisited, Δ) mirrors its
// self-instrumented slow-query diagnostics.

const EPS = 1e-6;
const clamp01 = (x) => (x < 0 ? 0 : x > 1 ? 1 : x);

// Band profiles. edgeWeights: per-edge-type multiplier ('*' = default). seedK: how many vector seeds.
// retain/decay/theta/hopBudget/scanBudget tune the spread.
// FAN-OUT NORMALIZATION is the key to query-relevant spread: a hub dumps activation / deg(m) into
// each neighbour, so high-degree nodes (CLAUDE.md, run-armaments) don't flood the ranking with a
// query-independent constant (the first cut did exactly that — every query returned the same hubs).
// SEEDING ALL nodes by cosine means the no-spread `data` band == the pure-vector baseline exactly,
// and structure/why bands = baseline + a degree-normalized neighbour lift (hybrid retrieval, the
// reverse-engineered HelixDB pattern). Retrieval ranks by the activation value; θ is for coverage.
// retain: 1.0 → the score keeps the FULL baseline cosine and only ADDS a neighbour lift (so activation
// can only re-rank, never discard, the pure-vector signal). decay = the lift strength. Spread is
// SYMMETRICALLY degree-normalized (/√(deg(m)·deg(n)), GCN-style) so neither hub sources nor hub
// targets dominate — the flaw the first two cuts hit.
export const BANDS = {
  // structure: follow explicit file/config references — "what is wired to this".
  'code-structure': { edgeWeights: { references: 1.0, governs: 0.7, contains: 0.5, '*': 0.3 }, retain: 1.0, decay: 0.5, theta: 0.05, hopBudget: 1, scanBudget: 20000 },
  // why: follow rule/rationale edges — "which rule or reflection governs this".
  why: { edgeWeights: { governs: 1.0, references: 0.5, contains: 0.3, '*': 0.2 }, retain: 1.0, decay: 0.5, theta: 0.05, hopBudget: 1, scanBudget: 20000 },
  // data: NO spread — pure vector similarity. Equals the baseline (falsifiability anchor).
  data: { edgeWeights: { '*': 0 }, retain: 1.0, decay: 0.0, theta: 0.05, hopBudget: 0, scanBudget: 20000 },
  // temporal: recency-weighted spread — "what changed together recently" (meta.recency in [0,1]).
  temporal: { edgeWeights: { references: 0.6, contains: 0.4, '*': 0.3 }, retain: 1.0, decay: 0.4, theta: 0.05, hopBudget: 1, scanBudget: 20000, recencyBoost: 0.3 },
};

// deterministic per-node degree (fan-out constraint), computed once per store call.
function degrees(store, nodes) {
  const d = new Map();
  for (const n of nodes) d.set(n.id, store.outgoing(n.id).length + store.incoming(n.id).length);
  return d;
}

// One deterministic activation pass for a single band. Seeds ALL nodes by cosine(query, node).
export function activate(store, query, bandName) {
  const band = BANDS[bandName];
  if (!band) throw new Error(`unknown band: ${bandName}`);
  const nodes = store.nodes();
  const deg = degrees(store, nodes);
  const q = store.vectorTopK(query, nodes.length); // full-corpus cosine, canonical order
  const a = new Map();
  for (const { id, score } of q) {
    let s = Math.max(0, score);
    if (band.recencyBoost) s = clamp01(s + band.recencyBoost * (store.getNode(id)?.meta?.recency ?? 0));
    a.set(id, s);
  }

  const telem = { band: bandName, hops: 0, nodesVisited: 0, nodes: nodes.length };
  for (let t = 0; t < band.hopBudget; t++) {
    const next = new Map();
    for (const n of nodes) next.set(n.id, clamp01(band.retain * (a.get(n.id) || 0)));
    let scan = 0, delta = 0;
    for (const n of nodes) {
      const am = a.get(n.id) || 0;
      if (am <= EPS) continue;
      const dm = deg.get(n.id) || 1; // fan-out normalizer
      const nbrs = store.outgoing(n.id).map((e) => ({ id: e.to, type: e.type, weight: e.weight }))
        .concat(store.incoming(n.id).map((e) => ({ id: e.from, type: e.type, weight: e.weight })));
      for (const nb of nbrs) {
        if (++scan > band.scanBudget) break; // deterministic scan-row budget (HelixDB pattern)
        const w = (band.edgeWeights[nb.type] ?? band.edgeWeights['*'] ?? 0) * nb.weight;
        if (w <= 0) continue;
        const dnb = deg.get(nb.id) || 1;
        next.set(nb.id, next.get(nb.id) + (am * w * band.decay) / Math.sqrt(dm * dnb)); // symmetric norm

      }
    }
    for (const n of nodes) { const v = clamp01(next.get(n.id)); delta += Math.abs(v - (a.get(n.id) || 0)); next.set(n.id, v); }
    for (const [k, v] of next) a.set(k, v);
    telem.hops = t + 1; telem.nodesVisited += scan;
    if (delta < EPS) break;
  }

  const ranked = [...a.entries()]
    .sort((x, y) => (y[1] - x[1]) || (x[0] < y[0] ? -1 : 1))
    .map(([id, act]) => ({ id, a: Number(act.toFixed(6)) }));
  const activated = ranked.filter((r) => r.a >= band.theta);
  return { ranked, activated, telem };
}

// Multi-band trace: run several bands, union/intersect the activated subgraphs.
export function trace(store, query, bandNames = Object.keys(BANDS)) {
  const perBand = {}; const telems = [];
  for (const b of bandNames) { const r = activate(store, query, b); perBand[b] = r.activated; telems.push(r.telem); }
  const sets = bandNames.map((b) => new Set(perBand[b].map((x) => x.id)));
  const union = [...new Set(sets.flatMap((s) => [...s]))].sort();
  const intersect = union.filter((id) => sets.every((s) => s.has(id))).sort();
  return { perBand, union, intersect, telems };
}

// ── brain-in-brain: cross-layer structural analysis (deterministic, falsifiable) ──
// A node reachable by one layer's band but not another's = a candidate disconnected edge. Structural
// form: the layer×layer edge matrix + "island" nodes (no cross-layer edge) + layer-pairs with ZERO
// connecting edges. These are the "not connected edges / useful findings" for the autonomous phase.
export function analyzeLayers(store) {
  const nodes = store.nodes();
  const layers = [...new Set(nodes.map((n) => n.label))].sort();
  const matrix = {}; // "A|B" -> count of edges between layer A and layer B
  for (const a of layers) for (const b of layers) matrix[`${a}|${b}`] = 0;
  const layerOf = new Map(nodes.map((n) => [n.id, n.label]));
  const crossDeg = new Map(nodes.map((n) => [n.id, 0])); // cross-layer edge count per node
  for (const e of store.edges()) {
    const la = layerOf.get(e.from), lb = layerOf.get(e.to);
    if (la == null || lb == null) continue;
    matrix[`${la}|${lb}`]++; if (la !== lb) matrix[`${lb}|${la}`]++;
    if (la !== lb) { crossDeg.set(e.from, crossDeg.get(e.from) + 1); crossDeg.set(e.to, crossDeg.get(e.to) + 1); }
  }
  const islands = nodes.filter((n) => crossDeg.get(n.id) === 0).map((n) => ({ id: n.id, label: n.label })).sort((x, y) => (x.id < y.id ? -1 : 1));
  const disconnectedLayerPairs = [];
  for (let i = 0; i < layers.length; i++) for (let j = i + 1; j < layers.length; j++) {
    if (matrix[`${layers[i]}|${layers[j]}`] === 0) disconnectedLayerPairs.push([layers[i], layers[j]]);
  }
  return { layers, matrix, islands, disconnectedLayerPairs, nodeCount: nodes.length, edgeCount: store.edges().length };
}
