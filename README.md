# dowiz

> A decentralized mesh-hub delivery platform — no platform tariffs on
> participation, no behavioral scoring or profiling of any participant
> (enforced in CI, not just promised), and network-level anonymity as an
> active, concretely-scoped roadmap item (Tor/onion access — see below), not
> a slogan. Built on a deterministic Rust/WASM kernel and a store-and-forward
> mesh protocol. **DeliveryOS** is the reference application built on top of
> it, not the other way around.

**Status: pre-1.0 / experimental.** The kernel math is deterministic and
self-verifying; the surrounding product surface is not yet a production GA.
Every claim below is either cited to a real file/test or explicitly marked as
in-progress — this project's culture is *verified, not claimed*, and this README
holds itself to the same rule.

---

## What it is

Most delivery/logistics platforms are built around a central server that owns
your order history, your location trail, and (usually) a "trust score" it
computes about you behind closed doors. dowiz starts from a different premise:
the parts of the system that actually have to be authoritative — money, order
state, identity — are pinned down in a small, deterministic Rust kernel that
anyone can audit and run themselves, offline if they want to. Everything else
(the courier mesh, the UI, the reference app) is built as a consenting layer on
top of that kernel, not the other way around.

- **Deterministic kernel** (`kernel/`) — the sole math authority for order
  lifecycle, money reversal, and compensation. Offline-first, no mandatory cloud
  dependency, exact integer arithmetic for money (no float drift, ever).
- **Mesh protocol** (`bebop2/`) — a post-quantum, capability-authenticated
  delivery protocol. The specification of how a hub agreement becomes a real
  kernel order transition lives here (`bebop2/delivery-domain/DESIGN.md`); the
  protocol's cryptographic core is developed in the companion **OpenBebop**
  repository and injected at a seam.
- **Physics-based rendering** (`engine/`) — no DOM. The UI is treated as a
  *field*: shapes are signed-distance fields and the frame is produced by
  integrating a damped wave equation over a field buffer. (Compute is CPU-side
  today; a GPU adapter is a declared, not-yet-wired seam — see below.)
- **DeliveryOS** — the reference hub/application demonstrating the protocol.

## Main concepts

**The kernel is the only authority.** Order state and money movement live in
deterministic Rust with exact `i64`/`i128` arithmetic and fail-closed red-line
gates. `kernel/src/order_machine.rs` is an explicit finite-state machine with a
`decide → Event`, `state = fold(events)` reducer; forbidden transitions are
errors, not silent no-ops. `kernel/src/money.rs` opens with "zero float
arithmetic on monetary values" — amounts are currency-typed and checked
(overflow-safe), and refunds flow through a double-entry ledger where a
compensated order nets to *exactly zero*. Because the core has no clock, RNG,
network, or floats in its decision path (`MANIFESTO.md` C2), every node computes
the same result from the same events, offline — no split-brain, no float drift,
and any node can replay and audit the ledger. Unsafe capabilities (ledger/money,
auth, migrations) are denied by default at a red-line gate
(`kernel/src/ports/agent/scope.rs`, `RedLinePolicy::DenyByDefault`); database-row
security (RLS) is enforced at the data layer.

**Couriers going offline is a first-class case, not an edge case.** The mesh is
designed for delay-tolerant networking (store-and-forward, DTN / RFC 9171 class)
specifically because a courier's phone losing signal is the *normal* case for a
delivery network. Messages are authored offline, signed, and carried by whichever
device reaches a peer next — which is why every message carries its own proof of
authenticity instead of relying on a live, continuously-connected session.
Low-latency gossip (libp2p-gossipsub) was explicitly rejected in favor of
reliability over latency (`RESEARCH-transport-dtn-mesh.md`, `DECISIONS.md` D3).

**Trust is a signed capability, never a score.** dowiz explicitly rejects
courier/node reputation systems — there is no rating, ranking, or trust score
computed about any participant. Access is granted by cryptographically signed,
attenuable capability certificates, not earned or lost through a behavior score a
black box maintains about you. This is enforced, not just intended: a CI job
(`no-courier-scoring`, `.github/workflows/ci.yml`) *fails the build* if any
`courier_score/rating/reputation` identifier appears in the kernel or engine, and
the routing enum deliberately omits `Ord`/`PartialOrd` so a "quality router" is
literally unrepresentable in the type system (`kernel/src/decision/mod.rs`,
`kernel/src/domain.rs`).

**Hybrid, not a bet on one algorithm.** The capability-cert architecture
(`kernel/src/capability_cert.rs`) signs with *both* a classical and a
post-quantum algorithm under a `RequireBoth` policy — both must verify, with no
OR code path — so a break in either scheme alone cannot forge a certificate.
This is deliberate defense-in-depth, not a pure post-quantum wager.

## What's genuinely novel here

Each item is backed by code in this tree, or explicitly flagged as a design
direction:

- **A type-level guarantee that animation code can never touch money.** In a
  physics-driven UI, everything wants to interpolate smoothly — money must not.
  `engine/src/money_guard.rs` makes the illegal path *unrepresentable*: the
  `Money` type deliberately does not implement the `FieldValue` trait, so
  `interpolate(money, …)` or `Spring<Money>` is a compile error — you cannot even
  write code that lerps a price. A runtime guard (`TweenGuard::present_money`)
  backs it up, rejecting any fractional amount (the tell-tale signature of an
  interpolated `155.5`). A prior version of the guard was found to be dead code
  (its `i64` parameter could never be fractional) and fixed to take an `f64` — a
  small but honest example of the audit culture below.

- **An equation → Rust compiler (`eqc`).** One source-of-truth math expression
  compiles to Rust with dual emission (floating-point and exact-integer) —
  `tools/eqc-rs/` is the compiler, `kernel/src/eqc_gen.rs` its committed output
  ("GENERATED by eqc-rs — do not hand-edit"). The generated integer "organs" are
  parity-pinned to the hand-written money law by a test asserting *exact integer
  equality*, so the law and its compiled form cannot silently diverge.

- **A finite-state machine that proves its own correctness.** The order FSM
  (`kernel/src/order_machine.rs`) is analyzed by five independent graph lenses —
  cycle detection, cyclomatic number, topological order, BFS reachability, and
  spectral radius — and pinned to a golden signature. Edit the lifecycle in a way
  that introduces a cycle, and the self-check goes red.

- **Deliberate hybrid classical + post-quantum crypto.** Verified in this tree:
  from-scratch, **NIST-ACVP-byte-exact** ML-DSA-65 (FIPS-204,
  `kernel/src/pq/dsa.rs` + `kernel/src/pq/kat/acvp/`) and a real X25519 +
  ML-KEM-768 hybrid KEM with no classical-only fallback (`kernel/src/pq/hybrid.rs`);
  at-rest data uses AES-256-GCM (`kernel/src/pq/volume.rs`). The certificate
  architecture is algorithm-agile (a suite tag is bound into the signed bytes to
  prevent downgrade). The classical Ed25519 signing leg is a *production-injected
  seam* — the real implementation lives in the companion OpenBebop repo, not
  committed here — which this README states plainly rather than overclaiming an
  in-kernel Ed25519.

- **Capability certs instead of reputation, by explicit rule.** No node or
  courier is ever scored, ranked, or rated — enforced in CI and in the type
  system (see "Trust is a signed capability" above). An enforced design rule, not
  an omission.

- **"Verified, not claimed" as an engineering discipline.** Fixes are expected to
  land with a RED→GREEN test proving the bug existed and is now closed, and every
  performance claim is expected to carry a real, measured benchmark number. The
  audit posture below is that discipline in practice, not a marketing section.

> A note on ambition vs. reality: the design corpus (`docs/design/`) sketches a
> further unification — a *single* Laplacian operator driving recall, decay,
> layout, motion, and blur across the whole system. In the code today those are
> distinct operators (the field wave equation, a graph Laplacian, and spring
> motion), and that grand unification is tagged **speculative/novel** in its own
> blueprint. It is a research direction, not a shipped claim. Likewise the GPU
> renderer: `wgpu`/WebGL/WebGPU are declared feature seams that are deliberately
> empty in the default build, and SDF *text* rendering is not yet implemented.

## Quickstart

```sh
# source-of-truth surface: the kernel's own tests
cd kernel && cargo test

# build the kernel's WASM surface (emits kernel/pkg + kernel/pkg-web)
bash scripts/build-kernel-wasm.sh

# run the zero-dependency, kernel-driven web demo
cd web && npm run serve      # → http://localhost:8099/web/index.html
```

## Governance & legal

- License: **AGPL-3.0-or-later** (`LICENSE`), with `NOTICE` + `TRADEMARK.md`.
- Contributions under the **Developer Certificate of Origin** (`DCO`) — sign off
  with `Signed-off-by:` (see `CONTRIBUTING.md`).
- Report vulnerabilities privately — see `SECURITY.md`.
- Community standards — `CODE_OF_CONDUCT.md`.

## Cite

See `CITATION.cff`.

---

## Current verified state

Test counts below were re-run against this tree on 2026-07-19 (measured, not
copied forward):

- `kernel/` — **859 passed, 0 failed, 3 ignored** (`cargo test --lib`).
- `engine/` — **116 passed, 0 failed** (`cargo test`: 112 lib + 4 integration),
  including the cross-crate Laplacian sign-convention KAT `TORVALDS-21`, which
  pins the engine's physics Laplacian `∇²=−(D−A)` against the kernel's graph
  Laplacian `L=+(D−A)`.

The bebop2 protocol crates have their own test suites in the companion OpenBebop
repository; those counts are maintained there and not re-asserted here.

### Audit posture

The 2026-07-18 adversarial audit (`docs/research/AUDIT-2026-07-18-*.md`) found
real in-repo defects; each was root-caused against the live tree and fixed with a
RED→GREEN test, not papered over. A representative sample:

- **NaN-panic sorts** (engine/kernel spectral stack) → `f64::total_cmp`.
- **`FileBlockStore::put` panic on a full disk** → returns `false` instead.
- **Kalman `gain()` panic on a singular matrix** → `Option<Mat>` (degrades safely).
- **Money guard dead `i64` check** → live `f64` fractional rejection.
- **`intake` unbounded integer-range enumeration (DoS)** → `MAX_ENUM_WIDTH` cap.
- **Laplacian sign hazard** (engine `∇²=−(D−A)` vs kernel `L=+(D−A)`) →
  convention doc at both sites + the cross-crate `TORVALDS-21` KAT.

Items the audit flagged as **doc / ops / design** (not mechanical code bugs) are
recorded in the audit synthesis scorecard and dispositioned there — they are not
silently "fixed."

### Roadmap

The active research/blueprint pipeline (mesh auth-layer hardening, performance
work, verification tooling) is tracked under `docs/design/`, anchored by
`docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`. The project is
under active pre-1.0 hardening — the post-quantum crypto and mesh layers in
particular are still being reviewed and tightened. Please report security issues
privately (see `SECURITY.md`).

A few specific directions worth knowing about, each honestly labeled by its real
status today:

- **Network-level anonymity via Tor/onion access** — a concretely scoped
  blueprint (`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P53-tor-onion-integration.md`),
  not yet built. It hides a client's or a small venue's network identity/location
  to reach the service — the same class of technology SecureDrop and similar
  privacy-critical services use — without touching the capability-cert auth
  layer at all: anonymity at the network layer is never a substitute for the
  authentication layer.
- **Spectral graph methods** — already real and load-bearing in specific,
  narrow places today (`kernel/src/spectral.rs`: PageRank-style ranking, Markov
  spectral-gap analysis, graph-Laplacian mode work), with active research into
  where else in the stack (CPU and GPU) the same eigenvector machinery
  genuinely earns its cost rather than being applied for its own sake.
- **A living-memory retrieval layer** — a real, multi-layer (trigram + BM25 +
  planned semantic) design exists, but it is explicitly a blueprint today: its
  only current driver in the tree is inert. Treat this as a research direction,
  not a shipped capability, until this note is updated.
- **A closed-loop self-improvement process** — this project's own development
  process uses a Markov-attractor-based feedback loop over its own tooling
  outcomes to catch drift; it's real infrastructure for how the project is
  built, not (yet) a feature exposed to platform users.
- **Quantum-sourced entropy** — cryptographic key material is already seeded
  from a real quantum random number source rather than a weaker PRNG alone;
  extending genuine quantum-noise injection further into the stack is an open
  research thread, not a finished claim.

---

*dowiz is pre-1.0 and evolving quickly. Status reflects the tree as of the stated
date; when in doubt, run the suites — the kernel is exercised, not asserted.*

---

> нахуя мені система, що працює проти мене
