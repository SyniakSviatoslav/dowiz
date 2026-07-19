# BLUEPRINT P80 — Kernel bench expansion (money tripwire · PQ lane · spectral · ppr/absorbing · contended locks) (2026-07-19)

> **Standalone COVERAGE blueprint (dowiz `kernel`).** One coherent, independently-buildable unit
> against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Planning document —
> writes ZERO product code, touches no branches, pushes nothing. This is a **bench-coverage +
> observability-baseline** unit: it adds benchmarks and fixes two bench doc-errors; it changes **no
> product algorithm** (the one apparent exception, `money.rs`, is a *binding no-change* per synthesis
> §2). Research sources: `docs/research/OPUS-PERF-BENCH-COVERAGE-MAP-2026-07-18.md` (R5, the both-repo
> coverage map), `OPUS-PERF-PPR-ANALYSIS` (R1 §3a/§4), `OPUS-PERF-ABSORBING-MARKOV-ANALYSIS` (R2),
> `OPUS-PERF-KERNEL-AUDIT` (R3 A1–A3), reconciled in `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md`
> §3.3-C1 + §2 + §5. Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
> Grounding tree read live this pass: `/root/dowiz/kernel` at HEAD.
>
> **One sentence:** give the kernel's currently-unbenched hot lanes their first criterion baselines —
> the PQ signing/KEM/hash lane, mesh chain-verify, the spectral/matmul/Kalman surface, retrieval/geo,
> the ppr and absorbing sweeps (with two bench doc-error fixes), and the money-ledger growth-tripwire
> — **all written into P75's fixed `<group>/<n>` schema**, with the contended-lock benches
> **cross-referenced from P90 (already landed on a branch), not re-specified**.

---

## VERDICT (stated up front)

**GO — but HARD-gated on P75, and it must NOT re-specify the contended-lock benches P90 already
built.** This is the largest and most mechanical unit of the perf pass (synthesis §1 tier C: "a large
but mechanical bench-coverage expansion"). Three honesty constraints are binding and load-bearing:

1. **No algorithmic change anywhere.** In particular `money.rs` gets a bench **only** — a growth-
   tripwire, never a "fix" (§2, binding per synthesis §2 reconciliation of R3 vs R5).
2. **The contended Mutex benches for `token_bucket`/`budget`/`admission` already exist** on branch
   `perf/contention-bench-2026-07-18` (P90). P80 **cross-references** them; it re-specifies them only
   if the P90 merge ruling (OD-2) says that branch will **not** merge. This is the ledger's stated
   "P80 after P90 merge ruling" sequencing (ledger §3 Wave 2; §5 OD-2).
3. **Benches are BENCH-FIRST evidence, not a mandate to rewrite.** Adding a contended-lock bench does
   NOT authorize a CAS rewrite — that is a separate, evidence-gated future blueprint (E12; standing
   rule `.claude/CLAUDE.md:182-195`).

---

## 0. Ground truth — every bench target re-verified live this pass (standard §2 item 1)

### 0.1 The existing kernel bench harness (what P80 extends, never rebuilds)

`kernel/benches/criterion.rs` (`harness = false`, wired at `kernel/Cargo.toml:146-148`) today has 11
groups: `place_order`, `fold_transitions`, `empirical_identify`, `token_bucket`,
`spectral_cache/{slem_cached,canonical_address}`, `graph_rebuild_rank/{heap,arena}`, `ppr`,
`absorbing`, `retrieval/recall_at_k`, `attention/matmul` (`criterion.rs:16-249`, `criterion_group!` at
`:251`). P80 **adds groups to this file** (and feature-gated sibling(s) for the `pq` lane); it does
not stand up a new harness. Trend storage: `kernel/benches/BENCH_HISTORY.md` is **git-ignored today**
(R7) — P75 fixes that; P80's baselines land in whatever committed store P75 defines.

### 0.2 The unbenched lanes (verified present, verified uncovered)

| Lane | Real target(s), live cite | Coverage today |
|---|---|---|
| **PQ signing/KEM/hash** (feature `pq`, `Cargo.toml:56`) | `pq/dsa.rs:996` `sign`, `:1003` `verify`, `:23` `shake256`; `pq/kem.rs:285` `encaps_internal`, `:332` `decaps_internal`; `pq/hybrid.rs:92` `hybrid_decaps`; `pq/keccak.rs:139` `shake256`, `:145` `shake256_xof` | **ZERO benches** — the entire kernel PQ lane is unmeasured (R5) |
| **mesh chain-verify** | `mesh.rs:225` `verify_chain`, `:132` `verify_sig` | unbenched |
| **spectral/matmul/Kalman** | `spectral.rs:225` `eigenvalues`, `:679` `classify_drift`, `:620` `laplacian`; `spectral_laplacian.rs:83` `laplacian_eigenmodes`; `mat.rs:132` `matmul_contig`; `kalman.rs:200` `predict`, `:212` `update` | only `spectral_cache/*` cached-path benched; the compute surface is not |
| **retrieval/geo** | `csr.rs:632` `recall_at_k`, `living_knowledge.rs:134` `recall_at_k`, `geo.rs:70` `progress_along_route`, `harmonic.rs:26` `harmonic_centrality` | only `retrieval/recall_at_k` fusion benched; bm25-isolate/geo/centrality not |
| **money ledger** | `money.rs:230` `ledger_sum` (inner scan `:238` `ledger.iter().any(\|r\| r.reverses == Some(e.id))`), caller `domain.rs:115` `ledger_balance` | unbenched (see §2 — bench-only, no fix) |

### 0.3 The two bench doc-errors to fix (R2, verified in the live bench file)

`kernel/benches/criterion.rs` `bench_absorbing` doc comment (`:212-214`) reads verbatim: *"Blind-spot
coverage: absorbing Markov fundamental matrix is O(n^3) — used by agentic decision gating."* Both
clauses are wrong (R2):
- **O(n^3) → O(n^4)** — the true cost of the fundamental-matrix inverse path (R2/synthesis §3.3-C1).
- **"used by agentic decision gating" is FALSE** — `absorbing.rs` has **zero production callers**; it
  is an order-lifecycle FSM primitive with n fixed at 5 (R2; synthesis §6 E6). Delete the clause.
- **Relabel** `absorbing/fundamental_matrix_16` (`criterion.rs:225`) → `..._cyclic_16` (the current
  bench measures the pessimal **cyclic** path, not the real DAG path — R2).

### 0.4 The contended-lock benches ALREADY EXIST on a branch (do NOT re-author)

Per `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (I3/Contention row) + §0: contended benches + budget CAS +
token_bucket clock-hoist are **DONE on local branch `perf/contention-bench-2026-07-18`** (commits
`8c865805b`, `8256dbffb`; worktree `/root/dowiz-perf-contention`; 637 kernel tests green on-branch;
`OPUS-PERF-CONTENTION-BENCH-RESULTS` doc is branch-only). These are **registered by P90**, not P80.
P80 must cite this and add nothing here **unless** OD-2 rules the branch will not merge (§8).

---

## 1. Prior-art / reuse map — adopt, don't invent (standard §2 item 19)

| Need | In-tree pattern | Cite |
|---|---|---|
| criterion group with sweep | existing `bench_*` fns + `criterion_group!` | `kernel/benches/criterion.rs:16-251` |
| allocation-count / heap-vs-arena bench | `graph_rebuild_rank/{heap,arena}` | `criterion.rs:161-194` |
| feature-gated bench (pq behind `pq`) | `pq` feature deps | `kernel/Cargo.toml:56`; PQ KATs in `kernel/src/pq/kat` |
| growth-tripwire bench + written revisit threshold (the ppr/absorbing/money treatment) | R1's ppr sweep + doc-comment threshold | R1 §3a/§4; synthesis §2 (money), §6 E7 (ppr) |
| iai-callgrind instruction-count lane for ns-scale benches | prescribed by P75 (R7 §6.3) | P75 (schema owner) |

**No new dependency, no new harness.** P80 extends `criterion.rs` and adds feature-gated sibling(s);
the only new *machinery* is P75's (schema/gate), which P80 consumes, not redefines (standard §2 item 19).

---

## 2. The money.rs binding no-change (synthesis §2 — reproduced because it governs a whole group)

**BINDING (synthesis §2, reconciling R3-C4 vs R5-§2A):** `ledger_sum` (`money.rs:230-245`) is
genuinely O(n²) — for each non-reversal `Earn` it runs `ledger.iter().any(|r| r.reverses ==
Some(e.id))` (`:238`), a full scan per entry. **But real-world `n ≤ 2` by construction**: the only
non-test caller is `Order::ledger_balance` (`domain.rs:115`) over a strictly per-order ledger; the only
production writers post ONE `Earn` + at most ONE `Reversal`; `ledger_append` enforces
at-most-one-reversal-per-earn + duplicate-id rejection (`money.rs:215`). The linear scans **are** the
fail-closed conservation/idempotency probes on money-authority code.

**Therefore, binding for P80:**
1. **No algorithmic change to `money.rs`.** Do NOT "fix" the O(n²); do NOT restructure the probes.
2. **Ship `money_ledger` as a growth-tripwire** — sweep **n ∈ {2, 8, 64, 256}** (n=2 is the real-shape
   anchor; larger sizes keep the quadratic curve on the record, exactly like the ppr sweep).
3. **Written revisit threshold** in the bench doc-comment (verbatim intent): *revisit `ledger_sum`
   representation only if a future change introduces multi-leg order ledgers (per-item earns, tips,
   fee splits, settlement legs) or any real ledger exceeds ~8 entries.* If that trigger ever fires, the
   fix is an O(n) reversed-id `HashSet` pre-pass inside `ledger_sum` (semantics-identical) — **but not
   before.**

This group is a *tripwire on a red-line surface*, so it carries an explicit RED-LINE baseline pin (no
timing regression on the money path should pass silently — standard §2 item 14).

---

## 3. Predefined bench-groups & constants — named before implementation (standard §2 items 4, 8)

All ids use P75's `<group>/<n>` convention; every sweep states its scaling axis (standard §2 item 8):

```
money_ledger/{2,8,64,256}                 // axis: ledger entries; real shape n=2; RED-LINE baseline pin (§2)
kernel_crypto_pq/dsa_sign                  // feature = "pq"
kernel_crypto_pq/dsa_verify
kernel_crypto_pq/kem_encaps
kernel_crypto_pq/kem_decaps
kernel_crypto_pq/hybrid_decaps
kernel_crypto_pq/shake256/{64,1024,16384}  // axis: input bytes (size-swept)
mesh_verify/chain/{1,8,64,256}             // axis: delegation-chain length
spectral_math/eigenvalues/{8,16,32,48}     // axis: matrix n — STRADDLES the n=32 QR↔Faddeev step
spectral_math/matmul_contig/{8,32,64}      // axis: square dim
spectral_math/kalman_predict               //  (+ kalman_update)
spectral_math/classify_drift/{8,32}
spectral_math/laplacian_spmv/{32,128}
retrieval_geo/bm25_isolate                 // bm25 alone (not the fused recall path already benched)
retrieval_geo/progress_along_route/{16,256}// axis: polyline vertices
retrieval_geo/harmonic_centrality/{16,64}  // axis: node count
ppr/rank/{32,128,256}                      // axis: graph nodes @ α per R1 §4 (see §4.5 α note); + revisit threshold
absorbing/fundamental_matrix_cyclic_{16}   // RELABELED from fundamental_matrix_16 (§0.3)
absorbing/lifecycle_5                       // the REAL DAG path (n=5), not the pessimal cyclic one
absorbing/dag_chain/{8,32}                  // DAG sweep
// contended locks: token_bucket/budget/admission — SEE §8; cross-referenced from P90, NOT defined here
```

Constants: sweep endpoints are the named values above; the money revisit threshold and ppr revisit
threshold (~256-node / ~50µs, R1 §3a) are **doc-comment constants** in their bench, not magic numbers.

---

## 4. Build items — spec → measured baseline (standard §2 items 2, 3, 10)

Each group: write the bench, run it, commit the baseline into P75's trend store. Because these are
*coverage* benches (not behavior changes), the "RED→GREEN" discipline manifests as: the bench must
**exist and produce a stable baseline**, and (for the tripwire/straddle groups) an **assertion or
threshold** that goes RED on a future regression.

### 4.1 `money_ledger` growth-tripwire — §2 binding
Sweep {2,8,64,256} over `ledger_sum` (`money.rs:230`). Doc-comment carries the revisit threshold (§2).
RED-LINE baseline pin: a companion assertion (or P75 per-bench threshold) fails CI if the n=2 real-shape
timing regresses. **No code change to `money.rs`.**

### 4.2 `kernel_crypto_pq` lane (feature `pq`)
Bench `dsa::sign`/`verify` (`pq/dsa.rs:996,1003`), `kem::encaps_internal`/`decaps_internal`
(`pq/kem.rs:285,332`), `hybrid_decaps` (`pq/hybrid.rs:92`), `shake256` size-swept (`pq/dsa.rs:23` /
`pq/keccak.rs:139`). Feature-gated sibling bench so the default build stays PQ-free. Seed inputs from
the existing `pq/kat` fixtures (deterministic). These are the ns-to-µs benches P75 routes to
iai-callgrind where wall-clock is too noisy (R7 §6.3).

### 4.3 `mesh_verify/chain` sweep {1,8,64,256}
Bench `mesh::verify_chain` (`mesh.rs:225`) over synthetic delegation chains of increasing length; the
axis is chain length (the per-link Ed25519 verify cost). `verify_sig` (`:132`) as the single-link anchor.

### 4.4 `spectral_math` group (straddle n=32)
`eigenvalues` (`spectral.rs:225`) swept {8,16,32,48} to bracket the QR↔Faddeev dispatch step;
`matmul_contig` (`mat.rs:132`); `kalman::predict`/`update` (`kalman.rs:200,212`); `classify_drift`
(`spectral.rs:679`); `laplacian`/`laplacian_eigenmodes` spmv (`spectral.rs:620`, `spectral_laplacian.rs:83`).
**Coordinate the `eigh_flat` slice with P79 B6** (shared group id under P75's schema — do not duplicate).

### 4.5 `retrieval_geo` group
bm25 in isolation (separated from the already-benched fused `retrieval/recall_at_k`);
`progress_along_route` (`geo.rs:70`); `harmonic_centrality` (`harmonic.rs:26`).

### 4.6 `ppr/rank` sweep + revisit threshold (R1 §4 "as written")
Extend the existing `bench_ppr` (`criterion.rs:196-213`) from the single `rank_32x32_k20` to a sweep
{32,128,256}. **Code R1 §4's sweep exactly as written**, and add its written revisit threshold
(~256-node / ~50µs, R1 §3a) as a doc-comment constant — the tripwire for the deferred dense→sparse
ppr migration (Tier D-1). **α NOTE (ground-truth flag):** the existing bench calls `ppr.rank(0, 0.85,
20)` (`criterion.rs:207`), while synthesis §3.3-C1 writes the sweep "@ α=0.15". These are complementary
conventions (restart-stay `0.85` vs teleport `0.15`). **Use R1 §4's exact parameter as authoritative**
and reconcile against the existing bench's `0.85`; do not silently pick one — confirm from R1 §4's code.

### 4.7 `absorbing` relabel + DAG benches + 2 doc-fixes (§0.3)
Relabel `fundamental_matrix_16` → `_cyclic_16`; add `lifecycle_5` (the real DAG path) and a `dag_chain`
sweep; fix the doc comment (`criterion.rs:212-214`): O(n^3)→O(n^4), delete "used by agentic decision
gating". No code change to `absorbing.rs` (E6 — zero production callers, n fixed at 5).

### 4.8 Contended-lock benches — CROSS-REFERENCE ONLY (see §8)
`token_bucket`/`budget`/`admission` contended benches are **P90's landed work** (§0.4). P80 adds a
pointer in the bench file header and does **not** re-author them, unless OD-2 rules against merge (§8).
**BENCH-FIRST caveat:** these benches exist to *measure* contention; they do NOT authorize a
Mutex→CAS rewrite (E12 — that needs a separate evidence-gated blueprint). Also fix the bench comment
that mislabels the Mutex as CAS (R3 A1–A3), if it lands in P80's scope rather than P90's.

---

## 5. Invariants to preserve (standard §2 items 6, 13)

1. **No behavior change (the whole unit's core invariant).** P80 adds benches + fixes bench doc-text;
   it changes no product function. Falsifier: `cargo test --lib` count stays at the current baseline
   (memory: kernel 561 `--test --lib`; 452 default + 107 pq-KAT per Ground-Truth) — a changed test
   outcome means P80 touched product code and is wrong.
2. **money.rs untouched (binding, §2).** Falsifier: `git diff kernel/src/money.rs` is empty (only the
   bench file + revisit-threshold doc-comment change). This is the RED-LINE invariant of this unit.
3. **Default build stays PQ-free.** The `kernel_crypto_pq` group is behind `feature = "pq"`; falsifier:
   default `cargo bench` does not compile the PQ lane; `cargo bench --features pq` does.
4. **Determinism of bench inputs.** All swept inputs use the deterministic constructors/KAT fixtures
   already in-tree (seedable RNG, fixed matrices) — no wall-clock or entropy-seeded input that would
   make a baseline non-reproducible (standard §2 item 10).
5. **Rollback as math (item 13):** a bench that fails to produce a stable baseline is simply not
   committed; there is no runtime state. Self-termination = the coverage unit adds nothing that can
   fail in production (benches don't ship in the product binary).

---

## 6. DoD — falsifiable, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier / check |
|---|---|---|
| D1 | every §3 group exists, runs, and has a committed baseline in P75's trend store | `cargo bench` (+ `--features pq`) lists all group ids; baselines present |
| D2 | `money_ledger` sweep {2,8,64,256} exists with the §2 revisit threshold; `money.rs` unchanged | `git diff kernel/src/money.rs` empty; doc-comment present |
| D3 | absorbing relabeled `_cyclic_16` + `lifecycle_5`/`dag_chain` added; both doc-errors fixed | grep: no `fundamental_matrix_16`; no "agentic decision gating"; "O(n^4)" |
| D4 | ppr sweep {32,128,256} per R1 §4 with revisit threshold; α reconciled to R1's spec | bench ids present; doc-comment threshold present; α matches R1 §4 |
| D5 | PQ lane benched behind `feature="pq"`; default build unaffected | `cargo bench` (default) omits PQ; `--features pq` includes it |
| D6 | contended-lock benches are cross-referenced from P90, not duplicated (unless OD-2 says otherwise) | bench-file header cites `perf/contention-bench-2026-07-18`; no duplicate token_bucket-contended group |
| D7 | no product-behavior change | `cargo test --lib` count == baseline; no product `.rs` diff outside bench file |
| D-SCHEMA | all ids use P75's `<group>/<n>` convention; no schema redefinition | review against P75's schema doc |

---

## 7. Benchmarks are the deliverable — measure-first honesty (standard §2 item 10)

The entire unit *is* the "measured before/after, not estimated" bar. Two honesty rules:

- **Coverage, not speedup.** P80 does not *claim* wins — it *establishes baselines* so future changes
  (P77/P79 fixes, a possible future CAS rewrite) have a real number to move against. The money/ppr/
  absorbing groups are **growth-tripwires** whose purpose is to keep a known quadratic/quartic curve on
  the record with a written revisit threshold, exactly the load-bearing-negative-result discipline of
  the whole perf pass (synthesis §6 E6/E7/E8).
- **ns-scale caveat.** Several benches (PQ hash, token_bucket) are sub-100ns and were proven ungateable
  by wall-clock on this host (±75% swings, R7/PERF-06). Route those to P75's **iai-callgrind
  instruction-count** lane; do not gate them on wall-clock mean delta.

---

## 8. Rollout / sequencing (consistent with the master ledger) — incl. the P90 dependency

Per `MASTER-STATUS-LEDGER-2026-07-19.md` §3 (Wave 2) and `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §5:

- **Lane:** dowiz kernel (parallel to bebop). **Wave:** **W2 bench-coverage**, build-parallel with
  **P81** (engine benches) and **P83** (observability). Ledger §3 Wave 2: `P80 ∥ P81 ∥ P83`.
- **HARD dependency — P75.** P80 must land **after P75** so its ~40 new baselines are written into the
  fixed `<group>/<n>` schema, not the broken one (synthesis §1 sequencing insight; §5 "P75 hard").
- **The P90 merge ruling (OD-2) — explicit, do not skip.** The contended-lock benches for
  `token_bucket`/`budget`/`admission` (synthesis §3.3-C1's last sub-item) are **already implemented and
  green** on local branch `perf/contention-bench-2026-07-18` (commits `8c865805b`, `8256dbffb`;
  worktree `/root/dowiz-perf-contention`), registered by **P90**. **P80's handling depends on the P90
  merge ruling (ledger §5 OD-2 — "Push/merge `perf/contention-bench-2026-07-18` to remote/main"):**
  - **If OD-2 = merge:** P80 **cross-references** P90's benches (a pointer in the bench-file header),
    adds nothing for the contended locks, and only ensures the `<group>/<n>` ids reconcile with P75's
    schema. This is the default expectation (ledger §3: "after … P90 merge ruling so contended benches
    aren't re-specified").
  - **If OD-2 = stays-local / won't-merge:** P80 **absorbs** the contended-lock benches from the branch
    (copy the bench definitions from `OPUS-PERF-CONTENTION-BENCH-RESULTS` / the branch worktree), so the
    coverage is not lost. Either way, **no CAS rewrite** (E12 — evidence-gated separate blueprint).
  - **Default if unruled** (ledger §5 OD-2): the branch stays local — so P80 should be written to
    *absorb-if-still-local at build time*, but the blueprint's stated preference is the merge path.
- **Coordinations:** the `spectral_math/eigh_flat` slice is shared with **P79 B6** (do not duplicate);
  the `<group>/<n>` convention and gate semantics are **P75's** single-owner contract (P80 cites).
- **Push:** dowiz `origin/main` is behind by a whole unpushed local main line (ledger §0; OD-4) — P80's
  commit lands on top of that line, consistent with the existing local main.

---

## 9. Open operator-decision points

| # | Decision | Owner | Effect on P80 | Source |
|---|---|---|---|---|
| OD-2 | Push/merge `perf/contention-bench-2026-07-18` (P90 contended benches) | operator | **Directly gates §8** — merge ⇒ P80 cross-references; won't-merge ⇒ P80 absorbs the branch benches | ledger §5 OD-2 |
| OD-1 | GCRA lock-free swap on `token_bucket` (3.6× @8t benched) | operator | Out of P80 scope (P90/E12) — recorded so no one adds a CAS bench-and-rewrite here | ledger §5 OD-1 |
| OD-10 | PPR determinism relaxation — standing default REJECTED | operator | None — the ppr **sweep** (§4.6) is the tripwire; approximate PPR stays rejected (E5/D-4) | ledger §5 OD-10; synthesis §4 D-4 |

Engineering decisions P80 makes itself (operator need not): exact iai-callgrind vs wall-clock split per
bench (defer to P75's per-bench threshold policy); the ppr α reconciliation against R1 §4 (§4.5);
feature-gated sibling file name for the PQ lane. **No new money/RLS/auth code decision** — P80 changes
no product algorithm (§2, §5).

---

## 10. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Cause & Effect / measurement-before-judgment:** a bench is the *cause-and-effect* instrument — it
  makes the cost of a change visible so future decisions follow evidence, not intuition (the whole
  BENCH-FIRST doctrine, `.claude/CLAUDE.md:182-195`). P80 refuses to rewrite on intuition (money, E12).
- **Polarity / no-middle:** a growth-tripwire encodes a hard boundary — below the written threshold the
  known-quadratic curve is *accepted* (no action); above it, a *revisit* is mandated. There is no vague
  middle "maybe optimize"; the threshold is a doc-comment constant (§2, §4.6).
- **Correspondence:** the bench baseline *corresponds* to the real hot-path cost — the straddle sweeps
  (n=32 QR↔Faddeev; money n=2 real-shape) are chosen so the measured number corresponds to the actual
  production shape, not a convenient synthetic one.

---

## 11. Standard-compliance map (all 20 points — standard §2)

| # | Item | Where |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (every bench target + the 2 doc-errors verified live) |
| 2 | Falsifiable DoD | §6 |
| 3 | Spec→test→code, event-driven | §4 (each group spec'd; tripwires assert on future regression) |
| 4 | Predefined types & constants | §3 (all group ids + threshold constants named) |
| 5 | Adversarial/breaking tests | §4.1/§4.7 (tripwire assertions that go RED on regression), §5 (money-diff-empty falsifier) |
| 6 | Hazard-safety from structure | §5 (no-behavior-change + money-untouched as machine-checked invariants) |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §3 (every sweep states its axis: entries/chain-len/n/bytes/nodes) |
| 9 | Linux engineering discipline | EXTENDS the criterion harness; REINFORCES BENCH-FIRST; DOES-NOT-TRANSFER (no rewrite) — §1 |
| 10 | Benchmarks + telemetry + measure-first | the entire unit; §7 (coverage-not-speedup; iai for ns-scale) |
| 11 | Isolation / bulkhead | §0.1 (adds to one bench file + feature-gated sibling; PQ behind `pq`; no product code) |
| 12 | Mesh awareness | §4.3 (`mesh_verify/chain` sweep = the mesh chain-verify cost budget) |
| 13 | Rollback/self-heal as math | §5.5 (benches don't ship; a bad baseline is simply not committed) |
| 14 | Error-propagation / smart index | §4.1 money RED-LINE pin; §7 (P75 CI gate catches timing regressions) |
| 15 | Living-memory awareness | §4.5 (`retrieval_geo` covers the living-knowledge read path — recall/centrality) |
| 16 | Tensor/spectral where applicable | §4.4 (`spectral_math` straddle sweep; coordinates with P79 B6) |
| 17 | Regression tracking | §6 (tripwire assertions permanent; baselines in P75 trend store; REGRESSION-LEDGER for the 2 doc-fixes) |
| 18 | Clear worker instructions | §13 |
| 19 | Reuse-first | §1 (extends `criterion.rs`; no new harness/dep; P75 owns the schema) |
| 20 | Hermetic principles | §10 |

---

## 12. Links to docs & memory (standard §2 item 7)

- `docs/research/OPUS-PERF-BENCH-COVERAGE-MAP-2026-07-18.md` (R5) — the both-repo coverage map / bench
  wave proposal; the authority for the exact PQ/mesh/spectral/geo function list.
- `docs/research/OPUS-PERF-PPR-ANALYSIS-2026-07-18.md` (R1) §3a/§4 — the ppr sweep code + revisit
  threshold to code "as written" (§4.6).
- `docs/research/OPUS-PERF-ABSORBING-MARKOV-ANALYSIS-2026-07-18.md` (R2) — the 2 doc-error fixes +
  cyclic/DAG relabel; zero-production-callers finding (E6).
- `docs/research/OPUS-PERF-KERNEL-AUDIT-2026-07-18.md` (R3) A1–A3 — the contended-lock BENCH-FIRST
  finding + the CAS-mislabel bench comment.
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C1 (the C1 bench list), §2 (money binding no-change),
  §5 (Wave W2, "P75 hard" + "after P90 merge ruling"), §6 (E5 approximate-PPR, E6 absorbing, E8 money,
  E12 contended-lock-CAS all rejected/gated).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P80 row: "cross-reference P90, not re-specify"; I3/Contention
  row), §3 (Wave 2 sequencing), §5 (OD-1/OD-2/OD-10), §0 (unpushed main line / branch-only doc).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract). `.claude/CLAUDE.md:182-195` (the
  Performance Standing Rule — BENCH-FIRST, no blanket rewrite).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `performance-priority-over-minimal-change-2026-07-17.md`; the P90/contention worktree
  `/root/dowiz-perf-contention` (branch `perf/contention-bench-2026-07-18`, `8c865805b`/`8256dbffb`).

## 13. Instructions for the executing worker (zero prior context — standard §2 item 18)

**Repo:** `/root/dowiz/kernel`. **Blocked until P75 lands** (schema + working gate) — write now, land
baselines into P75's schema. **Check OD-2 (P90 merge ruling) before authoring the contended-lock
benches** (§8).

1. Extend `kernel/benches/criterion.rs` with the §3 groups; add a **feature-gated sibling** for the
   `kernel_crypto_pq` lane (behind `feature = "pq"`), seeding from `kernel/src/pq/kat` fixtures.
2. **money_ledger** (§4.1/§2): bench-only, sweep {2,8,64,256}, revisit-threshold doc-comment. **DO NOT
   edit `money.rs`** — verify `git diff kernel/src/money.rs` is empty at the end (D2).
3. **absorbing** (§4.7): relabel `fundamental_matrix_16`→`_cyclic_16`; add `lifecycle_5` + `dag_chain`;
   fix the doc comment (O(n^3)→O(n^4); delete "used by agentic decision gating"). No `absorbing.rs` edit.
4. **ppr** (§4.6): sweep {32,128,256} coding **R1 §4 as written**; add the revisit-threshold comment;
   **reconcile α against R1 §4** (existing bench uses `0.85` at `criterion.rs:207`) — confirm, don't guess.
5. **Contended locks** (§8): read OD-2. If merge → add a header pointer to P90's branch, author nothing.
   If won't-merge → absorb the branch's bench definitions. **Never** add a CAS rewrite (E12).
6. Coordinate `spectral_math/eigh_flat` with **P79 B6** (shared id, do not duplicate).
7. Register every id under **P75's `<group>/<n>` schema**; commit baselines into P75's trend store.
   Route ns-scale benches (PQ hash, token_bucket) to P75's iai-callgrind lane (§7).
8. **Verify no product-behavior change:** `cargo test --lib` count unchanged; no product `.rs` diff
   outside the bench file(s) (D7). Add REGRESSION-LEDGER entries for the 2 absorbing doc-fixes.
