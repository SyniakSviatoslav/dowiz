# Changelog

All notable changes to the dowiz kernel + product are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/) + CalVer `YYYY.MM.PATCH`.

## [2026.07.0] — 2026-07-18

### Added
- `KERNEL_PROTO_VERSION` in-code wire version constant (kernel/src/lib.rs).
- Fail-closed drift gate: NaN/±inf + ragged (index-leak) operators rejected as
  `Unstable` before indexing (`classify_drift` + `Mat::from_vecvec_checked`).
- `CompensatedRefund` FSM compensation edge with mandatory ledger reversal
  (money nets to exactly zero; no un-reversed refund).
- `order_from_in` server-authoritative subtotal/total recompute (forged client
  total cannot survive a fold — E1 closed).
- Resource caps on untrusted-JSON `_js` entry points (Box::leak OOM, harmonic
  `n`, payload, log bounds).
- `compute_order_total` / `apply_tax` overflow-safe (checked_add/checked_mul).

### Fixed
- Spectral `spectral_radius` NaN-fail-open (poisoned spectrum read as healthy).
- P-A kernel parity: eig2x2 dedup, `spectral_radius`→proven const ρ=0,
  normalize-before-hash canonical key, eqc-rs Asin/Atan2/DivHalfUp + integer
  emission.
- P-B `RetainedBase::admit` no longer silently defeated by a non-finite operator.

### Verification
- Kernel 539 tests green; engine 57; ci-truth 30.
- bebop: Lyapunov NaN/PSD fail-closed (V1 #2), mesh replay-nonce + Sybil
  `IssuanceBudget` cap, RefSigner secret-leak removed.
