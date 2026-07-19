# BLUEPRINT — Item 7: Verification Wiring for Keccak · FSM Graph · NTT Arithmetic · GCRA (rescoped: native-exhaustive-first, Kani narrow)

- **Date:** 2026-07-19 · **Tier:** 2 (roadmap §C) · **Status:** BLUEPRINT v2 (planning artifact, no
  code) — **v2 rescopes v1 per the same-day feasibility research** (see below); v1's target
  inventory and per-property designs are retained, their *tooling assignment* changed.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §C Item 7 +
  §0 GCRA ADOPT ruling; `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §7 (Kani), §9.7
  (proof line), §10/P7, and the "Does NOT count" CI-time-tool rule (synthesis line ~21);
  `BLUEPRINT-ITEM-06-hardening-checklist-ci-2026-07-19.md` §2.5 + §5.6 (deferrals TO this item);
  **`RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md`** (per-target Bucket-A/B/C
  classification under the item-25 dependency-replacement procedure — the rescoping authority);
  worktree `/root/dowiz-wt-space-grade-exec` @ `c64ca923b` (branch
  `exec/space-grade-tier0-2026-07-19`) — ground truth for all code citations.
- **Upstream:** item 6 (SHIPPED @ `ae4964e61`: `docs/audits/hardening/{CHECKLIST.md,HOT-PATHS.tsv}`,
  `scripts/hardening-gate.sh`, `kernel/src/ct_gate.rs`, ci.yml job `hardening-gate` @ ci.yml:488).
- **Downstream:** item 8 (GCRA swap) — carries the GCRA obligations specified in §5.

---

## 0. THE RESCOPING — read this first

The feasibility research applied the binding item-25 dependency-replacement procedure to Kani
itself (operator directive: research native reimplementation like for every other dependency) and
classified all 22 harnesses this blueprint's v1 specified. Its findings, verified against the code
cited in §2:

- **16 / 22 targets need zero Kani.** Their input spaces are exhaustively enumerable (144 FSM
  pairs; `caddq` ~1.67e7; rounding [0,Q) ≈ 8.38e6; KEM pairs [0,3329)² ≈ 1.1e7; `reduce32` full
  i32 = 4.3e9, minutes-scale), or their control flow is input-independent (Keccak — no
  data-dependent branch anywhere in the permutation, so one execution exercises every path), or
  they follow from a proven part by a one-line compositional argument (`fold_transitions`). The
  codebase's **already-established exhaustive idiom** (`csr.rs:1296`
  `laplacian_dense_vs_spmv_parity_exhaustive_small` — 1099 bit-enumerated graphs;
  `order_machine.rs` `for i in 0..12` sweeps) delivers the **identical all-inputs guarantee** as
  plain `#[test]` functions, with zero new tooling and no Kani toolchain in the loop.
- **6 / 22 genuinely warrant Kani** (non-exhaustible space + interval/algebraic property where a
  machine-checked proof beats a hand lemma): `montgomery_reduce`, full `ntt`, full `invntt`, the
  Keccak cross-copy equivalence, and both GCRA proofs (already sequenced to item 8 in v1 §5).
- **0 / 22 need a hand-rolled SAT solver.** "Reimplement Kani natively" is the wrong question —
  a bounded model checker has no small slice to carve (unlike regex/tracing/serde_json/sha2, all
  linked runtime libs); Kani is CI-time tooling in the same class as rustc/cargo (procedure
  terminal state (c), legitimate boundary). The right move is **de-scoping which targets use Kani
  at all**, not replacing its engine.

**Consequences for this blueprint:**
1. §3 splits into **Bucket B (native exhaustive tests — 16 targets, plain `#[test]`, the primary
   deliverable)** and **Bucket C (Kani harnesses — 4 now + 2 deferred to item 8)**.
2. Bucket B rides item 6's **existing** `hardening-gate` lib-mode machinery (bump `min_tests` on
   existing manifest rows) — no new CI job needed for 16/22 of item 7's value.
3. The `kani-gate` job (§6) shrinks to ~4 harnesses + the planted-fault self-test, and **item 7 no
   longer gates on the Kani toolchain bootstrapping**: if `cargo kani setup` fails/crawls in the
   sandboxed CI environment, the 16 native targets still land whole and honest, and the 4
   Kani-dependent targets are reported deferred with the toolchain issue named — a first-class
   outcome, not a failure mode.

Everything below is written to that split. v1's harness *property definitions* are unchanged —
only the engine that checks them moved.

---

## 1. Verified: Kani is not used anywhere yet

Re-verified this session, not taken on citation:

- `grep -ri kani` over `kernel/src/`, `kernel/tests/`, `kernel/benches/`, `kernel/Cargo.toml`,
  `kernel/Cargo.lock`, and all `.rs/.toml/.yml/.lock` in both `/root/dowiz` and the worktree
  (excluding `target/`): **zero hits**. No `#[kani::proof]`, no `cfg(kani)` in source, no
  toolchain/workflow wiring.
- The only `cfg(kani)` tokens on disk are in
  `kernel/target/{debug,release}/build/zerocopy-*/output` — the **`zerocopy` dev-dependency's
  build script** emitting `cargo:rustc-check-cfg=cfg(kani)` (zerocopy carries Kani proofs upstream
  and declares the cfg). `zerocopy` sits in `Cargo.lock` only via the dev-dependency graph; the
  zero-dep gate measures `cargo tree -e no-dev`, which is at 0 external crates. This confirms the
  earlier research pass exactly.
- `cargo-kani` is not installed on this machine.

**What Kani is, scoped honestly:** a CBMC-backed bounded model checker over Rust MIR. It proves,
for all inputs within stated bounds: no panic, no overflow, no OOB, no assertion violation. It is
**not** full functional correctness — that remains the oracles' job (ACVP per-tcId, FSM golden
signature, differential probes; item 6 §3.1). Post-rescope, Kani's role narrows further: it is
reserved for the four properties whose input spaces cannot be enumerated *and* whose structure
rewards a machine-checked proof.

## 2. Verified target inventory (with the repo ruling)

**Repo ruling — NTT targets dowiz, not bebop.** The synthesis §7 names "the NTT/`red()`
arithmetic": `red()` is `kernel/src/pq/kem.rs:58` and the NTT is `kernel/src/pq/dsa.rs:199` —
**both dowiz kernel code**. Bebop's `pq_kem.rs` NTT (`bebop2/core/`, commit `cf1fc90`) appears in
the synthesis only as the *checklist precedent* (`ntt_ct_gate`, §1.6), and item 6's blueprint §1
already ruled "Item 6's job is the dowiz kernel." This session's bebop NTT wire-in is a different
repo and **out of item 7's scope**.

| Domain | Function(s) | Citation (worktree) | Bucket |
|---|---|---|---|
| Keccak copy A | `keccak_f` (private), `sponge`, `sha3_256`/`shake*` | `kernel/src/pq/keccak.rs:54` (permutation), `:96` (sponge), `:134–194`; `rotl` guard `:44` | B (panic-freedom) |
| Keccak copy B | `keccak_f` **nested inside** `pub fn sha3_256` | `kernel/src/event_log.rs:30` (outer), `:67` (nested) | B (panic-freedom) + C (equivalence) |
| FSM graph | `assert_transition`, `FSM_ADJ`, `reachable`, `topological_order`, `has_cycle`, `cyclomatic_number`, `fold_transitions` | `kernel/src/order_machine.rs:139,208,353,290,590,627,167`; `idx_of:264` | B (all) |
| NTT arithmetic (ML-DSA) | `montgomery_reduce`, `reduce32`, `caddq`, `ntt`, `invntt_tomont`, `power2round`, `decompose`, `poly_chknorm` | `kernel/src/pq/dsa.rs:74,81,87,199,220,247,253,283` (Q=8380417, `ZETAS:164`) | B (reduce32/caddq/rounding) + C (montgomery_reduce, ntt, invntt) |
| Ring arithmetic (ML-KEM) | `red`, `poly_add/sub`, `poly_mul` body, `compress`/`decompress`, `byte_encode`/`byte_decode` | `kernel/src/pq/kem.rs:58,68,77,95,184,190,131,154` (Q=3329) | B (all) |
| GCRA transition | **Does not exist in production** — only bench-local `GcraBucket` | `kernel/benches/contention.rs:225–261` (f64, self-labeled not-a-drop-in); production bucket is mutex `token_bucket.rs` | C, deferred to item 8 (§5) |

SIMD check: none of the six target files touches `core::arch`/`std::arch`/`simd` (verified).
Floats appear only in `token_bucket.rs`/bench `GcraBucket` (§5); `spectral_radius` is the proven
const `0.0` (`order_machine.rs:383`). `pq` is feature-gated (`lib.rs:13–14`) while `event_log` is
default-build — this is *why* the duplicate Keccak exists, and it means (a) pq-domain tests/rows
carry `--features pq`, (b) the Keccak dedup (see §3.3) is a structural change, not a one-liner.

## 3. The verification designs

### 3.1 Bucket B — 16 native exhaustive tests (primary deliverable, zero new tooling)

Plain `#[test]` functions in the target modules' existing `mod tests`, in the established idiom.
**The load-bearing pattern — shadow-widened arithmetic:** an exhaustive test proves
*overflow-freedom* only if overflow is detectable. Do not rely on debug-profile panics alone;
each test recomputes the function's intermediates in a wider type (i64/i128) and asserts each
intermediate fits the narrow type, alongside the output contract. That makes the enumeration a
genuine no-overflow proof under any build profile, in the same spirit as `csr.rs`'s parity
exhaustion.

| Test (native `#[test]`) | Domain enumerated | Property asserted (identical to v1's Kani property) |
|---|---|---|
| `exhaustive_fsm_transition_adj_consistent` | all 144 `(from,to)` pairs | `assert_transition` never panics; `Ok(())` ⟺ bit `idx_of(to)` in `FSM_ADJ[idx_of(from)]` — item 3's cross-check proven for every pair |
| `exhaustive_fsm_reachable_total` | all 12 start states | no panic; every internal index < 12 (holds because no `FSM_ADJ` row sets bits ≥ 12 — a wrongly-added 13th state trips it); result contains `from`'s bit; frontier loop terminates ≤ 13 iterations (assert an iteration counter) |
| `exhaustive_fsm_fold_corollary` | 144 pairs + one ≤13-len walk per pair | `fold_transitions` stops at first illegal pair; panic-freedom for arbitrary sequences follows from the 144-pair total proof + the documented one-line compositional argument (fold applies the pairwise fn) — comment in-test |
| `fsm_const_graph_checked_sweep` | input-free (const graph) | `topological_order`/`has_cycle`/`cyclomatic_number`/`fsm_graph_report` under shadow-checked arithmetic — honest note: near-zero marginal value over the existing 25 tests, included only for the checked-arithmetic sweep |
| `exhaustive_reduce32_contract` | **all 2^32 i32 values** (minutes; `#[ignore]` + release-mode row, dudect-row precedent) | via i64 shadow: `a + (1<<22)` fits i32 ⟺ `a ≤ i32::MAX − 2^22` (the exact overflow boundary, *discovered* by the sweep, then doc-commented on the fn); result ≡ a (mod Q); |result| ≤ 2^22 + (Q−1)/2 on the valid domain |
| `exhaustive_caddq_contract` | −Q < a < Q (~1.67e7) | result ∈ [0, Q), ≡ a (mod Q), no widened-intermediate overflow |
| `exhaustive_rounding_contracts` | a ∈ [0, Q) (~8.38e6) | `power2round`: `a == a1·2^D + a0`, `a0 ∈ (−2^{D−1}, 2^{D−1}]`; `decompose`: FIPS-204 recomposition incl. the GAMMA2 edge; `poly_chknorm` abs-trick overflow-free on the asserted coeff domain |
| `keccak_f_no_datadep_branches` (pq/keccak.rs) | one arbitrary state + the 25 RHO constants | Keccak-f control flow is **input-independent** (all indices compile-time / `%5` of loop vars; only branch is `rotl`'s `n==0` on constants) ⇒ one execution exercises every path; test asserts each RHO < 64 and exercises the `n==0` lane; panic-freedom for all 2^1600 states follows and the argument is written in the test comment |
| `exhaustive_rotl_basis` | 64 shifts × 64 single-bit basis vectors (4096) | `rotl(x,n) == x.rotate_left(n)` — linearity of bit-permutations makes the basis sweep total; ALSO file the ponytail note: replacing `rotl` with stdlib `rotate_left` moots this test |
| `exhaustive_sponge_lengths` (pq/keccak.rs) | `len ∈ 0..=271` × rates {168,136} × pads {0x1f,0x06} (~1084 runs) | padding/absorb/squeeze arithmetic panic-free; **complete** for the panic-freedom property because control flow depends on `len`, never on byte values — strictly stronger than v1's bounded Kani harness, which could not claim completeness |
| `exhaustive_evlog_sha3_lengths` (event_log.rs) | `len ∈ 0..=271` | same over `event_log::sha3_256` (`try_into().unwrap()` at `:116`, padding loop, squeeze) |
| `exhaustive_red_boundary_and_residues` (kem.rs) | i64 boundaries (MIN/MAX/±kQ/±1) + full residue sweeps | result ∈ [0,Q), ≡ x (mod Q). Honest note: universality over all 2^64 is NOT enumerable — it rests on Rust `%` semantics (3-line argument in the test comment) + boundary sweep. The one Bucket-B target where the native form is argument-backed rather than fully enumerated; the loss vs Kani's universal proof is noted, judged acceptable (the property IS the definition of `%`) |
| `exhaustive_poly_addsub_pairs` (kem.rs) | [0,Q)² ≈ 1.1e7 pairs | `a ± b` into `red` widened-overflow-free, result in [0,Q) |
| `exhaustive_poly_mul_body_lemma` (kem.rs) | `(r_val, term)` ∈ [0,Q)² ≈ 1.1e7 — the pair domain factors because `term = (ai·bj) % Q` has only Q values | both branch bodies (`kem.rs:104–121` incl. negacyclic wrap) preserve `r ∈ [0,Q)`, i64-widened overflow-free — the loop invariant the fn's own comment (`:91–94`) claims, machine-enumerated; the 65,536-iteration induction stays a documented argument (unchanged from v1) |
| `exhaustive_compress_decompress` (kem.rs) | [0,Q) × d ∈ {1,4,10,12} (13,316 per d) | panic/overflow-free, in-range outputs |
| `exhaustive_byte_codec_groups` (kem.rs) | per-group decode domains at deployed widths (d=12: 2^24 per 3-byte group; d∈{1,4,10}: smaller) | index arithmetic in bounds (the real OOB class on deserialization); round-trip within domain |

Runtime budget: everything except `exhaustive_reduce32_contract` (~minutes) is sub-second to
seconds. `reduce32` gets its own release-mode manifest row (the dudect-mode `--release -- --ignored`
mechanics are the precedent; executor picks the exact wiring and ledgers it).

### 3.2 Bucket C — 4 Kani harnesses now (the narrow, genuine Kani scope)

`#[cfg(kani)] mod kani_proofs` blocks in the target modules (private-fn access for free; compiled
out of every non-Kani build). Naming: `proof_<target>_<property>`.

| Harness | Property proven | Why Kani and not native |
|---|---|---|
| `proof_montgomery_reduce_contract` (dsa.rs, `pq`) | for symbolic `a: i64`, `assume(−(Q<<31) ≤ a ≤ Q<<31)` (documented precondition `:75`): no overflow; result `−Q < r < Q`; congruence `(r as i128)·2^32 ≡ a (mod Q)` checked in i128 | domain ±1.8e16 — not enumerable; interval + congruence is solver-cheap (straight-line, one multiply) |
| `proof_ntt_no_overflow` (dsa.rs, `pq`) | 256 symbolic i32 coeffs, `assume(|a[i]| < Q)`: across 8 layers / 1024 butterflies, `a[j] ± t` never overflows i32 (lazy-reduction bound < 9Q « 2^31) and every `montgomery_reduce` call meets its precondition (|zeta·a| < 9Q² < Q·2^31) | (2Q)^256 space; monotone interval propagation over a fixed schedule — machine-checked beats hand-argued induction, and a bound-slip here is a real historical bug class |
| `proof_invntt_no_overflow` (dsa.rs, `pq`) | same structure over `invntt_tomont` incl. the final `F·a[j]` Montgomery pass | same |
| `proof_keccak_copies_equivalent` (`pq` feature; **the keystone**) | `s: [u64;25] = kani::any()` → `event_log::keccak_f` and `pq::keccak::keccak_f` produce identical states — equivalence on all 2^1600 inputs | the two copies differ exactly in the ρ+π indexing formulation (`event_log.rs:84–92` dest_x/dest_y vs `keccak.rs:73–79` flat form) — the precise divergence a KAT pair can miss; bitwise circuits are CBMC's cheapest case. Retires the manifest's `keccak-copy-B` drift ledger while two copies exist |

*Honest cost flags carried from v1:* the two full-NTT harnesses are CBMC's known-expensive case
(bit-blasted 32×32 multipliers). Fallback ladder, in order: (i) butterfly-lemma harness (symbolic
`(x, y, zeta, k)`, `|x|,|y| < kQ`, `k ≤ 8`: one butterfly preserves `< (k+1)Q`) + documented
8-layer induction — note this ladder rung **is** the research doc's "native interval propagation"
in Kani clothing, so the delta between rungs is small and honest; (ii) `kani::stub`
`montgomery_reduce` by its separately-proven contract. Equivalence fallback: per-round equivalence
+ documented 24-fold induction. Whichever rung ships is recorded in the manifest gap column.

**Required micro-refactor (only code change to shipped logic, behavior-preserving):** lift
`event_log.rs`'s nested `keccak_f` (`:67`) to a module-level `pub(crate) fn` — today it is nested
inside `sha3_256` and unreachable by any harness or test; `pq/keccak.rs::keccak_f` gains
`pub(crate)`. Existing KATs on both sides (`event_log.rs:609–628`, `keccak.rs:223–269`) pin
behavior across the move.

### 3.3 The dedup alternative for the keystone (recorded, not chosen now)

The research doc's preferred endgame: **delete one Keccak copy** and the equivalence obligation
vanishes. Correct long-term — but it is a structural change (`pq` is feature-gated, `event_log`
is default-build, so keccak must first move out of the `pq` gate into an always-compiled module)
and it swaps event_log's hash primitive in the hash-chain path. That is its own ticket with its
own oracle discipline, already owed per the procedure doc, **not** item 7 scope. Ruling here:
build the equivalence proof now (it guards the interim, cheaply); when the dedup ticket lands,
the harness and the `keccak-copy-B` ledger line retire together.

## 4. Honest limits of the rescoped design

1. **`red()` universality** — the one Bucket-B target whose native form is
   boundary-sweep + language-semantics argument, not full enumeration (2^64). Kani's universal
   proof was free there; the loss is noted and accepted (§3.1 row).
2. **`poly_mul` induction** — unchanged from v1: body lemma is machine-enumerated, the
   65,536-iteration induction stays a documented argument. That is the ceiling at this code shape
   under *either* engine.
3. **Sponge completeness** — improved by the rescope: native length-exhaustion (0..=271, content
   arbitrary) is *complete* for panic-freedom since control flow never depends on byte values;
   v1's bounded Kani harness could not claim that.
4. **Concurrency** — Kani proves sequential code; GCRA CAS-loop linearizability is loom-class.
   Item 8 gets the pure-transition proofs + differential oracle, stated plainly (§5).
5. **Not closed by item 7:** the item-14 spot-check deferred the "exhaustive per-branch taint
   proof" here — but neither native exhaustion nor Kani sees **codegen or timing**. Item 7
   upgrades checklist item 4's *arithmetic* half to deterministic; constant-time stays with the
   dudect gate (`ct_gate.rs`) + the human assembly spot-check on compiler bumps (the sanctioned
   presence-check exception, item 6 §2.5). No claim that item 7 retires the asm spot-check.
6. **Kani-toolchain risk is now non-blocking** — see §6.4: 16/22 targets deliver with zero Kani;
   the 4 Kani targets are reported done-or-honestly-deferred separately.

## 5. GCRA sequencing ruling — spec now, proofs land with item 8 (unchanged from v1)

**Ruling: the GCRA proofs land inside item 8's change, specified here, mechanically obligated via
the manifest.** Production GCRA does not exist (bench-local `GcraBucket` only,
`contention.rs:225`, self-labeled not-a-drop-in); a proof CI cannot run is a presence artifact —
exactly what §10/P7 bans. Both GCRA targets are Bucket C (4×u64 space, algebraic property).

**Design requirements item 7 imposes on item 8** (recorded here + in the manifest gap column):

1. Transition logic as a **pure function**
   `fn gcra_decide(now_ns: u64, tat_ns: u64, cost_ns: u64, burst_ns: u64) -> Option<u64>` —
   provable + oracle-testable; the CAS-retry shell stays a thin loop around it.
2. **Integer nanos, not f64**, in the decision path (the bench version compares
   `new_tat as f64 > now as f64 + burst_nanos` — a CBMC cost cliff AND a rounding-determinism
   hazard at large `now`). f64→u64 conversion once, at construction.
3. `proof_gcra_transition_contract` (Kani, or native algebraic per the research — item 8's call,
   ledgered): with headroom assumes, no overflow in `max(tat,now) + cost`; grant ⇒
   `new_tat = max(tat,now) + cost` (monotone, conserves exactly `cost`); deny ⇔
   `max(tat,now) + cost > now + burst`.
4. `proof_gcra_two_step_interleaving`: two sequential applications from one symbolic state:
   combined grants conserve `cost₁+cost₂`, TAT monotone — the strongest interleaving statement a
   sequential prover can honestly make; the full concurrent argument = item 8's differential
   oracle + compare_exchange semantics.

## 6. CI integration design (rescoped)

### 6.1 The call: Bucket B rides the existing gate; Bucket C gets a small separate job

**Bucket B needs NO new CI machinery.** The 16 native tests are plain `#[test]`s under the
existing filters — they land by **bumping `min_tests` on the existing HOT-PATHS.tsv rows**
(`order_machine::` 25→29, `pq::keccak::` 4→7, `event_log::tests::sha3` 2→3, `pq::dsa::` gains an
arithmetic-filter row, `pq::kem::` 8→13 — executor recomputes exact counts) plus one release-mode
row for the slow `reduce32` sweep. The `cargo-test` job and `hardening-gate` re-execute them
exactly as they re-execute everything else (P7 discipline already built, item 6). Deleting a
native exhaustive test goes RED by the existing min-count floor — the anti-forgery core extends
to the new tests for free.

**Bucket C (4 harnesses) gets `mode = kani` manifest rows + a separate `kani-gate` job.** Same
manifest (single source of truth: one gap ledger, one @ZONE obligation surface — a hot-path diff
cannot dodge the Kani obligation while satisfying the test one), separate execution: Kani is a
separate toolchain (own rustc + CBMC via `cargo install kani-verifier` + `cargo kani setup`) with
minutes-scale proofs; folding it into `hardening-gate` would break that job's `--locked
--offline` P6 determinism and seconds-scale bite. `hardening-gate.sh` **skips** `mode=kani` rows
with a visible "deferred to kani-gate" notice; `scripts/kani-gate.sh` parses the same file.

### 6.2 New manifest rows (seed)

```
kernel/src/pq/dsa.rs      pq  proof_ntt     3  kani  4  full-ntt-may-drop-to-butterfly-lemma(§3.2-ladder);montgomery_reduce-included
kernel/src/event_log.rs   pq  proof_keccak_copies_equivalent  1  kani  4  retires-with-keccak-dedup-ticket(§3.3)
kernel/src/token_bucket.rs -  proof_gcra    2  kani  4  MISSING(item-8):spec-in-BLUEPRINT-ITEM-07-§5
```

(`min_tests` reinterpreted per mode as **min successful harnesses** — zero-match = RED, so a
deleted/renamed proof cannot stay green. Existing `MISSING(item-7)` gap entries flip to resolved
as the corresponding tests/harnesses land.)

### 6.3 Zero-dep status — the distinction, preserved correctly

The synthesis's rule (line ~21) names Kani verbatim as CI-time tooling that does **not** count
against zero-dep, and the wiring keeps that true mechanically: harnesses live in `#[cfg(kani)]`
modules; the `kani` API crate is injected by `cargo kani` only when `cfg(kani)` is active;
**nothing enters `Cargo.toml` or `Cargo.lock`**; normal builds compile every harness out (same
containment as `ct_gate.rs`'s `#[cfg(any(test, feature = "ct-gate"))]`). The zero-dep gate
(items 1+13) and per-crate gate (item 31) are untouched by construction. The feasibility research
§0 independently confirms this framing and adds the procedure ruling: Kani = terminal state (c),
legitimate CI boundary — **and** the rescope shrinks even that boundary to 4 harnesses.

### 6.4 `kani-gate` job design

```
kani-gate:
  trigger   : same diff-scope logic as hardening-gate (merge-base diff ∩ kani-row paths, plus
              the harness files / kani-gate.sh / workflow itself); PLUS unconditionally on
              pushes to main (deleted-harness floor).
  toolchain : cargo install kani-verifier --locked --version <PINNED> && cargo kani setup
              (cache ~/.kani keyed on the pin). Honest P6 note: install needs network — same
              toolchain-class exception rustup already is; offline discipline governs crate
              deps, not toolchains. CBMC results deterministic for a pinned version.
  execute   : per mode=kani row: cd kernel && cargo kani [--features F] --harness <prefix>;
              parse per-harness SUCCESS lines; assert count >= min (zero-match = RED).
  self-test : kernel/src/kani_selftest.rs — proof_selftest_planted_overflow, a deliberate i32
              overflow annotated #[kani::should_panic]: reported SUCCESSFUL only because the
              fault IS caught. Every run — synthesis §9.7's "seeded fault demonstrably caught"
              as a standing property, mirroring ct_gate's planted-leak self-test.
  budget    : #[kani::unwind(N)] everywhere (unwinding assertions double as termination
              proofs); job timeout ~30 min; a harness that can't close moves down its §3.2
              fallback ladder and is re-ledgered, never silently dropped.
  bootstrap : if `cargo kani setup` fails in this environment, the job is marked RED-with-reason
              (::error:: naming the toolchain failure) and item 7's completion report lists the
              4 harnesses as honestly deferred — Bucket B's 16 targets are unaffected and land
              regardless (§0 consequence 3).
```

### 6.5 RED-path demonstration before first merge (P7 one layer up, item 6 §2.4 precedent)

Demonstrate and record in the PR: **(a)** planted-overflow harness with `should_panic` removed →
kani-gate RED; **(b)** a kani row whose filter matches zero harnesses → RED; **(c)** a
transiently-planted real fault caught by BOTH engines — e.g. widen `exhaustive_caddq_contract`'s
domain past ±Q → native test RED, and drop the `n==0` guard in `rotl` → equivalence/no-panic RED;
**(d)** restore → all green. Without (c) neither gate has been seen to catch a real bug class.

## 7. Deliverables and scope verdict

**Executor deliverables:** (1) 16 native exhaustive `#[test]`s per §3.1 (with the shadow-widened
overflow pattern) in `order_machine.rs`, `pq/dsa.rs`, `pq/kem.rs`, `pq/keccak.rs`,
`event_log.rs`; (2) 4 `#[cfg(kani)]` harnesses per §3.2 + `kani_selftest.rs` + the `keccak_f`
lift; (3) HOT-PATHS.tsv `min_tests` bumps + the reduce32 release row + 3 `mode=kani` rows +
`hardening-gate.sh` kani-row skip notice; (4) `scripts/kani-gate.sh` + ci.yml `kani-gate` job
(pinned, cached, bootstrap-failure path per §6.4); (5) the §6.5 RED-path record.

**Provability verdict per domain (post-rescope):**

| Domain | Engine | Now / later / limited |
|---|---|---|
| FSM graph (4 targets) | native exhaustive | **Fully provable now, total** — no Kani |
| `reduce32`/`caddq`/rounding | native exhaustive | **Fully provable now, total** (reduce32 = minutes-scale release row) — no Kani |
| Keccak panic-freedom (both copies, sponges) | native (input-independence argument + length exhaustion) | **Fully provable now, complete** — no Kani; stronger than v1's bounded harnesses |
| ML-KEM (all 5) | native exhaustive | **Provable now** — `red` universality rests on `%` semantics + boundaries (noted); `poly_mul` = enumerated body lemma + documented induction (ceiling under any engine) |
| `montgomery_reduce` | **Kani** | Provable now if toolchain bootstraps; cheap harness |
| Full `ntt`/`invntt` sweeps | **Kani** | Provable now **with real timeout risk** — fallback ladder §3.2; ladder rung (i) ≈ the research's native interval argument, so the floor outcome is still honest |
| Keccak cross-copy equivalence (keystone) | **Kani** | Provable now if toolchain bootstraps; dissolves permanently when the owed dedup ticket lands (§3.3) |
| GCRA transition (2 targets) | Kani-or-native-algebraic, **item 8** | Spec'd here (§5); manifest row blocks item 8 without them |

**Net:** item 7 delivers 16/22 targets with zero new tooling on day one, concentrates Kani where
it genuinely pays (4 harnesses), defers 2 to item 8 by design, and hand-rolls no SAT solver.
