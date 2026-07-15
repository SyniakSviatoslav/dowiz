# Architecture-concept cluster — applied fit for openbebop/dowiz

> Scope: the AI/architecture concept material (Vectorless RAG, 30 LLM concepts, Claude/Fable-5
> operating practice, XAI, 17 AI-system-architecture concepts, prompt engineering, prioritization
> frameworks, AWS services, Terraform, 18 database patterns, bidirectional real-time mesh). These
> are patterns/methodologies, not GitHub-hosted tools — no benchmark numbers exist for "a pattern,"
> so fit is assessed as: already-implemented / net-new-with-concrete-benefit / conflicts-with-a-
> standing-decision. No adoption-popularity commentary, per instruction.

## Already implemented (no action — noted so the plan doesn't re-propose existing work)

- **Event Sourcing**: openbebop's core is event-sourced by design (not a candidate for adoption,
  it's the existing foundation).
- **CQRS-adjacent**: read/write separation exists at the port boundary (adapters never write to
  kernel state directly — reads flow through scoped projections per the integration-ports plan).
- **Reflection / verification loop** (the XAI/prompt-engineering "self-verification" pattern, and
  the "Reflection/Verification" prompting technique): already implemented as the harness's
  doubt-escalation + Verified-by-Math gate + reflection pipeline (docs/reflections/, ledger). Not a
  new concept to import — it's a working guardrail already.
- **Bidirectional real-time mesh (Pub/Sub + CRDT + reactive consumers)**: this is not new — it's a
  precise description of the already-scoped bebop2-mesh / Living-Memory-as-pgrust direction (CRDT
  merge, event-log replay, personalized-PageRank recall). The pasted architecture sketch validates
  the existing design rather than proposing an alternative; where it differs (NATS JetStream /
  Apache Pulsar as the event bus) is a concrete, evaluable choice — see BLUEPRINTS.md for the
  transport-layer comparison against what's already planned.

## Net-new, with concrete technical rationale (not "it's popular")

- **Vectorless RAG (structure-navigation over vector-similarity retrieval)**: directly applicable
  to this repo's own `docs/design/*`, `docs/adr/*`, `docs/governance/*` corpus — these are
  hierarchically structured documents (PLAN → BLUEPRINTS → RESEARCH-CONSPECT, consistently), which
  is exactly the case where structure-navigation outperforms embedding-similarity: it skips
  embedding-generation cost and vector-DB round-trip latency entirely for a corpus that already has
  a reliable navigational structure. Concrete fit: applies to the retrieval layer already being
  planned (trigram/BM25/HNSW/diffusion 4-layer) — could serve as a fifth, near-zero-latency layer
  specifically for structured docs, ahead of the more expensive layers.
- **Database patterns directly load-bearing for the pgrust migration already in flight**: of the 18
  patterns, the ones with concrete throughput/consistency payoff for that migration specifically:
  **Outbox Pattern** (atomicity between a DB write and an event publish — directly relevant to the
  event-sourced core's write path), **Write-Ahead Logging** (already how Postgres/pgrust works
  under the hood — worth flagging that pgrust's WAL compatibility is explicitly UNVERIFIED per the
  existing ops-reliability blueprint, a real open risk, not a pattern to "adopt" so much as verify),
  **Change Data Capture** (a candidate mechanism for the mesh's reactive Mesh→Agent propagation,
  alternative/complement to hand-rolled event emission), **Connection Pooling** (PgBouncer is
  already in the ops-reliability plan's latency stack — confirms rather than proposes).
- **AI Gateway / Model Routing / Semantic Caching** (from the 17-concept AI-systems list): only
  relevant if/once LLM-agent infra is actually built out; cross-reference the LLM-infra tool
  research (AirLLM/Mesh-LLM/Omni-route) for concrete routing-tool candidates rather than treating
  "model routing" as an abstract pattern to implement from scratch.

## Conflicts with an already-made decision (documented fact, not opinion)

- **AWS service list (EC2/Lambda/RDS/DynamoDB/VPC/CloudWatch/etc.)**: the ops-reliability plan
  already made an explicit, recorded decision to consolidate OFF managed clouds onto a single
  Hetzner box, with Cloudflare scoped to edge/hosting only ("Дроп Fly+Supabase" — drop Fly and
  Supabase, not add AWS). Introducing AWS services would add exactly the kind of managed-cloud
  dependency and cost surface that decision explicitly rejected. This is a direct conflict with
  existing architecture, not a matter of preference — flagging it as such rather than silently
  omitting AWS from the plan.
- **Terraform**: the ops-reliability blueprint (OPS-18) already specifies **OpenTofu**, not
  Terraform — OpenTofu is the Linux-Foundation-governed open-source fork after Terraform's 2023
  license change (BUSL), functionally near-identical at the HCL/provider level (same state model,
  same plan/apply workflow, same provider ecosystem via the OpenTofu registry). There is no
  speed/optimization delta between them — the engineering-merit-only answer is: the decision is
  already made (OpenTofu), and switching to Terraform would be a pure license/governance regression
  with zero performance upside. Not re-proposing Terraform adoption.

## Methodology-level material (no infra decision attached)

The remaining slide content — 30 LLM concepts, Claude/Fable-5 operating practice, 12 Claude use
cases, XAI explainability techniques, prompt-engineering techniques, prioritization frameworks —
are working methodologies rather than adoptable infrastructure. They don't have GitHub repos or
speed/memory benchmarks to compare, so they're out of scope for the metrics-only synthesis below;
noting them here for completeness rather than fabricating a "performance comparison" that doesn't
exist for a methodology.
