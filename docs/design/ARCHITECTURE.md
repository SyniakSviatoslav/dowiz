# SOVEREIGN ARCHITECTURE GUIDE — dowiz / DeliveryOS + bebop (CANON, 2026-07-16, MESH-PIVOT)

> Single source of truth. Merge, never append. Revisit ONLY via DECART-escape (falsifiable prototype).
> PIVOT 2026-07-16: **decentralized PQ mesh = FOUNDATION of everything.** dowiz = delivery service ON TOP of the protocol.
> Grounded in: bebop code (proto-cap ML-DSA-65 fail-closed, pq_kem.rs ML-KEM FIPS203 KAT, matcher.rs
> transport-agnostic, wasm-host capability-scoped) + SYNTHESIZED-BLUEPRINT + BRAIN-TOPOLOGY + web-verified
> (llama.cpp 120k★ MIT, vLLM 86k★ Apache-2.0, Modal H100 $0.001097/s, NIST FIPS204 finalized 2024-08-13).

## 0. MESH-FOUNDATION (M-series) — the substrate everything else rides on
- **M1** Mesh = FOUNDATION, not add-on. dowiz/bebop = services ABOVE the protocol. LOCK.
- **M2** PQ crypto: ML-DSA-65 (sig) + ML-KEM-768 (KEM), FIPS204/203, bit-exact KAT (in-repo). + classical-fallback (Ed25519/ECDH) for interop. LOCK.
- **M3** Quantum-noise (OPTIONAL layer): entropy from QRNG beacon (ANU/meetar) mixed into nonce/ephemeral-key gen → post-quantum + quantum-noise hardening. Off by default, operator-enabled. LOCK-as-optional.
- **M4** Every EDGE autonomous: self-certifying ML-DSA identity, signs own frames, no central CA. LOCK.
- **M5** Every HUB = autonomous HYDRA: may change OWN rules, open ports/bridges, use any models/API/MCP/agents at its discretion. Protocol defines ONLY inter-hub comms; intra-hub = hub's own business. LOCK.
- **M6** ZERO protocol dependencies: no external crate at the wire/trust boundary (proto-cap, pq_kem, matcher are zero-dep std-only). Transport swappable (iroh/quinn/HTTP/stdio) behind Trait. LOCK.
- **M7** No single point of failure: any node can drop; mesh heals via Dijkstra/A* + Union-Find/MST (P1). No leader election required for liveness. LOCK.
- **M8** Local-only metrics/logging: CPU+GPU telemetry gathered at-process, typed+filtered, NEVER exfiltrated (no surveillance). OTel-only-if-operator-opts-in, local sink. LOCK.
- **M9** Kill-switch + flexible access ONLY: operator may hard-kill a hub/subtree; access controls are per-hub configurable, not global. No other global control exists. LOCK.
- **M10** Inter-hub protocol = defined; intra-hub anarchy = allowed. Hub talks to hub via protocol; inside hub it does what it wants. LOCK.
- **M11** Living-organism experiment = UNBOUNDED: no caps, only kill-switch + configurable access. "нахуй систему" = no governing layer above hubs except the protocol wire-format itself. LOCK.
- **M12** Capability model (proto-cap, in-repo, 43★ UCAN rejected as heavier): ML-DSA-signed, fail-closed, nonce-replay, expiry, RevocationSet, red-line deny (auth/money/secrets/migrations). Per-agent scope. LOCK.

> ### SCOPE RULE (kills all global-vs-hydra contradictions)
> Every CI gate / policy below (V3, V5, E12, E8, E26, S4, E52/54) applies to the **canonical repo + operator's own build** ONLY.
> It is a DEV-TIME fence, NOT a runtime control. **At runtime every hub is a sovereign Hydra (M5/M9/M11):**
> it may fork, self-gate locally (eqc), ignore the upstream gate, change its DB/model/API/port, and replicate-or-reject any shared artifact.
> No dev-time gate may block a hub from self-evolving. If a gate reads as "global control", it is mis-scoped — re-read it as "canonical-repo recommendation, hub MAY override".

## 1. Tech stack (V2 + D + M)
- Core: Rust/WASM kernel + Trait-as-Port + content-addressing(sha3) + GPU offline/port-only.
- **Rejections (DECART-gated):** managed-cloud-default, k8s(zero-OCI), GraphQL-mesh, IAM/reputation(NO-COURIER-SCORING), literal-GPU/CUDA, digital-MCU-as-now.
- DB (D1): native vectorless DEFAULT for canonical build; **hub MAY promote pgrust (or any store) to primary at its discretion (M5)** — pgrust is OFF-recommendation, not a prohibition.
- Network (D2/M6): zero-dep proto-cap + iroh-QUIC primary, quinn fallback via DECART. deny-by-default capability-tokens.
- LLM infra (E13/E14, web-verified): self-host llama.cpp(120k★ MIT)+vLLM(86k★ Apache-2.0) GOAL; managed-advisory until GPU-unlock; Modal H100 $0.001097/s scale-to-zero. **V3/V5 governance gates = canonical-repo DEV-TIME fences (SCOPE RULE); at runtime hub self-enforces locally — no standing global verifier/gate (M5/M9).**
- Models (E15): harmonic_centrality+kelly_fraction adaptive tiering (wire П0-C1).
- Legal (E52/E54): AGPLv3 + TRADEMARK applies to **canonical code/brand ONLY** — protocol + runtime are free/unbounded (M11). TM is a brand leash, NOT a mesh-control; hub MAY fork code, drop brand, keep protocol. EUTM pending (operator action; see ADR-020).

## 2. Service architecture (S-series, code-grounded)
- Runtime (S1): zero-OCI native static binaries + systemd; microVM when fleet>5.
- Topology (S2): modular monolith; microVM per P1.
- Secrets (S3): systemd EnvFile + NEVER in-repo + gitleaks. (ADR-020: LICENSE now AGPLv3 since `ac1caba40`; secret scrub assessed CLOSED 2026-07-13, force-push declined as redundant — see `docs/adr/0020-oss-license-tm-dco.md`. EUTM still operator action.)
- API (S4): gRPC/protobuf internal + REST edge RECOMMENDED (not mandated); GraphQL client-edge ONLY. Hub MAY open any port/bridge at its discretion (M5) — inter-hub still speaks the protocol.
- Errors (S5): fail-closed Result always.
- Deploy (S6): single-env for operator's own build; V5-C red-line check = LOCAL per-hub re-exec, not a standing global service (see SCOPE RULE). Hub self-verifies.
- Observability (S7/S8/D7/M8): local tracing+typed CPU/GPU metrics; OTel opt-in local-only.
- Money (S9): integer + event-sourcing + saga-compensation. canonical-repo BLOCKING CI gate; at runtime hub enforces its own money law locally (M5).

## 3. Patterns (D6/E62): Trait-as-Port | content-addressing | eqc VERIFIED-BY-MATH | deny-by-default | event-sourcing | closure-criterion | DECART-gate new deps.

## 4. Ecosystem (E1–E62 summary, see STRATEGIC-VECTORS-LOCKED)
hub-ring(C) | GH-Actions+gitleaks | vendored+cache+audit | wasm-demo→video-after-GPU | GitHub-Discussions+AGPL | sovereign-PQ-infra(delivery=by-product) | B2B+grants | PER-HUB-REPLICATED-graph-wiki (no central SPOF) | Hermes-tool+verifier | ML-DSA-hybrid | native+pgrust-backup(MAY-promote) | UA+EN+AL(all-locales OSS) | agent-infra/models/GPU/storage/network/security/product/ops/community/business/growth clusters.

## 5. i18n (E12): UA+EN+AL now, EN-main, all-locales via OSS. **Canonical-repo blocking CI gate ONLY** (SCOPE RULE) — a hub may ship any locale set it wants.

---

## 6. NEXT 50 DECISIONS (F-series) — concrete situations, consequences, pro/con
Each: situation (possible/impossible) · consequence NOW · consequence FUTURE · pro · con · LOCK.
(Clusters; full Descartes per item in session dialog.)

### F-AUTONOMY / HUB-RULES (F1–F10)
- **F1** Hub changes its OWN routing policy mid-flight. SIT: possible (M5). NOW: local re-config, no global notice. FUT: network partitions tolerated. PRO: resilience. CON: debuggability. LOCK (M5).
- **F2** Hub opens a NEW inbound port for a bridge. SIT: possible. NOW: hub self-authorizes. FUT: port-scan surface grows. PRO: flexibility. CON: attack surface. LOCK + deny-by-default+rate-limit.
- **F3** Hub pulls a model from HuggingFace at runtime. SIT: possible (M5). NOW: model runs in capability scope. FUT: supply-chain risk. PRO: autonomy. CON: unverified weights. LOCK + sha3-verify-or-deny.
- **F4** Hub spins an MCP server it wrote itself. SIT: possible. NOW: local. FUT: protocol-clean if behind capability. PRO: extensibility. CON: proto-drift. LOCK + proto-cap-sign.
- **F5** Hub revokes another hub's trust. SIT: possible (RevocationSet). NOW: frames dropped. FUT: mesh heals. PRO: self-defense. CON: split-brain. LOCK.
- **F6** Hub uses a paid 3rd-party API (key in EnvFile). SIT: possible. NOW: billed to hub owner. FUT: cost leak. PRO: capability. CON: op-cost. LOCK + TokenBucket.
- **F7** Hub changes its consensus rule to "no-consensus" (pure anarchy). SIT: possible (M5/M11). NOW: hub isolates. FUT: island. PRO: experiment. CON: orphaned. LOCK (unbounded experiment).
- **F8** Hub bridges to a non-PQ legacy node. SIT: possible via classical-fallback. NOW: hybrid tunnel. FUT: PQ gap. PRO: interop. CON: weakens security. LOCK + flag-as-insecure.
- **F9** Hub auto-updates its own kernel from git. SIT: possible (M5). NOW: live patch. FUT: drift from canon. PRO: self-heal. CON: unverified. LOCK + eqc-gate-or-deny.
- **F10** Hub delegates to a sub-agent that opens its own sub-hub. SIT: possible (Hydra). NOW: recursion. FUT: depth blowup. PRO: emergent. CON: unbounded. LOCK + max-depth-cap.

### F-MESH / TRANSPORT (F11–F20)
- **F11** Two hubs disagree on wire format. SIT: impossible-by-protocol (M6). NOW: frame rejected. FUT: none. PRO: no lock-in. CON: strict. LOCK.
- **F12** Hub loses all peers, runs solo. SIT: possible. NOW: island mode. FUT: rejoins later. PRO: survives. CON: no sync. LOCK.
- **F13** QRNG beacon down → quantum-noise off. SIT: possible. NOW: falls back to ML-DSA-only. FUT: none. PRO: graceful. CON: less noise. LOCK (M3 optional).
- **F14** Hub uses webgl render of its own topology. SIT: possible (E23 feature-gated). NOW: local viz. FUT: GPU cost. PRO: insight. CON: GPU. LOCK + feature-gated.
- **F15** Mesh partitions, both sides think they're root. SIT: possible. NOW: two islands. FUT: merge via HRW. PRO: none-fatal. CON: dup. LOCK + HRW-merge.
- **F16** Hub encrypts traffic with ML-KEM to a peer it just met. SIT: possible (self-cert). NOW: 0-RTT-ish. FUT: none. PRO: zero-trust. CON: no reput. LOCK.
- **F17** Hub exposes gRPC internally + REST edge only. SIT: as-designed (S4). NOW: clean. FUT: none. PRO: boundary. CON: two stacks. LOCK.
- **F18** Hub batches 10k frames then flushes. SIT: possible. NOW: latency. FUT: throughput. PRO: eff. CON: lag. LOCK + tuned.
- **F19** Hub rejects a frame with unknown capability scope. SIT: fail-closed (M12). NOW: drop. FUT: none. PRO: safe. CON: false-neg. LOCK.
- **F20** Hub uses iroh for NAT punch, quinn if iroh down. SIT: possible (DECART fallback). NOW: seamless. FUT: none. PRO: robust. CON: 2 deps. LOCK.

### F-SECURITY / QUANTUM (F21–F30)
- **F21** Attacker has quantum computer. SIT: future-possible. NOW: ML-DSA holds (FIPS204). FUT: safe. PRO: PQ. CON: none. LOCK.
- **F22** + quantum-noise layer on. SIT: optional (M3). NOW: nonce hardened. FUT: even if PQ broken, noise adds. PRO: defense-in-depth. CON: beacon dep. LOCK-optional.
- **F23** Private key leaks. SIT: possible. NOW: RevocationSet kills it. FUT: re-gen. PRO: recoverable. CON: window. LOCK.
- **F24** Hub signs with expired capability. SIT: impossible-accepted (M12). NOW: rejected. FUT: none. PRO: safe. CON: re-issue overhead. LOCK.
- **F25** Replay attack with old nonce. SIT: impossible-accepted. NOW: dropped. FUT: none. PRO: safe. CON: state. LOCK.
- **F26** Red-line scope (money) in capability. SIT: denied by policy (M12). NOW: hard-reject. FUT: none. PRO: floor. CON: operator must sign. LOCK.
- **F27** Hub runs unaudited model. SIT: possible (M5). NOW: runs. FUT: risk. PRO: autonomy. CON: unverified. LOCK + sha3-gate.
- **F28** Operator kill-switch hits a hub. SIT: possible (M9). NOW: hub dies. FUT: restartable. PRO: control. CON: data loss if no COLD. LOCK + COLD-backup.
- **F29** Hub logs to remote OTel. SIT: possible but M8 says NO surveillance. NOW: denied default. FUT: opt-in local. PRO: privacy. CON: less central vis. LOCK (local-only).
- **F30** Hub encrypts its local DB with XChaCha20-Poly1305. SIT: possible. NOW: at-rest safe. FUT: none. PRO: safe. CON: key mgmt. LOCK + EnvFile-key.

### F-OBSERVABILITY / COST (F31–F40)
- **F31** Metrics gathered per-process CPU+GPU. SIT: as-designed (M8). NOW: local. FUT: trend. PRO: no surveillance. CON: no central. LOCK.
- **F32** Strict type-filter on every log line. SIT: possible. NOW: typed. FUT: queryable. PRO: clean. CON: boilerplate. LOCK.
- **F33** Hub hits GPU budget. SIT: possible. NOW: TokenBucket throttles. FUT: queue. PRO: cost-bound. CON: slow. LOCK.
- **F34** Hub uses Modal H100 at $0.001097/s. SIT: possible (E22). NOW: billed/sec. FUT: scale-to-zero. PRO: no idle. CON: cost. LOCK + budget-ceiling.
- **F35** Hub runs tiny SmolLM on edge. SIT: possible. NOW: cheap. FUT: fine for class. PRO: edge. CON: weak. LOCK.
- **F36** Claim-latency anomaly alert. SIT: possible (E47). NOW: alert. FUT: caught. PRO: verify. CON: noise. LOCK.
- **F37** Hub prunes its own state via deep-clean. SIT: possible (E30). NOW: shrinks. FUT: healthy. PRO: self-maint. CON: lost history. LOCK + COLD-first.
- **F38** Hub backs up to COLD zstd. SIT: possible (E27). NOW: archived. FUT: restorable. PRO: safe. CON: disk. LOCK.
- **F39** Hub exports metrics to Grafana. SIT: operator-opt-in only. NOW: denied default. FUT: local. PRO: privacy. CON: vis. LOCK (local).
- **F40** Hub self-reports to operator via signed envelope. SIT: possible. NOW: trace. FUT: audit. PRO: verify. CON: overhead. LOCK.

### F-PRODUCT / DELIVERY (dowiz on protocol) (F41–F50)
- **F41** dowiz order routed over mesh hub-ring. SIT: as-designed (E1). NOW: routed. FUT: PQ-safe. PRO: sovereign. CON: latency. LOCK.
- **F42** Proof-of-Delivery signed by edge ML-DSA. SIT: possible. NOW: attest. FUT: dispute-proof. PRO: trustless. CON: key mgmt. LOCK.
- **F43** Courier paid via integer-money saga. SIT: as-designed (S9). NOW: settle. FUT: no float. PRO: exact. CON: complexity. LOCK.
- **F44** Hub disputes an order. SIT: possible. NOW: arbitration via protocol. FUT: resolved. PRO: fair. CON: slow. LOCK + escrow.
- **F45** Route computed Dijkstra/A* over geo. SIT: gap-fill (P1). NOW: implement. FUT: optimal. PRO: real. CON: compute. LOCK + wire.
- **F46** Partition-tolerant delivery (Union-Find/MST). SIT: gap-fill (P1). NOW: implement. FUT: survives split. PRO: robust. CON: dup-risk. LOCK + wire.
- **F47** Demo = wasm physics/math render of a delivery. SIT: possible (E4/E41). NOW: demo. FUT: GPU after unlock. PRO: honest. CON: no video yet. LOCK.
- **F48** PER-HUB replicated graph-wiki (E8→corrected): each Hydra head keeps its OWN graph (BD+spectral+history), syncs opportunistically over protocol, NO central authority. SIT: possible. NOW: no SPOF. FUT: emergent knowledge. PRO: survives hub loss, hub owns truth. CON: dedup/merge cost. LOCK (corrected from single-central).
- **F49** i18n UA/EN/AL for courier app. SIT: as-designed (E12). NOW: 3 langs. FUT: all via OSS. PRO: access. CON: maint. LOCK.
- **F50** Living-organism = hubs freely evolve, only kill-switch bounds. SIT: as-designed (M11). NOW: unbounded. FUT: emergent. PRO: no ceiling. CON: chaotic. LOCK (unbounded).

---

## 7. Total locked: V1-6 + D1-8 + S1-9 + E1-62 + M1-12 + F1-50 = 147 anchors.
Single source: this guide + STRATEGIC-VECTORS-LOCKED. Arc-notes = history, not law.

## 8. Honest gaps (not self-certified)
- **HYDRA-CONTRADICTION SWEEP (2026-07-16, operator "дозвіл на війну"):** ALL global-sounding gates re-scoped to DEV-TIME-only via SCOPE RULE. C1(V3 CI) C2(V5 verifier) C3(E12 i18n) C5(E26 pgrust) C6(S4 API) C8(V1 verifier) → none block hub autonomy. C4 (single-graph wiki) → PER-HUB REPLICATED (no central SPOF). C7 (TM) → brand-leash only, protocol/runtime free. Zero residual contradiction with M5/M9/M11.
- ADR-020: LICENSE now AGPLv3 (flipped `ac1caba40`); secret scrub assessed CLOSED 2026-07-13 (force-push declined as redundant) — see `docs/adr/0020-oss-license-tm-dco.md`. EUTM pending (operator action).
- GPU: wgpu offline-ceiling (W21); GPU-unlock pending network.
- QRNG: ANU/meetar beacon reachable but api.quantum-random.com down; M3 optional, ML-DSA-only fallback confirmed.
- Web: Firecrawl blocked; facts via terminal curl (GitHub/NIST/Modal/arxiv). llama.cpp/vLLM/Modal verified.
- Residue (Fable): V1 independence = enforced approximation (logged), not person-separation.
- "нахуй систему": realized as M9/M11 — NO governing layer above hubs except the wire-format; kill-switch + flexible per-hub access are the ONLY global controls.
