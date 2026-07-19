# RESEARCH — Native (zero-dep) replacement of Kani: per-target feasibility

- **Date:** 2026-07-19 · **Type:** RESEARCH / feasibility (no code) · **Status:** DRAFT for Fable synthesis
- **Trigger:** Item 7 is wiring Kani (a **new external CI tool**) into the kernel *right now* (live
  execution, worktree `/root/dowiz-wt-space-grade-exec`, no proofs landed yet at time of writing).
  This session's standing pattern is to replace every external dependency with in-kernel code
  (regex→hand-rolled matcher, tracing→FDR logger, serde_json→`kernel::json`, sha2→hand-rolled
  Keccak). Question raised: does Kani belong in that same replace-with-native bucket?
- **Method:** Applied the binding 10-step procedure in
  `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` (commit `8f4180279`) to Kani,
  per-target against the 22 harnesses specified in `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md`.
  All source citations verified this session against the exec worktree / main checkout.
- **Scope discipline:** Pure read-only research in the shared checkout; no file item 7 touches was
  written. This doc is the only artifact; staged immediately per the untracked-file-safety rule.

---

## 0. The framing that changes the answer: Kani is CI-time tooling, not a linked dependency

The four precedent replacements — regex, tracing, serde_json, sha2 — were **runtime libraries that
link into the shipped kernel binary**. The kernel used a thin slice of each; hand-rolling that slice
removed real crates from `cargo tree -e no-dev` and shrank the trusted surface *of the artifact*.

Kani is categorically different, and the blueprint already establishes this mechanically (§6.3;
synthesis line ~21 names "Kani" verbatim as **not counting against zero-dep**):

- Harnesses live in `#[cfg(kani)]` modules; the `kani` API crate is injected by `cargo kani` **only
  when `cfg(kani)` is active**. Nothing is added to `Cargo.toml` or `Cargo.lock`. `cargo tree -e
  no-dev` stays at 0. Normal `cargo build/test` compiles every harness out.
- So Kani never enters the shipped binary or the dependency graph at all. It is a **build/CI tool in
  the same class as `rustc`, `cargo`, `cbmc`, and the host OS** — none of which this session proposes
  hand-rolling. The dependency-replacement procedure's terminal states (a) *removed outright* and
  (b) *opt-in feature* both presuppose a **linked** dependency; neither maps. Applied honestly,
  Kani lands in terminal state **(c) legitimate boundary** (§2 step 5) — a CI-time proof tool, not a
  crate in the artifact.

**Therefore the literal question — "hand-roll a native replacement for Kani" — is the wrong question.**
Hand-rolling Kani means hand-rolling a bounded model checker: MIR→SAT bit-blasting plus a CDCL SAT
backend (CBMC uses MiniSat/CaDiCaL/Kissat-class solvers). That is **decades of engineering** — CDCL,
clause learning, watched literals, VSIDS, restarts, preprocessing — a genuine multi-person-year
undertaking, and a weak hand-roll would time out on exactly the hard harnesses (the two full-NTT
sweeps) where a solver's power is the whole point. **A full native SAT-solver-class replacement is
not realistic and would be pointless to attempt.** Unlike the four precedents (small, single-purpose,
kernel-used-a-slice), Kani has no small slice.

The *useful* question — the one this doc answers per-target — is the deeper one: **granting Kani is a
legitimate CI tool, does each of item 7's 22 targets actually need its power, or does this codebase's
already-proven native exhaustive/oracle/differential pattern deliver the identical "for all inputs"
guarantee at zero new tooling?** That answer is per-target, and mostly "already covered."

---

## 1. What Kani does mechanically, honestly

Kani translates Rust MIR into a formula for CBMC (the C Bounded Model Checker), which **bit-blasts**
the program (finite unrolling of loops to a bound) into a propositional formula and hands it to a
SAT/SMT backend. It proves, for **all** inputs within stated bounds/assumptions: no panic, no
arithmetic overflow, no OOB, no assertion violation, plus any `assert!` you write. It is **not**
functional-correctness in general; it is exhaustive-over-a-bounded-input-space checking, decided by a
solver rather than by enumeration.

Kani itself is open-source (Apache-2.0/MIT, AWS); CBMC is open-source; the SAT backends are
open-source. The *core idea* (bounded symbolic execution → SAT) is understandable and re-implementable
at toy scale. What is **not** re-implementable at proportionate effort is the **solver engineering**
that makes it close real problems in minutes instead of never. That engineering is the moat.

The decisive observation for dowiz: **a SAT solver is only necessary when the input space is too large
to enumerate AND the property does not reduce to interval bounds or a closed-form algebraic lemma.**
Item 7's targets were reached for as a *generic* verifier. Swept individually, almost none actually
sit in that intersection.

---

## 2. Per-target classification (22 harnesses)

Buckets, per the task's decision framework:
- **B — already covered natively:** this specific target's input space is small enough for exhaustive
  enumeration, OR its control flow is input-independent, OR it's a corollary of another exhaustive
  proof. This codebase's existing pattern (`csr.rs:1296` `laplacian_dense_vs_spmv_parity_exhaustive_small`
  — **1099 graphs bit-enumerated**; the `blocklist.rs` exhaustive matches; `order_machine.rs:613`
  `for i in 0..12`) delivers the **identical** all-inputs guarantee with **zero new tooling**. Kani
  reached for out of genericity/habit, not necessity.
- **C — narrow native tool feasible:** input space non-exhaustible, but the property reduces to
  **interval/range propagation** or a **structural/algebraic lemma** — a purpose-built few-hundred-line
  checker (NOT a general SAT solver) suffices. Kani automates it; a native substitute exists at
  proportionate effort. Residual Kani value = *machine-checked* vs *hand-argued-plus-differential*.
- **A — genuinely needs Kani-class SAT:** non-exhaustible, non-interval, non-algebraic. **No feasible
  native replacement short of reimplementing a SAT solver.**

### Keccak (§3.1) — 5 harnesses
| Harness | Bucket | Why |
|---|---|---|
| `proof_keccak_f_total` | **B** | Keccak-f is constant-time: all indices are compile-time constants / `%5` of bounded loop vars; the only input-*in*dependent panic risk is the fixed `RHO` shift table (`rotl`'s `n==0` guard). Control flow does not depend on state values, so **one execution exercises every path** — the existing KATs already do. Enumerate the 25 RHO constants, not 2^1600 states. |
| `proof_rotl_contract` | **B** | `rotl` is a bit-permutation; agreement with `x.rotate_left(n)` holds for all `x` if it holds on the 64 single-bit basis vectors ⇒ 64 shift-amounts × 64 basis = 4096 cases. Cleaner still: **just call stdlib `rotate_left`** and the harness is moot. |
| `proof_sponge_bounded_total` | **B** | Index/padding arithmetic depends on `len` and `rate`, **not on byte values**. Exhaust `len ∈ 0..=271 × {2 rates}` ≈ 1084 runs on arbitrary bytes — exactly the `csr.rs` 1099-graph idiom. |
| `proof_evlog_sha3_bounded_total` | **B** | Same reasoning over `event_log::sha3_256`. |
| `proof_keccak_copies_equivalent` (keystone) | **C→B** | Nominally 2^1600-state equivalence (CBMC's home turf). But the two copies share θ/χ/ι byte-for-byte and **differ only in ρ+π indexing** (blueprint §3.1). Equivalence therefore reduces to "are the two ρ/π formulas the same bit-permutation" — a **finite 1600-position index-map comparison**, no SAT. Better: the **dedup of the dual Keccak-f is already an owed ticket** (procedure doc §3, `event_log.rs:67` vs `pq/keccak.rs:156`). Delete one copy ⇒ the equivalence obligation **disappears entirely** (→B). This is the superior answer. |

### FSM graph (§3.2) — 4 harnesses — all **B**
12-state `[u16;12]` const graph (`order_machine.rs:190,208,245`), already 25 tests.
| Harness | Bucket | Why |
|---|---|---|
| `proof_assert_transition_total_and_adj_consistent` | **B** | All **144** `(from,to)` pairs — trivially exhaustible. Identical to the 65536-pair / 1099-graph pattern, smaller. |
| `proof_reachable_total_bounded` | **B** | Symbolic `from` = **12** start states. Enumerate. |
| `proof_fold_transitions_bounded` | **B** | Panic-freedom over any sequence is a **corollary** of the 144-pair total proof + one-line compositional argument (fold applies the pairwise fn). No sequence enumeration needed. |
| `proof_const_graph_algos_no_ub` | **B** | **Input-free** — const graph, already exercised by the 25 tests. Only added value is overflow checking, obtained natively by running those tests under debug overflow-checks / explicit `checked_*`. Blueprint itself calls this "near-zero cost, modest value." |

### NTT arithmetic — ML-DSA (§3.3, `dsa.rs`) — 6 harnesses
| Harness | Bucket | Why |
|---|---|---|
| `proof_reduce32_contract` | **B** | `reduce32` takes **`i32`** (`dsa.rs:81`) → domain **2^32 ≈ 4.3e9**, fully enumerable in seconds. Exhaustion discovers the exact overflow boundary the blueprint wants Kani to exhibit. |
| `proof_caddq_contract` | **B** | Contract domain −Q<a<Q ≈ **1.67e7** residues — sub-second exhaustion. |
| `proof_rounding_contracts` | **B** | `power2round`/`decompose` over `a ∈ [0,Q)` ≈ **8.38e6** residues — exhaustible; `poly_chknorm` abs-trick over its bounded coeff domain likewise. |
| `proof_montgomery_reduce_contract` | **C** | Domain ±(Q·2^31) ≈ **±1.8e16** — not exhaustible. Overflow/range (`(t as i64)*Q` ≤ 1.8e16 < i64::MAX) = **interval analysis**; the congruence `r·2^32 ≡ a (mod Q)` = the standard Montgomery correctness **hand-lemma** + differential test over random+boundary. |
| `proof_ntt_no_overflow` | **C** | `(2Q)^256` — never exhaustible. But the lazy-reduction growth bound (coeffs stay < 9Q « 2^31) is a **monotone interval propagation** over the fixed 8-layer / 1024-butterfly schedule. The blueprint's own fallback (i) — "butterfly lemma + documented 8-layer induction" — **is** this interval argument by hand. A native interval checker specialized to the butterfly pattern closes it. |
| `proof_invntt_no_overflow` | **C** | Same structure over `invntt_tomont`. |

### Ring arithmetic — ML-KEM (§3.4, `kem.rs`) — 5 harnesses
| Harness | Bucket | Why |
|---|---|---|
| `proof_red_total` | **B** | `red` (`kem.rs:58`) is `x % Q; if r<0 {r+Q}`. Modulo by positive Q is **total by language guarantee** (result in (−Q,Q), fits i32); universality over all i64 is a 3-line inspection, congruence is the definition of `%`. Differential-test the residue. No tool needed. |
| `proof_poly_addsub_total` | **B** | Coeff pairs in [0,Q)² ≈ **1.1e7** — exhaustible; this *is* the "65536-pair" shape, slightly larger. |
| `proof_poly_mul_body_lemma` | **B** | Body input reduces to `(r_val, term)` pairs in [0,Q)² ≈ 1.1e7 — exhaustible. Blueprint already scopes this as "body-lemma + documented induction, NOT full proof"; the body lemma is precisely an exhaustible small check. |
| `proof_compress_decompress_bounds` | **B** | `x ∈ [0,Q) × d ∈ {1,4,10,12}` = **13,316** cases — trivial. |
| `proof_byte_codec_bounded` | **B** | Index bounds depend on the **fixed deployed widths**, not byte values; exhaust widths, and the per-element decode domain [0,2^d) is small. |

### GCRA (§5, item 8) — 2 harnesses — both **C**
| Harness | Bucket | Why |
|---|---|---|
| `proof_gcra_transition_contract` | **C** | 4×`u64` — non-exhaustible. But under the headroom `assume`s, no-overflow in `max(tat,now)+cost` = interval; grant/deny conservation = **simple linear integer-algebra lemma**. Native checker or hand-proof + property test. |
| `proof_gcra_two_step_interleaving` | **C** | Two applications; conservation + monotonicity = algebraic. (Kani cannot prove the CAS linearizability anyway — that's loom-class, blueprint §4.4.) |

### Tally
- **Bucket B (already covered natively, zero new tooling): 16 / 22** — all 4 FSM, `reduce32`/`caddq`/
  `rounding` (3 NTT), all Keccak panic-freedom (`keccak_f`, `rotl`, both sponges), all 5 KEM. Plus the
  keystone equivalence collapses to B under the already-owed dedup.
- **Bucket C (narrow native interval/algebraic tool feasible): 6 / 22** — `montgomery_reduce`, `ntt`,
  `invntt`, `keccak_copies_equivalent` (if not dedup'd), both GCRA.
- **Bucket A (genuinely needs SAT, no feasible native replacement): 0 / 22.** Every target decomposes
  into exhaustion (B) or interval/algebraic analysis (C). The single black-box-2^1600 candidate is
  dissolved by shared structure / dedup.

The **only** genuinely irreplaceable thing Kani offers on the C-targets is that its proof is
**machine-checked** where the native substitute is a **hand-argued lemma backed by differential/
interval tests**. That is a *confidence delta*, not a *capability gap* — and it is real: a human
interval/induction argument can be wrong in a way a solver would catch.

---

## 3. Precedent check — does `eqc-rs` already do native verification?

`tools/eqc-rs` (zero-dep, real) is a **differential-testing** precedent, **not** a bounded-verification
one. `emit_proof_program()` emits a self-contained Rust program that `tests/proof.rs` compiles with the
**real rustc, runs, and self-asserts** f64 ≈ fixed ≈ `Expr::eval` reference **at sample points**
(README §"dual emission"/§"Proof"). It proves agreement on a **sweep of samples**, not for all inputs;
its overflow strategy is **runtime** `checked_mul`/domain asserts (README §"Overflow guard"), not static
proof. The `cordic` kernel is pinned by an **FNV-1a digest over a fixed sample sweep** with a teeth-test.

So eqc-rs is a real native-verification precedent of a *different kind* than Kani: **generate-then-
differential-test-at-samples + runtime overflow guards**. It is the ready-made template for the Bucket-C
congruence obligations (differential the native impl against a bignum/`Expr::eval` reference over
random+boundary inputs) but it does **not** already do bounded all-inputs verification — it would need
extension, not mere reuse, to reach Kani's guarantee. Its existence supports Bucket-C being *native-idiomatic*,
not that Kani is redundant.

---

## 4. Terminal-state ruling, per-target (procedure §2 step 5)

- **16 B-targets:** ruling = **do not adopt Kani for these; write native exhaustive tests in the
  existing `csr.rs`/`order_machine.rs` idiom.** Identical guarantee, no `kani-gate` CI job, no
  network toolchain install, no 30-min proof budget, no version pin. This is a **material de-scope of
  item 7** as blueprinted (item 7 wires all 22 as Kani harnesses).
- **6 C-targets:** ruling = **keep Kani as the CI-time tool (terminal state (c))** — it is the
  cheapest way to get a *machine-checked* proof of the interval/congruence lemmas, and it costs the
  shipped binary literally nothing (§0). Building a bespoke native interval analyzer is feasible but is
  *more* work than a `#[cfg(kani)]` block for a *weaker-tooling* outcome; only pursue the native tool
  if operator policy forbids any external CI tool. Interim reliance on Kani here is the pragmatic call.
- **0 A-targets:** nothing forces the multi-person-year SAT-solver build.
- **Keystone Keccak equivalence:** ruling = **dedup the dual Keccak-f (owed ticket) first**; the
  equivalence obligation then vanishes rather than needing either Kani or a structural checker.

**Reopening trigger:** a future harness target whose property is (a) over a non-exhaustible space AND
(b) genuinely nonlinear/non-interval/non-algebraic (e.g. a real cross-implementation equivalence of two
*structurally different* nonlinear circuits, not a shared-structure pair) — that would be the first true
Bucket-A case and the first genuine justification for Kani's SAT power specifically. None exists in
item 7 today.

---

## 5. Bottom line

- **Full native SAT-solver-class replacement of Kani: not realistic, and unnecessary.** It's decades of
  solver engineering with no small slice to carve out — categorically unlike regex/tracing/serde_json/sha2.
- **But most of item 7 doesn't need Kani's power at all:** **~16 of 22** harnesses are already covered by
  this codebase's proven exhaustive/oracle pattern at zero new tooling; **~6** are interval/algebraic and
  either keep Kani (cheap, CI-only, machine-checked) or warrant a narrow native checker; **~0** require a
  SAT solver.
- **Top recommendation:** Do **not** hand-roll Kani. **De-scope item 7** so the 16 Bucket-B targets ship
  as native exhaustive tests (existing idiom), **dedup the dual Keccak-f** so the keystone equivalence
  disappears, and **keep Kani as a CI-only tool for the ~6 interval/congruence C-targets** where a
  machine-checked proof beats a hand lemma and the binary pays nothing. Kani stays a *narrowly-scoped
  legitimate CI boundary* (procedure state (c)), not an in-artifact dependency and not a
  replace-with-native candidate.
</content>
</invoke>
