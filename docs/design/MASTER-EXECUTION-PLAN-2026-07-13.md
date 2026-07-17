> **SUPERSEDED (2026-07-17)** — see `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (canonical
> roadmap, phases P01–P30; its header names this doc) and `CORE-ROADMAP-INDEX.md` +
> `CORE-ROADMAP-2026-07-17/` (the Layer A–I execution structure). Preserved for historical/
> audit-trail purposes only. **Lineage note (P-I audit §2.5):** this doc's 4-phase altitude spine
> (Земля → ядро → поверхня → платформа) is the direct ancestor of the Layer A–I axis — the same
> idea, re-expressed. Its lasting value is as the historical index to the 9 pre-pivot sub-plan
> directories (~151 blueprint units), whose live navigation now lives in MEMORY.md → "Active arcs
> (earlier)". This doc was omitted from CORE-ROADMAP-STANDARD §0's original 5-doc inventory — the
> P-I audit corrected the count to 6.

# dowiz / bebop2 — MASTER EXECUTION ROADMAP (усі плани 2026-07-13)

> Один майстер-план, що зводить 9 тематичних дизайн-планів + 1 виконаний операційний трек у **логічну
> послідовність виконання**, з індексацією на під-плани (PLAN), блюпринти (BLUEPRINTS) та дослідження
> (RESEARCH-CONSPECT). Це не 10 окремих речей — це **одна когерентна система**, зшита спільними нитками.
> ~151 блюпринт-одиниця across 9 планів.

---

## 0. Наскрізні нитки (що робить 10 планів ОДНІЄЮ системою)

| Нитка | Де з'являється | Суть |
|---|---|---|
| **Ядро незмінне + decide/fold Law** | mesh-real, integration-ports, ecosystem, field-ui (money-never-tween) | `dowiz-kernel` order_machine/money/domain = єдиний авторитет; усе інше не чіпає |
| **Capability-модель (SignedFrame/scope, KernelFacade)** | integration-ports (IP-01), mesh-real (MESH-02), docker-swap (WASI-p2=Scope), ecosystem (marketplace) | «кор незмінний, інтеграції=порти, ніколи пряме втручання» — механічний (3 ворота + compile-firewall) |
| **pgrust (local-first store)** | ops-reliability, mesh-real (per-node), ecosystem (RAG), docker-swap (native process) | одне сховище на нод; compat-gate ПРОЙШОВ |
| **WASM / WASI-p2** | rust-engine-rewrite, field-ui, mesh-real, **docker-swap** (WASI-cap=Scope) | двигун+порти як WASM-компоненти; capability-native |
| **3 кити (ЯДРО/ІНФРА/ПОТОКИ)** | ecosystem-strategy (каркас) → усі решта мапляться | organizing spine усього |
| **PQ crypto (ML-KEM/ML-DSA/Ed25519, entropy mix-never-replace)** | mesh-real, integration-ports (ANU QRNG), ops (backups age) | пост-квантова безпека наскрізно |
| **RED / VERIFIED-BY-MATH / статус-з-live-test** | усі 9 | кожен блюпринт має reachable red→green |

---

## 1. ★ ЛОГІЧНА ПОСЛІДОВНІСТЬ ВИКОНАННЯ (4 фази, 3 паралельні треки)

Залежності диктують: **земля → ядро → поверхня → платформа**. Два продуктові треки (Протокол/нод + Інтерфейс/клієнт)
паралельні після фундаменту; мета-трек (агенти/self-improvement) біжить збоку.

### ФАЗА 0 — ЗЕМЛЯ (інфра + як усе працює) · частково ВИКОНАНО
| # | План | Що | Залежить | Статус |
|---|---|---|---|---|
| 1 | **ops-reliability** (22 OPS-*) | pgrust-міграція, один-пульт-моніторинг+Telegram, запобіжники, бекапи 3-2-1-1-0, CF-hardening | — (фундамент) | pgrust-compat ✅, off-Hetzner-DR ✅, решта план |
| 2 | **docker-swap** (10 DK-*) | microVM+WASM runtime, нуль-OCI (як фізично запускати ноди/порти/pgrust) | capability-модель, pgrust | план |

### ФАЗА 1 — ПРОТОКОЛЬНЕ ЯДРО (серце MVP) · non-negotiable
| # | План | Що | Залежить |
|---|---|---|---|
| 3 | **mesh-real** (14 MESH-*) | реальний меш: kernel-as-decider, транспорт iroh+BPv7, authz(KernelFacade+revocation), per-node-local-first(pgrust+event-log+pull-anti-entropy+Merkle) | ФАЗА-0 (runtime+store) |
| 4 | **integration-ports** (21 IP-*) | capability-порти (як WASM-компоненти per docker-swap), реактивний інтерфейс, ANU-QRNG-entropy-порт | mesh-real (capability-ядро) |

### ФАЗА 2 — ІНТЕРФЕЙСНИЙ ДВИГУН (поверхня) · споживає kernel-стан
| # | План | Що | Залежить |
|---|---|---|---|
| 5 | **field-ui-engine** (17 FE-*) | ОДИН фізичний оператор M Ü+Γ U̇+c²LU=S малює весь UI (ζ=1=governor) | kernel-стан (mesh) |
| 6 | **rust-engine-rewrite** (12 RW-*) | увесь TS/JS-інтерфейс → нативна Rust/WASM бібліотека двигуна | FE-* |
| 7 | **dowiz-interfaces** (12 DZ-*) | дизайн-мова «Sea & Sheet» на field-UI; усі ролі, local-first, кросплатформ, мультимодально | FE-* |

### ФАЗА 3 — ПЛАТФОРМА + САМОВДОСКОНАЛЕННЯ (будується на всьому)
| # | План | Що | Залежить |
|---|---|---|---|
| 8 | **ecosystem-strategy** (20 EC-*) | 3 кити; own-inference/RAG/★caching(єдиний-gap); marketplace; мапа продуктів Delivery→Local→Fleet→Ledger→Marketplace | ядро-доведене (ФАЗА-1) |
| 9 | **hydraulic-loop-v2** (23 од.) | кібернетичний замкнутий цикл / self-improvement на L5-стеку (living-memory+агенти) | мета-трек, біжить збоку |

**Критичний шлях MVP:** ФАЗА-0 (ops+docker-swap) → mesh-real → integration-ports → ship. Інтерфейс-трек (FE→RW/DZ)
паралельний. ecosystem+hydraulic = після доведеного ядра.

---

## 2. ПОВНИЙ ІНДЕКС (усі плани × 3 доки)

| План | PLAN | BLUEPRINTS | RESEARCH | Одиниць |
|---|---|---|---|---|
| ops-reliability | `ops-reliability/OPS-RELIABILITY-PLAN.md` | `BLUEPRINTS-OPS-RELIABILITY.md` | `RESEARCH-CONSPECT.md` | 22 OPS |
| docker-swap | `docker-swap/DOCKER-SWAP-PLAN.md` | `BLUEPRINTS-DOCKER-SWAP.md` | `RESEARCH-CONSPECT.md` | 10 DK |
| mesh-real | `mesh-real/MESH-REAL-PLAN.md` | `BLUEPRINTS-MESH-REAL.md` | `RESEARCH-CONSPECT.md` | 14 MESH |
| integration-ports | `integration-ports/INTEGRATION-PORTS-PLAN.md` | `BLUEPRINTS-INTEGRATION-PORTS.md` | `RESEARCH-CONSPECT.md` | 21 IP |
| field-ui-engine | `field-ui-engine/FIELD-UI-ENGINE-PLAN.md` | `BLUEPRINTS-FIELD-UI.md` | `RESEARCH-CONSPECT.md` | 17 FE |
| rust-engine-rewrite | `rust-engine-rewrite/RUST-ENGINE-REWRITE-PLAN.md` | `BLUEPRINTS-RUST-ENGINE-REWRITE.md` | `RESEARCH-CONSPECT.md` | 12 RW |
| dowiz-interfaces | `dowiz-interfaces/DOWIZ-INTERFACES-PLAN.md` | `BLUEPRINTS-DOWIZ-INTERFACES.md` | `RESEARCH-CONSPECT.md` | 12 DZ |
| ecosystem-strategy | `ecosystem-strategy/ECOSYSTEM-STRATEGY-PLAN.md` | `BLUEPRINTS-ECOSYSTEM.md` | `RESEARCH-CONSPECT.md` | 20 EC |
| hydraulic-loop-v2 | `hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` | `BLUEPRINTS.md` | `MATH-RESEARCH-CONSPECT.md` + `CODE-ANALYSIS-CONSPECT.md` + `SCHEME-AUDIT.md` | 23 |

*(усі шляхи відносно `docs/design/`)*

---

## 3. 10-й трек — ВИКОНАНА ОПЕРАЦІЙКА (не план, а зроблене сьогодні)

Консолідація на Hetzner + де-ризикування pgrust, виконано в коді (не лише план):
- **R2 → Hetzner Object Storage**: 65 image-об'єктів перенесено (`dowiz/images/`).
- **Supabase → age-дамп**: повний `pg_dump` PG17 (16.9MB), age-зашифровано, у Hetzner-бакеті (`dowiz/db/`).
- **Холодні бандли**: age-зашифровано (`dowiz/cold/`, 3.2G).
- **★ pgrust COMPAT-GATE — ПРОЙШОВ** (pgrust-18.3 везе citext/pgcrypto/bcrypt/uuid+contrib) → весь ризик pgrust-
  immediate знято.
- **★ off-Hetzner DR (Cloudflare R2 `dowiz-offsite`)** — бакет створено, критичний дамп + повне дзеркало (у процесі).
- Vault `.secrets.local` (age-шифр бекапи, age-ключ офлайн). Том уніфіковано, MEMORY стиснуто.

---

## 4. Наскрізні інваріанти (тримають усі 151 блюпринт)

- **Кор незмінний** (KernelFacade compile-firewall: адаптер що імпортує kernel = build-fails).
- **Гроші ніколи не tween/float/write-behind/CRDT-merge** (i64 by type, event-sourcing not CRDT).
- **Capability не bearer** (attenuation-only, offline-verify; WASI-p2 = це безкоштовно).
- **NO-COURIER-SCORING** (механічний CI-grep).
- **Ентропія mix-never-replace** (OS-floor fail-closed).
- **microVM лише де справді потрібен, WASM за замовчуванням, нуль-OCI**.
- **Статус тільки з live-test** (red-team-доки бувають застарілі).
- **Reuse-first** (~90% ядра+crypto+capability вже існує; воскресити з attic, не будувати).

---

## 5. Résumé

10 планів = одна система в 4 фази. **Земля** (ops+docker-swap: pgrust+моніторинг+бекапи+microVM/WASM-runtime) → **ядро**
(mesh-real+integration-ports: реальний PQ-меш з capability-портами) → **поверхня** (field-ui+rust-engine+dowiz-interfaces:
фізичний Rust/WASM-інтерфейс) → **платформа** (ecosystem: 3-кити+marketplace; hydraulic-loop: self-improvement). Зшито
спільними нитками: незмінне-ядро, capability-модель, pgrust, WASM/WASI-p2, PQ-крипта, RED-дисципліна. Критичний шлях
MVP: ФАЗА-0→mesh-real→integration-ports. Інтерфейс паралельно. Операційка (Hetzner+pgrust-compat+off-Hetzner-DR) — вже
виконана.
