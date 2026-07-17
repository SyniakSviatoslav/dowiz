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

---

## §6 — Extended Context

**Why this matters beyond closing a Hermetic finding — the split is a *latent* bug, and its
invisibility is the danger.** Today the CI is fully green: `field_frame`'s stencil (`−(D−A)`) and
`csr`/`spectral` (`+(D−A)`) each have their own passing tests, and *nothing is red*. That green is
precisely the hazard. A latent sign bug that produces zero failures is worse than a loud one, because
it carries no pressure to be fixed and it will ship silently into the first caller who crosses the
seam. The only thing standing between the split and a production incident is the *accident* that no
caller has yet computed a step with the `+`-convention operator and fed it into the `−`-convention
integrator (or vice-versa). `engine/src/bridge.rs:125` is the near-miss on the record: `apply_field`
already calls `laplacian_spmv(.., Normalized)` in a non-test `impl` block — reached only from tests
today, but a wired public API one call away from the seam. The remediation plan named exactly this
condition as the trigger to un-suspend finding #8; it has fired.

**What a caller crossing the unpinned seam would actually observe.** Physical diffusion is
`∂U/∂t = +c²∇²U` with `∇² = −(D−A)` — it drives a field toward its local average and *settles*, energy
monotone down, the screen relaxing into smooth gradients. If a caller assumes the `+(D−A)` sign the
kernel operators expose and plugs *that* into the integrator's diffusion term, the effective sign
flips to anti-diffusion: the eigenmodes with the **largest** Laplacian eigenvalue now grow fastest,
the field's amplitude increases every step, and the simulation **blows up** — an exponential
divergence that saturates to `NaN`/`Inf` within tens of steps once `f32` overflows, rendered as a
display that explodes into noise instead of quieting. The decisive point: the existing
bounded-and-converges test (`field_frame.rs:273-316`) runs the *correct internal path* and stays
green throughout — it cannot see the mismatch because it never crosses the seam. **No test in the
tree has a failure mode that says "a caller wired the wrong-sign operator into the integrator."** That
absence — not any defect inside either module — is the finding, and the sign-pin test (§2a #3 /
§4.2) is the missing tripwire.

**Connection to the agentic-mesh-protocol arc (B1-B4): honestly, there is none — and saying so is the
point.** Direct check of the sibling arc: B1 (`AgentBridge` port + signed manifest admission), B2
(`WorkReceipt` + HTLC settlement), B3 (`ExposureLedger` + rate envelopes), B4 (crypto ground-truth
bench + Ed25519 batching) are the mesh's *economic-trust and admission-capability* layer — "who may
act, and what they owe." The **only** occurrence of "spectral" anywhere in the entire B-series is a
documentation cross-reference in B1's header pointing at E3's self-harness blueprint; not one B
blueprint reads, extends, or depends on `laplacian`, `incidence`, `field_frame`, `noether`, or the
Dirichlet energy. E1's Laplacian work and the mesh protocol are **orthogonal**, and forcing a link
would be exactly the metaphor-discipline (V6) violation this arc's classification was built to reject
(it is why clusters 6 and 7 were rejected). The one substrate the two arcs *could* have shared —
`event_log.rs::commit_after_decide` (H1/hydra), which the mesh work touches for money/receipt events —
E1 does **not** touch at all: E1 is pure kernel math plus a test-side gate, changing no runtime
contract. That non-overlap is what makes E1 Wave-0 parallel-safe not just with H1-H4 but with the
whole B-series.

---

## §7 — Definition of Done

The §4 acceptance criteria are per-artifact falsifiable checks. This DoD is the complementary
*boundary and integration* contract: what "done" includes, what it deliberately excludes, and how it
sits against work already shipped this session.

**In scope — E1 is done when:**
- All six §4 acceptance criteria are green, each pin/gate witnessed **RED→GREEN** against its injected
  break first (the sign-pin goes red on a flipped `field_frame.rs:103`; the energy gate goes red on
  the anti-diffusion and negated-Γ variants).
- The three deliverables exist and no more: `kernel/src/incidence.rs` (new reference operator),
  `noether::lyapunov_nonincreasing` (~15-line one-sided sibling in the existing file), and the
  engine-side energy-gate **test** module. One `pub mod incidence;` line in `kernel/src/lib.rs`.
- The doc-side ledger is reconciled (a **documentation** action, not code): remediation-plan finding
  #8's row is marked "code-half landed," and the arc gains its MEMORY.md index line (per
  `SPECTRAL-EVOLUTION-CONSOLIDATED.md` §6 Ananke).

**Out of scope — "done" explicitly does NOT include (each deferred item is a named E53-form waiver,
not a silent drop):**
1. **Merging the ≥4 Laplacians into one implementation.** The dense/CSR/implicit-grid trio stay
   separate hot-path representations, bound by parity tests, *not* collapsed (§2b). See §9(c) for the
   debt this books and its revisit trigger.
2. **bebop `core/field.rs:82` cross-repo bind — confirmed still deferred, and now *owned*.** The
   existing §2b text names this as future work but assigns no owner; this DoD closes that gap in E53
   form so it is a real, trackable item rather than a floating "future work":
   > *what:* parity-bind bebop's fourth Laplacian (`core/field.rs:82`) to the dowiz reference operator.
   > *why-suspended:* a cross-repo bind needs a shared fixture/crate; finding #18's lesson is that
   > cross-repo *comment*-pins do not hold. *named-owner:* the next author to touch bebop
   > `core/field.rs` (cross-listed on the bebop UNIFIED-DELIVERY arc — do **not** mint a second owner).
   > *revisit-trigger:* a shared kernel/fixture crate spanning dowiz⊕bebop exists (removing the #18
   > fragility), **or** `field.rs:82` gains a caller crossing into dowiz-shared state. *date:*
   > 2026-07-16.
3. **The Normalized-Laplacian branch is NOT parity-bound.** The fixtures bind `Unnormalized +(D−A)`
   (and the dense operator); the live trigger caller `bridge.rs:125` uses `laplacian_spmv(Normalized)`
   — a *different* operator the parity web does not cover. Recorded here and in §9(b) as a known
   coverage gap with its own trigger ("a Normalized-convention consumer moves from wired-API to a live
   loop" ⇒ add a Normalized parity fixture).
4. **No `FieldFrame::step` runtime change, and no wiring of the energy gate into a live loop.** The
   gate is a test-side adapter (§2b); promoting it to a runtime detector is a separate, triggered
   decision (§8).
5. Completing finding #22's `mat.rs`/`from_edges` consolidation hub (future; the incidence primitive
   is its natural seed, not its delivery).

**Sequencing against the Hermetic-remediation work already shipped this session — E1 duplicates
nothing:**
- **vs H2 (mirror-pin sweep).** H2 pinned two mirrors of a *different kind*: `DT_STABLE` (an `f32`
  constant) and `DriftClass` (a three-variant enum + its wire mapping) — **scalar/enum value**
  mirrors, pinned by self-assertion tests, touching `field_frame.rs` (dt default), `spectral.rs`
  (`wire_code`), `wasm.rs`, `loop_.rs`, and the spool tools. E1 binds the **operator** mirror — the
  function identity `L = D−A` across three *implementations* — by property tests on shared graph
  fixtures, touching `incidence.rs` (new), `noether.rs`, and test modules. Same *discipline*
  (pin-by-test over independent declarations), **different objects**, **disjoint files**. This is not
  overlap but the explicit hand-off: H2 §5 states its work "narrows the remaining unpinned-mirror
  surface … to the Laplacian operator identity itself, which H-series Correspondence work … can then
  address in isolation" — **E1 is that isolation.** The single shared file, `field_frame.rs`, E1
  touches test-only (or not at all if the gate lands in a separate test file); H2 Site-1 already
  landed the `dt` pin there (`field_frame.rs:51` reads `DT_STABLE`) and E1 does not revisit it.
- **vs H1 (event-log `Result` fix).** H1 restored the typed `StoreError` failure pole on `EventStore`.
  E1 touches the event log **not at all**, so there is no ordering constraint — strictly parallel.
- **Net:** E1 has one *soft* prerequisite (H2, whose Site-1 is already on this branch, narrowing the
  seam and supplying the template) and zero hard ones. It may land independently of the remaining
  Hermetic rows.

**Done does NOT mean** "all Laplacians unified" nor "the field integrator carries a runtime energy
alarm." It means: one canonical reference operator exists, one sign convention is pinned across the
seam by a red-provable test, and the integrator carries a falsifiable, non-vacuous energy-monotonicity
certificate — with every larger ambition booked as a named, triggered waiver.

---

## §8 — Event-Driven Architecture Treatment

**Plainly: this is kernel MATH machinery — Laplacian operators, a Dirichlet energy functional, a
Lyapunov check — and it is not naturally event-sourced.** The incidence primitive, the parity tests,
and the energy gate are pure, deterministic functions over state vectors; there is no command, no
projection, no fold, no `MeshEvent` in the design, and none is being retrofitted. Forcing an
event-sourcing frame onto operator algebra would be ceremony without a consumer.

**The one genuine question worth a real call: if `noether::lyapunov_nonincreasing` ever detected an
energy-monotonicity violation at *runtime* — a live field simulation in production, not a test —
should that emit an auditable, replayable `MeshEvent` ("the field integrator's energy invariant was
violated at tick N"), the way `hydra.rs::integrity_check` failures already do via
`raise_breach_alarm`?**

**Design call: NO — no `MeshEvent`, no runtime mesh alarm. Justified, not defaulted:**

1. **The field integrator is client-side UI physics, not a mesh-trust computation.** `FieldFrame`
   lives in the `engine` crate, which is CPU-side and whose `Cargo.toml` states plainly that GPU/wasm
   is "a display surface" — it is the ambient-field renderer for the Sea interface. Its output is
   *pixels*. Nothing downstream trusts the field's energy the way the mesh trusts a signed
   `WorkReceipt` or the organism trusts its own persisted topology spectrum.
2. **Contrast with `raise_breach_alarm` is categorical, not degree.** That alarm fires because the
   **kernel core's own base-topology spectral radius** crossed `ρ≥1` (`hydra.rs:180-194`) — evidence
   the persisted, hash-chained core was *tampered by foreign code*. That is intrinsically
   mesh-trust-relevant: a tampered core is an attack on every peer in the hub ("одне взломане ядро =
   взлом усіх"), so it self-witnesses to the WORM log (content-addressed evidence row) and broadcasts
   an ML-DSA-signed, unsuppressable alert. An energy violation in a client's field renderer has **no
   cross-peer consequence**: it is a numerical or authoring bug in one client's UI; forging or
   suppressing it social-engineers nobody; no settlement, admission, or fold decision consumes the
   number. It fails the exact test this arc used to reject clusters 6 (quantum-steering trust) and 7
   (circuits) — *no mesh-trust decision reads it.*
3. **The blueprint already places the check test-side, where the correct auditable record is CI, not
   a runtime event.** A Lyapunov violation in the engine-side test module is a **test failure** — CI
   red plus git history is the replayable, durable log for a dev-time invariant, exactly as
   `noether::catches_euler_energy_drift` is a *test*, never a runtime alarm. Emitting a `MeshEvent`
   from a CI assertion would be a category error.
4. **Where the answer would flip — the named trigger.** IF the field integrator were ever promoted to
   a mesh-trust role — field energy becoming an input to a settlement/admission decision, or a
   *server-side* (not client) field computation feeding shared state — THEN a runtime detector
   emitting a `MeshEvent` **would** be warranted, and the shape already exists to reuse:
   `commit_after_decide` / the breach-witness pattern, `payload = {tick N, E_prev, E_next,
   operator-id}`, self-witnessing and replayable. That promotion is out of scope (§7 item 4) and
   should itself be an E53-triggered decision. **Revisit-trigger:** *field energy becomes an input to
   any mesh-trust decision (settlement, admission, or a shared-state fold), or the field integrator
   runs server-side rather than client-side.* Until then: no runtime mesh event-sourcing, by design.

---

## §9 — Long-Term Consequences, Safety, Scalability

**(a) Scalability of the incidence `grad`/`div` primitive.** First, an honest correction to the
premise: in the *shipped* design the incidence operator does **not** run on every field-integrator
step. `FieldFrame::step` (`field_frame.rs:143-160`) keeps the hand-rolled 5-point stencil
(`field_frame.rs:146`) as the runtime operator, unchanged (§4 acceptance #6); `incidence.laplacian` is
the **test-side reference oracle**, exercised on fixture graphs (`K₃`, `P₃`, a small lattice). So its
runtime cost is a non-question for production — it runs `O(once)` in CI on graphs of a handful of
nodes. Addressing the hypothetical the question raises (a future caller wiring incidence — or the
energy check — into a live per-step loop):
- *Stencil:* one `Vec<f32>` alloc of `w·h`, one structured strided pass with fixed `±1`/`±w` offsets
  (cache-friendly), ~5 flops/cell — sub-millisecond well past `256×256`.
- *Incidence `L = BᵀWB`:* `grad` allocates a `Vec<f64>` of length `n_edges`, `div` a `Vec<f64>` of
  length `n`; for a `w×h` grid `n_edges ≈ 2·w·h`, so ~2-3× the memory traffic **plus** edge-indexed
  gather/scatter (each edge touches two arbitrary node indices ⇒ cache-unfriendly indirect access,
  unlike the stencil's fixed offsets) **plus** `f64` vs the stencil's `f32`.
- *Crossover:* incidence is more expensive at *every* grid size (strictly more allocation, worse
  locality), but the **absolute** gap only becomes measurable — say the >1 ms/step that threatens a
  16 ms 60 fps frame budget — once `w·h` reaches the tens of thousands (a `256×256 ≈ 65k`-cell field),
  where the stencil is still comfortably sub-ms but the incidence gather begins thrashing cache. Below
  ~`64×64` (~4k cells) the difference is noise.
- *Does it matter?* For the shipped design, **no** — incidence never touches the hot path. For the
  hypothetical, it runs **client-side in the UI** (the engine is a CPU-side display renderer), *not*
  the mesh-critical path: a client dropping frames on its own ambient-field animation degrades that
  one client's visual smoothness and harms no peer and no ledger. The worst-case blast radius is one
  client's frame-rate. This is precisely why keeping the stencil as the runtime operator and incidence
  as the test oracle is the right split — full parity guarantee, zero incidence cost where it would
  bite.

**(b) Safety — can a passing parity test give false confidence?** Yes, and this is a real limit of
test-based parity-binding versus a proof, stated honestly. Two implementations can agree on the
*test's specific topology* while diverging on one the test never exercises. Concrete divergence classes
the `K₃`/`P₃`/small-lattice fixtures do **not** cover:
- *Self-loops / multi-edges* — incidence's "one oriented edge per undirected pair" contract and CSR's
  symmetric two-edge doubling handle these differently.
- *Disconnected graphs / isolated (degree-0) nodes* — empty rows.
- *Signed or zero edge weights.*
- *Large-degree hub nodes* — floating-point summation order in the accumulator differs between
  implementations (non-associativity), so an exact `==` parity assertion could even *falsely fail*, or
  a `tol`-based one *falsely pass*.
- **The sharpest hole: the Normalized Laplacian.** The parity web binds `Unnormalized +(D−A)`, but the
  live trigger caller `bridge.rs:125` uses `laplacian_spmv(Normalized)` — `D^{-1/2}(D−A)D^{-1/2}`, a
  *different operator* the fixtures never touch. **The test that "fires" on the trigger caller does not
  cover the branch that caller actually uses.**

Named honestly: test-based parity-binding proves *agreement on the tested topologies*, not *operator
identity* — an existence-of-agreement check, not a `∀`-graphs proof. Closing the gap meaningfully
needs either (1) a symbolic/algebraic equivalence argument (both structurally compute `BᵀWB`), or,
more practically, (2) **topology-diverse property testing** — a `proptest`/`quickcheck` generator over
random `n`, random edge sets *including* self-loops, multi-edges, disconnected components, and random
weights, **plus** an explicit Normalized-convention parity fixture binding the `bridge.rs:125` branch.
Until that Normalized fixture exists, the DoD (§7 item 3) records "Normalized branch unbound" as a
known coverage gap. The minimum fixture additions to move from "green on three graphs" toward "green
on the divergence classes that bite": a disconnected graph, a non-unit-weighted graph, a hub/star
graph (degree + summation-order stress), a self-loop/multi-edge case exercising the incidence-vs-CSR
contract difference explicitly, and the Normalized parity test.

**(c) Long-term — the debt of three parity-tested implementations.** The decision to keep three
hot-path representations (dense for the `n≤32` eigensolve, CSR for PPR/SpMV sparsity, allocation-free
implicit-grid stencil for the UI hot path) bound by parity tests *rather than merged into one* books
concrete debt: every future edit to any one of the three must re-run and, for new cases, extend the
parity suite; a new Laplacian variant (a fourth hot path, or the currently-unbound Normalized branch)
must be *actively added to the parity web* or it drifts unpinned; and the tests are the *only* thing
holding the identity — tests can be skipped, weakened, or left behind, so the guarantee is exactly as
durable as the suite's upkeep. **Is this permanent or temporary?** Honestly: a *deliberate,
possibly-permanent* scope-narrowing, not an accident. The three representations serve genuinely
different performance profiles, and merging would pessimize at least two while touching the
eigensolver and the integrator (§2b) — so this is not debt awaiting an obvious payoff. It is
**revisitable but not urgent**, and it carries a named trigger in the same E53-waiver discipline the
arc uses elsewhere:
> *what:* unify the ≥3 Laplacian implementations behind one operator (retire parity-by-test for
> identity-by-construction). *why-suspended:* three distinct hot-path performance profiles; merge
> pessimizes ≥2 and is a much larger refactor. *named-owner:* the kernel-operators owner (E1's
> `incidence.rs` author; same owner as finding #22's `from_edges` hub). *revisit-trigger:* a **fourth**
> Laplacian representation is proposed, **or** the parity suite's maintenance cost (parity-test edits
> per Laplacian change) exceeds what a single-implementation refactor would remove, **or** finding
> #22's `mat.rs` `from_edges` hub lands as the single home. *date:* 2026-07-16.

Until a trigger fires, "three implementations, parity-tested" is the standing architecture — a
conscious choice with a written exit condition, not silent debt.

---

*Verified live on `feat/spectral-energy-flow-evolution` as of 2026-07-16. Read-only planning
artifact; no source code was written or edited. §6-§9 added 2026-07-17; all file:line and cross-arc
claims (`hydra.rs`, `event_log.rs`, `field_frame.rs`, `engine/Cargo.toml`, B1-B4 mesh blueprints,
H2 §5) re-read live before writing.*
