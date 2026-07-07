// store.mjs — the living-knowledge STORE behind a thin PORT (playbook §1.5 seam rule).
//
// One interface, two backends:
//   • MemoryStore — sovereign, in-repo, zero-infra, deterministic (the DEFAULT). Graph (nodes+typed
//     edges) + a vector index (deterministic hash embeddings). Persists to a JSON snapshot.
//   • HelixStore  — dev-gated adapter to the REAL HelixDB engine (ghcr.io/helixdb/enterprise-dev on
//     :6969), translating store ops to HelixDB's confirmed JSON-AST wire format
//     ({request_type, query:{queries:[{Query:{name,steps}}]}, returns, parameters}; steps AddN/NWhere/
//     Count). Reverse-engineered from a live engine (see ../helix-recon.md). Never the prod default —
//     the sovereign backend is (closed/unlicensed engine collides with the Sovereign-Core thesis).
//
// Backend chosen by LK_BACKEND=memory|helix (default memory). The activation function (activate.mjs)
// runs in OUR layer over data pulled from whichever backend — so retrieval math is backend-agnostic;
// the backend choice is a storage/scale decision, not a correctness one.
import { embed, cosine, tokenize } from './embed.mjs';

// ── PORT ─────────────────────────────────────────────────────────────────────
// addNode({id,label,title,text,meta}) · addEdge({from,to,type,weight}) · finalize()
// getNode(id) · nodes() · edges() · incoming(id) · outgoing(id) · vectorTopK(text,k)

export class MemoryStore {
  constructor() {
    this._nodes = new Map();
    this._edges = [];
    this._out = new Map();  // id -> [{to,type,weight}]
    this._in = new Map();   // id -> [{from,type,weight}]
    this.idf = null;        // token -> inverse-document-frequency (built in finalize)
  }
  addNode(n) {
    if (this._nodes.has(n.id)) return;
    const etext = `${n.title || ''} ${n.text || ''}`;
    this._nodes.set(n.id, { id: n.id, label: n.label, title: n.title || n.id, text: n.text || '', meta: n.meta || {}, _etext: etext, vec: embed(etext) });
    this._out.set(n.id, []); this._in.set(n.id, []);
  }
  addEdge(e) {
    if (!this._nodes.has(e.from) || !this._nodes.has(e.to)) return;
    const edge = { from: e.from, to: e.to, type: e.type || 'ref', weight: e.weight ?? 1 };
    this._edges.push(edge);
    this._out.get(e.from).push({ to: e.to, type: edge.type, weight: edge.weight });
    this._in.get(e.to).push({ from: e.from, type: edge.type, weight: edge.weight });
  }
  // Build corpus IDF and re-embed every node with it (deterministic). Smoothed IDF is always > 0, so
  // ubiquitous tokens get a small weight (not dropped) and distinctive tokens dominate.
  finalize() {
    const N = this._nodes.size || 1;
    const df = new Map();
    for (const n of this._nodes.values()) {
      for (const t of new Set(tokenize(n._etext))) df.set(t, (df.get(t) || 0) + 1);
    }
    this.idf = new Map();
    for (const [t, c] of df) this.idf.set(t, Math.log((N + 1) / (c + 0.5)));
    for (const n of this._nodes.values()) n.vec = embed(n._etext, this.idf);
    return this;
  }
  getNode(id) { return this._nodes.get(id); }
  nodes() { return [...this._nodes.values()]; }
  edges() { return this._edges; }
  outgoing(id) { return this._out.get(id) || []; }
  incoming(id) { return this._in.get(id) || []; }
  // deterministic vector index: cosine over all node vectors, tie-broken by id (canonical order).
  vectorTopK(text, k = 8) {
    const q = embed(text, this.idf);
    return this.nodes()
      .map((n) => ({ id: n.id, score: cosine(q, n.vec) }))
      .sort((a, b) => (b.score - a.score) || (a.id < b.id ? -1 : 1))
      .slice(0, k);
  }
  snapshot() {
    return {
      nodes: this.nodes().map((n) => ({ id: n.id, label: n.label, title: n.title, meta: n.meta })),
      edges: this._edges,
    };
  }
}

// ── HelixStore — dev-gated adapter to the real engine ────────────────────────
// Only the graph-storage path is implemented against the confirmed AST (AddN/NWhere/Count); vector
// KNN via HelixDB's HNSW is left to a follow-up (its vector-step AST wasn't reverse-engineered). The
// point of this adapter is the empirical head-to-head: prove the store round-trips on the REAL
// engine. Requires the engine running on LK_HELIX_URL (default http://localhost:6969).
export class HelixStore {
  constructor(url = process.env.LK_HELIX_URL || 'http://localhost:6969') {
    this.url = url; this._mem = new MemoryStore(); // mirror locally so activation still works
  }
  async _query(body) {
    const res = await fetch(`${this.url}/v1/query`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
    });
    return { status: res.status, body: await res.json().catch(() => ({})) };
  }
  // Wire contract confirmed against the LIVE engine (ghcr.io/helixdb/enterprise-dev, 2026-07-07): the
  // raw /v1/query endpoint accepts {request_type, query, parameters, query_name?} and REJECTS a
  // `returns` field (that was a CLI-only wrapper key — the engine's own parser error listed the valid
  // fields). Results come back keyed by each Query's `name`.
  // Wire contract confirmed against the LIVE engine (ghcr.io/helixdb/enterprise-dev, 2026-07-07): the
  // raw /v1/query endpoint takes {request_type, query, parameters}; `returns` is REJECTED at root but
  // REQUIRED INSIDE `query` (sibling of `queries`) — the recon/CLI shape had it at root. Derived from
  // the engine's own parser errors ("unknown field returns" at root; "missing field returns" in the
  // inline query). Results come back keyed by each Query's `name`.
  async health() {
    const r = await this._query({ request_type: 'read', query: { queries: [{ Query: { name: 'readiness', steps: [{ NWhere: { Eq: ['$label', { String: '__HelixReadiness__' }] } }, 'Count'], condition: null } }], returns: ['readiness'] }, parameters: {} });
    return r.status === 200;
  }
  // AddN with properties as a SEQUENCE OF PAIRS (recon: a map shape gets a 400 — strict AST).
  async addNode(n) {
    this._mem.addNode(n);
    // property value wrapper confirmed live: [key, {Value:{String:...}}] (not {String:...}, not {Expr}).
    const props = [['lk_id', { Value: { String: String(n.id) } }], ['title', { Value: { String: String(n.title || n.id) } }]];
    const r = await this._query({ request_type: 'write', query: { queries: [{ Query: { name: 'add', steps: [{ AddN: { label: String(n.label || 'Node'), properties: props } }], condition: null } }], returns: ['add'] }, parameters: {} });
    return r.status === 200;
  }
  async countByLabel(label) {
    const r = await this._query({ request_type: 'read', query: { queries: [{ Query: { name: 'c', steps: [{ NWhere: { Eq: ['$label', { String: String(label) }] } }, 'Count'], condition: null } }], returns: ['c'] }, parameters: {} });
    return r.body?.c?.count ?? r.body?.count ?? null;
  }
  addEdge(e) { this._mem.addEdge(e); }
  finalize() { return this; }
  getNode(id) { return this._mem.getNode(id); }
  nodes() { return this._mem.nodes(); }
  edges() { return this._mem.edges(); }
  outgoing(id) { return this._mem.outgoing(id); }
  incoming(id) { return this._mem.incoming(id); }
  vectorTopK(text, k) { return this._mem.vectorTopK(text, k); }
}

export function makeStore() {
  return (process.env.LK_BACKEND === 'helix') ? new HelixStore() : new MemoryStore();
}
