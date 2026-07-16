# SWARM MANIFEST — execute blueprints from here (2026-07-15, v2)

> Standing goal (user verbatim): kernel/rust/wasm **async bidirectional mesh** + **vectorless indexes** +
> **pgrust** + **rustls/ring** + **replace bash/python with native kernel+adapters** + **spectral+energy math**
> (M∇²U+ΓU̇+c²LU=S) + **physics rendering instead of classic UI/FE** + **event-driven architecture**.
> Scope: **OpenBebop** (`/root/bebop-repo`, remote SyniakSviatoslav/OpenBebop) + **dowiz** (`/root/dowiz`) ONLY.
> Process (DEFAULT, binding): research→plan→blueprint → THIS manifest → dispatch parallel subagents FROM it,
> verify (cargo test fresh) + commit local → wave N+1, self-adjusting on live telemetry + retro/self-upgrade.

## Invariants (every subagent MUST honor)
- **Kernel immutable** — never modify `dowiz-kernel` decide/fold/Law/money authority. Add layers, don't touch authority.
- **Money never tween/float/CRDT-merge** — i64 by type, event-sourcing not CRDT.
- **RED→GREEN with a live test** — every unit ships a reachable red→green test; status only from live-test, never prose.
- **Reuse-first** (~90% exists) — extend scaffolds, don't rewrite.
- **Determinism** — fixed-K / fixed-order / f64 advisory; never gate a decision on an async push.
- **Do NOT `git commit` / `git push`** — orchestrator verifies + commits after evidence. Leave changes in-tree.
- **Red-line items (crypto/auth/money/RLS/migrations)** — verify-first against source + ship red→green test + leave LOCAL (no push). Operator gate before merge.

## Repo crate topology (ground truth)
- **dowiz** `/root/dowiz`: `kernel/` (dowiz-kernel, cdylib+rlib, wasm/serde UNgated), `engine/` (VertexBridge, wgpu-free), `tools/eqc/` (equation→Rust generator, built).
- **OpenBebop** `/root/bebop-repo`: workspace = `rust-core` + `crates/bebop` + `bebop2/core` + scaffolds `bebop2/proto-wire` (iroh+WSS), `bebop2/proto-cap`, `bebop2/proto-crypto`, `bebop2/delivery-domain`, `bebop2/mesh-node`, `bebop2/wasm-host` (excluded).

## Wave 1 — IN FLIGHT (dispatched, 6 parallel subagents) — non-red-line
| # | Unit | Repo / crate | RED→GREEN gate | Owns |
|---|---|---|---|---|
| W1-1 | MESH-01 feature-gate kernel | dowiz `kernel/Cargo.toml` + OpenBebop `bebop2/delivery-domain` | `cargo build -p bebop-delivery-domain` (no wasm feature) succeeds; kernel decide/fold/money diff=0 | ONLY `dowiz/kernel/Cargo.toml` + cfg gate in wasm.rs |
| W1-2 | MESH-08 CRDT compile-fence | OpenBebop (workspace + `ci/`) | build FAILS if order/money crate depends on CRDT-merge crate | NEW `ci/crdt-fence.rs` |
| W1-3 | retrieval M1 L0 trigram | dowiz `kernel/src/retrieval/` (NEW) | trigram index + regex-automata; ~20% corpus, 0 false positives vs ripgrep | NEW files under `kernel/src/retrieval/` |
| W1-4 | retrieval M3 L3 diffusion | dowiz `kernel/src/retrieval/` (NEW) | PPR via `markov.rs` fixed-K deterministic; recall vs M1 | NEW files under `kernel/src/retrieval/` |
| W1-5 | field-ui Phase 0 zero-copy | dowiz `engine/` (NEW) | flat `Vec<f32>`→view→1 writeBuffer, 0 JSON.parse in frame loop | NEW `engine/src/zerocopy.rs` |
| W1-6 | MESH-10 rustls TLS | OpenBebop `bebop2/proto-wire` | WSS→tokio-rustls TLS1.3; plaintext rejected; replay rejected; slowloris dropped (PQ payload SKIPPED) | EDIT `bebop2/proto-wire` |

## Wave 2 — NEW UPGRADES (from 2026-07-14/15 research) — dispatch after W1 verified
| # | Unit (source) | Repo / crate | RED→GREEN gate | Red-line? |
|---|---|---|---|---|
| W2-1 | eqc S1 — consolidate 4 Jacobi eigensolvers → 1 eqc-generated organ (math-first §P1; dual-authority finding) | dowiz `kernel/src/{spectral,field}` + OpenBebop `bebop2/core/src/{field,kalman}` | bit-exact parity vs the 4 divergent copies (f64+fixed-point); eigensolve divergence killed | no (math) |
| W2-2 | field-UI Laplacian-SpMV — ONE L-core serves 5 subsystems; wire VertexBridge→real wgpu buffer (FE-01) | dowiz `engine/` (CSR-SpMV compute-pass in Rust; GPU behind `gpu` feature) | one SpMV kernel; heat/diffusion/blur reuse same L; VertexBridge copy into real buffer | no (GPU deferred feature) |
| W2-3 | Kalman B1 — extend `ema_next`→full Kalman filter for courier geo state (SE(3)) | OpenBebop `bebop2/core/src/kalman.rs` | predict+update with gain H/R; geo transform + probabilistic correction | no |
| W2-4 | micrograd B2 — minimal autodiff (~200 LOC) for capture fitting | OpenBebop `bebop2/core/src/` (NEW `autodiff.rs`) | gradient-check vs finite-diff; offline-only | no |
| W2-5 | backup B4 — native Rust backup organ (content-addressed + FastCDC + ATTIC 2-phase) | OpenBebop `crates/bebop` or `bebop2/` (NEW `backup.rs`) | FastCDC deterministic; ATTIC 2-phase harden; verify-failure→retrieval trigger | no |
| W2-6 | retrieval M2 BM25 + A2/M0 living-knowledge recall resurrection | dowiz `kernel/src/retrieval/` + `spikes/living-knowledge/` | BM25 over fixture; recall engine indexes 174-file corpus, recall@5 re-proven | no |
| W2-7 | knowledge-spine P0 — OpenSpec revive + Rust in-kernel index + MAP.md | dowiz `docs/` + `kernel/src/retrieval/` | frontmatter validator; MAP.md generated; index 295 files | no |
| W2-8 | crypto P0 — H-1 ct_eq ML-KEM decap · H-2 iroh TLS-verify gate · H-3 domain-sep KDF hybrid id (security-review + bebop P0) | OpenBebop `bebop2/core/src/pq_kem.rs` + `proto-wire` | each: verify finding vs source → fix → red→green test (KyberSlash CT, TLS-verify-gated, seed KDF) | 🔴 RED-LINE crypto — verify-first + local-only + operator gate |

## Wave 3+ (after W2 verified) — higher-level integration
- MESH-07 sync pull (red-line sync-frame, defer) · MESH-09 iroh transport body (red-line) · field-ui Phase1 SDF · M4 living-memory→pgrust · physics wgpu render · event-log wiring · knowledge-spine P1-P4 · self-mod-effector (DESIGN ONLY, never activate) · hermes HK-00..06 (already done in EXCLUDED repo — skip unless user confirms).

## Verification contract (subagent returns)
- exact `cargo test` / `cargo build` output proving RED→GREEN
- absolute paths of files created/edited (must match Owns column)
- explicit "no red-line crossed" OR "red-line touched, local-only, awaiting operator gate" statement
