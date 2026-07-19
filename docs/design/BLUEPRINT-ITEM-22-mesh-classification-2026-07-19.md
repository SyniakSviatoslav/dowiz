# BLUEPRINT — Item 22 (verification half): `kernel/src/mesh.rs` wired-vs-stub classification

**Date:** 2026-07-19 · **Tier:** 0 (read-only) · **Status:** BLUEPRINT with preliminary findings —
executor confirms/extends, does not start blind.
**Source ruling:** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §0 — mesh integration is
**REIMPLEMENT IN DOWIZ, ZERO-DEP**; bebop's mesh (`bebop2/mesh-node`, `proto-wire`, `proto-cap`) is
design-reference/parity-oracle only. This verification therefore scopes *how much of `mesh.rs` is
reusable* for the reimplementation, not whether to proceed. Downstream dependency: synthesis
addendum item 23 (gossip extensions) is explicitly sequenced after this item.
**Proof condition (roadmap §D.4):** classification cited by file:line — typed boundary plus real
kernel caller, or no-caller finding filed.

## 1. Confirmed real path and structure

Path verified: `/root/dowiz/kernel/src/mesh.rs` — 387 lines, module-gated `#![cfg(feature = "pq")]`
(`mesh.rs:25`), registered at `kernel/src/lib.rs:137` (`pub mod mesh;`, itself under
`#[cfg(feature = "pq")]` at `lib.rs:136`). Note: `pq` is **not** a default feature
(`kernel/Cargo.toml:15` `default = ["std"]`; `pq` defined at `Cargo.toml:56`) — a default build does
not compile this module at all.

Public surface, one line each:

- `SignedEntry` (struct, `mesh.rs:36`) — chain entry: `prev_hash` (SHA3-256 link), opaque
  `payload`, ML-DSA-65 `sig`, `pubkey`. Methods: `signed_bytes` (:126), `verify_sig` (:132),
  `content_hash` (:147).
- `MeshError` (enum, `mesh.rs:51`) — fail-closed: `BadSignature` / `BrokenLink{expected,got}` /
  `GenesisMustBeRoot` / `Transport(String)`.
- `Signer` (trait, `mesh.rs:72`) — `sign(&[u8]) -> Vec<u8>` + `pubkey()`. Injection seam for
  hardware/threshold signers.
- `MlDsaSigner` (struct, `mesh.rs:84`) — kernel-native `Signer` over the existing KAT-gated
  `crate::pq::dsa` ML-DSA-65 primitive; `from_seed` (:94), `pubkey_bytes` (:100), `impl Signer`
  (:105). RNG-free, deterministic-signing capable.
- `MeshLog` (struct, `mesh.rs:159`) — append-only signed hash chain; `new`/`len`/`is_empty`/
  `entry`/`entries` (:165–:187), `append` (:193), `verify_chain` (:225).
- `HubTransport` (trait, `mesh.rs:260`) — caller-supplied `send`/`recv` seam; kernel bakes in no
  endpoint/protocol/network.

Internal wiring INTO the kernel is real: `crate::event_log::sha3_256` (`mesh.rs:27`) and
`crate::pq::dsa::{keygen, sign, verify, …}` (`mesh.rs:28`) — the FIPS-204 ACVP-vector-gated
primitive, no new crypto. Five in-module red/green unit tests (`mesh.rs:280–386`): valid chain
accepted, tampered payload rejected, broken link rejected, transport round-trip via a test-supplied
`MemHub`, genesis-must-be-root.

## 2. Caller findings (preliminary — targeted `grep -rn` per symbol, excluding `mesh.rs`)

Repo-wide grep for `dowiz_kernel::mesh` / `kernel::mesh` outside `kernel/benches/` returned
**zero hits**. Per-symbol (all `Signer` grep noise is `RefSigner` from `ports/agent/cap.rs:102` — a
different, unrelated symbol):

| symbol | defined | caller found outside mesh.rs | class |
|---|---|---|---|
| `SignedEntry` | `mesh.rs:36` | NONE | — |
| `MeshError` | `mesh.rs:51` | NONE | — |
| `Signer` (mesh) | `mesh.rs:72` | `kernel/benches/mesh_verify.rs:9` | bench only |
| `MlDsaSigner` | `mesh.rs:84` | `kernel/benches/mesh_verify.rs:9,14` | bench only |
| `MeshLog` | `mesh.rs:159` | `kernel/benches/mesh_verify.rs:9,11,15` (+ doc mention `lib.rs:132`) | bench only |
| `HubTransport` | `mesh.rs:260` | NONE (doc mention `lib.rs:132` only; zero impls outside the in-module test at `mesh.rs:342`) | — |

The bench is registered at `kernel/Cargo.toml:209–212` (`mesh_verify`, `required-features =
["pq"]`). `kernel/tests/` contains **no** mesh integration test (`ls | grep -i mesh` empty).
**No production kernel caller exists for any mesh symbol.** No-caller finding hereby filed for
`SignedEntry`, `MeshError`, `HubTransport`; bench-only finding for `Signer`/`MlDsaSigner`/`MeshLog`.

## 3. Executor table schema

Fill one row per public symbol (and per public method if a method's caller story diverges from its
type's):

| symbol | file:line defined | caller file:line or NONE | verdict (wired / stub) |
|---|---|---|---|

Refinement (recommended, keeps the roadmap's proof condition intact): qualify each caller with a
class — `prod` / `integration-test` / `bench` / `doc-only` — because a bench-only caller must not
be counted as "wired". Only `prod` or `integration-test` callers justify a `wired` verdict; a
tested-but-caller-less primitive is verdict `stub` per the roadmap's binary, with the nuance
recorded in a notes column.

## 4. Honest assessment — scoping signal for the reimplementation

Genuinely mixed, and the two halves point different directions:

- **Not a hollow stub.** The module is a small, real, adversarially-tested cryptographic
  primitive — an append-only ML-DSA-65-signed hash chain wired into the kernel's existing KAT-gated
  crypto and SHA3. Its internal quality is not in question; its red-suite tests distinguish the two
  failure poles (signature vs link) deliberately (`mesh.rs:114–125`).
- **But zero-wired at the caller layer.** Nothing in the kernel constructs a `MeshLog`, appends to
  one, or implements `HubTransport` outside the module's own tests and one bench. It is
  "built-but-unwired" in the project's own established sense — and off by default (`pq` opt-in).
- **And absent at the protocol layer.** Relative to bebop's mesh surface (sync —
  `proto-wire/tests/mesh_sync_integration.rs`; consensus — `proto-cap/tests/mesh_consensus.rs`;
  capability issuance; gossip admission per synthesis §17(b)), `mesh.rs` covers only the bottom
  log-primitive layer. None of the protocol machinery exists in dowiz in any form.

**Net for item-23 scoping:** closer to "mostly stub" at the scale the §0 ruling cares about — the
reimplementation starts near scratch for everything above the signed log — but the executor should
treat `SignedEntry`/`MeshLog`/`MlDsaSigner` as **reusable as-is** (keep, don't rewrite) and
`HubTransport` as the already-correct transport firewall seam to build against. Do not overstate
either way: "70% built" (CORE-ROADMAP-INDEX §0 PROTOCOL row) describes *bebop's* side of the
strand, not this file. Gossip admission itself must extend `decision/import_unit()`'s six-check
pipeline, not this module (synthesis §17(b) — one admission mechanism, never a parallel importer).

## 5. Handoff note (CORE-ROADMAP-INDEX "every planning doc gets a row")

Add to `docs/design/CORE-ROADMAP-INDEX.md`:

> | Item 22 mesh classification blueprint (space-grade Tier 0, 2026-07-19) |
> [BLUEPRINT-ITEM-22-mesh-classification-2026-07-19.md](BLUEPRINT-ITEM-22-mesh-classification-2026-07-19.md) |
> P34-adjacent. Preliminary classification done: `mesh.rs` = real tested signed-log primitive
> (ML-DSA-65 chain, 5 red/green tests), ZERO production kernel callers (bench-only:
> `benches/mesh_verify.rs`; `HubTransport` has zero impls outside in-module test), protocol layer
> absent. Feeds reimplementation scoping per §0 ruling (REIMPLEMENT ZERO-DEP, bebop = parity
> oracle). Executor: confirm table §3, then item 23 unblocks. |

Executor checklist: (1) re-run the §2 greps at execution time (this repo has concurrent writers
today — findings may age); (2) fill the §3 table with verdicts; (3) file the completed
classification against roadmap §D.4's proof condition; (4) only then open item 23.
