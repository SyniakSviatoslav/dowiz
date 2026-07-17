# BLUEPRINT-E1 — Gradient/Lyapunov energy gate + Laplacian parity-bind

> **Anchors:** Correspondence (P2 "one concept, one primitive; forced divergence pinned by a parity
> check") × the gradient-unification cluster (`RESEARCH-AND-REASONING.md` §3.1) × a **new**
> falsifiable claim — the field integrator's energy is monotone non-increasing — that no test in the
> tree expresses today.
> **Closes:** Hermetic finding **#8 — the *code half*** (parity-bind the ≥3 live Laplacians; the
> remediation plan scheduled only the doc-half, `HERMETIC-REMEDIATION-PLAN.md` §5 BACKLOG). Retires
> the last unpinned mirror at the kernel↔engine seam that **RC-4** left open after H2 (the Laplacian
> operator identity itself, `HERMETIC-ARCHITECTURE-PRINCIPLES.md` §2 RC-4, and
> `BLUEPRINT-H2-mirror-pin-sweep.md` §5's explicit hand-off).
> **Depends-on:** none — fresh design, Wave-0 safe (adds a new module + tests + one thin `noether`
> helper; changes no runtime contract). **Soft prerequisite: H2** — it pins `DT_STABLE`/`DriftClass`
> and thereby *narrows* the seam to the Laplacian, and it supplies the pin-by-test template this
> reuses. (H2 Site-1 is already landed on this branch: `field_frame.rs:51` reads
> `dowiz_kernel::DT_STABLE`.) This blueprint is the scheduled un-suspension of #8: the remediation
> plan's own revisit-trigger — "`csr::laplacian_spmv` gains its first production caller" — has fired
> at `engine/src/bridge.rs:125`.
> **Parallel-safe-with:** H1 (EventStore `Result`), H3 (peer probe), H4 (doubt ritual), P07 (money
> degrade) — all touch disjoint files.
> **Status:** PLANNING ARTIFACT ONLY. No `.rs` file is edited by this document.
> **Re-verified live** against `feat/spectral-energy-flow-evolution` on 2026-07-16 (every file:line
> below was read directly, not trusted from the research doc).

---

## §0 — The problem (both halves, from direct re-verification)

**Half A — an unproven physical claim.** `engine/src/field_frame.rs` integrates the operator field
equation and its test suite proves the field stays **bounded** and **converges** (`:273-316`). It
never proves the property that makes a damped system *physical*: that energy does not spontaneously
grow. The kernel already owns the exact organ for this — `kernel/src/noether.rs` is a
conserved-/bounded-quantity checker whose own non-vacuousness proof *is* the physics case in point
(explicit Euler on a harmonic oscillator gains energy; the checker catches it, `:87-97`). The two
organs have never met. A wrong sign, a negated damping coefficient, or a switch back to plain
explicit Euler would all silently pass today's bounded-and-converges tests up to the point of visible
divergence.

**Half B — a mirror with no parity, and a live opposite-sign split.** The graph Laplacian `L = D − A`
is implemented at least three times inside dowiz — dense (`kernel/src/spectral.rs:287`), sparse-CSR
(`kernel/src/csr.rs:307`), and as an implicit grid stencil (`engine/src/field_frame.rs:92`) — plus a
fourth in bebop (`core/field.rs:82`). Nothing pins them to each other. Worse, re-reading confirms the
research doc's load-bearing finding: **the grid stencil and the CSR/dense operators use opposite sign
conventions.** `field_frame` computes `∇²U = −(D−A)U`; `csr` and `spectral` compute `+(D−A)`. Each
side is internally correct and internally tested; the *relationship between them is asserted by
nothing*. A future caller crossing the seam who assumes a shared convention gets anti-diffusion —
i.e. divergence. This is precisely the RC-4 "unpinned mirrors at the kernel↔engine seam" root cause,
seen from the physics side, and it is the reason Half A cannot be built safely on its own (see §2b).

---

## §1 — Current-state evidence (exact file:line, re-verified live)

- **noether checker.** `noether::step_preserves(x0, update, invariant, steps, tol)`
  (`kernel/src/noether.rs:19`) — two-sided `|I(f(x)) − I(x)| ≤ tol`. `invariant_drift` (`:42`) — total
  variation `Σ|ΔI|`. Non-vacuous proof `catches_euler_energy_drift` (`:87-97`): explicit Euler gains
  energy → tight-`tol` `step_preserves` returns `false`, `invariant_drift > 0`. Signatures take a
  **single** state vector `&[f64]` for both `update` and `invariant`.
- **Field stencil (−convention).** `laplacian(u,w,h)` (`field_frame.rs:92-107`), line **103**:
  `out[i] = left + right + up + down - 4.0*u[i]` = `(Σ nbrs) − 4u_i = −[(D−A)u]_i`. Own sign test
  `laplacian_peak_negative_at_center_of_disk` (`:247-267`) pins `∇²(peak) < 0`.
- **Field integrator (semi-implicit).** `FieldFrame::step` (`:143-160`): `udot = (u − uprev)/dt`
  (`:154`), `num = u + dt*(Γ·udot + c²·l) + dt*s` (`:155`), `den = 1 + dt*M` (`:156`); note `l` is the
  ∇² stencil, so `+c²·l = −c²·(D−A)U` — physically-correct diffusion. Bounded-and-converges test at
  `:273-316` — **no energy assertion.**
- **CSR Laplacian (+convention).** `Csr::laplacian_spmv` (`csr.rs:307`), `Unnormalized` branch
  `:316-324`: `acc = deg[i]*x[i]; acc -= val[k]*x[col]` = `+[(D−A)x]_i`. `LaplacianKind` doc pins the
  `L·1 = 0` conservation invariant (`:280-290`).
- **Dense Laplacian (+convention, same as CSR).** `spectral::laplacian(adj)` (`spectral.rs:287-297`):
  `l.set(i,j, if i==j {deg − adj[i][j]} else {−adj[i][j]})` = `+(D−A)`. **So the split is precisely
  `field_frame` (−) vs `{csr, spectral}` (+)** — two of the three already agree, which the parity-bind
  exploits.
- **The revisit-trigger that fired.** `engine/src/bridge.rs:125` (`VertexBridge::apply_field`, W20)
  calls `laplacian_spmv(..., Normalized)` in a non-test, non-feature-gated `impl` block. *Caveat
  (carried from `RESEARCH-VERIFICATION.md` Check 4):* `apply_field` is a wired public API only reached
  from tests today (`scene.rs:168` constructs a bridge but does not call it) — a production call-site,
  not yet a live loop. That is still exactly the condition `HERMETIC-REMEDIATION-PLAN.md` §5 named as
  the trigger to move #8 out of backlog.

---

## §2 — Target-state design

### (a) The discrete `grad`/`div` (incidence) primitive — the canonical reference operator

**Where it lives: a new `kernel/src/incidence.rs`.** Justification, against the edit-don't-create
bias: the incidence factorization is a *distinct concept* (an oriented edge operator), not a variant
of CSR SpMV, and P2's own corollary is "one concept, one primitive." An oriented-edge representation
is genuinely different in shape from CSR's directed-both-ways adjacency and from the dense
`Vec<Vec<f64>>`; bolting `grad`/`div` onto `Csr` would either misuse its symmetric two-edge
convention or duplicate storage. The whole *point* of this primitive is to be the small, hand-oracle-
tested **reference** every other Laplacian is checked against — the house pattern of `harmonic.rs` /
`noether.rs` (a focused, ~100-line kernel organ). It shares the **edge-tuple input contract**
`&[(usize,usize,f64)]` with `Csr::from_edges` (`csr.rs:79`), which also nods at finding #22 (one
edge-list contract, not a fifth hand-rolled `from_edges`).

**Signature.**
```
pub struct Incidence { /* oriented edges (one per undirected pair), weights w, n */ }
impl Incidence {
    pub fn from_edges(n: usize, edges: &[(usize, usize, f64)]) -> Self; // canonical orientation head>tail
    pub fn grad(&self, x: &[f64]) -> Vec<f64>;        // node-field → edge-flow; (B x)_e = x_head − x_tail; len = n_edges
    pub fn div(&self, flow: &[f64]) -> Vec<f64>;      // edge-flow → node-field; div = Bᵀ; len = n
    pub fn laplacian(&self, x: &[f64]) -> Vec<f64>;   // div(W · grad(x)) == +(D−A)x  (POSITIVE convention)
}
```
`L = Bᵀ W B` by construction, so `laplacian` emits the **positive** graph Laplacian, matching `csr`
and `spectral`.

**Property tests (the parity-bind).**
1. `incidence.laplacian(x) == csr.laplacian_spmv(x, Unnormalized)` for the same graph (undirected
   edges doubled into CSR per its contract).
2. `incidence.laplacian(x) == spectral::laplacian(adj) · x` (dense, same +convention).
3. **The sign-pin test that would have caught the split.** On a lattice edge-list built from the same
   `w×h` grid `field_frame` walks, assert the *explicit* relationship
   `field_frame::laplacian(u) == negate(incidence.laplacian(u))` — i.e. `∇² = −(D−A)`, pinned with the
   minus sign visible in the assertion, plus a boundary note that Neumann zero-flux (stencil) vs the
   interior edge-list agree only on interior nodes (the test uses an interior-only comparison mask, or
   an edge-list that encodes the reflective boundary — decide at implementation; interior mask is the
   simpler, honest choice and is stated as such).

### (b) The Dirichlet-energy Lyapunov gate on `FieldFrame::step`

**Energy functional (exact).**
`E(U, U̇) = ½‖U̇‖²  +  ½c²·⟨U, L₊ U⟩  −  ⟨S, U⟩`,
where **`L₊` is the POSITIVE `(D−A)` operator** (so the potential term
`½c²⟨U,L₊U⟩ = ½c²·Σ_edges w(U_i−U_j)² ≥ 0` is a genuine well). This is the mechanical energy of the
damped-wave family `M Ü + Γ U̇ + c² L₊ U = S`, and it is the same certificate the physics-UI thesis
names (`physics-ui-capture-blueprint.md` §1-2: bebop's `field_physics.rs` `wave_energy`, `dH/dt ≤ 0`)
— this blueprint brings that certificate to the dowiz side, where it is absent.

**Why (b) needs (a) — the load-bearing tie.** `field_frame`'s own stencil returns `−(D−A)` (§1). An
author writing E naively as `½c²⟨U, field_frame::laplacian(U)⟩` gets `−½c²⟨U,L₊U⟩ ≤ 0` — an
**upside-down potential**, a hill instead of a well — and the energy check becomes nonsense that
either always-passes or reports spurious energy gain. The potential term must be evaluated through the
**+convention** operator, and only the parity-bind (a) guarantees which sign you are holding across
the seam. Concretely: compute the potential as `+½c²·⟨U, incidence.laplacian(U_lattice)⟩` (or
equivalently `−½c²·⟨U, field_frame::laplacian(U)⟩` once test #3 above has pinned the identity). This
is RC-4 made operational: the sign-pin is the *precondition* for the energy gate to be trustworthy.

**Wiring into `noether`.** `noether`'s `update`/`invariant` are pure functions of *one* state vector,
but `E` depends on both `U` and `U̇ = (U − U_prev)/dt`, and `FieldFrame` carries two buffers. Pack the
state: `z = [U ‖ U_prev]` (length `2·w·h`). Then `update(z) = [ step(U, U_prev) ‖ U ]` (a pure closure
over one `FieldEquilibrium` + source `S`), and `invariant(z) = E(U, (U−U_prev)/dt)` using the packed
`U_prev` as the reconstructed velocity — the *same* backward difference the integrator itself uses at
`field_frame.rs:154`. No integrator change; the gate is a test-side adapter around the existing
`step`.

**Scheme class & tolerance — get this right.** The default is `Γ = 0.2` (`field_frame.rs:48`), and the
scheme is **semi-implicit and dissipative by construction** (the `1 + dt·M` denominator plus the CFL
bound `assert_stable`, `:59-72`). Two honest consequences:
- The correct claim is **monotone non-increase**, `E(z_{n+1}) − E(z_n) ≤ tol_E` (one-sided), **not**
  two-sided conservation `|ΔE| ≤ tol`. `noether::step_preserves` is two-sided and would wrongly forbid
  the *legitimate* energy decay. So this blueprint adds a **thin one-sided sibling**,
  `noether::lyapunov_nonincreasing(x0, update, potential, steps, tol) -> bool` (checks `V(f(x)) − V(x)
  ≤ tol` each step) — ~15 lines, same shape as `step_preserves`, in the same file. `invariant_drift`
  is reused unchanged as the reported **total dissipation** diagnostic.
- `tol_E` is a small positive float slack absorbing (i) IEEE rounding in `‖U̇‖²`/`⟨U,L₊U⟩` and (ii) the
  transient wiggle that the over-damped default (a `U_next − (1+Γ)U + ΓU_prev` scheme, which
  interpolates gradient-flow at `Γ=0` and a true second difference at `Γ=1`) can produce on the
  reconstructed velocity. The blueprint sets `tol_E` empirically from a *known-good* run and states it
  as a pinned constant with the derivation in a comment — never a hand-waved magic number.

**What this blueprint does NOT attempt (scope, by judgment).** It does **not** collapse the ≥4
Laplacians into one canonical implementation. Reason: the dense (`spectral`, for the n≤32 eigensolve),
sparse-CSR (for PPR/SpMV), and implicit-grid (for the allocation-free stencil) representations serve
three different hot paths; merging them behind one operator would pessimize at least two and is a much
larger refactor touching the eigensolver and the integrator. The P2 guarantee we need — *they cannot
drift* — is delivered by the **parity tests**, not by a merge. bebop's `field.rs:82` is **scoped
out**: a cross-repo bind needs a shared fixture/crate, and finding #18's lesson is that cross-repo
comment-pins do not hold; it is named as future work, not attempted here. Completing `mat.rs`'s
half-retired consolidation (finding #22) as the eventual single home is likewise future work.

---

## §3 — Migration steps (dependency order)

1. **`incidence.rs` primitive** — `struct` + `from_edges`/`grad`/`div`/`laplacian`, with hand-oracle
   tests (a triangle `K₃`, a path `P₃`: `grad` differences, `div = gradᵀ`, `laplacian == +(D−A)` by
   hand).
2. **Parity tests (+convention):** `incidence.laplacian == csr.laplacian_spmv(Unnormalized) ==
   spectral::laplacian·x` on shared graphs.
3. **Sign-pin test (−):** `field_frame::laplacian(u) == −incidence.laplacian(u_lattice)` on interior
   nodes.
4. **`noether::lyapunov_nonincreasing`** helper + its own non-vacuous unit test (reuse the
   `catches_euler_energy_drift` oscillator: the explicit-Euler variant must return `false`).
5. **Energy gate wiring** (engine test module): the packed-state `update`/`invariant` closures around
   `FieldFrame::step`; assert `lyapunov_nonincreasing(..) == true` on the default scheme; report
   `invariant_drift` as total dissipation.
6. **Non-vacuous mutation tests** (§4.5): sign-flipped stencil and negated-Γ variants must make the
   gate return `false`.
7. Run `cargo test` (kernel + engine); every new test green, and each pin/gate test seen to go **red**
   against its injected break first (RED→GREEN).

One edit per step; confirm green before the next.

---

## §4 — Acceptance criteria (falsifiable)

1. `kernel/src/incidence.rs` exists; `Incidence::laplacian` emits `+(D−A)` and equals both
   `csr::laplacian_spmv(_, Unnormalized)` and `spectral::laplacian(adj)·x` on the `K₃` and `P₃`
   fixtures (parity tests green).
2. **Sign-pin RED→GREEN.** A test asserts `field_frame::laplacian(u) == −incidence.laplacian(u)` on
   interior lattice nodes and is **green**; the *same* assertion written without the minus
   (`== +incidence.laplacian`) is **red** against live code — i.e. the test proves the split exists.
   Independently: before this pin lands, flipping `field_frame.rs:103` to `4u − Σ` turns **no** test
   red (demonstrating today's unpinned state); after it lands, that flip turns the sign-pin **red**.
3. `noether::lyapunov_nonincreasing` exists (one-sided `ΔV ≤ tol`) and its unit test shows the
   explicit-Euler oscillator variant returns `false` while the mass-conserving exchange returns
   `true` (non-vacuous, mirroring `noether.rs:87`).
4. **Energy monotonicity green.** On `FieldEquilibrium::default()` driven by a finite SDF source, the
   packed-state gate reports `lyapunov_nonincreasing(E, steps=N) == true` within the pinned `tol_E`,
   and `invariant_drift(E)` equals the reported total dissipation (a positive, bounded number).
5. **Energy gate catches an artificially-broken integrator (non-vacuous).** A test variant that either
   (a) flips the stencil sign (anti-diffusion, `+c²(D−A)U`) **or** (b) negates Γ (anti-damping,
   `Γ = −0.2`) makes `E` **increase**, and `lyapunov_nonincreasing` returns **false** for that variant
   — proving the checker is not vacuous, exactly as `noether.rs`'s explicit-Euler-gains-energy proof
   does for its checker.
6. No runtime contract changes: `FieldFrame::step`, `laplacian_spmv`, and `spectral::laplacian`
   signatures are untouched; the new code is a module + tests + one thin `noether` helper. `pnpm
   typecheck` and the full kernel/engine `cargo test` suites pass.

---

## §5 — What this unblocks, and how it composes with the Hermetic work

This is the **code-half of finding #8** that `HERMETIC-REMEDIATION-PLAN.md` §5 explicitly left in
backlog pending a production caller — a caller that has since appeared (`bridge.rs:125`). It is the
direct continuation of **H2's mirror-pin sweep**: H2 §5 states that pinning `DT_STABLE` and
`DriftClass` "narrows the remaining unpinned-mirror surface at that boundary to the Laplacian operator
identity itself, which H-series Correspondence work (separate blueprint) can then address in
isolation." **E1 is that separate blueprint.** It applies the *same* `DT_STABLE` pin-discipline — two
(here, three) independent implementations, each bound to the others by a test asserting the shared
identity — to the Laplacian, closing the last hand-mirrored operator at the kernel↔engine seam and
thereby completing RC-4 for the operator axis.

Beyond the audit, E1 produces something the tree did not have: a **new, falsifiable physical
guarantee**. "The field integrator's energy is monotone non-increasing" is now a test, catchable and
caught when broken — the honest, code-true installment of the physics-UI thesis's "ONE Laplacian, one
spectral calculus `f(L)`" ambition (`physics-ui-capture-blueprint.md` §1). It does not *reject* that
ambition; it delivers the enforceable core of it (one reference operator, one sign convention, one
Lyapunov certificate) and honestly defers the full one-implementation unification and the cross-repo
bind to named future work. The incidence primitive it lands is also the natural seed for finding
#22's `from_edges` conversion hub and #25's second-party checks, should those triggers fire next.

*Verified live on `feat/spectral-energy-flow-evolution` as of 2026-07-16. Read-only planning
artifact; no source code was written or edited.*
