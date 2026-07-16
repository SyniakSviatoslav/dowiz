# ARCHITECTURE — dowiz / DeliveryOS + bebop (CANONICAL, 2026-07-16)

> Single source of truth. Supersedes sprawling arc-notes (see STRATEGIC-VECTORS-LOCKED-2026-07-16).
> Merge, never append. Revisit rejections ONLY via DECART-escape (falsifiable comparison-prototype).
> Grounded in: code reads (field_frame.rs/attention.rs/iroh_transport.rs/Dockerfile/native-spa-server/event_log.rs),
> SYNTHESIZED-BLUEPRINT-PLAN, BRAIN-TOPOLOGY research, + web-verified facts (llama.cpp 120k★ MIT,
> vLLM 86k★ Apache-2.0, Modal H100 $0.001097/s, NIST FIPS204 ML-DSA finalized 2024-08-13).

## 0. Strategic vectors (locked, see STRATEGIC-VECTORS-LOCKED)
V1 independence=ML-DSA split-identity+adversarial verifier. V2 stack=law+DECART-escape.
V3 governance=load-bearing gates blocking CI+rsa-triage trigger. V4 org=split-track+closure-criterion, self-dev PRIMARY.
V5 verify=claim-latency+red-line verifier. V6 future=dual-track(delivery G11+bebop PQ-protocol)+kernel-growth.

## 1. Tech stack (LAW, D-series + web-facts)
- **Core:** Rust/WASM kernel + Trait-as-Port + content-addressing(sha3) + GPU offline/port-only.
- **Rejections (DECART-gated):** managed-cloud-default, K8s(zero-OCI), GraphQL-mesh, IAM/reputation(NO-COURIER-SCORING), literal-GPU/CUDA, digital-MCU-as-now.
- **DB (D1):** native vectorless index DEFAULT; pgrust OPT-IN (feature flag OFF) as backup/fallback read-model (E26). BlockStore content-addressed.
- **Network (D2/E1):** iroh-QUIC primary mesh; custom-QUIC(quinn) fallback allowed via DECART (C-fallback). deny-by-default capability-tokens. Hub-ring topology (E1-C), NO-COURIER-SCORING (topology not reputation).
- **LLM infra (E13/E14) [web-verified]:** self-host llama.cpp (120k★ MIT, GGUF, C/C++) + vLLM (86k★ Apache-2.0, PagedAttention, OpenAI-API) on owned box as GOAL. Until GPU-unlock: HK05 dev-routing uses managed API (advisory, V2-managed-cloud=adapter-not-architecture). Modal (H100 $0.001097/s, scale-to-zero) = GPU-port provider (E21/E22), P2-gated on volume.
- **Models (E15):** harmonic_centrality+kelly_fraction adaptive tiering (built, wire П0-C1). Tiny models (SmolLM-class) for edge agents.

## 2. Service architecture (S-series) [code-grounded]
- **Runtime (S1):** zero-OCI native static binaries (`FROM scratch`, check-zero-oci.sh DK-08) + systemd units (pgrust.service). MicroVM(Firecracker/KVM) when fleet>5 (P0-A6/OpenTofu).
- **Topology (S2):** modular monolith (kernel+engine+web, Trait-as-Port isolation). Microservices/microVM per P1/P0-A6.
- **Config/secrets (S3):** systemd EnvironmentFile + NEVER in-repo + gitleaks CI (П0-A3/V3). (LICENSE=Apache-2.0 in tree vs ADR-020 AGPLv3 mandate — D3 partial: non-destructive files ready, force-push scrub BLOCKED red-line, EUTM pending operator.)
- **API (S4):** gRPC/protobuf INTERNAL + REST/JSON edge. GraphQL client-edge ONLY (never mesh).
- **Errors (S5):** fail-closed + explicit Result always.
- **Deploy (S6):** single-env (operator choice) — BUT V5-C red-line verifier + П1-B still enforced on money/auth/RLS/migrations.
- **Testing (S7):** eqc(unit math)+cargo-test(kernel)+node-test(wasm)+Playwright(staging-if-exists).
- **Observability (S7/S8/D7):** tracing + Envelope.trace + OTel later (P1).
- **Money/orders (S9):** integer money + event-sourcing-NOT-CRDT + saga-compensation (П0-A4). BLOCKING per V3.

## 3. Code patterns (D6/E62) — ratify, new pattern ⇒ eqc or DECART
Trait-as-Port | content-addressing(sha3) | eqc VERIFIED-BY-MATH | deny-by-default | event-sourcing(money/orders) | closure-criterion(V4) | DECART-gate new deps.

## 4. Ecosystem-wide (E-series)
- **Hub (E1):** sparse hub-ring + P2P fallback. **Pipelines (E2/E3):** GitHub Actions + pre-commit + zero-oci + gitleaks + V5 verifier; vendored/cached crates + cargo-audit.
- **Demo (E4):** wasm-demo now (native-spa-server + math render). Video/WebGL2 splat AFTER GPU-unlock (W21 ceiling). **GPU-unlock (E4-B):** track wgpu-cache; when `cargo add wgpu` succeeds → real sink.
- **Social (E5):** GitHub Discussions + AGPL-community (dev-first) + X/Telegram lite (announcements).
- **Marketing (E6):** "Sovereign PQ delivery infrastructure" — delivery = by-product of infrastructure ecosystem.
- **Money sustain (E7):** protocol-B2B licensing + grants(NLnet). tx-fee after G11.
- **Docs (E8):** single-graph wiki (BD + spectral + change-history + index). **Agent infra (E9/E13-20):** Hermes-subagent-as-tool + П1-B verifier; MCP(bebop port); capability-tokens per-agent; TokenBucket cost-control; critical/naive paired debate (Fable G-method).
- **Security (E10/E36-40):** ML-DSA-65 hybrid + classical-fallback; operator-gated genesis; RevocationSet; signed event_log; systemd EnvFile+gitleaks.
- **Product (E41-45):** dowiz UI = deterministic physics/math wasm (D4); openbebop = existing design; web-first responsive; WCAG via native-spa; wasm-demo→video.
- **Ops (E46-50):** tracing+OTel; cron-clean+claim-latency alerts; H8-SECRET-SCRUB-RUNBOOK; OpenTofu when fleet>1; COLD-restore+git-bundle.
- **Community (E51-55):** single-graph wiki; DCO+AGPL; rsa-triage trigger; AGPLv3+TM (D3); Manifesto.
- **Business (E56-60):** protocol-B2B+grants; usage-based PQ-api; NO-COURIER-SCORING law; TRADEMARK.md (EUTM pending); EU/UA GDPR-aware.
- **Growth (E61-62):** kernel math-first (eqc/S0.5); DECART-gate all new deps.

## 5. i18n / localization (E12)
UA + EN + Albanian NOW; EN = main lingua; ALL locales via OSS libs/API/models (LibreTranslate/Argos, crowd). i18n = blocking CI gate (V3 load-bearing, 12× recurring defect).

## 6. Extended 50 decisions (E13–E62) — summary locks
[Full Descartes-quadrant per item in STRATEGIC-VECTORS-LOCKED + session dialog. Key locks:]
E13 agent=Hermes-tool+verifier | E14 LLM=self-host llama.cpp/vLLM goal, managed-advisory-until-GPU
E15 tiering=harmonic+kelly (wire) | E16 memory=spectral+BD single-graph | E17 MCP(bebop) | E18 agent-capability-tokens
E19 TokenBucket cost | E20 paired-debate | E21 GPU=offline-port | E22 Modal scale-to-zero | E23 webgl/webgpu feature-gated
E24 SIMD f64x4 | E25 core-pinning(NUMA) | E26 pgrust=backup/fallback | E27 COLD zstd+content-addr
E28 event-replay migration | E29 sha3 cache | E30 deep-clean cron | E31 iroh+HRW | E32 Dijkstra/A* | E33 Union-Find/MST
E34 iroh-quinn NAT | E35 3-tier locality | E36 ML-DSA hybrid | E37 operator-gated genesis | E38 RevocationSet
E39 signed event_log | E40 EnvFile+gitleaks | E41 physics/math wasm | E42 openbebop existing | E43 web-first
E44 WCAG | E45 wasm-demo→video | E46 tracing+OTel | E47 claim-latency alerts | E48 H8 runbook | E49 OpenTofu
E50 COLD-restore | E51 single-graph wiki | E52 DCO+AGPL | E53 rsa-triage | E54 AGPLv3+TM(partial)
E55 Manifesto | E56 B2B+grants | E57 usage-PQ-api | E58 NO-COURIER-SCORING | E59 TM(pending EUTM)
E60 EU/UA | E61 kernel math-first | E62 DECART-gate deps.

## 7. Honest gaps (not self-certified)
- ADR-020: LICENSE Apache-2.0 vs AGPLv3 mandate; force-push scrub BLOCKED (red-line); EUTM not filed.
- GPU: wgpu uncached offline → W21 ceiling; GPU-unlock pending network.
- Web-research: Firecrawl blocked; facts via terminal curl (GitHub/NIST/Modal/arxiv). Modal pricing verified; llama.cpp/vLLM verified.
- Residue (Fable): V1 independence = enforced approximation (logged), not person-separation.
