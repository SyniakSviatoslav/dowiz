# BLUEPRINT — Item 36: eqc-rs Indexed-Summation IR Extension (quantized-dot target)

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість."* Here: **one IR,
  two consumers, never two IRs** — the Σ-over-index construct the Laplacian neighbor-sum (item 32)
  needs is the *same* construct the quantized inner law needs; building it twice would be the
  divergence the ruling forbids.
- **Sources read this session:** roadmap §H item 36 (lines 540–546) + §0 (item 32 IR extension ruled
  **PURSUE**, line 23) + item 32 split (line 399); the live eqc-rs IR —
  `tools/eqc-rs/src/lib.rs:56-76` (the scalar `Expr` enum: `Sym/Num/Sum/Prod/Pow/Sqrt/Sin/Cos/Exp/
  Asin/Atan2/DivHalfUp` — **no indexed-summation, no array access**), `:208` (`Expr::eval`, the
  tree-walking interpreter that is the independent proof reference), `:442` (`emit_proof_program`),
  `:499` (`emit_fixed`), `:552` (`emit_int_checked`, incl. the checked i128 `Sum`/`Prod`/`DivHalfUp`
  paths). Synthesis §1.3 (eqc-rs is zero-dep, empty `[dependencies]`; the quantized dot is the
  already-approved extension's **second consumer**).
- **Dependency gate:** **after item 35** (the number format the emitter targets). **Extends item 32's
  ruled IR work** — must land so item 32's Laplacian consumer stays green.

---

## 1. Scope / goal + non-goals

**Goal.** Grow eqc-rs's `Expr` with an **indexed-summation** construct (Σ over an index) plus the
**indexed array access** it requires, so ONE IR expresses BOTH (a) the Laplacian neighbor-sum
`Σ_j L_ij x_j` (item 32) and (b) the quantized dot / matmul inner law `acc = Σ_k a_k·w_k` (this
arc). Teach the integer-exact emission path the **i32-accumulator Q-format** shape, and keep the
`emit_proof_program` self-assert (emitted code vs the tree-walking `eval`) working on an emitted
quantized dot.

**Non-goals.** eqc-rs stays a *codegen tool at authoring time*, not a runtime transpiler
(`lib.rs:12-16`). No general tensor algebra — only the single missing Σ construct. **No new external
crate** — eqc-rs's `[dependencies]` stays empty (synthesis §1.3). The IR does not gain a SIMD notion
(item 39 hand-writes the intrinsics; eqc emits the scalar reference law and its proof).

## 2. Current-state grounding — the exact gap

The IR is **scalar** (`lib.rs:56-76`): a `Sum(Vec<Expr>)` is a *fixed-arity* sum of listed terms,
not a Σ over a symbolic index range, and there is **no array-index node**. So `Σ_{k=0}^{K-1}
a[k]·w[k]` is inexpressible today — confirmed by the space-grade §26(d) honest limit ("the `Expr`
IR is scalar — no indexed-summation construct — so it cannot emit a matmul"). The three emission
paths and the reference evaluator are all `match`-exhaustive over the enum, so adding a node touches
each in a single, mechanically-checked place:
- `emit_int_checked` (`lib.rs:552`) already produces `checked_add`/`checked_mul` i128 chains with
  typed `?`-propagating overflow errors — the exact shape an indexed accumulator wants.
- `Expr::eval` (`lib.rs:208`) walks an `env: HashMap<String, f64>` of **scalars** — the new nodes
  need arrays in the env, the one non-mechanical extension.

## 3. Implementation plan

1. **Add two IR nodes** (the minimal set):
   - `Index { array: String, idx: Box<Expr> }` — read `array[idx]` (idx an `Expr` over loop vars).
   - `IndexSum { var: String, len: <bound>, body: Box<Expr> }` — `Σ_{var=0}^{len-1} body`, where
     `body` may reference `var` and `Index` nodes. `len` is a **compile-time constant or a named
     bound** (never a symbol whose value isn't build-time-known) — this is what makes the emitted
     loop's trip count fixed (feeds item 42's cyclomatic-1 spine).
2. **Extend `Expr::eval`** to take arrays in the env (`HashMap<String, Vec<i64/f64>>` alongside
   scalars); `IndexSum` folds `body` over the range, `Index` reads the array. This is the
   independent reference for the proof program — it MUST NOT share code with the string emitter.
3. **Extend `emit_int_checked` with the i32-accumulator Q-format path.** `IndexSum` over an i8×i8
   product emits a fixed-trip loop accumulating into an i32 (`checked_add`, `?`-propagating), with
   the requantize/`div_half_up` from item 35 applied after. The emitter **refuses (typed `Err`)**
   any `IndexSum` whose overflow bound (item 35 §3.4, `K·P_MAX ≤ 2^31−1`) is not provable from the
   declared `len` and the i8 range — refuse-never-fall-back, mechanically inherited.
4. **Refusal on inexpressible nodes preserved:** `emit_fixed` (Q-format) and the f64 path refuse the
   new nodes where they don't belong, exactly as they refuse `DivHalfUp`/`Sqrt` today
   (`lib.rs:532-544`, the adversarial-refusal test idiom `lib.rs:646,675,700`).
5. **Wire `emit_proof_program` for an emitted quantized dot:** emit a standalone Rust program that
   feeds fixed i8 arrays through the emitted `_int` fn and asserts bit-equality against
   `Expr::eval` on the same arrays; compile with **real rustc** and run (exit 0 ⇒ codegen correct).
   This is the existing proof-program mechanism (`lib.rs:442`, `int_checked_overflow_never_wraps`
   `lib.rs:716` compiles+runs emitted code) extended to indexed sums.
6. **Keep item 32's Laplacian consumer green.** The Laplacian neighbor-sum `Σ_j L_ij x_j` is the
   same `IndexSum`+`Index` construct; its existing/landing tests must stay green after the
   extension — the shared-IR regression check.

## 4. Required proofs (5-point hardening-checklist mapping)

eqc-rs is CI-time codegen tooling; the **generated organ** (not eqc-rs itself) is the kernel hot
path registered in HOT-PATHS.tsv at item 39. Item 36's own proofs:
- **1 (oracle) — the load-bearing proof:** `emit_proof_program` green on an emitted quantized dot —
  emitted code self-asserted against the tree-walking `eval`, compiled by real rustc. This is the
  "differential against a simple reference" clause, built into the tool.
- **3 (differential):** the proof-program is per-sample differential; additionally the item-32
  Laplacian consumer's existing differential stays green (one IR).
- **2/4/5:** N/A at the IR level (no timing, no asm, no Kani in a codegen tool). The emitted kernel's
  asm/dudect/oracle land in items 39/43/37.

## 5. Falsifiable acceptance criteria

1. `Expr` gains `Index` + `IndexSum`; `eval` extends to array envs; the integer-exact path emits the
   i32-accumulator Q-format loop.
2. `emit_proof_program` on an emitted quantized dot **compiles with real rustc and exits 0**,
   self-asserted vs `eval`. **RED→GREEN:** a deliberately-wrong emitted accumulation order or a
   dropped term fails the proof program.
3. A deliberately-**inexpressible** node is **REFUSED** (typed `Err`) — e.g. an `IndexSum` whose
   `len` is a free symbol (bound not build-time-known) or whose i8×i8 accumulation exceeds the
   item-35 overflow ceiling. **RED→GREEN** adversarial refusal test (the `lib.rs:646` idiom).
4. **All existing eqc-rs tests stay green** (scalar consumers unbroken) AND **item 32's Laplacian
   consumer stays green** — proving one IR serves both, never two.
5. `cargo tree -e no-dev` for eqc-rs still resolves to itself alone (empty `[dependencies]`
   untouched).

## 6. Dependency gate + operator-decision-needed

- **Gate:** after item 35; extends item 32's ruled IR work; parallel with item 37 (per roadmap
  dependency line).
- **Operator-decision-needed:** **none** — item 32's IR extension is already ruled PURSUE. **One
  coordination hazard, FLAGGED (not an operator gate):** items 32 and 36 both edit the shared
  `tools/eqc-rs/src/lib.rs` `Expr` enum. Whoever lands second rebases onto the first; the shared-IR
  regression test (§5.4) is the guardrail. If item 32's Laplacian half has NOT yet landed when item
  36 is dispatched, item 36 lands the `Index`/`IndexSum` nodes *and* the Laplacian consumer's IR use
  in one diff (still "one IR"), and notes it in the module doc so item 32's executor consumes rather
  than re-adds.
