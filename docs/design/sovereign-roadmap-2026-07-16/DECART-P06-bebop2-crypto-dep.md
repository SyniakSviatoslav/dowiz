# DECART — P06 real signing: how dowiz `ci-truth` consumes bebop2 PQ crypto

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion). **bebop today: self-hosting, gen3 == gen4 fixpoint, the DAG runtime DG1-DG10 LANDED, `bebop.bin 8a0b4325`; `bebop2/` is the July line and is kept unchanged; status in `bebop-lang/ROADMAP.md` (banner dated 2026-10-02).**


> Standing rule (operator, 2026-07-14): every new integration / dependency MUST pass a decart
> evaluation first and leave a comparison report in the introducing change. No silent adoption.
> This report gates the dowiz → bebop2 crypto dependency that Wave E (P06 `HybridSigner`) requires.
> It is written BEFORE the code lands, per the Detailed Planning Protocol (DECART inline, before code).

## The integration under evaluation

P06's V1 merge-gate currently runs **unsigned** (`signed:false` at `tools/ci-truth/src/main.rs:423`).
Real `key_K`/`key_V` signatures need Ed25519 + ML-DSA-65. The substrate already exists in the
**bebop2** repo (`bebop2/core/src/sign.rs`, `bebop2/core/src/pq_dsa.rs` — ACVP-KAT-verified, zero-dep,
wasm-empty-import). dowiz currently has **zero** bebop2 dependency. The question: how does
`ci-truth` (a pure-std Rust binary) obtain that crypto without reinventing it or breaking supply-chain?

## Candidates

| Criterion | A: path/dep `bebop2-core` (MIT) | B: path/dep `bebop-proto-cap` (AGPL-3.0) | C: reimplement Ed25519+ML-DSA in dowiz |
|---|---|---|---|
| Fit to sovereign bare-metal core (Rust/WASM, zero-dep) | exact: `bebop2-core` is already core+alloc, empty-import wasm, zero deps | pulls AGPL crate; same engine underneath | re-implements from scratch — duplicative |
| Correctness & security (falsifiable) | ACVP KAT in `pq_dsa/acvp_tests.rs`; RFC 8032 §7.1 KAT in `sign.rs`; C4b constant-time gate being closed now | same engine, wrapped | would need its OWN KAT + constant-time gate — high risk of a *weaker* unaudited impl |
| Performance | measured: existing KAT suite runs in-ci, no regression | same | unknown; re-impl cost |
| Supply-chain & license | `bebop2-core` = **MIT**, `cargo-deny` clean, no C build | **AGPL-3.0** — dowiz crates are all **MIT**; linking AGPL from `ci-truth` creates copyleft entanglement (dowiz would have to ship under AGPL or keep `ci-truth` AGPL) | self-contained but a new unaudited crypto surface |
| Maintainability & clarity | reuse one vetted impl; single source of truth for signatures | reuse + an extra indirection (hybrid wrap) | two codebases to keep in lockstep |
| Reversibility (port / adapter / fallback) | `bebop2-core` is a sibling crate; can vendor or path-dep; swappable | same | not reversible without deleting |
| Evidence cited | `bebop2/core/Cargo.toml:2` (MIT, zero-dep); `pq_dsa/acvp_tests.rs`; `sign.rs` RFC8032 KAT | `bebop-proto-cap/Cargo.toml:5` (AGPL) | n/a |

## Decision

**DECISION: A — `bebop2-core` (MIT) as a path/dep from `ci-truth`, composing the Ed25519⊕ML-DSA hybrid
inline** (reuse `sign.rs` `keygen`/`sign` + `pq_dsa.rs` `sign_pq`/`verify_pq`, exactly as
`proto-cap/src/hybrid_gate.rs` already does). **NOT B** (AGPL-3.0 — copyleft entanglement with MIT
dowiz, banned by supply-chain/license criterion). **NOT C** (re-implementing audited PQ crypto is the
exact "fake/reinvent" trap AGENTS.md forbids — never fake crypto; reuse the KAT-gated primitive).

- **Tiebreak:** A is both the Rust-native default AND license-clean; B fails the license criterion on
  merit; C fails the "never reinvent audited crypto" rule.
- **Older-as-adapter:** the existing `agent-governance-wasm/Cargo.toml:15`
  (`bebop2-core = { path = "/root/bebop-repo/bebop2/core" }`) is the **precedent to fix, not follow** —
  an absolute path breaks CI/portability. The adopted form is a **relative** path dep
  (`{ path = "../../bebop-repo/bebop2/core" }`) OR a vendored copy under `vendor/bebop2-core/` committed
  to dowiz so CI has no network and no cross-checkout path. The cross-checkout path-dep is a *bridge*;
  vendoring is the committed fallback if the path dep proves fragile in CI.
- **Probe (strongest honest argument against A):** "A path-dep on a *sibling checkout* at `../../bebop-repo/...`
  is fragile — it hard-codes repo layout, breaks in CI, and couples dowiz builds to an unpinned bebop2
  HEAD." This is a REAL concern and the reason for the older-as-adapter note: the mitigation is to
  **vendor `bebop2-core` into dowiz** (copy the `core/src` + `Cargo.toml` under `vendor/bebop2-core`,
  committed, deny-clean, offline) so P06 consumes a pinned, CI-safe copy while bebop2 remains the
  upstream source of truth. Vendoring is chosen as the *committed* form; the relative path-dep is the
  dev convenience. This is structurally the same "older-as-adapter / bridge" pattern the rule mandates.

## Gating chain this decart unblocks

1. **C4b (bebop2, in flight):** constant-time `mod_l` must be GREEN before `sign.rs` is fit for
   production signing. Until then, `ci-truth` keeps `UnsignedSigner` default — the `HybridSigner` slot
   exists but is **not selected**.
2. **Wave E (dowiz):** add `ci-truth/src/v1.rs` `struct HybridSigner` implementing the `Signer` trait by
   calling vendored `bebop2-core` `sign`/`pq_dsa`; wire `v1-verify` to verify a real signature when
   present. Ships behind the decart above. Requires bebop2-core vendored first.
3. **No change to `signed:false` default** until both C4b-GREEN and the vendored dep are in; fails-closed
   by design (matches blueprint §0 hard precondition).
