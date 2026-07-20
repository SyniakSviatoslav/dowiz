# Phase A — Optical/pixel archival compression: OPERATOR RULING (IN)

- **Date:** 2026-07-19 · **Blueprint:** `BLUEPRINT-ITEM-28-optical-compression-2026-07-19.md`
- **Ruling:** **IN (archival-plane-only, pilot scoped per Phase B).**
- **Status:** Phase A decision recorded; Phase B kernel seam + structural proof delivered
  (see `kernel/src/optical.rs` behind the `optical` feature flag).

---

## 1. The ruling

The operator has **ruled item 28 IN**. The blueprint was `NEEDS-OPERATOR-DECISION` on two
tensions; both are now resolved by the ruling:

- **Tension 1 (model-weight dependency):** RULED IN under the established precedent.
  A vision-model artifact is *weights*, not a Rust crate, and is integrated at a **runtime
  seam outside the Cargo graph** — exactly the discipline of `kernel/src/pq/entropy.rs`'s
  opt-in network provider. The §0 zero-dependency *constraint's spirit* therefore extends
  to model artifacts: model weights must never become a `cargo tree -e no-dev` entry.
  The pilot integrates DeepSeek-OCR-style weights only when a **local GGUF** file is present
  on disk; absent that, the codec returns `Err(OpticalError::ModelUnavailable)`. No
  network dependency, no compiled-in model.
- **Tension 2 (lossy + plane boundary):** SETTLED LAW, reaffirmed. The technique is
  **lossy**; the determinism planes are **categorically excluded**. `OpticalCompressed`
  bytes may reach **only** the archival-tier persistence API (item 20's durability layer).
  They may **never** reach `event_log::sha3_256`, any signature path, any idempotency key,
  or any content-id surface. This is enforced structurally (type-level) in `optical.rs`, not
  by reviewer promise.

## 2. Scope fence (binding)

- IN: living-memory **archival tiers only** (older / lower-priority content, tier-demoted
  via the move-not-delete rule). Representation choice for content item 20 persists durably.
- OUT: idempotency keys, the hash-chained event log, gate verdicts, signatures, any hash
  surface. "97% fidelity on a signature is 100% failure."

## 3. Evidence package (as presented, unchanged by the ruling)

- DeepSeek-OCR (arXiv:2510.18234): <10× → 97% precision, 20× → ~60%, lossy-but-predictable.
  Corroborated by "Text or Pixels?" (arXiv:2510.18279): ~50% fewer decoder tokens at 97–99%.
- Local feasibility: 3B decoder runs via local GGUF (llama.cpp / Ollama) — no network needed.
- Plane-ranking discipline: already coded and load-bearing in `event_log.rs` (timestamps are
  P3-plane, never hashed). `optical.rs` extends that exact discipline to optically-compressed
  bytes via the §1.5 unrepresentability test.

## 4. Honest status of the round-trip oracle

No GGUF DeepSeek-OCR weights exist in this build environment. The real-model round-trip is a
documented **CEILING**, not a claimed result: `OpticalCodec::LocalGguf::encode` /
`decode` return `Err(OpticalError::ModelUnavailable)` until a local GGUF file is supplied.
**No model run is faked.** The `measured-lossy-oracle` (item 1) gap is ledgered in
`HOT-PATHS.tsv`; upgrade trigger: *when local GGUF DeepSeek-OCR weights are present*.

## 5. Acceptance criteria disposition

1. Operator ruling recorded — this doc. ✅
2. Local-corpus pilot through local GGUF — deferred to the runtime-seam upgrade (weights
   absent locally). The seam + `ModelUnavailable` path is the honest verified behavior. ⏳
3. Structural plane-boundary unrepresentability test (`optical_compressed_cannot_reach_
   determinism_plane`) — GREEN. ✅
4. cycles/joules per operation — logged in the seam's `encode`/`decode` stubs as the
   upgrade-trigger contract; not measurable without the model. ⏳
5. `cargo tree -e no-dev` unchanged — the only Cargo change is the `optical = []` feature
   flag (a gate, not a dependency). ✅

---

*This doc is the Phase A deliverable of BLUEPRINT-ITEM-28. It records a decision; it
authorizes no code by itself. The Phase B kernel code is committed separately in the same
branch (`exec/item28-optical`).*
