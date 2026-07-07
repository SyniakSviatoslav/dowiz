# Living Knowledge + HelixDB + multi-band activation — arc spec (2026-07-07)

Operator directive (2026-07-07): "research and build the living knowledge for the project; check
helix-db — if better than currently used tools, integrate; add an activation function so multiple
bands can trace any data." Captured as a ready-to-execute arc — the *check* is done here; the
integration is a FRESH-session build (this session hit the 300K context-budget gate, and blind-
integrating a new DB in a rotting context is exactly the degraded work that gate prevents).

## 1 · HelixDB — the check (done, WebFetch github.com/HelixDB/helix-db)
- **What it is:** Rust graph **+** vector DB (also KV/doc/relational), Apache-2.0 (passes
  `guardrail-license` — non-copyleft), **embeddable** (`helix start dev`, default port 6969) or cloud.
  Active (v3.0.8, Jul 2026, ~5.6k★). Query DSLs in Rust/TS/Go/Python compile to a JSON AST → `POST
  /v1/query`. Built-in full-text search, ACID, auto-scaling reads. Positioned as AI "memory /
  company-brain" federated store.
- **Why it fits "living knowledge":** it unifies in ONE store what dowiz currently splits across three
  (per AGENTS.md): repowise (why-synthesis), codebase-memory (code-structure graph), VSA (data
  compression). A single graph+vector store is the natural home for a *living* (continuously-updated,
  self-evolving) knowledge base tied to §7 (self-evolving harness).
- **Honest verdict:** PROMISING, not proven-better. It must earn integration by a ground-truth eval
  (§0·GP), not a vibe. It also adds a running service = an infra change (gated). Recommendation: a
  scoped SPIKE + head-to-head eval before adoption — do NOT rip out repowise/codebase-memory first.

## 2 · The activation function — "multiple bands trace any data" (design sketch)
Concept = **spreading activation over the graph+vector store.** A query seeds activation at the
node(s)/vector(s) most similar to the query; activation propagates along edges with a decay factor;
nodes above a threshold form the "activated subgraph" any consumer can trace.
- **Bands** = independent consumers/modalities that trace the SAME store on different signals — e.g.
  code-structure band (import/call edges), why band (repowise rationale edges), data band (VSA payload
  vectors), temporal band (commit/session recency). Each band = a weighting profile over edge-types +
  a seed strategy; all resolve against one HelixDB graph.
- **Activation function (deterministic):** `a(n, t+1) = clamp( a(n,t)·retain + Σ_{m→n} a(m,t)·w(edge_type, band)·decay )`,
  seeded by vector-similarity(query, n); stop when Δactivation < ε or hop-budget reached; return nodes
  with `a ≥ θ`. Deterministic given (query, band-weights, decay, θ) ⇒ reproducible, cacheable,
  gate-able. This is the retrieval dual of §7·A speculative decoding: a cheap activation pass proposes
  the relevant subgraph, a ground-truth check (does the traced node actually contain the answer bytes)
  verifies before use.
- **Multi-band trace = run several band-profiles, union/intersect the activated subgraphs.** A datum is
  "traceable by any band" iff at least one band's activation reaches it — an auditable coverage metric.

## 3 · Fresh-session build plan (execute in order; each step gated)
1. **SPIKE (isolated, `spikes/` per post-edit-gates boundary):** stand up `helix start dev`, ingest a
   bounded slice (e.g. the domain crate's symbols + edges) via the Rust DSL. DoD: a graph traversal +
   a vector query both return correct results against a hand-derived oracle. Red-line: none (spike).
2. **EVAL vs current tools (§0·GP ground truth):** same 20 retrieval questions against
   HelixDB vs repowise+codebase-memory; measure precision/recall + tokens. DoD: a committed
   comparison table with a GO/NO-GO. Integrate ONLY if measurably better.
3. **INTEGRATE (if GO):** add HelixDB as the living-knowledge store behind a thin port (so it swaps —
   playbook §1.5 seam rule); ingestion pipeline from commits/sessions/reflections = the "living" feed;
   Apache-2.0 dep + the new service registered (env-classification guardrail). Red-line: infra/deps →
   human-gated (protect-paths blocks package.json/lock; operator applies).
4. **ACTIVATION FUNCTION:** implement the deterministic spreading-activation above as a query layer
   over HelixDB; define ≥3 band profiles; expose `trace(query, bands[]) → activated subgraph`. Gate:
   a determinism test (same inputs → identical activated set, canonical order) + a coverage metric.
5. **WIRE into the harness:** activation-traced context replaces blind full-file reads where a band
   can reach the answer (token lever); the acceptance is ground-truth-checked (§7·A).

## 4 · Guardrails / constraints
- License Apache-2.0 = OK (non-copyleft). New service + deps = infra change → protect-paths blocks the
  lockfile/package.json; operator applies. Ethics Charter unaffected (internal knowledge tooling).
- Do NOT delete repowise/codebase-memory until the eval (step 2) proves HelixDB better (§7·B: deletion
  needs a deterministic proof of rot — here, a measured worse-or-equal on the eval).
- Keep it a swappable port (seam) so a NO-GO or a future better store costs an impl swap, not a rewrite.
