# dowiz / bebop2 — Ecosystem Strategy: Ядро · Інфраструктура · Потоки — PLAN

> **Дата:** 2026-07-13 · **Тип:** дослідження → тріаж → синтез → стратегічний план (НЕ код) ·
> **сьомий і фінальний план сьогодні.**
> **Стоїть на:** усіх 6 попередніх арках (hydraulic-loop → field-ui → dowiz-interfaces →
> rust-engine-rewrite → integration-ports) + kernel/bebop2 PQ + repowise + agent-governance +
> living-memory harness + pgrust + DOD/wgpu/egui.
> **Каркас (оператор, дослівно):** *«будь-яка екосистема тримається на 3 китах — ядро,
> інфраструктура, потоки».* Кожна з ~70 бібліотек і кожен патерн лягає рівно на один кит.
> **Мета:** не сервіс доставки їжі, а **цілісна екосистема багатьох продуктів на спільному
> фундаменті** — автономна інфраструктура без залежностей (build-your-own-x), закладена ЗАРАЗ у
> правильних місцях, не вся відразу.

---

## 0. Три кити — каркас усього плану

| Кит | Що це | Правило | Що сюди лягає |
|---|---|---|---|
| **1 · ЯДРО** (Core) | Незмінне, домен-агностичне серце: `kernel` decide/fold Law, integer money, append-only event-log (єдина спільна істина), capability-модель (SignedFrame/scope/AnchorRoster) | **Мале й вічне.** Ніколи не чіпається; всі продукти ділять без змін | патерни-як-властивості-ядра: idempotency, CQRS, event-sourcing; domain-generality (zvt/TradingAgents) |
| **2 · ІНФРАСТРУКТУРА** (Infrastructure) | Власний переюзабельний субстрат навколо ядра: own-inference, own-RAG, **кеш-шари (єдиний реальний gap)**, індексація, pgrust-сховище, dev-toolbelt, security-CI, marketplace | **Власна й багатократна.** Build-your-own-x, без зовнішніх залежностей | llama.cpp, pgvector-on-pgrust, chunking/HNSW/RRF, кешування, rg/toolbelt, HackAgent |
| **3 · ПОТОКИ** (Flows) | Рух через систему: event-driven комунікація, saga-choreography, agent-flows, ingestion-пайплайни, load-balancing по мешу, backpressure | **Асинхронні, ідемпотентні, capability-gated.** Продукт = нова конфігурація потоків над спільними ядром+інфрою | gossip, matcher, outbox, agents-as-capability-holders, markitdown→chunk→embed |

**Стратегічний тезис одним реченням:** dowiz — це не «застосунок доставки їжі», а **local-first,
capability-scoped ledger-of-truth** (детермінований kernel decide/fold над event-log), який будь-яка
кількість незалежно збудованих локально-акторних продуктів безпечно розширює через offline-verifiable
порти, ніколи не торкаючись і не псуючи код чи довіру одне одного.

---

## 1. ТРІАЖ — усі ~70 бібліотек/тем за один погляд

Легенда вердиктів: **INTEGRATE** (пряма залежність, адаптер порту) · **CLONE** (reverse-engineer,
чиста реалізація в Rust, нуль vendoring) · **INSPIRE** (взяти ідею, не код) · **ADOPT-DEV** (внутрішній
інструмент/CI, ніколи не в продукт) · **CUT** (не варте / несумісне / неідентифіковане) ·
**THREAT** (вивчити оборонно, не інтегрувати).

### 1.1 INTEGRATE-DIRECTLY (дуже мало — тільки вузьке, low-blast-radius, за портом)
| Item | Чому | Кит |
|---|---|---|
| **llama.cpp / ggml** (`llama-cpp-2` Rust crate) | ЄДИНИЙ inference-двигун із 10, що лінкується **в процес** Rust-бінарника, повністю офлайн, деградує GPU→Pi. MIT. Owned engine. | Інфра |
| **markitdown** (MS) | MIT, zero-external-dep, bytes-in/Markdown-out, in-memory-sandboxable → чистий doc-ingestion порт | Інфра/Потоки |
| **Stagehand** (Browserbase) | MIT, TS-native, кешує елементи (token-economy), вузький — тільки за capability-портом для «unauth 3rd-party web-portal» | Потоки |
| **browser-use** | те саме, Python; Stagehand кращий-fit | Потоки |
| **HackAgent** (AISecurityLab) | Apache, local-first, SQLite, no-key — red-team наших shipped-агентів у CI | Інфра (security-CI) |
| **github/gitignore** | 10-хв vendor/diff проти монорепо (`target/`/`*.wasm`/`.turbo`) | Інфра (dev) |
| **ripgrep (rg)** | harness-discipline: default-.gitignore/binary-skip = token-cost control; заборонити raw `grep -r` | Інфра (dev) |

### 1.2 CLONE / REVERSE-ENGINEER (вивчити глибоко, реалізувати чисто в Rust — build-your-own-x)
| Item | Що саме клонуємо | Кит |
|---|---|---|
| **vLLM** | PagedAttention (block-table KV) + hash-chain prefix-cache + continuous-batching (тільки якщо GPU-hub) | Інфра |
| **ComfyUI** | ancestor-signature-hashed **DAG-memoization** (recursive) — blueprint для pipeline-cache над decide/fold | Інфра (кеш) |
| **llama_index** | chunking-taxonomy + PGVectorStore/HNSW-pattern + RRF-fusion + IngestionCache | Інфра (RAG) |
| **mem0** | extract→similarity-lookup→LLM-arbitrated-CRUD, **mark-invalid-never-hard-delete** → living-memory | Інфра/Потоки |
| **claude-context** | Merkle-tree **incremental re-index** (re-embed тільки змінене) | Інфра (RAG) |
| **RAGFlow** (DeepDoc) | document-type-aware **typed chunk-templates** + human-correctable-UI | Інфра (RAG) |
| **AnythingLLM** | swappable **vector-store port trait** (embedded-default, no-API-key) | Інфра (RAG) |
| **open-webui** | hybrid BM25+dense+rerank + doc-hash-keyed embedding-cache | Інфра (RAG) |
| **firecrawl** | web-ingestion recipe: readability + robots.txt + maxAge-cache + load-aware-rate-limit (thin Rust port, не 4-контейнерний сервіс) | Потоки |
| **ollama** | model-lifecycle UX (registry/idle-unload-GC/prefix-cache) у native node-model-manager | Інфра |
| **CrewAI** | manager-enforced hierarchy (sub-agents talk only to orchestrator) = capability-ports shape | Потоки |
| **AutoGen** | actor-core (addressable agents, typed messages, tracing) над SignedFrame | Потоки |
| **DeepSeek MLA** | «стисни attention-state в low-rank → дешевий disk-cache → content-address by prefix-hash» — принцип кешу | Інфра (кеш) |
| **transformers** | Cache-trait taxonomy (Dynamic/Static/Quantized/Offloaded/SlidingWindow) як design-ref | Інфра |
| **casync/rsync** (build-your-own-x) | content-defined-chunking (rolling-hash, Merkle/prolly-tree) для mesh-sync великих блобів | Інфра/Потоки |

### 1.3 INSPIRE (ідея, не код) — включно з навчальними ресурсами
LangChain→LangGraph (explicit-graphs = вже kernel), Langflow (canvas для owner-automation LATER),
MetaGPT (SOP-as-code), aider (tree-sitter repo-map), ruflo (**federation-across-machines** — єдина нова
ідея), supermemory (product-shape single-local-binary), maxun (teach-by-demonstration recorder LATER),
nanochat (own-training-pipeline literacy), Cursor (autonomy-slider + quality-over-volume metrics),
steipete/agent-scripts (**skill-sync** across worktrees), awesome-claude-skills (**auth-broker-skill**
pattern), andrej-karpathy-skills (5-min diff vs our CLAUDE.md), iFixAI (misalignment-diagnostic pilot),
Ciphey (IR-triage narrow), Multi-agent-plugin-marketplace (adopt SKILL.md **format** не trust-model),
zvt/TradingAgents/public-apis (domain-generality proof, §5). **PIN-курикулум:** System-Design-Primer
(vocabulary+CAP не solutions), Developer-Roadmap, Art-of-Command-Line, You-Dont-Know-JS.

### 1.4 CUT (не варте / несумісне / неідентифіковане)
Dify (Apache+multitenant-restriction = dealbreaker бо ми multitenant-SaaS), n8n (Sustainable-Use-License
+ ambient-node-authority), lobe-chat (5-container SaaS, no innovation), bumblebee (Elixir/BEAM),
Octosuite (GitHub-OSINT, не dev-tool), 30-seconds-of-code (archived + superечить house-style),
Tech-Interview-Handbook/Coding-Interview-University/freeCodeCamp (wrong altitude для senior-PQ-team),
Shell-sort (CS-trivia, name-based false-positive), darkfly (unaudited offensive bundle), sms-forwarder
(OTP-interception mechanism — ніколи не ship), semble/heromap/theisnospoon (неідентифіковані — не тягнути
невідоме в PQ-trust-stack), maigret (dual-use mass-enumeration → self-audit-only, ніколи не customer),
будь-який Milvus/Pinecone/ES/hosted-vector-DB.

### 1.5 THREAT (вивчити оборонно) + ADOPT-DEV (внутрішній CI)
THREAT: maigret/Octosuite (OSINT-profiling → minimize on-wire metadata, per-actor-PQ-identity
unlinkability, self-audit-own-assets), sms-forwarder (OTP-interception → capability-not-bearer +
device-binding + WebAuthn upgrade). ADOPT-DEV: HackAgent (CI vs agent-governance), iFixAI (periodic
misalignment cross-check), rg (harness), gitignore.

---

## 2. КИТ 1 — ЯДРО: мале, вічне, домен-агностичне

### 2.1 Ядро вже є і вже домен-агностичне

`kernel/src` (dowiz-kernel Rust→WASM) — `order_machine` (decide/fold), `domain` (place_order/apply_event
Decider), `money` (integer, no-float RED-LINE), `analytics` (ChannelLedger read-projection +
reduce_anomalies), `intake` (admit). Це вже **entity/event separation + deterministic fold** — та сама
форма, до якої **незалежно прийшов** `zvtvz/zvt` (quant: TradableEntity + event-schemas + pluggable
recorders + one-code-path-backtest/live). Незв'язана команда в незв'язаному домені збіглася на
entity/event + pluggable-adapter = **доказ, що патерн домен-загальний**, не delivery-specific.
Генералізація вимагає зняти **словник**, не **субстрат**.

### 2.2 Патерни — як властивості ядра, не як окремі сервіси

Кожен «мікросервісний» патерн винайдено для **привілейованого мережевого центру**, якого меш не має.
FUNCTION (проблема-реальна) лишається; FORM (standalone-coordinating-service) = anti-pattern тут.
**Load-bearing 5:**

1. **Idempotency = event-ID content-hash** `hash(prev, actor_pubkey, actor_seq, payload)`; fold пропускає
   вже-застосоване → **log Є dedup-store, без TTL** (робить at-least-once gossip безпечним).
2. **CQRS = decide/fold** — `decide`=write single-writer-append, `fold`=N read-projections; **немає
   мережі між ними** → немає replication-lag, тільки sync-lag (власність gossip).
3. **Saga = choreography** (не orchestration) — cross-actor workflow (order→claim→pickup→deliver→settle) =
   ланцюг capability-gated подій, компенсації (`claim_released`) = first-class event-types. Orchestrator
   = привілейований вузол, що володіє чужими переходами = заборонено.
4. **Capability-gate = api-gateway + rate-limiter + service-mesh-mTLS, згорнуті в ОДИН offline-verifiable
   примітив.** SignedFrame/scope заміняє 3 централізовані турботи, перевіряється локально кожним нодом.
5. **CRDTs + outbox на 2 правильних швах** — CRDT для комутативної периферії (**ніколи money/state-
   machine** — «eventually-converges» ≠ «obeys-legal-transitions»); outbox там, де log говорить із
   реальним не-event-sourced зовнішнім (payment/SMS).

### 2.3 Пастки ядра (що НЕ робити)
- Імпортувати класичну **FORM** (окремий always-on сервіс) навіть «залишаючи тільки function»: буквальний
  LB / broker / cache-server / saga-orchestrator / mesh-control-plane — кожен реінтродукує SPOF/authority.
- **Domain-leakage**: eventual-consistency/CRDT у money/order-state (ламає інваріанти); недетерміновані
  структури (ANN-index, unordered-merge) у `decide()` (ламає «same-event→same-decision», від якого залежить
  matcher).

---

## 3. КИТ 2 — ІНФРАСТРУКТУРА: власна, багатократна, без залежностей

### 3.1 Власний inference-двигун
**OWN llama.cpp** через `llama-cpp-2` Rust-crate — єдиний embedded-engine, лінкується в процес (нуль
network-hop), mmap GGUF, quant Q8_0→IQ2, multi-backend CPU/CUDA/Metal/Vulkan з одного джерела, деградує
GPU-box→Pi-edge, **за capability-портом**. `ollama` = dev-convenience-wrapper (ніколи не shipped-prod —
це llama.cpp за daemon+HTTP-hop). vLLM (PagedAttention/prefix-cache/continuous-batching) = **blueprint для
native-reimplement ТІЛЬКИ якщо** пізніше multi-tenant-hub потребує GPU-scale — ніколи `pip install`.

### 3.2 Власний RAG (bind у pgrust, нуль зовнішнього vector-DB)
- **Chunking:** llama_index-taxonomy (sentence-window default / hierarchical parent-child для structured /
  semantic для long-form) + RAGFlow document-type-templates з human-correctable admin-UI (меню/контракти/
  dispute-evidence).
- **Index:** **pgvector на pgrust**, HNSW, дзеркалить llama_index `PGVectorStore` точно — **жодного
  зовнішнього vector-DB**. Ловиться в **той самий local-first store**, що event-log + living-memory =
  той самий backup/sync/trust-boundary безкоштовно.
- **Retrieval:** BM25 (Postgres FTS) + dense, злиті **RRF**, + cross-encoder **rerank**.
- **Embeddings:** локальні (repowise вже юзає local-Ollama `qwen3-embedding:0.6b` dim-1024) — reuse.

### 3.3 ★ Кешування — ЄДИНИЙ реальний gap (інвентар: жодного кеш-шару в репо)
П'ять шарів, кожен у правильному місці:
1. **Embedding-cache** — content-hash-keyed таблиця в pgrust (llama_index `IngestionCache`).
2. **Incremental re-index** — Merkle-tree над event-log (claude-context): re-embed **тільки змінене**.
3. **Prompt/prefix-cache** — native KV-slot reuse llama.cpp + **DeepSeek-style content-addressed disk-tier**
   для довгоживучих system-prompt/context-prefix (compress-to-low-rank→cheap-disk→content-address).
4. **Retrieval/pipeline-cache** — **ComfyUI ancestor-signature-hashed DAG-memoizer** прямо на kernel
   decide/fold (invalidation = детермінований refold, не timer → cache-incoherence-bugs зникають).
5. **Semantic-cache** — опційний, найнижчий пріоритет.

### 3.4 Living-memory upgrade (узгоджено з ATTIC-move-never-delete)
Клонувати mem0-петлю: **extract** (LLM/turn → atomic facts) → **similarity-lookup** (top-k у pgrust) →
**LLM-arbitrated-CRUD** ADD/UPDATE/DELETE/NOOP — але **mark-invalid-never-hard-delete** (зберігає
історію) = точно наш living-memory дух. Local-inference-port робить extraction/arbitration, pgrust тримає
embeddings, **нуль mem0-runtime**. Пам'ять лишається **advisory**; guardrail/human ратифікує.

### 3.5 Build-your-own-x пріоритети (що OWN у Rust vs ADOPT)
**OWN:** gossip/replication-transport (crux — жоден MQ не fits offline-first), capability/token-verifier
(security-kernel — вже є), content-addressed-chunker (casync-lesson, high-leverage), «cache»=fold-
projection (нуль Redis). **ADOPT-not-reinvent:** audited-PQ-crypto (own integration, не math — вже),
embedded-FTS/ANN-crate (не hand-roll HNSW для prod). **NON-GOAL:** managed LB/gateway/broker/cache-server
(немає fleet-to-front). build-your-own-x — щоб **зрозуміти** ці примітиви, не завжди щоб ship bespoke.

### 3.6 Dev-toolbelt + security-CI
`TOOLBELT.md` біля CLAUDE.md (rg-discipline + Art-of-Command-Line) — кожна сесія+hire успадковує.
Skill-sync (steipete) — тримати CLAUDE.md/skills byte-identical across dowiz/bebop-repo/dowiz-pq
worktrees (real drift-risk). **HackAgent** CI vs agent-governance (feed drift-detector, advisory-never-
sole-gate); **iFixAI** periodic misalignment cross-check. Curriculum: pin 4, ignore решту.

---

## 4. КИТ 3 — ПОТОКИ: асинхронні, ідемпотентні, capability-gated

### 4.1 Event-driven = gossip над log
Log = topic; sync = p2p-gossip/QUIC (не broker); consumer-offset = peer's last-`actor_seq`; delivery =
at-least-once + idempotent-fold; cap-scope = subscription-ACL evaluated-locally.

### 4.2 Saga-as-choreography + Outbox на швах портів
Cross-actor workflow = ланцюг capability-gated подій, компенсації first-class. **Outbox** re-enters
**на кожному integration-port**: append «intent-to-call-external» як подію в **тій самій fold-txn**, що
й зміна стану → relay/worker виконує виклик → append «completion» подію. Це правильна форма для **кожного**
порту з integration-ports арки.

### 4.3 Агенти = capability-HOLDERS, не привілейовані caller'и
CrewAI-shaped orchestrator видає кожному агентові **scoped-attenuated SignedFrame** (звужений per-subtask,
ніколи не broadened); кожна дія агента = decide/fold подія через kernel-log (auditable/replayable);
AutoGen-shaped actor-messaging над SignedFrame. Living-memory **строго advisory**. **BIGGEST TRAP =
ambient-trust orchestration** (кожен framework default install→broad-access; trust-in-install-act не
crypto-scope = INVERSE of bebop2 law). Кожна запозичена ідея — **за capability-межею** перед продуктом.

### 4.4 Load-balancing = matcher; backpressure = pull anti-entropy
Немає replica-fleet → «load balancing» = **детермінований matcher** (pure-fn, кожен нод рахує незалежно
→ same-answer, ближче до consistent/rendezvous-hashing). Backpressure inter-node = pull-based anti-entropy
self-throttle (slower-peer requests only next-N); intra-node = classic bounded-channel.

### 4.5 Ingestion-пайплайн (потік даних у пам'ять)
`markitdown` (bytes→Markdown, за портом) → **dowiz-owned chunker** (typed templates) → local-embed →
**pgvector-pgrust index** → hybrid-retrieval RRF+rerank. Web-ingestion = thin-Rust-port за firecrawl-
рецептом (readability+robots+maxAge-cache+load-aware-limit), не 4-контейнерний сервіс.

---

## 5. Екосистемна стратегія — багато продуктів на спільному фундаменті

### 5.1 Мапа продуктів (найближчий-adjacent першим, найменше нового фундаменту)
1. **dowiz Delivery** (baseline) — kernel + ports + PQ + field-UI.
2. **dowiz Local** (пральня/ремонт/бакалія/послуги) — ~100% reuse; тільки новий entity-vocab + field-UI
   skin. **Нуль нового фундаменту.**
3. **dowiz Fleet** (B2B last-mile, multi-venue) — reuse kernel + courier-device-sig-settlement +
   venue/claim-plumbing; **+1 matching/dispatch порт**.
4. **dowiz Ledger** (agent-governed settlement/reconciliation) — TradingAgents-ланцюг
   analyst→trader→risk **мапиться** на actor-gate→pricing→settlement; kernel має integer-money +
   PQ-signed-settlement; +compliance/market-data ports (найдальший).
5. **dowiz Marketplace** — сам capability-registry як продукт, проданий іншим командам на тому ж kernel =
   мета-продукт, що перетворює 1-4 на екосистему, а не 4 застосунки однієї компанії.

**Multi-product-ness = нові event-словники на спільній інфрі, не нова інфра на продукт.**

### 5.2 Marketplace — 3 рівні, усі capability-token-gated, жоден не дзвонить додому
- **🟢 Ready** — статичний git-native index (public-apis-style, **no-backend**), кожен порт несе
  `Capability{scope: Resource×Action}` дескриптор; «install» = **attenuate** capability, ніколи не
  bearer-key.
- **🟡 Customizable** — `marketplace.json` git-catalog (Claude-Code pattern); 3rd-parties fork/параметризують
  порт **у межах scope-grammar**; досі offline-verifiable, не може перевищити `{resource,action}`.
- **🔧 Dev-agent** — AI-агент (сам role-scoped через agent-governance) тримає attenuated-cap, **DRAFTS**
  порт проти фіксованого decide/fold-контракту, але **не може merge** без human/deterministic-gate.

KernelFacade-firewall робить «агенти/3rd-parties тільки mint/consume capabilities, ніколи не edit kernel-
source» **build-time факт**, не політику — те, чого **жоден API-key marketplace** (Shopify/SF/Stripe) не
може дати. Capability-token над API-key: (a) zero-round-trip peer-verify (local-first); (b) attenuation
structurally; (c) least-privilege = один `{resource,action}`; (d) public-key → compromised-marketplace
не forge-for-everyone; (e) **survives-marketplace-outage** (plugins keep-working p2p).

### 5.3 Sequencing — що ЗАРАЗ, що DEFER (малий team, не все відразу)
**ЗАРАЗ (load-bearing, cheap-now-ruinous-to-retrofit):**
1. Тримати 3 ворота ядра + KernelFacade-firewall (вже).
2. Розширити **той самий scope-grammar** на «plugin-calls-kernel-op-X» (contract/doc-extension, near-zero-
   code).
3. Закласти **кеш-шари** (§3.3) — єдиний реальний gap, і фундамент швидкості для всіх продуктів.
4. Закласти **own-inference + own-RAG** субстрат (llama.cpp + pgvector-pgrust) — фундамент для agent-flows
   і будь-якого AI-продукту.
5. Ship **рівно ОДИН** 2nd-продукт (dowiz Local, nearest-adjacent) end-to-end через порти — довести reuse
   **емпірично**.

**DEFER (не load-bearing поки немає 2nd-продукту):** registry-UI, revenue-billing, dev-agent-tier, GPU-
scale inference, federation-across-machines, owner-facing automation-canvas. Будувати marketplace-machinery
до 2nd-tenant = найпоширеніша platform-пастка (Shopify/SF/Stripe усі будували marketplace **після** того,
як core мав незалежне usage).

---

## 6. Що вже маємо vs що будувати (ґрунтовано інвентарем)

| Шар | Вже маємо (reuse) | Будувати / клонувати |
|---|---|---|
| **Ядро** | kernel decide/fold, integer-money, event-log, ChannelLedger, intake | нове event-словник per-product |
| **Capability** | proto-cap: Capability/SignedFrame/scope/hybrid_gate/AnchorRoster (Ed25519⊕ML-DSA-65, no-central-issuer) | scope-grammar extension для plugins |
| **PQ crypto** | ML-DSA-65/ML-KEM/AEAD/KDF/hash from-scratch, rng.rs fail-closed | Entropy-порт (з integration-ports арки) |
| **RAG** | repowise (LanceDB + local-Ollama-embeddings, MCP) — code-RAG | **product-RAG**: pgvector-pgrust + chunking-taxonomy + RRF+rerank |
| **Кешування** | ⚠️ **НІЧОГО (єдиний gap)** | 5 кеш-шарів (§3.3) |
| **Inference** | — | own llama.cpp (`llama-cpp-2`) за портом |
| **Ingestion** | — | markitdown-порт → owned-chunker |
| **Agent-infra** | agent-governance (drift+error-learning, WASM), 15 subagent-specs, hooks, living-memory, regression-ratchet | CrewAI-shaped orchestrator + AutoGen-actor-messaging (Rust, за capability) |
| **CI/build** | ci.yml validate-job, eslint-plugin-local 17-rules, gitleaks, backup-drill | HackAgent-CI, TOOLBELT.md, skill-sync |
| **Reliability** | health/rate-limit/circuit-breaker/k6-spike.js (у attic, pending decentralization) | re-home як capability-port-boundary patterns |
| **Транспорт** | proto-wire (iroh-p2p + WSS) | gossip/anti-entropy + content-addressed-chunker |

---

## 7. Хвилі (foundation-first, additive)

| Хвиля | Кит | Що | Ризик |
|---|---|---|---|
| **W0** | Ядро | Scope-grammar extension для plugins (contract/doc, near-zero-code); зафіксувати 3-ворота+KernelFacade | 🔴 capability red-line |
| **W1** | Інфра | Кеш-шари (embedding/Merkle-reindex/prefix-disk/ComfyUI-DAG/semantic) — **єдиний gap** | середній |
| **W2** | Інфра | Own-inference порт (llama.cpp `llama-cpp-2`, in-process, capability-scoped) | середній |
| **W3** | Інфра | Own-RAG (pgvector-pgrust + chunking-taxonomy + RRF+rerank), reuse repowise-embeddings | середній |
| **W4** | Потоки | Ingestion-порт (markitdown→owned-chunker) + firecrawl-recipe thin-Rust web-port | низький |
| **W5** | Потоки | Agents-as-capability-holders (CrewAI-hierarchy + AutoGen-actor over SignedFrame); living-memory mem0-loop (mark-invalid) | 🔴 agent-authority |
| **W6** | Інфра | Dev-toolbelt (rg-discipline/TOOLBELT.md/skill-sync) + security-CI (HackAgent/iFixAI) | низький |
| **W7** | Екосистема | Ship **dowiz Local** (2nd product) end-to-end через порти — довести reuse | середній (proof) |
| **W8** | Екосистема | Marketplace T1 static-git-index (тільки після 2nd-продукту) | низький, DEFER |
| **W-RED** | всі | RED-proof кожної хвилі (core-untouched, capability-isolation, cache-coherence, determinism) | обов'язковий gate |

---

## 8. Найбільший ризик і найбільша пастка

- **Стратегічний ризик:** local-first-moat = perceived-performance + protocol-trust, **не data-lock-in**
  (local-first structurally не тримає дані заручником). Форк open-sourced (ADR-020) протоколу+портів
  може реплікувати механіку. Тому durable-advantage = **якість capability-governance + agent-
  orchestration**, не код — і стає реальним **лише коли 2nd незалежний builder** біжить на ньому. Доти
  «платформа» = claim з нуль-external-adopters (AT-Proto failure-mode, не ActivityPub success). **Не
  over-invest у marketplace-machinery для платформи з рівно одним tenant.**
- **Архітектурна пастка:** ambient-trust orchestration — bolt-on feature-rich-orchestrator для demo-
  velocity + grant-kernel-adjacent-authority-to-save-sprint = ламає всю offline-verifiable-attenuation-
  only історію. Кожна запозичена ідея re-implemented **за capability-межею** перед тим, як торкнеться
  продукту.

---

## 9. RED / proof-дисципліна

Кожна хвиля закрита тільки коли RED red→green (VERIFIED-BY-MATH, Mandatory-Proof, deterministic-guardrail
+ regression-ledger-row):
- **Core-untouched** — адаптер імпортує kernel → build FAILS (KernelFacade firewall).
- **Capability-isolation** — порт/агент за scope читає поза grant → deny; attenuation не widen → verify-fails.
- **Cache-coherence** — новий event → refold інвалідовує derived-cache детерміновано (не timer); money
  ніколи не з write-behind-cache.
- **Determinism** — same-event→same-decision across nodes; ANN/HNSW **ніколи** в `decide()`.
- **Agent-authority** — агент = capability-holder; дія без валідного SignedFrame → reject, стан незмінний.

---

## 10. Résumé одним абзацом

Три кити. **Ядро** мале й вічне — decide/fold Law, integer-money, event-log, capability-модель; патерни
(idempotency/CQRS/saga-choreography/gate) — його властивості, не окремі сервіси; воно вже домен-агностичне
(zvt-доказ). **Інфраструктура** власна й багатократна — own llama.cpp inference, own pgvector-pgrust RAG,
і **п'ять кеш-шарів** (єдиний реальний gap); build-your-own-x над транспортом/verifier/chunker/cache;
нуль зовнішніх залежностей. **Потоки** асинхронні, ідемпотентні, capability-gated — gossip/choreography/
outbox/agents-as-capability-holders/matcher-as-LB. Екосистема = нові event-словники на спільних ядрі+інфрі:
Delivery→Local→Fleet→Ledger→Marketplace. Закладаємо ЗАРАЗ фундамент (scope-grammar, кеш, inference, RAG) +
ship рівно один 2nd-продукт, щоб довести reuse; усе інше — DEFER. Найбільший ворог — ambient-trust, що
рятує спринт ціною всієї моделі довіри.
