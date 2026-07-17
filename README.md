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

*Brand name is a `{{BRAND}}` placeholder pending the O16 decision
(dowiz vs DeliveryOS). This README does not assert a public-flip "go"; the flip
is an explicit, separate operator action.*
