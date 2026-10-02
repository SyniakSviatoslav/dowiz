# BLUEPRINT — Item 43: Constant-Time Inference Gate (plane-classified; dudect design owed)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — constant-time is
  the ruling's side-channel face: **execution time must not leak input**. For a *public-plane* pilot
  the leak has no adversary, so the gate is cheap-but-optional — **but the mandatory dudect design is
  specified now** so a future secret-adjacent pilot never bolts safety on late.
- **Sources read this session:** roadmap §H item 43 (lines 594–605) + item 34 scope-consequence
  (lines 524–528: the toy pilot's input plane is **public/synthetic by construction**, so item 43
  takes its cheap branch; the mandatory branch activates only for the deferred real-product pilots —
  item 34's reopening trigger); CHECKLIST.md item 2 (dudect gate **with a planted-leak self-test
  proving the gate itself works**, `src/ct_gate.rs`, bebop `ntt_ct_gate` standard);
  `kernel/Cargo.toml:45-52` (`ct-gate` feature — the zero-dep dudect harness, `#[cfg(any(test,
  feature="ct-gate"))]`, planted-leak self-test in release via `hardening-gate.sh`); HOT-PATHS.tsv
  `ct_gate` row (`dudect` mode); `RAW-PROMPT-4` part-4 §3 (replace `if x>0` with bitmask / cmov).
- **Dependency gate:** **after item 42**; **scope decided by input-plane classification first** —
  which item 34's ruling has **already settled** for this pilot.

---

## 1. Scope / goal + non-goals

**Goal.** (a) **Record the plane classification** for the toy pilot — public/synthetic by
construction → cheap-but-optional branch, with the named reopening trigger. (b) **Fully design the
mandatory dudect gate** (Welch-t across input classes + planted-leak self-test + branchless ReLU) so
the deferred real-product pilots inherit a ready gate, not a re-derivation.

**Non-goals.** NOT making the toy pilot pay for a gate it does not need (over-design guard). NOT
inventing a new dudect harness — `ct_gate.rs` (item 6) is the substrate. NOT hand-waving the
mandatory branch — the ruling requires it be *specified*, with falsifiable criteria, even though it
does not run for this pilot.

## 2. Grounding

- **The dudect substrate exists.** `ct_gate.rs` (item 6, `ct-gate` feature, `Cargo.toml:45-52`) is a
  zero-dep Welch-t timing gate with a **planted-leak self-test** — a deliberately leaky comparator
  must be *rejected by the same machinery*, or the gate is RED (CHECKLIST.md item 2). It is CI-time,
  not linked into release.
- **The plane ranking is §10/P6 doctrine** (secret-adjacent vs public). Item 34's ruling classifies
  the toy pilot's inputs as public/synthetic **by construction** (no capability/crypto/secret-adjacent
  inputs, no PII, no product data) — so the constant-time property has no adversary *for this pilot*.
- **The branchless-ReLU technique is standard** (dialogue part-4 §3): `relu(x)` as a sign-mask
  (`x & !(x >> 31)` on i32, arithmetic shift) or `cmov` — data-oblivious, no branch to mispredict or
  time-leak.

## 3. Implementation plan

### 3A. Toy pilot — record the cheap-branch ruling (what actually ships now)
1. **Record the plane classification** with reasoning: inputs public/synthetic by construction ⇒
   **cheap-but-optional branch**. The item-39 kernels and item-38 workspace are already data-oblivious
   in *memory access* (fixed offsets, fixed lane order); the only data-dependent branch is ReLU,
   which on a public plane leaks nothing of value.
2. **Name the reopening trigger** (verbatim into the record): *any* new secret-adjacent consumer —
   i.e. the deferred real-product pilots fed from capability/crypto/PII surfaces — flips the mandatory
   branch on. This is an operator-dispatch point for the second pilot (§6).

### 3B. Mandatory branch — design owed now (what the real pilots inherit)
3. **dudect gate** (the `ct_gate.rs` template) over the inference call: **Welch's t-test across input
   classes** — e.g. `class A` = all-positive activations (ReLU never clamps), `class B` = all-negative
   (ReLU always clamps), plus boundary vs interior inputs — assert **|t| < 4.5** (the dudect
   threshold, the `ntt_ct_gate` standard).
4. **Planted-leak self-test** (the load-bearing half): a deliberately data-dependent path — a real
   `if x > 0 { … } else { early-return }` ReLU with an early exit — MUST be **caught** by the same
   Welch-t machinery (|t| ≥ 4.5), or the gate is RED. This proves the gate can *reject*, not just
   pass (P7, mirroring `ct_gate`'s existing planted-leak self-test).
5. **Branchless ReLU / activations** for the secret plane: replace `if x>0` with the sign-mask
   (`x & !(x >> 31)`) or `cmov`; any other data-dependent activation likewise mask/cmov'd — the model
   runs the same instruction count regardless of input.
6. **CI-time, not linked into release** (`ct-gate` feature containment) — the harness self-test runs
   in the same invocation as the gate (`hardening-gate.sh` release run), per CHECKLIST.md item 2.

## 4. Required proofs (5-point checklist mapping)

- **2 (dudect) — the central item:**
  - *Toy pilot (not gated):* the recorded ruling names the reopening trigger — the "cheap-but-optional"
    outcome CHECKLIST.md item 2 permits when timing is not secret-dependent.
  - *Mandatory branch (designed, ready):* **Welch |t| < 4.5 across input classes AND the planted leak
    demonstrably caught.**
- **1/3/4/5:** carried by items 37/39/40/42 (oracle, differential, asm, structural). Item 43 owns only
  the timing/side-channel axis.

## 5. Falsifiable acceptance criteria

1. The **plane classification is recorded with its reasoning** (public/synthetic by construction for
   the toy pilot).
2. The **toy pilot is NOT gated**, and the record **names the reopening trigger** (any new
   secret-adjacent consumer) — an explicit, falsifiable condition, not a vague "later".
3. The **mandatory-branch design is complete and ready** so a real pilot does not re-derive it:
   the dudect input-class plan (§3B.3), the planted-leak self-test (§3B.4), and the branchless
   mask/cmov ReLU form (§3B.5) are all specified.
4. **IF a future pilot is gated** (the trigger fires): **Welch |t| < 4.5 across input classes AND the
   planted leak demonstrably caught** — stated now as the binding acceptance for that reopening.
5. The gate, when active, is CI-time only (`ct-gate` containment) and its self-test runs in the same
   invocation (never presence-checked).

## 6. Dependency gate + operator-decision-needed

- **Gate:** after item 42; scope pre-settled by item 34's ruling (public plane ⇒ cheap branch).
- **Operator-decision-needed — FLAGGED (the named reopening trigger IS the operator-dispatch point):**
  when the **deferred second (real-product) pilot** is chosen, its input plane MUST be re-classified;
  if secret-adjacent (fed from capability/crypto/PII surfaces), the **mandatory dudect branch +
  branchless mask/cmov activations become required** before that pilot ships. This is not an open
  question for the *toy* pilot (settled), but it is the explicit operator gate for the *second*
  pilot — named here so it is never silently skipped. No answer is invented; the trigger and its
  consequence are recorded.
