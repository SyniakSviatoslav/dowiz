# BLUEPRINT — Item 52: `miri-gate` — Targeted UB Detection over the Real Unsafe Surface

- **Date:** 2026-07-19 · **Tier:** roadmap §J (fourth wave) · **Status:** BLUEPRINT v1 (planning
  artifact, no code). **Independent — zero prerequisites on items 47/50/51; the on-`main` targets are
  dispatchable now** (roadmap:825, verified §0).
- **Sources (read this session):**
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §J item 52 (lines 825–850);
  `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` §2.2 (scope RULING + SIMD-limit honesty);
  `docs/audits/hardening/CHECKLIST.md`; `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (the CI-gate
  template this mirrors); `rust-toolchain.toml` (item-14 build pin).
- **Ground-truth code cited (branch `main`, re-verified in-tree this session — the inventory is
  CORRECT and current):** `kernel/src/arena.rs`; `kernel/src/simd.rs`; `kernel/src/householder.rs`;
  `kernel/src/fdr/pmu.rs`; `.github/workflows/ci.yml` (kani-gate/hardening-gate job shapes);
  `docs/audits/hardening/HOT-PATHS.tsv`.
- **Upstream:** none (dispatchable now). `fdr/pmu.rs` was already merged with the exec branch (§0), so
  even its targets are available — the roadmap's "joins post-FDR-merge" caveat for pmu is satisfied.
- **Downstream:** item 53 (`lint-gate`) promotes this job to a required check ("miri-required").

---

## 0. Dependency-status correction

The roadmap splits item 52 into "on-`main` targets (`arena`/`simd`/`householder`) dispatchable now"
vs "`fdr/pmu` folds in post-FDR-merge" (lines 838–839, 899–900). **The FDR merge has happened** —
`main` HEAD `6701bbb6f` includes `kernel/src/fdr/pmu.rs` (verified in-tree). So **all four modules
are available now**; the split collapses. The build toolchain pin `channel = "1.96.1"`
(`rust-toolchain.toml:6`) is untouched by this item.

## 1. Scope / goal

Add ONE CI job, `miri-gate`, running `cargo miri test` **restricted to the modules that carry real
`unsafe`**, so undefined-behavior in the kernel's raw-pointer / FFI code is caught by the Rust UB
interpreter — turning the aspirational Miri doc-comments (§2.3) into an enforced, re-executed gate.
Honest about what Miri cannot reach (SIMD intrinsics, `_rdtsc`, raw `syscall`).

**Non-goals:** NOT a blanket "miri-everything" mandate; NOT the four `unsafe`-free wrapper modules
(filtering them matches zero `unsafe` — theater, synthesis §2.2); NOT a claim that a green gate means
"SIMD/PMU is Miri-clean" (§4 honest-limit doc); NOT a build-toolchain change (Miri pins its OWN
analysis nightly, §3.4).

## 2. Current-state grounding

### 2.1 The real unsafe surface — 19 blocks in exactly 4 modules (re-verified, exact line numbers)

A precise grep (`unsafe fn|unsafe impl|unsafe trait|unsafe {|unsafe extern`, excluding comment
mentions) over `kernel/src/` this session returns **exactly** the synthesis's corrected inventory:

| Module | Real `unsafe` blocks | Cited lines | What the unsafe does |
|---|---|---|---|
| `kernel/src/arena.rs` | **6** | `:90`, `:104`, `:105` (production bump allocator); `:286`, `:287`, `:291` (`#[cfg(test)] CountingAlloc`'s `GlobalAlloc` impl) | raw-pointer bump allocation from an `UnsafeCell<Vec<u8>>` region + `from_raw_parts_mut` — **the classic Miri payoff** |
| `kernel/src/simd.rs` | **5** | `:66`, `:174`, `:222`, `:326`, `:380` | `#[target_feature(enable="avx2")]` intrinsic fns + call sites |
| `kernel/src/fdr/pmu.rs` | **5** | `:132` (`_rdtsc`), `:253` (`syscall5` inline `asm!`), `:288`, `:311`, `:334` (raw `perf_event_open` syscall FFI) | `_rdtsc` + raw x86-64 `syscall` — **outside Miri's reach** |
| `kernel/src/householder.rs` | **3** | `:32` (`dot_fma` FMA intrinsic), `:62`, `:68` (call sites) | AVX2 FMA dot-product + scalar-fallback dispatch |

Total = **19**. The synthesis's "old 21-block / 7-module list corrected" holds:
`messenger.rs`/`slot_arena.rs`/`chaos.rs`/`bounded_drainer.rs` carry only `unsafe` in *comments*
(their real-unsafe count is 0), and `pq/` (crypto) has **zero** unsafe. Confirmed this session — the
inventory is not stale.

### 2.2 arena's production unsafe is where UB actually hides — and is bounded

The 3 production blocks (`arena.rs:90`, `:104`, `:105`) are: `&mut *self.buf.get()` on the
`UnsafeCell`, `buf.as_mut_ptr().add(start) as *mut T`, and `slice::from_raw_parts_mut(slice_ptr,
len)`. The soundness argument (monotone bump offset ⇒ disjoint slices; `T: Copy + Default` ⇒ no Drop
hazard; `reset(&mut self)` ⇒ borrow-checker-proven no live loans) is written out at `arena.rs:13–33`
and `:85–108`. Miri is precisely the tool that *tests that argument holds* rather than just reads
plausibly (`arena.rs:32` says so verbatim). The 3 test-only blocks (`:286–291`) are the
`count-allocs` global allocator and ride along under `cargo miri test`.

### 2.3 The aspirational Miri comments already exist — item 52 makes them real

`arena.rs:31–33`: *"The done-check (§7 W5) runs this module's tests under Miri to confirm the
soundness argument holds, not just that it reads plausibly."* Also referenced from
`arena.rs:13` ("same as `householder.rs` / `simd.rs` §1.4"). The roadmap's grounded baseline —
"Miri runs nowhere (aspirational doc-comments only)" (roadmap:826–827) — is confirmed: no
`cargo miri` invocation exists in `.github/workflows/ci.yml` (the job list is telemetry/eqc/
claim-latency/v5c-reexec/cargo-test/bench/gitleaks/dco/supply-chain/zero-dep/decart/no-courier-
scoring/no-pub-raw-matrix/fence/regression-digest/firewall/mesh-adapter/toolchain-bump/hardening/
kani — no miri, verified).

### 2.4 SIMD + PMU are outside Miri by construction — the honest limit

`simd.rs:15–18` documents the house pattern: `is_x86_feature_detected!("avx2")` → AVX2 lane, scalar
fallback otherwise. Under Miri, feature detection reports the feature **unavailable**, so the
interpreted run exercises the **scalar path** — the AVX2 intrinsic bodies are never interpreted (and
`core::arch` AVX2 intrinsics are substantially unsupported by Miri anyway). `fdr/pmu.rs:253–268` is a
raw `core::arch::asm!("syscall")` — Miri cannot execute inline asm. So `simd.rs`/`householder.rs`
scalar paths are covered; their intrinsic bodies and `pmu`'s FFI stay covered by the
items-37/39 differential oracles + item 7 (Kani/asm), exactly as the synthesis states
(roadmap:838–843). A green `miri-gate` is therefore **never** "SIMD/PMU is Miri-clean."

## 3. Implementation plan

1. **The gate script `scripts/miri-gate.sh`** (mirrors `scripts/kani-gate.sh`'s structure): parse a
   small module→test-filter table (the real-unsafe modules only), run
   `cd kernel && cargo +<nightly-pin> miri test <filter>` per row, assert exit 0 AND the parsed
   `N passed` ≥ the row's min (the item-6 anti-forgery clause: a filter matching **zero** tests is
   RED, CHECKLIST §"§10/P7"). The filter table:
   - `arena::` (all 3 arena test fns: `alloc_slice_returns_zeroed_copy_values` (`arena.rs:175`),
     `alignment_is_respected_across_mixed_types` (`:199`), `reset_frees_region_for_reuse` (`:231`)
     — the bump-allocator UB surface);
   - `simd::` (the scalar-path bit-identity tests — covers the scalar branch under Miri);
   - `householder::` (scalar-path tests).
   `fdr/pmu.rs` is **documented-not-filtered**: its unsafe is `_rdtsc`/`syscall` FFI which Miri
   cannot interpret; running its tests under Miri would error on the asm, so pmu is listed in the
   gate's own doc as "covered by items 37/39/7, not miri-reachable", NOT added to the filter table
   (the synthesis's "documented not silently green", roadmap:840–843).
2. **Planted-UB self-test** — `kernel/src/miri_selftest.rs` behind `#[cfg(miri_selftest)]` (mirrors
   `kernel/src/kani_selftest.rs:1–28`): a deliberate out-of-bounds read or use-after-free that Miri
   MUST flag. The gate runs it and asserts Miri reports UB (the self-test PASSES only because the UB
   IS caught) — the `ct_gate`/`kani_selftest` planted-fault idiom, making "the gate detects real UB"
   a standing property, not a one-off demo.
3. **CI job `miri-gate`** in `.github/workflows/ci.yml`, slotted **after `kani-gate`** (ci.yml:528),
   modeled on kani-gate's shape (ci.yml:528–555): checkout; cache the analysis toolchain; install the
   pinned nightly + `rustup component add miri`; `cargo +<nightly> miri setup`; run
   `bash scripts/miri-gate.sh`. Like kani-gate it is a separate job (network toolchain, minutes-scale
   interpretation) and does NOT fold into the `--locked --offline` hardening-gate (§10/P6).
4. **Manifest touch (optional).** Item 6's `HOT-PATHS.tsv` is keyed to `mode=lib/dudect/kani`; Miri
   is a separate gate over `arena`/`simd`/`householder` (already `@ZONE`s at
   `HOT-PATHS.tsv:29`(householder) + others). Recommended: keep the miri filter table in
   `scripts/miri-gate.sh` (self-contained, like kani-gate's script parses the shared manifest but
   miri's target set is tiny) and cross-reference it from the manifest `gap` column so a hot-path diff
   surfaces the miri obligation. Executor's call whether to add `mode=miri` rows or keep it script-local.
5. **Toolchain isolation.** The build pin `1.96.1` (`rust-toolchain.toml:6`) is NOT touched. The job
   pins its OWN `nightly-YYYY-MM-DD` (Miri needs nightly), recorded in the workflow env + a line in
   `docs/audits/toolchain/` (bumps recorded, not floating — item-14 discipline in spirit; the analysis
   toolchain never builds shipped artifacts, so item-14's letter is intact, synthesis §2.2 /
   roadmap:844–846).

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

Item 52 is CI machinery, so the "tests" are the gate's own proven RED path plus the honest-limit doc:

1. **Oracle — the module tests are the oracle re-run under Miri.** The existing `arena`/`simd`/
   `householder` `#[test]`s ARE the corpus; Miri re-executes them as an interpreter (a stronger
   execution than the native run). No new oracle needed.
2. **dudect — `N/A`** (Miri is a UB interpreter, not a timing gate).
3. **Debug cross-check — `N/A`** (Miri already runs debug asserts; the gate's value IS the
   interpreted execution).
4. **Assembly spot-check — orthogonal** (stays with `toolchain-bump-gate` + item 7; Miri doesn't see
   codegen).
5. **The gate's own re-execution proof (the item-6/P7 layer one up):**
   - the planted-UB self-test (`#[cfg(miri_selftest)]`) makes the gate RED when the UB is present and
     GREEN when removed — recorded in the PR;
   - a filter matching **zero** tests is RED (anti-forgery clause reused);
   - a clean run over `arena`/`simd`/`householder` is green;
   - the build toolchain pin (`rust-toolchain.toml`) byte-unchanged after the run (asserted).
   - **First-run empirical confirmation, NOT asserted in advance:** the exact set of `simd`/
     `householder` intrinsic bodies Miri can/can't interpret is *confirmed on first CI run* and
     recorded in the gate's doc (synthesis §2.2: "exact intrinsic support to be confirmed empirically
     on first run, not asserted").

## 5. Falsifiable acceptance criteria

- `cargo miri test arena::` passes on a clean tree and FAILS when a UB is planted in `arena.rs`'s
  bump-allocator (bounds bumped past the region, or a `from_raw_parts_mut` with a bad len) — RED
  demonstrated before merge.
- The planted-UB self-test flips the gate RED→GREEN as the `#[cfg(miri_selftest)]` fault is
  added/removed.
- A miri filter matching zero tests exits non-zero.
- The gate's doc explicitly states pmu/SIMD-intrinsic bodies are NOT Miri-covered and names their
  real coverage (items 37/39/7) — a reviewer can find this claim, so a future "miri covers SIMD"
  misreading is pre-empted.
- `git diff rust-toolchain.toml` is empty across the whole item.

## 6. Dependency gates (honest)

| Gate | Status | Effect |
|---|---|---|
| items 47/50/51 | not required | independent — dispatchable now (roadmap:825). |
| FDR merge (for `fdr/pmu.rs`) | **MET** (§0) | pmu is available; but it is documented-not-filtered anyway (§3.1), so this gate is moot for item 52's actual filter set. |
| Nightly Miri toolchain availability in CI sandbox | **OPEN — empirical** | like kani-gate's `cargo kani setup`, `rustup component add miri` needs network; if it fails in the sandbox the job is RED-with-reason and the item reports the toolchain issue named — the `arena` payoff is the whole point, so a Miri bootstrap failure is a first-class reported outcome, not a silent skip. |

## 7. Operator / executor decision points (flagged)

1. **Nightly pin selection.** Which `nightly-YYYY-MM-DD` to pin for Miri. Recommend the newest
   nightly whose Miri passes `arena` cleanly at spec time, recorded in `docs/audits/toolchain/`.
   Executor picks + ledgers; bumps follow item-14 discipline.
2. **`mode=miri` manifest rows vs script-local table.** Whether to extend `HOT-PATHS.tsv` with a
   fourth mode or keep the tiny target set in `scripts/miri-gate.sh`. Recommend script-local (only 3
   filters; kani-gate already precedents a per-mode script) with a manifest `gap`-column cross-ref.
3. **Whether `cargo miri test` runs the full crate then filters, or filters at the cargo level.**
   `cargo miri test <filter>` still *compiles* the whole crate under Miri's target; the filter only
   selects which tests *run*. Interpreting the whole compile is the cost. Executor confirms the job
   time budget on first run (mirrors kani-gate's ~30-min budget note, ci.yml:532 comment).
