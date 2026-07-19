# Physics / Wave-Rendering Algorithms & Atomicity·Quantization·Eigenvector·Tensor Synthesis

> Research-only. Zero code written, no branches touched. This is a recreation of a
> doc deleted earlier today; the investigation was redone from scratch against live
> source and fresh web sources. Referenced by
> `docs/design/CORE-ROADMAP-2026-07-17/SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md`
> row R14.
>
> Provenance note: the eigenvector verdict below (favor DCT/FFT over reusing
> `spectral.rs` for field eigenmodes) is the honest *technical* finding. The operator
> has since **overruled** it as a deliberate falsifiable bet (mandating `spectral.rs`
> be used anyway — see P89 / SYNTHESIS §2 row 3). That policy override is a separate,
> already-settled item and is **not** re-litigated here. This document reports only
> what the physics and the code actually say.

---

## 0. What the field engine actually is (read, not assumed)

The renderer is a **CPU, single-threaded, semi-implicit damped-wave PDE integrator**
on a row-major SDF field buffer. The governing operator is stated verbatim in the
source header (`engine/src/field_frame.rs:9-14`):

```
M·U̇ = −ΓU̇ − c²·L·U + S   with  U̇ ≈ (U − U_prev)/dt
⇒ U_next = (U + dt·(Γ·U̇ + c²·L·U) + dt·S) / (1 + dt·M)
```

Concretely, from the live code:

- **State**: two full-frame f32 buffers `u` (current) and `u_prev`
  (`field_frame.rs:161-162`), plus two pre-allocated scratch buffers so `step()`
  is allocation-free (`field_frame.rs:163-166, 174-183`).
- **Spatial operator `L`**: a 5-point finite-difference Laplacian with **Neumann
  zero-flux edges** (missing neighbour = centre value), `field_frame.rs:140-154`.
  Sign convention is the **physics** Laplacian `∇² = −(D−A)` (negative-definite),
  explicitly documented as the *opposite* sign of the kernel's graph Laplacian
  `+(D−A)` (`field_frame.rs:119-128`).
- **Time integration**: a sequential per-cell loop (`field_frame.rs:207-216`) doing
  a backward-difference velocity `U̇ = (U−U_prev)/dt` and the semi-implicit
  `(1+dt·M)` denominator, followed by a **two-`std::mem::swap` buffer rotation**
  (`field_frame.rs:220-221`) — the classic ping-pong.
- **Clock**: `dt` pinned to the kernel's authoritative `DT_STABLE = 0.02 s` = 50 Hz
  (`field_frame.rs:51-64`, `kernel/src/lib.rs:338`).
- **Stability**: fail-closed `assert_stable()` (`field_frame.rs:70-98`) enforcing the
  von Neumann / Jury bound for the 2-D 5-point Neumann Laplacian (λ ∈ [−8,0]):
  `dt < (2 + 2·Γ) / (8·c² − M)`, with a regression canary proving the old 1-D
  formula `M/(Γ+2c²)` wrongly admitted a divergent dt=0.45 (`field_frame.rs:283-298`).
- **Presentation**: `frame_rgba()` maps sign→hue, magnitude→brightness, quantizing
  the f32 field to **u8** RGBA for display (`field_frame.rs:229-249`).
- **Energy certificate** (test-only): `field_energy.rs` wires the kernel's
  `noether::lyapunov_nonincreasing` to prove the Dirichlet energy is monotone
  non-increasing per step (`field_energy.rs:288-320`), and pins the `−(D−A)` vs
  `+(D−A)` sign split with a red-provable tripwire (`field_energy.rs:200-248`).

**This is textbook computational physics done correctly.** The scheme is a member of
the FDTD / leapfrog / Newmark family for the damped wave equation, and its stability
guard is a genuine CFL-class condition (see §1). Two-position storage (`u`, `u_prev`)
+ backward-difference velocity is exactly the position-Verlet idiom. Nothing here is
naive.

---

## 1. Established wave/particle rendering algorithms (real citations) and their fit

Five load-bearing algorithms from production graphics, each fetched and verified this
pass, mapped honestly onto what dowiz's field engine actually does.

### 1.1 FDTD + the CFL condition — *directly applicable; already implemented*

The finite-difference time-domain method marches a wave field on a grid with a
leapfrog time update, and is stable only under the **Courant–Friedrichs–Lewy (CFL)**
bound; in 2-D, `Δt ≤ Δx/(c√2)`
([Wikipedia: FDTD](https://en.wikipedia.org/wiki/Finite-difference_time-domain_method)).
dowiz's integrator is precisely this family: an explicit/semi-implicit stencil march
with a fail-closed stability assert. The 2-D Jury bound
`dt < (2+2Γ)/(8c²−M)` at `field_frame.rs:81-97` **is** the damped-wave CFL condition
for the 5-point Neumann Laplacian — the `8c²` factor is the |λ|=8 worst-case
(checkerboard) eigenvalue of that stencil, the direct analogue of FDTD's `√2`.
Verdict: real, established, and **already correct in the tree**. No change indicated;
the value of the citation is that it *confirms the existing guard is the right one*.

### 1.2 Verlet / semi-implicit (symplectic) Euler — *directly applicable; already the idiom*

Position-Verlet stores `x_n, x_{n-1}` and reconstructs velocity by finite difference,
giving a symplectic, time-reversible scheme whose energy oscillates around truth
rather than drifting
([Wikipedia: Verlet integration](https://en.wikipedia.org/wiki/Verlet_integration)).
dowiz stores exactly `u` and `u_prev` and reconstructs `U̇ = (u−u_prev)/dt`
(`field_frame.rs:212`) — the position-Verlet pattern, with an added damping term and
the semi-implicit mass denominator for extra dissipation. `motion.rs` independently
uses semi-implicit Euler for its spring integrator (`engine/src/motion.rs:47-63`).
Verdict: established and **already the codebase's chosen scheme**; the Lyapunov
energy gate (`field_energy.rs:288`) is the empirical proof the discretization is
dissipative-correct.

### 1.3 Tessendorf FFT ocean waves — *real, NOT applicable as-is; feeds the eigenbasis argument*

Tessendorf, *Simulating Ocean Water* (SIGGRAPH course notes), builds an ocean height
field as a **statistical superposition of sinusoids drawn from a Phillips spectrum**,
evolves each Fourier mode by the deep-water dispersion relation, and inverse-FFTs to
the spatial height field each frame at **O(N log N)**
([Tessendorf PDF](https://jtessen.people.clemson.edu/reports/papers_files/coursenotes2004.pdf)).
It deliberately *replaces* PDE integration with spectral synthesis.
Fit: dowiz's field is **source-driven** (SDF shapes are attractors/repellors,
`field_frame.rs:255-256`), not a wind-driven statistical open-ocean surface. The
Tessendorf model is **not applicable as a swap**. Its enduring lesson is the one that
*is* relevant: on a regular grid, evolving in a **spectral (Fourier/cosine) basis is
O(N log N)** and diagonalizes the spatial operator — which is exactly the eigenmode
argument in §3.3.

### 1.4 DyRT modal analysis — *real, applicable to the FUTURE modal-motion use; picks the basis question*

James & Pai, *DyRT: Dynamic Response Textures for Real Time Deformation Simulation
with Graphics Hardware*, SIGGRAPH 2002
([DyRT PDF](https://www.cs.cornell.edu/~djames/papers/DyRT.pdf)). The method
**precomputes the dominant eigenmodes** (natural frequency + spatial shape) of an
elastic FEM body; the modal transformation **decouples the ODE system**, so runtime
animation is a superposition of a handful of independent modal oscillators driven by
rigid-body motion — replacing full FEM time integration with lightweight per-mode
convolutions. This is the canonical justification for the field engine's *own*
stated future need: `kernel/src/spectral.rs:12` says "the field-UI engine needs
Laplacian eigenmodes (λ_k) for modal motion." DyRT is the reason that need is real.
It also forces the central question of §3: **which eigenbasis, computed how?**

### 1.5 Stam Stable Fluids — *real, NOT applicable; its stability lesson already borrowed*

Stam, *Stable Fluids*, SIGGRAPH 1999
([Stam PDF](https://www.dgp.toronto.edu/people/stam/reality/Research/pdf/ns.pdf)):
semi-Lagrangian backward advection + an **implicit** pressure projection give
**unconditional stability for any Δt**, escaping the CFL limit; the periodic variant
uses an FFT solver. Fit: dowiz has **no advection term and no incompressibility
constraint** — it is a linear damped-wave field, not a Navier–Stokes fluid. Not
applicable. But the general lesson (an *implicit* term buys stability) is already
present: the `(1+dt·M)` semi-implicit denominator (`field_frame.rs:214`) is precisely
a small implicit stabilizer widening the real margin above the asserted explicit
bound (`field_frame.rs:36-38`).

---

## 2. Atomicity / Quantization / Tensor — concrete verdicts against live code

### 2.1 Atomicity (lock-free / CAS) — *not applicable to the step; first real site is the energy reduction*

**Finding (grounded):** `engine/src/` contains **zero atomics** — a full grep for
`Atomic*`, `compare_exchange`, `fetch_add`, `Ordering::` returns hits **only** in
`kernel/src/*` (`wasm.rs`, `arena.rs`, `spectral_cache.rs`, `json_api.rs`), never in
the engine. The field `step()` is single-threaded and writes each cell to a **disjoint**
index `next_scratch[i]` (`field_frame.rs:215`), then rotates buffers with
`std::mem::swap` (`field_frame.rs:220-221`). There is **no shared mutable state**, so
a CAS/atomic would be pure overhead.

- If the per-cell loop were ever parallelized (rayon / GPU), it stays
  **embarrassingly parallel with disjoint writes** — still no atomics, because it is
  a map, not a scatter/accumulate.
- The **first place** an atomic is even tempting is a **reduction**: the Dirichlet
  energy sum (`field_energy.rs:80, 91-101`) and the field-norm check
  (`field_frame.rs:404-410`). There, an atomic **float** add would be the *wrong*
  tool: float add is non-associative, so a lock-free reduction would be
  **non-deterministic**, breaking the codebase's hard bit-determinism contract
  (`compose` must be byte-identical across calls, `field_frame.rs:429-451, 459-481`).
- **Verdict:** the field step needs zero atomics. The one real future site (energy /
  norm reduction) must use a **fixed-order deterministic** (or fixed-point / Kahan)
  reduction, **not** an atomic-float-add. Lock-free machinery here is speculative and
  net-negative.

### 2.2 Quantization (f16 / i8 / fixed-point) — *presentation-side only; hard-walled from money and the oracle*

**Finding (grounded):** `engine/src/` uses **no** f16/half/fixed-point — grep returns
nothing. Compute state is f32 buffers (`field_frame.rs:161-165`) with per-cell
arithmetic promoted to **f64** (`field_frame.rs:208-215`). The only quantization that
already exists is the correct one: field → **u8** RGBA at display
(`field_frame.rs:229-249`).

- Quantizing the **display/upload** path further (f16 texels, RGBA/RG32F packing for
  multi-channel field state) is genuinely valuable on GPU and is already the scope of
  the RGB-packing lane (SYNTHESIS R13). That is presentation-side.
- Quantizing the **compute** state (`u`/`u_prev` → f16) is **not** advisable: the
  Jury stability margin is tight and the convergence gate asserts a
  `max_delta < 1e-2` equilibrium after 3000+ steps (`field_frame.rs:412-424`); f16's
  ~3 significant digits would accumulate error across that many steps and risk the
  gate. Fixed-point on the compute state buys nothing the f32/f64 split does not
  already provide.
- **Hard wall:** quantization must never touch money or the spectral/oracle decision
  path. `kernel/src/spectral.rs:25-26` states the no-float rule is money-only, and
  money is integer end-to-end. Low-bit representations are a **rendering** technique,
  firewalled from settlement and from the drift/admission classifier.
- **Verdict:** quantization is a presentation-side / GPU-upload optimization only,
  hard-walled from money and from the oracle. Compute-state quantization is
  speculative and stability-risky.

### 2.3 Tensor operations — *the stencil already IS the tensor contraction; no ML tensor lib warranted*

The field step is already a **2-D convolution** of the field with the fixed 5-tap
Laplacian kernel (`field_frame.rs:146-151`) — i.e. a tensor (stencil) contraction.
Adopting an ML tensor library (batched matmul / einsum) adds nothing to a single
scalar field evolved by a fixed small stencil. The one honest tensor angle is
**multi-channel** field state: a `w×h×C` tensor where C independent fields share the
same stencil, cadence, and stability class can be evolved by a single GPU shader pass
and share one ping-pong pair (this is exactly the SYNTHESIS R13/R16 packing rule, not
a CPU tensor-library adoption). **Verdict:** the stencil is the tensor op; no tensor
framework is warranted; the real leverage is multi-channel GPU packing, already
scoped elsewhere.

---

## 3. Eigenvectors / spectral decomposition — the domain-mismatch verdict (independently reconfirmed)

### 3.1 The claim under test

The deleted doc reportedly found that `spectral.rs`'s graph-Laplacian convention
differs in sign from what a field-engine eigenmode approach needs, and favored
DCT/FFT-style approaches for wave-specific eigenmode work over reusing `spectral.rs`
directly. I verified this independently rather than assuming it.

### 3.2 What `spectral.rs` actually computes

`kernel/src/spectral.rs` is a genuinely good **general graph/operator spectral
engine**: Faddeev–LeVerrier + Durand–Kerner for all complex eigenvalues of a general
real matrix (`spectral.rs:114-244`), a Householder fast path for n≤32
(`spectral.rs:225-261`), a sparse deflated power-iteration `topk_symmetric` over CSR
(`spectral.rs:269-402`), plus ρ / |λ₂| / spectral-gap / Fiedler / graph-energy /
drift-class (`spectral.rs:533-757`). Its Laplacian is the **positive** graph
Laplacian `L = D − A` (`spectral.rs:620-630`), and `incidence.rs` confirms the
whole-kernel convention: `csr`/`spectral` emit `+(D−A)`, `field_frame` emits `−(D−A)`
(`kernel/src/incidence.rs:9-10, 23, 103`).

### 3.3 Three concrete reasons DCT/FFT is the right eigenbasis for THIS field

1. **Sign convention differs — but that part is cosmetic.** For a *symmetric* matrix,
   `+(D−A)` and `−(D−A)` share identical eigen**vectors**; only the eigenvalue sign
   flips. So the sign mismatch (`field_frame.rs:119-128` vs `incidence.rs:23`) does
   **not** by itself make the eigenvectors wrong. It matters for the *ordering* (next
   point) and it is a real seam that has already caused a pinned tripwire
   (`field_energy.rs:200-248`).

2. **Wrong end of the spectrum — this is the substantive mismatch.**
   `topk_symmetric` returns eigenpairs sorted **descending by |λ|**
   (`spectral.rs:396-401`). For a Laplacian, the largest-|λ| modes are the
   **highest-frequency / checkerboard** modes (the |λ|=8 corner the Jury bound guards
   against). Modal motion à la DyRT (§1.4) wants the **lowest-frequency, smoothest,
   longest-wavelength** modes — the *opposite* end. Reusing `topk_symmetric` for
   field eigenmodes would hand back the noisiest modes, precisely wrong for smooth
   animation. Getting the low end from a general solver means computing the *whole*
   spectrum or shift-inverting — expensive and indirect.

3. **Wrong cost class, and the dense paths are too small.** The Neumann 5-point
   Laplacian on a `w×h` grid has a **closed-form eigenbasis: the DCT-II cosine
   modes.** The DCT is exactly the transform whose basis vectors are the eigenvectors
   of the symmetric second-difference matrix with Neumann (reflecting) boundary
   conditions — the standard result of Gilbert Strang, *The Discrete Cosine
   Transform*, SIAM Review 41(1):135–147, 1999; the even/Neumann boundary link is
   confirmed by
   [Wikipedia: DCT](https://en.wikipedia.org/wiki/Discrete_cosine_transform)
   ("DCT-II implies even boundary conditions"), and the engine's stencil uses exactly
   those Neumann zero-flux edges (`field_frame.rs:103, 129, 146-151`). A DCT is
   **O(N log N)** with no iteration and no matrix assembly. By contrast
   `spectral.rs`'s dense symmetric paths (`eigh`, Householder) **cap at n≤32**
   (`spectral.rs:253`), far below a field frame's thousands of nodes, leaving only
   `topk_symmetric`'s per-mode power iteration — many spmv sweeps per mode, and the
   wrong modes at that.

### 3.4 Verdict on eigenvectors

- **`spectral.rs` is real and excellent for its actual job** — graph spectra: FSM
  cyclicity (ρ), mesh algebraic connectivity (Fiedler), Markov drift class, order-
  machine spectral radius. Keep using it there.
- **For field eigenmodes it is a domain mismatch on three axes**: eigenvalue-sign
  convention (cosmetic), spectrum-end / ordering (substantive — `topk` gives the
  noisiest modes, not the smoothest), and cost class (general iterative solver, dense
  path capped at n≤32, vs a closed-form O(N log N) DCT that is *the* eigenbasis of the
  Neumann grid Laplacian).
- **The DCT/FFT recommendation is technically well-founded and independently
  confirmed here.** (Operator override to use `spectral.rs` anyway as a falsifiable
  bet is a separate settled policy item, §0, and is not contested by this technical
  finding.)

---

## 4. Honest three-bucket summary

**Real, established, directly applicable — and ALREADY in the tree (no change needed):**
- FDTD / leapfrog wave march with a CFL-class stability guard → the Jury bound
  `dt < (2+2Γ)/(8c²−M)` at `field_frame.rs:81-97`.
- Position-Verlet / semi-implicit Euler (two-position storage + backward-difference
  velocity + implicit mass term) → `field_frame.rs:161-162, 212-221`.
- The `frame_rgba` u8 quantization at the display boundary → `field_frame.rs:229-249`.

**Real, but NOT applicable to dowiz's actual field (do not swap in):**
- Tessendorf FFT ocean (statistical wind-driven surface; dowiz is source-driven).
- Stam Stable Fluids (advection + incompressible projection; dowiz has neither).
- DyRT modal analysis is real and applicable *to the future modal-motion feature* —
  but it selects the DCT basis, not a general graph eigensolver, for the Neumann grid.

**Speculative / net-negative for the actual code (reject unless evidence changes):**
- Lock-free atomics in the field step (zero shared state; the only real site is a
  reduction that must stay fixed-order-deterministic, not atomic-float).
- Low-bit quantization of the **compute** state (stability-margin risk; presentation-
  side f16/packing is fine and lives in the GPU lane).
- Adopting an ML tensor library (the 5-tap stencil already IS the tensor contraction).
- Reusing `spectral.rs` for field eigenmodes on the merits — a domain mismatch;
  DCT/FFT is the correct eigenbasis. (Overridden by operator as a deliberate bet; not
  re-argued here.)

---

## 5. Sources

Code (live, this pass):
- `engine/src/field_frame.rs` (integrator, Laplacian, Jury bound, frame_rgba)
- `engine/src/field_energy.rs` (Lyapunov energy gate, sign-pin tripwire)
- `engine/src/motion.rs` (semi-implicit Euler spring)
- `kernel/src/spectral.rs` (general spectral engine, `topk_symmetric`, `eigh`)
- `kernel/src/incidence.rs` (`+(D−A)` vs `−(D−A)` convention statement)
- `kernel/src/lib.rs:338` (`DT_STABLE = 0.02`); `engine/Cargo.toml:8,45-55` (wgpu out of scope)

Literature (fetched and verified):
- Jerry Tessendorf, *Simulating Ocean Water* — [PDF](https://jtessen.people.clemson.edu/reports/papers_files/coursenotes2004.pdf)
- Doug L. James & Dinesh K. Pai, *DyRT: Dynamic Response Textures for Real Time Deformation Simulation with Graphics Hardware*, SIGGRAPH 2002 — [PDF](https://www.cs.cornell.edu/~djames/papers/DyRT.pdf)
- Jos Stam, *Stable Fluids*, SIGGRAPH 1999 — [PDF](https://www.dgp.toronto.edu/people/stam/reality/Research/pdf/ns.pdf)
- [Wikipedia: Finite-difference time-domain method](https://en.wikipedia.org/wiki/Finite-difference_time-domain_method) (CFL 2-D: `Δt ≤ Δx/(c√2)`)
- [Wikipedia: Verlet integration](https://en.wikipedia.org/wiki/Verlet_integration) (`x_{n+1}=2x_n−x_{n-1}+aΔt²`, symplectic)
- [Wikipedia: Discrete cosine transform](https://en.wikipedia.org/wiki/Discrete_cosine_transform) (DCT-II ↔ even/Neumann boundary)
- Gilbert Strang, *The Discrete Cosine Transform*, SIAM Review 41(1):135–147, 1999 (DCT basis = eigenvectors of the Neumann second-difference / Laplacian matrix)
