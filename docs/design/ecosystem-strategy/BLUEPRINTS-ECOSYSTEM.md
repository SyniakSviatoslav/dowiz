# dowiz / bebop2 — Ecosystem Strategy — BLUEPRINTS

> **Дата:** 2026-07-13 · Супроводжує [ECOSYSTEM-STRATEGY-PLAN.md](./ECOSYSTEM-STRATEGY-PLAN.md).
> 20 одиниць EC-01..20, згруповані за **трьома китами** (Ядро · Інфраструктура · Потоки) + Екосистема.
> Кожна: **Мета · Межа (чіпаємо / НЕ чіпаємо) · Форма · Джерело-ідеї (тріаж) · RED-контракт · Кит.**
> НЕ код — блюпринт. Ядро недоторкане; інфра власна; потоки capability-gated. Усе адитивно.
>
> 🔴 = red-line (capability/crypto/money/agent-authority → human gate).

---

## КИТ 1 — ЯДРО (Core): мале, вічне, домен-агностичне

### EC-01 · Kernel-as-generic-substrate (зняти словник, не субстрат)
- **Мета:** зафіксувати kernel як домен-агностичний entity/event decide/fold субстрат, придатний для
  БАГАТЬОХ продуктів, не тільки delivery.
- **Межа:** ЧІПАЄМО — документуємо generic-контракт (Entity, Event, Decider, Projection, Matcher-score).
  НЕ ЧІПАЄМО — наявні order_machine/money/analytics (re-consume, не rewrite).
- **Форма:** kernel вже entity/event-separated (`domain` Order + `order_machine` transitions + `analytics`
  projection). Новий продукт = новий event-словник + нова fold-projection + нова matcher-scoring-fn над
  **тим самим** decide/fold. Доказ домен-загальності: `zvtvz/zvt` (quant) незалежно збігся на
  TradableEntity+event-schemas+pluggable-recorders+one-code-path-backtest/live.
- **Джерело:** zvt/TradingAgents = INSPIRE (domain-generality proof, не код).
- **RED:** додати другий event-словник (напр. `parcel.*`) → ті самі decide/fold/projection примітиви
  приймають його без зміни kernel-core; assert kernel-crate diff = 0 рядків логіки.
- **Кит:** Ядро.

### EC-02 · Load-bearing patterns як властивості ядра (не сервіси)
- **Мета:** реалізувати idempotency/CQRS/saga/gate як **властивості event-log+kernel**, не як окремі
  завжди-доступні сервіси.
- **Межа:** ЧІПАЄМО — специфікацію event-ID + projection-contract. НЕ ЧІПАЄМО — decide/fold чистоту.
- **Форма:** (1) **Idempotency** = event-ID `hash(prev, actor_pubkey, actor_seq, payload)`, fold skips
  applied → log-є-dedup-store-no-TTL. (2) **CQRS** = decide(write single-append)/fold(N read-projections),
  нуль-мережі-між → нуль-replication-lag. (3) **Saga=choreography** — cross-actor = capability-gated
  event-chain, компенсації first-class event-types, orchestrator-заборонений. (4) **Capability-gate =
  api-gateway+rate-limiter+mesh-mTLS в одному** offline-verifiable примітиві.
- **Джерело:** system-design-primer = INSPIRE (vocabulary/CAP=AP не solutions); patterns-lane RE-INTERPRET.
- **RED:** доставити ту саму подію двома gossip-шляхами → fold-ефект рівно один (idempotency); money-запис
  через write-behind-cache → тест FAILS (заборонено).
- **Кит:** Ядро. 🔴 (money invariant)

---

## КИТ 2 — ІНФРАСТРУКТУРА (Infrastructure): власна, багатократна

### EC-03 · Own inference engine (llama.cpp, in-process, capability-port)
- **Мета:** власний AI-inference без зовнішнього SaaS/daemon, лінкований у процес, деградує GPU→edge.
- **Межа:** ЧІПАЄМО — новий `inference` crate (адаптер порту). НЕ ЧІПАЄМО — kernel (агенти/inference за
  capability-межею).
- **Форма:** `llama-cpp-2` Rust-binding до `libllama` (plain-C-API) — mmap GGUF, quant Q8_0→IQ2, multi-
  backend CPU/CUDA/Metal/Vulkan з одного джерела, per-slot continuous-batching + auto-prefix-reuse, **нуль
  network-hop**. За capability-scoped портом (`Inference·Generate`). ollama = **dev-only wrapper**, ніколи
  prod. vLLM PagedAttention/prefix-cache = clone-blueprint ТІЛЬКИ якщо GPU-hub пізніше.
- **Джерело:** llama.cpp = INTEGRATE-DIRECTLY; ollama/vLLM/transformers/nanochat/deepseek = INSPIRE/CLONE.
- **RED:** inference-порт зі scope `Generate` намагається read event-log поза grant → deny; вимкнути мережу
  → inference працює (fully-offline); assert.
- **Кит:** Інфра.

### EC-04 · Own RAG (pgvector-on-pgrust, chunking + HNSW + RRF + rerank)
- **Мета:** власний retrieval, зв'язаний у той самий local-first store, що event-log — нуль зовнішнього
  vector-DB.
- **Межа:** ЧІПАЄМО — новий `rag` crate + pgvector на pgrust. НЕ ЧІПАЄМО — repowise (лишається code-RAG;
  це **product-RAG**, окремий).
- **Форма:** **Chunking** — llama_index-taxonomy (sentence-window default / hierarchical parent-child /
  semantic) + RAGFlow typed-templates. **Index** — pgvector HNSW (m/ef_construction/ef_search), дзеркалить
  `PGVectorStore`. **Retrieval** — BM25(Postgres-FTS)+dense, RRF-fused, cross-encoder-rerank. **Embeddings**
  — reuse local-Ollama `qwen3-embedding:0.6b`. Той самий backup/sync/trust-boundary, що event-log — free.
- **Джерело:** llama_index/open-webui/AnythingLLM/RAGFlow = CLONE (алгоритми, не runtime).
- **RED:** RAG-token намагається mutate → deny (read-only by cap-class); retrieval повертає тільки scoped-
  проєкцію (не PII поза grant); assert.
- **Кит:** Інфра.

### EC-05 · ★ П'ять кеш-шарів (ЄДИНИЙ реальний gap)
- **Мета:** закрити єдиний інфра-gap (інвентар: жодного кеш-шару в репо) — фундамент швидкості для всіх
  продуктів.
- **Межа:** ЧІПАЄМО — новий `cache` crate. НЕ ЧІПАЄМО — money (ніколи write-behind), decide/fold чистоту.
- **Форма:** (1) **Embedding-cache** content-hash-keyed таблиця pgrust (llama_index IngestionCache). (2)
  **Incremental re-index** Merkle-tree над event-log (claude-context) — re-embed тільки змінене. (3)
  **Prompt/prefix-cache** native KV-slot llama.cpp + DeepSeek-style content-addressed **disk-tier** для
  long-lived prefix. (4) **Retrieval/pipeline-cache** **ComfyUI ancestor-signature-hashed DAG-memoizer**
  прямо на decide/fold — invalidation = детермінований refold (не timer). (5) **Semantic-cache** опційний.
- **Джерело:** llama_index/claude-context/DeepSeek-MLA/ComfyUI/vLLM = CLONE (кеш-алгоритми).
- **RED:** новий event → derived-cache інвалідується refold'ом детерміновано (не за TTL); money-величина
  ніколи не читається з async-cache; flip cached-embedding-source-hash → cache-miss-refresh; assert усі три.
- **Кит:** Інфра. 🔴 (cache-coherence money invariant)

### EC-06 · Content-addressed chunker (casync) для mesh-sync
- **Мета:** нод, що переприєднується після офлайну, синхронізується chunked-diff'ами, не монолітним
  resend.
- **Межа:** ЧІПАЄМО — новий `chunker` crate. НЕ ЧІПАЄМО — event-log hash-chaining (chunk-hashes
  компонуються з ним).
- **Форма:** content-defined-chunking rolling-hash (Rabin/Buzhash), Merkle/prolly-tree структура; chunk-
  hashes компонуються з log-hash-chaining; для великих блобів (menu-photos, log-catchup). casync/rsync
  трансферуються майже незмінно.
- **Джерело:** build-your-own-x (BitTorrent/casync) = CLONE.
- **RED:** нод офлайн N-днів → re-sync тягне тільки змінені chunks (не весь log); assert bytes-transferred
  ≪ full-log.
- **Кит:** Інфра/Потоки.

### EC-07 · Living-memory upgrade (mem0-loop, mark-invalid-never-delete)
- **Мета:** підсилити living-memory extract→lookup→CRUD-петлею, узгодженою з ATTIC-move-never-delete.
- **Межа:** ЧІПАЄМО — memory-crate над pgrust. НЕ ЧІПАЄМО — advisory-статус (guardrail/human ратифікує).
- **Форма:** (1) **extract** local-inference-port/turn → atomic-facts; (2) **similarity-lookup** top-k
  pgrust; (3) **LLM-arbitrated-CRUD** ADD/UPDATE/DELETE/NOOP — **mark-invalid-never-hard-delete** (зберігає
  історію). Нуль mem0-runtime. Пам'ять інформує, guardrail вирішує.
- **Джерело:** mem0 = CLONE (extract→lookup→arbitrate); supermemory = INSPIRE (product-shape).
- **RED:** конфліктний факт → старий marked-invalid не hard-deleted (історія збережена); agent-«learning»
  тільки PROPOSES guardrail, deterministic-gate ратифікує; assert.
- **Кит:** Інфра/Потоки.

### EC-08 · Document ingestion port (markitdown → owned chunker)
- **Мета:** будь-який формат (PDF/DOCX/меню/контракт) → Markdown → owned-chunker → pgrust, за портом.
- **Межа:** ЧІПАЄМО — ingestion-порт-адаптер. НЕ ЧІПАЄМО — kernel; chunker owned (markitdown не chunk'ає).
- **Форма:** `markitdown` (MIT, bytes-in/Markdown-out, in-memory-sandboxable, opt-in-LLM-vision) за
  capability-scoped `Ingest·Document` портом → dowiz-owned typed-chunker → local-embed → pgvector-pgrust.
  Web-варіант = thin-Rust-port за firecrawl-рецептом (readability+robots+maxAge-cache+load-aware-limit),
  не 4-контейнерний сервіс.
- **Джерело:** markitdown = INTEGRATE-DIRECTLY; firecrawl/RAGFlow-DeepDoc/maxun = CLONE/INSPIRE.
- **RED:** ingestion-порт за scope `Document` намагається create_order → deny; malformed-PDF → graceful
  reject (не panic); assert.
- **Кит:** Потоки/Інфра.

### EC-09 · Build-your-own-x пріоритети (OWN vs ADOPT)
- **Мета:** явний список — що OWN у Rust (автономія), що ADOPT (не reinvent), що NON-GOAL.
- **Межа:** ЧІПАЄМО — рішення-реєстр (TOOLING-REGISTRY розширення). НЕ ЧІПАЄМО — наявні live-tools.
- **Форма:** **OWN:** gossip/replication-transport (crux, no-MQ-fits-offline), capability/token-verifier
  (є), content-addressed-chunker (EC-06), cache=fold-projection (EC-05). **ADOPT-not-reinvent:** audited-PQ-
  crypto (own integration не math), embedded-FTS/ANN-crate (не hand-roll HNSW-prod). **NON-GOAL:** managed
  LB/gateway/broker/cache-server. build-your-own-x = зрозуміти примітиви, не завжди ship bespoke.
- **Джерело:** build-your-own-x (Redis/Kafka/DB/TCP from-scratch) = INSPIRE (validates own-embedded-choices).
- **RED:** N/A (реєстр-рішення); guardrail = no-new-managed-service-dependency lint-rule (додати до eslint-
  plugin-local).
- **Кит:** Інфра.

### EC-10 · Dev-toolbelt standard (rg-discipline, TOOLBELT.md, skill-sync, curriculum)
- **Мета:** внутрішня velocity/reliability/token-economy через стандартний toolbelt, що компаундиться.
- **Межа:** ЧІПАЄМО — новий `TOOLBELT.md` біля CLAUDE.md + sync-скрипт. НЕ ЧІПАЄМО — house-style.
- **Форма:** rg-discipline (заборонити raw `grep -r`; Claude-Grep вже ripgrep-backed); `TOOLBELT.md`
  (Art-of-Command-Line + rg-rule) — кожна сесія+hire успадковує; **skill-sync** (steipete) byte-identical
  CLAUDE.md/skills across dowiz/bebop-repo/dowiz-pq worktrees (real drift-risk); auth-broker-skill pattern
  (awesome-claude-skills) single-mediated-auth. Curriculum: PIN 4 (System-Design-Primer/Developer-Roadmap/
  Art-of-Command-Line/YDKJS), ignore решту. gitignore 10-хв vendor.
- **Джерело:** rg/gitignore = ADOPT-DEV; steipete/karpathy-skills/Cursor = INSPIRE.
- **RED:** N/A (dev-process); metric = skill-drift-across-worktrees = 0 (sync-check у CI).
- **Кит:** Інфра (dev-velocity).

### EC-11 · Security-CI (HackAgent + iFixAI cross-check на agent-governance)
- **Мета:** red-team наших shipped-агентів із crypto-KAT-строгістю; independent cross-check на homegrown
  agent-governance.
- **Межа:** ЧІПАЄМО — новий CI-lane (isolated). НЕ ЧІПАЄМО — agent-governance (cross-checked, не замінений).
- **Форма:** **HackAgent** (Apache, local-first, SQLite, no-key) vs agent-governance + будь-яка LLM-exposed
  surface — prompt-injection/jailbreak/goal-hijack/tool-misuse; failures → feed drift-detector/error-
  pattern loop **advisory-never-sole-gate**. **iFixAI** periodic misalignment-diagnostic (32-45 tests,
  judged-by-different-model) — pilot, skim-judge-methodology-first. Ciphey = narrow IR-triage.
- **Джерело:** HackAgent = INTEGRATE-DIRECTLY (defensive); iFixAI = INSPIRE→pilot.
- **RED:** HackAgent знаходить injection у agent-governance surface → CI-lane RED (advisory), feeds ledger;
  assert red-path reachable (не false-green).
- **Кит:** Інфра (security-CI). 🔴 (agent-safety)

---

## КИТ 3 — ПОТОКИ (Flows): асинхронні, ідемпотентні, capability-gated

### EC-12 · Event-driven flows = gossip над log
- **Мета:** pub/sub без broker — log = topic, sync = p2p.
- **Межа:** ЧІПАЄМО — gossip/anti-entropy шар над proto-wire. НЕ ЧІПАЄМО — event-log immutability.
- **Форма:** log=topic; sync=p2p-gossip/QUIC (proto-wire iroh+WSS); consumer-offset=peer's-last-`actor_seq`;
  delivery=at-least-once+idempotent-fold; cap-scope=subscription-ACL evaluated-locally. Немає broker-IAM.
- **Джерело:** patterns-lane RE-INTERPRET (event-driven); build-your-own-x Kafka-lesson validates.
- **RED:** peer отримує подію двічі (два gossip-шляхи) → fold-ефект один; peer без scope не отримує подію
  (local-ACL); assert.
- **Кит:** Потоки.

### EC-13 · Saga-as-choreography + Outbox на швах портів
- **Мета:** cross-actor workflow без orchestrator; надійний виклик зовнішніх систем на межі портів.
- **Межа:** ЧІПАЄМО — compensation-event-types + outbox-relay. НЕ ЧІПАЄМО — kernel decide (порт не вирішує).
- **Форма:** cross-actor (order→claim→pickup→deliver→settle) = capability-gated event-chain, компенсації
  (`claim_released`/`order_cancelled`) first-class event-types. **Outbox** на КОЖНОМУ integration-port:
  append «intent-to-call-external» як подію в тій-самій-fold-txn що зміна-стану → relay/worker виконує →
  append «completion» подію (payment/SMS/webhook).
- **Джерело:** patterns-lane RE-INTERPRET (saga/outbox); integration-ports арка (кожен порт).
- **RED:** external-call падає mid-way → intent-event лишається, relay retry idempotent, стан консистентний
  (compensation якщо потрібно); assert no-partial-state.
- **Кит:** Потоки.

### EC-14 · Агенти = capability-HOLDERS (orchestrator-hierarchy + actor-messaging)
- **Мета:** агенти діють тільки в межах attenuated-capability; жодного ambient-authority.
- **Межа:** ЧІПАЄМО — orchestrator + actor-messaging crate (Rust). НЕ ЧІПАЄМО — kernel (агент = capability-
  holder, не privileged-caller).
- **Форма:** **CrewAI-shaped orchestrator** видає кожному агентові scoped-attenuated **SignedFrame**
  (звужений per-subtask, ніколи broadened; sub-agents talk only-to-orchestrator). **AutoGen-shaped** actor-
  messaging (addressable-agents, typed-messages, tracing) над SignedFrame. Кожна дія = decide/fold подія
  через kernel-log (auditable/replayable). Living-memory advisory.
- **Джерело:** CrewAI/AutoGen = CLONE (Rust, не Python-runtime); ruflo/mem0 = INSPIRE.
- **RED (🔴 BIGGEST TRAP):** агент діє без валідного SignedFrame / за межі scope → reject, стан незмінний;
  sub-agent намагається broaden delegated-scope → verify-fails; assert (ambient-trust неможливий by-
  construction).
- **Кит:** Потоки. 🔴 (agent-authority)

### EC-15 · Load-balancing = matcher; backpressure = pull anti-entropy
- **Мета:** розподіл роботи без LB/replica-fleet; flow-control без privileged-router.
- **Межа:** ЧІПАЄМО — anti-entropy pull-scheduler. НЕ ЧІПАЄМО — matcher determinism (crates/bebop).
- **Форма:** «load-balancing» = **детермінований matcher** (pure-fn, кожен нод рахує незалежно→same-answer,
  ближче до consistent/rendezvous-hashing; sync-fanout self-balances via gossip-peer-sampling).
  Backpressure inter-node = **pull-based anti-entropy self-throttle** (slower-peer requests only-next-N);
  intra-node = classic bounded-channel.
- **Джерело:** patterns-lane RE-INTERPRET (LB→matcher, backpressure→pull); matcher.rs вже є.
- **RED:** два ноди над тим самим станом → matcher дає ідентичний assignment (determinism); повільний peer
  не завалюється (pull-only-next-N); assert.
- **Кит:** Потоки.

---

## ЕКОСИСТЕМА: багато продуктів на спільному фундаменті

### EC-16 · Multi-product substrate (event-словник per-product)
- **Мета:** довести, що 2nd продукт = новий event-словник, не нова інфра.
- **Межа:** ЧІПАЄМО — новий product event-schema. НЕ ЧІПАЄМО — kernel/ports/PQ/RAG/cache (100% reuse).
- **Форма:** мапа найближчий-adjacent-першим: **dowiz Local** (пральня/ремонт/бакалія ~100% reuse, тільки
  vocab+field-UI-skin, нуль-нового-фундаменту) → **dowiz Fleet** (B2B last-mile +1-matching-port) →
  **dowiz Ledger** (settlement, TradingAgents-chain→actor-gate→pricing→settlement, +compliance-ports) →
  **dowiz Marketplace** (registry-as-product).
- **Джерело:** zvt/TradingAgents/public-apis = INSPIRE (domain-generality + discovery-economy).
- **RED:** dowiz-Local ships end-to-end через ті самі порти → kernel-crate logic-diff = 0; assert reuse
  empirically (не aspirationally).
- **Кит:** Екосистема (expands all 3).

### EC-17 · Capability-token marketplace (3 tiers, static-git, attenuate-not-issue)
- **Мета:** plugin/port-marketplace, gated capability-token'ами, жоден не дзвонить додому.
- **Межа:** ЧІПАЄМО — static-git-index + consent-mint. НЕ ЧІПАЄМО — capability-крипта (mint=SignedFrame
  вже є); KernelFacade-firewall незмінний.
- **Форма:** **🟢 Ready** static-git-native-index (public-apis-style, no-backend), кожен порт несе
  `Capability{scope:Resource×Action}`; install=**attenuate** не bearer-key. **🟡 Customizable**
  `marketplace.json` git-catalog, fork/параметризувати в межах scope-grammar. **🔧 Dev-agent** AI-агент
  (role-scoped) DRAFTS порт проти decide/fold-контракту, не може merge без human/gate. Capability>API-key:
  zero-round-trip-verify / attenuation / least-privilege / public-key-no-forge / survives-outage-p2p.
- **Джерело:** capability.rs вже є; Shopify/VS-Code/Stripe/MCP/wshobson = INSPIRE (narrow-surface+curation-
  moat); Multi-agent-marketplace = adopt SKILL.md-format не trust-model.
- **RED:** consent на `read order status` → minted-frame має РІВНО цей scope (не ширше); Revoke → наступний
  запит fails-verify mesh-wide; 3rd-party-plugin намагається edit-kernel → build-fails (KernelFacade);
  assert усі три.
- **Кит:** Екосистема. 🔴 (capability) · DEFER до 2nd-продукту.

### EC-18 · Foundation-first sequencing + YAGNI guard
- **Мета:** закласти load-bearing фундамент ЗАРАЗ, відкласти speculative-machinery.
- **Межа:** ЧІПАЄМО — roadmap-рішення. НЕ ЧІПАЄМО — нічого передчасно.
- **Форма:** **ЗАРАЗ** (cheap-now-ruinous-to-retrofit): 3-ворота+KernelFacade (є), scope-grammar-extension
  (near-zero-code), кеш-шари (EC-05), own-inference+RAG (EC-03/04), ship-1-2nd-product (EC-16). **DEFER**
  (не load-bearing поки немає 2nd-продукту): registry-UI, revenue-billing, dev-agent-tier, GPU-scale,
  federation-across-machines, owner-automation-canvas. Правило: AI-coding lowers-cost-of-later → YAGNI
  harder EXCEPT load-bearing-invariants.
- **Джерело:** YAGNI-2026 + platform-vs-product = INSPIRE; ecosystem-lane synthesis.
- **RED:** N/A (sequencing); guardrail = «no marketplace-machinery merge before 2nd-product ships» (doc-
  gate).
- **Кит:** Екосистема.

### EC-19 · Reliability re-home (attic patterns → capability-port boundaries)
- **Мета:** повернути наявні reliability-примітиви (health/rate-limit/circuit-breaker/k6) з attic як
  port-boundary-patterns.
- **Межа:** ЧІПАЄМО — re-home у port-adapter-layer. НЕ ЧІПАЄМО — decide/fold (circuit-breaker тільки на
  sync-external-port-boundaries, не intra-mesh).
- **Форма:** circuit-breaker → тільки на genuine-sync-external-ports (payment/SMS/non-mesh); rate-limit →
  quota-in-capability-token (attenuation, не central-count); health → per-node-local; k6 spike.js →
  reuse для port-load-testing. Intra-mesh «breaker» = gossip-reachability-heuristic (не state-path).
- **Джерело:** inventory (attic reliability.rs/health/rate-limit/circuit-breaker/spike.js) = REUSE;
  patterns-lane (circuit-breaker APPLIES-at-port-boundaries).
- **RED:** circuit-breaker trips на dead-payment-port → order-flow (cash) не блокується (degrade-closed);
  rate-limit-quota у token не central; assert.
- **Кит:** Інфра/Потоки.

### EC-20 · RED / proof-suite для всього плану
- **Мета:** довести, що три кити тримаються — кожна гарантія reachable red→green, нуль false-green.
- **Межа:** ЧІПАЄМО — новий test-крейт + CI-lane. НЕ ЧІПАЄМО — нічого продакшн.
- **Форма:** зібрати в один gate:
  - **Core-untouched** (EC-01/17) — адаптер/plugin імпортує kernel → build FAILS.
  - **Capability-isolation** (EC-03/04/14/17) — порт/агент за scope reads-out-of-grant → deny; attenuation-
    no-widen → verify-fails.
  - **Cache-coherence** (EC-05) — new-event→refold-invalidate детерміновано; money-never-write-behind.
  - **Determinism** (EC-01/15) — same-event→same-decision; ANN/HNSW ніколи в decide().
  - **Agent-authority** (EC-14) — агент без-SignedFrame → reject, стан незмінний.
  - **Idempotency** (EC-02/12) — duplicate-gossip → fold-ефект один.
- **RED:** кожен рядок має відомий стан, де падає до фіксу; жоден не `expect(true)`/skip/inflated-timeout;
  regression-ledger-row кожен.
- **Кит:** всі. 🔴

---

## Зведення: блюпринт → кит → хвиля

| BP | Назва | Кит | Хвиля | Red-line |
|---|---|---|---|---|
| EC-01 | Kernel-as-generic-substrate | Ядро | W0 | — |
| EC-02 | Patterns-as-kernel-properties | Ядро | W0 | 🔴 money |
| EC-03 | Own inference (llama.cpp) | Інфра | W2 | — |
| EC-04 | Own RAG (pgvector-pgrust) | Інфра | W3 | — |
| EC-05 | ★ П'ять кеш-шарів (gap) | Інфра | W1 | 🔴 coherence |
| EC-06 | Content-addressed chunker | Інфра/Потоки | W1 | — |
| EC-07 | Living-memory mem0-loop | Інфра/Потоки | W5 | — |
| EC-08 | Ingestion port (markitdown) | Потоки/Інфра | W4 | — |
| EC-09 | Build-your-own-x priorities | Інфра | W0 | — |
| EC-10 | Dev-toolbelt standard | Інфра | W6 | — |
| EC-11 | Security-CI (HackAgent) | Інфра | W6 | 🔴 agent-safety |
| EC-12 | Event-driven gossip | Потоки | W3 | — |
| EC-13 | Saga-choreography + outbox | Потоки | W4 | — |
| EC-14 | Agents=capability-holders | Потоки | W5 | 🔴 authority |
| EC-15 | LB=matcher, backpressure=pull | Потоки | W3 | — |
| EC-16 | Multi-product substrate | Екосистема | W7 | — |
| EC-17 | Capability-token marketplace | Екосистема | W8 | 🔴 · DEFER |
| EC-18 | Foundation-first sequencing | Екосистема | W0 | — |
| EC-19 | Reliability re-home | Інфра/Потоки | W6 | — |
| EC-20 | RED proof-suite | всі | W-RED | 🔴 |

**Інваріант усіх 20:** ядро мале-й-вічне (core-untouched build-proven); інфраструктура власна-й-
багатократна (own-inference+RAG+кеш, build-your-own-x, нуль-зовнішніх-залежностей); потоки асинхронні-
ідемпотентні-capability-gated (агенти=holders-не-callers). Екосистема = нові event-словники на спільному
фундаменті. Ambient-trust неможливий by-construction. Money ніколи не tween/write-behind. Усе — для людини.
