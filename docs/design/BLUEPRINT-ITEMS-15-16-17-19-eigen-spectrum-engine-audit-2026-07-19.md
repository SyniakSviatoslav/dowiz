# BLUEPRINT — Items 15 / 16 / 17 / 19: eigen-surface, GraphSpectrum, engine thick/thin, retrieval spectral-routing (Tier 0 audits)

> Planning artifact, 2026-07-19. Covers the four "any order" Tier-0 read-only audits from
> `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A (items 15, 16, 17, 19), proof
> conditions from `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` items 15–17
> (lines 247–249) and 19 (line 292), sections §11/§12 (items 15/16), §13 (item 17), §15 (item 19).
> All file:line cites below were verified against the live tree this session (post-`e10ea4e54`).
> Status legend: **PRE-ANSWERED** = this blueprint already read the load-bearing bytes; executor
> confirms, does not re-derive.

---

## Item 15 — eigen-surface entry-point + parity-scope verification. **PRE-ANSWERED (holds, one scope gap)**

**Proof condition (synthesis line 247, restated):** read the bodies of `eigenvalues` / `eigh` /
`topk_symmetric` and show they route into one backend; identify the cross-solver parity test *by
name and scope*. Outcome = either (a) single backend + named parity test cited file:line, with a
vector-scope test added if parity is values-only, or (b) a P2 defect filed. Context: commit
`03ac0fefe` (BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17) claimed single surface, no `lowrank.rs`.

**Confirmed locations & preliminary read:**
- `kernel/src/spectral.rs:225` `pub fn eigenvalues` — n ≤ 32 → `crate::householder::eigenvalues_contig`
  (`:234`); n > 32 → `charpoly` + Durand-Kerner `roots()` (`:236-244`). The charpoly path is
  *documented* as deliberate dual-role: ":224 — n > 32 and as the parity oracle in `householder::tests`".
- `kernel/src/spectral.rs:251-260` `pub fn eigh` — pure façade over `householder::eigh_contig`
  (`kernel/src/householder.rs:387`), `debug_assert!(n <= 32)`.
- `kernel/src/spectral.rs:269` `pub fn topk_symmetric` — independent Hotelling-deflated power
  iteration on `Csr` (sparse tier, own loop; doc: "sign fixed as in `eigh_contig`"). Also
  `topk_symmetric_in` at `:424`.
- `lowrank.rs` — **absent** (`ls` fails; zero grep hits in `kernel/src/`). The `03ac0fefe` claim holds.
- Parity tests, **by name**:
  - `householder.rs:646` `eigh_values_match_eigenvalues_contig` (applied `:782-784`) — values parity.
  - `householder.rs:597` R1 block — `eigh_contig` KAT + orthonormality; sign-determinism `:804-805`.
  - `spectral.rs:1216` `r3_topk_symmetric_parity_p3` — sparse-topk vs dense-eigh cross-solver parity.
  - `spectral.rs:1257` `r3_topk_symmetric_determinism` — bitwise vector determinism.

**The one finding — parity SCOPE:** `r3_topk_symmetric_parity_p3` compares eigenvalue
*magnitudes* only (`spectral.rs:1238`) and validates only the *dominant* eigenvector, by
*residual* (`:1242-1253`); the dense eigh basis is explicitly discarded — `let _ = dvecs;` at
`spectral.rs:1255`. This is byte-for-byte the shape §12 warned about (bebop finding #25: "parity
harness binds eigen*values* only, vectors validated by residual and no second solver"). Per the
proof condition, the outcome is branch (a)-with-addendum: **single surface confirmed; parity test
named; a cross-solver vector-scope test (topk basis vs eigh basis, sign/ordering/degenerate-subspace
convention) is the missing piece.** Executor: confirm the cites above (one Read each), then either
add the vector-scope test (small, same-file) or file the P2 defect doc — do not re-derive routing.

---

## Item 16 — `GraphSpectrum` single-spectrum audit. **PRE-ANSWERED (proof condition FAILS — P2-class finding)**

**Proof condition (synthesis line 248 / §11 T1, restated):** the call graph must show *one*
eigenvalue computation feeding all scalar functionals (`spectral_radius`, `slem`, `spectral_gap`,
`algebraic_connectivity`, `graph_energy`, `dominant_period`, `classify_drift`); otherwise a
refactor lands making the functionals post-processing of `GraphSpectrum`, all numeric tests green.

**Confirmed locations & preliminary read — each functional recomputes:**
- `spectral.rs:566` `spectral_radius` → calls `eigenvalues(a)`.
- `spectral.rs:575` `slem` → calls `eigenvalues(a)`.
- `spectral.rs:588` `spectral_gap` → calls `slem(a)` (transitively a fresh pass).
- `spectral.rs:604` `graph_energy` → calls `eigenvalues(adj)`.
- `spectral.rs:660` `algebraic_connectivity` → `laplacian(adj)` + `eigenvalues(&l)`, own pass.
- `spectral.rs:704` `classify_drift` → calls `spectral_radius(a)` → fresh pass.
- `spectral.rs:769` `dominant_period` → calls `eigenvalues(a)`, own pass.
- `spectral.rs:612/621` `GraphSpectrum` / `graph_spectrum` — the one-shot profile EXISTS, but
  (i) the standalone functionals do **not** derive from it, and (ii) it is itself impure: after
  computing `eigenvalues(adj)` + `eigenvalues(&laplacian)` it calls `classify_drift(adj)` (`:644`),
  which recomputes `eigenvalues(adj)` a third time — its own doc-comment "Single eigenvalue pass;
  all downstream quantities derived from the spectrum" (`:611`) is **false as written**. Likewise
  `graph_energy_report` (`:751`) makes 4 independent eigen passes while claiming "single pass over
  the spectrum via the reused helpers".

**Verdict:** each-recomputes — the proof condition's first branch fails. Mitigation: all passes go
through the ONE `eigenvalues` entry point (item 15), so this is O(n³)-recompute waste + two false
doc-claims, not a dual-solver correctness risk. **This is the real P2-class finding of the cluster
→ file as a defect doc** (see Handoff), with the fix sketch already dictated by the synthesis:
functionals become post-processing of one `graph_spectrum` pass; `classify_drift` gains a
spectrum-taking inner form. Executor: confirm the seven call sites (grep `eigenvalues(` in
`spectral.rs` bodies), write the defect doc; the refactor itself is NOT Tier 0.

---

## Item 17 — `engine` thick/thin classification table. **Scoped; RC-4 traced; 2 of 3 mirrors now pinned**

**Proof condition (synthesis line 249, restated):** every public item of `engine` classified
boundary-vs-computation in a table; each computation item gets a move-to-kernel ticket OR a stated
forcing reason + P2 parity pin. RC-4's three mirrored items are the first three rows.

**What RC-4 actually is (traced, not guessed):** RC-4 = root cause 4 of the 2026-07-16 hermetic
audit, "**Unpinned mirrors at the kernel↔engine seam**" —
`docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md:238` (§2 RC-1…RC-4),
closure blueprint `BLUEPRINT-H2-mirror-pin-sweep.md` ("RC-4 closure", landed in commit `4dec04218`).
Its ★RC-4 rows there are findings **#10** (dt), **#18** (`TG_MIN_GAP_S`, tools/ not engine), **#23**
(`DriftClass`). The *synthesis* (§13, line 237) names the engine-relevant triple for THIS table:
**`DriftClass` / `dt` / L-operator** — #18 is out of engine scope, replaced by the damped-wave
L-operator. Use the synthesis triple as rows 1–3.

**Current state of the three rows (verified live):**
1. **`DriftClass` mirror** — still re-declared at `engine/src/bridge.rs:683` (`drift_from_code`
   `:706`), but now PINNED: kernel authority `DriftClass::wire_code()` (`kernel/src/spectral.rs:691`,
   pin test `:796`) + engine round-trip test `bridge.rs:820-830`. Classification: computation
   mirror, forcing reason stated (f32 wire), parity-pinned. CLOSED row.
2. **`dt` mirror** — `engine/src/field_frame.rs:62` now binds directly:
   `dt: dowiz_kernel::DT_STABLE as f64` (the 0.016 default is gone); `engine/src/loop_.rs:19`
   `DT_STABLE: f32 = 0.02` remains a hand mirror but carries the mirror-pin test `loop_.rs:157-165`
   (matching kernel pin `kernel/src/lib.rs:377-379`). CLOSED row (pin, not unification).
3. **L-operator** — `engine/src/field_frame.rs:10-11` damped-wave semi-implicit scheme with an
   engine-side 5-point Neumann Laplacian: still engine-side COMPUTATION, unpinned to the kernel's
   `csr.rs:552 laplacian_spmv` / `spectral_laplacian.rs` / `field_eigenmodes.rs` (which engine
   already consumes elsewhere — `engine/src/lib.rs:18` `field_modal`). **OPEN row** — overlaps
   roadmap item 18's Laplacian parity pin; the table should cross-reference, not duplicate, item 18.

**Table scope:** `engine/src/lib.rs` is 84 lines; public surface = pub mods `:17-63` (~14) + `pub
use` groups `:66-84` (~30 items). Executor: one row per `pub use` item + one per pub mod without
re-exports; classify boundary (I/O topology: GPU sink, zerocopy, text input, a11y) vs computation
(`money_guard::interpolate`, `motion::heat_kernel_delay`/`Spring`, `sdf`, `field_frame`, `friction`
FSM…); rows 1–3 as above, two already closed with their pins cited.

---

## Item 19 — retrieval spectral-routing audit. **PRE-ANSWERED (independent by design — with a comment-bound mirror finding)**

**Proof condition (synthesis line 292, restated):** read `retrieval/diffusion.rs` + `retrieval/ppr.rs`
bodies for routing into `spectral.rs`/`GraphSpectrum` vs independent implementation — item 16's
question asked of its likeliest second consumer. Outcome: shared backend cited file:line, OR a P2
defect + collapse-or-parity-pin ticket; plus the §15(b) broad pagerank-grep resolved into a
confirmed per-file list.

**Confirmed locations & preliminary read:**
- `kernel/src/retrieval/ppr.rs` (152 lines) and `retrieval/diffusion.rs` (327 lines) contain
  **zero** `use crate::spectral` / `GraphSpectrum` / `eigh` / `topk` references (grep confirmed).
- Independence is *explicit and reasoned*: `ppr.rs:1-16` — "No eigendecomposition — pure power
  iteration"; fixed K, fixed i-outer/j-inner summation order, bitwise-deterministic. Legitimate
  forcing reason: PPR = (I−αW)⁻¹-style iteration, O(K·n²) dense here, vs O(n³) full spectrum;
  determinism is load-bearing.
- **The finding:** `ppr.rs:3-5` — "Reuses the EXACT deterministic accumulation order of
  `kernel/src/markov.rs`'s damped-PageRank kernel… We never touch `markov.rs`; we **mirror** its
  proven bitwise-reproducible left-product." That is a **comment-bound mirror of `markov.rs`'s inner
  loop** — the RC-4 smell, intra-kernel this time. No pin test referencing `markov` exists in
  `retrieval/tests.rs` (grep negative; executor: confirm across `retrieval/` and `markov.rs` tests).

**Verdict:** not the "collapse to GraphSpectrum" branch — the forcing reason stands and §15(d)'s
hoped-for "second spectral consumer" is answered NO (it is a *markov.rs* consumer-by-mirror
instead). The correct ticket per the proof condition is a **parity-pin test** (ppr inner loop vs
`markov.rs` damped step, bit-exact on a fixture) — H2-row format, cite RC-4 as precedent.
Remaining executor legwork: the §15(b) per-file pagerank-grep list (`arena.rs`, `slot_arena.rs`,
`event_log.rs`, `hydra.rs`, `evals.rs`, `csr.rs`, `retrieval/*`) — classify each hit real-PPR vs
vocabulary-only.

---

## Handoff (per `docs/design/CORE-ROADMAP-INDEX.md` "every planning doc gets a row")

1. **Register this blueprint** as a row in `CORE-ROADMAP-INDEX.md` (cross-track, Tier-0 audit
   cluster, items 15/16/17/19).
2. **Item 16 → defect doc** (P2-class, confirmed): functionals recompute the spectrum
   independently; `graph_spectrum`'s and `graph_energy_report`'s "single pass" doc-claims are false.
   File as `DEFECT-P2-spectral-functional-recompute-2026-07-19.md` (own row), not a passing note.
3. **Item 15 →** vector-scope parity test addition (small) or a P2 defect doc if deferred —
   `r3_topk_symmetric_parity_p3` is values+dominant-residual only (`spectral.rs:1238,1255`).
4. **Item 19 →** parity-pin ticket for the comment-bound `ppr.rs`↔`markov.rs` accumulation-order
   mirror (RC-4 precedent format). Not a collapse.
5. **Item 17 →** table doc; row 3 (L-operator) cross-references roadmap item 18 rather than
   duplicating its parity pin.
