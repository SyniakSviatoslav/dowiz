---
id: KS-BLUEPRINT
title: Consolidated Knowledge Spine — SDD + Plans + Docs + Memory (graph + vector)
status: proposed            # proposed | active | done | superseded | archived
type: blueprint             # constitution | spec | change | adr | blueprint | roadmap | steering | reflection | lesson | regression | memory | report
owner: SyniakSviatoslav
created: 2026-07-14
updated: 2026-07-14
supersedes: []
superseded_by: null
links:
  - relates_to: "[[math-first-architecture-arc-2026-07-14]]"
  - relates_to: "[[internal-retrieval-living-memory-arc-2026-07-14]]"
  - relates_to: "[[integration-research-tf-attention-circuit-kalman-arc-2026-07-14]]"
  - depends_on: "spikes/living-knowledge/"        # the proven, stranded recall engine
  - governs: "openspec/"                            # dormant change-object substrate
inclusion: manual           # always | fileMatch:<glob> | auto | manual   (Kiro-style steering)
confidence: high
decisions_locked: [spec-model=openspec-best-of-breed, index=sovereign-rust, xrepo=federated+shared-root, rollout=index-first]
tags: [sdd, knowledge, retrieval, memory, openspec, graph, vector, adr, steering, staleness]
---

# Consolidated Knowledge Spine

> **One canonical approach** for spec-driven development + structured agent planning + docs +
> decisions + living memory, across the whole `/root` ecosystem — stored as **markdown
> source-of-truth** with a **rebuildable graph+vector index**, so *nothing is ever missed,
> agents always know where to look, and every decision/approach/vision is preserved in remote
> history + living memory.*
>
> This document **dogfoods** the frontmatter contract it proposes (see its own header) and is
> intended to become the **single canonical knowledge-architecture blueprint** — the ~10 competing
> `MASTER-*/ROADMAP-*` docs are *reference*, not authority (Phase 2 tombstones them).

---

## 0. Decisions locked (operator, 2026-07-14)

| # | Decision | Chosen | Why (one line) |
|---|----------|--------|----------------|
| D1 | Spec/planning substrate | **OpenSpec best-of-breed** | Revive the already-installed change-delta model; add constitution-bind + Kiro steering-inclusion + MADR ADR spine. Only brownfield-native, drift-fighting substrate — and it is already on disk. |
| D2 | Graph+vector index engine | **Sovereign Rust, in-kernel** | Port the stranded `activate.mjs` recall onto `kernel/markov.rs` + tantivy(BM25) + usearch(HNSW) + CSR graph. Zero-dep, MIT/Apache, matches non-AI-first ethos, unstrands proven `recall@5 = 1.0`. |
| D3 | Cross-repo model | **Federated index + shared root** | Each repo keeps its own stores; ONE index spans all; a small shared root holds the shared constitution + cross-repo capability specs + org-wide ADRs (referenced, not duplicated). |
| D4 | Rollout | **Phase 0 = index-first** | Deliver "nothing is missed" on day one (unstrand retrieval + frontmatter + generated `MAP.md`), *then* revive OpenSpec, consolidate, federate, and finally harden into Rust. |

> **Reconciling D2 (Rust) with time-to-value:** the proven JS engine (`spikes/living-knowledge`,
> `recall@5 = 1.0`) is used **as the working index in Phase 0 and as the golden oracle** the Rust
> port must match (recall-parity gate) in Phase 4. Value lands before the port; the port is a
> hardening step, not a blocker.

---

## 1. The core finding — this is consolidation, not greenfield

Every piece the "one consolidated system" needs already exists in the ecosystem but is **stranded,
dormant, or fragmented**:

| Need | Already have | State |
|------|--------------|-------|
| Spec/change front-door | `openspec/` (schema-driven `opsx` gen; `config.yaml` already encodes red-lines + "adopt gradually") | **dormant since 2026-06-20** — 1 archived change, 1 spec |
| Constitution / durable rules | `.claude/CLAUDE.md` + `AGENTS.md` (red-lines, Mandatory Proof Rule, Task-Exit Rule) | live, **not bound** to specs |
| Decision spine (the *why*) | `docs/adr/` (16) + ADRs in memory-dir, design dirs, `CHALLENGE-LOG.md`, root `DECISIONS.md` | **≥5 locations, 2 naming schemes** |
| Graph+vector retrieval | `spikes/living-knowledge/` — vector-seed → deterministic **spreading-activation** recall | **`recall@5 = 1.0` but wired to nothing; ingests a partly-deleted file list** |
| Diffusion / personalized-PageRank math | `kernel/src/markov.rs` + `spectral.rs` | built, **not applied to docs** |
| Steering / context injection | `docs/design/*` arcs + 177 memory topic files | freeform, **no `inclusion:` metadata** |
| Self-improvement (schema'd + wired) | `docs/{reflections,lessons,regressions}/` + `pre-edit-lessons.sh` hook | **the one good pattern — generalize it** |

**External research converges on exactly this stack** (full citations in the research appendix, §11):
OpenSpec's change-delta (only brownfield-native, drift-fighting substrate); Kiro steering-inclusion
for token-cheap context; MADR ADRs as the decision spine; and for storage — **keep markdown as
source-of-truth with a *rebuildable derived* graph+vector index** (Basic Memory pattern), built
**deterministically with no LLM at index time** (LazyGraphRAG), updated by **invalidate-not-delete**
(Graphiti bi-temporal — your existing "demote to ATTIC, never delete" doctrine, now peer-reviewed),
retrieved **vector-seed → spreading-activation / personalized-PageRank** (= HippoRAG-2 = your
`activate.mjs`), with a **staleness banner on every result** (generalize repowise's `stale_warning`).

---

## 2. Pain points this closes (ranked, with hard numbers)

- **P1 — No live index over ~553 markdown artifacts** (308 dowiz docs + 177 memory + 68 bebop design); the one engine that *proves* `recall@5=1.0` over this corpus is stranded and ingests deleted files. repowise indexes **code only** and is ~30 days stale. → *Phase 0.*
- **P2 — Plan-truth smeared across ~10 overlapping `MASTER-*/ROADMAP-*` docs** with prose-only status that drifts (memory itself carries "STALE 178→144" corrections). No single source of truth. → *Phase 2.*
- **P3 — Decisions scattered across ≥5 stores, 2 naming schemes.** No canonical register for "why was X decided". → *Phase 2.*
- **P4 — `MEMORY.md` is capped + injected every call** → active knowledge silently ages into `MEMORY-ATTIC.md`; reactivated topics aren't promoted back. → *Phase 0 + Phase 4 (bi-temporal).*
- **P5 — Proven artifacts orphaned** (`activate.mjs`), plus a broken `[[fsm-graph-analysis]]` wikilink → a missing file. → *Phase 0.*
- **P6 — Cross-repo fragmentation**: 2 separate memory stores (`-root-dowiz`, `-root-bebop-repo`); dowiz + dowiz-pq have OpenSpec+ADRs, **bebop-repo has neither** (68 design docs, 0 ADRs). → *Phase 3.*
- **P7 — The richest reports exit to Telegram** (20 `Report→TG msg N` refs), unindexed and unrecoverable by any repo tool. → *Phase 3.*
- **P8 — No freshness signal on any doc** (dated filenames are the only cue; superseded docs sit beside current ones with no tombstone). → *Phase 4.*

---

## 3. Target architecture — one "knowledge spine" per repo + a cross-repo hoist

```
                        ┌───────────────────────────────────────────────┐
   agent reads first →  │  MAP.md  (generated, always-current index)     │  ← "nothing is missed"
                        └───────────────────────────────────────────────┘
                                          ▲  regenerated on change (hook)
   ┌──────────────────────────────────────┴───────────────────────────────────────┐
   │                    GRAPH + VECTOR INDEX  (rebuildable, derived)                │
   │   vector-seed (usearch HNSW + tantivy BM25) → spreading-activation / PPR       │
   │   (markov.rs, CSR graph) → temporal rerank (status-aware) → staleness banner   │
   └──────────────────────────────────────▲───────────────────────────────────────┘
                                          │  indexes (never authoritative)
   ┌──────────────────────────────────────┴───────────────────────────────────────┐
   │                    MARKDOWN SOURCE-OF-TRUTH  (git-tracked, human+agent)        │
   │                                                                                │
   │  CONSTITUTION      .claude/CLAUDE.md · AGENTS.md          (always injected)     │
   │  SPECS (living)    openspec/specs/<capability>/spec.md    (WHAT — EARS)         │
   │  CHANGES           openspec/changes/<name>/{proposal,design,tasks,specs-delta}  │
   │      → archive     openspec/changes/archive/YYYY-MM-DD-<name>/  (dated ledger)  │
   │  DECISIONS (why)   docs/adr/NNNN-<title>.md               (MADR spine)          │
   │  STEERING          docs/design/<arc>/*.md  + inclusion: frontmatter            │
   │  SELF-IMPROVE      docs/{reflections,lessons,regressions}/ (KEEP — already good)│
   │  MEMORY (living)   memory/*.md  (frontmatter + typed [[wikilinks]], per-row τ)  │
   │  REPORTS           docs/reports/<date>-<slug>.md  ← Telegram capture-back       │
   └────────────────────────────────────────────────────────────────────────────────┘
```

**Invariants**
1. **Markdown is source-of-truth; the index is a rebuildable cache** — never let the index become authoritative.
2. **Deterministic index build — no LLM at index time** (LazyGraphRAG). Edges come from frontmatter `links:`, `[[wikilinks]]`, `supersedes`, and co-occurrence.
3. **Invalidate, never delete** — supersede/demote with a per-row τ; the ATTIC is a *tier*, not a graveyard.
4. **Retrieval = vector kNN seed → spreading-activation expansion → status-aware rerank** (vector is the entry point; diffusion is the multi-hop complement, never a replacement).
5. **Every recall result carries `indexed_commit` + a `stale_warning`** when the source changed after indexing.
6. **Ceremony ∝ blast radius** — red-lines (money/auth/RLS/migrations) gate to a human; reversible non-red-line work flows lightly.

---

## 4. The frontmatter contract (KS-01) — what turns the corpus into a typed graph

Every durable doc adopts this header (this blueprint's own header is the reference instance). It is
**machine-parseable**, drives the graph edges, `MAP.md`, steering-inclusion, and the staleness/temporal layers.

```yaml
---
id: <STABLE-ID>              # short, stable, e.g. KS-01 / ADR-0012 / MEM-<slug>
title: <human title>
status: proposed | active | done | superseded | archived   # the temporal/lifecycle field
type: constitution | spec | change | adr | blueprint | roadmap | steering | reflection | lesson | regression | memory | report
owner: <handle>
created: YYYY-MM-DD
updated: YYYY-MM-DD
supersedes: [<id>...]        # tombstone edges — invalidate-not-delete
superseded_by: <id> | null
links:                       # TYPED edges → the knowledge graph
  - relates_to: "[[<id-or-slug>]]"
  - depends_on: "[[...]]"
  - blocks: "[[...]]"
  - decided_by: "[[ADR-...]]"
inclusion: always | fileMatch:<glob> | auto | manual        # Kiro-style steering injection
confidence: high | medium | low
tags: [...]
---
```

**Validator (deterministic gate)** — a lint that fails on: missing required field, dangling
`[[wikilink]]` (catches the current broken `[[fsm-graph-analysis]]`), unknown `type`/`status`,
or a `superseded_by` pointing at a non-existent id. Red→green proof required (Mandatory Proof Rule).

---

## 5. Retrieval architecture (sovereign Rust — D2)

**Corpus:** all durable markdown across repos, filtered by frontmatter `type`; both memory stores;
`CLAUDE.md`/`AGENTS.md`; `openspec/**`.

**Index build (deterministic, incremental):**
- **Chunk** by heading; **embed** with the deterministic embedder inherited from the spike
  (`lib/embed-semantic.mjs` → ported); **content-hash** each file for change detection.
- **FTS**: tantivy (BM25, MIT). **ANN**: usearch or `hnsw_rs` (HNSW, Apache-2.0).
- **Graph (CSR)**: edges from frontmatter `links:` (typed) + `[[wikilinks]]` + `supersedes` +
  LazyGraphRAG co-occurrence — reusing `kernel/src/markov.rs` for the adjacency + diffusion.

**Query path (= `activate.mjs` bands, HippoRAG-2 shape):**
1. **Seed** = BM25 ∪ kNN top-k.
2. **Expand** = deterministic spreading-activation / personalized-PageRank over the CSR graph
   (`a(n,t+1)=clamp01(a(n,t)·retain + Σ a(m,t)·w(edge,band)·decay)`, degree-normalized so hubs
   don't flood; "no-spread band == pure-vector baseline").
3. **Rerank** = status-aware: prefer `active` over `superseded`; respect per-row τ (recency/decay).
4. **Banner** = attach `indexed_commit` + `stale_warning` if the source changed after indexing.

**`MAP.md` generation:** a deterministic pass over all frontmatter emits an always-current index,
grouped by `type` → `status` → arc, with links and staleness flags. Regenerated on any change via a
hook. **This is what agents read first** — the anti-amnesia mechanism, generalized from files to a
retrievable, self-updating map.

**Anti-drift / freshness:** content-hash → incremental re-index of only the changed file; where a
design-doc claim can be made executable (mdBook `test` / Rust doctest), do so — drift then breaks CI.

---

## 6. Cross-repo federation (D3)

- **Shared root** (`knowledge/` hub, hosted in dowiz as primary): the shared constitution excerpt,
  **cross-repo capability specs**, and **org-wide ADRs** — referenced (not duplicated) per repo.
- **Federated index**: one index process points at all repo roots + both memory stores. dowiz-pq /
  dowiz-b1-kalman are git **worktrees** (physically triplicated `docs/`) → deduped by content-hash.
  bebop-repo is a separate repo → indexed as a distinct root.
- **bebop-repo onboarding**: gets `openspec/` + `docs/adr/` + the frontmatter contract (it has 68
  design docs and none today).
- **Telegram capture-back (P7)**: every `Report→TG` also writes the report body as a
  `type: report` doc under `docs/reports/` with the TG `msg_id` as a pointer → the richest artifacts
  become indexed + recoverable instead of exiting into Telegram.

---

## 7. Roadmap — forward-only, each unit gated by the Mandatory Proof Rule

> Building is a **separate, gated act** (Ship Discipline): each unit → commit on a feature branch →
> validate with pasted proof → one `docs/regressions/REGRESSION-LEDGER.md` row per guardrail.
> Ceremony scales to blast radius; red-line units gate to a human.

### Phase 0 — Index-first ("nothing is missed", day one)
- **KS-01** Frontmatter contract + **validator** (lint). *Proof:* validator RED on a doc missing a field / with a dangling wikilink → GREEN after fix.
- **KS-02** **Unstrand** the retrieval engine: repair `ingest.mjs` corpus list (drop deleted files, add `docs/design/**`, `docs/adr/**`, `memory/**`), re-run `eval.mjs`. *Proof:* pasted `eval-results.json` `recall@5` on the refreshed oracle ≥ baseline.
- **KS-03** Generate **`MAP.md`** from frontmatter. *Proof:* a test asserts MAP entry-count == filesystem doc-count AND zero dangling `[[wikilinks]]` (closes P5's broken link).
- **KS-04** Pre-work **retrieval skill/hook** so agents query the index before planning (advisory). *Proof:* returns top-k for a sample query; recorded in the skill's self-test.

### Phase 1 — Revive OpenSpec best-of-breed (D1)
- **KS-05** Bind constitution (CLAUDE.md red-lines) into `openspec/config.yaml` `rules`; document `propose → apply → sync → archive` as the standard flow in `AGENTS.md`.
- **KS-06** Add **steering-inclusion** frontmatter (`inclusion: fileMatch:<glob> | auto`) to the design arcs — highest-ROI token-economy upgrade.
- **KS-07** **Dogfood:** run *this blueprint* through the flow as the first real OpenSpec change. *Proof:* an archived change with synced canonical specs.

### Phase 2 — Consolidate roadmap + ADR spine (P2, P3)
- **KS-08** Elect ONE canonical roadmap (base = `ROADMAP-GROUND-TRUTH-2026-07-14`); mark the ~9 others `status: superseded` + `superseded_by:` (**tombstone, don't delete**). *Proof:* MAP shows exactly 1 `active` roadmap, N `superseded`.
- **KS-09** **MADR ADR consolidation:** single `docs/adr/` register, unified numbering; migrate scattered ADRs (memory-dir, design-dir, `CHALLENGE-LOG`, root `DECISIONS.md`) in as references; RETRO council routes confirmed decisions → ADRs. *Proof:* `docs/adr/INDEX.md` reconciles; a "why was X decided" query resolves to one ADR.

### Phase 3 — Cross-repo federation (P6, P7)
- **KS-10** Shared root (constitution + shared specs + org-wide ADRs); onboard bebop-repo (`openspec/` + `adr/` + frontmatter). *Proof:* index spans 3 repos; a cross-repo query returns a bebop hit.
- **KS-11** **Telegram capture-back** (`type: report` docs). *Proof:* a captured TG report is retrievable by the index.

### Phase 4 — Rust port + hardening (D2 target, P4, P8)
- **KS-12** Port `activate.mjs` → kernel (`markov.rs` personalized-PageRank + tantivy + usearch + CSR). *Proof:* **recall-parity gate** vs the JS oracle (`recall@5` ≥ JS baseline), deterministic across two runs.
- **KS-13** Incremental re-index + **staleness banner** on every result. *Proof:* touch one file → only it re-indexes; results carry `stale_warning` until re-index.
- **KS-14** **Invalidate-not-delete** guardrail: no `rm` in `memory/`|`specs/`; only `status` change / ATTIC demotion. *Proof:* a hook/test blocks a deletion attempt (red→green).

---

## 8. Risks & mitigations

| Risk | Mitigation |
|------|-----------|
| **Over-ceremony / token cost** (Spec Kit's 2,000-line problem) | Deltas not full specs; Agent-OS "lite" files; Kiro inclusion-gating; ceremony ∝ blast radius. Never load a full arc when a MAP entry / skeleton / delta will do. |
| **Adoption drift** (OpenSpec already went dormant once) | Make the flow the *path of least resistance*: skills + pre-work retrieval hook + always-current MAP. A light gate, not heavy process. |
| **Rust port slow to land value** | Phase 0 ships value on the JS engine; the port is Phase 4 with a recall-parity gate. |
| **Index becomes authoritative / drifts from source** | Hard invariant: markdown is truth, index is a rebuildable cache; content-hash incremental re-index; staleness banner. |
| **Spec↔code drift** (the unsolved SDD problem) | OpenSpec archive-sync makes reconciliation a *required* step; the regression ledger + RETRO council own it. |
| **`.claude/` edits are governance-protected** | KS-04/05 touch harness surfaces → use the sanctioned `! <cmd>` operator-unlock channel + asserted patch + red→green proof; do not bypass the gate-topology. |

---

## 9. Definition of done (system-level)

- An agent, before planning any work, reads a **single always-current `MAP.md`** and can **retrieve
  the relevant spec / ADR / arc / memory by concept**, with a staleness banner — over **all repos**.
- **New work** flows `propose → apply → sync → archive`; **decisions** land in the MADR spine;
  **status** is a machine field, not drifting prose.
- **Nothing is deleted** — superseded knowledge is tombstoned/demoted and still retrievable
  ("what did we decide as of commit X").
- The proven `recall@5=1.0` engine is **live and sovereign** (Rust, in-kernel), not stranded.

---

## 10. Open (deferred, non-blocking) decisions
- Exact embedding function for the Rust port (reuse the spike's deterministic embedder vs a small local model) — decided at KS-12 by the recall-parity gate.
- Whether `MAP.md` is one global map or per-repo maps + a federated root map — decided empirically at KS-03/KS-10 by token cost.
- SurrealDB-embedded remains the sanctioned **fallback** engine if the sovereign-Rust glue proves too costly (research had it as the turnkey runner-up).

## 11. Research appendix (sources)
- **SDD frameworks:** GitHub Spec Kit · OpenSpec (change-delta, installed here) · AWS Kiro (steering + EARS) · Tessl/spec-as-source (do NOT adopt — non-determinism vs the Proof Rule) · Agent OS · BMAD · Taskmaster/Conductor · ADRs (Nygard/MADR) · Anthropic "context engineering" · the "waterfall-in-markdown" critique.
- **Memory / graph+vector:** GraphRAG + **LazyGraphRAG** (deterministic, no-LLM index) · **Graphiti/Zep** (bi-temporal invalidate-never-delete) · **Basic Memory** (markdown + SQLite + embeddings — the reference architecture) · **HippoRAG / HippoRAG-2** (vector-seed → personalized-PageRank) · substrate survey (SurrealDB embeddable ✓, HelixDB is a *server* not a lib, KuzuDB abandoned Oct-2025, tantivy + usearch = sovereign Rust building blocks) · RAG freshness / staleness-banner patterns.

*(Full URL-cited research reports are archived with this session; key claims verified against primary sources.)*
