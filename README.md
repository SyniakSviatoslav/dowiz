# dowiz

> Sovereign, post-quantum delivery infrastructure — a deterministic Rust/WASM
> kernel and mesh protocol. **DeliveryOS** is the reference application built
> on top of it, not the other way around.

**Status: pre-1.0 / experimental.** The kernel math is deterministic and
self-verifying; the surrounding product surface is not yet a production GA. No
fabricated maturity claims — see the verification gates in `docs/design/`.

## What it is

Most delivery/logistics platforms are built around a central server that owns
your order history, your location trail, and (usually) a "trust score" it
computes about you behind closed doors. dowiz starts from a different premise:
the parts of the system that actually have to be authoritative — money,
order state, identity — are pinned down in a small, deterministic Rust
kernel that anyone can audit and run themselves, offline if they want to.
Everything else (the courier mesh, the UI, the reference app) is built as a
consenting layer on top of that kernel, not the other way around.

- **Deterministic kernel** (`kernel/`) — the sole math authority for order
  lifecycle, money reversal, and compensation. Offline-first, no mandatory cloud
  dependency, exact integer arithmetic for money (no float drift, ever).
- **Mesh protocol** (`bebop2/`) — post-quantum capability-auth + proof-of-delivery
  over consenting hubs; JS/TS **never** re-implements kernel math.
- **Physics-based rendering** (`engine/`) — no DOM. UI state (recall, decay,
  layout, motion) is driven by a damped-wave field simulation on `wgpu`,
  with a synthetic accessibility layer instead of real DOM elements.
- **DeliveryOS** — the reference hub/application demonstrating the protocol.

## Main concepts

**The kernel is the only authority.** Order state and money movement live in
one deterministic Rust module with exact `i64` arithmetic and fail-closed
red-line gates — money, auth, RLS, and migrations are denied by default unless
explicitly proven safe. No service anywhere else in the stack is allowed to
re-derive or override what the kernel says happened.

**Couriers going offline is a first-class case, not an edge case.** The mesh
protocol is built on delay-tolerant networking (a real Bundle Protocol v7
implementation) specifically because a courier's phone losing signal is the
normal case for a delivery network, not a failure mode to paper over. Messages
are authored offline, signed, and carried by whichever device reaches a peer
next — which is why every message carries its own proof of authenticity
instead of relying on a live, continuously-connected session.

**Trust is a signed capability, never a score.** dowiz's mesh explicitly
rejects courier/node reputation systems — there is no rating, ranking, or
trust score computed about any participant anywhere in the protocol. Access is
granted by cryptographically signed capability certificates issued by an
anchor, not earned or lost through a behavior score a black box maintains
about you.

**Hybrid, not a bet on one algorithm.** Every signature in the system is
double-signed — classical Ed25519 *and* post-quantum ML-DSA-65 — so a break in
either scheme alone doesn't compromise the system. This is deliberate
defense-in-depth, not a pure post-quantum wager.

## What's genuinely novel here

- **A type-level guarantee that animation code can never touch money.** The
  rendering engine has a dedicated boundary type (`FieldValue`) and a runtime
  guard that make it a compile-time/runtime error for any UI tweening or
  interpolation logic to operate on a monetary value — the two domains are
  structurally incapable of touching, not just conventionally kept apart.
- **One physics operator, many UI concerns.** The design direction for the
  rendering engine is to drive recall, decay, layout, and motion off a single
  damped-wave field equation rather than separate ad-hoc animation systems for
  each — see `docs/design/` for the field-UI research; this is an active
  design line, not a finished claim.
- **DTN-first mesh, not store-and-forward bolted on.** Delay-tolerant
  networking (offline authoring, signed bundles, exactly-once delivery on
  reconnect) is the mesh's default operating mode, not a fallback path.
- **Capability certs instead of reputation, by explicit rule.** No node or
  courier is ever scored, ranked, or rated anywhere in the protocol — this is
  an enforced design rule, not an omission.
- **"Verified, not claimed" as an engineering discipline.** Every fix in this
  repo's history is expected to land with a RED→GREEN test proving the bug
  existed and is now closed, and every performance claim is expected to carry
  a real benchmark number — not asserted, measured. The audit log below is an
  example of that discipline in practice, not a marketing section.

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
- Active hardening in progress on the post-quantum crypto stack — see
  `docs/design/` for the current state; nothing unresolved here is exposed as
  a production claim above.

## Governance & legal

- License: **AGPL-3.0-or-later** (`LICENSE`), with `NOTICE` + `TRADEMARK.md`.
- Contributions under the **Developer Certificate of Origin** (`DCO`) — sign off
  with `Signed-off-by:` (see `CONTRIBUTING.md`).
- Report vulnerabilities privately — see `SECURITY.md`.
- Community standards — `CODE_OF_CONDUCT.md`.

## Cite

See `CITATION.cff`.

---

## Current verified state (as of 2026-07-18)

The kernel is the math authority and it is **exercised, not claimed**. Latest
green suites recorded at that date (this tree, default features) — treat as
of that date, not re-asserted as of today without re-running:

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

### Roadmap

The active research/blueprint pipeline (mesh auth-layer hardening,
performance work, verification tooling) is tracked in
`docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §19, with
the detailed dependency-ordered status in
`docs/design/CORE-ROADMAP-2026-07-17/MASTER-STATUS-LEDGER-2026-07-19.md`.

---

> нахуя мені система, що працює проти мене
