# OPUS — Spectral-Everywhere Sweep (CPU & GPU)

> **Research-only. Writes no product code, touches no branch, pushes nothing.**
> Recreated 2026-07-19 after the original 2026-07-18 file was accidentally deleted;
> filename/date preserved so the existing references resolve
> (`SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` row R15,
> `OPUS-HIGHERABSTRACTION-PRODUCT-SCAN-2026-07-19.md`).
>
> **Directive (operator, verbatim):** *"спектральну логіку поширити на інші частини
> коду, зокрема CPU & GPU"* — spread spectral logic to other parts of the code, CPU
> **and** GPU — *"модальна eigendecomposition також для GPU — не обговорюється"* —
> modal eigendecomposition for GPU too, not up for debate.
>
> **Method:** live `Read`/`Grep` over the working tree (kernel + engine + bebop2),
> file:line citations throughout. Discipline per the day's rule: a surface earns a
> spectral target only when the real workload justifies it; the finding is not padded
> to satisfy the directive, but a genuine fit is not undersold either — the operator
> has made the GPU modal path non-negotiable, and there IS a real fit for it.

---

## 0. One-paragraph verdict

The kernel is **already spectral at every CPU surface where a general
eigendecomposition genuinely earns its keep** — `spectral.rs` was built precisely to
fill the four gaps its own module doc names (`spectral.rs:3-15`), and those four
consumers (`order_machine` ρ, `markov` λ₂/gap/period, `spectral_laplacian` modes,
`spectral_cache`, plus the hidden `hydra` spectral spine) are all wired. The sweep
found **no new high-value CPU graph surface** that a spectral method would improve
over the exact combinatorial tool already in place. The **one genuinely new,
high-value, operator-mandated extension is the GPU field-eigenmode path**: precompute
the Laplacian modal basis on the CPU (`spectral_laplacian::laplacian_eigenmodes`,
which already exists but has **no live caller yet** — `kernel/src/lib.rs:247`), upload
the basis to the GPU, and evolve modal amplitudes GPU-side. This is the
James & Pai *DyRT* pattern (SIGGRAPH 2002) and it is exactly what blueprint unit P89
covers. Two small, honest, already-computable opportunities exist as *complements*
(never replacements): algebraic-connectivity λ₂ as a mesh-health "distance-to-
partition" signal, and spectral community detection of the retrieval wikilink graph.
Everywhere else — absorbing-chain Neumann series, PPR power-iteration, Dijkstra
routing, do-calculus c-components, geo kinematics — spectral is either already the
chosen tool, **strictly worse** than the exact method, or hard-walled by the money/
float red line.

---

## 1. What `spectral.rs` actually is today (grounding)

`kernel/src/spectral.rs` (1371 LOC) is a **general (non-symmetric) spectral engine**,
not a graph-clustering library. Its own preamble (`spectral.rs:3-15`) records the four
subsystems it was reverse-engineered to serve:

- `order_machine::spectral_radius()` computed only ρ (top eigenvalue via power
  iteration) and could not see the spectral gap (`spectral.rs:6-7`);
- `tools/loop-signals/markov_attractor.py` computed all eigenvalues but only in
  Python on the hook path (`spectral.rs:8-9`);
- the hydraulic-loop design named a general real eigensolver as its #1 missing
  primitive (`spectral.rs:10-11`);
- the field-UI engine needs Laplacian eigenmodes λ_k for modal motion
  (`spectral.rs:12`).

**Surface inventory** (all in `spectral.rs` unless noted):

| Primitive | Line | Role |
|---|---|---|
| `charpoly` / `charpoly_in` | 114 / 145 | Faddeev–LeVerrier char-poly (n>32 fallback + arena twin) |
| `roots` | 171 | Durand–Kerner all-complex-roots, deterministic seed |
| `eigenvalues` | 225 | Householder (n≤32) → FL+DK fallback |
| `eigh` | 251 | dense symmetric decomposition (n≤32) |
| `topk_symmetric` / `_in` | 269 / 411 | sparse CSR top-k, Hotelling-deflated power method |
| `spectral_radius` / `slem` / `spectral_gap` | 541 / 550 / 563 | ρ, |λ₂|, γ=1−|λ₂| |
| `laplacian` | 620 | **`+(D−A)`** dense graph Laplacian |
| `algebraic_connectivity` | 635 | Fiedler λ₂ |
| `graph_energy` / `graph_spectrum` | 579 / 596 | Σ|λ| activity + one-pass profile |
| `classify_drift` / `dominant_period` | 679 / 744 | DMD |μ|-vs-1 class + oscillation period |

Graph-Laplacian convention is **positive `+(D−A)`** (`spectral.rs:617-630`), and this
is the shared kernel convention: `incidence::laplacian` (`incidence.rs:23-26`, 102-117),
`csr::laplacian_spmv(_, Unnormalized)`, and `spectral_laplacian::build_laplacian`
(`spectral_laplacian.rs:35-54`) are all parity-bound to it.

**CPU consumers already spectral (correctly scoped):**

- `markov.rs:1-19` — the Markov attractor detector *reuses* `crate::spectral`
  (`markov.rs:21`), killing the old dual-authority Python/Rust eigensolver hazard.
  slem, gap γ, mixing time τ≈1/γ, and the period signal all come from `spectral.rs`.
- `order_machine::spectral_radius()` — ρ via power iteration; the sub-dominant λ₂ it
  lacked is now supplied by `spectral.rs`. Cross-checked live at `spectral.rs:889-893`.
- `spectral_laplacian.rs` — the canonical Laplacian-eigenmode producer (the field-UI
  Fourier basis), reusing `eigh`/`topk_symmetric` (`spectral_laplacian.rs:19-25`).
- `spectral_cache.rs:1-16` — content-addressed eigensolve cache with a monotonic
  `recomputes` falsifier.
- `hydra.rs:1-11` — the hidden self-evolution spine explicitly hides a "decide/fold +
  **spectral** spine" (`hydra.rs:5-6`).

The conclusion this forces: **`spectral.rs` is not under-deployed on the CPU. It is
deployed at precisely the surfaces that named it as a missing primitive**, and it was
built to those four requirements, not speculatively.

---

## 2. The field engine: operator convention & the sign mismatch (confirmed REAL)

**Question:** can `spectral.rs`'s Laplacian be naively reused for field eigenmodes?

**Answer:** the operators differ in sign, so a naive splice would inject
anti-diffusion — but the mismatch is already *known, documented, and bit-pinned*, and
critically it does **not** block modal reuse (see the nuance below).

**Field operator.** `engine/src/field_frame.rs` integrates the operator's physics-
render equation `M·U̇ = −Γ·U̇ − c²·L·U + S` (`field_frame.rs:3-16`) where the grid
Laplacian is the **negative** stencil `∇² = −(D−A)`, negative-definite
(`field_frame.rs:120-123`). The kernel/CSR/incidence side is the **positive** `+(D−A)`.

**The mismatch is real and matters.** `incidence.rs:8-13` names it outright: the grid
stencil "uses the OPPOSITE sign convention … a latent sign bug that produces zero
failures until a caller crosses the seam and gets anti-diffusion (divergence)." It is
not left to trust — `engine/src/field_energy.rs` carries a **razor-sharp bit-level
sign-pin** binding the two: `field_frame::laplacian == −incidence.laplacian` on
interior nodes (`field_energy.rs:187-227`), and it *also* asserts that the naive
shared-convention form `stencil == +incidence` is **FALSE** at interior nodes
(`field_energy.rs:193-197`, 244-245) — i.e. the split genuinely exists and the sign
actually matters. Flipping the stencil sign is proven to give the anti-diffusion break
where the largest-eigenvalue modes GROW and energy increases (`field_energy.rs:327-329`).

**The load-bearing nuance (why the mandate is still trivially satisfiable).**
`−(D−A)` and `+(D−A)` are the same operator up to a global sign. A global sign flip
**negates eigenvalues but leaves eigenvectors identical**. Therefore:

- the **modal basis** (eigenvectors) produced by
  `spectral_laplacian::laplacian_eigenmodes` on the `+(D−A)` Laplacian is
  **directly reusable** as the field's modal basis — no reconciliation needed on the
  vectors;
- only the **eigenvalue signs/ordering** must be reinterpreted: the field wants the
  *smoothest* (smallest `+(D−A)` eigenvalue ⇒ largest, least-negative `−(D−A)`) modes
  first. `laplacian_eigenmodes` already returns ascending `+(D−A)` eigenvalues, i.e.
  smoothest-first (`spectral_laplacian.rs:56-101`), which is exactly the field-UI
  order.

So the "cannot be naively reused" caveat is real for the *time-stepping operator*
(splicing the wrong sign = divergence, already fail-closed), but **false for the modal
eigenbasis** — which is the thing the GPU path in §4 actually needs. The engine, by
design, never re-implements eigensolving: it consumes the kernel spectrum over the
FE-07 wasm bridge (`engine/src/bridge.rs:657-678`, decoding ρ/slem/gap/drift/eigen-
pairs), per the 2026-07-14 "engine uses kernel math" directive (`bridge.rs:659-662`).

---

## 3. Candidate-surface sweep (honest, per-candidate)

Every graph-like / iterative surface in the kernel, assessed for whether a spectral
method would genuinely improve it. Verdicts are evidence-first.

| Surface | File:line | Current method | Spectral fit? |
|---|---|---|---|
| Order lifecycle funnel | `absorbing.rs:1-14` | Absorbing Markov chain; **nilpotent Q** ⇒ exact finite Neumann series `N = Σ Q^k` (Q⁵=0), no inversion, no tolerance | **NO.** Spectral is strictly worse: the answer is closed-form and exact. Eigendecomposition would add iteration + float error for zero gain. |
| Retrieval relatedness (L3) | `retrieval/ppr.rs:1-16`, `retrieval/diffusion.rs:1-16` | Personalized-PageRank power iteration, **deliberately no eigendecomposition** (fixed K, fixed summation order, mass-conserving, seed-personalized) | **Mostly NO.** PPR is chosen for byte-determinism + per-seed personalization; a global eigenbasis loses the personalization. Spectral *community detection* of the 20-node wikilink graph is feasible but low-value (see §3a). |
| Connected components / mesh-heal | `dsu.rs:1-13`, 84-93 | Union-Find + Kruskal MST; exact partition in near-linear time | **NO for components** (DSU is optimal, exact, deterministic). **Small YES for a complement:** λ₂ as "distance-to-partition" (§3b). |
| Causal c-components | `cgraph.rs:11-13` | Confounded-component partition via DSU flood-fill (Shpitser–Pearl do-calculus) | **NO.** Structural/exact identifiability; no numeric relaxation is admissible — a spectral approximation would be *wrong*, not just slower. |
| Shortest-path routing | `router.rs:1-14` | CSR-native Dijkstra / A* / contraction hierarchies, admissible haversine heuristic | **NO.** Exact combinatorial shortest path; spectral is irrelevant to metric routing. |
| Geo kinematics | `geo.rs:1-6`, 38-70 | Haversine / lerp / bearing / EMA (1-D Kalman) / ray-cast PIP | **NO.** Pointwise scalar kinematics — no graph, no operator to decompose. |
| Food-court / catalog / money | `foodcourt.rs:1-15`, `money.rs` | Integer money, N-leg saga, PCI red line | **NO — HARD WALL.** `spectral.rs:25-27` states float is graph/operator-only; money is integer-only. Spectral/float is *forbidden* here, not merely unhelpful. |
| Attention lens | `attention.rs:1-12` | `softmax(QKᵀ/√d)·V` framed as ONE diffusion step of the `f(L)` family | **Already scoped.** The lens is explicit; the spectrum of the learned affinity is computable but has no consumer need. Marginal. |
| Self-evolution spine | `hydra.rs:5-6` | decide/fold + hidden spectral spine | **Already spectral.** |

### 3a. Spectral community detection of the retrieval graph — feasible, low-value

The wikilink graph (`retrieval/diffusion.rs:11` — a frozen 20-node/41-edge fixture)
could be spectrally clustered (Fiedler-vector bisection, Pothen–Simon–Liou 1990) to
surface communities. But: (i) the graph is tiny and exact-search L0/trigram already
covers lookup; (ii) PPR already answers "what is related to X" with personalization a
static clustering cannot; (iii) `algebraic_connectivity`/`eigh` already exist to do it
if a real corpus ever makes it worthwhile. **Verdict: real technique, no live payload
— park it until the retrieval graph is 10²–10³ nodes and a *global* community view
(not per-seed diffusion) is actually requested.**

### 3b. Algebraic connectivity λ₂ as mesh-health early-warning — small genuine win

`dsu::components` gives a *binary* answer: connected or not. The mesh-heal /
partition-tolerance consumers (`dsu.rs:5-7`) act only *after* a partition. The Fiedler
value λ₂ (`spectral.rs:635`, `algebraic_connectivity`) is the **continuous**
"distance-to-partition" that DSU cannot express: λ₂→0⁺ warns that the mesh is *about
to* split while it is still connected. This is:
- **already computable** (`algebraic_connectivity` is live and tested,
  `spectral.rs:909-923`), just **not wired** to any mesh-health readout;
- **complementary, never a replacement** — DSU stays the authority for the actual
  partition; λ₂ is an advisory early-warning gauge, in the same advisory spirit as the
  `markov` attractor signals.

**Verdict: a real, small, low-risk extension** — expose λ₂ (and the existing
`graph_spectrum` profile) as a mesh-connectivity health signal. It writes no new math,
only a wiring. Sketch: on the mesh adjacency snapshot, call `graph_spectrum(adj)` and
surface `.fiedler` as an advisory gauge; alarm as it approaches the `DRIFT_BAND`
(`spectral.rs:677`) of zero.

---

## 4. The GPU angle — the mandated, high-value extension (P89 / DyRT)

The directive's non-negotiable clause is *modal eigendecomposition for the GPU*. This
is **not** about finding a new graph surface; it is the canonical
**CPU-offline-eigendecomposition → GPU-online-modal-synthesis** production pattern:

- **Precedent (real):** D. L. James & D. K. Pai, *"DyRT: Dynamic Response Textures for
  Real Time Deformation Simulation with Graphics Hardware,"* ACM TOG / SIGGRAPH 2002,
  21(3):582–585. Modal analysis (eigenvectors of a linearized elastic operator) is
  precomputed **once on the CPU**; at runtime only the small vector of **modal
  amplitudes** is evolved and the deformation is reconstructed **on the GPU**. The
  expensive eigensolve never runs per-frame. (Cited from standing knowledge — live web
  verification was unavailable this session; the citation is a well-established
  graphics result and matches the field-UI modal-motion requirement one-for-one.)

- **Why it fits DeliveryOS exactly:** the field-UI equation
  `M·U̇ = −Γ·U̇ − c²·L·U + S` (`field_frame.rs:10`) is a damped linear wave operator.
  Its motion is a superposition of Laplacian eigenmodes with per-mode damped-oscillator
  amplitudes. Diagonalizing `L` **once** (CPU, `spectral_laplacian::laplacian_
  eigenmodes`, `spectral_laplacian.rs:83-101`) converts the coupled grid update into
  `k` *independent* scalar modal ODEs — embarrassingly parallel, and cheap enough to
  run every frame on the WebGL2/wgpu floor by uploading the `n×k` modal basis once and
  streaming only the `k` amplitudes.

- **The sign mismatch does not block it** (§2 nuance): the eigenvectors of `+(D−A)` and
  the field's `−(D−A)` are identical; `laplacian_eigenmodes` already returns
  smoothest-first modes with an orthonormal, sign-fixed, byte-deterministic basis
  (`spectral_laplacian.rs:56-101`, determinism pinned `spectral_laplacian.rs:217-242`).
  Determinism is a hard requirement for a GPU upload that must reproduce across clients,
  and it is already guaranteed.

- **The producer exists but is unwired.** `spectral_laplacian` is registered
  (`kernel/src/lib.rs:247`) yet has **no live caller** in the tree (grep: only the
  `mod` line). P89 is precisely the wire-in: CPU `laplacian_eigenmodes` → FE-07 bridge
  → GPU modal-amplitude evolution. This confirms R14's note in
  `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md:31`: the source physics report
  recommended DCT/FFT over `spectral.rs`; **the operator overruled that in favour of
  the kernel eigensolver** (that doc §2 row 3 / P89). This sweep agrees with the
  override on the merits: DCT/FFT modes are only correct on a *regular* lattice with
  uniform boundaries; `spectral.rs` eigenmodes are correct on *any* weighted graph
  Laplacian (irregular meshes, Neumann/mixed boundaries, weighted couplings), which is
  the field engine's actual domain (`field_frame.rs:120-123` uses Neumann zero-flux
  edges — a boundary a raw DCT does not honour).

- **GPU packing corollary (already ruled).** Complex/multi-channel modal state packs
  into RGBA/RG32F texels with free hardware bilinear sampling — the R13 GPU-texture-
  packing ruling (`SYNTHESIS-…:30`) is the transport for the modal amplitudes; this
  sweep does not re-open it.

**Verdict on the GPU mandate: satisfiable and genuinely valuable, with the primitive
already built.** The work is wiring + a GPU modal-amplitude integrator, not new
kernel math. This is the single new high-value spectral target the sweep found, and it
is exactly the one the operator named.

---

## 5. The money wall (unconditional)

`spectral.rs:25-27` states the rule verbatim: float is used *deliberately* because
"this is graph/operator structure, never money (the no-float rule is money-only)."
Every candidate that touches `money.rs` / `foodcourt.rs` / the CPU determinism-crypto
oracle is therefore **out of scope by red line**, independent of any spectral merit.
This matches the excluded-scope item in `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-
18.md:53-55`. No sketch is offered for those surfaces because none is admissible.

---

## 6. Final verdict

1. **CPU: already spectral where it belongs.** `spectral.rs` is deployed at exactly the
   four surfaces that named it as a missing primitive (`order_machine`, `markov`,
   `spectral_laplacian`, hydraulic/`hydra`). The sweep found **no new high-value CPU
   graph surface** a spectral method would improve over the exact tool already there —
   absorbing chains (Neumann series), routing (Dijkstra), components (DSU), and
   do-calculus (c-components) are each *correctly* non-spectral, and several would be
   strictly *worse* if forced spectral.

2. **GPU: one real, mandated, high-value extension.** The field-UI modal path (P89 /
   DyRT): CPU-offline `laplacian_eigenmodes` → upload basis → GPU modal-amplitude
   synthesis. The producer exists (`spectral_laplacian.rs`), is unwired
   (`lib.rs:247`), and the sign mismatch is a non-issue for the eigenVECTORS the GPU
   needs. The operator's overrule of the DCT/FFT recommendation is *correct on the
   merits* for the engine's irregular/Neumann domain.

3. **Two small honest complements** (already computable, never replacements): λ₂ as a
   mesh "distance-to-partition" early-warning (§3b — a wiring, not new math), and
   spectral community detection of the retrieval graph (§3a — parked until the corpus
   and a global-view requirement justify it).

4. **The money/oracle red line is an absolute wall** for spectral/float (§5).

This is consistent with the record already in
`SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md:32` (R15): *"no new high-value
spectral target beyond the field engine; kernel already spectral everywhere it applies;
graph `+(D−A)` vs field `−(D−A)` operator mismatch re-confirmed; money = absolute wall
for spectral/float."*

---

### Appendix — primary sources (file:line)

- `kernel/src/spectral.rs:3-27` (raison d'être + float/money rule), `:617-644` (+(D−A)
  Laplacian, Fiedler), `:889-893` (order_machine cross-check)
- `kernel/src/spectral_laplacian.rs:1-101` (eigenmode producer), `:217-242` (determinism pin)
- `kernel/src/incidence.rs:8-26`, `:102-117` (sign-convention statement + reference `+(D−A)`)
- `engine/src/field_frame.rs:3-16`, `:120-123` (field `−(D−A)` operator)
- `engine/src/field_energy.rs:11-16`, `:106-107`, `:187-227`, `:244-245`, `:327-329` (bit-level sign-pin + anti-diffusion break)
- `engine/src/bridge.rs:657-678` (FE-07 kernel-spectral bridge; engine never re-eigensolves)
- `kernel/src/markov.rs:1-21`, `absorbing.rs:1-14`, `retrieval/ppr.rs:1-16`,
  `retrieval/diffusion.rs:1-16`, `dsu.rs:1-13`, `cgraph.rs:11-13`, `router.rs:1-14`,
  `geo.rs:1-6`, `attention.rs:1-12`, `hydra.rs:5-6`, `spectral_cache.rs:1-16`,
  `kernel/src/lib.rs:247`
- **External:** D. L. James & D. K. Pai, *DyRT: Dynamic Response Textures*, SIGGRAPH
  2002 (CPU-offline modal analysis → GPU real-time synthesis); Fiedler 1973 (algebraic
  connectivity) / Pothen–Simon–Liou 1990 (spectral partitioning). *Live web
  verification unavailable this session — WebSearch budget exhausted; citations from
  standing knowledge, flagged as such.*
