# P13 · delivery-on-protocol — design

Zero-dep Layer-B decider-core adapter that turns a hub agreement into a
**real** order transition against the UNMODIFIED `dowiz-kernel` Law.

## Pipeline

```
hub-ring (HRW)  →  IntakeEdge.emit()  →  SignedFrame
   (owner +          (from → to intent,    (canonical OrderTransition bytes
    replicas)         canonical bytes)       + hybrid cap + dual sig)
        │
        ▼
DeliveryReceiver.admit_and_fold(frame, local_order, now)
   gate WIRE  (proto-cap hybrid cap: ed25519 + ML-DSA-65, replay ledger)   ─┐
   gate LAW   (kernel Law: legal transition? verified against local state)  ├─ KernelFacade
   gate MONEY (kernel red-line gate: money move authorized? default-deny) ─┘  (re-exports
                                                                                dowiz-kernel,
                                                                                no reimpl)
        │
        ▼
apply_event → local order state advances (Σ conserved; reversal/compensation edges)
```

## Modules

- `hub_ring.rs` — consistent HRW ownership: `owner_of(order_id)` + replica set.
- `intake.rs` — `IntakeEdge` (owner keypair + PQ keypair + delegation chain)
  and `DeliveryReceiver` (the fold entry point, type-forces the WIRE→LAW→MONEY
  order). `emit` derives a deterministic per-transition nonce
  `(order_id ^ from ^ to)` so the WIRE gate's replay ledger never rejects a
  legitimately-new frame.
- `pod.rs` — k-of-n Proof-of-Delivery: each signer stamps one shared
  SHA3-256 digest with **both** Ed25519 and ML-DSA-65 (NO FROST; the digest is
  the binding, not a threshold signature scheme). `sign_claim` is the
  hub-level hybrid signer.
- `finalization.rs` — F46 partition-then-merge: `detect_conflict` flags a
  conflict **only** when two records disagree on a *terminal* status
  (genuine split-brain). A non-terminal → terminal step is a legal advance,
  not a double-finalize. `reconcile` returns `MergeOutcome::Conflict`
  (quarantine) on conflict — never a silent winner.

## Crypto

All primitives REUSED VERBATIM from `bebop2-core`:
Ed25519 (RFC 8032, from-scratch), ML-DSA-65 (FIPS 204, `pq_dsa`), SHA3.
Zero new dependencies.

## Honesty

- **Default build** (`--no-default-features`): zero kernel dependency; a
  local-law table stands in for the Law (R-MESH01a). 4 tests.
- **`--features kernel-rlib`**: links the UNMODIFIED `dowiz-kernel` Law via the
  `KernelFacade` re-exports — the tests then exercise the REAL Law, not a stub.
  33 tests (intake / pod / finalization).
- The `docs/design` count for this crate is 4 (default) + 33 (kernel-rlib);
  the latter run only with the opt-in feature, so they are not in the
  `cargo test --workspace` (default-features) total.
