# dowiz / bebop2 — Docker full-swap (microVM + WASM, zero-OCI) — BLUEPRINTS

> **Дата:** 2026-07-13 · Супроводжує [DOCKER-SWAP-PLAN.md](./DOCKER-SWAP-PLAN.md).
> 10 одиниць DK-01..10. Кожна: **Мета · Межа · Форма · Reuse · RED · Хвиля.** НЕ код.
> Принцип: мінімальна достатня ізоляція — native-Rust / WASM(default) / microVM(лише untrusted-не-WASM) / нуль-OCI.
> 🔴 = red-line.

## Форма 2 — WASM-components (default)

### DK-01 · WASM build toolchain + supply-chain
- **Мета:** білдити capability-scoped навантаження як `wasm32-wasip2` компоненти + підписаний supply-chain.
- **Межа:** ЧІПАЄМО — нова toolchain-ланка (cargo-component). НЕ ЧІПАЄМО — kernel-hot-path (native-rlib).
- **Форма:** `cargo component build --target wasm32-wasip2`; embed **wasmtime** (reference WASI-p2). Supply-chain:
  syft-SBOM + Trivy/Grype-scan + cosign-keyless-sign (SLSA). Note: kernel сьогодні `wasm-bindgen`(browser) — порти-
  компоненти = окремий wasip2-таргет.
- **Reuse:** kernel-Rust, integration-ports-blueprints. **RED:** компонент-артефакт має SBOM + cosign-signature; білд
  не породжує OCI-image. **Хвиля:** DK0.

### DK-02 · Port-as-WASM-component (WR-01)
- **Мета:** порт/адаптер = WASM-компонент, WASI-grant = його `required_scope()`.
- **Межа:** ЧІПАЄМО — компіляція одного OutboundPort у компонент. НЕ ЧІПАЄМО — SignedFrame/Scope (reuse).
- **Форма:** перший порт (напр. Telegram/IP-15) → `wasm32-wasip2`-компонент; host інстанціює з WASI-context, що дає
  ТІЛЬКИ host-функції, які декларує `required_scope()` (`Notify·Telegram`). = WASI-p2 zero-ambient-authority = 1:1 до
  Capability{scope:Resource×Action}. Прецедент: Microsoft-Wassette, Extism-manifest(deny-by-default).
- **Reuse:** IP-15, Capability/Scope, IP-02. **RED (WR-01):** інстанс із scope=`Notify·Telegram` → спроба відкрити
  file/non-allowlisted-socket **TRAPS at host-boundary** (не silently-no-op); дзеркалить IP-02-R4. **Хвиля:** DK1. 🔴

### DK-03 · wasmtime host — Scope→WASI capability instantiation (KernelFacade-free)
- **Мета:** зробити «нема ambient-authority в ядро» безкоштовним при інстанціації (не hand-rolled-lint).
- **Межа:** ЧІПАЄМО — wasmtime-embedding-host. НЕ ЧІПАЄМО — KernelFacade-concept (WASM реалізує його механічно).
- **Форма:** host мапить bebop2-`Capability.scope` → WASI-import-set компонента; компонент фізично не може дістати нічого
  поза наданим import-set. wasmCloud: беремо ПАТЕРН (actor+capability-provider), відкидаємо СУБСТРАТ (NATS≠DTN/BPv7).
- **Reuse:** capability.rs, scope.rs, IP-01/08. **RED:** компонент без наданого capability → нуль host-imports (deny-
  by-default); out-of-scope-import → trap. **Хвиля:** DK2. 🔴

## Форма 1 — Native (нода + трасти)

### DK-04 · Native static-server (drop nginx container)
- **Мета:** прибрати nginx-контейнер для static-SPA.
- **Межа:** ЧІПАЄМО — runtime static-serve. НЕ ЧІПАЄМО — CSP/headers (переносимо).
- **Форма:** chainguard/nginx-контейнер → **native-Rust static-server** (`static-web-server` 4MB, SPA-fallback+CSP+
  brotli+HTTP/2) systemd АБО WASM-static-component. Нуль контейнера.
- **Reuse:** docker/nginx-default.conf (CSP→config). **RED:** SPA-fallback + CSP-headers присутні; no-container-runtime.
  **Хвиля:** DK3.

### DK-05 · pgrust as native systemd process
- **Мета:** pgrust як native-процес (drop Docker-хаб), бо trusted-first-party.
- **Межа:** ЧІПАЄМО — pgrust-runtime. НЕ ЧІПАЄМО — pgrust-binary (compat-gate вже PASSED).
- **Форма:** pgrust systemd-managed native-binary на хабі + PgBouncer native. Ізоляція = хост + app-level-capability-
  gate (ризик pgrust = SQL/RLS/незрілість = окремий gate, не process-isolation). microVM ТІЛЬКИ якщо оператор хоче
  VM-grade-DB-ізоляцію (опційно, не default).
- **Reuse:** compat-gate-результат, ops-reliability-plan. **RED:** pgrust стартує systemd без Docker; RLS-cross-tenant=0
  (app-gate). **Хвиля:** DK4.

## Форма 3 — microVM (лише untrusted-не-WASM)

### DK-06 · Firecracker/unikernel tier для untrusted-не-WASM
- **Мета:** ізолювати untrusted НЕ-WASM код (dev-agent-tier-адаптери, 3rd-party-MCP-сервери) на server-class-хабах.
- **Межа:** ЧІПАЄМО — Firecracker-tier + host-KVM-probe. НЕ ЧІПАЄМО — WASM-path (default для untrusted-WASM).
- **Форма:** Firecracker-microVM (direct, НЕ Kata=OCI) для untrusted-не-WASM (IP-01/02 dev-agent-tier). Max-isolation-
  sovereign-box → unikernel (NanoVMs/OPS, Phase-3). Upgrade наявного `unshare -n` (crates/bebop/sandbox.rs) → Firecracker
  для VM-grade. **Server-class-хаби ONLY (KVM обов'язковий).**
- **Reuse:** sandbox.rs, integration-ports dev-agent-tier, sovereign-Phase-3-docs. **RED (fail-closed):** host без KVM →
  microVM-tier ВІДМОВЛЯЄТЬСЯ стартувати untrusted-код (не fallback-to-unisolated); on-phone → path-недоступний, untrusted-
  non-WASM НЕ виконується. **Хвиля:** DK5. 🔴

### DK-07 · agent-governance = no-sandbox (уточнення)
- **Мета:** зафіксувати, що agent-governance НЕ потребує ізоляції (не microVM-кейс).
- **Межа:** ЧІПАЄМО — документацію-рішення. НЕ ЧІПАЄМО — agent-governance-код (pure-filter).
- **Форма:** `agent-governance/index.ts` = pure text/voice-policy-filter (gender/profanity/archetype/HARD-BAN), executes-
  NOTHING-untrusted → нема ізоляції взагалі. (Виправляє початкове припущення, що це microVM-кандидат.)
- **Reuse:** — . **RED:** grep — agent-governance не викликає exec/spawn/eval untrusted-code. **Хвиля:** DK5.

## Форма 4 — Build/dev + RED

### DK-08 · CI daemonless build + SBOM/scan/sign + zero-OCI
- **Мета:** білд без Docker-daemon/OCI-runtime, з supply-chain.
- **Межа:** ЧІПАЄМО — CI-build-кроки. НЕ ЧІПАЄМО — validate-job (compose-with).
- **Форма:** cargo→wasm-component + cargo→native + Firecracker-rootfs(mkosi/Buildah) → syft-SBOM → Trivy(primary)+Grype
  (2nd-opinion) → cosign-keyless-sign+SLSA. Daemonless (kaniko-style якщо будь-який OCI-артефакт лишиться). CI-Playwright-
  visual (deterministic) → Firecracker-microVM АБО keep-pinned-image. ⚠️Dokploy=Docker-Engine-API → якщо лишається,
  deploy-layer може тримати rootful-Docker (decision-point).
- **Reuse:** ci.yml-validate, ops-reliability-Trivy. **RED:** білд не породжує OCI-runtime-image (тільки wasm/native/
  rootfs); SBOM-present; 0-critical-CVE. **Хвиля:** DK6.

### DK-09 · Dev ergonomics (dev-only)
- **Мета:** локальна dev-ергономіка без Docker-Desktop.
- **Межа:** ЧІПАЄМО — dev-машину. НЕ ЧІПАЄМО — prod/CI.
- **Форма:** OrbStack (Mac, 3-5s-boot, rootless, Docker-CLI-compat) / Podman-Desktop (rootless, free). dev-tools (ollama/
  libretranslate) — Podman-локально або native. Docker-Engine CVE-2026-34040 = аргумент за rootless. Apple-Containers =
  not-prod-ready-2027.
- **Reuse:** docker-compose.dev.yml. **RED:** N/A (dev). **Хвиля:** DK7.

### DK-10 · RED-suite + no-ambient-authority + native-node proofs
- **Мета:** один RED-gate: WASM-default-sound, microVM-fail-closed, zero-OCI, native-node.
- **Межа:** ЧІПАЄМО — test-крейт/CI-lint. НЕ ЧІПАЄМО — продакшн.
- **Форма:** зібрати: WR-01(port-component-traps-out-of-scope); no-ambient-authority(no-capability→no-host-import);
  microVM-fail-closed(no-KVM→refuse, no-fallback-unisolated; phone→untrusted-non-WASM-not-run); zero-OCI(build=no-OCI-
  runtime, SBOM-present, 0-critical-CVE); native-node(binary-starts-direct-no-container/VM/WASM-runtime).
- **RED:** кожен reachable red→green, regression-ledger-row. **Хвиля:** DK-RED. 🔴

---

## Зведення: блюпринт → форма → хвиля

| BP | Назва | Форма | Хвиля | Red-line |
|---|---|---|---|---|
| DK-01 | WASM build toolchain + supply-chain | WASM | DK0 | — |
| DK-02 | Port-as-WASM-component (WR-01) | WASM | DK1 | 🔴 |
| DK-03 | wasmtime host Scope→WASI (KernelFacade-free) | WASM | DK2 | 🔴 |
| DK-04 | Native static-server (drop nginx) | Native | DK3 | — |
| DK-05 | pgrust native systemd (drop Docker-hub) | Native | DK4 | — |
| DK-06 | Firecracker/unikernel untrusted-не-WASM | microVM | DK5 | 🔴 |
| DK-07 | agent-governance = no-sandbox | — | DK5 | — |
| DK-08 | CI daemonless + SBOM/scan/sign zero-OCI | Build | DK6 | — |
| DK-09 | Dev ergonomics (OrbStack/Podman) | Dev | DK7 | — |
| DK-10 | RED-suite + no-ambient + native-node | всі | DK-RED | 🔴 |

**Інваріант усіх 10:** мінімальна достатня ізоляція; нода/pgrust=native-Rust(Docker-тут-ніколи-не-був); WASM=default
(WASI-cap=Scope, KernelFacade-free); microVM=тільки-untrusted-не-WASM(KVM≠телефон, server-class-only); нуль-OCI;
на-телефоні-untrusted-код=тільки-WASM. Формалізує sovereign-3-фазний ladder (Docker-DROPPED, WASI-default, unikernel-tier).
