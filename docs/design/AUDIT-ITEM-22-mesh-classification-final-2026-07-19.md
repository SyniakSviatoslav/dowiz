# AUDIT — Item 22 (verification half): `kernel/src/mesh.rs` wired-vs-stub classification (FINAL)

**Date:** 2026-07-19 · **Tier:** 0 (read-only) · **Status:** COMPLETE — proof artifact filed.
**Executor verdict:** blueprint CONFIRMED (no correction needed), with one additional finding
(`mesh-adapter`, §4) that *strengthens* the zero-production-caller conclusion.

**Roadmap:** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A (Tier 0, Item 22) / §G.4.
**Proof condition (roadmap §G.4 / §D.4):** *"the classification cited by file:line — typed boundary
plus real kernel caller, or no-caller finding filed."* Satisfied by §3 below (one row per public
symbol, file:line defined, caller file:line or NONE, verdict).
**Source blueprint (preliminary):** `BLUEPRINT-ITEM-22-mesh-classification-2026-07-19.md`.
**§0 ruling this feeds:** mesh integration is **REIMPLEMENT IN DOWIZ, ZERO-DEP** (bebop =
parity oracle only). This audit therefore scopes *how much of `mesh.rs` is reusable* for item 23's
reimplementation, not whether to proceed.

## 1. Independent re-verification — every blueprint claim re-run at execution time

Each claim below was re-verified by the executor with its own `grep`/`Read` (not inherited from the
blueprint). Result: all confirmed.

| Blueprint claim | Independent check | Result |
|---|---|---|
| `mesh.rs` is 387 lines | `wc -l kernel/src/mesh.rs` → 387 | ✅ CONFIRMED |
| Registered at `lib.rs:137`, gated at `lib.rs:136` | Read `lib.rs:136-137`: `#[cfg(feature = "pq")]` / `pub mod mesh;` | ✅ CONFIRMED |
| `pq` is NOT a default feature | Read `Cargo.toml:15` `default = ["std"]`; `pq` defined `Cargo.toml:56` | ✅ CONFIRMED — a default build does not compile this module |
| Real crypto: ML-DSA-65 + SHA3-256, no new crypto | `mesh.rs:27` `use crate::event_log::sha3_256;` · `mesh.rs:28` `use crate::pq::dsa::{keygen, sign, verify, …}` | ✅ CONFIRMED — reuses the KAT-gated FIPS-204 primitive |
| 5 in-module red/green tests | `mesh.rs:280-386`: `valid_chain_is_accepted`, `tampered_payload_is_rejected`, `broken_prev_link_is_rejected`, `hub_transport_impl_supplied_by_test`, `genesis_must_be_root` | ✅ CONFIRMED (5 tests present in source; not re-executed — see §5 note) |
| Whole-repo grep `dowiz_kernel::mesh` hits only the bench | `grep -rn "dowiz_kernel::mesh\|kernel::mesh\|crate::mesh" --include="*.rs" .` → **single hit** `kernel/benches/mesh_verify.rs:9` | ✅ CONFIRMED |
| Zero prod callers for `SignedEntry`/`MeshError`/`HubTransport` | per-symbol grep excluding `mesh.rs` → only doc mentions at `lib.rs:132` | ✅ CONFIRMED |
| `MeshLog`/`MlDsaSigner`/`Signer` appear ONLY in the bench | per-symbol grep → only `kernel/benches/mesh_verify.rs` | ✅ CONFIRMED |
| No `kernel/tests/` test exercises mesh | `ls kernel/tests/ \| grep -i mesh` → empty | ✅ CONFIRMED (see §2 — the `verify_chain_wire.rs` false-positive was ruled out) |
| No protocol layer (sync/consensus/capability) in dowiz | no `proto-wire`/`proto-cap`/`mesh-node` dirs in-repo | ✅ CONFIRMED (`mesh-adapter` exists but is NOT that — see §4) |
| No `apps/` or binary caller | `grep` over `apps/`, `kernel/src/bin/` → zero mesh refs | ✅ CONFIRMED |

## 2. Ruled-out false positive: `kernel/tests/verify_chain_wire.rs`

`kernel/tests/` contains `verify_chain_wire.rs`, and `verify_chain` is a `MeshLog` method name — a
plausible missed caller. **It is not.** The test imports from `dowiz_kernel::event_log`
(`verify_chain_wire.rs:10`: `EventLog, MemEventStore, MeshEvent, verify_chain_before_trust`), i.e.
the *separate* `event_log.rs` hash-chain (`EventLog::verify_chain`, `event_log.rs:475`), not
`mesh.rs`'s `MeshLog::verify_chain` (`mesh.rs:225`). The kernel has a **family** of distinct,
independently-named `verify_chain` verifiers — `event_log.rs`, `spine.rs`, `capability_cert.rs`
(`verify_chain_hybrid`), `ports/agent/cap.rs`, `hub_provisioning.rs` — none of which are the mesh
module. `MeshLog::verify_chain` is exercised **only** by the in-module tests (`mesh.rs:288`, etc.)
and the bench (`mesh_verify.rs:28`).

## 3. Classification table (proof artifact) — one row per public symbol

Definitions: **wired** = has a `prod` or `integration-test` caller · **bench-only** = the sole
external caller is the criterion bench · **stub** = no external caller at all (only in-module tests).
A bench-only caller does **not** justify a `wired` verdict (roadmap's binary; nuance in Notes).

| Symbol | file:line defined | Caller (file:line) or NONE | Caller class | Verdict |
|---|---|---|---|---|
| `SignedEntry` (struct) | `mesh.rs:36` | NONE (doc-only `lib.rs:132`) | — | **stub** |
| `SignedEntry::signed_bytes` | `mesh.rs:126` | NONE outside mesh.rs | in-module only | **stub** |
| `SignedEntry::verify_sig` | `mesh.rs:132` | NONE outside mesh.rs | in-module only | **stub** |
| `SignedEntry::content_hash` | `mesh.rs:147` | NONE outside mesh.rs | in-module only | **stub** |
| `MeshError` (enum) | `mesh.rs:51` | NONE | — | **stub** |
| `Signer` (trait) | `mesh.rs:72` | `mesh_verify.rs:9` | bench | **bench-only** |
| `MlDsaSigner` (struct) | `mesh.rs:84` | `mesh_verify.rs:9,14` | bench | **bench-only** |
| `MlDsaSigner::from_seed` | `mesh.rs:94` | `mesh_verify.rs:14` | bench | **bench-only** |
| `MlDsaSigner::pubkey_bytes` | `mesh.rs:100` | NONE outside mesh.rs | in-module only | **stub** |
| `MlDsaSigner` `impl Signer` (`sign`/`pubkey`) | `mesh.rs:105-112` | via `MeshLog::append` in bench | bench (indirect) | **bench-only** |
| `MeshLog` (struct) | `mesh.rs:159` | `mesh_verify.rs:9,11,15` (+ doc `lib.rs:132`) | bench | **bench-only** |
| `MeshLog::new` | `mesh.rs:165` | `mesh_verify.rs:15` | bench | **bench-only** |
| `MeshLog::len`/`is_empty`/`entry`/`entries` | `mesh.rs:170,175,180,185` | NONE outside mesh.rs | in-module only | **stub** |
| `MeshLog::append` | `mesh.rs:193` | `mesh_verify.rs:17` | bench | **bench-only** |
| `MeshLog::verify_chain` | `mesh.rs:225` | `mesh_verify.rs:28` | bench | **bench-only** |
| `HubTransport` (trait) | `mesh.rs:260` | NONE (doc `lib.rs:132`; zero impls outside in-module test `mesh.rs:342`) | — | **stub** |

**Bench registration:** `kernel/Cargo.toml:209-212` (`[[bench]] name = "mesh_verify"`,
`required-features = ["pq"]`).

**No-caller finding filed** (per roadmap §G.4 proof condition) for: `SignedEntry` (+ all 3 methods),
`MeshError`, `HubTransport`, and `MeshLog`'s inspector methods. **Bench-only finding filed** for
`Signer`, `MlDsaSigner`, `MeshLog` (construct/append/verify). **Zero production kernel callers exist
for any mesh symbol.**

## 4. Additional executor finding — `mesh-adapter` (strengthens the conclusion)

The repo contains a `mesh-adapter/` crate (`dowiz-mesh-adapter`) — the §0 ruling names its
"sibling paths." It does **not** contradict the stub finding; it reinforces it:

- Its `Cargo.toml` depends on `dowiz-kernel` with `default-features = false, features = ["std"]` —
  **`pq` is not enabled**, so this crate cannot even compile the `mesh` module (`#![cfg(feature =
  "pq")]`, `mesh.rs:25`).
- It references **zero** `mesh.rs` symbols (grep for `MeshLog`/`SignedEntry`/`HubTransport`/
  `MlDsaSigner`/`dowiz_kernel::mesh` over `mesh-adapter/` → empty).
- Its protocol-adjacent work instead goes through **bebop path-deps** (`bebop-delivery-domain`,
  `bebop-proto-cap`).

So the *only* protocol-adjacent surface that exists in dowiz today (`mesh-adapter`) is
bebop-vendored — exactly what §0's **REIMPLEMENT ZERO-DEP** ruling targets — and it bypasses
`mesh.rs` entirely. `mesh.rs` is not merely uncalled; nothing in the tree is positioned to call it.

## 5. Honest assessment — "mostly stub above the log layer" (item-23 scoping)

Blueprint §4 confirmed. The two halves genuinely point different ways:

- **Not a hollow stub at the primitive layer.** `mesh.rs` is a small, real, adversarially-tested
  ML-DSA-65-signed append-only hash chain, wired to the kernel's *existing* KAT-gated crypto
  (`pq::dsa`) and SHA3 (`event_log::sha3_256`) — no invented crypto. Its 5 red/green tests split the
  two failure poles deliberately (`BadSignature` vs `BrokenLink`, `mesh.rs:114-125`).
- **But zero-wired at the caller layer.** Nothing in the kernel constructs a `MeshLog`, appends to
  one, or implements `HubTransport` outside the module's own tests and one bench — and it is off by
  default (`pq` opt-in). "Built-but-unwired" in the project's established sense.
- **And absent at the protocol layer.** Relative to bebop's mesh (sync/consensus/capability
  issuance/gossip admission), `mesh.rs` covers only the bottom log-primitive layer. None of the
  protocol machinery exists in dowiz in any form (§4 confirms even `mesh-adapter` reaches for
  bebop's, not for `mesh.rs`).

**Net for item 23 (gated strictly after item 22 — do not start it from this audit):**
- **Reuse as-is (keep, don't rewrite):** `SignedEntry`, `MeshLog`, `MlDsaSigner`, and the `Signer`
  seam — the signed-log primitive is done and tested.
- **`HubTransport`** is the already-correct transport-firewall seam to build the reimplementation
  *against* (kernel bakes in no endpoint/protocol/network — `mesh.rs:250-265`).
- **Build near-scratch above the log:** sync, consensus, capability issuance, and gossip admission
  do not exist in dowiz. Per synthesis §17(b), gossip admission must **extend
  `decision/import_unit()`'s existing pipeline**, not add a parallel importer to this module.
- **Do not overstate.** "≈70% built" (CORE-ROADMAP-INDEX §0 PROTOCOL row) describes *bebop's* side
  of the strand, not this file. On the dowiz side, above the signed log, the reimplementation starts
  near zero.

*Note on test execution:* the 5 in-module tests are confirmed present in source; they were not
re-executed in this pass to avoid contending the shared `target/` dir with the concurrent writers
active today (this is a read-only classification item; greenness is not the roadmap §G.4 proof
condition, which is caller classification by file:line — satisfied by §3). `cargo test -p
dowiz-kernel --features pq mesh::` re-runs them if independent green is later required.

## 6. Verification checklist (roadmap §G.4)

- [x] Classification cited by file:line — §3 table, one row per public symbol.
- [x] No-caller finding filed for the uncalled symbols; bench-only finding for the bench-exercised ones.
- [x] Blueprint claims independently re-run at execution time (§1), false positive ruled out (§2).
- [x] Item-23 scoping finding recorded (§5) for whoever executes the reimplementation next.
- [x] Item 22 verification half COMPLETE. Item 23 (reimplementation) remains gated strictly after —
      not started here.
