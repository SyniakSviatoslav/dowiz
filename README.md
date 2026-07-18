# {{BRAND}}

> Sovereign, post-quantum delivery infrastructure — a deterministic Rust/WASM
> kernel and mesh protocol; DeliveryOS is the proof-of-concept riding on top.

**Status: pre-1.0 / experimental.** The kernel math is deterministic and
self-verifying; the surrounding product surface is not yet a production GA. No
fabricated maturity claims — see the verification gates in `docs/design/`.

## What it is

- **Deterministic kernel** (`kernel/`) — the sole math authority for order
  lifecycle, money reversal, and compensation. Offline-first, no mandatory cloud
  dependency.
- **Mesh protocol** (`bebop2/`) — post-quantum capability-auth + proof-of-delivery
  over consenting hubs; JS/TS **never** re-implements kernel math.
- **DeliveryOS** — the reference hub/application demonstrating the protocol.

## Quickstart

```sh
# source-of-truth surface: the kernel's own tests
cd kernel && cargo test

# the wasm web demo
cd web && wasm-pack build   # (see web/README.md)
```

## Architecture at a glance

- The **kernel is the only authority** on order state and money movement.
- All crypto is real and from-scratch / FIPS-204 verified (Ed25519 RFC 8032,
  ML-DSA-65, XChaCha20-Poly1305) — no placeholders.
- Fail-closed by design: the kernel red-line gate denies money / auth / RLS /
  migrations by default.

## Governance & legal

- License: **AGPL-3.0-or-later** (`LICENSE`), with `NOTICE` + `TRADEMARK.md`.
- Contributions under the **Developer Certificate of Origin** (`DCO`) — sign off
  with `Signed-off-by:` (see `CONTRIBUTING.md`).
- Report vulnerabilities privately — see `SECURITY.md`.
- Community standards — `CODE_OF_CONDUCT.md`.

## Cite

See `CITATION.cff`.

---

## Current verified state (2026-07-18)

The kernel is the math authority and it is **exercised, not claimed**. Latest
green suites (this tree, default features):

- `kernel/` — **632 passed, 0 failed** (`cargo test --lib`).
- `engine/` — **63 passed, 0 failed** (incl. the cross-crate Laplacian
  sign-convention KAT, `TORVALDS-21`).
- `bebop2/core` — **264 passed, 0 failed**; `bebop2/proto-wire` — **68 passed,
  0 failed** (see the `bebop2` repo for its own README).

### Audit posture

The 2026-07-18 adversarial audit (`docs/research/AUDIT-2026-07-18-*.md`) found
real in-repo defects; each was root-caused against the live tree and fixed with
a RED→GREEN test, not papered over:

- **NaN-panic sorts** (engine/kernel spectral stack) → `f64::total_cmp`.
- **`FileBlockStore::put` panic on I/O** (full-disk) → returns `false`.
- **Kalman `gain()` panic on singular `S`** → `Option<Mat>` (degrades).
- **`zerocopy` unchecked `view_as_f32`** → checked `Option<&[f32]>` (release-safe).
- **Money guard dead `i64` check** → live `f64` fractional rejection.
- **`intake` unbounded integer-range enumeration (DoS)** → `MAX_ENUM_WIDTH` cap.
- **Tax EQC parity** skipped the negative-rate edge → generated organ now
  mirrors the law's `subtotal==0 → Ok(0)` short-circuit; parity sweep covers it.
- **Telemetry JSONL corruption** (duplicate `kind` key + double-escape) → fixed;
  output verified valid single-key JSON.
- **Committed binary** `tools/telemetry/hermes-kernel` → untracked (build from
  source at provision; gitignored).
- **Laplacian sign hazard** (engine `∇² = −(D−A)` vs kernel `L = +(D−A)`) →
  convention doc at both sites + cross-crate KAT.

Items the audit flagged as **doc / ops / design** (not mechanical code bugs) —
error-enum consistency, `tracing`-vs-`println` practice, backup e2e drill,
roadmap critical-path consistency, stale CI docs — are recorded in the audit
synthesis scorecard and dispositioned there; they are not silently "fixed".

---

*Brand name is a `{{BRAND}}` placeholder pending the O16 decision
(dowiz vs DeliveryOS). This README does not assert a public-flip "go"; the flip
is an explicit, separate operator action.*
