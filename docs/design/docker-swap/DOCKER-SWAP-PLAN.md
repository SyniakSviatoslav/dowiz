# dowiz / bebop2 — Docker full-swap: microVM + WASM (zero-OCI) — PLAN

> **Дата:** 2026-07-13 · **Тип:** дослідження → аналіз → синтез → план з блюпринтами (НЕ код) · десята задача.
> **Рішення оператора (final):** «microVM лише де він справді потрібен, в усіх інших випадках wasm» → **WASM=default,
> microVM=тільки-untrusted-НЕ-WASM-ізоляція, НУЛЬ Docker/OCI.**
> **Стоїть на:** [[mesh-real-arc-2026-07-13]] (native-node) + [[integration-ports-reactive-arc-2026-07-13]] (capability-
> ports=WASI-p2) + [[ops-reliability-arc-2026-07-13]] (pgrust) + kernel/bebop2. 4 паралельні дослідницькі смуги.

---

## 0. Головна теза + найважливіше уточнення

**Теза:** повний своп Docker — це НЕ «замінити один container-runtime іншим», а **розкласти навантаження на 4 форми
за принципом «мінімальна достатня ізоляція»**: native-Rust (нода/трасти), WASM-component (все capability-scoped за
замовчуванням), microVM (лише untrusted-не-WASM), і НУЛЬ OCI.

**★ Найважливіше уточнення (з інвентаря):** запланована **нода — вже НАТИВНИЙ Rust-бінарник**, не контейнер і навіть
не WASM на гарячому шляху (MESH-REAL Layer A: kernel-rlib, direct C-ABI, «БЕЗ WASM/JSON-hop»; ARCHITECTURE.md; WASM =
лише браузер). **Docker ніколи й не був runtime ноди.** Тому «свап Docker на ноді» — здебільшого мимо: нода стартує
напряму (systemd на боксі, native-app на телефоні). Docker сьогодні живе лише в build/dev/CI + інтерим-pgrust-хабі.
bebop2/ = ZERO docker-references.

**★ Архітектурний виграш (з WASM-смуги):** WASI Preview-2 = **zero-ambient-authority** (нема глобального open/socket;
кожен fs/socket/clock — явний import, який дає хост) = object-capability на рівні bytecode-ABI = **1:1 до bebop2
`Capability{subject_key, scope:Resource×Action, nonce, expiry}`**. WASM-компонент отримує гарантію KernelFacade
(«нема ambient-authority в ядро», IP-01) **безкоштовно при інстанціації** — без hand-rolled-lint. **Уже доведено:**
Microsoft **Wassette** (WASM-компонент = MCP-tool = наданий scope, deny-by-default, attenuation) — саме end-state, до
якого йде наш IP-08.

---

## 1. Що де сьогодні (Docker) — і куди

| Використання Docker сьогодні | Куди (за рішенням) |
|---|---|
| **Нода / kernel** (планована) | НАТИВНИЙ Rust-бінарник (systemd/native-app) — **не Docker, не WASM-runtime, не microVM** (вже так у плані) |
| **static-SPA serve** (chainguard/nginx) | **WASM-component** (static-file-server на wasmtime) АБО native-Rust-static-server (`static-web-server` 4MB). Нуль контейнера |
| **pgrust** (інтерим-хаб) | **NATIVE process** (systemd, trusted-first-party, ризик = SQL/RLS не process-isolation) — microVM лише якщо оператор хоче VM-grade DB-ізоляцію (опційно) |
| **ports/adapters/plugins/agents** (майбутнє) | **WASM-components** (wasmtime; WASI-cap = Scope) — default |
| **untrusted dev-agent-tier adapters / 3rd-party MCP** (майбутнє) | **microVM (Firecracker/unikernel)** — єдиний справжній microVM-кейс; server-class-хаби (KVM ≠ телефон) |
| **agent-governance** | нічого не виконує (pure text-policy-filter) → **нема ізоляції взагалі** (не microVM!) |
| **dev tools** (ollama/libretranslate) | native / Podman-локально (dev-only, мінор) |
| **CI Playwright-visual** (deterministic) | Firecracker-microVM АБО лишити pinned-image (CI-only, low-risk) |
| **CI build** | cargo→wasm-component + cargo→native + Firecracker-rootfs/unikernel; supply-chain syft/Trivy/cosign. Нуль OCI |

---

## 2. Чотири форми (принцип «мінімальна достатня ізоляція»)

### Форма 1 — NATIVE Rust (нода + трасти-first-party)
Нода = native-Rust-бінарник (kernel-rlib, direct C-ABI). pgrust = native systemd-process (trusted DB; ізоляція = сам
хост; VM-обгортати trusted-код від себе самого — беззмістовно). Static-server можна native-Rust. **Нуль контейнера.**
Це вже архітектурний напрям — план лише формалізує «Docker тут ніколи не був потрібен».

### Форма 2 — WASM-components (DEFAULT для всього capability-scoped)
**wasmtime** (embedded; reference WASI-p2, найглибший аудит: Cranelift-formally-verified + Miri + cargo-vet + 2026-
multivendor-security-sprint; no_std+AArch64 = edge/phone). **WasmEdge** — build-опція для телефона (1.5ms/8MB,
prebuilt-Android). Ports/adapters/plugins/agents/MCP-tools = `wasm32-wasip2` компоненти; **WASI-grant = required_scope()**.
Static-file-serving = WASM-компонент. **wasmCloud: беремо ПАТЕРН** (actor + pluggable-capability-provider = наш
InboundPort/OutboundPort), **відкидаємо СУБСТРАТ** (NATS-lattice = broker/gossip = саме те, що D3 відкинув на користь
DTN/BPv7). Wasmer = WASIX-non-standard (не той шлях для Scope-map). Spin = trigger-server-shaped + Akamai-consolidation.

### Форма 3 — microVM (Firecracker/unikernel, ТІЛЬКИ де справді потрібно)
Єдиний справжній кейс: **untrusted НЕ-WASM код** — майбутні **dev-agent-tier port-адаптери / 3rd-party MCP-сервери**
(IP-01/IP-02), код, який dowiz не може повністю перевірити, server-side. **Firecracker** (~125ms, <5MiB/VM, powers
Lambda/Fly-Machines, Apache-2.0) — direct, НЕ Kata (Kata = OCI-wrapper → суперечить «zero-OCI»). Для max-isolation
sovereign-client-боксів — **unikernel** (NanoVMs/OPS, вже Phase-3 у sovereign-node-ladder). **Обмеження (чесно): KVM
обов'язковий → телефон кур'єра КАТЕГОРИЧНО не годиться** (mobile-firmware EL1); owner-small-box можливий-але-крихкий на
Pi-class. → **microVM = server-class-хаби only.** НЕ для kernel(trusted-compile-firewalled), static-SPA(no-execution),
pgrust(risk=SQL/RLS), agent-governance(pure-filter).

### Форма 4 — Build / supply-chain (заміна Docker-build)
WASM: `cargo component build`→`wasm32-wasip2` + cosign-keyless-sign + syft-SBOM + Trivy/Grype-scan. Native: `cargo build`
+ SBOM/sign. microVM: mkosi/Buildah-rootfs АБО unikernel-image + SBOM/sign/scan. **Нуль OCI-runtime.** CI = daemonless.
Dev-ергономіка: OrbStack (Mac) / Podman-Desktop — але це dev-only (Docker-Engine CVE-2026-34040 = аргумент за rootless).

---

## 3. Чому це вирівняно з архітектурою (не довільний вибір)

- **Capability-ports = WASI-p2 capability-model** — WASM-компонент з WASI-грантом = рівно наш `Scope` (Resource×Action).
  KernelFacade-firewall стає безкоштовним (import-set = єдине, що компонент може торкнутися). Wassette це вже реалізує.
- **Нода native-Rust** — WASM/microVM не на гарячому шляху (це вже рішення MESH-REAL/ARCHITECTURE).
- **DTN/BPv7 locked (D3)** — тому wasmCloud-NATS-lattice відкинуто (той самий broker/gossip, що D3 відкинув).
- **Sovereign-node 3-фазний ladder** (Docker→WASI/WasmEdge→unikernel) — план формалізує: Phase-1-Docker DROPPED,
  Phase-2-WASI = default, Phase-3-unikernel = microVM-tier.
- **Телефон**: тільки WASM phone-viable (Docker no-root-on-iOS/Android; Firecracker needs-KVM). → нода-на-телефоні =
  native-Rust + WASM-компоненти; microVM там неможливий by-design.

---

## 4. Хвилі (additive; кожна RED red→green)

| Хвиля | Форма | Що | Ризик |
|---|---|---|---|
| **DK0** | Build | Toolchain: cargo→wasm32-wasip2 (cargo-component) + wasmtime-embed + syft/Trivy/cosign supply-chain | середній |
| **DK1** | WASM | WR-01: перший `OutboundPort` (напр. Telegram/IP-15) як wasm32-wasip2-компонент, WASI-imports=required_scope() | 🔴 (capability) |
| **DK2** | WASM | wasmtime host: інстанціює компоненти-порти з WASI-context=Scope; KernelFacade-free-at-instantiation | 🔴 |
| **DK3** | Native | static-SPA: chainguard/nginx → native-Rust-static-server (АБО WASM-static-component); прибрати nginx-контейнер | низький |
| **DK4** | Native | pgrust → systemd native-process (drop Docker-hub); PgBouncer native | середній |
| **DK5** | microVM | Firecracker-tier для untrusted dev-agent-tier-адаптерів/3rd-party-MCP (server-class hubs); host-capability fail-closed probe | 🔴 (isolation) |
| **DK6** | Build | CI: daemonless (cargo-wasm + Firecracker-rootfs) + SBOM+scan+cosign-sign; retire OCI images; CI-Playwright→Firecracker-or-pinned | середній |
| **DK7** | Dev | OrbStack/Podman-Desktop dev-ергономіка (dev-only) | низький |
| **DK-RED** | всі | RED-suite + no-ambient-authority proofs | обов'язковий |

---

## 5. RED-контракти

- **WR-01 (port-as-component):** компонент-порт з WASI-context, що дає ТІЛЬКИ `required_scope()` → будь-яке відкриття
  file/non-allowlisted-socket **TRAPS at host-boundary** (не silently-no-op). Дзеркалить IP-02-R4 (out-of-scope-attenuated
  → verify-fails).
- **no-ambient-authority:** компонент без наданого capability не дістає жодного host-import (deny-by-default at
  instantiation).
- **microVM-fail-closed:** якщо host не має KVM → microVM-tier відмовляється стартувати untrusted-код (не fallback-to-
  unisolated); on-phone → microVM-path недоступний, untrusted-non-WASM-код НЕ виконується там взагалі.
- **zero-OCI:** білд-пайплайн не породжує OCI-runtime-контейнера (cargo-wasm/native/rootfs only); SBOM present; 0-critical-CVE.
- **native-node:** нода-бінарник стартує без будь-якого container/VM/WASM-runtime (direct execution).

---

## 6. Найбільші ризики / чесні межі

- **microVM ≠ телефон** (KVM/EL2 недоступні) — тому untrusted-не-WASM-код НЕ можна ізолювати на пристрої кур'єра; на
  edge/phone усе untrusted МУСИТЬ бути WASM (capability-scoped) або не виконуватись. Це формує правило: **3rd-party/dev-
  agent код на edge = ТІЛЬКИ WASM; microVM-fallback = лише server-class-хаб.**
- **pgrust-як-native-process** — прибирає Docker-ізоляцію DB; ризик pgrust = його незрілість (окремий gate, не container-
  проблема), ізоляція = хост + capability-gate на рівні застосунку.
- **WASI-p2 re-target** — kernel сьогодні `wasm-bindgen`(browser-JS-blob, не-WASI-p2); порти-як-компоненти потребують
  `wasm32-wasip2`/cargo-component (нова toolchain-ланка, DK0).
- **wasmCloud-спокуса** — не брати NATS-lattice (суперечить DTN/BPv7); лише патерн.
- **Firecracker-vs-unikernel** для max-isolation — unikernel(NanoVMs) = Phase-3-plan, важчий build; Firecracker-rootfs =
  простіший старт.

---

## 7. Résumé одним абзацом

Повний своп Docker = 4 форми за «мінімальною достатньою ізоляцією», нуль OCI. **Нода/kernel/pgrust = native-Rust-процеси**
(Docker тут ніколи й не був потрібен). **WASM-компоненти (wasmtime; WasmEdge-phone) = DEFAULT** для всього capability-
scoped (ports/adapters/plugins/agents/static-serve) — WASI-p2-zero-ambient-authority = 1:1 до bebop2-Capability/Scope,
KernelFacade-безкоштовно (Wassette довів). **microVM (Firecracker/unikernel) = ТІЛЬКИ untrusted-не-WASM** (dev-agent-tier-
адаптери, server-class-хаби; KVM≠телефон). agent-governance = pure-filter, не microVM. Формалізує наявний sovereign-3-
фазний ladder. Найбільша межа: на телефоні microVM неможливий → untrusted-edge-код = тільки WASM.
