# Unified Tech Comparison — 2026-07-15

> One document, everything researched this session: named tools/libraries (metrics-based), the 18
> database patterns, AWS services, Terraform, and the AI/LLM methodology-concept clusters —
> compared for openbebop (Rust/WASM sovereign core) and dowiz (TS/React SPA + Rust kernel).
> Engineering fit only — industry popularity is not a factor anywhere in this document. Full
> sourcing for Part I is in RESEARCH-CONSPECT.md; this file adds Parts II-V, which weren't given
> item-by-item treatment before.

---

## Part I — Named tools/libraries (full sourcing: RESEARCH-CONSPECT.md, BLUEPRINTS.md)

| Item | Verdict | One-line reason |
|---|---|---|
| Harmonic centrality | **Tier 1 — build now** | Extends `spectral.rs`'s existing Laplacian eigenmodes directly, near-zero cost |
| Vectorless RAG (docs corpus) | **Tier 1 — build now** | Zero embedding cost for an already-structured corpus |
| Hydra (authorized staging test) | **Tier 1 — build now** | Proves OPS-12 rate-limiting holds under its own 110x-parallelism benchmark |
| Gamma function | Tier 2 — conditional | Only if `empirical_identify` needs credible intervals, not point estimates |
| Gaussian Splatting (`web-splat`/`brush`) | Tier 2 — conditional | Real Rust/wgpu fit, but WebGPU is Chrome-only today; no content pipeline exists |
| Mesh-LLM | Tier 2 — conditional | Best-fit port adapter IF LLM-agent infra is built; pre-1.0/experimental |
| Netclode | Tier 2 — conditional | Aligned with zero-OCI direction; single-maintainer, license unconfirmed |
| Skylos | Tier 2 — conditional | Needs a bake-off vs existing ESLint/`ts-prune` before adding |
| Born rule | Tier 3 — no hook | Kernel is classical Pearl-SCM, zero quantum substrate anywhere |
| Wronskian | Tier 3 — no hook | Real math, but redundant with existing rank/determinant checks |
| CuPy | Tier 3 — no hook | Not Rust-integrable; kernel's N≤32 eigensolve is below GPU break-even |
| AirLLM | Tier 3 — no hook | Python-only, no serving mode, 10-40min/200tok unquantized (3rd-party) |
| Omni-route | Tier 3 — no hook | Large irrelevant dev-tool surface vs a minimal custom router |
| OpenInterpreter (both) | Tier 3 — no hook | Current=agent not library; classic=deprecated+AGPL |
| OpenAlice | Tier 3 — no hook | Trading-agent domain, zero overlap beyond one abstract pattern |
| VeRa | Tier 3 — no hook | Only relevant if fine-tuning an LLM locally (neither codebase does) |
| Webscope | Tier 3 — no hook | Dev tooling, not a runtime dependency |
| Octorender | Dropped | Does not exist (confirmed via GitHub/crates.io/npm search) |
| Afaan/mc | Dropped | Unidentified after real search effort |
| Excluded outright | N/A | njRat, AsyncRAT, SpyNote, 888 RAT, GhostShell, hexsec-rat, KittySploit — no legitimate integration story |

---

## Part II — Database patterns (all 18, individually)

| # | Pattern | Status for openbebop/dowiz |
|---|---------|------------------------------|
| 1 | Normalization | Baseline RDBMS practice — already the working assumption for the pgrust schema; not a decision point |
| 2 | Denormalization | Not currently needed — no read-latency problem identified that normalization is causing |
| 3 | Indexing | Standard practice, already implicit in the pgrust migration's row-verify/COMPAT-GATE work; not a novel adoption |
| 4 | Polyglot Persistence | **Explicitly rejected by standing decision** — the whole ops-reliability arc consolidates onto ONE store (pgrust) specifically to reduce operational surfaces; adding a second DB type would reverse that |
| 5 | Sharding | Not applicable at current single-Hetzner-box scale; would only matter post-scale-out, not scoped |
| 6 | Replication | Relevant only once a second node exists — ops-reliability blueprint explicitly notes "no 2nd node → cold-boot runbook instead," i.e. replication is deliberately deferred, not adopted |
| 7 | Caching | Already flagged as "the only gap" in the ecosystem-strategy arc (per project memory) — real, unaddressed need, but a memory-cache decision, not something this doc's tool research resolves |
| 8 | Connection Pooling | **Already planned** — PgBouncer is in the ops-reliability latency stack (OPS-20) |
| 9 | CQRS | **Partially already implemented** — read/write separation exists at the port boundary (adapters read via scoped projections, never write to kernel state directly) |
| 10 | Database-per-Service | Not applicable — this is a single-core, port-based architecture by design (kernel-per-service isn't the model; ports-around-one-core is) |
| 11 | Shared Database | Not applicable / not a risk — no multi-service sprawl exists to accidentally share a DB across |
| 12 | Saga Pattern | No current distributed-transaction need identified — the event-sourced core's single-writer model avoids the cross-service-transaction problem Sagas solve |
| 13 | **Outbox Pattern** | **Real candidate** — directly relevant to the event-sourced core's write path (atomic DB-write + event-publish); see BLUEPRINTS.md scope |
| 14 | **Change Data Capture** | **Real candidate** — a mechanism option for the mesh's reactive Mesh→Agent propagation, alternative to hand-rolled event emission |
| 15 | **Event Sourcing** | **Already implemented** — openbebop's core is event-sourced by design, not a proposal |
| 16 | **Write-Ahead Logging** | **Verification item, not adoption** — pgrust's WAL compatibility is explicitly flagged UNVERIFIED in the existing ops-reliability blueprint; this is a real open risk to close, not a new pattern to add |
| 17 | Dead Letter Queue | No queue-based async processing identified yet that would need one; relevant only once one exists |
| 18 | API Composition | Partially already the port-adapter model's job (aggregating scoped reads across ports) — not a gap |

**Net new from this list: Outbox Pattern + CDC.** Everything else is either already implemented, explicitly deferred by a standing decision, or not applicable at current scale.

---

## Part III — AWS services (each, with the Hetzner-native equivalent already in the plan)

> Headline finding unchanged: adopting AWS conflicts with the ops-reliability arc's already-recorded
> decision to consolidate OFF managed cloud onto one Hetzner box. Listed individually below so the
> comparison is complete, not just a blanket dismissal.

| AWS service | What it does | Already-planned Hetzner-native equivalent |
|---|---|---|
| EC2 | Virtual compute | The Hetzner box itself (already the target, not a gap) |
| Lambda | Serverless functions | Not replicated — no serverless-functions need identified; would be new scope, not a swap |
| S3 | Object storage | Backup topology (OPS-14, 3-2-1-1-0) already targets self-hosted/rsync.net object storage, not S3 |
| RDS | Managed SQL | pgrust on the Hetzner box — this IS the entire point of the migration (drop managed DB hosting) |
| DynamoDB | Managed NoSQL | Not applicable — no NoSQL workload identified; pgrust/Postgres covers current needs |
| VPC | Private networking | Cloudflare Tunnel (origin-hiding, already deployed this session) covers the equivalent isolation need without a cloud VPC |
| IAM | Access management | Capability-based (macaroon/biscuit) model already the standing architecture — arguably stronger-grained than IAM, already decided |
| CloudFront | CDN | Cloudflare already fills this role (already the standing edge provider) |
| API Gateway | Managed API front door | Cloudflare + the port-adapter pattern already cover this |
| CloudWatch | Logs/monitoring | This is exactly OPS-07/08 (VictoriaMetrics+VictoriaLogs+Grafana) — already blueprinted, self-hosted |
| SNS | Notifications | Telegram-first notification stack (already built this session — `ops-gatus`, `plane-telemetry`) covers this |
| SQS | Message queues | No queue-based workload identified yet; would be new scope if ever needed, not a gap today |

**Conclusion:** every AWS service on the list either has an already-decided self-hosted/Cloudflare equivalent in motion, or has no current workload need. None are recommended.

---

## Part IV — Terraform / IaC

Already resolved: the ops-reliability blueprint's OPS-18 already specifies **OpenTofu**, the
functionally-identical open-source fork (same HCL, same provider ecosystem, same state/plan/apply
workflow) adopted after Terraform's 2023 BUSL license change. Zero speed/optimization delta between
them at the tool level — this is a license/governance choice, already made. The Terraform
workflow/command material (`init`/`plan`/`apply`/`destroy`, remote state + locking via S3+DynamoDB
equivalent, `count`/`for_each`, dynamic blocks, CI/CD integration) applies identically to OpenTofu —
nothing in that material changes the recommendation; it's a vocabulary match, not new information
that reopens the decision.

---

## Part V — AI/LLM methodology concepts

> These are working methodologies, not benchmarkable software — there is no "speed" metric for a
> prioritization framework. Compared instead on: already-practiced-here vs. a genuine candidate vs.
> not applicable. Stated plainly where that's the honest answer, rather than manufacturing a
> performance comparison that doesn't exist for a methodology.

**Vectorless RAG** — covered in Part I as a Tier-1 buildable item (the one concept in this cluster with a concrete implementation path).

**30 LLM foundational concepts / XAI (9 techniques) / 17 AI-system-architecture concepts** — these are baseline literacy, not decision points requiring comparison; the pieces with an actual architecture decision attached are already covered elsewhere in this doc (AI Gateway/Model Routing → Part I's Mesh-LLM/Omni-route entries; Semantic Caching → Part II's Caching row; Guardrails/Grounding → already implemented as the harness's Verified-by-Math gate and reflection/verification loop; the rest — tokens, embeddings, RLHF, feature-importance, saliency maps — are conceptual vocabulary with no adoption decision to make).

**Prompt-engineering techniques (9)** — of these, **Reflection/Verification is already a working guardrail** here (doubt-escalation + Verified-by-Math + the reflection pipeline), not a new technique to adopt. Chain-of-Thought/Tree-of-Thought are inference-time prompting choices made per-task by whoever is driving an agent, not something a codebase "adopts." No infrastructure decision follows from this list.

**Prioritization frameworks (9: Eisenhower, ICE, OODA, 80/20, 5 Whys, 10/10/10, SWOT, Rule of 40/70)** — process tools for humans making planning decisions, not software with a technical-fit dimension. No comparison on speed/optimization is possible or honest to fabricate here; noting for completeness only, per the same standard applied to the rest of this document (no invented metrics).

**Claude/Fable-5 operating practice + 12 Claude-usage ideas** — describes how to work with a coding agent (autonomous loops, memory files, parallel subtasks), which is the operating model this very session followed (background research agents, the memory system, task tracking) — already in active use, not a pending adoption.

**Bidirectional real-time mesh (Pub/Sub + CRDT + reactive consumers)** — already covered: this validates rather than proposes an alternative to the already-scoped bebop2-mesh / Living-Memory-as-pgrust direction. Where it's concretely different (NATS JetStream / Apache Pulsar as the specific transport) is the one open, comparable technical question — but that comparison belongs inside the mesh arc's own design work, not this document, since it depends on decisions (single-node vs multi-node timeline) already tracked there.

---

## Bottom line

Across every category researched — named libraries, database patterns, AWS services, IaC tooling,
and AI/LLM methodology — the same three outcomes recur: **(a)** a small number of genuinely new,
low-cost items with a concrete technical hook into existing code (harmonic centrality, Vectorless
RAG, Outbox+CDC, authorized Hydra testing), **(b)** a larger set already implemented or already
decided that this research confirms rather than reopens, and **(c)** items with no current
algorithmic or architectural hook, each with a specific cited reason rather than a popularity
judgment. Nothing in this document overrides Part I's PLAN.md tiering — it extends the same
methodology to the material that wasn't given item-by-item treatment the first time.
