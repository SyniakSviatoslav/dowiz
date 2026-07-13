# Ecosystem Strategy — RESEARCH CONSPECT (7 lanes)

> Дата: 2026-07-13 · 7 паралельних дослідницьких смуг + інвентар власної інфри, зведені для
> [ECOSYSTEM-STRATEGY-PLAN.md](./ECOSYSTEM-STRATEGY-PLAN.md) + [BLUEPRINTS](./BLUEPRINTS-ECOSYSTEM.md).
> Каркас (оператор): «екосистема тримається на 3 китах — ядро, інфраструктура, потоки».

## Lane 1 — AI-agent frameworks (ПОТОКИ)
INTEGRATE вузько: Stagehand/browser-use (port-scoped web), HackAgent (CI red-team). CLONE: CrewAI (manager-
hierarchy = capability-ports shape), AutoGen (actor-core over SignedFrame). INSPIRE: LangChain→LangGraph
(=вже kernel), MetaGPT (SOP-as-code), aider (repo-map), ruflo (**federation-across-machines** нова ідея),
mem0 (memory-layering), steipete (skill-sync), awesome-claude-skills (auth-broker pattern), karpathy-skills.
CUT: Dify (multitenant-license dealbreaker), n8n (fair-code+ambient-authority), Langflow-as-dep. Multi-agent-
marketplace: adopt SKILL.md FORMAT не trust-model (36% skills had prompt-injection). BIGGEST TRAP = ambient-
trust orchestration (trust-in-install-act не crypto-scope = INVERSE of bebop2). Агенти = capability-HOLDERS.

## Lane 2 — Inference + RAG (ІНФРАСТРУКТУРА)
INFERENCE: OWN **llama.cpp** (llama-cpp-2 Rust, in-process, GGUF, GPU→edge — єдиний embeddable of-10).
ollama=dev-wrapper. vLLM PagedAttention/prefix-cache/continuous-batching = CLONE-if-GPU-hub. RAG: chunking
[llama_index-taxonomy sentence-window/hierarchical/semantic + RAGFlow typed-templates+human-correct], index
[**pgvector-on-pgrust HNSW** no-external-DB], retrieval [BM25+dense RRF + rerank]. ★CACHING (the-one-gap):
embedding-cache (IngestionCache), Merkle-incremental-reindex (claude-context), prefix/KV-disk (DeepSeek-MLA
principle), **ComfyUI ancestor-signature-hashed DAG-memoizer** on decide/fold, semantic-cache. mem0 CLONE
(extract→lookup→CRUD-arbitrate, mark-invalid-never-delete = living-memory). markitdown INTEGRATE (ingestion-
port). CUT: bumblebee/lobe-chat/supermemory-code/maigret/hosted-vector-DB.

## Lane 3 — Distributed patterns (★ ВСІ 3 КИТИ backbone)
Кожен патерн винайдено для privileged-network-center, якого меш не має → FUNCTION-real vs FORM-anti-pattern.
Load-bearing 5: (1) idempotency=event-ID-content-hash (log=dedup-store-no-TTL); (2) CQRS=decide/fold (no-
replication-lag); (3) saga=CHOREOGRAPHY (orchestrator=forbidden-authority, compensations first-class); (4)
capability-gate = api-gateway+rate-limiter+mesh-mTLS COLLAPSED-into-one offline-verifiable; (5) CRDT+outbox
at-2-seams (CRDT never-money, outbox at-port-boundaries). ANTI-PATTERNS: literal-LB/broker/cache-server/
orchestrator/control-plane (each=SPOF), CRDT-in-money, ANN-in-decide() (breaks determinism). Chunking=
APPLIES-AS-IS high-leverage (casync mesh-sync). CAP=AP. system-design-primer=vocabulary-not-solutions.
build-your-own-x=★own-infra (courier-phone IS node, managed-Redis/Kafka/ALB unavailable): validates decide/
fold+embedded-rusqlite+content-addressed-chunking.

## Lane 4 — Dev-tooling (ІНФРАСТРУКТУРА: dev-velocity)
rg=ADOPT (harness-discipline, filtering>speed). eza/bat=optional-human-sugar. Cursor=INSPIRE (autonomy-
slider+quality-over-volume-metrics). Octosuite=CUT-for-dev (OSINT). gitignore=ADOPT (10-min). Curriculum PIN
4: System-Design-Primer/Developer-Roadmap/Art-of-Command-Line/YDKJS; CUT interview-prep/freeCodeCamp/30-
seconds. Shell-sort=CS-trivia. iFixAI=INSPIRE→pilot (misalignment-diagnostic, judged-by-different-model).
Compounding: TOOLBELT.md + curated-reference-index + iFixAI-CI-cross-check.

## Lane 5 — Defensive security (ІНФРАСТРУКТУРА: security-CI)
Scope-check: dowiz NO-GraphQL (grep=0), agents-with-tool-access=real-surface. ONE belongs-in-CI = HackAgent
(vs agent-governance). graphw00f=N/A-shelve. Ciphey=narrow-IR-triage. 3 threat-classes: (1) OTP-interception
(sms-forwarder) → SMS=low-trust-bootstrap-not-bearer, capability-bind-to-device + WebAuthn; (2) OSINT-
profiling (maigret/Octosuite) → minimize-on-wire-metadata + per-actor-PQ-identity-unlinkability + self-audit-
own-only; (3) agent-manipulation → red-team-with-KAT-rigor. CUT: darkfly (offensive-bundle), sms-forwarder
(never-ship), semble/heromap/theisnospoon (unidentifiable), maigret (self-audit-only-never-customer).
Entropy mix-never-replace = root-of-trust, untouched.

## Lane 6 — Platform & ecosystem strategy (★ expands ALL 3 to multi-product)
THESIS: не food-app а local-first capability-scoped ledger-of-truth. Marketplace invariants (Shopify/VS-Code/
Salesforce/Stripe/MCP/wshobson): narrow-versioned-surface, revenue-share, power-law, curation=moat. ★
CAPABILITY-TOKEN > API-KEY: zero-round-trip-verify / attenuation / least-privilege / public-key-no-forge /
survives-outage-p2p (capability.rs ALREADY-BUILT = marketplace-primitive). PRODUCT-MAP: Delivery→Local(~100%
reuse)→Fleet(+matching-port)→Ledger(TradingAgents-chain→actor-gate→pricing→settlement)→Marketplace(meta).
3-tiers all-capability-gated none-call-home. SEQUENCING: NOW scope-grammar+cache+inference+RAG+ship-1-2nd-
product; DEFER registry-UI/billing/dev-agent-tier. RISK: local-first-moat=perceived-perf+protocol-trust not-
data-lock-in → durable-advantage=governance-quality only-real-once-2nd-independent-builder (AT-Proto-failure
vs ActivityPub-success). NO-COURIER-SCORING: decide CI-check-vs-protocol-spec-MUST before-3rd-parties.
zvt/TradingAgents/public-apis = domain-generality + discovery-economy proof.

## Lane 7 — Inventory: WHAT DOWIZ ALREADY OWNS (ground-truth)
★caching = THE ONE GENUINE GAP. Already-built: agent-governance (drift+error-learning WASM) + 15 subagent-
specs + hooks + living-memory (lessons/reflections/regressions) + eslint-plugin-local 17-rules; ★repowise
LIVE (LanceDB + local-Ollama qwen3-embedding code-RAG, MCP) = one-sanctioned-RAG (Mem0/RAGFlow deferred);
kernel Rust/WASM (order_machine/domain/money/analytics/intake, NO server-crate=DROPPED-D1 static-SPA);
bebop2 proto-cap (★capability.rs "signed {subject_key,scope,nonce,expiry} verifiable-by-any-peer-without-
central-issuer replaces-JWT")/hybrid_gate/roster/signed_frame, PQ from-scratch, rng.rs fail-closed, matcher.rs
deterministic; CI validate-job + gitleaks + backup-drill; reliability (health/rate-limit/circuit-breaker/k6
spike.js) in-attic pending-decentralization.

---
**Ground-truth anchors:** `kernel/src/{order_machine,domain,money,analytics,intake,wasm}.rs`; `agent-
governance/index.ts` + `agent-governance-wasm/`; `.repowise/` (LanceDB+local-Ollama); bebop2 `proto-cap/src/
{capability,scope,hybrid_gate,roster,signed_frame}.rs`, `core/src/{rng,pq_dsa,pq_kem,vsa}.rs`; `crates/bebop/
src/matcher.rs`; `proto-wire/src/{iroh_transport,wss_transport}.rs`; attic `apps-api/{health,rate-limit,
circuit-breaker}` + `load/spike.js`; `MEMORY-MAP.md`, `TOOLING-REGISTRY.md`.
