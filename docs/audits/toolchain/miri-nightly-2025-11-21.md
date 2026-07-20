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
