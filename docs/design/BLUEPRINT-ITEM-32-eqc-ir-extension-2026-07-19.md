# BLUEPRINT — Item 32: eqc indexed-summation IR extension (PURSUE, independent) + the §16 control-law half (gated)

- **Date:** 2026-07-19 · **Tier:** parallel lane / §F spectral (roadmap §F) · **Status:** BLUEPRINT
  (planning artifact, no code). **Split item:** the Laplacian half already lands with item 18 (Tier 0);
  the **eqc IR extension is ruled PURSUE and is independent of the breaker** (roadmap §E item 32, §0
  gate); the **§16 pilot-control-law half needs items 9 + 21**.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §E item 32
  (lines 399–401), §F (lines 405–406), §0 gate (line 23, "extend eqc's `Expr` language to support the
  Laplacian's neighbor-sum operator"); `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §26
  (eqc as the standing answer), §14 (one Laplacian, two representations), §16 (control laws as checkable
  equations); live source `tools/eqc-rs/src/lib.rs` (the `Expr` language), `kernel/src/csr.rs`
  (`laplacian_spmv`).
- **Relationship to items 9/18/21:** the IR extension is standalone eqc-rs capability work (roadmap
  §F: "runs alongside, independent"); item 18 (Tier 0, DONE-area) landed the Laplacian dense/spmv
  parity pin; the control-law half feeds item 21's autonomic laws (each a checkable eqc equation) and
  routes extreme responses through item 9.

---

## 1. Scope / goal (one paragraph)

Extend the `eqc` equation→Rust compiler's `Expr` language with an **indexed-summation / neighbor-sum
operator** so it can express the graph Laplacian's `(Lu)_i = Σ_{j∈N(i)} w_ij (u_i − u_j)` — not just
the scalar control laws it compiles today (roadmap §0: "extend eqc's `Expr` language to support the
Laplacian's neighbor-sum operator, not just scalar control laws"). This is the **PURSUE** half — an
independent eqc-rs capability, ungated by the breaker (roadmap §F: "eqc IR extension … runs alongside,
independent"). The payoff (synthesis §26/§14): the kernel's *one Laplacian, two representations*
(dense `laplacian()` ↔ matrix-free `laplacian_spmv`, parity-pinned by item 18) gains a *third,
authoritative* representation — the **equation itself**, compiled to Rust by eqc, so the Laplacian
math is a checkable equation feeding the kernel rather than hand-written code diverging from its spec.
The document also scopes the **§16 control-law half** (each autonomic control law authored as an eqc
`Expr` and compiled — "every control law is itself a checkable equation per §10/P1, never learned"),
which is gated on items 9 + 21 (the pilot-control-law needs a running breaker + the autonomic layer).

---

## 2. Verified current state — grounded

- **The eqc `Expr` language is scalar-only today — verified.** `tools/eqc-rs/src/lib.rs:56–76`
  `pub enum Expr { Sym, Num, Sum(Vec<Expr>), Prod(Vec<Expr>), Pow, Sqrt, Sin, Cos, Exp, Asin, Atan2,
  DivHalfUp }`. `Sum(Vec<Expr>)` is a **fixed-arity** sum of explicit sub-expressions — there is **no
  indexed/bound-variable summation** (`Σ_{j∈N(i)}`), no notion of a neighbor set. So the Laplacian's
  neighbor-sum is **not expressible** in the current IR — this is exactly the gap roadmap §0 names.
- **The three emission modes the extension must respect are coded.** eqc emits three ways with typed
  refusal at the boundaries: `emit_f64_rust` (dynamics), `emit_int_checked_rust` (integer-exact,
  `Result<i64, &str>` with every step checked, `lib.rs:503–569`), and the fixed-point/Q-format path.
  The typed-refusal discipline is load-bearing: `FixedPointUnsupported` (`lib.rs:81`),
  `IntEmissionUnsupported` (`lib.rs:97`), `F64EmissionUnsupported` (`lib.rs:113`) each refuse
  nodes not representable in their mode. **The new indexed-sum node must slot into this same
  three-mode typed-refusal structure** — a neighbor-sum over `f64` weights is f64-mode (the Laplacian
  is real-valued dynamics), and int/fixed emission refuses it (like `Sqrt`/`Sin`).
- **The Laplacian target the IR must match is `csr.rs:552` `laplacian_spmv`.** The matrix-free
  application `laplacian_spmv(&self, x, out, kind)` — already parity-pinned against dense `laplacian()`
  by item 18's exhaustive-small + random-corpus tests (`csr.rs:1296` `..._exhaustive_small`, `:1386`
  `..._random_corpus`). The eqc-emitted Laplacian must produce the **byte-identical** (to float
  epsilon) result as this third parity leg (synthesis §14: "one Laplacian, two representations,
  parity-pinned" → the eqc form is the third, and the parity net extends to it).
- **The control-law half's target (item 21's `LAW_TABLE`) does not exist yet** — item 21 builds the
  autonomic layer; item 32's control-law half authors those laws as eqc `Expr`s. And the breaker
  (item 9) is the extreme-response route. So the control-law half is gated; the IR extension is not.

---

## 3. Implementation plan — the IR extension (PURSUE, buildable now) + the control-law half (gated)

**Part A — the indexed-summation IR extension (independent, buildable now):**
1. **`tools/eqc-rs/src/lib.rs` — new `Expr` variant.** Add an indexed-summation node, e.g.
   `NeighborSum { over: BoundVar, body: Box<Expr> }` — a bound variable `j` ranging over a
   neighbor-set `N(i)`, with `body` an `Expr` in `i` and `j` (e.g. `w_ij * (u_i − u_j)`). Keep it
   minimal (ponytail): the *only* summation shape needed is the Laplacian's neighbor-sum, so do not
   build general Einstein-summation — one bound variable, one index set, matching the exact operator
   §0 names. The node carries the semantics of `Σ_{j∈N(i)}`.
2. **Emission — f64 mode only, with typed refusal elsewhere.** `emit_f64_rust` gains a case emitting a
   loop over the neighbor set (the CSR row's `col_idx`/`values`, matching `laplacian_spmv`'s access
   pattern); `emit_int_checked_rust`/fixed-point **refuse** the node via the existing
   `IntEmissionUnsupported`/`FixedPointUnsupported` (the Laplacian is f64 dynamics — the same boundary
   `Sqrt`/`Sin` already sit on, `lib.rs:66–75`). This preserves eqc's "some nodes are f64-only" honest
   boundary.
3. **The eval path** (`Expr::eval`, `lib.rs:214`) gains the `NeighborSum` case over an environment
   carrying the graph adjacency — so the eqc equation is *evaluatable* (for the differential oracle)
   as well as *emittable*.

**Part B — the §16 control-law half (gated on items 9 + 21):**
4. **`tools/eqc-rs` control-law equations.** Each item-21 autonomic control law
   (`Damped → rate *= 1.0`, `Resonant → rate *= 0.9`, etc.) authored as an eqc `Expr` and compiled to
   the Rust in `kernel/src/autonomic.rs` — so "every control law is itself a checkable equation per
   §10/P1, never learned" (synthesis §16(b)(iii)) is literally true: the law's Rust is *generated from*
   its equation, not hand-written. This is gated: item 21's `LAW_TABLE` must exist to be generated,
   and item 9's breaker is the extreme-response route.

Zero new dependency — eqc-rs is an "exemplary zero-dep" tool crate (synthesis §25); the extension is
pure-Rust IR + emission logic.

---

## 4. Tests / proofs — 5-point hardening applicability

The IR extension produces an algorithmic hot path (the emitted Laplacian) and eqc has a
generated-code-parity discipline already (the money law is emitted by eqc and asserts *exact integer
equality* against the hand-written law — CLAUDE.md). The 5-point standard:

- **Item 1 (oracle — the headline, extends item 18's net):** the eqc-emitted Laplacian must be
  differential-checked against **both** existing representations — dense `laplacian()` and
  `laplacian_spmv` (`csr.rs:552`) — over the **same** exhaustive-small + random-corpus fixtures item
  18 already uses (`csr.rs:1296`, `:1386`), **green to float epsilon**. This is the "cargo tree
  unchanged, parity green" proof (synthesis §9 item 18/§14) extended to the third leg. The eqc `eval`
  path (§3.3) is the reference for a per-call differential too.
- **Item 3 (debug-differential):** `debug_assert!` the eqc-emitted Laplacian result against
  `laplacian_spmv` per call (the per-call reference exists — the strong form, like the money-law
  parity assert eqc already carries).
- **Item 5 (Kani/formal):** the emitted neighbor-sum's index arithmetic (CSR row bounds, the
  `col_idx` access) is a **real OOB class** — a candidate for item-7-style native-exhaustive or Kani
  panic/overflow-freedom over small graphs (the `laplacian_spmv` bounds are already exercised by item
  18's exhaustive-small; the eqc-emitted form inherits the same enumeration). Record as
  `native-exhaustive(item-18-fixture-shared)`.
- **Item 2 (dudect):** **N/A** — Laplacian math has no secret-dependent timing. Record
  `N/A(no-secret-compare)`.
- **Item 4 (asm):** **N/A** — no branch-free constant-time path.

**Control-law half (Part B):** its parity proof is that the eqc-generated `LAW_TABLE` Rust is
byte-identical to (or exact-equal against) item 21's hand-specified laws — the same generated-code-
parity assertion eqc uses for the money law (a test asserting the emitted law == the reference).

---

## 5. Acceptance criteria (falsifiable)

**Part A (IR extension — buildable now):**
1. **`Expr` expresses the neighbor-sum** — the Laplacian `(Lu)_i = Σ_{j∈N(i)} w_ij(u_i − u_j)` is
   authorable as an `Expr` tree and compiles via `emit_f64_rust`.
2. **A parity test computes Lu via dense `laplacian()`, via `laplacian_spmv`, AND via the eqc-emitted
   form — exhaustive over small graphs plus a large randomized corpus — green to float epsilon**
   (synthesis §9 item 18/§14, extended to the third leg).
3. **Int/fixed emission refuses the node** with a typed error (`IntEmissionUnsupported`/
   `FixedPointUnsupported`) — the honest-boundary discipline preserved.
4. **`cargo tree` unchanged** (eqc-rs stays zero-dep); the IR extension is independent of the breaker.

**Part B (control-law half — gated):**
5. **Each item-21 control law is authored as an eqc `Expr` and its generated Rust matches the
   reference** (generated-code-parity, the money-law precedent) — "every control law is a checkable
   equation."

---

## 6. Dependency gates

- **Part A (IR extension): PURSUE, independent** (roadmap §E item 32, §F: "runs alongside,
  independent" — no breaker dependency). Can start now. Rides item 18's already-landed Laplacian
  parity fixtures as its oracle (item 18 is Tier 0 / DONE-area).
- **Part B (control-law half): gated on items 9 + 21** (roadmap §E item 32: "only the §16 pilot-
  control-law half needs items 9 + 21"). Item 21's `LAW_TABLE` must exist to be generated; item 9's
  breaker is the extreme-response route.
- **The Laplacian half itself already landed with item 18** (roadmap §E item 32: "Laplacian half
  already lands with item 18 (Tier 0)") — item 32 is the *IR extension* + *control-law* halves, not
  the parity pin itself.

---

## 7. Open questions (operator ruling)

None requiring an operator ruling. Two **executor** scope judgments (flagged, not operator gates):
1. **IR generality.** Recommendation (ponytail): build *only* the single-bound-variable neighbor-sum
   the Laplacian needs, not general Einstein/tensor summation — the roadmap §0 names exactly the
   neighbor-sum operator, and a general summation IR is over-engineering against "schema rich, runtime
   minimal." If a *second* indexed-summation consumer appears later (e.g. a diffusion kernel), widen
   then, with the parity net as the safety. Executor scopes; no operator decision.
2. **Part-A-then-Part-B sequencing.** Part A is buildable now and delivers the third parity leg
   independently; Part B waits on items 9+21. Recommendation: land Part A alone (it stands on its own
   merit — the equation-as-authoritative-Laplacian value), then Part B when the autonomic layer
   exists. Named so the executor does not block Part A on the gated Part B.
