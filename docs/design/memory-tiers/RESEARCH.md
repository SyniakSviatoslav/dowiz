# Tiered Cognitive-Memory Architecture for the dowiz/DeliveryOS Harness — Research Dossier

> **Read-only research input** (2026-07-03). Maps the agentic-memory literature onto this repo's
> existing file+hook+meta-controller substrate and proposes a minimal, file-based tiered design.
> No harness code is changed by this document. Every external claim carries a source URL.

---

## Part 1 — How agentic-AI memory layers are designed and how they self-reinforce

### 1.1 The four (five) canonical memory types

The taxonomy used across current agent-memory surveys inherits directly from the cognitive
architectures Soar and ACT-R: **working**, **episodic**, **semantic**, **procedural** (plus a
transient **sensory** buffer that rarely matters for text agents).

- **Working / short-term memory** — the agent's *active workspace*: the information currently being
  manipulated inside the context window (the running task, recent tool calls, scratch state). It is
  small, volatile, and the thing that gets overwritten each step.
- **Episodic memory** — *records of concrete experiences*: individual tool calls, conversation
  turns, environment observations, "what happened at time T." Time-indexed and event-shaped.
- **Semantic memory** — *concepts and factual knowledge* abstracted away from any single event:
  user preferences, profiles, stable facts about the world/system.
- **Procedural memory** — *knowing how to perform actions and skills*: verified code patterns,
  reusable workflows, IF-THEN rules. "Software-engineering agents lean heavily on procedural memory
  (verified code patterns and architecture decisions); personal assistants lean on semantic."

Sources:
[Types of AI Agent Memory (Atlan)](https://atlan.com/know/types-of-ai-agent-memory/) ·
[Rethinking Memory Mechanisms of Foundation Agents: A Survey (arXiv 2602.06052)](https://arxiv.org/html/2602.06052v3) ·
[Anatomy of Agentic Memory (arXiv 2602.19320)](https://arxiv.org/html/2602.19320v1) ·
[Memory for Autonomous LLM Agents (arXiv 2603.07670)](https://arxiv.org/html/2603.07670v1)

The types are **interconnected by consolidation**: "an episodic fact like *the user corrected the
date format on Jan 5, Jan 12, and Feb 1* may consolidate into the semantic record *user prefers
DD/MM/YYYY*." This episodic→semantic lift is the through-line of every system below.
([Atlan](https://atlan.com/know/types-of-ai-agent-memory/))

### 1.2 Tiered memory / virtual-context management — MemGPT / Letta

MemGPT (now **Letta**) reframes the LLM as an operating system: the context window is **RAM**, an
external store is **disk**, and the model **pages** information between them under its own control —
"virtual context management … inspired by hierarchical memory systems which provide the illusion of
an extended virtual memory via paging between physical memory and disk."
([Leonie Monigatti](https://www.leoniemonigatti.com/blog/memgpt.html) ·
[MemGPT paper, arXiv 2310.08560](https://arxiv.org/abs/2310.08560))

Three tiers:
- **Core memory** — a small block that *lives in the context window* (RAM). The agent reads and
  **self-edits** it directly (e.g. a persona block + key user facts).
- **Recall memory** — searchable conversation history stored *outside* context (a disk cache).
- **Archival memory** — long-term cold storage the agent queries via explicit tool calls.

"Self-editing memory": the LLM decides when to push information from main context to archival
(*paging out*), retrieve from archival (*paging in*), search recall memory, or even edit its own
system prompt — all implemented as **function calls the model can invoke** inside its reasoning
loop. When context fills, a **memory-pressure warning** triggers a flush.
([DeepLearning.AI course](https://www.deeplearning.ai/courses/llms-as-operating-systems-agent-memory) ·
[lmatlas building-block](https://www.lmatlas.com/building-blocks/memgpt-letta))

The load-bearing idea for us: **context window = working memory is finite; the *policy* for what to
page in and out is the whole game.** In this repo, "paging in" is what the `pre-edit-lessons` hook
already does (it injects a relevant lesson into context before an edit).

### 1.3 Reflection & consolidation — Stanford Generative Agents

Generative Agents store experiences in a natural-language **memory stream** and retrieve them with a
weighted score over three factors
([paper, arXiv 2304.03442](https://ar5iv.labs.arxiv.org/html/2304.03442) ·
[MemX glossary](https://memx.app/glossary/generative-agents/)):

```
score = α_recency·recency + α_importance·importance + α_relevance·relevance      (all α = 1)
```

- **Recency** — exponential decay (factor **0.995**) over hours since the memory was **last
  accessed** (so retrieval itself refreshes recency — a reinforcement signal).
- **Importance** — a **poignancy** score the LLM assigns *at creation time*, 1 (mundane: brushing
  teeth) to 10 (poignant: a breakup).
- **Relevance** — cosine similarity between the memory's embedding and the current query.

**Reflection** synthesises memories into higher-level inferences. It fires **when the sum of
importance scores of the latest observations exceeds a threshold (150)** — "roughly two or three
times a day" — and reflections can themselves reflect on prior reflections, forming a **tree** whose
leaves are raw observations and whose upper nodes are ever-more-abstract thoughts.
([WebFetch of arXiv 2304.03442](https://ar5iv.labs.arxiv.org/html/2304.03442))

This is a **file-friendly** design: no vector DB is intrinsic to the *idea* — the scoring is three
cheap numbers per memory, and reflection is a scheduled batch job gated by an importance threshold.

### 1.4 Retrieval-augmented memory systems — Mem0 vs Zep/Graphiti

- **Mem0** is **vector-first**: it uses an LLM to **extract salient facts** from conversations into
  embeddings (optionally a graph in its Pro tier), via a two-phase **extraction → update**
  pipeline. Time is stored as **metadata**; on a conflict (user changed jobs) it keeps *both* facts
  and makes the answering model resolve by timestamp.
- **Zep / Graphiti** is a **temporal knowledge graph**: every fact/edge carries `valid_from`,
  `valid_to`, `invalid_at` markers, so a superseded fact is *explicitly* marked and queries return
  only the currently-true value by default. Time is **structure**, not metadata. On LongMemEval,
  Graphiti's temporal handling beat Mem0 (63.8% vs 49.0% with GPT-4o).

Sources:
[Mem0 paper (arXiv 2504.19413)](https://arxiv.org/html/2504.19413v1) ·
[Zep vs Mem0 (Atlan)](https://atlan.com/know/zep-vs-mem0/) ·
[Mem0 vs Zep vs Letta tested (Particula)](https://particula.tech/blog/agent-memory-frameworks-tested-mem0-zep-letta-cognee-2026)

**Tradeoffs — vector vs graph vs file:**
- *Vector* → best for fuzzy semantic recall over unstructured text; weak at "what was true then,"
  contradiction resolution, and multi-hop; needs an embedding store + index (a new heavy dep).
- *Graph* → best for temporal reasoning, supersession, and relationships; heaviest to build/operate.
- *File* (this repo today) → best for **auditability, determinism, zero-dep, human-in-the-loop
  review**, and git-native history/decay; weak at fuzzy similarity ranking and scale. For a harness
  whose store is measured in **dozens** of items (11 lessons, 10 INBOX reflections, ~120 memory
  files) and whose culture is YAGNI/ponytail + zero-new-dep, **file wins** — the missing pieces are
  *scoring, decay, and supersession*, all of which are cheap to add to files.

### 1.5 Cognitive-architecture grounding — SOAR & ACT-R (brief)

- **ACT-R** splits **declarative** memory (facts as symbolic *chunks*) from **procedural** memory
  (IF-THEN *productions*). It **learns by strengthening**: chunks strengthen when accessed and rules
  strengthen when they fire; productions are rated by probability-of-success, cost, and goal value
  for conflict resolution. New rules start at a **specified strength that must be re-learned** over
  many firings before they win.
- **SOAR** has working memory + procedural memory + long-term declarative memory split into
  **semantic and episodic** stores. Its **chunking** mechanism *compiles* deliberate multi-step
  reasoning into a single production rule (a compression: multi-step → automatic single step), and
  it uses **reinforcement learning** to compare operators. SOAR chunks are **immediately available**
  (one-shot), where ACT-R rules must be strengthened.

Sources:
[ACT-R architecture (Ritter et al., PSU)](https://acs.ist.psu.edu/papers/ritterTOip.pdf) ·
[Comparing SOAR/ACT-R/CLARION/DUAL (RoboticsBiz)](https://roboticsbiz.com/comparing-four-cognitive-architectures-soar-act-r-clarion-and-dual/) ·
[Cognitive architectures & agents (Purdue)](https://ccn.psych.purdue.edu/papers/cogArch_agent-springer.pdf)

The one idea to steal: **"compile a reflection into a production rule."** SOAR-chunking ≈ this
repo's **reflection → deterministic guardrail** promotion (a multi-step lesson compiled into a
single always-on gate). ACT-R strengthening ≈ **reinforcement-on-reuse** (a lesson that keeps firing
earns its keep; one that never fires decays).

### 1.6 Self-reinforcement mechanisms (the key ask)

How the layers *strengthen and decay themselves*, synthesised across sources:

1. **Reinforcement on reuse (recall strengthens).** "Memory is reinforced through repetition — when
   an event is recalled, the model updates its temporal significance … repeated recall makes the
   memory less susceptible to forgetting." Generative Agents encode this literally: recency decays,
   but **access resets last-access time**, so a re-used memory stays retrievable.
   ([Consolidation Problem (Hindsight/Vectorize)](https://hindsight.vectorize.io/blog/2026/05/21/agent-memory-consolidation) ·
   [Generative Agents](https://ar5iv.labs.arxiv.org/html/2304.03442))
2. **Importance-driven retention.** Importance is scored at creation (poignancy 1–10) and gates both
   retrieval and reflection; "importance-driven forgetting integrates temporal, frequency, and
   semantic signals to retain high-value knowledge while pruning redundancy."
   ([Memory Systems research (Steve Kinney)](https://stevekinney.com/writing/agent-memory-systems))
3. **Decay / forgetting.** A **four-lever** consolidation framework — **importance** (what becomes a
   memory), **merge** (unify related facts), **decay** (confidence degrades over time), **eviction**
   (when a memory leaves). "Recent episodic memories maintain high fidelity, while older ones
   compress into semantic summaries or fade entirely," and "episodic memories that contributed most
   to successful outcomes persist longer while routine interactions fade."
   ([Consolidation Problem (Hindsight)](https://hindsight.vectorize.io/blog/2026/05/21/agent-memory-consolidation))
4. **Promotion episodic → semantic → procedural.** Complementary Learning Systems theory:
   **fast episodic acquisition** + **slow semantic consolidation** reduces interference while
   preserving adaptability. OpenAI-style RL agents "maintain detailed episodic records during active
   learning, then gradually compress these into semantic policy updates." SOAR chunking then
   compiles the stable pattern into a procedural rule.
   ([Memory in the Age of AI Agents (arXiv 2512.13564)](https://arxiv.org/pdf/2512.13564) ·
   [Memory Systems: Episodic vs Semantic (ctoi)](https://ctoi.substack.com/p/memory-systems-in-ai-agents-episodic))
5. **Contradiction handling / supersession.** Zep's temporal edges **explicitly mark** the old fact
   superseded (`invalid_at`) so the current value wins; Mem0 keeps both and resolves by timestamp at
   read time. Either way, contradiction must **demote, not silently coexist**.
   ([Atlan Zep vs Mem0](https://atlan.com/know/zep-vs-mem0/))
6. **Learned memory policies (frontier).** The field is moving "from rule-based memory management to
   LLM-assisted and now RL-driven," with **Memory-R1 / Mem-α** training policies for *when to store,
   consolidate, forget*.
   ([Memory in the Age of AI Agents (arXiv 2512.13564)](https://arxiv.org/pdf/2512.13564)) — flagged
   as **out of scope** for this repo (needs training infra; violates zero-new-dep/YAGNI).

**Reinforcement in one line:** a memory earns retention by being *re-used* (recency reset + reuse
count), *important* (poignancy / red-line weight), and *consistent* (not superseded); it loses
retention by going *stale* (never retrieved), *routine* (low importance), or *contradicted*
(demoted). Promotion up the tiers is the reward; pruning is the penalty.

---

## Part 2 — Mapping the literature onto THIS repo (current stores)

The harness **already implements a file-based cognitive-memory system** — it just isn't *named* as
one and lacks explicit scoring/decay. Current stores, mapped to tiers and to the mechanisms above:

| Repo store | What it holds | Tier (cognitive) | Reinforcement mechanism present today | Gap vs literature |
|---|---|---|---|---|
| **Session context window** + `.claude/logs/harness-events.jsonl` (982+ events; hook injects, routes, nudges) | The running task; a time-ordered event stream of hook fires | **Working / short-term** + a raw **episodic event log** | Volatile; meta-controller reads freshness (`STALE_TELEMETRY`) | No explicit persistent **core-memory block** across sub-agents (MemGPT core-memory analog); event log is never scored/summarised |
| `MEMORY.md` + `/root/.claude/projects/-root-dowiz/memory/*.md` (~120 files, frontmatter `type: project|reference|feedback`) | Distilled durable facts/state per topic (curated index + detail files) | **Semantic** (facts/profile) — the "declarative chunk" store | Manual curation; MEMORY.md is a hand-ranked index | No recency/importance/relevance **scoring**; no automatic **decay** of superseded facts (e.g. "OTP disabled" vs later re-enable); retrieval is human-eyeball |
| `docs/reflections/{INBOX,ARCHIVE,RETRO}/*` (10 INBOX; frontmatter CONTEXT/DECISIONS/WHERE/**WHY**/CONFIDENCE/NEXT-TIME) | Causal post-hoc "why it went wrong" per qualified fix | **Episodic** (events + a causal poignancy) — the memory *stream* | Threshold-gated write (Council qualifier: ≥3 files / ≥3 iterations / stage-close / red-line) ≈ Generative-Agents importance gate; `INBOX → ARCHIVE` = decay/eviction | `CONFIDENCE` is a poignancy signal but **not numeric/scored**; reflection→retrieval is Council-triggered, not continuous; no relevance ranking when many reflections exist |
| `docs/lessons/*.md` (11) + `INDEX.md` (`\| TRIGGER \| file \|`), injected by `pre-edit-lessons.sh` | Trigger-keyed IF-THEN distilled advice (TRIGGER glob/error-sig → ACTION + LINK) | **Procedural (advisory)** — ACT-R productions, *not yet strengthened* | **This is the paging-in policy**: relevance = path-glob match → inject into working memory | Relevance is **binary** (glob hit/miss), **no ranking** when >1 matches; **no reuse count / recency / importance** → no ACT-R strengthening, no decay of never-fired lessons |
| `docs/regressions/REGRESSION-LEDGER.md` (rows #1–#69) | One row per recurring bug class → its **deterministic guardrail** (eslint/boot-guard/E2E/CI-gate), red→green proven | **Procedural (authority)** + **semantic** (the fact "this class recurs") | **SOAR chunking realised**: a reflection compiled into a single always-on production (gate). Monotonic ratchet = never weaken | Guardrails have **no fire/hit telemetry** — no signal for which gates actually catch regressions (reuse) vs are dead weight |
| `loops/*.yaml` + `registry.md` + `loops/memory/*.md`; `.claude/skills/` | Certified reusable procedures (loops) and skills (SKILL.md) | **Procedural (skills/how-to)** — the highest procedural tier | `skill-evolution` promotion bar: **recurred ≥2× AND stable steps AND checkable DoD** ≈ ACT-R "must be re-learned before it wins"; health-pass flags "sick" loops | Promotion bar is human/agent-judged, not driven by measured **reuse frequency** telemetry |
| `scripts/meta-controller.mjs` (L5) + `docs/design/harness/META-CONTROLLER.md` | Ingests all 5 layers → detects gaps → **proposes** additive artifacts (never auto-applies; immutable core refused) | **The consolidation + reflection engine** (a system-level "reflection over reflections") | Already ingests reflections + ledger + telemetry + skill-drafts; ranks gaps; `metrics` records gap-history deltas | **Ingests but does not score memories**: no recency/importance/reuse scoring, no `STALE_LESSON`/decay gap, no contradiction/supersession gap |

**Consolidation pipeline that already exists** (this repo's episodic→semantic→procedural lift, matching §1.6.4):

```
qualified fix ─▶ worker reflection (EPISODIC, docs/reflections/INBOX/)
             ─▶ Council retro (cause-critic challenges WHY · pattern-critic finds cross-reflection
                 structural root · ratchet-critic picks cheapest deterministic output)
             ─▶ librarian ENACTS:  → SEMANTIC (MEMORY.md / memory row)
                                    → PROCEDURAL advisory (docs/lessons/* + INDEX row, hook-injected)
                                    → PROCEDURAL authority (REGRESSION-LEDGER guardrail, red→green)  ← "chunking"
                                    → prune the store + move reflection to ARCHIVE/ (decay/eviction)
```

This is a faithful, deterministic, human-gated implementation of the CLS "fast episodic → slow
semantic → compiled procedural" loop — with the correct safety inversion the literature lacks:
**advisory signals inform, deterministic artifacts + humans decide** (see `META-CONTROLLER.md`
immutable core). The pieces the literature has and this repo does *not* are all in the
**scoring / decay / supersession** layer — not in the pipeline shape.

---

## Part 3 — Proposed design for THIS repo (minimal, file-based, zero-new-dep)

Design constraint honoured: **no vector DB, no embedding store, no new heavy dependency.** The store
is dozens of items; fuzzy similarity is not the bottleneck — **scoring, decay, and supersession**
are. All additions are cheap text metadata + meta-controller (Node ESM, existing deps) read-only
scoring. The advisory-vs-authority inversion is preserved end-to-end.

### 3.1 The four tiers → concrete stores

| Tier | Concrete store(s) | Status |
|---|---|---|
| **Working / short-term** | Session context window; `.claude/logs/harness-events.jsonl` (event stream); the `pre-edit-lessons` inject = paging-in. *Optional new*: a per-run "core-memory" scratch note the lead writes for salient in-flight facts (MemGPT core-memory analog) — but the existing IN-FLIGHT `MEMORY.md` entries already serve this. | **EXISTS** (core-memory note = optional, likely YAGNI) |
| **Episodic** | `docs/reflections/{INBOX,ARCHIVE,RETRO}/*` | **EXISTS** |
| **Semantic** | `MEMORY.md` + `memory/*.md` (facts) + `REGRESSION-LEDGER.md` (the fact "class X recurs") | **EXISTS** |
| **Procedural** | `docs/lessons/*` + `INDEX.md` (advisory IF-THEN); guardrails / eslint-local rules / boot-guards / hooks (authority IF-THEN = compiled); `loops/*.yaml` + `.claude/skills/` (skills) | **EXISTS** |

The tier scaffolding is **already complete**. The design is therefore **additive scoring/decay**,
not new stores.

### 3.2 Consolidation rules (episodic → semantic → procedural)

Keep the existing pipeline (§Part 2). Make two things explicit:

- **NEW (doc only):** name the promotion ladder in one place — reflection (episodic) → memory row
  (semantic) → lesson (procedural-advisory) → guardrail/skill (procedural-authority) — and state the
  bar for each hop (already lived in `reflections/README.md`, `skill-evolution`, the ledger process
  rule; just not co-located). This is the "compile a reflection into a production" (SOAR-chunking)
  rule made legible.
- **EXISTS:** the Council roster (cause/pattern/ratchet critics) + librarian executor already
  perform merge (pattern-critic cross-reflection root) and eviction (INBOX→ARCHIVE + store prune).

### 3.3 Retrieval scoring (recency · relevance · importance)

Adopt the Generative-Agents three-factor score, **file-cheap**, applied where retrieval already
happens — the **lessons INDEX** (and optionally the memory index):

- **relevance** — already computed (TRIGGER glob / error-signature match). Keep as the primary gate.
- **importance** — add one token per lesson/guardrail: red-line weight (auth/money/RLS/migrations =
  high) vs cosmetic (low). Poignancy, assigned at creation, exactly like §1.3.
- **recency / reuse** — derive from `.claude/logs/harness-events.jsonl`: count `pre-edit-lessons →
  inject` events per lesson and the last-inject timestamp. This is the **reinforcement-on-reuse**
  signal (§1.6.1) and needs **no new store** — the telemetry already logs every inject.
- **use:** when **>1 lesson matches** the same edit, rank by `importance·w1 + recency·w2 +
  reuse·w3` and inject the top-k (the hook currently injects all matches unranked). Single-match
  behaviour is unchanged, so this is backward-safe.

**NEW** but tiny: an `importance:` field in the lesson frontmatter + a meta-controller scorer that
joins INDEX ↔ harness-events. **EXISTS:** relevance (glob) + the telemetry feed.

### 3.4 Decay / forgetting (pruning)

- **EXISTS:** reflections decay `INBOX → ARCHIVE`; `bias-to-prune` is a standing rule for lessons and
  skills; the ledger is monotonic (guardrails never decay — correct: a compiled production is
  permanent, per SOAR one-shot chunking + the ratchet invariant).
- **NEW (advisory only):** a meta-controller gap `STALE_LESSON` (sibling of the existing
  `STALE_TELEMETRY`) — a lesson with **zero injects in N days** and a low importance weight is
  surfaced as a *prune candidate*. It **flags; it never deletes** (advisory signal; librarian/human
  prune). Guardrails are explicitly **exempt from decay** (monotonic ratchet).

### 3.5 Reinforcement (reuse strengthens; contradiction demotes)

- **Reuse strengthens (ACT-R):** the inject-count from telemetry *is* the strength signal. A lesson
  that fires often is retained and ranks higher; a lesson that fires often **and** correlates with a
  later ledger row for that path is a **promotion candidate** (advisory → authority) — which is
  exactly the librarian's existing job, now **data-driven** instead of memory-judged.
- **Contradiction demotes (Zep supersession, file-form):** add a `STATUS: superseded-by <link>`
  marker to lessons and semantic memory rows instead of silent deletion (auditable, git-native
  temporal edge). A meta-controller `CONTRADICTION` gap can flag two active facts/lessons that
  assert opposite things (e.g. OTP-disabled vs OTP-enabled; schema column renamed) for human
  resolution. **NEW**, small, and *demote-not-delete* keeps the audit trail the ledger culture
  prizes.

### 3.6 How the meta-controller ingests/scores the tiers

The controller is the natural home — it already ingests reflections + ledger + telemetry +
skill-drafts and **writes nothing** on `report` (§META-CONTROLLER.md). Additions are **read-only,
advisory, and land as new gap types** (never new authority):

- **EXISTS:** `UNRATCHETED_REFLECTION`, `PENDING_LEDGER_PROOF`, `STALE_TELEMETRY`, `SKILL_DRAFT`.
- **NEW gap types (report-only):** `STALE_LESSON` (decay candidate), `CONTRADICTION` (supersession),
  `UNSCORED_MEMORY` (a semantic file with no importance/recency to rank by). Each is a *signal*; a
  human + the gate decide, exactly like every existing gap. The controller may **propose** an inert
  scoring/prune draft under `meta-proposals/`; it may **never** auto-prune, auto-demote, or edit the
  `pre-edit-lessons` authority hook.

### 3.7 Exists vs new — at a glance

- **EXISTS (no work):** all four tiers as stores; the episodic→semantic→procedural consolidation
  pipeline; Council merge/eviction; the ratchet (compiled procedural = guardrail); INBOX→ARCHIVE
  decay; the telemetry feed; the meta-controller ingest+gap+propose loop; advisory-vs-authority
  inversion.
- **NEW (all additive, all cheap):** (1) three-factor **retrieval scoring** on lessons (importance
  field + telemetry-derived recency/reuse + rank-when-multiple); (2) **decay flagging**
  (`STALE_LESSON` gap, advisory); (3) **supersession markers** (`STATUS: superseded-by` +
  `CONTRADICTION` gap, demote-not-delete); (4) one **doc** co-locating the promotion ladder.

### 3.8 The one seam that is NOT additive-safe (must stay gated)

Everything above is safe **as long as it stays advisory**. Two operations would cross into authority
and must remain human/gate-decided:
1. **Changing the `pre-edit-lessons` hook's injection logic** (ranking/top-k) — it is an authority
   hook adjacent to the immutable core; a ranking change that dropped a red-line lesson is a safety
   regression. Ship ranking behind the hook as a *scored INDEX the hook reads*, not as new hook
   logic, or gate the hook change through council.
2. **Auto-pruning or auto-demoting** any lesson/guardrail. The monotonic ratchet ("never weaken a
   gate") is a red-line invariant; decay must **flag**, a human/librarian must **cut**. Auto-cut is
   forbidden.

---

## Executive summary (≤12 lines)

The harness already **is** a file-based four-tier cognitive-memory system; it lacks only *scoring,
decay, and supersession*. Tier → store mapping:

| Tier | Store (exists today) |
|---|---|
| Working/short-term | session context + `.claude/logs/harness-events.jsonl` (+ MEMORY.md IN-FLIGHT notes) |
| Episodic | `docs/reflections/{INBOX,ARCHIVE,RETRO}/` |
| Semantic | `MEMORY.md` + `memory/*.md` + `REGRESSION-LEDGER.md` |
| Procedural | `docs/lessons/*`+`INDEX.md` (advisory) · guardrails/eslint/hooks (authority) · `loops/*`+`.claude/skills/` |

**Top 3 gaps to close:** (1) **retrieval scoring** — lessons are relevance-only (binary glob); add
importance + telemetry-derived recency/reuse to rank when >1 matches (Generative-Agents three-factor,
file-cheap); (2) **decay/reinforcement telemetry** — nothing counts lesson-injects or guardrail-fires,
so no reuse-strengthens/stale-prunes signal (add `STALE_LESSON` advisory gap off the existing
harness-events feed); (3) **supersession** — superseded facts/lessons silently coexist; add
`STATUS: superseded-by` + a `CONTRADICTION` gap (demote-not-delete, Zep-style, file-form).

**Verdict:** **Additive-safe** — all four tiers and the episodic→semantic→procedural pipeline already
exist; the proposal adds only read-only scoring metadata + advisory meta-controller gaps. **No full
council required for the scoring/decay/supersession-flagging layer.** *One seam does require a
council*: any change to the `pre-edit-lessons` authority hook's injection logic, or any *auto*-prune/
*auto*-demote of a lesson or guardrail (monotonic-ratchet red-line) — those stay human/gate-decided.
