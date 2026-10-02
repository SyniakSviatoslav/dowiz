# BLUEPRINT — Item 37: Reference Oracle (the inference "Schoolbook")

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — the oracle is
  the ruling in one module: a slow, obviously-correct, permanently-retained reference against which
  every optimized path is differentially proven bit-exact. Speed is explicitly *not* its job.
- **Sources read this session:** roadmap §H item 37 (lines 547–553, "scalar, obviously-correct
  integer-domain matmul + activation set, i64/i128 shadow accumulation, std-only, retained forever,
  NTT-schoolbook precedent"); CHECKLIST.md item 1 (the oracle clause: "reference retained forever as
  a test-only crate-internal module"); `kernel/src/mat.rs:132` (`matmul_contig` — the ONE matmul
  *shape* the oracle mirrors in integer domain, P2 Correspondence) and `:161` (`matmul_contig_in`,
  arena twin); synthesis §4 (P2: "the oracle reuses `mat.rs`'s matmul shape — one primitive"); the
  NTT schoolbook precedent (`pq/dsa.rs`, item 7 blueprint).
- **Dependency gate:** **after item 35** (needs the number format), **parallel with item 36**. **Feeds
  items 39 (differential target), 40 (golden-checksum source), 42 (end-to-end reference).**

---

## 1. Scope / goal + non-goals

**Goal.** A scalar, dependency-free, obviously-correct **integer-domain** reference implementation of
the pilot's operations — matmul + the activation/requantize/argmax set — with **i64/i128 shadow
accumulation** so it is its own overflow detector. It is the permanent, test-only differential
target for the optimized SIMD path (item 39), the source of the golden per-layer checksums (item
40), and the `f(x)=y` ground truth (item 34) for end-to-end tests (item 42).

**Non-goals.** NOT fast — no SIMD (that's item 39), no arena (that's item 38), no unrolling. NOT a
second number-format authority — it *implements* item 35's laws, it does not redefine them. NOT
deleted on optimization — the NTT schoolbook is retained forever; so is this (the load-bearing
discipline). No external dependency (`std`-only) — a differential oracle that pulled a crate would
breach the empty allowlist.

## 2. Current-state grounding

- **The matmul *shape* already exists** (P2 Correspondence): `kernel/src/mat.rs:132`
  `matmul_contig` is "the ONE matmul implementation" (f64, contiguous row-major, `aik==0`
  short-circuit). The oracle is its **integer-domain twin** — same triple-loop shape, i8 in, i32
  accumulate, i128 shadow — not a new algorithm, a re-typing of a proven shape.
- **The schoolbook-oracle discipline is established.** CHECKLIST.md item 1 mandates a reference
  "retained forever as a test-only crate-internal module"; the NTT schoolbook (`pq/dsa.rs`) and the
  regex-replacing pattern matcher's naive reference are the in-repo precedents (item 5/7 blueprints).
- **The wide-accumulator idiom is house practice.** Item 7 blueprint §3.1 names "shadow-widened
  arithmetic" — recompute intermediates in a wider type and assert they fit the narrow type — as the
  load-bearing pattern that makes an enumeration a genuine no-overflow proof. The oracle bakes it in.

## 3. Implementation plan

1. **Place it test-only.** A crate-internal module (e.g. `kernel/src/inference/oracle.rs`) gated so
   it never ships in a release binary — either `#[cfg(any(test, feature = "inference"))]` (matching
   `ct_gate.rs`'s `#[cfg(any(test, feature="ct-gate"))]` containment) or plain `#[cfg(test)]` if the
   engine itself is feature-gated. It carries **no production caller** — differential-only.
2. **Implement the ops, scalar + shadowed:**
   - `oracle_matmul_i8(a: &[i8], w: &[i8], m, k, n) -> Vec<i32>` — the `mat.rs` triple-loop shape,
     accumulating in i32, **with an i128 shadow accumulator** asserting (a) the shadow equals the
     i32 result and (b) the shadow fits i32 (the item-35 overflow lemma, checked at runtime for the
     given shapes). A shape that would overflow makes the oracle *fail loudly*, never wrap.
   - `oracle_relu_i32`, `oracle_requantize` (item-35 `div_half_up` + saturating clamp), and
     `oracle_argmax` — each scalar and obvious.
3. **Fix and document the accumulation order.** The oracle's left-to-right sum order IS the golden
   order; item 39's SIMD path (whose integer associativity *permits* reorder) must still match it
   bit-exact, so the order is pinned and commented here.
4. **Document permanence.** A module-level doc contract ("PERMANENT — never delete on optimization,
   per CHECKLIST.md item 1 and the NTT-schoolbook precedent") + the HOT-PATHS manifest note.

## 4. Required proofs (5-point hardening-checklist mapping)

- **1 (oracle) — this module IS the oracle**, so its *own* proof is self-consistency against a wider
  shadow: **exhaustive small-dimension cases** (e.g. all i8×i8 products for `1×1`, and enumerable
  small `m,k,n`) **+ a large randomized corpus**, oracle result vs the i128 wide-accumulator shadow,
  **zero divergence**. This is the "exhaustive where the input space permits; otherwise a large
  randomized corpus differentially checked" clause, applied to the oracle itself.
- **3 (differential):** the oracle is the *per-call reference* item 39 wires into `debug_assert_eq!`
  — its correctness here is what makes that assertion meaningful.
- **2/4/5:** N/A — the oracle is scalar, timing-irrelevant (test-only), not asm-audited, not Kani'd
  (its correctness is by construction + shadow, the strongest form for a schoolbook).

## 5. Falsifiable acceptance criteria

1. **Exhaustive small-dimension cases + large randomized corpus**, oracle vs i128 wide-accumulator
   shadow, **ZERO divergence**. **RED→GREEN:** a planted off-by-one in the accumulation order or a
   wrong requantize rounding turns it RED.
2. A shape whose accumulation would exceed the item-35 i32 ceiling makes the oracle **fail loudly**
   (assert), never silently wrap — demonstrated with a synthetic over-ceiling shape.
3. The oracle module is documented **PERMANENT** (never deleted on optimization) and carries the
   HOT-PATHS manifest note recording it as the item-39 differential reference.
4. The oracle is **scalar / `std`-only** — no SIMD, no arena, no dependency (verifiable by
   inspection; the zero-dep gate confirms no crate entered).
5. On the item-34 domain D, the oracle's `f(x)=y` outputs are recorded as the golden reference that
   item 40's per-layer checksums and item 42's end-to-end test consume.

## 6. Dependency gate + operator-decision-needed

- **Gate:** after item 35; parallel with item 36; feeds items 39/40/42.
- **Operator-decision-needed:** **none.** The only design choice — accumulation order — is fixed to
  the `mat.rs` left-to-right shape and documented (§3.3); it is executor engineering, not an operator
  gate. (Because integer addition is associative, the *choice* of order does not affect the oracle's
  result — but pinning ONE order is what lets item 39's reordered SIMD path be checked bit-exact.)
