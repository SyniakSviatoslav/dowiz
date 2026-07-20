# Miri analysis-toolchain pin (roadmap item 52 — `miri-gate`)

- **Pinned toolchain:** `nightly-2025-11-21` (the newest Miri-capable nightly
  present in the local toolchain set at item-52 implementation time).
- **Why pinned (item-14 discipline in spirit):** Miri is a *separate analysis
  toolchain* — it runs on nightly and is never used to build shipped artifacts,
  so the shipped build pin `channel = "1.96.1"` (`rust-toolchain.toml:6`) is
  byte-untouched by this item. The analysis nightly is recorded here (not
  floating) so a `miri-gate` run is reproducible; bumps follow item-14's
  ledger-in-same-diff rule.
- **Component:** `rustup component add --toolchain nightly-2025-11-21 miri`
  (+ `cargo +nightly-2025-11-21 miri setup`).
- **Empirical confirmation required (NOT asserted):** the exact set of
  `simd`/`householder` AVX2/FMA intrinsic bodies Miri can/can't interpret, and
  whether `is_x86_feature_detected!("avx2")` returns `false` under Miri on the
  CI host (the scalar-fallback assumption in BLUEPRINT-ITEM-52 §2.4), must be
  confirmed on the FIRST CI run and recorded here. If a host reports avx2
  available and Miri then fails to interpret an intrinsic, the affected test
  filter is re-ledgered (not silently dropped).
- **Bootstrap-failure policy:** if the pinned nightly / `miri` component cannot
  be installed in the CI sandbox, the `miri-gate` job is marked RED-with-reason
  (names the toolchain failure). The `arena` bump-allocator UB surface is the
  whole point, so a Miri bootstrap failure is a first-class reported outcome per
  BLUEPRINT-ITEM-52 §6, not a silent skip.

## Empirical confirmation (2026-07-20, first real run — recorded per the requirement above)

Run on `nightly-2025-11-21` miri against `main`-tree kernel (reconcile branch
`reconcile/space-grade-7-branches-2026-07-20`):

- **`miri_selftest::`** — planted OOB read CAUGHT (`Undefined Behavior` reported, exit non-zero).
  The UB row is honest. ✔
- **`arena::`** — Miri found REAL UB on the first run: `alloc_slice` rounded the *offset* up to
  `align_of::<T>()` but the backing `Vec<u8>` base pointer is only guaranteed 1-aligned, so the
  carved `&mut [T]` could be (and under Miri's allocator, was) unaligned —
  `constructing invalid value: encountered an unaligned reference (required 8 byte alignment but
  found 1)` at `arena.rs:105`. Fixed in the same diff (round the absolute *address*, not the
  offset); `arena::` now 7/7 GREEN under Miri and 7/7 native. This is the gate doing exactly the
  job item 52 built it for.
- **`householder::`** — 14/14 GREEN under Miri (FMA intrinsic bodies fall back to the scalar
  path; `dot_fma_matches_scalar` + 13 more).
- **`simd::`** — the 4 `kalman_batch` tests GREEN; the 4 `simd_softmax_*` bit-identity tests FAIL
  under Miri because Miri deliberately perturbs float results (its float-nondeterminism testing
  mode), which is incompatible with bit-identity assertions by design. Per the re-ledger policy
  above, the manifest row filter is narrowed to `simd::tests::kalman` (min 4); the full `simd::`
  suite still runs natively in the always-on suite. NOT silently dropped — this note is the ledger.
