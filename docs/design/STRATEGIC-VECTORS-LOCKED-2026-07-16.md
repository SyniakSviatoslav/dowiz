# Strategic Vectors — LOCKED (2026-07-16)

> Operator + agent dialog, 6 decisions. Each vector is a FIXED direction, revisitable only via
> the escape clause named per-vector. This document is the canonical strategy anchor
> (merge-target, NOT append — supersedes sprawling arc-notes where it contradicts them).
> Grounded in: SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16, BRAIN-TOPOLOGY-ORG-PSYCH-EMERGENCE-RESEARCH-2026-07-16,
> MEMORY.md, and direct code reads (field_frame.rs/attention.rs/iroh_transport.rs/Dockerfile).

## V1 — Independence mechanism (the residue): A+B hybrid
- **A** ML-DSA split-identity: every diff signed by key_K, every verify/review by key_V
  (operator-gated genesis, like MESH-12). GREEN/RED cannot be authored by the same key.
  Reuses built bebop2 PQ stack (ML-DSA-65, C8/C4/C6/C7b).
- **B** Adversarial verifier: each diff auto-dispatches an independent-context verifier
  that re-executes cargo test / node-test / playwright and emits RED|GREEN with rationale.
  Merge blocked without verifier signature.
- **Escape:** identity-separation ≠ person-separation (Fable residue unknown). If a second
  human reviewer becomes available, they take the V-role. Until then: A+B is the enforced
  approximation, logged as such.

## V2 — Tech stack as law (+DECART-escape): A+escape
- Core LOCKED: Rust/WASM kernel + Trait-as-Port + content-addressing(sha3) + GPU only as
  offline/behind-a-port capability (NEVER in-kernel, NEVER in request path).
- Permanent rejections (cross-corroborated in code, not opinion): managed-cloud-default,
  Kubernetes (zero-OCI), GraphQL-as-mesh-protocol, IAM/reputation-trust (capability-tokens only),
  literal GPU/CUDA, digital-MCU-as-description (north-star only).
- **Escape:** each rejection re-openable ONLY via honest falsifiable DECART comparison-prototype
  (modern/rust-native default + tiebreak). Not a ban; a gated revisitation.
- ACTION: consolidate 20+ arc-files into ONE ARCHITECTURE.md via MERGE (not append) — direct
  answer to Fable Pattern 1 (append-only law). Stale arc-notes archived, not layered.

## V3 — Governance gates: B+trigger
- Restore LOAD-BEARING gates as BLOCKING CI: secrets(gitleaks), i18n, IDOR, OTP, + dormant
  bebop wires (no-courier-scoring/crdt-fence/kernel-fence pre-commit; empty-import/claim-live-test CI).
- Each future suspension (operator OR system) MUST carry a falsifiable reinstatement-trigger,
  in the form of the rsa/RUSTSEC-2023-0071 triage (named owner + checkable revisit condition).
  A governance change without a trigger is, by VERIFIED-BY-MATH, an unshipped RED.
- "Structuring" discipline (enumerate/bound/prove/close) supplied by V1-B verifier, not
  separate advisory gates.

## V4 — Work organization: D+B + self-development focus
- SPLIT-TRACK: stable (product, gated by V3, prepares ADR-020) vs experimental (kernel-growth,
  NEVER merges to main without explicit promotion).
- Self-development / growth = PRIMARY lens (operator directive 2026-07-13); experimental-track
  is its home; stable-track serves validated delivery.
- Each arc (stable or experimental) carries a CLOSURE-CRITERION: done-when + falsifiable
  evidence + strand/archive condition (reuses V3-trigger form).

## V5 — Verification & ship discipline: B+C
- **B** claim-latency statistic: time(diff-landing → GREEN-claim) logged per commit;
  anomalies (e.g. 52s self-green) flagged for sample-audit. Observation, not block.
- **C** V1-B verifier ENFORCED on red-line/money/orders (P0-A4 saga/reversal, auth/RLS/migrations):
  independent re-execution required before merge. Non-red-line commits rely on existing
  cargo-test/eqc CI + B-metric.
- VERIFIED-BY-MATH guard stays: works? provable? falsifiable? → ship RED otherwise.

## V6 — Future / ecosystem strategy: C+D
- DUAL-TRACK balanced: stable = delivery (G11 first real order) + bebop PQ-protocol as
  HEADLINER (ML-DSA infra, ADR-020 open-source); experimental = kernel-growth/self-development.
- Neither track dominates; V4-D isolates risk (experimental never silently merges to main).
- Metaphor discipline (Fable Pattern 6): "emergent/swarm/organism" ONLY adjacent to a named,
  computed criterion (SLEM exemplar); else "designed coordination."

## Reconciliation note (no cognitive dissonance)
- V1 resolves "no second party" via enforced approximation (logged honestly).
- V2 resolves stack-drift via single merged ARCHITECTURE.md + DECART-escape (not permanent ban).
- V3 resolves gate-suspension via trigger-discipline (rsa-triage form) + CI enforcement.
- V4 resolves arc-sprawl via split-track + closure-criterion (fractal-seed answer).
- V5 resolves self-green via claim-latency + red-line verifier.
- V6 resolves product-vs-growth via dual-track (growth in experimental, delivery validated in stable).

All six vectors point one direction. Revisit any via its named escape clause, with falsifiable
evidence — never by overlay.

---

## EXTENDED ECOSYSTEM DECISIONS (E-series, operator dialog 2026-07-16)

### Web-research evidence (terminal curl, Firecrawl blocked — no credits)
- llama.cpp: 120,590★ MIT, "LLM inference in C/C++", GGUF self-host standard.
- vLLM: 86,420★ Apache-2.0, PagedAttention, OpenAI-compatible serving.
- Modal GPU: H100 $0.001097/s (~$3.95/hr), A100-80G $0.000694/s, L4 $0.000222/s, scale-to-zero, $30/mo free. Confirms P2-B1 (~$1.25-1.67/job).
- NIST FIPS 204 (ML-DSA): finalized 2024-08-13. Confirms PQ-strategy.
- arXiv: context-engineering / multi-agent / tool-learning agent-infra papers exist (E13/E20 direction).

### E1–E12 (operator locked)
E1 hub-ring+sparse-P2P(C) | E2 GH-Actions+gitleaks+V5(A) | E3 vendored+cache+audit(A)
E4 demo=wasm now, video AFTER GPU-unlock(B) | E5 GitHub-Discussions+AGPL+X/Telegram-lite(A+B)
E6 "sovereign PQ delivery infra"(A) delivery=by-product | E7 B2B+grants(A+C)
E8 single-graph wiki BD+spectral+history(A) | E9 agent=Hermes-tool+verifier(A) | E10 ML-DSA hybrid(A)
E11 native+pgrust-backup(A) | E12 UA+EN+AL, EN-main, all-locales OSS(A)

### E13–E62 (agent-proposed locks, operator "lock all")
Full Descartes-quadrant per item in session dialog. Cluster summary:
- Agent infra/models (E13-20): self-host llama.cpp/vLLM GOAL; managed-advisory until GPU; harmonic+kelly tiering; spectral+BD memory; MCP; per-agent capability-tokens; TokenBucket; paired-debate.
- Compute/GPU (E21-25): offline-port; Modal scale-to-zero; webgl/webgpu feature-gated; SIMD f64x4; NUMA core-pinning.
- Storage (E26-30): pgrust=backup/fallback; COLD zstd; event-replay; sha3 cache; deep-clean cron.
- Network/mesh (E31-35): iroh+HRW; Dijkstra/A*; Union-Find/MST; iroh-quinn NAT; 3-tier locality.
- Security (E36-40): ML-DSA hybrid; operator-gated genesis; RevocationSet; signed event_log; EnvFile+gitleaks.
- Product (E41-45): physics/math wasm; openbebop existing; web-first; WCAG; wasm-demo→video.
- Ops (E46-50): tracing+OTel; claim-latency alerts; H8 runbook; OpenTofu; COLD-restore.
- Community (E51-55): single-graph wiki; DCO+AGPL; rsa-triage; AGPLv3+TM(partial); Manifesto.
- Business (E56-60): B2B+grants; usage-PQ-api; NO-COURIER-SCORING; TM(pending EUTM); EU/UA.
- Growth (E61-62): kernel math-first; DECART-gate deps.

### MESH-FOUNDATION pivot (M-series, 2026-07-16 operator clarification)
Mesh = FOUNDATION of everything; dowiz = delivery service ON TOP of protocol.
M1 mesh=foundation-not-addon | M2 ML-DSA-65+ML-KEM-768 FIPS204/203 KAT+classical-fallback
M3 quantum-noise=OPTIONAL (QRNG beacon, off-default) | M4 every-edge-autonomous self-cert
M5 every-hub=autonomous-HYDRA (own rules/ports/bridges/models/API/MCP/agents) | M6 ZERO protocol deps (proto-cap/pq_kem/matcher std-only)
M7 no-SPOF (Dijkstra/A*+Union-Find/MST heal) | M8 local-only typed CPU/GPU metrics (no surveillance)
M9 kill-switch+flexible-access ONLY (no other global control) | M10 inter-hub=protocol, intra-hub=anarchy
M11 living-organism UNBOUNDED ("нахуй систему" = no layer above hubs but wire-format) | M12 capability=proto-cap (ML-DSA fail-closed, reject UCAN as heavier).

### NEXT 50 (F-series, 2026-07-16) — concrete situations w/ consequences+pro/con
F1-10 autonomy/hub-rules | F11-20 mesh/transport | F21-30 security/quantum | F31-40 observability/cost | F41-50 product/delivery-on-protocol.
Each: situation(possible/impossible) · now · future · pro · con · LOCK. Full text in ARCHITECTURE.md §6.

### HYDRA-CONTRADICTION SWEEP (2026-07-16, operator "дозвіл на війну")
All global-sounding gates re-scoped DEV-TIME-only via SCOPE RULE in ARCHITECTURE.md §0.
C1 V3-CI, C2 V5-verifier, C3 E12-i18n, C5 E26-pgrust, C6 S4-API, C8 V1-verifier → none block hub autonomy (canonical-repo fence, hub MAY override).
C4 single-graph-wiki → PER-HUB REPLICATED (no central SPOF). C7 TM → brand-leash only, protocol/runtime free.
ZERO residual contradiction with M5/M9/M11. "нахуй систему" = no layer above hubs but wire-format.

### Total locked: V1-6 + D1-8 + S1-9 + E1-62 + M1-12 + F1-50 = 147 anchors.
Single source: ARCHITECTURE.md (canon) + this doc. Arc-notes = history, not law.
