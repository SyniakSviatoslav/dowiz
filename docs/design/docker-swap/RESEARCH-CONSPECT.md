# Docker full-swap (microVM + WASM, zero-OCI) — RESEARCH CONSPECT (4 lanes)

> Дата: 2026-07-13 · 4 паралельні смуги (inventory · WASM-runtime · microVM · hardened-images/build) ґрунтовані
> в коді + WebSearch. Для [DOCKER-SWAP-PLAN.md](./DOCKER-SWAP-PLAN.md) + [BLUEPRINTS](./BLUEPRINTS-DOCKER-SWAP.md).
> ★ ОПЕРАТОРСЬКЕ РІШЕННЯ (final): «microVM лише де він справді потрібен, в усіх інших випадках wasm» — WASM=default,
> microVM=тільки-untrusted-НЕ-WASM-ізоляція, НУЛЬ Docker/OCI.

## LANE D1 — inventory + node-architecture
- Docker TODAY (мало, build/dev/CI): dowiz Dockerfile=2-stage static-SPA (node-build→★cgr.dev/chainguard/nginx-serve,
  вже-hardened, pure-static-no-backend). docker-compose.dev=DEV-ONLY(ollama+libretranslate). visual.yml CI=pinned-
  playwright(deterministic-visual). skillspector-Dockerfile=scanner-CLI. ci.yml=NO-docker(deploy-retired-D1). ★bebop2/=
  ZERO-docker. bebop-repo Dockerfile.sovereign=OLD-TS-agent(SUPERSEDED). pgrust-docker=INTERIM-hub-only.
- ★★ PLANNED NODE = NATIVE RUST BINARY, not-container not-even-WASM-on-hot-path (MIGRATION-PLAN: cdylib→host-agent→
  native-Rust "no-TS/node_modules/pnpm"; MESH-REAL-Layer-A: kernel-plain-rlib "БЕЗ-WASM/JSON-hop"; ARCHITECTURE.md:
  "direct-cdylib-C-ABI, no-wasm-bindgen/JSON-in-hot-path"). WASM=browser-only. Docker=NEVER-the-node. → «swap-Docker-
  at-node»=здебільшого-moot: нода native-Rust, runs-directly(systemd-on-box, native-app-on-phone).
- WASM-footprint: kernel-already-WASM(kernel/pkg/) але-runs-IN-BROWSER; NO wasmtime/wasmer/wasmedge-in-live-path (only-
  archived). bebop2/core-wasm32-EMPTY-import-section(pure-compute-gate).
- ★UNTRUSTED-ISOLATION TODAY = crates/bebop/src/sandbox.rs `unshare -n`+egress-blocklist, NOT-Docker (=microVM-upgrade-
  candidate). ★EXISTING-3-PHASE-LADDER(sovereign-node-docs): Phase1-Docker→Phase2-WASI/WasmEdge→Phase3-NanoVMs/OPS-
  UNIKERNEL = вже-в-дусі-microVM+WASM.

## LANE D2 — WASM component runtimes (the default)
- ★WASI-p2 (WASI-0.2-stable-Jan2024, wasmtime-reference, WASI-1.0-targeted-2026) = ZERO-AMBIENT-AUTHORITY: no-global-
  open/socket; every-fs/socket/clock = explicit-import-host-instantiates = object-capability-at-bytecode-ABI. ★1:1-MAP
  до bebop2 Capability{subject_key,scope:Resource×Action,nonce,expiry}(closed-enum, offline-verifiable, not-bearer). →
  WASI-p2-component-gets-KernelFacade-guarantee(IP-01 no-ambient-authority)-FOR-FREE-at-instantiation(no-hand-rolled-
  lint). ★PROVEN: Microsoft-Wassette(Aug2025, wasmtime, WASM-component=MCP-tool=granted-scope, deny-by-default,
  attenuation)=exactly-dowiz-IP-08-end-state, shipped. Extism-manifest(allowed_hosts/paths deny-by-default).
- RUNTIMES: ★wasmtime(BytecodeAlliance, WASI-p2-reference, ~3ms/15MB/12K-rps, Cranelift-formally-verified+Miri+cargo-
  vet+2026-multivendor-security-sprint, no_std+AArch64-Winch=edge/phone, underlies-Spin+Wassette, Apache-2.0-LLVM-exc)
  = RECOMMENDED per-node-embedded. WasmEdge(CNCF-sandbox, 1.5ms/8MB/15K-rps=fastest, ★prebuilt-Android-binaries=best-
  phone-story)=phone-build-option(both-WASI-p2). Spin(Fermyon, trigger-model-server-shaped, ★Akamai-acquired-2026=
  vendor-consolidation). Wasmer(WASIX-non-standard-fork, MIT, wins-usability-not-standardization=wrong-path-for-Scope-
  map). wasmCloud(CNCF-incubating, actor+capability-provider=near-exact-InboundPort/OutboundPort-precedent BUT lattice=
  NATS=broker/gossip=EXACTLY-D3-REJECTED-shape(libp2p/Zenoh); wRPC-over-QUIC-decouple=roadmap-not-shipped) → ADOPT-
  PATTERN-REJECT-SUBSTRATE.
- BOUNDARY: WASM-runs kernel/ports/adapters/plugins/agents/MCP-tools(capability-scoped, syscall-light). NOT pgrust
  (native-multi-process-Postgres-in-Rust, no-fork()-under-WASM; has-browser-demo-WASM-but-prod-server-needs-thread/
  fork-concurrency) NOT nginx-class(TLS/privileged-ports/socket-opts=native). → honest-boundary, not-partial-failure.
- ★PHONE-FIT: only-WASM-phone-viable (WasmEdge-1.5ms-Android; Docker-~50ms+no-iOS/Android-without-root; Firecracker-
  125ms+needs-KVM-not-phone). RED-blueprint WR-01: port-as-component, granted-ONLY-required_scope()-WASI-imports, any-
  other-fs/socket-TRAPS-at-host-boundary(mirror-IP-02-R4).

## LANE D3 — microVM (only where genuinely needed)
- ★KEY-CORRECTION: agent-governance/index.ts(operator's-flagged-candidate) = PURE text/voice-policy-filter(gender/
  profanity/archetype/HARD-BAN), executes-NOTHING-untrusted → NEEDS-NO-SANDBOX. GENUINE-microVM-case = FUTURE dev-
  agent-tier port-adapters/3rd-party-MCP-servers(IP-01/IP-02 dev-agent-tier), code-dowiz-can't-vet, server-side.
- Firecracker(~125ms-boot, <5MiB/VM, Apache-2.0, powers-AWS-Lambda/Fargate+Fly-Machines) vs Cloud-Hypervisor(~200ms,
  more-features) vs Kata(OCI-wrapper-around-either, "VM=security-boundary"). D3-note: dowiz-decentralized-non-hyperscale
  → Kata-on-containerd-fits-operationally BUT Kata=OCI-container-wrapper → CONFLICTS-«zero-OCI» → для-zero-OCI-decision =
  Firecracker-direct OR unikernel(NanoVMs/OPS Phase-3), NOT-Kata.
- OVERKILL-stated-directly: kernel/server(trusted-compile-firewalled-Rust — VM-wrapping-isolates-trusted-from-itself);
  static-SPA(no-execution-surface); pgrust(risk=SQL/RLS-shaped-not-process-isolation-shaped). CRUX-matrix: dowiz-scope.rs-
  Resource×Action=already-structurally-WASM-Component-WIT-world-import-list → WASM-Components-architecturally-FREE-to-add,
  microVM=entirely-NEW-machinery. → WASM-for-untrusted-code-you-control-compilation-of; microVM-ONLY-for-untrusted-NON-
  WASM(arbitrary-Linux-process, syscall-level).
- ★EDGE/PHONE: KVM-hard-required → courier-phone CATEGORICALLY-blocked(mobile-firmware-EL1-never-EL2, OEM-signed); owner-
  small-box-possible-but-fragile-Pi-class. → microVM=SERVER-CLASS-HUBS-ONLY.
- Verdict: microVM=targeted-opt-in-fallback-tier-for-non-WASM-untrusted-code-ONLY, NOT-Docker-replacement.

## LANE D4 — hardened-images/engine/build (container-remainder → но-під-рішення=native/WASM)
- Dockerfile-ALREADY-chainguard/nginx(operator-idea-half-done). Chainguard(Wolfi, no-shell/pkg-mgr, daily-CVE, signed-
  SBOM): chainguard/static(nonroot-65532+/tmp+ca-certs vs-raw-scratch) для-static-Rust-binary. Alpine/musl=AVOID-runtime-
  threaded-Rust(~7x-allocator-slowdown). ★static-web-server(Rust-4MB-SPA-fallback)="delete-nginx-ship-4MB-Rust-binary"=
  under-decision→WASM-component/native-binary. pgrust: DON'T-inherit-upstream-convenience-image; risk=maturity-not-base.
- Podman+Buildah(rootless-daemonless; ★CVE-2026-34040-Docker-Engine-authz-bypass-CVSS-8.8) > nerdctl(still-daemon) >
  keep-Docker. kaniko=daemonless-CI-build. ⚠️Dokploy=Docker-Engine-API-no-Podman-backend. ★base-image>engine-swap(most-
  CVEs=OS-packages). BUILD(forward): kaniko/Buildah→syft-SBOM→trivy+grype→cosign-keyless-sign+SLSA. DEV: OrbStack(Mac).
- HONEST: container-remainder-narrow(pgrust/static-serve/CI); UNDER-operator-decision → static-serve=WASM/native, pgrust=
  native-systemd-process(trusted)-or-microVM-if-isolation-wanted, CI-build=cargo-wasm+microVM-rootfs+SBOM. CI-Playwright-
  visual(deterministic)=one-genuine-container-shaped-CI-need→Firecracker-microVM-or-keep-pinned.

## ★ SYNTHESIS FRAME (all 4 lanes + decision):
1. NODE=native-Rust-binary (already-plan, NOT-Docker/WASM-runtime/microVM) — runs-directly systemd/native-app.
2. WASM-DEFAULT(wasmtime; WasmEdge-phone): ports/adapters/plugins/agents/MCP-tools=wasm32-wasip2-components, WASI-cap=
   Scope, KernelFacade-free-at-instantiation. static-serve=WASM-component-or-native-Rust. wasmCloud-pattern-not-substrate.
3. microVM(Firecracker/unikernel, ONLY-genuine): untrusted-NON-WASM dev-agent-tier-adapters/3rd-party-MCP, server-class-
   hubs-only(KVM≠phone). NOT-kernel/SPA/pgrust. NOT-agent-governance(pure-filter). NOT-Kata(OCI).
4. NATIVE-process(no-container/VM): pgrust(systemd, trusted, risk=SQL/RLS), dev-tools.
5. ZERO-Docker/OCI. Build=cargo→wasm-component+native-binary+Firecracker-rootfs/unikernel; supply-chain=syft/Trivy/
   Grype/cosign. Formalizes-sovereign-3-phase-ladder(Phase1-Docker-DROPPED, Phase2-WASI-default, Phase3-unikernel-tier).
Sources: wasmtime/WasmEdge/Spin/wasmCloud/Wasmer, WASI-p2/Wassette/Extism, Firecracker/Cloud-Hypervisor/Kata, Chainguard/
static-web-server/Podman/CVE-2026-34040/kaniko/trivy-grype/syft/cosign/OrbStack, pgrust, sovereign-node-3-phase-docs.
